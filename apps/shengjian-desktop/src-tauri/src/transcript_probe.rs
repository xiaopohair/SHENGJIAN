//! 设备测试面板用的"临时识别内容"探针。
//!
//! 默认情况下语音捕获只在用户开启「语音记录」时才启动。测试面板需要"不管设置开没开
//! 都能看到识别内容"，但测试不该往历史里写记录，所以：
//!
//! - 面板打开期间强制开启捕获，并把型号/文本留在**内存槽**里；
//! - 启动时安装的 sink 检测到探针激活就只存内存、不落盘、不广播；
//! - 面板关闭时恢复用户原来的捕获开关（设置文件从未被改写）。
use crate::host::TranscriptRecord;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

struct Probe {
    active: AtomicBool,
    previous_capture: AtomicBool,
    slot: Mutex<Option<TranscriptRecord>>,
}

static PROBE: Probe = Probe {
    active: AtomicBool::new(false),
    previous_capture: AtomicBool::new(false),
    slot: Mutex::new(None),
};

pub fn is_active() -> bool {
    PROBE.active.load(Ordering::Relaxed)
}

/// 开始探测；`previous_capture` 是用户当前的「语音记录」开关，结束时恢复它。
pub fn begin(previous_capture: bool) {
    PROBE
        .previous_capture
        .store(previous_capture, Ordering::Relaxed);
    if let Ok(mut slot) = PROBE.slot.lock() {
        *slot = None;
    }
    PROBE.active.store(true, Ordering::Relaxed);
}

/// 结束探测，返回之前记下的捕获开关。
pub fn end() -> bool {
    PROBE.active.store(false, Ordering::Relaxed);
    PROBE.previous_capture.load(Ordering::Relaxed)
}

pub fn store(record: TranscriptRecord) {
    if let Ok(mut slot) = PROBE.slot.lock() {
        *slot = Some(record);
    }
}

pub fn take() -> Option<TranscriptRecord> {
    PROBE.slot.lock().ok().and_then(|mut slot| slot.take())
}
