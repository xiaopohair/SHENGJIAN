//! ATVV 只读监听探针：**不写任何字节**，只看设备主动推了什么。
//!
//! 用途：回答"某个按键走哪条通道"。订阅 ATVV 的 CONTROL(0xAB5E0004) 与
//! AUDIO(0xAB5E0003)，把收到的每条通知原样打出来（时间戳 + 长度 + 十六进制），
//! 全程只做 CCCD 订阅，不发 `GET_CAPS` / `MIC_OPEN` / `MIC_CLOSE`。
//!
//! 为什么需要它（2026-11-18 Chromecast Remote 探测）：
//! 那台遥控器的助理键**不产生任何 HID 报告**（14 个消费页码里没有它），
//! 但按下去应用能起语音会话（`source=stream_start`）。本探针用来确认：
//! 设备是否在按键时**主动**发 `0x04 STREAM_START`（ATVV 控制通道），
//! 而不是依赖主机先发 `MIC_OPEN`。
//!
//! 用法：
//!   cargo run -p shengjian-windows --example atvv_control_listen -- \
//!       --mac A7:60:10:14:3D:23 --seconds 40 --out atvv-listen.txt

use std::future::IntoFuture;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::Instant;

use windows::core::{Interface, GUID};
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristic, GattCharacteristicProperties,
    GattClientCharacteristicConfigurationDescriptorValue, GattCommunicationStatus,
    GattValueChangedEventArgs,
};
use windows::Devices::Bluetooth::{BluetoothCacheMode, BluetoothLEDevice};
use windows::Foundation::TypedEventHandler;
use windows::Storage::Streams::{DataReader, DataWriter, IBuffer};
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

const ATVV_SERVICE: u128 = 0xab5e00015a214f05bc7daf01f617b664;
const ATVV_AUDIO: u128 = 0xab5e00035a214f05bc7daf01f617b664;
const ATVV_CONTROL: u128 = 0xab5e00045a214f05bc7daf01f617b664;
const ATVV_TRANSMIT: u128 = 0xab5e00025a214f05bc7daf01f617b664;

struct ProbeLog {
    file: Mutex<std::fs::File>,
    start: Instant,
}

static LOG: OnceLock<ProbeLog> = OnceLock::new();
static AUDIO_FRAMES: AtomicU64 = AtomicU64::new(0);
static AUDIO_BYTES: AtomicU64 = AtomicU64::new(0);

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

fn hex(bytes: &[u8], max: usize) -> String {
    bytes
        .iter()
        .take(max)
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn buffer_to_vec(buffer: &IBuffer) -> windows::core::Result<Vec<u8>> {
    let length = buffer.Length()? as usize;
    let reader = DataReader::FromBuffer(buffer)?;
    let mut bytes = vec![0u8; length];
    reader.ReadBytes(&mut bytes)?;
    Ok(bytes)
}

fn block<T, E: std::fmt::Debug>(op: windows::core::Result<E>) -> Option<T>
where
    E: IntoFuture<Output = windows::core::Result<T>>,
{
    match op {
        Ok(op) => match futures::executor::block_on(op.into_future()) {
            Ok(value) => Some(value),
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

fn find_characteristic(
    device: &BluetoothLEDevice,
    service_uuid: GUID,
    characteristic_uuid: GUID,
    label: &str,
) -> Option<GattCharacteristic> {
    let mut service = None;
    for attempt in 0..6 {
        let cache = if attempt % 2 == 0 {
            BluetoothCacheMode::Uncached
        } else {
            BluetoothCacheMode::Cached
        };
        if let Some(result) =
            block::<_, _>(device.GetGattServicesForUuidWithCacheModeAsync(service_uuid, cache))
        {
            if result.Status().ok() == Some(GattCommunicationStatus::Success) {
                if let Ok(services) = result.Services() {
                    if services.Size().unwrap_or(0) > 0 {
                        service = services.GetAt(0).ok();
                        break;
                    }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    let service = service?;
    for attempt in 0..6 {
        let cache = if attempt % 2 == 0 {
            BluetoothCacheMode::Uncached
        } else {
            BluetoothCacheMode::Cached
        };
        if let Some(result) = block::<_, _>(
            service.GetCharacteristicsForUuidWithCacheModeAsync(characteristic_uuid, cache),
        ) {
            if result.Status().ok() == Some(GattCommunicationStatus::Success) {
                if let Ok(characteristics) = result.Characteristics() {
                    if characteristics.Size().unwrap_or(0) > 0 {
                        return characteristics.GetAt(0).ok();
                    }
                }
            }
        }
        emit(&format!("  {label}: 特征枚举重试（{}/6）", attempt + 1));
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    None
}

fn subscribe(
    tag: &str,
    characteristic: &GattCharacteristic,
    on_bytes: impl Fn(&[u8]) + 'static + Sync + Send,
) -> bool {
    let properties = match characteristic.CharacteristicProperties() {
        Ok(properties) => properties,
        Err(_) => return false,
    };
    let value = if properties.0 & GattCharacteristicProperties::Notify.0 != 0 {
        GattClientCharacteristicConfigurationDescriptorValue::Notify
    } else if properties.0 & GattCharacteristicProperties::Indicate.0 != 0 {
        GattClientCharacteristicConfigurationDescriptorValue::Indicate
    } else {
        emit(&format!("[atvv] {tag} 不可通知"));
        return false;
    };
    match block::<_, _>(characteristic.WriteClientCharacteristicConfigurationDescriptorAsync(value))
    {
        Some(GattCommunicationStatus::Success) => {}
        other => {
            emit(&format!("[atvv] {tag} CCCD 写入失败 status={other:?}"));
            return false;
        }
    }
    let on_bytes = std::sync::Arc::new(on_bytes);
    let handler =
        TypedEventHandler::<GattCharacteristic, GattValueChangedEventArgs>::new(move |_, args| {
            if let Some(args) = args.as_ref() {
                if let Ok(buffer) = args.CharacteristicValue() {
                    if let Ok(bytes) = buffer_to_vec(&buffer) {
                        on_bytes(&bytes);
                    }
                }
            }
            Ok(())
        });
    match characteristic.ValueChanged(&handler) {
        Ok(_) => {
            emit(&format!("[atvv] 订阅成功 {tag}（只读）"));
            true
        }
        Err(error) => {
            emit(&format!("[atvv] 订阅事件失败 {tag} {error:?}"));
            false
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut seconds = 40u64;
    let mut out: Option<String> = None;
    let mut mac: Option<u64> = None;
    let mut caps_only = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--seconds" => {
                seconds = args
                    .get(index + 1)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(40);
                index += 2;
            }
            "--out" => {
                out = args.get(index + 1).cloned();
                index += 2;
            }
            "--caps-only" => {
                caps_only = true;
                index += 1;
            }
            "--mac" => {
                mac = args.get(index + 1).and_then(|v| parse_mac(v));
                index += 2;
            }
            _ => index += 1,
        }
    }

    let path = out.unwrap_or_else(|| "atvv-control-listen.txt".to_string());
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
        "=== ATVV 只读监听 target={} {seconds}s（不写任何字节）===",
        masked(address)
    ));

    let Some(device) = connect(address) else {
        emit("结论: 连不上设备");
        return;
    };
    emit(&format!(
        "已连接 name=\"{}\"",
        device
            .Name()
            .map(|name| name.to_string())
            .unwrap_or_default()
    ));

    let service_uuid = GUID::from_u128(ATVV_SERVICE);
    let Some(control) = find_characteristic(
        &device,
        service_uuid,
        GUID::from_u128(ATVV_CONTROL),
        "CONTROL",
    ) else {
        emit("结论: 找不到 ATVV CONTROL");
        return;
    };

    subscribe("CONTROL", &control, |bytes| {
        emit(&format!(
            "[CONTROL] len={} b=[{}]",
            bytes.len(),
            hex(bytes, 32)
        ));
    });

    if let Some(audio) =
        find_characteristic(&device, service_uuid, GUID::from_u128(ATVV_AUDIO), "AUDIO")
    {
        subscribe("AUDIO", &audio, |bytes| {
            let frames = AUDIO_FRAMES.fetch_add(1, Ordering::Relaxed) + 1;
            AUDIO_BYTES.fetch_add(bytes.len() as u64, Ordering::Relaxed);
            // 音频会刷屏：只记首帧和每 50 帧一次。
            if frames == 1 || frames % 50 == 0 {
                emit(&format!(
                    "[AUDIO] 第 {frames} 帧 len={} b=[{}]",
                    bytes.len(),
                    hex(bytes, 12)
                ));
            }
        });
    }

    if caps_only {
        match find_characteristic(
            &device,
            service_uuid,
            GUID::from_u128(ATVV_TRANSMIT),
            "TRANSMIT",
        ) {
            Some(transmit) => {
                let bytes = [0x0A, 0x01, 0x00, 0x00, 0x03, 0x03];
                match DataWriter::new() {
                    Ok(writer) => {
                        if writer.WriteBytes(&bytes).is_ok() {
                            if let Ok(buffer) = writer.DetachBuffer() {
                                match block::<_, _>(transmit.WriteValueAsync(&buffer)) {
                                    Some(status) => emit(&format!("[caps-only] GET_CAPS 写入 status={status:?}（不写 MIC_OPEN）")),
                                    None => emit("[caps-only] GET_CAPS 写入失败"),
                                }
                            }
                        }
                    }
                    Err(error) => emit(&format!("[caps-only] DataWriter 失败: {error:?}")),
                }
            }
            None => emit("[caps-only] 找不到 ATVV TRANSMIT"),
        }
    }

    emit("---------------------------------------------------------------");
    emit("请按遥控器：先按助理键（按住 2 秒再松开），再按一下音量+ 做对照。");
    emit("---------------------------------------------------------------");

    std::thread::sleep(std::time::Duration::from_secs(seconds));

    emit(&format!(
        "=== 结束：AUDIO {} 帧 / {} 字节 ===",
        AUDIO_FRAMES.load(Ordering::Relaxed),
        AUDIO_BYTES.load(Ordering::Relaxed)
    ));
    emit(if AUDIO_FRAMES.load(Ordering::Relaxed) > 0 {
        "结论：设备在没有主机 MIC_OPEN 的情况下自己推了音频 —— 按键走 ATVV 控制通道"
    } else {
        "结论：没有音频 —— 本次按键没有触发设备侧推流"
    });
    let _ = device.Close();
    unsafe {
        windows::Win32::System::WinRT::RoUninitialize();
    }
}
