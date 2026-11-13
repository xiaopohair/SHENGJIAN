//! Loads the editable JSON catalogs shipped in the installer resources.
//!
//! The files are copied from `<repo>/config/` into `src-tauri/resources/config/`
//! at build time (`scripts/prepare-config.ps1`) and bundled by Tauri. On startup we
//! read the packaged copy; if it is missing or invalid we fall back to the copy
//! embedded in `shengjian-domain` so `tauri dev` and tests still work.

use std::path::Path;
use tauri::Manager;

pub fn initialize(app: &tauri::AppHandle) {
    let resource_dir = app.path().resource_dir().ok();
    install_catalog(
        resource_dir.as_deref(),
        "app-presets.json",
        "app preset catalog",
        shengjian_domain::app_catalog::install_from_json,
        shengjian_domain::app_catalog::install_defaults,
    );
    install_catalog(
        resource_dir.as_deref(),
        "voice-tools.json",
        "voice tool catalog",
        shengjian_domain::voice_tools::install_from_json,
        shengjian_domain::voice_tools::install_defaults,
    );
    // 推荐宏必须先于推荐方案加载：推荐方案的 bindings 可能引用 `run_macro`，
    // 方案校验要用推荐宏目录判断引用是否存在。
    install_catalog(
        resource_dir.as_deref(),
        "recommended-macros.json",
        "catalog recommended_macros",
        shengjian_domain::recommended_macros::install_from_json,
        shengjian_domain::recommended_macros::install_defaults,
    );
    install_catalog(
        resource_dir.as_deref(),
        "button-profiles.json",
        "catalog button_presets",
        shengjian_domain::button_presets::install_from_json,
        shengjian_domain::button_presets::install_defaults,
    );
    // 输入法的 TSF 身份（固定 GUID / 显示名发现）来自配置目录；解析失败时
    // 保持内置常量，不阻断启动。
    for line in shengjian_windows::install_ime_profiles_from_catalog() {
        crate::platform::note(format!("catalog {line}"));
    }
}

fn install_catalog(
    resource_dir: Option<&Path>,
    file_name: &str,
    label: &str,
    install: fn(&str) -> Result<(), String>,
    install_defaults: fn() -> Result<(), String>,
) {
    if let Some(base) = resource_dir {
        for dir in [base.join("resources/config"), base.join("config")] {
            let path = dir.join(file_name);
            let Ok(contents) = std::fs::read_to_string(&path) else {
                continue;
            };
            match install(&contents) {
                Ok(()) => {
                    eprintln!("{label}: loaded {}", path.display());
                    return;
                }
                Err(error) => {
                    eprintln!("{label}: invalid {}: {error}", path.display());
                }
            }
        }
    }
    match install_defaults() {
        Ok(()) => eprintln!("{label}: using embedded fallback"),
        Err(error) => eprintln!("{label}: embedded fallback failed: {error}"),
    }
}
