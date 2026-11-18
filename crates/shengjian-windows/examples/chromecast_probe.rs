//! Chromecast Remote（Google TV 遥控器）合并探针。
//!
//! 目的：回答"这台遥控器能不能进声键"，并把证据落成一份可复核的时间线。
//! 一次运行同时开两条采集通道：
//!
//!   A. GATT 通知：电池 `0x2A19`、厂商 `0xAE42`、Google 私有 `D343BFC5`、
//!      ATVV 控制 `AB5E0004`（阳性对照：按语音键应看到通知）。
//!   B. Raw Input：键盘页（含 Consumer 映射后的 VK）、鼠标页、HID 原始上报。
//!      厂商自定义集合（Col04 / Col05）的原始报告也在这一路里。
//!
//! 只读：GATT 侧只做读取与 CCCD 订阅（Notify），**不写任何厂商特征值**；
//! Raw Input 侧只旁听（RIDEV_INPUTSINK），不拦截、不吞键。
//!
//! 注意：按键仍然照常投递给前台应用（音量键会调系统音量、连接键可能唤起应用），
//! 这是 HID 设备的系统行为，探针不改变它。
//!
//! 用法：
//!   cargo run --release -p shengjian-windows --example chromecast_probe -- \
//!       --mac A7:60:10:14:3D:23 --seconds 120 --out chromecast-probe.txt

use std::future::IntoFuture;
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use windows::core::{GUID, Interface};
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristic, GattCharacteristicProperties,
    GattClientCharacteristicConfigurationDescriptorValue, GattCommunicationStatus,
    GattDeviceService, GattValueChangedEventArgs,
};
use windows::Devices::Bluetooth::{BluetoothCacheMode, BluetoothLEDevice};
use windows::Foundation::TypedEventHandler;
use windows::Storage::Streams::{DataReader, IBuffer};
use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use windows::Win32::UI::Input::{
    GetRawInputData, GetRawInputDeviceInfoW, GetRawInputDeviceList, RegisterRawInputDevices,
    HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTDEVICELIST, RIDEV_INPUTSINK, RIDI_DEVICEINFO,
    RIDI_DEVICENAME, RID_DEVICE_INFO, RID_INPUT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, PeekMessageW, RegisterClassW,
    TranslateMessage, HWND_MESSAGE, MSG, PM_NOREMOVE, WM_APP, WM_INPUT, WM_KEYUP, WM_QUIT,
    WM_SYSKEYUP, WNDCLASSW,
};

// ---------- UUID ----------

const ATVV_CONTROL: u128 = 0xab5e00045a214f05bc7daf01f617b664;
const GOOGLE_D343_NOTIFY: u128 = 0xd343bfc55a214f05bc7daf01f617b664;
const VENDOR_AE42: u128 = 0x0000ae4200001000800000805f9b34fb;
const BATTERY_LEVEL: u128 = 0x00002a1900001000800000805f9b34fb;
const SERVICE_CHANGED: u128 = 0x00002a0500001000800000805f9b34fb;

const WM_QUIT_PROBE: u32 = WM_APP + 0x91;

// ---------- 输出 ----------

struct ProbeLog {
    file: Mutex<std::fs::File>,
    start: Instant,
}

static LOG: OnceLock<ProbeLog> = OnceLock::new();

fn emit(text: &str) {
    let Some(log) = LOG.get() else { return };
    let t = log.start.elapsed().as_secs_f64();
    let line = format!("t={t:>8.3}  {text}");
    println!("{line}");
    if let Ok(mut file) = log.file.lock() {
        let _ = writeln!(file, "{line}");
        let _ = file.flush();
    }
}

// ---------- 工具 ----------

fn buffer_to_vec(buffer: &IBuffer) -> windows::core::Result<Vec<u8>> {
    let length = buffer.Length()? as usize;
    let reader = DataReader::FromBuffer(buffer)?;
    let mut bytes = vec![0u8; length];
    reader.ReadBytes(&mut bytes)?;
    Ok(bytes)
}

fn hex(bytes: &[u8], max: usize) -> String {
    bytes
        .iter()
        .take(max)
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn short_uuid(uuid: &GUID) -> String {
    let v = uuid.to_u128();
    const BASE_TAIL: u128 = 0x0000_1000_8000_0080_5f9b_34fb;
    const TAIL_MASK: u128 = (1u128 << 96) - 1;
    if (v >> 112) == 0 && (v & TAIL_MASK) == BASE_TAIL {
        format!("0x{:04X}", (v >> 96) as u16)
    } else {
        format!("{uuid:?}")
    }
}

fn label(uuid: &GUID) -> &'static str {
    match uuid.to_u128() {
        ATVV_CONTROL => "ATVV_CONTROL",
        GOOGLE_D343_NOTIFY => "GOOGLE_D343BFC5",
        VENDOR_AE42 => "VENDOR_AE42",
        BATTERY_LEVEL => "BATTERY_LEVEL",
        SERVICE_CHANGED => "SERVICE_CHANGED",
        _ => "",
    }
}

fn block<T, E: std::fmt::Debug>(op: windows::core::Result<E>) -> Option<T>
where
    E: IntoFuture<Output = windows::core::Result<T>>,
{
    match op {
        Ok(op) => match futures::executor::block_on(op.into_future()) {
            Ok(v) => Some(v),
            Err(error) => {
                emit(&format!("!! 异步调用失败: {error:?}"));
                None
            }
        },
        Err(error) => {
            emit(&format!("!! 调用失败: {error:?}"));
            None
        }
    }
}

fn parse_mac(raw: &str) -> Option<u64> {
    let cleaned: String = raw.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if cleaned.len() != 12 {
        return None;
    }
    u64::from_str_radix(&cleaned, 16).ok()
}

fn masked(address: u64) -> String {
    let hex = format!("{address:012X}");
    format!("{}:{}:**:**:**:**", &hex[0..2], &hex[2..4])
}

// ---------- Raw Input ----------

fn key_name(vk: u16) -> &'static str {
    match vk {
        0x08 => "VK_BACK",
        0x0D => "VK_RETURN",
        0x1B => "VK_ESCAPE",
        0x20 => "VK_SPACE",
        0x21 => "VK_PRIOR",
        0x22 => "VK_NEXT",
        0x24 => "VK_HOME",
        0x25 => "VK_LEFT",
        0x26 => "VK_UP",
        0x27 => "VK_RIGHT",
        0x28 => "VK_DOWN",
        0x2C => "VK_SNAPSHOT",
        0x2D => "VK_INSERT",
        0x2E => "VK_DELETE",
        0x5B => "VK_LWIN",
        0x70..=0x87 => "VK_F1..F24",
        0x90 => "VK_NUMLOCK",
        0xA6 => "VK_BROWSER_BACK",
        0xA7 => "VK_BROWSER_FORWARD",
        0xA8 => "VK_BROWSER_REFRESH",
        0xA9 => "VK_BROWSER_STOP",
        0xAA => "VK_BROWSER_SEARCH",
        0xAB => "VK_BROWSER_FAVORITES",
        0xAC => "VK_BROWSER_HOME",
        0xAD => "VK_VOLUME_MUTE",
        0xAE => "VK_VOLUME_DOWN",
        0xAF => "VK_VOLUME_UP",
        0xB0 => "VK_MEDIA_NEXT_TRACK",
        0xB1 => "VK_MEDIA_PREV_TRACK",
        0xB2 => "VK_MEDIA_STOP",
        0xB3 => "VK_MEDIA_PLAY_PAUSE",
        0xB4 => "VK_LAUNCH_MAIL",
        0xB5 => "VK_LAUNCH_MEDIA_SELECT",
        0xB6 => "VK_LAUNCH_APP1",
        0xB7 => "VK_LAUNCH_APP2",
        0x5D => "VK_APPS",
        0x5F => "VK_SLEEP",
        _ => "",
    }
}

// ---------- 设备过滤（只认 Chromecast Remote）----------
//
// RIDEV_INPUTSINK 注册的是「整页设备」，物理键盘和鼠标同样会投递进来。
// 2026-11-18 第一轮日志就被用户自己敲键盘的事件污染过，所以这里按设备接口
// 路径过滤：只有 VID_18D1&PID_9450（Google / Chromecast Remote）才算数。

static DEVICE_NAMES: OnceLock<Mutex<std::collections::HashMap<usize, String>>> = OnceLock::new();
static OTHER_DEVICES: OnceLock<Mutex<std::collections::HashSet<String>>> = OnceLock::new();
static OTHER_EVENTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static TARGET_COLLECTIONS: OnceLock<Mutex<Vec<(u16, u16)>>> = OnceLock::new();

fn device_name(handle: usize) -> Option<String> {
    let map = DEVICE_NAMES.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Some(name) = map.lock().ok()?.get(&handle) {
        return Some(name.clone());
    }
    let mut length: u32 = 0;
    let rc = unsafe {
        GetRawInputDeviceInfoW(
            Some(HANDLE(handle as *mut core::ffi::c_void)),
            RIDI_DEVICENAME,
            None,
            &mut length,
        )
    };
    if rc == u32::MAX || length == 0 {
        return None;
    }
    let mut buffer = vec![0u16; length as usize];
    let rc = unsafe {
        GetRawInputDeviceInfoW(
            Some(HANDLE(handle as *mut core::ffi::c_void)),
            RIDI_DEVICENAME,
            Some(buffer.as_mut_ptr() as *mut core::ffi::c_void),
            &mut length,
        )
    };
    if rc == u32::MAX {
        return None;
    }
    let taken = (length.saturating_sub(1) as usize).min(buffer.len());
    let text = String::from_utf16_lossy(&buffer[..taken]);
    if let Ok(mut map) = map.lock() {
        map.insert(handle, text.clone());
    }
    Some(text)
}

fn is_target(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("18d1") && lower.contains("9450")
}

/// 从设备路径里摘出集合名（`&Col01` 之类），方便区分键盘 / Consumer / 厂商集合。
fn collection_tag(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    if let Some(position) = lower.find("col") {
        let tail: String = name[position..].chars().take(5).collect();
        return format!(" [{}]", tail.trim_end_matches('#'));
    }
    String::new()
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // Windows 绑定层不允许 panic 穿过 FFI 边界（会直接 abort），这里兜住，
    // 让一次坏报告不至于整探针崩溃。
    let _ = std::panic::catch_unwind(|| unsafe {
        handle_raw_input(hwnd, msg, wparam, lparam);
    });
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

unsafe fn handle_raw_input(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) {
    if msg == WM_INPUT {
        // RAWINPUT 要求 8 字节对齐：用 u64 数组当存储，不能用 [u8; N]
        // （debug 版会以 misaligned pointer dereference 直接 panic）。
        let mut storage = [0u64; 128];
        let base = storage.as_mut_ptr() as *mut u8;
        let mut size = (storage.len() * 8) as u32;
        let got = GetRawInputData(
            HRAWINPUT(lparam.0 as *mut core::ffi::c_void),
            RID_INPUT,
            Some(base as *mut core::ffi::c_void),
            &mut size,
            std::mem::size_of::<windows::Win32::UI::Input::RAWINPUTHEADER>() as u32,
        );
        if got != u32::MAX {
            let buf = std::slice::from_raw_parts(base as *const u8, storage.len() * 8);
            let raw = &*(base as *const RAWINPUT);
            let handle = raw.header.hDevice.0 as usize;
            let name = device_name(handle).unwrap_or_default();
            if !is_target(&name) {
                OTHER_EVENTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let seen = OTHER_DEVICES.get_or_init(|| Mutex::new(std::collections::HashSet::new()));
                let mut should_log = false;
                if let Ok(mut set) = seen.lock() {
                    let key = if name.is_empty() {
                        format!("handle:{handle}")
                    } else {
                        name.clone()
                    };
                    if set.insert(key) {
                        should_log = true;
                    }
                }
                if should_log {
                    emit(&format!(
                        "[filter] 收到非目标设备（已忽略）：{}",
                        if name.is_empty() {
                            format!("handle={handle}")
                        } else {
                            name.clone()
                        }
                    ));
                }
                return;
            }
            let tag = collection_tag(&name);
            match raw.header.dwType {
                1 => {
                    // RIM_TYPEKEYBOARD
                    let keyboard = &raw.data.keyboard;
                    let message = keyboard.Message;
                    let direction = match message {
                        m if m == WM_KEYUP as u32 || m == WM_SYSKEYUP as u32 => "up",
                        _ => "down",
                    };
                    emit(&format!(
                        "[HID-KBD{tag}] vk=0x{:02X} {} dir={direction} scan=0x{:02X} flags=0x{:04X}",
                        keyboard.VKey,
                        key_name(keyboard.VKey),
                        keyboard.MakeCode,
                        keyboard.Flags
                    ));
                }
                0 => {
                    // RIM_TYPEMOUSE
                    let mouse = &raw.data.mouse;
                    emit(&format!(
                        "[HID-MOUSE{tag}] flags=0x{:04X} dx={} dy={}",
                        mouse.usFlags.0, mouse.lLastX, mouse.lLastY
                    ));
                }
                2 => {
                    // RIM_TYPEHID —— 厂商自定义集合的原始报告走这里
                    let hid = &raw.data.hid;
                    let count = hid.dwCount as usize;
                    let size_hid = hid.dwSizeHid as usize;
                    let offset = std::mem::size_of::<windows::Win32::UI::Input::RAWINPUTHEADER>() + 8;
                    let available = (size as usize).saturating_sub(offset);
                    let total = (count.saturating_mul(size_hid)).min(available).min(96);
                    let bytes = std::slice::from_raw_parts(buf.as_ptr().add(offset), total);
                    emit(&format!(
                        "[HID-RAW{tag}] reports={count} sizeHid={size_hid} b=[{}]",
                        hex(bytes, 96)
                    ));
                }
                other => emit(&format!("[RAW{tag}] type={other} len={size}")),
            }
        }
    }
    let _ = (hwnd, wparam);
}

/// 枚举当前所有 Raw Input 设备，打印每个集合的 Usage Page / Usage。
/// 厂商自定义集合（0xFF00 页）只能靠这里拿到确切的 (page, usage) 才能注册。
fn enumerate_raw_input_devices() -> Vec<(u16, u16)> {
    unsafe {
        let mut count: u32 = 0;
        let entry = std::mem::size_of::<RAWINPUTDEVICELIST>() as u32;
        if GetRawInputDeviceList(None, &mut count, entry) == u32::MAX || count == 0 {
            emit("[enum] GetRawInputDeviceList(计数) 失败");
            return Vec::new();
        }
        let mut list = vec![RAWINPUTDEVICELIST::default(); count as usize];
        let rc = GetRawInputDeviceList(Some(list.as_mut_ptr()), &mut count, entry);
        if rc == u32::MAX {
            emit("[enum] GetRawInputDeviceList 失败");
            return Vec::new();
        }
        let mut targets: Vec<(u16, u16)> = Vec::new();
        let mut others = 0usize;
        for item in &list[..count as usize] {
            let handle = item.hDevice.0 as usize;
            let name = device_name(handle).unwrap_or_default();
            let mut info: RID_DEVICE_INFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<RID_DEVICE_INFO>() as u32;
            let mut size = info.cbSize;
            let rc = GetRawInputDeviceInfoW(
                Some(item.hDevice),
                RIDI_DEVICEINFO,
                Some(&mut info as *mut RID_DEVICE_INFO as *mut core::ffi::c_void),
                &mut size,
            );
            let (page, usage) = if rc == u32::MAX {
                (0u16, 0u16)
            } else {
                (info.Anonymous.hid.usUsagePage, info.Anonymous.hid.usUsage)
            };
            if is_target(&name) {
                emit(&format!(
                    "[enum] 遥控器集合 type={:?} page=0x{page:04X} usage=0x{usage:04X}{}  {}",
                    item.dwType,
                    collection_tag(&name),
                    name
                ));
                if !targets.contains(&(page, usage)) {
                    targets.push((page, usage));
                }
            } else {
                others += 1;
            }
        }
        emit(&format!(
            "[enum] 共 {} 个 Raw Input 设备，其中非目标 {others} 个；遥控器集合 {} 个",
            count, targets.len()
        ));
        targets
    }
}
fn spawn_raw_input_thread() -> Option<std::thread::JoinHandle<()>> {
    let (tx, rx) = std::sync::mpsc::channel::<Option<u32>>();
    let handle = std::thread::spawn(move || unsafe {
        use windows::core::w;
        let mut warm = MSG::default();
        let _ = PeekMessageW(&mut warm, None, 0, 0, PM_NOREMOVE);
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            lpszClassName: w!("shengjian_chromecast_probe"),
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 {
            let _ = tx.send(None);
            return;
        }
        let hwnd = match CreateWindowExW(
            Default::default(),
            w!("shengjian_chromecast_probe"),
            w!("shengjian_chromecast_probe"),
            Default::default(),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            None,
            None,
        ) {
            Ok(hwnd) if !hwnd.is_invalid() => hwnd,
            _ => {
                let _ = tx.send(None);
                return;
            }
        };
        // 只注册标准三页。注意：usUsagePage/usUsage = 0/0（"全部 HID"）
        // 配 RIDEV_INPUTSINK 会直接注册失败（2026-11-18 实测），
        // 厂商集合（0xFF00 页）因此收不到原始上报 —— 已知盲区，见报告。
        // 先注册三张标准页，再把遥控器自己声明的集合（含厂商页）全部补上。
        // usUsagePage/usUsage = 0/0（"全部 HID"）配 RIDEV_INPUTSINK 会直接注册
        // 失败（2026-11-18 实测），所以必须逐个集合注册。
        let mut pages: Vec<(u16, u16)> = vec![(0x01, 0x06), (0x01, 0x02), (0x0C, 0x01)];
        if let Some(extra) = TARGET_COLLECTIONS.get() {
            if let Ok(extra) = extra.lock() {
                for page in extra.iter() {
                    // page=0 是「全部 HID」的占位，单页注册里非法；直接跳过。
                    if page.0 == 0 {
                        continue;
                    }
                    if !pages.contains(page) {
                        pages.push(*page);
                    }
                }
            }
        }
        // 一次整批注册（调用是原子的：混一个非法项整批失败，但分多次调用会互相顶掉，
        // 2026-11-18 实测逐页注册后只剩最后一页生效）。所以先过滤掉 usage=0 的
        // 非法项，再把剩下的合法页一次交出去。
        let devices: Vec<RAWINPUTDEVICE> = pages
            .iter()
            .filter(|(page, usage)| *page != 0 && *usage != 0)
            .map(|(page, usage)| RAWINPUTDEVICE {
                usUsagePage: *page,
                usUsage: *usage,
                dwFlags: RIDEV_INPUTSINK,
                hwndTarget: hwnd,
            })
            .collect();
        for (page, usage) in &pages {
            if *page == 0 || *usage == 0 {
                emit(&format!(
                    "[raw-input] 跳过非法项 page=0x{page:04X} usage=0x{usage:04X}"
                ));
            }
        }
        for device in &devices {
            emit(&format!(
                "[raw-input] 注册 page=0x{:04X} usage=0x{:04X}",
                device.usUsagePage, device.usUsage
            ));
        }
        if RegisterRawInputDevices(&devices, std::mem::size_of::<RAWINPUTDEVICE>() as u32).is_err() {
            let _ = tx.send(None);
            return;
        }
        let thread_id = windows::Win32::System::Threading::GetCurrentThreadId();
        let _ = tx.send(Some(thread_id));
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            if msg.message == WM_QUIT || msg.message == WM_QUIT_PROBE {
                break;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
    match rx.recv() {
        Ok(Some(thread_id)) => {
            // 把线程 id 藏在 handle 里不方便，直接返回 id 通过第二个通道
            let _ = thread_id;
            emit("[raw-input] 已注册：键盘页 / 鼠标页 / Consumer 页（RIDEV_INPUTSINK，只旁听）");
            Some(handle)
        }
        _ => {
            emit("[raw-input] 注册失败（键盘/鼠标/Consumer 页）");
            None
        }
    }
}

// ---------- GATT ----------

fn connect(address: u64) -> Option<BluetoothLEDevice> {
    for attempt in 0..6 {
        let operation = BluetoothLEDevice::FromBluetoothAddressAsync(address).ok()?;
        match futures::executor::block_on(operation.into_future()) {
            Ok(device) if !device.as_raw().is_null() => return Some(device),
            Ok(_) => emit(&format!("连接返回空（尝试 {}/6）", attempt + 1)),
            Err(error) => emit(&format!("连接失败（尝试 {}/6）: {error:?}", attempt + 1)),
        }
        if attempt < 5 {
            std::thread::sleep(std::time::Duration::from_millis(800));
        }
    }
    None
}

fn get_services(device: &BluetoothLEDevice) -> Vec<GattDeviceService> {
    for attempt in 0..8 {
        let cache = if attempt % 2 == 0 {
            BluetoothCacheMode::Uncached
        } else {
            BluetoothCacheMode::Cached
        };
        if let Some(result) = block::<_, _>(device.GetGattServicesWithCacheModeAsync(cache)) {
            let status = result
                .Status()
                .unwrap_or(GattCommunicationStatus::Unreachable);
            if status == GattCommunicationStatus::Success {
                let services = result.Services().unwrap();
                if services.Size().unwrap_or(0) > 0 {
                    let mut list = Vec::new();
                    for index in 0..services.Size().unwrap_or(0) {
                        if let Ok(service) = services.GetAt(index) {
                            list.push(service);
                        }
                    }
                    return list;
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(600));
    }
    Vec::new()
}

fn subscribe(tag: &str, characteristic: &GattCharacteristic) -> bool {
    let properties = match characteristic.CharacteristicProperties() {
        Ok(properties) => properties,
        Err(_) => return false,
    };
    let value = if properties.0 & GattCharacteristicProperties::Notify.0 != 0 {
        GattClientCharacteristicConfigurationDescriptorValue::Notify
    } else if properties.0 & GattCharacteristicProperties::Indicate.0 != 0 {
        GattClientCharacteristicConfigurationDescriptorValue::Indicate
    } else {
        emit(&format!("[gatt] 跳过 {tag}：不可通知"));
        return false;
    };
    match block::<_, _>(characteristic.WriteClientCharacteristicConfigurationDescriptorAsync(value)) {
        Some(GattCommunicationStatus::Success) => {
            let handler_tag = tag.to_string();
            let handler = TypedEventHandler::<GattCharacteristic, GattValueChangedEventArgs>::new(
                move |_, args| {
                    if let Some(args) = args.as_ref() {
                        if let Ok(buffer) = args.CharacteristicValue() {
                            if let Ok(bytes) = buffer_to_vec(&buffer) {
                                emit(&format!("[gatt] {handler_tag} len={} b=[{}]", bytes.len(), hex(&bytes, 32)));
                            }
                        }
                    }
                    Ok(())
                },
            );
            match characteristic.ValueChanged(&handler) {
                Ok(_) => {
                    emit(&format!("[gatt] 订阅成功 {tag}"));
                    true
                }
                Err(error) => {
                    emit(&format!("[gatt] 订阅事件失败 {tag} {error:?}"));
                    false
                }
            }
        }
        other => {
            emit(&format!("[gatt] CCCD 写入失败 {tag} status={other:?}"));
            false
        }
    }
}

// ---------- 主流程 ----------

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut seconds = 120u64;
    let mut out: Option<String> = None;
    let mut mac: Option<u64> = None;
    let mut no_gatt = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--seconds" => {
                seconds = args.get(index + 1).and_then(|v| v.parse().ok()).unwrap_or(120);
                index += 2;
            }
            "--out" => {
                out = args.get(index + 1).cloned();
                index += 2;
            }
            "--mac" => {
                mac = args.get(index + 1).and_then(|v| parse_mac(v));
                index += 2;
            }
            "--no-gatt" => {
                no_gatt = true;
                index += 1;
            }
            _ => index += 1,
        }
    }

    let path = out.unwrap_or_else(|| "chromecast-probe.txt".to_string());
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
        let _ = std::fs::write("chromecast-probe-panic.txt", &text);
        let _ = emit(&text);
    }));

    unsafe {
        if let Err(error) = RoInitialize(RO_INIT_MULTITHREADED) {
            emit(&format!("RoInitialize 失败: {error:?}"));
        }
    }

    let Some(address) = mac else {
        emit("用法: --mac AA:BB:CC:DD:EE:FF [--seconds N] [--out 文件]");
        return;
    };
    emit(&format!(
        "=== chromecast_probe 开始 target={} seconds={seconds} ===",
        masked(address)
    ));

    let collections = enumerate_raw_input_devices();
    let _ = TARGET_COLLECTIONS.set(Mutex::new(collections));

    let raw_thread = spawn_raw_input_thread();

    if no_gatt {
        emit("[gatt] --no-gatt：不建立 GATT 会话，只抓 Raw Input（用来排除\"探针占用 GATT 把 HID 挤掉\"）");
    }

    if !no_gatt {
    let Some(device) = connect(address) else {
        emit("结论: 连不上（设备休眠 / 未配对 / 被其他会话占用）");
        return;
    };
    emit(&format!(
        "已连接 name=\"{}\"",
        device.Name().map(|n| n.to_string()).unwrap_or_default()
    ));

    let services = get_services(&device);
    emit(&format!("服务数: {}", services.len()));

    let mut subscribed = 0usize;
    for service in &services {
        let service_uuid = service.Uuid().unwrap();
        let characteristics = match block::<_, _>(
            service.GetCharacteristicsWithCacheModeAsync(BluetoothCacheMode::Uncached),
        ) {
            Some(result) => result.Characteristics().unwrap(),
            None => continue,
        };
        for index in 0..characteristics.Size().unwrap_or(0) {
            let Ok(characteristic) = characteristics.GetAt(index) else {
                continue;
            };
            let uuid = characteristic.Uuid().unwrap();
            let wanted = matches!(
                uuid.to_u128(),
                BATTERY_LEVEL | VENDOR_AE42 | GOOGLE_D343_NOTIFY | ATVV_CONTROL | SERVICE_CHANGED
            );
            if !wanted {
                continue;
            }
            let tag = format!(
                "{}/{}",
                short_uuid(&service_uuid),
                label(&uuid)
            );
            if subscribe(&tag, &characteristic) {
                subscribed += 1;
            }
        }
    }
    emit(&format!("订阅通道数: {subscribed}"));
    }

    emit("---------------------------------------------------------------");
    emit("开始采集。请按下面节奏按键（每段之间停 2 秒以上，不要连按）：");
    emit("  段 1  语音/助理键：按住 2 秒后松开（ATVV_CONTROL 阳性对照）");
    emit("  段 2  返回");
    emit("  段 3  主页");
    emit("  段 4  方向上 / 下 / 左 / 右（每次停 1 秒）");
    emit("  段 5  确定（中键）");
    emit("  段 6  音量+ / 音量-（各 1 次）");
    emit("  段 7  静音");
    emit("  段 8  电源键（会合盖/睡眠，可选，跳过请直接等到结束）");
    emit("  段 9  Netflix / YouTube 键（会唤起应用，可选）");
    emit("  段 10 输入源 / 设置键（如果有）");
    emit("---------------------------------------------------------------");

    std::thread::sleep(std::time::Duration::from_secs(seconds));

    emit(&format!("=== 采集结束 {seconds}s ==="));
    emit(&format!(
        "过滤掉的非目标设备事件: {}（本机物理键鼠与其它 HID）",
        OTHER_EVENTS.load(std::sync::atomic::Ordering::Relaxed)
    ));
    if let Some(handle) = raw_thread {
        // 线程 id 拿不到就让它自己一直转；进程退出会回收。
        std::mem::forget(handle);
    }
    unsafe {
        windows::Win32::System::WinRT::RoUninitialize();
    }
}
