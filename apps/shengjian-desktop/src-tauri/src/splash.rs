use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, State};

/// Keep the splash on screen briefly even when the UI hydrates very fast, so
/// the brand screen reads as intentional instead of a one-frame flicker.
pub const MIN_SPLASH_DURATION: Duration = Duration::from_millis(800);

/// If the frontend never reports readiness (JavaScript error, blocked IPC, or a
/// regression in the boot sequence), reveal the main window anyway.
const SPLASH_TIMEOUT: Duration = Duration::from_secs(10);

pub struct StartupClock {
    started_at: Instant,
}

impl Default for StartupClock {
    fn default() -> Self {
        Self {
            started_at: Instant::now(),
        }
    }
}

/// Show the main window and destroy the splash. `destroy` is used instead of
/// `close` so the splash close guard cannot intercept the handoff.
pub fn reveal_main(app: &AppHandle) {
    let Some(main) = app.get_webview_window("main") else {
        // A second launch or timeout can fire before setup creates the main
        // window. Keep the splash alive instead of leaving no window at all.
        return;
    };
    let _ = main.show();
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.destroy();
    }
    let _ = main.set_focus();
}

/// 启动时是否弹出主窗口。
///
/// 设置页「启动与窗口 > 启动后隐藏到系统托盘」是这个值的**反向**开关：
/// 用户打开它即写入 `open_window_at_launch = false`，普通启动只驻留托盘，
/// 之后由托盘菜单（「显示主窗口」）或再次启动应用打开主窗口。
///
/// 读取失败时按「弹出」处理：宁可多显示一次窗口，也不要让应用悄悄留在托盘里。
pub fn should_open_at_launch(app: &AppHandle) -> bool {
    app.try_state::<crate::state::AppState>()
        .and_then(|state| state.engine.settings().load().ok())
        .map(|settings| settings.open_window_at_launch)
        .unwrap_or(true)
}

/// 隐藏启动：只收起启动画面，不显示主窗口（应用驻留托盘）。
pub fn dismiss_splash(app: &AppHandle) {
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.destroy();
    }
}

pub fn schedule_fallback(app: AppHandle) {
    let _ = std::thread::Builder::new()
        .name("shengjian-splash-fallback".to_owned())
        .spawn(move || {
            std::thread::sleep(SPLASH_TIMEOUT);
            if app.get_webview_window("splash").is_some() {
                if should_open_at_launch(&app) {
                    reveal_main(&app);
                } else {
                    dismiss_splash(&app);
                }
            }
        });
}

#[tauri::command]
pub async fn finish_startup(app: AppHandle, clock: State<'_, StartupClock>) -> Result<(), String> {
    let remaining = min_splash_remaining(clock.started_at.elapsed());
    if !remaining.is_zero() {
        let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(remaining)).await;
    }
    if should_open_at_launch(&app) {
        reveal_main(&app);
    } else {
        dismiss_splash(&app);
    }
    Ok(())
}

fn min_splash_remaining(elapsed: Duration) -> Duration {
    MIN_SPLASH_DURATION.saturating_sub(elapsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splash_remaining_is_bounded_by_minimum_duration() {
        assert_eq!(min_splash_remaining(Duration::ZERO), MIN_SPLASH_DURATION);
        assert_eq!(
            min_splash_remaining(Duration::from_millis(300)),
            Duration::from_millis(500)
        );
        assert_eq!(min_splash_remaining(Duration::from_secs(5)), Duration::ZERO);
    }
}
