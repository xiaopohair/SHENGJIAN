# 设备身份：RC001 / RC003 / ARN9

状态：**ARN9 已解冻（2026-11-12 真机证据）**；RC001 仍为 `research`。

## 结论先行

- **RC003（小米蓝牙语音遥控器 2 Pro）**：Windows 真机验收通过。
- **ARN9（小米蓝牙语音遥控器 普通款）**：2026-11-12 完成真机探测，**确认为独立型号**，已在代码里与 RC003 分开（`RemoteModel::Arn9`）。
- **RC001（小米蓝牙语音遥控器 2）**：仍保持 `profiles/xiaomi-rc001.json` 的 `research` 状态，不进入运行时匹配。

## 2026-11-12 真机证据（两台设备对比）

用项目自身的 WinRT GATT 路径读取，两台设备同时配对在同一台机器上。

| 字段 | 新（ARN9）`f4:22:7a:aa:a8:4a` | 旧（RC003）`c0:5d:39:f8:56:6a` |
| --- | --- | --- |
| 广播名 | 小米蓝牙语音遥控器 | 小米蓝牙语音遥控器 |
| **型号号 `0x2A24`** | **`ARN9`** | **`RC003`** |
| 制造商 `0x2A29` | `MIOM` | `MIOM` |
| 序列号 `0x2A25` | `230324` | `250519` |
| 硬件版本 `0x2A27` | `V2.0` | `V2.0` |
| 固件版本 `0x2A26` | `2671` | `2671` |
| 软件版本 `0x2A28` | `A.4.2.0` | `A.7.0.6` |
| 电量 `0x2A19` | 95% | 63% |
| PnP ID `0x2A50` | VID `0x2717`(10007) / PID `0x32B8`(12984) | 完全相同 |
| 电池服务额外特征 `0x2BED` | **无** | **有** |
| GATT 服务数 | 9 | 9 |

两者的服务集合完全一致，都含 **ATVV 语音服务** `AB5E0001`
（`0002` write / `0003` notify / `0004` notify）——与 `ble.rs` 里
`SERVICE_UUID` / `TRANSMIT_UUID` / `AUDIO_UUID` / `CONTROL_UUID` 完全相同。

### ARN9 按键实测（2026-11-12）

| 按键 | 捕获路径 | usage | 结果 |
| --- | --- | --- | --- |
| 上/下/左/右 | Raw Input | HID 键盘 usage | 映射正确 |
| Home | Raw Input | HID 键盘 usage | 映射正确 |
| Menu | Raw Input | HID 键盘 usage | 映射正确 |
| 音量+ / 音量- | 增强捕获 | `0x0080` / `0x0081` | 捕获正确 |
| 返回 | 增强捕获 | `0x00F1` | 捕获正确 |
| 语音键 | ATVV + F5 抑制 | — | 语音链路通过 |

语音链路实测：会话 2.49 秒、36480 采样、peak −2.7 dBFS、无削波，
识别文本入库且 `source = BluetoothRemote`。

## 结论：ARN9 与 RC003 的关系

**硬件等价，型号不同。**

- 应用实际使用的全部能力（ATVV 语音、9 个 GATT 服务、导航键 HID 码、
  音量/返回的增强捕获 usage）在两者上**完全一致**。
- 唯一实质差异：**ARN9 没有 TV 键**，以及型号号/软件版本不同。

因此代码的处理是：

| 层面 | 处理 |
| --- | --- |
| 型号识别 | `RemoteModel::Arn9`，`remote_model_from_model_number("ARN9")` 返回它（不再映射成 `Rc003`） |
| 型号标签 | `"ARN9"`，作为**按键方案隔离**的依据 |
| 设备档案 `device_id` | `xiaomi-arn9` → `profiles/xiaomi-arn9.json`（2026-11-12 补齐） |
| 按键布局 | `profile_buttons_for_model("ARN9")` 返回 11 键（去掉 TV） |
| 产品图 / 热区 | `xiaomi-arn9.png`（514×1640）独立标定；连接页用 `xiaomi-arn9-line.png` |
| 产品正式名称 | 「小米蓝牙语音遥控器」 |

## 仍未解决

- **RC001 与 ARN9 是否同一硬件**：未验证。RC001 profile 保持 `research`。
- **ARN9 的 macOS 支持**：upstream 记录已验收，但本仓库未验证，profile 里保持 `planned`。

## 参考

- `profiles/xiaomi-rc001.json`、`profiles/xiaomi-rc003.json`
- `crates/shengjian-windows/src/lib.rs`：`remote_model_from_model_number` / `RemoteModel`
- `crates/shengjian-contract/src/button_profile.rs`：`profile_buttons_for_model`
- `docs/plan-device-model-profiles.md`：按型号隔离方案的实施计划
- upstream `nijez/open-voice-bridge`：`device-profiles/xiaomi-arn9.json`、`device-profiles/xiaomi-rc003.json`
