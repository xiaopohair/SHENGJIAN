//! 应用级显示缩放。
//!
//! Tauri 主窗口在启动和 DPI 变化时把 scale factor 写到这里；原生浮层只读，
//! 不再自己调用 GetDpiForSystem，避免每个原生窗口各算一套。

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// 千分之一为单位，1000 = 100%。
static SCALE_PER_MILLE: AtomicU32 = AtomicU32::new(1000);
/// 当前应用主题是否为深色（由 Tauri 主窗口上报）。
static DARK_THEME: AtomicBool = AtomicBool::new(true);

pub fn set_scale_factor(scale: f64) {
    let clamped = scale.clamp(0.5, 4.0);
    SCALE_PER_MILLE.store((clamped * 1000.0).round() as u32, Ordering::SeqCst);
}

pub fn scale_factor() -> f64 {
    SCALE_PER_MILLE.load(Ordering::Relaxed) as f64 / 1000.0
}

pub fn scale_px(value: i32) -> i32 {
    (value as f64 * scale_factor()).round() as i32
}

pub fn set_dark_theme(dark: bool) {
    DARK_THEME.store(dark, Ordering::SeqCst);
}

pub fn dark_theme() -> bool {
    DARK_THEME.load(Ordering::Relaxed)
}
