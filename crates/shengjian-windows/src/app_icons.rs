//! 应用图标提取（「打开应用」选择器列表用）。
//!
//! 输入是 OpenApp target 的各种历史形态（预设 id / 完整 exe 路径 / `.lnk` /
//! `shell:AppsFolder\…` AUMID / 裸 exe 名），输出 `data:image/png;base64,...`
//! （32×32；列表里约 20px 显示，payload 尽量小）。失败返回 `None`，前端退回通用图标。
//!
//! 约定：
//! - 只读：不启动应用、不写文件；
//! - COM 在独立 STA 线程内初始化（与 `pick_custom_app` / 应用扫描同一线程模型），
//!   一次批量请求只初始化一次；
//! - 进程内缓存 `target -> Option<data_url>`（含失败结果），避免列表刷新时重复提取；
//! - 日志只记数量/耗时/来源种类，不记个人路径；
//! - 非 Windows 返回全 `None`，保持跨平台编译。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::app_launcher::{is_bare_executable_name, preset_app, preset_aumid, PresetApp};
use crate::registered_apps::{is_registered_target, REGISTERED_PREFIX};

/// data URL 前缀：前端可直接塞进 `<img src>`。
const PNG_DATA_URL_PREFIX: &str = "data:image/png;base64,";

/// 图标边长（像素）。
const ICON_SIZE: i32 = 32;

/// 进程内缓存：target（trim 后）→ data URL；`Some(None)` 表示提取失败也已缓存。
static ICON_CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();

fn icon_cache() -> &'static Mutex<HashMap<String, Option<String>>> {
    ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 批量提取应用图标，返回顺序与输入一致；失败/空 target 为 `None`。
///
/// 同一 target 只提取一次（进程内缓存，含失败结果）；同一批里的重复 target 会去重。
pub fn app_icon_data_urls(targets: &[String]) -> Vec<Option<String>> {
    let result = icon_data_urls_with_cache(icon_cache(), targets, extract_icon_data_urls);
    #[cfg(windows)]
    crate::ble::gatt_note(format!(
        "app_icons action=data_urls phase=completed requested={} provided={}",
        targets.len(),
        result.iter().filter(|url| url.is_some()).count()
    ));
    result
}

/// 单个 target 的图标 data URL（批量入口的便捷包装）。
pub fn app_icon_data_url(target: &str) -> Option<String> {
    app_icon_data_urls(std::slice::from_ref(&target.to_owned()))
        .into_iter()
        .next()
        .flatten()
}

/// 缓存 + 批量提取的核心：注入缓存与提取函数，便于单测覆盖命中/顺序/失败缓存。
///
/// `extract` 只对未命中的 target 调用一次（同一批里的重复 target 已去重），
/// 返回值与输入一一对应、长度可能小于未命中数（提取方实现异常时按 `None` 处理）。
fn icon_data_urls_with_cache<F>(
    cache: &Mutex<HashMap<String, Option<String>>>,
    targets: &[String],
    extract: F,
) -> Vec<Option<String>>
where
    F: FnOnce(&[String]) -> Vec<Option<String>>,
{
    // 键用 trim 后的 target：同一应用不会因首尾空格重复提取。
    let keys: Vec<String> = targets
        .iter()
        .map(|target| target.trim().to_owned())
        .collect();
    let mut results: Vec<Option<String>> = vec![None; keys.len()];
    let mut missing: Vec<(usize, String)> = Vec::new();
    {
        let Ok(guard) = cache.lock() else {
            // 锁中毒：退化为不缓存的一次性提取（不能 panic，也不能吞掉图标）。
            return extract(&keys);
        };
        for (index, key) in keys.iter().enumerate() {
            match guard.get(key) {
                // 命中（含缓存的失败结果 None）直接返回，不重复提取。
                Some(cached) => results[index] = cached.clone(),
                None => missing.push((index, key.clone())),
            }
        }
    }
    if missing.is_empty() {
        return results;
    }
    // 同批重复 target 只提取一次（列表可能同时出现预设 id 与同一条目的重复项）。
    let mut unique_keys: Vec<String> = Vec::new();
    for (_, key) in &missing {
        if !unique_keys.iter().any(|existing| existing == key) {
            unique_keys.push(key.clone());
        }
    }
    let extracted = extract(&unique_keys);
    if let Ok(mut guard) = cache.lock() {
        for (key, value) in unique_keys.iter().zip(extracted.iter()) {
            guard.insert(key.clone(), value.clone());
        }
    }
    for (index, key) in &missing {
        if let Some(position) = unique_keys.iter().position(|existing| existing == key) {
            results[*index] = extracted.get(position).cloned().flatten();
        }
    }
    results
}

/// target 形态分类（纯字符串 + 预设表判定，不触碰 OS；便于单测）。
enum IconTargetKind {
    /// 预设 id：按预设启动位置/登记的 AUMID 取图标。
    Preset(&'static PresetApp),
    /// `shell:AppsFolder\…`：直接走 Shell 项图标。
    Aumid(String),
    /// `.lnk`：先解析目标 exe，再退回 Shell 项图标。
    Shortcut(String),
    /// 完整路径（.exe/.bat/.cmd 等）。
    Path(String),
    /// 裸可执行文件名（旧式 target），能解析成路径才有图标。
    Bare(String),
    /// 空串/未知形态：没有图标。
    Invalid,
}

fn classify_icon_target(target: &str) -> IconTargetKind {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return IconTargetKind::Invalid;
    }
    if let Some(app) = preset_app(trimmed) {
        return IconTargetKind::Preset(app);
    }
    if is_registered_target(trimmed) {
        return IconTargetKind::Aumid(trimmed.to_owned());
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains('\\') || lower.contains('/') {
        return if lower.ends_with(".lnk") {
            IconTargetKind::Shortcut(trimmed.to_owned())
        } else {
            IconTargetKind::Path(trimmed.to_owned())
        };
    }
    if is_bare_executable_name(trimmed) {
        return IconTargetKind::Bare(trimmed.to_owned());
    }
    IconTargetKind::Invalid
}

/// 标准 base64 编码（RFC 4648，带 `=` 填充）。
///
/// 手写实现：项目未依赖 base64 crate，而这里只需要编码 PNG 字节这一种用途。
fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = u32::from(chunk[0]);
        let second = u32::from(chunk.get(1).copied().unwrap_or(0));
        let third = u32::from(chunk.get(2).copied().unwrap_or(0));
        let triple = (first << 16) | (second << 8) | third;
        encoded.push(ALPHABET[((triple >> 18) & 0x3f) as usize] as char);
        encoded.push(ALPHABET[((triple >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            encoded.push(ALPHABET[((triple >> 6) & 0x3f) as usize] as char);
        } else {
            encoded.push('=');
        }
        if chunk.len() > 2 {
            encoded.push(ALPHABET[(triple & 0x3f) as usize] as char);
        } else {
            encoded.push('=');
        }
    }
    encoded
}

#[cfg(not(windows))]
fn extract_icon_data_urls(targets: &[String]) -> Vec<Option<String>> {
    vec![None; targets.len()]
}

// ---------------------------------------------------------------------------
// Windows 实现
// ---------------------------------------------------------------------------

#[cfg(windows)]
fn extract_icon_data_urls(targets: &[String]) -> Vec<Option<String>> {
    let count = targets.len();
    if count == 0 {
        return Vec::new();
    }
    let requested = targets.to_vec();
    std::thread::Builder::new()
        .name("shengjian-app-icons".to_owned())
        .spawn(move || {
            // Shell 项图标（UWP/.lnk/普通文件）需要 COM；放到独立 STA 线程，
            // 避免占用 Tauri 命令线程的套间（与 pick_custom_app 同一模型）。
            let com_initialized = unsafe {
                windows::Win32::System::Com::CoInitializeEx(
                    None,
                    windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
                )
                .is_ok()
            };
            // 开始菜单索引整批只扫一次（预设 id 的快捷方式兜底解析要用）。
            let shortcuts = crate::app_launcher::start_menu_shortcuts();
            let result: Vec<Option<String>> = requested
                .iter()
                .map(|target| extract_target_icon_data_url(target, &shortcuts))
                .collect();
            if com_initialized {
                unsafe {
                    windows::Win32::System::Com::CoUninitialize();
                }
            }
            result
        })
        .ok()
        .and_then(|handle| handle.join().ok())
        .unwrap_or_else(|| vec![None; count])
}

#[cfg(windows)]
fn extract_target_icon_data_url(
    target: &str,
    shortcuts: &[(String, std::path::PathBuf)],
) -> Option<String> {
    extract_target_icon_png(target, shortcuts)
        .map(|png| format!("{PNG_DATA_URL_PREFIX}{}", base64_encode(&png)))
}

/// 按 target 形态取 PNG 字节：可执行文件走 `PrivateExtractIconsW`（快、清晰），
/// AUMID / `.lnk` / 脚本类走 Shell 项图标（`IShellItemImageFactory`，UWP 可靠）。
#[cfg(windows)]
fn extract_target_icon_png(
    target: &str,
    shortcuts: &[(String, std::path::PathBuf)],
) -> Option<Vec<u8>> {
    use crate::app_launcher::{
        preset_executable_path, resolve_bare_executable, shortcut_executable_path,
    };

    match classify_icon_target(target) {
        IconTargetKind::Preset(app) => {
            // 遥控器自身：直接取当前进程 exe，不依赖安装路径探测。
            if app.id == "shengjian" {
                if let Ok(executable) = std::env::current_exe() {
                    if let Some(png) = file_icon_png(&executable.to_string_lossy(), ICON_SIZE) {
                        return Some(png);
                    }
                }
            }
            if let Some(path) = preset_executable_path(app, shortcuts) {
                if let Some(png) = file_icon_png(&path, ICON_SIZE) {
                    return Some(png);
                }
            }
            let aumid = preset_aumid(app.id)?;
            shell_item_icon_png(&format!("{REGISTERED_PREFIX}{aumid}"))
        }
        IconTargetKind::Aumid(target) => shell_item_icon_png(&target),
        IconTargetKind::Shortcut(path) => {
            if let Some(executable) = shortcut_executable_path(&path) {
                if let Some(png) = file_icon_png(&executable, ICON_SIZE) {
                    return Some(png);
                }
            }
            shell_item_icon_png(&path)
        }
        IconTargetKind::Path(path) => {
            if let Some(png) = file_icon_png(&path, ICON_SIZE) {
                return Some(png);
            }
            shell_item_icon_png(&path)
        }
        IconTargetKind::Bare(name) => {
            let path = resolve_bare_executable(&name)?;
            file_icon_png(&path.to_string_lossy(), ICON_SIZE)
        }
        IconTargetKind::Invalid => None,
    }
}

/// 文件图标：先 `PrivateExtractIconsW`（32bpp、边缘清晰），失败再 `SHGetFileInfoW`
/// 兜底（覆盖图标只登记在 Shell 关联/图标缓存里的可执行文件，如部分启动器）。
#[cfg(windows)]
fn file_icon_png(path: &str, size: i32) -> Option<Vec<u8>> {
    icon_png_from_executable(path, size).or_else(|| shell_file_icon_png(path))
}

/// `SHGetFileInfoW` 的大图标（32×32）→ PNG，作为 `PrivateExtractIconsW` 的兜底。
#[cfg(windows)]
fn shell_file_icon_png(path: &str) -> Option<Vec<u8>> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
    use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;

    let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
    let mut info = SHFILEINFOW::default();
    let result = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_ATTRIBUTE_NORMAL,
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    if result == 0 || info.hIcon.is_invalid() {
        return None;
    }
    let png = icon_to_png(info.hIcon);
    unsafe {
        let _ = DestroyIcon(info.hIcon);
    }
    png
}

/// 从可执行文件提取指定尺寸图标并编码为 PNG。
///
/// 回眸历史（64×64）与「打开应用」选择器（32×32）共用这一实现；短路径限制
/// 与 `PrivateExtractIconsW` 的 WIN32 路径上限一致（>=259 个 UTF-16 单元放弃）。
#[cfg(windows)]
pub(crate) fn icon_png_from_executable(path: &str, size: i32) -> Option<Vec<u8>> {
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, PrivateExtractIconsW, HICON};

    if !(1..=256).contains(&size) || path.encode_utf16().count() >= 259 {
        return None;
    }
    let mut file = [0u16; 260];
    for (index, value) in path.encode_utf16().enumerate() {
        file[index] = value;
    }
    let mut icons = [HICON::default()];
    let count = unsafe { PrivateExtractIconsW(&file, 0, size, size, Some(&mut icons), None, 1) };
    if count == 0 || icons[0].is_invalid() {
        return None;
    }
    let encoded = icon_to_png(icons[0]);
    unsafe {
        let _ = DestroyIcon(icons[0]);
    }
    encoded
}

/// `HICON` → PNG（32bpp 图标：BGRA 直乘 alpha，无 alpha 时补全不透明）。
#[cfg(windows)]
fn icon_to_png(icon: windows::Win32::UI::WindowsAndMessaging::HICON) -> Option<Vec<u8>> {
    use std::ffi::c_void;

    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetIconInfo;

    let mut info: windows::Win32::UI::WindowsAndMessaging::ICONINFO = unsafe { std::mem::zeroed() };
    if unsafe { GetIconInfo(icon, &mut info) }.is_err() {
        return None;
    }
    let color = info.hbmColor;
    let mask = info.hbmMask;
    let encoded = (|| {
        let mut bitmap: BITMAP = unsafe { std::mem::zeroed() };
        let copied = unsafe {
            GetObjectW(
                color.into(),
                std::mem::size_of::<BITMAP>() as i32,
                Some(&mut bitmap as *mut BITMAP as *mut c_void),
            )
        };
        if copied == 0 {
            return None;
        }
        let width = bitmap.bmWidth;
        let height = bitmap.bmHeight;
        if !(1..=256).contains(&width) || !(1..=256).contains(&height) {
            return None;
        }
        let mut info_header: BITMAPINFO = unsafe { std::mem::zeroed() };
        info_header.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
        let dc = unsafe { CreateCompatibleDC(None) };
        if dc.is_invalid() {
            return None;
        }
        let lines = unsafe {
            GetDIBits(
                dc,
                color,
                0,
                height as u32,
                Some(pixels.as_mut_ptr() as *mut c_void),
                &mut info_header,
                DIB_RGB_COLORS,
            )
        };
        unsafe {
            let _ = DeleteDC(dc);
        }
        if lines == 0 {
            return None;
        }
        let mut rgba = Vec::with_capacity(pixels.len());
        let mut any_alpha = false;
        for pixel in pixels.chunks_exact(4) {
            any_alpha |= pixel[3] != 0;
            rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
        if !any_alpha {
            for pixel in rgba.chunks_exact_mut(4) {
                pixel[3] = 255;
            }
        }
        encode_rgba_png(width as u32, height as u32, &rgba)
    })();
    unsafe {
        if !color.is_invalid() {
            let _ = DeleteObject(color.into());
        }
        if !mask.is_invalid() {
            let _ = DeleteObject(mask.into());
        }
    }
    encoded
}

/// Shell 项（UWP/`.lnk`/普通文件）图标 → PNG。
///
/// 用 `IShellItemImageFactory` 而不是 `SHGetFileInfoW`：后者对
/// `shell:AppsFolder\…` 这类 Shell 命名空间项不可靠，前者是 UWP 图标的标准口径。
#[cfg(windows)]
fn shell_item_icon_png(shell_path: &str) -> Option<Vec<u8>> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::Graphics::Gdi::DeleteObject;
    use windows::Win32::UI::Shell::{
        IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
    };

    let wide: Vec<u16> = shell_path.encode_utf16().chain(Some(0)).collect();
    let item: IShellItemImageFactory =
        unsafe { SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None) }.ok()?;
    let bitmap = unsafe {
        item.GetImage(
            SIZE {
                cx: ICON_SIZE,
                cy: ICON_SIZE,
            },
            SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK,
        )
    }
    .ok()?;
    let png = hbitmap_to_png(bitmap);
    // GetImage 返回调用方拥有的 HBITMAP，必须释放。
    unsafe {
        let _ = DeleteObject(bitmap.into());
    }
    png
}

/// `HBITMAP` → PNG。Shell 返回的位图是**预乘 alpha** 的 BGRA；PNG 需要直乘 alpha，
/// 否则半透明边缘（UWP 圆角图标）会发黑。
#[cfg(windows)]
fn hbitmap_to_png(bitmap: windows::Win32::Graphics::Gdi::HBITMAP) -> Option<Vec<u8>> {
    use std::ffi::c_void;

    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
        BI_RGB, DIB_RGB_COLORS,
    };

    let mut source: BITMAP = unsafe { std::mem::zeroed() };
    let copied = unsafe {
        GetObjectW(
            bitmap.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut source as *mut BITMAP as *mut c_void),
        )
    };
    if copied == 0 {
        return None;
    }
    let width = source.bmWidth;
    let height = source.bmHeight;
    if !(1..=256).contains(&width) || !(1..=256).contains(&height) {
        return None;
    }
    let mut info_header: BITMAPINFO = unsafe { std::mem::zeroed() };
    info_header.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width,
        biHeight: -height,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };
    let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
    let dc = unsafe { CreateCompatibleDC(None) };
    if dc.is_invalid() {
        return None;
    }
    let lines = unsafe {
        GetDIBits(
            dc,
            bitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut c_void),
            &mut info_header,
            DIB_RGB_COLORS,
        )
    };
    unsafe {
        let _ = DeleteDC(dc);
    }
    if lines == 0 {
        return None;
    }
    // 少数旧图标资源没有 alpha 通道（GetDIBits 补的 32bpp 全 0）；此时按不透明处理，
    // 否则整张图会被当成全透明。
    let has_alpha = pixels.chunks_exact(4).any(|pixel| pixel[3] != 0);
    let mut rgba = Vec::with_capacity(pixels.len());
    for pixel in pixels.chunks_exact(4) {
        if !has_alpha {
            rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255]);
            continue;
        }
        let alpha = pixel[3];
        if alpha == 0 {
            rgba.extend_from_slice(&[0, 0, 0, 0]);
        } else if alpha == 255 {
            rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255]);
        } else {
            let unpremultiply = |channel: u8| -> u8 {
                ((u32::from(channel) * 255 + u32::from(alpha) / 2) / u32::from(alpha)).min(255)
                    as u8
            };
            rgba.extend_from_slice(&[
                unpremultiply(pixel[2]),
                unpremultiply(pixel[1]),
                unpremultiply(pixel[0]),
                alpha,
            ]);
        }
    }
    encode_rgba_png(width as u32, height as u32, &rgba)
}

#[cfg(windows)]
fn encode_rgba_png(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    if rgba.len() != (width as usize) * (height as usize) * 4 {
        return None;
    }
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(rgba).ok()?;
    }
    Some(png)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// base64 标准向量（含三种填充情况与 PNG 文件头）。
    #[test]
    fn base64_encoder_matches_rfc4648_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        // PNG 文件签名编码后是固定前缀（前端可用它校验 data URL 有效）。
        assert_eq!(base64_encode(b"\x89PNG\r\n\x1a\n"), "iVBORw0KGgo=");
    }

    /// target 形态分类：预设 / AUMID / .lnk / 路径 / 裸名 / 空。
    #[test]
    fn classifies_target_shapes() {
        assert!(matches!(
            classify_icon_target("wechat"),
            IconTargetKind::Preset(app) if app.id == "wechat"
        ));
        assert!(matches!(
            classify_icon_target("shell:AppsFolder\\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"),
            IconTargetKind::Aumid(_)
        ));
        assert!(matches!(
            classify_icon_target(r"C:\Apps\Tool\Tool.exe"),
            IconTargetKind::Path(path) if path == r"C:\Apps\Tool\Tool.exe"
        ));
        assert!(matches!(
            classify_icon_target(r"C:\Apps\Tool.lnk"),
            IconTargetKind::Shortcut(_)
        ));
        assert!(matches!(
            classify_icon_target("notepad.exe"),
            IconTargetKind::Bare(name) if name == "notepad.exe"
        ));
        // 首尾空格按 trim 后判定（缓存也用 trim 键）。
        assert!(matches!(
            classify_icon_target("  wechat  "),
            IconTargetKind::Preset(app) if app.id == "wechat"
        ));
        for invalid in ["", "   ", "微信", "not-a-known-app-id"] {
            assert!(
                matches!(classify_icon_target(invalid), IconTargetKind::Invalid),
                "应判为无效：{invalid:?}"
            );
        }
    }

    fn empty_cache() -> Mutex<HashMap<String, Option<String>>> {
        Mutex::new(HashMap::new())
    }

    /// 缓存：同一 target 只提取一次；结果含缓存的失败（None）。
    #[test]
    fn cache_extracts_each_target_once_including_failures() {
        let cache = empty_cache();
        let mut calls = 0;
        let targets = vec!["a".to_owned(), "b".to_owned(), "a".to_owned()];
        let first = icon_data_urls_with_cache(&cache, &targets, |missing| {
            calls += 1;
            assert_eq!(missing, ["a".to_owned(), "b".to_owned()]);
            vec![Some("url-a".to_owned()), None]
        });
        assert_eq!(
            first,
            vec![Some("url-a".to_owned()), None, Some("url-a".to_owned())]
        );
        assert_eq!(calls, 1);
        assert_eq!(
            cache.lock().unwrap().get("b").cloned(),
            Some(None),
            "失败结果也要缓存，避免反复提取"
        );

        // 第二次全部命中缓存（包括 None），不再调用提取函数。
        let second = icon_data_urls_with_cache(&cache, &targets, |_| {
            panic!("缓存命中时不应再提取");
        });
        assert_eq!(second, first);
    }

    /// 命中/部分命中混合时只对未命中项提取，且保持输入顺序。
    #[test]
    fn cache_preserves_order_for_partial_misses() {
        let cache = empty_cache();
        cache
            .lock()
            .unwrap()
            .insert("cached".to_owned(), Some("url-cached".to_owned()));
        let targets = vec!["cached".to_owned(), "fresh".to_owned()];
        let result = icon_data_urls_with_cache(&cache, &targets, |missing| {
            assert_eq!(missing, ["fresh".to_owned()]);
            vec![Some("url-fresh".to_owned())]
        });
        assert_eq!(
            result,
            vec![Some("url-cached".to_owned()), Some("url-fresh".to_owned())]
        );
    }

    /// 提取方返回数量不足时按 None 兜底，不 panic、不错位。
    #[test]
    fn cache_tolerates_short_extraction_results() {
        let cache = empty_cache();
        let targets = vec!["a".to_owned(), "b".to_owned()];
        let result = icon_data_urls_with_cache(&cache, &targets, |_| Vec::new());
        assert_eq!(result, vec![None, None]);
    }

    /// 空输入直接返回空结果（不启动提取线程）。
    #[test]
    fn empty_targets_return_empty_result() {
        let cache = empty_cache();
        let result = icon_data_urls_with_cache(&cache, &[], |_| panic!("空输入不应调用提取函数"));
        assert!(result.is_empty());
    }

    /// 真机口径：系统 exe 路径能提取出 PNG data URL（前缀与 PNG 签名可校验）。
    #[cfg(windows)]
    #[test]
    fn extracts_data_url_from_system_executable() {
        let url = app_icon_data_url(r"C:\Windows\System32\notepad.exe").expect("notepad icon");
        assert!(url.starts_with(PNG_DATA_URL_PREFIX));
        let payload = url.trim_start_matches(PNG_DATA_URL_PREFIX);
        assert!(
            payload.starts_with("iVBORw0KGgo"),
            "payload 应是 PNG：{}",
            &payload[..payload.len().min(32)]
        );
        assert!(payload.len() > 100, "32×32 图标 payload 不应为空");
    }

    /// 真机口径：预设 id 与裸 exe 名都能经解析链路取到图标（选择器主要入口）。
    #[cfg(windows)]
    #[test]
    fn resolves_preset_and_bare_name_targets() {
        let preset = app_icon_data_url("notepad").expect("notepad preset icon");
        assert!(preset.starts_with(PNG_DATA_URL_PREFIX));
        let bare = app_icon_data_url("notepad.exe").expect("notepad bare name icon");
        assert!(bare.starts_with(PNG_DATA_URL_PREFIX));
        // 无效 target / 空串没有图标，但不能 panic。
        assert_eq!(app_icon_data_url("not-a-known-app-id"), None);
        assert_eq!(app_icon_data_url(""), None);
    }

    /// 真机口径（默认 ignore）：UWP/Store 应用的 AUMID 图标走 Shell 项。
    #[cfg(windows)]
    #[test]
    #[ignore = "requires the Calculator Store app on this machine"]
    fn extracts_uwp_aumid_icon_on_this_machine() {
        let url =
            app_icon_data_url("shell:AppsFolder\\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App")
                .expect("calculator aumid icon");
        assert!(url.starts_with(PNG_DATA_URL_PREFIX));
    }
}
