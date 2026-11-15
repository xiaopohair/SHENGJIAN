//! 应用内语音测试板：置顶的无边框原生窗口 + 标准 `EDIT`，视觉上做成面板的"附属卡片"。
//!
//! 为什么需要它：输入法（TSF）切到**本进程的 WebView2** 会触发整页重载
//! （`Bugs/2026-09-12`），所以前台是主窗口时核心会跳过输入法激活。原生 EDIT
//! 不受这个限制，输入法可以正常在里面落字。
//!
//! 形态：无边框、置顶的**独立窗口**，位置精确贴在「识别内容」框上（父窗口客户区
//! 坐标 → 屏幕坐标），配色由前端按当前主题传入，视觉上就是面板的一部分。
//!
//! 注意：`WS_CHILD` 子窗口会被 WebView2 的绘制盖住（实测：聚焦后往里打字，屏幕上
//! 毫无变化），所以不能真做子窗口；只能靠"独立窗口 + 贴靠 + 同色"来模拟。
//!
//! 资源约定（60 次连续打开实测无增长）：
//! - 窗口只创建一次，之后每次 `show` 只做 SetWindowPos/ShowWindow/SetFocus；
//! - 窗口类只注册一次，重复注册按"已存在"放过（窗口被系统销毁后可重建）；
//! - 位置监听钩子只装一次；`WM_PAINT` 里创建的画刷当场删除，Region 由系统接管。
#![cfg(windows)]

use std::sync::atomic::{AtomicI32, AtomicIsize, AtomicU32, Ordering};

use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    DWM_WINDOW_CORNER_PREFERENCE,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, ClientToScreen, CreateRoundRectRgn, CreateSolidBrush, DeleteObject, EndPaint,
    FillRect, FrameRect, GetStockObject, SetWindowRgn, DEFAULT_GUI_FONT, HFONT, PAINTSTRUCT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::Input::KeyboardAndMouse::{SetFocus, VK_ESCAPE};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClassInfoExW, GetClientRect,
    GetForegroundWindow, GetGUIThreadInfo, GetWindowTextLengthW, GetWindowTextW, LoadCursorW,
    MoveWindow, RegisterClassExW, SendMessageW, SetForegroundWindow, SetWindowPos, SetWindowTextW,
    ShowWindow, ES_AUTOVSCROLL, ES_LEFT, ES_MULTILINE, EVENT_OBJECT_LOCATIONCHANGE, GUITHREADINFO,
    HWND_TOP, IDC_ARROW, OBJID_WINDOW, SWP_SHOWWINDOW, SW_HIDE, SW_SHOW, WINDOW_STYLE,
    WINEVENT_OUTOFCONTEXT, WM_CLOSE, WM_CREATE, WM_DESTROY, WM_ERASEBKGND, WM_GETTEXT, WM_KEYDOWN,
    WM_PAINT, WM_SETFONT, WM_SIZE, WNDCLASSEXW, WS_CHILD, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP, WS_VISIBLE, WS_VSCROLL,
};

const PAD_CLASS: windows::core::PCWSTR = w!("ShengjianVoiceTestPad");
const PAD_TITLE: windows::core::PCWSTR = w!("语音测试");

static PAD_HWND: AtomicIsize = AtomicIsize::new(0);
static EDIT_HWND: AtomicIsize = AtomicIsize::new(0);
/// 背景色 / 边框色（COLORREF，0x00BBGGRR），由前端按当前主题传入。
static BG_COLOR: AtomicU32 = AtomicU32::new(0x00FF_FFFF);
static BORDER_COLOR: AtomicU32 = AtomicU32::new(0x00D8_D8D8);
/// 卡片应在主窗口客户区里的位置（由 show 写入，跟随线程反复套用）。
static PAD_PARENT: AtomicIsize = AtomicIsize::new(0);
static CARD_X: AtomicI32 = AtomicI32::new(0);
static CARD_Y: AtomicI32 = AtomicI32::new(0);
static CARD_W: AtomicI32 = AtomicI32::new(0);
static CARD_H: AtomicI32 = AtomicI32::new(0);
/// 展开时的完整矩形；收起后按它还原。
static FULL_Y: AtomicI32 = AtomicI32::new(0);
static FULL_H: AtomicI32 = AtomicI32::new(0);
const COLLAPSED_HEIGHT: i32 = 46;
static LOCATION_HOOK: AtomicIsize = AtomicIsize::new(0);
static HOOK_OWNER: AtomicIsize = AtomicIsize::new(0);
const CORNER_RADIUS: i32 = 18;

fn hwnd_of(value: isize) -> Option<HWND> {
    (value != 0).then(|| HWND(value as *mut core::ffi::c_void))
}

fn pad_hwnd() -> Option<HWND> {
    hwnd_of(PAD_HWND.load(Ordering::Relaxed))
}

fn edit_hwnd() -> Option<HWND> {
    hwnd_of(EDIT_HWND.load(Ordering::Relaxed))
}

/// 设置卡片配色（`0x00RRGGBB`，前端从 CSS 计算样式取）；内部转成 COLORREF。
pub fn set_theme(background: u32, border: u32) {
    let to_colorref = |rgb: u32| {
        let r = (rgb >> 16) & 0xFF;
        let g = (rgb >> 8) & 0xFF;
        let b = rgb & 0xFF;
        r | (g << 8) | (b << 16)
    };
    BG_COLOR.store(to_colorref(background), Ordering::Relaxed);
    BORDER_COLOR.store(to_colorref(border), Ordering::Relaxed);
}

pub fn has_focus() -> bool {
    let (Some(pad), Some(edit)) = (pad_hwnd(), edit_hwnd()) else {
        return false;
    };
    unsafe {
        let foreground = GetForegroundWindow();
        if foreground.0.is_null() {
            return false;
        }
        if foreground == pad {
            return true;
        }
        let thread_id =
            windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(foreground, None);
        if thread_id == 0 {
            return false;
        }
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        if GetGUIThreadInfo(thread_id, &mut info).is_err() {
            return false;
        }
        info.hwndFocus == edit || info.hwndFocus == pad
    }
}

/// 打开测试板：贴在「识别内容」框的位置（父窗口客户区坐标，物理像素）。
pub fn show(parent: isize, x: i32, y: i32, width: i32, height: i32) -> Result<(), String> {
    if pad_hwnd().is_none() {
        create_pad()?;
    }
    let Some(pad) = pad_hwnd() else {
        return Err("测试板窗口尚未创建".to_owned());
    };
    PAD_PARENT.store(parent, Ordering::Relaxed);
    CARD_X.store(x, Ordering::Relaxed);
    CARD_Y.store(y, Ordering::Relaxed);
    CARD_W.store(width.max(160), Ordering::Relaxed);
    CARD_H.store(height.max(48), Ordering::Relaxed);
    FULL_Y.store(y, Ordering::Relaxed);
    FULL_H.store(height.max(48), Ordering::Relaxed);
    place_card(pad);
    unsafe {
        let _ = ShowWindow(pad, SW_SHOW);
        let _ = SetForegroundWindow(pad);
        if let Some(edit) = edit_hwnd() {
            let _ = SetFocus(Some(edit));
            SetWindowTextW(edit, w!(""));
        }
    }
    watch(parent);
    Ok(())
}

fn create_pad() -> Result<(), String> {
    unsafe {
        let instance = GetModuleHandleW(None).map_err(|error| error.to_string())?;
        let class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(pad_proc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: PAD_CLASS,
            ..Default::default()
        };
        // 类只需注册一次；窗口被销毁后重建时这里会失败（类已存在），要当成正常情况。
        if RegisterClassExW(&class) == 0 {
            let mut existing = windows::Win32::UI::WindowsAndMessaging::WNDCLASSEXW::default();
            let known = windows::Win32::UI::WindowsAndMessaging::GetClassInfoExW(
                Some(instance.into()),
                PAD_CLASS,
                &mut existing,
            );
            if known.is_err() {
                return Err("注册测试板窗口类失败".to_owned());
            }
        }
        let pad = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            PAD_CLASS,
            PAD_TITLE,
            WINDOW_STYLE(WS_POPUP.0),
            0,
            0,
            320,
            120,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .map_err(|error| error.to_string())?;
        PAD_HWND.store(pad.0 as isize, Ordering::Relaxed);
        // Win11 原生圆角（拿不到就算了，WM_SIZE 里还有一层 Region 兜底）。
        let preference: DWM_WINDOW_CORNER_PREFERENCE = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            pad,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &preference as *const _ as *const core::ffi::c_void,
            std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        );
    }
    Ok(())
}

/// 把卡片摆到"主窗口客户区原点 + 记录的位置"上（主窗口移动/缩放后也复用）。
fn place_card(pad: HWND) {
    let mut x = CARD_X.load(Ordering::Relaxed);
    let mut y = CARD_Y.load(Ordering::Relaxed);
    if let Some(owner) = hwnd_of(PAD_PARENT.load(Ordering::Relaxed)) {
        let mut origin = POINT { x: 0, y: 0 };
        unsafe {
            let _ = ClientToScreen(owner, &mut origin);
        }
        x += origin.x;
        y += origin.y;
    }
    unsafe {
        let _ = SetWindowPos(
            pad,
            None,
            x,
            y,
            CARD_W.load(Ordering::Relaxed),
            CARD_H.load(Ordering::Relaxed),
            windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
        );
    }
}

/// 主窗口移动/缩放的回调（WinEvent 的 EVENT_OBJECT_LOCATIONCHANGE 就是 WM_MOVE/WM_SIZE
/// 的系统级通知，宿主窗口过程是 Tauri 的，没法直接子类化，用钩子拿到了同样的时机）。
unsafe extern "system" fn on_location_change(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    _thread: u32,
    _time: u32,
) {
    if event != EVENT_OBJECT_LOCATIONCHANGE {
        return;
    }
    if id_object != OBJID_WINDOW.0 || id_child != 0 {
        return;
    }
    if hwnd.0 as isize != HOOK_OWNER.load(Ordering::Relaxed) {
        return;
    }
    if let Some(pad) = pad_hwnd() {
        place_card(pad);
    }
}

/// 安装一次位置监听；必须在该窗口所属线程（有消息循环）上调用。
fn watch(owner: isize) {
    HOOK_OWNER.store(owner, Ordering::Relaxed);
    if LOCATION_HOOK.load(Ordering::Relaxed) != 0 {
        return;
    }
    unsafe {
        let hook = SetWinEventHook(
            EVENT_OBJECT_LOCATIONCHANGE,
            EVENT_OBJECT_LOCATIONCHANGE,
            None,
            Some(on_location_change),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );
        LOCATION_HOOK.store(hook.0 as isize, Ordering::Relaxed);
    }
}

/// 原生窗口当前是不是前台（用来判断"点的是测试板还是别处"）。
pub fn is_foreground() -> bool {
    let Some(pad) = pad_hwnd() else {
        return false;
    };
    unsafe { GetForegroundWindow() == pad }
}

/// 收起成底部一条细带：把上面的文字露出来，同时保住键盘焦点（输入法才不会丢目标）。
pub fn collapse() {
    let full_y = FULL_Y.load(Ordering::Relaxed);
    let full_h = FULL_H.load(Ordering::Relaxed);
    CARD_Y.store(full_y + full_h - COLLAPSED_HEIGHT, Ordering::Relaxed);
    CARD_H.store(COLLAPSED_HEIGHT, Ordering::Relaxed);
    if let Some(pad) = pad_hwnd() {
        place_card(pad);
        if let Some(edit) = edit_hwnd() {
            unsafe {
                let _ = SetFocus(Some(edit));
            }
        }
    }
}

/// 还原成完整高度（下一次按下语音键时用）。
pub fn expand() {
    CARD_Y.store(FULL_Y.load(Ordering::Relaxed), Ordering::Relaxed);
    CARD_H.store(FULL_H.load(Ordering::Relaxed), Ordering::Relaxed);
    if let Some(pad) = pad_hwnd() {
        place_card(pad);
    }
}

pub fn raise() {
    if let Some(pad) = pad_hwnd() {
        unsafe {
            let _ = SetWindowPos(
                pad,
                Some(HWND_TOP),
                0,
                0,
                0,
                0,
                windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE
                    | windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE
                    | windows::Win32::UI::WindowsAndMessaging::SWP_SHOWWINDOW,
            );
        }
    }
}

/// 彻底关掉：销毁原生窗口（必须在创建它的线程上调用），下次打开重建。
pub fn close() -> Result<(), String> {
    let Some(pad) = pad_hwnd() else {
        return Ok(());
    };
    unsafe {
        if DestroyWindow(pad).is_err() {
            return Err("销毁测试板窗口失败".to_owned());
        }
    }
    Ok(())
}

/// 只隐藏（保留窗口与已输入内容）——目前供无主线程调用的场景兜底。
pub fn hide() {
    if let Some(pad) = pad_hwnd() {
        unsafe {
            let _ = ShowWindow(pad, SW_HIDE);
        }
    }
}

pub fn text() -> Option<String> {
    let edit = edit_hwnd()?;
    unsafe {
        let len = GetWindowTextLengthW(edit);
        if len <= 0 {
            return None;
        }
        let mut buffer = vec![0u16; len as usize + 1];
        let copied = GetWindowTextW(edit, &mut buffer);
        if copied <= 0 {
            return None;
        }
        let value = String::from_utf16_lossy(&buffer[..copied as usize])
            .trim()
            .to_owned();
        (!value.is_empty()).then_some(value)
    }
}

unsafe extern "system" fn pad_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let edit = CreateWindowExW(
                Default::default(),
                w!("EDIT"),
                w!(""),
                WINDOW_STYLE(
                    WS_CHILD.0
                        | WS_VISIBLE.0
                        | WS_VSCROLL.0
                        | ES_LEFT as u32
                        | ES_MULTILINE as u32
                        | ES_AUTOVSCROLL as u32,
                ),
                1,
                1,
                300,
                100,
                Some(hwnd),
                None,
                None,
                None,
            );
            if let Ok(edit) = edit {
                EDIT_HWND.store(edit.0 as isize, Ordering::Relaxed);
                let font = GetStockObject(DEFAULT_GUI_FONT);
                SendMessageW(
                    edit,
                    WM_SETFONT,
                    Some(WPARAM(HFONT(font.0).0 as usize)),
                    Some(LPARAM(1)),
                );
            }
            LRESULT(0)
        }
        WM_SIZE => {
            let width = (lparam.0 & 0xFFFF) as i32;
            let height = ((lparam.0 >> 16) & 0xFFFF) as i32;
            if let Some(edit) = edit_hwnd() {
                let _ = MoveWindow(edit, 2, 2, (width - 4).max(80), (height - 4).max(32), true);
            }
            // 无边框窗口自己裁圆角（DWM 的圆角对无框架弹窗不一定生效）。
            let region =
                CreateRoundRectRgn(0, 0, width + 1, height + 1, CORNER_RADIUS, CORNER_RADIUS);
            if !region.is_invalid() {
                let _ = SetWindowRgn(hwnd, Some(region), true);
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            let background = CreateSolidBrush(COLORREF(BG_COLOR.load(Ordering::Relaxed)));
            let border = CreateSolidBrush(COLORREF(BORDER_COLOR.load(Ordering::Relaxed)));
            let _ = FillRect(hdc, &rect, background);
            let _ = FrameRect(hdc, &rect, border);
            let _ = DeleteObject(background.into());
            let _ = DeleteObject(border.into());
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_KEYDOWN if wparam.0 as u16 == VK_ESCAPE.0 => {
            // 卡片是独立窗口，Esc 会先到它这里：收起卡片并把焦点还给主窗口。
            let _ = ShowWindow(hwnd, SW_HIDE);
            if let Some(owner) = hwnd_of(PAD_PARENT.load(Ordering::Relaxed)) {
                let _ = SetForegroundWindow(owner);
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            PAD_HWND.store(0, Ordering::Relaxed);
            EDIT_HWND.store(0, Ordering::Relaxed);
            LRESULT(0)
        }
        WM_GETTEXT => DefWindowProcW(hwnd, msg, wparam, lparam),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
