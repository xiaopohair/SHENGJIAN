# 虚拟 HID 键盘（WinUHid）接入评估

日期：2026-11-17

> 背景：报告层受器件描述符限制，每帧最多两键、第三槽写入不产生事件（2026-11-16 低级键盘钩子
> 实测）。`Alt+W`、`Ctrl+Shift+X` 这类组合无法靠改写遥控器报文发出。要突破只能从"发键"这一端
> 换成虚拟 HID 键盘。本文评估 [cgutman/WinUHid](https://github.com/cgutman/WinUHid) 这条路。

## 结论摘要

可行，但它是**装驱动 + 在系统里信任一张证书**的重量级方案。它能同时解决两件事：任意组合/任意
键数，以及"注入键被输入法丢弃"（虚拟键盘发出的键不是 `SendInput`，不带 `LLKHF_INJECTED`）。

**2026-11-17 已做真机原型验证：Chatterfly 接受虚拟键盘发出的组合键**（详见文末「原型验证结果」）。
下一步是决定是否产品化。

## 事实（源码核对，2026-11-17）

| 项目 | 结论 |
|---|---|
| 许可证 | **MIT**（Copyright (c) 2025 Cameron Gutman），与仓库的 GPL-3.0-only 兼容，需保留版权声明 |
| 驱动形态 | root 枚举设备 `Root\WinUHid`，**UMDF2**（`WinUHidDriver.dll`），经系统自带 `hidvhf.inf`（VHF）+ `wudfrd.inf` 暴露为虚拟 HID |
| 是否需要内核驱动 | 否。VHF 与 UMDF 都是系统组件，**HVCI 下可用**（这点比 Interception 好） |
| 设备对象权限 | INF 里写死 `D:P(A;;GA;;;BA)(A;;GA;;;SY)(A;;GA;;;UD)`：只有 **管理员 / SYSTEM / UMDF 驱动**能创建虚拟设备 |
| HID 描述符 | 运行时通过 `IOCTL_WINUHID_SET_REPORT_DESCRIPTOR` 下发，**布局由调用方定义**（可做标准键盘：6 键 + 修饰位） |
| 现成设备 | `WinUHidDevs` 只附带鼠标、PS4、PS5、Xbox One —— **没有键盘**，键盘描述符与报文编码要自己实现 |
| 安装 | WiX v4 MSI：安装驱动包、把签名证书放进 `trustedPublisher`、创建 root 设备；需要管理员；支持 major upgrade 与回滚 |
| 信任链 | 仓库自带 `WinUHidCertificate.cer` 与 `RootDevCA.dll`：靠**安装时信任发布者证书**落地，而不是 Microsoft attestation 签名 |
| 平台 | INF 分 `10.0...19041`（Win10）与 `10.0...22000`（Win11）两套安装段，WUDF 过滤路径不同 |

## 对我们产品的影响

1. **能力**：任意组合、最多 6 键并发、任意键位（含 F 键、标点、方向键），彻底摆脱器件描述符的
   两键上限与 `VOICE_SYNTHABLE_*` 白名单。
2. **架构**：虚拟设备必须由**提权进程**持有（ACL 限制），正好落在现有的 `shengjian-helper` 上；
   主程序通过现有桥接下发"打这个和弦"。agent 继续负责清遥控器原生边沿——两者职责不重叠。
3. **信任成本**：用户机器上要多信任一张自签名证书并多装一个驱动。对个人开源项目可以接受，但
   必须显式二次确认、可一键卸载，并在文档里写清楚。
4. **发布链路**：应用侧照旧 SignPath；驱动包需要自己的 catalog 签名。两条路：
   - **直接分发官方 MSI**：省事，但版本与证书由上游控制；
   - **fork 驱动 + 自签 catalog**：可控，但要维护 WDK 构建与证书轮换。
5. **回退**：报告层保留为"无驱动"路径；虚拟键盘只作为「按键级后端」的可选项，与 MoteDeck 的
   做法一致（每个按键/每个语音槽单独选后端）。

## 原型计划（最小验证，约 1–2 小时）

1. 安装官方 MSI（需要管理员；会信任其发布者证书、创建 root 设备）。
2. 写一个最小探针（C 或 Rust 直接走 IOCTL，不必用它的 C++ 封装）：
   `SET_DEVICE_INFO` → `SET_REPORT_DESCRIPTOR`（标准启动键盘）→ `CREATE_DEVICE` →
   `START_DEVICE`，然后发一次 `Ctrl+Shift+Alt`。
3. 验证两件事：低级键盘钩子看到三键且 `flags` 不含 `0x10`（非注入）；Chatterfly 把热键设成
   `Ctrl+Shift+Alt` 时能被拉起并出字。
4. 通过则再谈产品化；不通过（例如输入法还挑设备）就没有必要继续。

## 风险清单

- **证书信任**：安装会弹安全确认；受管企业机可能禁止安装驱动或信任证书。
- **驱动生命周期**：Windows 大版本升级后可能需要重装；需要版本检测与修复入口。
- **杀软/EDR**：UMDF 驱动通常比内核驱动温和，但仍可能被拦；要提供失败时的明确降级提示。
- **卸载**：必须能干净移除 root 设备 + 证书 + 驱动包（官方 MSI 已有卸载 CA，可复用其流程）。
- **安全面**：虚拟键盘等于一个可编程 HID 设备，必须只让管理员级进程使用（INF 已限制），
  我们的命令面也要只对提权助手开放。

## 建议

1. 先做原型验证（**需要你同意在这台机器上装一次驱动**），只验证"Chatterfly 是否接受虚拟键盘"；
2. 验证通过后，把它做成「按键级后端」选项（默认仍是报告层），而不是无脑替换；
3. 若验证失败，维持现状：报告层两键（`Ctrl+Alt` / `Ctrl+右Ctrl`）已经覆盖两家输入法的语音键。

## 原型验证结果（2026-11-17，已通过）

**环境**：用 MoteDeck 随包的正规安装器装了 WinUHid 驱动包（`WinUHidDriver.inf` + `.cat` +
`WinUHidCertificate.cer`），设备节点 `ROOT\WINUHID\0000`（`WinUHid Virtual HID Enumerator`，
Status OK）。驱动包与客户端 DLL 均为 MIT（Cameron Gutman, 2025）。

**探针**：PowerShell P/Invoke `WinUHid.dll` —— `WinUHidCreateDevice` + `WinUHidSubmitInputReport`，
标准启动键盘描述符（8 字节报文：修饰位 + 保留 + 6 键，带 5 位 LED 输出），VID/PID `0x1209/0x0001`，
不注册任何事件（`SupportedEvents = 0`）。

**观测**：

| 组合 | 低级键盘钩子 | 结论 |
|---|---|---|
| `Ctrl+Shift+Alt`（修饰位 `0x07`） | `0xA2`/`0xA0`/`0xA4` 同帧到达（间隔约 3ms），`HARDWARE`，`flags=0x0` | 非注入，三键齐到 |
| `Ctrl+Alt`（修饰位 `0x05`） | `0xA2`/`0xA4` 到达，`HARDWARE`，`flags=0x0` | 非注入，两键齐到 |

**结论**：Chatterfly **会接受虚拟键盘发出的组合键**（现场确认语音条被拉起）。

**一个容易误判的坑**：Chatterfly 的语音条挂在 **TSF 文本上下文**上。第一次探针在提权的
PowerShell 控制台里发键，键到了但语音条没出现，很容易误判成"设备被拒"；换成先点进文本输入框
（记事本/聊天框）再发键就正常。以后做同类验证必须先准备文本焦点。

**顺带确认**：早先一次 `Ctrl+Alt` 没反应，原因是 Chatterfly 当时的快捷键不是这个组合，
不是虚拟键盘的问题——这也说明产品化时**后端选择要能按工具/按键分别配置**，而不是全局一刀切。

## 产品化进展（2026-11-17：已接通并真机验证）

代码已落地（`53aa2f9` / `f7e11fa` / `a74ae1e`）：

- **助手侧**：`helper/rc003/src/virtual_hid.rs` 动态加载 `WinUHid.dll`，创建标准启动键盘
  （63 字节描述符、8 字节报文），`press(mods, usage)` / `release()`；桥接新增 `K` 行
  （`K ?` 查询、`K <mods> <usage>` 按下、`K -` 释放），回 `K ok/no/ack/err`。
- **主程序侧**：hello 发 `K ?`；回执驱动可用性；语音按下按
  `report_layer → virtual_hid → send_input` 降级，互斥且各自留判据日志；释放统一挂在
  `release_voice_hold_hotkey()`（助手侧幂等），不会因某个 close 分支漏掉而粘键。
- **真机结果**：热键设 `Ctrl+Shift+Alt`（报告层表达不了的组合）→ 日志
  `voice_attempt ... path=virtual_hid` + `chord_press result=skipped reason=virtual_hid_active`；
  低级键盘钩子见三键同帧到达、`flags=0x0`（INJECTED 计数 0）；**Chatterfly 弹条并识别出文字**。

部署时踩到的两个坑（写进运维须知）：

1. `WinUHid.dll` 必须放在 `shengjian-helper.exe` **同目录**（`LoadLibraryW` 首先搜 exe 目录），
   否则 `K ?` 只回 `K no dll_missing`；
2. 应用必须用 `pnpm tauri build` 构建（带 `custom-protocol` 特性）。`cargo build --release`
   产物仍走 devUrl，运行时会显示「127.0.0.1 拒绝连接」。

## 发货前还差（未做）

1. **打包**：`WinUHid.dll`（MIT）随助手一起分发，版权声明进 `THIRD_PARTY_NOTICES.md`；
2. **驱动安装入口**：设置页加「驱动管理」（运行随包官方安装器，或自签 MSI），失败要能明确
   提示"未安装 → 虚拟键盘后端不可用"；
3. **卸载**：root 设备 + 证书 + 驱动包一起清（官方 MSI 的卸载 CA 可复用其流程）；
4. **设置项**：显示虚拟键盘后端可用性/版本（`virtual_hid_ready driver_version=`），
   未安装时不让选。
