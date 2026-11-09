use std::sync::Mutex;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{App, AppHandle, Emitter, Manager, Runtime, Theme};

static TRAY_LANGUAGE: Mutex<String> = Mutex::new(String::new());
static TRAY_UPDATE_VERSION: Mutex<Option<String>> = Mutex::new(None);

/// Bright pine tray glyph for dark taskbars (brand: tray-32-color).
const TRAY_ICON_FOR_DARK_TASKBAR: &[u8] = include_bytes!("../icons/tray-32-color.png");
/// Ink tray glyph for light taskbars (brand: tray-32-ink).
const TRAY_ICON_FOR_LIGHT_TASKBAR: &[u8] = include_bytes!("../icons/tray-32-ink.png");
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

fn tray_tooltip(language: &str, update_version: Option<&str>) -> String {
    match update_version {
        Some(version) if language == "en" => {
            format!("声键 · Update available: {version}")
        }
        Some(version) => format!("声键 · 有新版本：{version}"),
        None => "声键".to_owned(),
    }
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
    let tooltip = tray_tooltip(&language, update_version.as_deref());
    tray.set_tooltip(Some(tooltip.as_str()))
        .map_err(|error| error.to_string())
}

/// Picks the tray glyph that stays readable on the current taskbar.
///
/// The bright pine mark is the brand default on dark surfaces; light taskbars
/// get the ink variant for maximum contrast on light grey.
fn tray_icon_for_theme(theme: Option<Theme>) -> Option<Image<'static>> {
    let bytes = match theme {
        Some(Theme::Light) => TRAY_ICON_FOR_LIGHT_TASKBAR,
        // `Theme` is non-exhaustive; dark is the safe default for the bright glyph.
        _ => TRAY_ICON_FOR_DARK_TASKBAR,
    };
    Image::from_bytes(bytes).ok()
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

/// Swaps the tray and window icons when the taskbar switches between light and dark.
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
    let mut applied = false;
    if let Some(tray) = app.tray_by_id("main-tray") {
        if let Some(icon) = tray_icon_for_theme(Some(theme)) {
            applied = tray.set_icon(Some(icon)).is_ok() || applied;
        }
    }
    if let Some(window) = app.get_webview_window("main") {
        if let Some(icon) = window_icon_for_theme(Some(theme)) {
            applied = window.set_icon(icon).is_ok() || applied;
        }
    }
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
    let theme = app
        .get_webview_window("main")
        .and_then(|window| window.theme().ok());
    if let Some(icon) = tray_icon_for_theme(theme) {
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

/// Re-apply the theme-specific tray/window icons immediately (called when the
/// app finishes switching light/dark theme).
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
            tray_tooltip("zh", Some("v0.3.0")),
            "声键 · 有新版本：v0.3.0"
        );
        assert_eq!(tray_tooltip("zh", None), "声键");
    }

    #[test]
    fn brand_tray_and_window_icons_decode_for_both_taskbar_themes() {
        for theme in [Theme::Dark, Theme::Light] {
            assert!(
                tray_icon_for_theme(Some(theme)).is_some(),
                "tray icon must decode for {theme:?}"
            );
            assert!(
                window_icon_for_theme(Some(theme)).is_some(),
                "window icon must decode for {theme:?}"
            );
        }
    }
}
