//! 虚拟 HID 键盘（WinUHid 客户端）。
//!
//! 背景：报告层受遥控器描述符限制——每帧最多两个键，第三槽写入不产生事件
//! （2026-11-16 低级键盘钩子实测）。要发三键或任意组合，只能换一个"发键"的出口：
//! 用 [WinUHid](https://github.com/cgutman/WinUHid)（MIT）的 UMDF2 + 系统内置 VHF
//! 驱动，在系统里注册一台**虚拟键盘**，由提权助手持有并注入报文。
//!
//! 为什么由助手持有：WinUHid 的 INF 把控制设备的 ACL 写死成"管理员 / SYSTEM /
//! UMDF"（`D:P(A;;GA;;;BA)(A;;GA;;;SY)(A;;GA;;;UD)`），非提权进程 `CreateFile`
//! 直接 ACCESS_DENIED。助手本来就是提权计划任务，正好是这个角色的持有者。
//!
//! 本模块**动态加载** `WinUHid.dll`（`LoadLibraryW` + `GetProcAddress`），不引入
//! 编译期依赖：DLL 或驱动缺失时 `open()` 返回 `Err`，上层据此判定"虚拟键盘后端
//! 不可用"并回退报告层，而不是让整个助手起不来。
//!
//! 真机验证（2026-11-17）：本模块的原型（PowerShell P/Invoke 同一套 API）发
//! `Ctrl+Shift+Alt` 时，低级键盘钩子看到三键同帧到达且 `flags=0x0`（非注入），
//! Chatterfly 语音条被成功拉起。

use std::ffi::c_void;

type Handle = *mut c_void;

extern "system" {
    fn LoadLibraryW(name: *const u16) -> Handle;
    fn GetProcAddress(module: Handle, name: *const u8) -> *mut c_void;
    fn FreeLibrary(module: Handle) -> i32;
}

/// `WINUHID_DEVICE_CONFIG`（`#pragma pack(1)`，见上游 `WinUHid.h`）。
#[repr(C, packed)]
struct DeviceConfig {
    supported_events: i32,
    vendor_id: u16,
    product_id: u16,
    version_number: u16,
    report_descriptor_length: u16,
    report_descriptor: *const c_void,
    container_id: [u8; 16],
    instance_id: *const u16,
    hardware_ids: *const u16,
    read_report_period_us: u32,
}

type VersionFn = unsafe extern "C" fn() -> u32;
type CreateFn = unsafe extern "C" fn(*const DeviceConfig) -> *mut c_void;
type StartFn = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> i32;
type SubmitFn = unsafe extern "C" fn(*mut c_void, *const u8, u32) -> i32;
type DestroyFn = unsafe extern "C" fn(*mut c_void);

/// 标准启动键盘描述符：修饰位 + 保留字节 + 6 个键，另带 5 位 LED 输出。
///
/// 与 WinUHid 仓库自带的设备（鼠标/手柄）不同，**键盘描述符要自己写**；这份是
/// USB HID 启动键盘的经典布局，Windows 直接认成普通键盘。
static KEYBOARD_REPORT_DESCRIPTOR: [u8; 63] = [
    0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x05, 0x07, 0x19, 0xE0, 0x29, 0xE7, 0x15, 0x00, 0x25, 0x01,
    0x75, 0x01, 0x95, 0x08, 0x81, 0x02, 0x95, 0x01, 0x75, 0x08, 0x81, 0x03, 0x95, 0x05, 0x75, 0x01,
    0x05, 0x08, 0x19, 0x01, 0x29, 0x05, 0x91, 0x02, 0x95, 0x01, 0x75, 0x03, 0x91, 0x03, 0x95, 0x06,
    0x75, 0x08, 0x15, 0x00, 0x25, 0x65, 0x05, 0x07, 0x19, 0x00, 0x29, 0x65, 0x81, 0x00, 0xC0,
];

/// 设备报文长度：修饰位(1) + 保留(1) + 6 键(6)。
const REPORT_LEN: usize = 8;

/// 把 (修饰位, 单 usage) 编成 8 字节输入报文。
///
/// 修饰位直接就是 HID 掩码（`0x01` 左 Ctrl、`0x02` 左 Shift、`0x04` 左 Alt、
/// `0x10` 右 Ctrl…），与报告层 `VOICE_SYNTH_MODS` 用的是同一套位；单键目标
/// （左/右 Alt）则写进第一个键槽。
fn report_bytes(mods: u8, usage: u16) -> [u8; REPORT_LEN] {
    let mut report = [0u8; REPORT_LEN];
    report[0] = mods;
    if usage != 0 {
        report[2] = (usage & 0xFF) as u8;
        report[3] = (usage >> 8) as u8;
    }
    report
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 动态加载的 WinUHid 客户端。
pub struct WinUHid {
    module: Handle,
    create: CreateFn,
    start: StartFn,
    submit: SubmitFn,
    destroy: DestroyFn,
    version: VersionFn,
}

// 句柄与函数指针只被持有、不跨线程共享裸指针语义；实际使用经 `Mutex` 串行化。
unsafe impl Send for WinUHid {}

impl WinUHid {
    /// 加载 `WinUHid.dll`。DLL 不在搜索路径时返回 `Err`（调用方据此回退）。
    pub fn open() -> Result<Self, String> {
        let name = wide("WinUHid.dll");
        let module = unsafe { LoadLibraryW(name.as_ptr()) };
        if module.is_null() {
            return Err("WinUHid.dll 未找到".to_owned());
        }
        let resolve = |symbol: &str| -> Result<*mut c_void, String> {
            let mut name: Vec<u8> = symbol.as_bytes().to_vec();
            name.push(0);
            let address = unsafe { GetProcAddress(module, name.as_ptr()) };
            if address.is_null() {
                Err(format!("WinUHid.dll 缺少导出 {symbol}"))
            } else {
                Ok(address)
            }
        };
        let loaded = (|| {
            Ok(Self {
                module,
                create: unsafe { std::mem::transmute(resolve("WinUHidCreateDevice")?) },
                start: unsafe { std::mem::transmute(resolve("WinUHidStartDevice")?) },
                submit: unsafe { std::mem::transmute(resolve("WinUHidSubmitInputReport")?) },
                destroy: unsafe { std::mem::transmute(resolve("WinUHidDestroyDevice")?) },
                version: unsafe { std::mem::transmute(resolve("WinUHidGetDriverInterfaceVersion")?) },
            })
        })();
        match loaded {
            Ok(library) => Ok(library),
            Err(error) => {
                unsafe { FreeLibrary(module) };
                Err(error)
            }
        }
    }

    /// 驱动接口版本；0 表示驱动未安装或不可用（非提权调用也会得到 0）。
    pub fn driver_version(&self) -> u32 {
        unsafe { (self.version)() }
    }

    /// 创建一台虚拟键盘并启动（不注册任何事件：键盘只需要输入方向）。
    pub fn create_keyboard(&self) -> Result<VirtualKeyboard, String> {
        let instance = wide("shengjian-virtual-keyboard");
        let config = DeviceConfig {
            supported_events: 0,
            vendor_id: 0x1209,
            product_id: 0x5A11,
            version_number: 0x0001,
            report_descriptor_length: KEYBOARD_REPORT_DESCRIPTOR.len() as u16,
            report_descriptor: KEYBOARD_REPORT_DESCRIPTOR.as_ptr().cast(),
            container_id: [0u8; 16],
            instance_id: instance.as_ptr(),
            hardware_ids: std::ptr::null(),
            read_report_period_us: 0,
        };
        let device = unsafe { (self.create)(&config) };
        if device.is_null() {
            return Err(format!(
                "WinUHidCreateDevice 失败（GetLastError={}）",
                std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
            ));
        }
        let started = unsafe { (self.start)(device, std::ptr::null_mut(), std::ptr::null_mut()) };
        if started == 0 {
            unsafe { (self.destroy)(device) };
            return Err("WinUHidStartDevice 失败".to_owned());
        }
        Ok(VirtualKeyboard {
            device,
            submit: self.submit,
            destroy: self.destroy,
        })
    }
}

impl Drop for WinUHid {
    fn drop(&mut self) {
        unsafe { FreeLibrary(self.module) };
    }
}

/// 一台已创建的虚拟键盘。`Drop` 时销毁设备，避免残留 HID 设备。
pub struct VirtualKeyboard {
    device: *mut c_void,
    submit: SubmitFn,
    destroy: DestroyFn,
}

unsafe impl Send for VirtualKeyboard {}

impl VirtualKeyboard {
    /// 按下：`mods` 是 HID 修饰位，`usage` 是单键目标（没有就传 0）。
    pub fn press(&self, mods: u8, usage: u16) -> Result<(), String> {
        let report = report_bytes(mods, usage);
        self.submit_report(&report)
    }

    /// 释放：全零报文（所有键/修饰位抬起）。
    pub fn release(&self) -> Result<(), String> {
        self.submit_report(&[0u8; REPORT_LEN])
    }

    fn submit_report(&self, report: &[u8; REPORT_LEN]) -> Result<(), String> {
        let ok = unsafe {
            (self.submit)(
                self.device,
                report.as_ptr(),
                REPORT_LEN as u32,
            )
        };
        if ok == 0 {
            Err(format!(
                "WinUHidSubmitInputReport 失败（GetLastError={}）",
                std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
            ))
        } else {
            Ok(())
        }
    }
}

impl Drop for VirtualKeyboard {
    fn drop(&mut self) {
        unsafe { (self.destroy)(self.device) };
    }
}

#[cfg(test)]
mod tests {
    use super::{report_bytes, REPORT_LEN};

    #[test]
    fn modifier_mask_lands_in_the_modifier_byte() {
        // Ctrl+Alt：掩码 0x05 原样进修饰位，键槽全空。
        let report = report_bytes(0x05, 0);
        assert_eq!(report[0], 0x05);
        assert!(report[1..].iter().all(|byte| *byte == 0));
        // Ctrl+Shift+Alt：0x07（报告层做不到的三键组合，虚拟键盘可以）。
        assert_eq!(report_bytes(0x07, 0)[0], 0x07);
    }

    #[test]
    fn single_usage_lands_in_the_first_key_slot() {
        // 右 Alt = usage 0x00E6，写进第一个键槽，修饰位保持 0。
        let report = report_bytes(0, 0x00E6);
        assert_eq!(report[0], 0);
        assert_eq!(report[2], 0xE6);
        assert_eq!(report[3], 0x00);
        // 报文长度固定 8 字节（描述符决定），其余槽位必须为零。
        assert_eq!(report.len(), REPORT_LEN);
        assert!(report[4..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn zero_target_is_an_all_zero_report() {
        assert_eq!(report_bytes(0, 0), [0u8; REPORT_LEN]);
    }
}


