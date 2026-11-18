//! HID 直读探针：绕开 Raw Input，直接用 `CreateFile` + `ReadFile` 读遥控器的 HID 输入报告。
//!
//! 背景（2026-11-18）：这台 Chromecast Remote 的按键在 Windows 上既不产生 Raw Input
//! 事件，也不触发原生动作（按音量+ 音量条不弹）。另外两个开源项目（MoteDeck、
//! vibe-mote）都往里注入 Frida 才拿到报告，说明报告确实存在、只是 HID 映射层不认。
//! 本探针先验证最便宜的一条：**用户态直接打开集合读报告**，不注入、不提权。
//!
//! 做法：
//!   1. 用 Raw Input 的 `RIDI_DEVICENAME` 拿到每个 HID 集合的接口路径
//!      （那就是 `CreateFile` 要的路径）；
//!   2. 对目标设备（默认按 VID_18D1 & PID_9450 匹配）的每个集合尝试
//!      `CreateFile(GENERIC_READ, FILE_SHARE_READ|WRITE)`；
//!   3. 每个成功打开的句柄起一个读线程做阻塞 `ReadFile`，把长度与字节落盘；
//!   4. `--seconds` 到点直接退出进程（阻塞的读线程随之结束，不需要优雅取消）。
//!
//! 用法：
//!   target\debug\examples\hid_read_probe.exe --seconds 60 --out .dsh-ops\hid-read.txt
//!   （可选 --vid 18d1 --pid 9450；--all 连非目标设备一起试）

use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::UI::Input::{
    GetRawInputDeviceInfoW, GetRawInputDeviceList, RAWINPUTDEVICELIST, RIDI_DEVICENAME,
};

struct ProbeLog {
    file: Mutex<std::fs::File>,
    start: Instant,
}

static LOG: OnceLock<ProbeLog> = OnceLock::new();
static TOTAL_REPORTS: AtomicU64 = AtomicU64::new(0);

fn emit(text: &str) {
    let Some(log) = LOG.get() else { return };
    let t = log.start.elapsed().as_secs_f64();
    let line = format!("t={t:>7.3}  {text}");
    println!("{line}");
    if let Ok(mut file) = log.file.lock() {
        let _ = writeln!(file, "{line}");
        let _ = file.flush();
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn device_name(handle: HANDLE) -> Option<String> {
    let mut length: u32 = 0;
    let rc = unsafe { GetRawInputDeviceInfoW(Some(handle), RIDI_DEVICENAME, None, &mut length) };
    if rc == u32::MAX || length == 0 {
        return None;
    }
    let mut buffer = vec![0u16; length as usize];
    let rc = unsafe {
        GetRawInputDeviceInfoW(
            Some(handle),
            RIDI_DEVICENAME,
            Some(buffer.as_mut_ptr() as *mut core::ffi::c_void),
            &mut length,
        )
    };
    if rc == u32::MAX {
        return None;
    }
    let taken = (length.saturating_sub(1) as usize).min(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..taken]))
}

fn collection_tag(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    match lower.find("col") {
        Some(position) => name[position..]
            .chars()
            .take(5)
            .collect::<String>()
            .trim_end_matches('#')
            .to_string(),
        None => "?".to_string(),
    }
}

/// 打开一个 HID 集合，返回句柄；失败则返回 Win32 错误码。
fn open_collection(name: &str) -> Result<HANDLE, u32> {
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    };
    match handle {
        Ok(handle) if !handle.is_invalid() => Ok(handle),
        Ok(_) => Err(0),
        Err(error) => Err(error.code().0 as u32),
    }
}

fn spawn_reader(tag: String, handle: HANDLE) -> std::thread::JoinHandle<()> {
    // HANDLE 不是 Send：转成 usize 再进线程，线程里重建。
    let handle_value = handle.0 as usize;
    std::thread::spawn(move || {
        let mut buffer = [0u8; 128];
        let mut last: Option<Vec<u8>> = None;
        loop {
            let mut read: u32 = 0;
            let result = unsafe {
                ReadFile(
                    HANDLE(handle_value as *mut core::ffi::c_void),
                    Some(&mut buffer[..]),
                    Some(&mut read),
                    None,
                )
            };
            if result.is_err() {
                let error = unsafe { windows::Win32::Foundation::GetLastError() };
                emit(&format!("[read {tag}] 读失败，停止 err={}", error.0));
                return;
            }
            if read == 0 {
                continue;
            }
            let bytes = buffer[..read as usize].to_vec();
            TOTAL_REPORTS.fetch_add(1, Ordering::Relaxed);
            if last.as_ref() != Some(&bytes) {
                emit(&format!("[REPORT {tag}] len={read} b=[{}]", hex(&bytes)));
                last = Some(bytes);
            }
        }
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut seconds = 60u64;
    let mut out: Option<String> = None;
    let mut vid = "18d1".to_string();
    let mut pid = "9450".to_string();
    let mut all = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--seconds" => {
                seconds = args
                    .get(index + 1)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60);
                index += 2;
            }
            "--out" => {
                out = args.get(index + 1).cloned();
                index += 2;
            }
            "--vid" => {
                vid = args
                    .get(index + 1)
                    .cloned()
                    .unwrap_or(vid)
                    .to_ascii_lowercase();
                index += 2;
            }
            "--pid" => {
                pid = args
                    .get(index + 1)
                    .cloned()
                    .unwrap_or(pid)
                    .to_ascii_lowercase();
                index += 2;
            }
            "--all" => {
                all = true;
                index += 1;
            }
            _ => index += 1,
        }
    }

    let path = out.unwrap_or_else(|| "hid-read-probe.txt".to_string());
    let file = match std::fs::File::create(&path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("无法创建输出文件 {path}: {error}");
            return;
        }
    };
    let _ = LOG.set(ProbeLog {
        file: Mutex::new(file),
        start: Instant::now(),
    });

    std::panic::set_hook(Box::new(|info| {
        let text = format!("PANIC: {info}");
        let _ = std::fs::write("hid-read-probe-panic.txt", &text);
        let _ = emit(&text);
    }));

    emit(&format!(
        "=== HID 直读探针 vid={vid} pid={pid} 采集 {seconds}s（不注入、不提权） ==="
    ));

    // 1) 枚举所有 Raw Input 设备，取它们的接口路径
    let mut count: u32 = 0;
    let entry = std::mem::size_of::<RAWINPUTDEVICELIST>() as u32;
    if unsafe { GetRawInputDeviceList(None, &mut count, entry) } == u32::MAX || count == 0 {
        emit("GetRawInputDeviceList 计数失败");
        return;
    }
    let mut list = vec![RAWINPUTDEVICELIST::default(); count as usize];
    if unsafe { GetRawInputDeviceList(Some(list.as_mut_ptr()), &mut count, entry) } == u32::MAX {
        emit("GetRawInputDeviceList 失败");
        return;
    }
    emit(&format!("Raw Input 设备共 {count} 个"));

    // 2) 挑出目标设备的集合，逐个 CreateFile
    let mut opened = 0usize;
    let mut threads = Vec::new();
    let mut seen_collections = 0usize;
    for item in &list[..count as usize] {
        let Some(name) = device_name(item.hDevice) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        let target = lower.contains(&format!("vid&02{vid}_pid&{pid}"))
            || lower.contains(&format!("vid&01{vid}_pid&{pid}"));
        if !target && !all {
            continue;
        }
        if target {
            seen_collections += 1;
        }
        let tag = format!(
            "{}{}",
            collection_tag(&name),
            if target { "" } else { "（非目标）" }
        );
        match open_collection(&name) {
            Ok(handle) => {
                emit(&format!("[open {tag}] OK  CreateFile(GENERIC_READ) 成功"));
                opened += 1;
                threads.push(spawn_reader(tag, handle));
            }
            Err(code) => emit(&format!("[open {tag}] 失败 err=0x{code:08X}（{code}）")),
        }
    }
    emit(&format!(
        "目标设备集合 {seen_collections} 个，成功打开 {opened} 个"
    ));
    if opened == 0 {
        emit("结论：一个都打不开 —— 用户态直读这条路走不通，需要走驱动层旁路（Frida）");
        return;
    }

    emit("---------------------------------------------------------------");
    emit("已开始读报告。请按遥控器：音量+ / 音量- / 返回 / 主页 / 确定，每个间隔 2 秒。");
    emit("---------------------------------------------------------------");

    std::thread::sleep(Duration::from_secs(seconds));

    let total = TOTAL_REPORTS.load(Ordering::Relaxed);
    emit(&format!(
        "=== 采集结束 {seconds}s，共收到 {total} 条报告 ==="
    ));
    emit(if total > 0 {
        "结论：用户态能直接读到 HID 报告 —— 不需要 Frida，也不需要提权"
    } else {
        "结论：句柄能打开但一条报告都没读到 —— 直读这条路不通，得走驱动层旁路"
    });
    let _ = threads; // 阻塞中的读线程随进程退出结束
    std::process::exit(0);
}
