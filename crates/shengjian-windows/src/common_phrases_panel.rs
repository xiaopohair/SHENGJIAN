//! 常用语浮层：不抢焦点的原生窗口 + 文本插入。
//!
//! 浮层展示常用语列表；遥控器 ↑/↓ 移动选择，OK 插入，返回/Menu 关闭。

#![cfg(windows)]

use crate::common_phrases::CommonPhraseSlot;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Arc, Mutex, Once, OnceLock};

use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, CreatePen, CreateRoundRectRgn, CreateSolidBrush, DeleteObject,
    DrawTextW, EndPaint, FillRect, FrameRect, GetStockObject, InvalidateRect, RoundRect,
    SelectObject, SetBkMode, SetMapMode, SetTextColor, SetViewportExtEx, SetWindowExtEx,
    SetWindowRgn, CLEARTYPE_QUALITY, DEFAULT_CHARSET, DRAW_TEXT_FORMAT, DT_CENTER, DT_END_ELLIPSIS,
    DT_LEFT, DT_RIGHT, DT_SINGLELINE, DT_TOP, DT_VCENTER, FONT_CLIP_PRECISION,
    FONT_OUTPUT_PRECISION, HDC, MM_ANISOTROPIC, NULL_BRUSH, NULL_PEN, PAINTSTRUCT, PS_SOLID,
    TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, PostMessageW,
    PostQuitMessage, RegisterClassW, SetWindowPos, ShowWindow, TranslateMessage, CS_DROPSHADOW,
    CS_HREDRAW, CS_VREDRAW, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_SHOWNOACTIVATE,
    WM_CLOSE, WM_DESTROY, WM_LBUTTONUP, WM_PAINT, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};

const PANEL_WIDTH: i32 = 520;
const PANEL_HEIGHT: i32 = 500;
const ROW_HEIGHT: i32 = 54;
const VISIBLE_ROWS: usize = 6;
const LIST_TOP: i32 = 104;
const LIST_BOTTOM: i32 = LIST_TOP + ROW_HEIGHT * VISIBLE_ROWS as i32;

#[derive(Clone, Copy)]
struct Palette {
    bg: u32,
    border: u32,
    border_strong: u32,
    text: u32,
    muted: u32,
    faint: u32,
    accent: u32,
    accent_soft: u32,
    chip: u32,
}

fn dark_palette() -> Palette {
    Palette {
        bg: 0x00151614,
        border: 0x002c2e2b,
        border_strong: 0x003a3d37,
        text: 0x00f3f5f2,
        muted: 0x00919590,
        faint: 0x00676b66,
        accent: 0x008fd6b4,
        accent_soft: 0x00252f2a,
        chip: 0x00232622,
    }
}

fn light_palette() -> Palette {
    Palette {
        bg: 0x00f4f4ef,
        border: 0x00d8d6ce,
        border_strong: 0x00c4c1b7,
        text: 0x001c1e1a,
        muted: 0x006b7068,
        faint: 0x0092968d,
        accent: 0x002f7d59,
        accent_soft: 0x00e2f0e7,
        chip: 0x00f0efe9,
    }
}

fn palette() -> Palette {
    if PANEL_LIGHT_PANEL.load(Ordering::Relaxed) {
        light_palette()
    } else {
        dark_palette()
    }
}

static PANEL_HWND: AtomicIsize = AtomicIsize::new(0);
static PANEL_LIGHT_PANEL: AtomicBool = AtomicBool::new(false);
static REGISTER_CLASS: Once = Once::new();
static PANEL_MODEL: OnceLock<Mutex<Option<PanelModel>>> = OnceLock::new();

struct PanelModel {
    slots: Vec<CommonPhraseSlot>,
    selected: usize,
}

fn panel_model_slot() -> &'static Mutex<Option<PanelModel>> {
    PANEL_MODEL.get_or_init(|| Mutex::new(None))
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

unsafe fn draw_text_flags(
    hdc: HDC,
    text: &str,
    rect: &mut RECT,
    color: u32,
    size: i32,
    flags: DRAW_TEXT_FORMAT,
) {
    let font = CreateFontW(
        -size,
        0,
        0,
        0,
        if size >= 15 { 600 } else { 400 },
        0,
        0,
        0,
        DEFAULT_CHARSET,
        FONT_OUTPUT_PRECISION(0),
        FONT_CLIP_PRECISION(0),
        CLEARTYPE_QUALITY,
        0,
        w!("Segoe UI"),
    );
    let previous = SelectObject(hdc, font.into());
    SetBkMode(hdc, TRANSPARENT);
    SetTextColor(hdc, COLORREF(color));
    let mut buffer = wide(text);
    let len = buffer.len().saturating_sub(1);
    DrawTextW(
        hdc,
        &mut buffer[..len],
        rect,
        flags | DT_SINGLELINE | DT_END_ELLIPSIS,
    );
    SelectObject(hdc, previous);
    let _ = DeleteObject(font.into());
}

unsafe fn draw_text(hdc: HDC, text: &str, rect: &mut RECT, color: u32, size: i32) {
    draw_text_flags(hdc, text, rect, color, size, DT_LEFT | DT_VCENTER);
}

unsafe fn draw_text_top(hdc: HDC, text: &str, rect: &mut RECT, color: u32, size: i32) {
    draw_text_flags(hdc, text, rect, color, size, DT_LEFT | DT_TOP);
}

unsafe fn round_fill(hdc: HDC, rect: &RECT, radius: i32, color: u32) {
    let brush = CreateSolidBrush(COLORREF(color));
    let pen = GetStockObject(NULL_PEN);
    let old_brush = SelectObject(hdc, brush.into());
    let old_pen = SelectObject(hdc, pen);
    let _ = RoundRect(
        hdc,
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        radius,
        radius,
    );
    SelectObject(hdc, old_brush);
    SelectObject(hdc, old_pen);
    let _ = DeleteObject(brush.into());
}

unsafe fn round_stroke(hdc: HDC, rect: &RECT, radius: i32, color: u32) {
    let pen = CreatePen(PS_SOLID, 1, COLORREF(color));
    let brush = GetStockObject(NULL_BRUSH);
    let old_pen = SelectObject(hdc, pen.into());
    let old_brush = SelectObject(hdc, brush);
    let _ = RoundRect(
        hdc,
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        radius,
        radius,
    );
    SelectObject(hdc, old_pen);
    SelectObject(hdc, old_brush);
    let _ = DeleteObject(pen.into());
}

unsafe fn draw_badge(hdc: HDC, c: Palette) {
    let rect = RECT {
        left: 26,
        top: 26,
        right: 60,
        bottom: 60,
    };
    round_fill(hdc, &rect, 9, c.accent_soft);
    round_stroke(hdc, &rect, 9, c.accent);
    let ink = CreateSolidBrush(COLORREF(c.accent));
    for line in 0..3 {
        let y = 33 + line * 6;
        let dot = RECT {
            left: 33,
            top: y,
            right: 37,
            bottom: y + 3,
        };
        FillRect(hdc, &dot, ink);
        let bar = RECT {
            left: 41,
            top: y,
            right: 53,
            bottom: y + 3,
        };
        FillRect(hdc, &bar, ink);
    }
    let _ = DeleteObject(ink.into());
}

/// 键帽：浅底 + 细边框 + 底部一道深色线，做出「实体按键」的立体感。
///
/// `primary` 给主操作（OK）用强调色描边；其余按键保持中性，和弹窗底栏的
/// 「键帽 + 说明文字」排版一致。
unsafe fn draw_keycap(
    hdc: HDC,
    left: i32,
    top: i32,
    label: &str,
    width: i32,
    primary: bool,
    c: Palette,
) -> i32 {
    let rect = RECT {
        left,
        top,
        right: left + width,
        bottom: top + 24,
    };
    round_fill(hdc, &rect, 5, if primary { c.accent_soft } else { c.chip });
    round_stroke(
        hdc,
        &rect,
        5,
        if primary { c.accent } else { c.border_strong },
    );
    // 键帽下沿的暗线：比整块边框重一档，读起来像一颗立起来的键。
    let edge = RECT {
        left: left + 4,
        top: rect.bottom - 3,
        right: left + width - 4,
        bottom: rect.bottom - 2,
    };
    let edge_brush = CreateSolidBrush(COLORREF(if primary { c.accent } else { c.border_strong }));
    FillRect(hdc, &edge, edge_brush);
    let _ = DeleteObject(edge_brush.into());
    let mut text_rect = RECT {
        left,
        top,
        right: left + width,
        bottom: top + 23,
    };
    draw_text_flags(
        hdc,
        label,
        &mut text_rect,
        if primary { c.accent } else { c.text },
        11,
        DT_CENTER | DT_VCENTER,
    );
    left + width
}

unsafe fn paint(hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    let dpi = (96.0 * crate::display::scale_factor()).round().max(96.0) as i32;
    let _ = SetMapMode(hdc, MM_ANISOTROPIC);
    let _ = SetWindowExtEx(hdc, 96, 96, None);
    let _ = SetViewportExtEx(hdc, dpi, dpi, None);
    let client = RECT {
        left: 0,
        top: 0,
        right: PANEL_WIDTH,
        bottom: PANEL_HEIGHT,
    };
    let c = palette();

    let background = CreateSolidBrush(COLORREF(c.bg));
    FillRect(hdc, &client, background);
    let _ = DeleteObject(background.into());

    let border = CreateSolidBrush(COLORREF(c.border));
    FrameRect(hdc, &client, border);
    let _ = DeleteObject(border.into());

    draw_badge(hdc, c);

    let mut title_rect = RECT {
        left: 70,
        top: 28,
        right: client.right - 116,
        bottom: 52,
    };
    draw_text(hdc, "常用语", &mut title_rect, c.text, 18);

    let mut subtitle_rect = RECT {
        left: 70,
        top: 50,
        right: client.right - 116,
        bottom: 72,
    };
    draw_text(
        hdc,
        "选择要插入到当前输入框的文本",
        &mut subtitle_rect,
        c.muted,
        12,
    );

    let model = panel_model_slot().lock().ok().and_then(|guard| {
        guard
            .as_ref()
            .map(|model| (model.slots.clone(), model.selected))
    });
    let Some((slots, selected)) = model else {
        let _ = EndPaint(hwnd, &ps);
        return;
    };

    let mut count_rect = RECT {
        left: client.right - 110,
        top: 34,
        right: client.right - 26,
        bottom: 64,
    };
    draw_text_flags(
        hdc,
        &format!("{} 条", slots.len()),
        &mut count_rect,
        c.faint,
        12,
        DT_LEFT | DT_VCENTER,
    );

    let divider = CreateSolidBrush(COLORREF(c.border));
    let divider_rect = RECT {
        left: 26,
        top: 86,
        right: client.right - 26,
        bottom: 87,
    };
    FillRect(hdc, &divider_rect, divider);
    let _ = DeleteObject(divider.into());

    let empty = slots.is_empty();
    if empty {
        let mut empty_rect = RECT {
            left: 28,
            top: LIST_TOP + 40,
            right: client.right - 28,
            bottom: LIST_TOP + 82,
        };
        draw_text(hdc, "还没有常用语", &mut empty_rect, c.text, 16);
        let mut hint_rect = RECT {
            left: 28,
            top: LIST_TOP + 82,
            right: client.right - 28,
            bottom: LIST_TOP + 112,
        };
        draw_text(
            hdc,
            "请先在设置 → 常规 → 常用语中添加",
            &mut hint_rect,
            c.muted,
            13,
        );
    } else {
        let start = if selected >= VISIBLE_ROWS {
            selected + 1 - VISIBLE_ROWS
        } else {
            0
        };
        for (offset, slot) in slots.iter().skip(start).take(VISIBLE_ROWS).enumerate() {
            let index = start + offset;
            let top = LIST_TOP + offset as i32 * ROW_HEIGHT;
            // 列表行（对齐宏导出弹窗的列表）：选中态是整行淡色底，不画卡片边框，
            // 前面的勾选框去掉——一行就是一个可选条目，勾选语义和「当前项」重复。
            let row = RECT {
                left: 16,
                top: top + 3,
                right: client.right - 16,
                bottom: top + ROW_HEIGHT - 3,
            };
            if index == selected {
                round_fill(hdc, &row, 8, c.accent_soft);
            }

            // 行分隔线：画在行底，最后一行不画，免得列表底部多出一条。
            if index + 1 < slots.len() {
                let divider = RECT {
                    left: 28,
                    top: top + ROW_HEIGHT - 1,
                    right: client.right - 28,
                    bottom: top + ROW_HEIGHT,
                };
                let divider_brush = CreateSolidBrush(COLORREF(c.border));
                FillRect(hdc, &divider, divider_brush);
                let _ = DeleteObject(divider_brush.into());
            }

            let mut index_rect = RECT {
                left: client.right - 54,
                top: row.top,
                right: client.right - 28,
                bottom: row.bottom,
            };
            draw_text_flags(
                hdc,
                &format!("{:02}", index + 1),
                &mut index_rect,
                c.faint,
                12,
                DT_RIGHT | DT_VCENTER,
            );

            let label = slot.label.trim();
            let text = slot.text.trim();
            let title = if !label.is_empty() {
                label.to_owned()
            } else if !text.is_empty() {
                text.lines().next().unwrap_or("").chars().take(26).collect()
            } else {
                "未填写".to_owned()
            };
            let preview = if text.is_empty() {
                "未填写文本".to_owned()
            } else {
                text.lines().next().unwrap_or("").chars().take(46).collect()
            };

            // 行首不再有勾选框，文字直接从左边距起排（与列表行的左内边距对齐）。
            let text_left = 30;
            let mut title_line = RECT {
                left: text_left,
                top: top + 10,
                right: client.right - 62,
                bottom: top + 31,
            };
            draw_text_top(hdc, &title, &mut title_line, c.text, 14);
            let mut preview_line = RECT {
                left: text_left,
                top: top + 29,
                right: client.right - 62,
                bottom: top + 48,
            };
            draw_text_top(hdc, &preview, &mut preview_line, c.muted, 12);
        }

        if slots.len() > VISIBLE_ROWS {
            let track = RECT {
                left: client.right - 12,
                top: LIST_TOP + 4,
                right: client.right - 8,
                bottom: LIST_BOTTOM - 4,
            };
            let track_brush = CreateSolidBrush(COLORREF(c.border));
            FillRect(hdc, &track, track_brush);
            let _ = DeleteObject(track_brush.into());
            let track_height = (track.bottom - track.top) as f32;
            let thumb_height = (track_height * VISIBLE_ROWS as f32 / slots.len() as f32).max(20.0);
            let progress = if slots.len() > 1 {
                selected as f32 / (slots.len() - 1) as f32
            } else {
                0.0
            };
            let thumb_top = track.top as f32 + (track_height - thumb_height) * progress;
            let thumb = RECT {
                left: track.left,
                top: thumb_top as i32,
                right: track.right,
                bottom: (thumb_top + thumb_height) as i32,
            };
            let thumb_brush = CreateSolidBrush(COLORREF(c.accent));
            FillRect(hdc, &thumb, thumb_brush);
            let _ = DeleteObject(thumb_brush.into());
        }
    }

    let footer_top = client.bottom - 48;
    let footer_line = RECT {
        left: 26,
        top: footer_top - 8,
        right: client.right - 26,
        bottom: footer_top - 7,
    };
    let footer_brush = CreateSolidBrush(COLORREF(c.border));
    FillRect(hdc, &footer_line, footer_brush);
    let _ = DeleteObject(footer_brush.into());

    let mut x = 26;
    x = draw_keycap(hdc, x, footer_top + 6, "↑↓", 34, false, c) + 8;
    let mut label_rect = RECT {
        left: x,
        top: footer_top + 6,
        right: x + 34,
        bottom: footer_top + 32,
    };
    draw_text(hdc, "选择", &mut label_rect, c.muted, 12);
    x += 38;
    x = draw_keycap(hdc, x, footer_top + 6, "OK", 32, true, c) + 8;
    label_rect = RECT {
        left: x,
        top: footer_top + 6,
        right: x + 34,
        bottom: footer_top + 32,
    };
    draw_text(hdc, "插入", &mut label_rect, c.muted, 12);
    x += 38;
    x = draw_keycap(hdc, x, footer_top + 6, "Menu", 48, false, c) + 8;
    label_rect = RECT {
        left: x,
        top: footer_top + 6,
        right: x + 34,
        bottom: footer_top + 32,
    };
    draw_text(hdc, "关闭", &mut label_rect, c.muted, 12);

    if !empty {
        let mut position_rect = RECT {
            left: client.right - 140,
            top: footer_top + 6,
            right: client.right - 26,
            bottom: footer_top + 32,
        };
        draw_text_flags(
            hdc,
            &format!("{} / {}", selected + 1, slots.len()),
            &mut position_rect,
            c.faint,
            12,
            DT_LEFT | DT_VCENTER,
        );
    }

    let _ = EndPaint(hwnd, &ps);
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            if PANEL_HWND.load(Ordering::SeqCst) == hwnd.0 as isize {
                PANEL_HWND.store(0, Ordering::SeqCst);
                if let Ok(mut guard) = panel_model_slot().lock() {
                    *guard = None;
                }
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

fn register_class() -> bool {
    REGISTER_CLASS.call_once(|| unsafe {
        let instance = match GetModuleHandleW(None) {
            Ok(handle) => handle,
            Err(_) => return,
        };
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW | CS_DROPSHADOW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance.into(),
            lpszClassName: w!("ShengjianCommonPhrases"),
            ..Default::default()
        };
        let _ = RegisterClassW(&class);
    });
    true
}

fn create_window() -> Option<HWND> {
    if !register_class() {
        return None;
    }
    let instance = unsafe { GetModuleHandleW(None).ok()? };
    let screen_width = unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
            windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN,
        )
    };
    let screen_height = unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
            windows::Win32::UI::WindowsAndMessaging::SM_CYSCREEN,
        )
    };
    // 跟应用主题同向：浅色应用用浅色面板，深色应用用深色 raised 面板。
    PANEL_LIGHT_PANEL.store(!crate::display::dark_theme(), Ordering::SeqCst);
    let width = crate::display::scale_px(PANEL_WIDTH);
    let height = crate::display::scale_px(PANEL_HEIGHT);
    let x = ((screen_width - width) / 2).max(0);
    let y = ((screen_height - height) / 2).max(0);
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            w!("ShengjianCommonPhrases"),
            w!("常用语"),
            WS_POPUP,
            x,
            y,
            width,
            height,
            None,
            None,
            Some(instance.into()),
            None,
        )
    }
    .ok()?;
    PANEL_HWND.store(hwnd.0 as isize, Ordering::SeqCst);
    let radius = crate::display::scale_px(20);
    let region = unsafe { CreateRoundRectRgn(0, 0, width + 1, height + 1, radius, radius) };
    unsafe {
        SetWindowRgn(hwnd, Some(region), true);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            x,
            y,
            width,
            height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    }
    Some(hwnd)
}

pub fn show_panel(slots: Vec<CommonPhraseSlot>) -> bool {
    hide_panel();
    PANEL_HWND.store(0, Ordering::SeqCst);
    if let Ok(mut guard) = panel_model_slot().lock() {
        *guard = Some(PanelModel { slots, selected: 0 });
    }
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("shengjian-common-phrases-panel".to_owned())
        .spawn(move || {
            let hwnd = create_window();
            let _ = sender.send(hwnd.is_some());
            if hwnd.is_none() {
                return;
            }
            let mut message = windows::Win32::UI::WindowsAndMessaging::MSG::default();
            unsafe {
                while GetMessageW(&mut message, None, 0, 0).as_bool() {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
        })
        .ok();
    receiver
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap_or(false)
}

pub fn hide_panel() {
    let raw = PANEL_HWND.load(Ordering::SeqCst);
    if raw == 0 {
        return;
    }
    let hwnd = HWND(raw as *mut std::ffi::c_void);
    unsafe {
        let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
    }
}

pub fn move_selection(delta: isize) {
    let Ok(mut guard) = panel_model_slot().lock() else {
        return;
    };
    let Some(model) = guard.as_mut() else {
        return;
    };
    if model.slots.is_empty() {
        return;
    }
    let next =
        (model.selected as isize + delta).clamp(0, model.slots.len().saturating_sub(1) as isize);
    model.selected = next as usize;
    let raw = PANEL_HWND.load(Ordering::SeqCst);
    if raw != 0 {
        let hwnd = HWND(raw as *mut std::ffi::c_void);
        unsafe {
            let _ = InvalidateRect(Some(hwnd), None, true);
        }
    }
}

pub fn selected_text() -> Option<String> {
    let guard = panel_model_slot().lock().ok()?;
    let model = guard.as_ref()?;
    let slot = model.slots.get(model.selected)?;
    Some(slot.text.trim().to_owned())
}

pub fn insert_text(text: &str) -> Result<(), String> {
    let units: Vec<u16> = text.encode_utf16().collect();
    if units.is_empty() {
        return Ok(());
    }
    for chunk in units.chunks(64) {
        for unit in chunk {
            let down = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: *unit,
                        dwFlags: KEYEVENTF_UNICODE,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            let up = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: *unit,
                        dwFlags: KEYBD_EVENT_FLAGS(KEYEVENTF_UNICODE.0 | KEYEVENTF_KEYUP.0),
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            let sent = unsafe { SendInput(&[down, up], std::mem::size_of::<INPUT>() as i32) };
            if sent != 2 {
                return Err("input_unavailable".to_owned());
            }
        }
    }
    Ok(())
}
