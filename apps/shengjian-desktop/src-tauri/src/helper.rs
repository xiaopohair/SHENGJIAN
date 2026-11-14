//! One-time-installed elevated RC003 helper task control.
//!
//! The task itself is registered with admin rights by `helper/rc003`
//! (`--install-task`). This module only starts/stops it; no elevation is
//! requested at runtime.

use std::path::PathBuf;
use std::process::Command;
use tauri::Manager;

const TASK_NAME: &str = "Shengjian RC003 Helper";

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn run_schtasks(args: &[&str]) -> bool {
    let mut command = Command::new("schtasks");
    command.args(args);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn start_helper_task() -> bool {
    run_schtasks(&["/Run", "/TN", TASK_NAME])
}

pub fn stop_helper_task() -> bool {
    run_schtasks(&["/End", "/TN", TASK_NAME])
}

pub fn task_installed() -> bool {
    run_schtasks(&["/Query", "/TN", TASK_NAME])
}

fn resource_candidates(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let Ok(resource_dir) = app.path().resource_dir() else {
        return Vec::new();
    };
    vec![
        resource_dir.join("resources/rc003/shengjian-helper.exe"),
        resource_dir.join("rc003/shengjian-helper.exe"),
    ]
}

pub fn bundled_helper_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    resource_candidates(app)
        .into_iter()
        .find(|path| path.is_file())
}

pub fn request_helper_task_install(app: &tauri::AppHandle) -> Result<(), String> {
    let helper =
        bundled_helper_path(app).ok_or_else(|| "bundled RC003 helper is missing".to_owned())?;
    let helper = helper.to_string_lossy().replace('\'', "''");
    let script =
        format!("Start-Process -FilePath '{helper}' -ArgumentList '--install-task' -Verb RunAs");
    let mut command = Command::new("powershell");
    command.args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script]);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("cannot start elevated helper installer: {error}"))
}

/// 提权移除计划任务（`helper --remove-task`）。和安装一样通过 RunAs 弹一次 UAC。
pub fn request_helper_task_remove(app: &tauri::AppHandle) -> Result<(), String> {
    let helper =
        bundled_helper_path(app).ok_or_else(|| "bundled RC003 helper is missing".to_owned())?;
    let helper = helper.to_string_lossy().replace('\'', "''");
    let script =
        format!("Start-Process -FilePath '{helper}' -ArgumentList '--remove-task' -Verb RunAs");
    let mut command = Command::new("powershell");
    command.args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script]);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("cannot start elevated helper uninstaller: {error}"))
}
