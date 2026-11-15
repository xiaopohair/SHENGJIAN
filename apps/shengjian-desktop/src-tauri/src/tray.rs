use std::sync::Mutex;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{App, AppHandle, Emitter, Manager, Runtime, Theme};

static TRAY_LANGUAGE: Mutex<String> = Mutex::new(String::new());
static TRAY_UPDATE_VERSION: Mutex<Option<String>> = Mutex::new(None);
#[derive(Debug, Clone, Default)]
pub struct TrayStatus {
    pub profile_name: String,
    pub remote_name: String,
    pub input_method: String,
    pub battery_level: Option<u8>,
}

static TRAY_STATUS: Mutex<Option<TrayStatus>> = Mutex::new(None);

/// Tray glyph: the 32 px app icon (brand: app-icon-32, bundled as 32x32.png).
///
/// Its own dark background keeps the keycap readable on light and dark
/// taskbars alike, so the tray no longer needs a per-theme variant.
const TRAY_ICON: &[u8] = include_bytes!("../icons/32x32.png");
/// Bright pine window / taskbar mark for dark taskbars (brand: mark-64).
const WINDOW_ICON_FOR_DARK_TASKBAR: &[u8] = include_bytes!("../icons/mark-64.png");
/// Deep pine window / taskbar mark for light taskbars (brand: mark-64-deep).
const WINDOW_ICON_FOR_LIGHT_TASKBAR: &[u8] = include_bytes!("../icons/mark-64-deep.png");

fn labels(language: &str) -> (&'static str, &'static str, &'static str, &'static str) {
    if language == "en" {
        (
            "Show main window",
            "Hide main window",
            "Open settings",
            "Open log folder",
        )
    } else {
        ("显示主窗口", "隐藏主窗口", "打开设置", "打开日志目录")
    }
}

fn build_menu<R: Runtime, M: Manager<R>>(
    manager: &M,
    language: &str,
    update_label: Option<&str>,
) -> tauri::Result<Menu<R>> {
    let (show_label, hide_label, settings_label, logs_label) = labels(language);
    let show = MenuItem::with_id(manager, "show", show_label, true, None::<&str>)?;
    let hide = MenuItem::with_id(manager, "hide", hide_label, true, None::<&str>)?;
    let settings = MenuItem::with_id(manager, "settings", settings_label, true, None::<&str>)?;
    let logs = MenuItem::with_id(manager, "logs", logs_label, true, None::<&str>)?;
    let quit_label = if language == "en" { "Quit" } else { "退出" };
    let quit = MenuItem::with_id(manager, "quit", quit_label, true, None::<&str>)?;
    let menu = Menu::new(manager)?;
    menu.append(&show)?;
    menu.append(&hide)?;
    menu.append(&settings)?;
    if let Some(label) = update_label {
        let update = MenuItem::with_id(manager, "update", label, true, None::<&str>)?;
        menu.append(&update)?;
    }
    menu.append(&logs)?;
    menu.append(&quit)?;
    Ok(menu)
}

fn update_label(language: &str, version: &str) -> String {
    if language == "en" {
        format!("Update available: {version}")
    } else {
        format!("有新版本：{version}")
    }
}

fn non_empty(value: &str, fallback_zh: &str, fallback_en: &str, language: &str) -> String {
    if value.trim().is_empty() {
        if language == "en" {
            fallback_en.to_owned()
        } else {
            fallback_zh.to_owned()
        }
    } else {
        value.trim().to_owned()
    }
}

fn tray_tooltip(
    language: &str,
    update_version: Option<&str>,
    status: Option<&TrayStatus>,
) -> String {
    let profile = non_empty(
        status.map(|value| value.profile_name.as_str()).unwrap_or(""),
        "默认",
        "Default",
        language,
    );
    let remote = non_empty(
        status.map(|value| value.remote_name.as_str()).unwrap_or(""),
        "未连接",
        "Not connected",
        language,
    );
    let input_method = non_empty(
        status.map(|value| value.input_method.as_str()).unwrap_or(""),
        "未选择",
        "Not selected",
        language,
    );
    let battery = status
        .and_then(|value| value.battery_level)
        .map(|level| format!("{level}%"))
        .unwrap_or_else(|| "--".to_owned());

    let mut lines = vec![
        "声键".to_owned(),
        if language == "en" {
            format!("Button profile: {profile}")
        } else {
            format!("按键方案：{profile}")
        },
        if language == "en" {
            format!("Remote: {remote}")
        } else {
            format!("遥控器：{remote}")
        },
        if language == "en" {
            format!("Input method: {input_method}")
        } else {
            format!("输入法：{input_method}")
        },
        if language == "en" {
            format!("Battery: {battery}")
        } else {
            format!("电量：{battery}")
        },
    ];
    if let Some(version) = update_version {
        lines.push(if language == "en" {
            format!("Update available: {version}")
        } else {
            format!("有新版本：{version}")
        });
    }
    lines.join("\n")
}

fn refresh_tray(app: &AppHandle) -> Result<(), String> {
    let language = TRAY_LANGUAGE
        .lock()
        .map(|value| value.clone())
        .unwrap_or_else(|_| "zh".to_owned());
    let update_version = TRAY_UPDATE_VERSION
        .lock()
        .ok()
        .and_then(|value| value.clone());
    let update_text = update_version
        .as_deref()
        .map(|version| update_label(&language, version));
    let menu =
        build_menu(app, &language, update_text.as_deref()).map_err(|error| error.to_string())?;
    let tray = app
        .tray_by_id("main-tray")
        .ok_or_else(|| "tray not found".to_owned())?;
    tray.set_menu(Some(menu))
        .map_err(|error| error.to_string())?;
    let status = TRAY_STATUS
        .lock()
        .ok()
        .and_then(|value| value.clone());
    let tooltip = tray_tooltip(&language, update_version.as_deref(), status.as_ref());
    tray.set_tooltip(Some(tooltip.as_str()))
        .map_err(|error| error.to_string())
}

/// 状态线程只更新 tooltip 内容，不重建菜单，也不会被语言/更新刷新覆盖。
pub fn set_tray_status(app: &AppHandle, status: TrayStatus) {
    if let Ok(mut stored) = TRAY_STATUS.lock() {
        *stored = Some(status.clone());
    }
    let language = TRAY_LANGUAGE
        .lock()
        .map(|value| value.clone())
        .unwrap_or_else(|_| "zh".to_owned());
    let update_version = TRAY_UPDATE_VERSION
        .lock()
        .ok()
        .and_then(|value| value.clone());
    let tooltip = tray_tooltip(&language, update_version.as_deref(), Some(&status));
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(tooltip.as_str()));
    }
}

/// Decodes the tray glyph. The app icon is theme-independent.
fn tray_icon() -> Option<Image<'static>> {
    Image::from_bytes(TRAY_ICON).ok()
}

/// Picks the window (taskbar / alt-tab) mark for the current taskbar.
fn window_icon_for_theme(theme: Option<Theme>) -> Option<Image<'static>> {
    let bytes = match theme {
        Some(Theme::Light) => WINDOW_ICON_FOR_LIGHT_TASKBAR,
        _ => WINDOW_ICON_FOR_DARK_TASKBAR,
    };
    Image::from_bytes(bytes).ok()
}

/// Reads the window theme, which follows the Windows app (taskbar) theme.
pub fn current_theme<R: Runtime>(app: &AppHandle<R>) -> Option<Theme> {
    app.get_webview_window("main")
        .and_then(|window| window.theme().ok())
}

/// Swaps the window icon when the taskbar switches between light and dark.
///
/// The tray keeps the dark-plate app icon in both themes, so only the window
/// mark needs to follow the taskbar.
///
/// `last_theme` is the theme whose icons are already applied, not the current
/// theme. Passing a freshly-read current theme makes the function a no-op.
pub fn sync_brand_icons<R: Runtime>(app: &AppHandle<R>, last_theme: &mut Option<Theme>) {
    let Some(theme) = current_theme(app) else {
        return;
    };
    if *last_theme == Some(theme) {
        return;
    }
    let applied = app
        .get_webview_window("main")
        .and_then(|window| {
            let icon = window_icon_for_theme(Some(theme))?;
            Some(window.set_icon(icon).is_ok())
        })
        .unwrap_or(false);
    if applied {
        *last_theme = Some(theme);
    }
}

pub fn setup_tray(app: &App, language: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(mut stored) = TRAY_LANGUAGE.lock() {
        *stored = language.to_owned();
    }
    let menu = build_menu(app, language, None)?;
    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("声键")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "hide" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            "settings" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
                let _ = app.emit("navigate", "settings");
            }
            "update" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
                let _ = app.emit("navigate", "about");
            }
            "logs" => {
                if let Some(state) = app.try_state::<crate::state::AppState>() {
                    let _ = state.host.open_diagnostic_log_folder();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = tray_icon() {
        builder = builder.icon(icon);
    } else if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

pub fn set_tray_language(app: &AppHandle, language: &str) -> Result<(), String> {
    if let Ok(mut stored) = TRAY_LANGUAGE.lock() {
        *stored = language.to_owned();
    }
    refresh_tray(app)
}

pub fn set_tray_update(app: &AppHandle, version: Option<String>) -> Result<(), String> {
    if let Ok(mut stored) = TRAY_UPDATE_VERSION.lock() {
        *stored = version.filter(|value| !value.is_empty());
    }
    refresh_tray(app)
}

/// Re-apply the theme-specific window icon immediately (called when the app
/// finishes switching light/dark theme). The tray icon never changes.
#[tauri::command]
pub fn sync_tray_icons(app: AppHandle) {
    let mut applied_theme = None;
    sync_brand_icons(&app, &mut applied_theme);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_labels_and_tooltip_are_localized() {
        assert_eq!(update_label("zh", "v0.3.0"), "有新版本：v0.3.0");
        assert_eq!(update_label("en", "v0.3.0"), "Update available: v0.3.0");
        assert_eq!(
            tray_tooltip("zh", None, None),
            "声键
按键方案：默认
遥控器：未连接
输入法：未选择
电量：--"
        );
        let status = TrayStatus {
            profile_name: "默认".to_owned(),
            remote_name: "小米蓝牙语音遥控器 2 Pro".to_owned(),
            input_method: "微信输入法".to_owned(),
            battery_level: Some(65),
        };
        assert_eq!(
            tray_tooltip("zh", None, Some(&status)),
            "声键
按键方案：默认
遥控器：小米蓝牙语音遥控器 2 Pro
输入法：微信输入法
电量：65%"
        );
        assert_eq!(
            tray_tooltip("en", None, Some(&status)),
            "声键
Button profile: 默认
Remote: 小米蓝牙语音遥控器 2 Pro
Input method: 微信输入法
Battery: 65%"
        );
        assert!(tray_tooltip("zh", Some("v0.3.0"), Some(&status))
            .ends_with("
有新版本：v0.3.0"));
    }

    #[test]
    fn brand_tray_and_window_icons_decode_for_both_taskbar_themes() {
        assert!(tray_icon().is_some(), "tray icon must decode");
        for theme in [Theme::Dark, Theme::Light] {
            assert!(
                window_icon_for_theme(Some(theme)).is_some(),
                "window icon must decode for {theme:?}"
            );
        }
    }
}
