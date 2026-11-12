# Apple Siri Remote 适配评估（暂缓）

状态：**评估完成，暂不适配（2026-11-12）**。资源已备好，等将来有需要再启动。

## 结论

**不做。** 决定因素是**语音拿不到**：声键的核心价值是「语音输入 + 按键映射」，而 Siri Remote
在 Windows 侧没有任何可用的语音方案（详见「语音为什么不可得」）。

其余理由：能拿到的能力只到「按键 + 触控板当鼠标」，用户装个第三方工具就能实现；而 2/3 代要稳定
还得绕开 Windows 原生蓝牙栈、自己实现 BLE central，工程量与收益严重不匹配。

## 型号谱系

| 代次 | 型号 | FCC ID | 发布 | 蓝牙 | 布局 / 差异 |
| --- | --- | --- | --- | --- | --- |
| 第 1 代 | A1513（2017 修订版 A1962） | BCGA1513 | 2015-10-30 | 4.0 | 玻璃多点触控板（上 1/3）、Menu 键、无 TV/电源/静音 |
| 第 2 代 | **A2540** | BCGA2540 | 2021-05-21 | 5.0 | 圆形可按触控板、返回键、新增电视电源 + 静音、Siri 键移到侧边 |
| 第 3 代 | **A2854** | BCGA2854 | 2022-11-04 | 5.0 | **与第 2 代硬件一致，仅 Lightning → USB-C** |
| 第 4 代 | **A3592** | BCGA3592 | 2026-10-13 | **6.0** | 新增 Find My 精确定位 + 内置扬声器 |

第 4 代的 A3592 由 FCC 文件确认：`BCGA3592`，Model `A3592`，设备类别
`Wireless Remote Control`，频段 2402–2480 MHz，授权日期 2026-10-13（同日还有
A3593 数字媒体设备、A3374 智能音箱等 Apple TV 产品线送检）。

**A2540 与 A2854 除充电口外完全相同** —— 若将来适配，一款工作量可覆盖两台。

## 按键清单（以 A2854 产品图为准）

电源、麦克风孔（Siri）、触控板（四方向靠滑动 + 中央可按）、返回、TV/主屏幕、
**播放/暂停**、音量 +、**静音**、音量 −。

与小米三款的差异：**没有 Menu 键**，但多出播放/暂停与静音。

## 协议现状（社区逆向成果）

上游 Linux 项目 `siri-remote`（Rust，BLE HID-over-GATT）已完整逆向：

| 报告 | 用途 |
| --- | --- |
| `0xFA` | 麦克风音频：Opus CELT-only WB，20 ms 帧，48 kHz 单声道 |
| `0xFB` | 系统按键：2 字节掩码，13 位（仅 gen-3 布局） |
| `0xFC` | 触控帧：11 字节（单指）/ 18 字节（双指），含 7 字节接触椭圆尾 |
| `0x2A19` / `0x2A1A` | 电量 / 电源状态（0xAB 充电、0xAF 放电、0xBB 已插电） |

- **解锁**：遥控器默认完全静默，必须先向每个 HID Output report 写 `0xAF`
- **广告指纹**：Apple 厂商前缀 `07 0D 02 15 03 02 <6 字节身份地址> 4F 50 50` + HID 服务 UUID `0x1812`
- 蓝牙地址每约 20 秒轮换一次

Windows 侧现成实现：`Transient-Onlooker/AppleRemoteWin`（WPF .NET 10）

- 第 1 代（A1513/A1962）走**原生 Windows HID**：`HidBthLE` + `HidD_SetFeature`（`0xF0 = 0xAF`）+ 异步 `ReadFile`
- A2854 走**独立路径**：专用 Barrot USB 蓝牙适配器 `33FA:0001` 挂 WinUSB，自实现 BLE central +
  SMP 加密 + DPAPI bond，绕开原生栈
- 真机记录：约 **100 秒**真实运行、按键与真实 Windows 鼠标移动成功；另有一次约 83 秒、
  两个 HID CCCD 正常关闭、断开原因 `0x16`
- 稳定性仍未证明：间歇 `0x13` peer 错误、Stop 时 `0x08` supervision timeout

## 语音为什么不可得

- Windows 侧的 A2854 实现**主动放弃**语音，安全规范明确写 `Never subscribe to or log microphone 0xFA`
- 第 1 代实现明确 `Voice (Siri) not decoded`
- 上游 Linux 项目**能**取到（`siri-remote mic` 输出到 PipeWire），说明硬件可行 —— 但 Windows 两个
  实现都没做，且 `0xFA` 位于加密链路上

结论：语音不是「难」，是「被放弃」，没有可参考的 Windows 实现。

## 若将来要做：先做 30 分钟前置验证

在投入开发前，只需要验证一件事：**Windows 原生栈能否解锁第 2/3 代**。

1. 用 Windows 蓝牙正常配对 A2540 或 A2854
2. 设备管理器确认出现 HID 设备（VID `004C`）
3. 写 feature report `0xF0 = 0xAF`，看能否收到 `0xFB` 按键报告

- **成功** → 原生 HID 路线可行，成本可控，值得继续
- **失败** → 确认必须走 WinUSB 那套，直接放弃

## 若真要做，需要改的代码（已核实）

| 阻塞 | 位置 | 说明 |
| --- | --- | --- |
| 连接硬依赖 ATVV | `crates/shengjian-windows/src/ble.rs` `service_discovery` | 找不到 `ab5e0001` 就 `VoiceServiceMissing` 失败，Siri Remote 永远连不上 |
| 扫描按名字白名单过滤 | `crates/shengjian-windows/src/lib.rs` `is_supported_remote_name` | 非白名单设备被 `continue` 跳过，列表里根本不出现 |
| 缺按键类型 | `crates/shengjian-contract/src/model.rs` `LogicalButton` | 没有 `PlayPause` |
| Mute 不可编辑 | `crates/shengjian-contract/src/button_profile.rs` `PROFILE_BUTTONS` | `VolumeMute` 是 base-owned，方案里改不了 |

需要的改动方向：引入**「无语音设备」的连接语义**（HID-only 会话，不碰 ATVV 管线，小米三款零影响），
新增 `PlayPause`，并让 Mute 对这类设备可编辑。

## 备用资源

| 文件 | 说明 |
| --- | --- |
| `apps/shengjian-desktop/src/assets/devices/apple-siri-a2854.png` | A2854 正面产品图，335×1081 |
| `apps/shengjian-desktop/src/assets/devices/apple-siri-a2854-line.png` | A2854 线图 |

A2540 与 A2854 硬件一致，这两张图可直接复用。尚未接入任何界面、也未标定热区。

## 参考

- FCC：`BCGA1513` / `BCGA2540` / `BCGA2854` / `BCGA3592`
- `https://github.com/Transient-Onlooker/AppleRemoteWin`（Windows 实现与 A2854 交接文档）
- `https://github.com/Transient-Onlooker/siri-remote-for-windows`（Linux 上游的存档快照，含协议说明）
- Wikipedia「Siri Remote」条目（型号 / 代次 / 规格对照）
