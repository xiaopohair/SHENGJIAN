//! One-time-installed elevated RC003 helper task control.
//!
//! The task itself is registered with admin rights by `helper/hid-host`
//! (`--install-task`). This module only starts/stops it; no elevation is
//! requested at runtime.

use std::path::PathBuf;
use std::process::Command;
use tauri::Manager;

const TASK_NAME: &str = "Shengjian HID Host";

/// 改名前的任务名（2026-11-17：`rc003-helper` → `hid-host`）。
///
/// 只用于「装过旧版」的机器上判断服务在不在：看到旧任务也算已安装，
/// 用户点「更新」时新版助手会把它删掉（迁移在 `--install-task` 里）。
const LEGACY_TASK_NAME: &str = "Shengjian RC003 Helper";

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

/// `schtasks` 的退出码；连进程都起不来（找不到程序）时返回 `None`。
fn run_schtasks_code(args: &[&str]) -> Option<i32> {
    let mut command = Command::new("schtasks");
    command.args(args);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command.status().ok().and_then(|status| status.code())
}

pub fn start_helper_task() -> bool {
    run_schtasks(&["/Run", "/TN", TASK_NAME])
}

pub fn stop_helper_task() -> bool {
    run_schtasks(&["/End", "/TN", TASK_NAME])
}

/// 重启全按键支持服务：`/End` 当前实例，再 `/Run` 拉起新的。**不需要管理员**。
///
/// 2026-11-16 现场：宿主（WUDFHost）是共享进程时，助手会拒绝自我刷新
/// （日志 `[AGENT-REFRESH] reason=shared_host`），换了 agent 之后会**静默**跑在
/// 旧实例上，只能人工用管理员权限 taskkill 宿主。实测这对 `schtasks` 动作就能让
/// 助手重新扫描宿主并注入新 agent——设置页那个「重启宿主」按钮走的就是这里。
///
/// `/End` 对「本来没在跑」的任务返回非 0，不算失败；判据只看 `/Run` 是否成功。
pub fn restart_helper_task() -> Result<(), String> {
    let stop_code = run_schtasks_code(&["/End", "/TN", TASK_NAME]);
    // `/End` 只是"请求结束"，任务实例收尾要一点时间；立刻 `/Run` 可能撞上
    // "任务正在运行"。重试几次（最多约 1.5 秒），避免让用户看到假失败。
    let mut start_code = None;
    for attempt in 0..6 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        start_code = run_schtasks_code(&["/Run", "/TN", TASK_NAME]);
        if start_code == Some(0) {
            break;
        }
    }
    restart_outcome(stop_code, start_code)
}

/// 把两条 `schtasks` 退出码翻成可操作的话（纯函数，便于单测）。
fn restart_outcome(stop_code: Option<i32>, start_code: Option<i32>) -> Result<(), String> {
    match start_code {
        Some(0) => Ok(()),
        Some(code) => Err(format!(
            "schtasks /Run 失败（exit={code}；/End exit={stop_code:?}）。请确认计划任务「{TASK_NAME}」仍存在，必要时先在设置页重新安装服务。"
        )),
        None => Err("无法执行 schtasks：请确认系统 PATH 正常。".to_owned()),
    }
}

pub fn task_installed() -> bool {
    run_schtasks(&["/Query", "/TN", TASK_NAME])
        || run_schtasks(&["/Query", "/TN", LEGACY_TASK_NAME])
}

/// 以管理员身份启动助手（UAC）。
///
/// 用 `ShellExecuteW(..., "runas", ...)` 而不是"隐藏 PowerShell 里 Start-Process -Verb RunAs"：
/// 后者在部分环境里根本弹不出 UAC（2026-11-17 实测：用户连续两次都没看到窗口），
/// 前者是资源管理器那条标准提权路径，MoteDeck 也是这么做的。
#[cfg(windows)]
fn run_elevated(exe: &std::path::Path, arguments: &str) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(
            hwnd: *mut core::ffi::c_void,
            operation: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show: i32,
        ) -> isize;
    }
    const SW_SHOWNORMAL: i32 = 1;

    fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
        value.encode_wide().chain(std::iter::once(0)).collect()
    }

    let verb = wide(std::ffi::OsStr::new("runas"));
    let file = wide(exe.as_os_str());
    let params = wide(std::ffi::OsStr::new(arguments));
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            params.as_ptr(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecute 的约定：返回值 > 32 才是成功。
    if result as usize > 32 {
        Ok(())
    } else {
        Err(format!("elevation request failed (code {result})"))
    }
}

#[cfg(not(windows))]
fn run_elevated(_exe: &std::path::Path, _arguments: &str) -> Result<(), String> {
    Err("helper task management is Windows-only".to_owned())
}

fn resource_candidates(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let Ok(resource_dir) = app.path().resource_dir() else {
        return Vec::new();
    };
    vec![
        resource_dir.join("resources/hid-host/shengjian-helper.exe"),
        resource_dir.join("rc003/shengjian-helper.exe"),
    ]
}

/// 迁移（改名）：清掉旧布局目录 `resources/rc003`。
///
/// 旧版助手就是从那个目录跑起来的，进程还在时删不掉——所以只做 best effort，
/// 失败无所谓（下次启动再试），关键迁移动作在 `--install-task` 里（删旧任务）。
pub fn cleanup_legacy_layout(app: &tauri::AppHandle) {
    if let Ok(dir) = app.path().resource_dir() {
        let legacy = dir.join("resources/rc003");
        if legacy.is_dir() {
            let _ = std::fs::remove_dir_all(&legacy);
        }
    }
}

pub fn bundled_helper_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    resource_candidates(app)
        .into_iter()
        .find(|path| path.is_file())
}

pub fn request_helper_task_install(app: &tauri::AppHandle) -> Result<(), String> {
    let helper =
        bundled_helper_path(app).ok_or_else(|| "bundled HID host helper is missing".to_owned())?;
    run_elevated(&helper, "--install-task")
}

/// 提权移除计划任务（`helper --remove-task`）。和安装一样通过 RunAs 弹一次 UAC。
pub fn request_helper_task_remove(app: &tauri::AppHandle) -> Result<(), String> {
    let helper =
        bundled_helper_path(app).ok_or_else(|| "bundled HID host helper is missing".to_owned())?;
    run_elevated(&helper, "--remove-task")
}

#[cfg(test)]
mod tests {
    use super::restart_outcome;

    #[test]
    fn restart_succeeds_only_when_run_exits_zero() {
        assert!(restart_outcome(Some(0), Some(0)).is_ok());
        // 任务本来没在跑：`/End` 返回非 0 不是失败，只要 `/Run` 起来就算成功。
        assert!(restart_outcome(Some(1), Some(0)).is_ok());
        assert!(restart_outcome(None, Some(0)).is_ok());
    }

    #[test]
    fn restart_error_carries_both_exit_codes() {
        let failed = restart_outcome(Some(1), Some(2)).expect_err("run failure must surface");
        assert!(failed.contains("exit=2"), "{failed}");
        assert!(failed.contains("Some(1)"), "{failed}");
        assert!(failed.contains("重新安装"), "{failed}");

        let missing = restart_outcome(None, None).expect_err("missing schtasks must surface");
        assert!(missing.contains("PATH"), "{missing}");
    }
}
