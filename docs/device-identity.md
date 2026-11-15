# 设备身份：RC001 / RC003 / ARN9

状态：**ARN9 已解冻（2026-11-12 真机证据）**；**RC001 已完成真机探测（2026-11-15），型号身份与按键布局均已确认（12 键，含 TV，与 RC003 一致）**。

## 结论先行

- **RC003（小米蓝牙语音遥控器 2 Pro）**：Windows 真机验收通过。
- **ARN9（小米蓝牙语音遥控器 普通款）**：2026-11-12 完成真机探测，**确认为独立型号**，已在代码里与 RC003 分开（`RemoteModel::Arn9`）。
- **RC001（小米蓝牙语音遥控器 2）**：2026-11-15 完成真机探测，型号号读出 `RC001`，**不是 ARN9 的同一型号**（同硬件族、不同 SKU）；**按键布局与 RC003 一致（12 键，含 TV），2026-11-15 用户实机确认**；profile 已由 `research` 转为 `implemented`；语音会话亦已在 2026-11-15 实测通过（两次真实会话，输入法均判定为已响应）。

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

## 2026-11-15 真机证据（RC001）

用项目自身的 WinRT GATT 路径（`examples/gatt_probe`，`enum` 模式）读取本次新配对的 RC001，
地址 `e8:55:40:e1:0b:c5`。可重跑：

```powershell
cargo run --release -p shengjian-windows --example gatt_probe -- enum --mac E8:55:40:E1:0B:C5 --out rc001-enum.txt
```

| 字段 | RC001（本次） | ARN9（2026-11-12） | RC003（2026-11-12） |
| --- | --- | --- | --- |
| 广播名 | 小米蓝牙语音遥控器 | 同 | 同 |
| **型号号 `0x2A24`** | **`RC001`** | `ARN9` | `RC003` |
| 制造商 `0x2A29` | `MIOM` | `MIOM` | `MIOM` |
| 序列号 `0x2A25` | `250513` | `230324` | `250519` |
| 硬件版本 `0x2A27` | `V2.0` | `V2.0` | `V2.0` |
| 固件版本 `0x2A26` | `2671` | `2671` | `2671` |
| 软件版本 `0x2A28` | **`A.6.0.3`** | `A.4.2.0` | `A.7.0.6` |
| 电量 `0x2A19` | 100%（可 notify） | 95% | 63% |
| PnP ID `0x2A50` | VID `0x2717`(10007) / PID `0x32B8`(12984) / REV `0x00A4` | 同 | 同 |
| 电池服务额外特征 `0x2BED` | **无** | **无** | **有** |
| GATT 服务数 | 9 | 9 | 9 |

服务集合与 ARN9 / RC003 完全一致：GAP `0x1801`、GATT `0x1800`、BAS `0x180F`、
DIS `0x180A`、HID `0x1812`、ATVV `AB5E0001`（`0002` write / `0003` notify /
`0004` notify）、小米厂商服务 `8A7A0001`、`0x01BF`、`0xFE59`（Mi Service，
版本特征读回 `Version1.0`，应用未使用）。

本次顺带读到的、此前两份档案都没记录的字段：

| 特征 | 值 | 含义 |
| --- | --- | --- |
| Appearance `0x2A01` | `0x03C1` | HID 键盘类 |
| PPCP `0x2A04` | `10 00 10 00 31 00 58 02` | 连接间隔 20 ms、从机延迟 49、超时 6 s |
| Central Address Resolution `0x2AA6` | `00` | 不支持；配对后地址保持固定公共地址 |
| System ID `0x2A23` | `"SystemID"` | 占位字符串，不是真实 IEEE 地址 |
| IEEE `0x2A2A` | `"IEEE"` | 占位字符串 |

### 2026-11-15 语音链路实测（RC001）

12:03 用遥控器语音键跑了两次真实会话（微信输入法，`voice_trigger_mode=hold`）：

| 会话 | 时长 | 采样 | 峰值 | 有效值 | 削波采样 | 输入法响应 |
| --- | --- | --- | --- | --- | --- | --- |
| 4 | 2.4 s | 36,000（15.0 kHz） | 0.0 dBFS | −15.7 dBFS | 159 | `verdict=reacted` |
| 5 | 7.2 s | 108,720（15.0 kHz） | 0.0 dBFS | −18.2 dBFS | 421 | `verdict=reacted` |

- **语音链路通**：ATVV 音频按 15.0 kHz 到达（设备标称 16 kHz，属时钟偏差），两次都真的拉起了微信输入法。
- **第一次为什么没反应**：声键里存的自定义语音热键是 `Ctrl+Ctrl`，微信输入法自己的语音快捷键是 `Ctrl+Win`（安装包 `config/voice-tool-presets.json` 里 wechat 的 `defaultHotkey`），输入法完全不响应；清空自定义热键、回落到工具默认值后通过。**这一条日志无法自证**：微信输入法 3.0.2.44 已不写 ConsentStore，存活标记通道也不可用，所以只会看到 `wetype_check skipped reason=observation_unavailable` 与 `verdict=unknown`，只能靠「输入法有没有弹语音条」肉眼确认。
- **增益实测（2026-11-15 12:13–12:18，同一次坐姿，三款各按三档）**：`voice_level` 记的是**增益之后**的电平（送进虚拟线的信号），把增益减回去就得到设备源电平：

| 设备 | 增益 | 峰值 | 有效值 | 削波 |
| --- | --- | --- | --- | --- |
| RC003 | 13（三次） | 0 / −4.2 / 0 | −20.2 / −23.5 / −10.2 | 102 / 0 / 1 |
| RC001 | 0 / 12 / 24 | −17.4 / −5.4 / 0 | −39.9 / −26.9 / −15.2 | 0 / 0 / 695 |
| ARN9 | 0 / 12 / 24 | −9.0 / −11.8 / 0 | −39.2 / −30.1 / −17.3 | 0 / 0 / 236 |

- **源电平（rms）**：RC001 ≈ −39 dBFS（三次 −39.9 / −38.9 / −39.2，线性很好）；ARN9 ≈ −41 ~ −39；**RC003 ≈ −33 ~ −36，最热，比另两款高 6–10 dB**，13 dB 下已经开始削波。
- **RC003 那一轮增益没生效**：三次会话的日志都是 `audio_gain ... gain_db=13`，说明 0/12/24 的调整没有到达内核，RC003 的 0/12/24 仍缺数据。
- **跨批次不可比**：11:58–12:05 那批 RC001 在 13 dB 下的源 rms ≈ −28 ~ −31，比本次高约 8 dB，两批之间只有说话距离/音量不同；ARN9 两批一致（≈ −39）。选增益必须用同一次坐姿内的数据。
- **设备级覆盖优先于全局，且界面没有提示（2026-11-15 复现并验证）**：RC003 两轮共六次会话，无论怎么拖「设置页 → 音频」的滑杆，日志里全是 `gain_db=13`。原因是这条设备覆盖：`devices["xiaomi-rc003"].windows.gainDb = 13` —— 它由**连接页设备卡「设置」抽屉里的「独立音频设置」**写入（`ConnectionPage.vue` 的 `saveConnectionSettings`），连接时 `apply_effective_audio_settings` 用 `document.resolve(device_id, platform_id)` 解析，**设备级覆盖优先**（`settings_namespace.rs` 的 `resolves_global_then_device_override`），全局值被遮住而界面没有任何提示。
  验证：把覆盖改成 10、全局保持 12 后重启 —— 连接瞬间快照变成 `gainDb=10`（覆盖生效）；再把全局拖到 16，`settings.json` 里 `global.gain_db` 变成 16，但核心快照仍是 10，说明覆盖开启时全局滑杆确实无效。结论：**「按设备设增益」早就支持**，入口是抽屉里的「独立音频设置」，只是全局音频页看不出来。
- **结论（范围）**：三台在一次坐姿内的源电平都在 −39 ~ −41 dBFS 附近；0 dB 时有效值约 −39（偏轻），24 dB 必然削波，10–13 dB 是可用区间（RC003 在 10 dB 下两次峰值 −2.7 / −4.4 dBFS、削波 0）。**0–24 dB 的范围合适，不需要引入负增益**；要补的是「全局滑杆被设备覆盖遮住」的可见性。

### 结论：RC001 的型号位置

- **不是 ARN9**：型号号 `RC001` ≠ `ARN9`，软件版本 `A.6.0.3` 独立于 `A.4.2.0` /
  `A.7.0.6`，序列号也不同。此前「RC001 与 ARN9 是否同一硬件」的疑问，答案是
  **同一硬件族、不同 SKU**，不能把 RC001 当成 ARN9。
- **电池路径与 ARN9 一致**：同样没有 `0x2BED` 额外电池特征（RC003 有），
  即 GATT notify 优先、失败回退 60 秒缓存轮询的行为可以直接沿用 ARN9 的实测结论。
- **按键布局与 RC003 一致**：用户实机确认本机遥控器**有 TV 键**（2026-11-15），12 键布局与
  RC003 相同，`profile_buttons_for_model("RC001")` 走全量 `PROFILE_BUTTONS` 即可，不需要像 ARN9 那样特例化。
- **与 RC003 同 PnP**：`VID 0x2717 / PID 0x32B8 / REV 0x00A4` 三者一致，Windows 把它们
  归到同一个 HID 硬件 ID，设备管理器里同名（小米蓝牙语音遥控器），只能靠 GATT 型号号区分。
- **HID 报告描述符仍拿不到**：HID 服务（`0x1812`）的用户态特征枚举全部
  `GattCommunicationStatus::Unreachable`（Windows HID 驱动独占），`DeviceInformation`
  选择器路径（命中 6 个接口）的 `FromIdAsync` 也返回空对象；改用 HID API 时
  `HidP_GetCaps` 只能读到顶层集合 usage（Generic Desktop / Keyboard），
  `IOCTL_HID_GET_REPORT_DESCRIPTOR` 在无读权限句柄下返回 `ERROR_INVALID_FUNCTION`、
  `GENERIC_READ` 打开被拒（`ERROR_ACCESS_DENIED`）。因此**按键映射仍只能走 Raw Input
  + 增强捕获**，按键集合无法从软件侧静态读出（TV 键已由用户实机确认为有）。

## 仍未解决
- **ARN9 的 macOS 支持**：upstream 记录已验收，但本仓库未验证，profile 里保持 `planned`。

## 参考

- `profiles/xiaomi-rc001.json`、`profiles/xiaomi-rc003.json`
- `crates/shengjian-windows/src/lib.rs`：`remote_model_from_model_number` / `RemoteModel`
- `crates/shengjian-contract/src/button_profile.rs`：`profile_buttons_for_model`
- `docs/plan-device-model-profiles.md`：按型号隔离方案的实施计划
- upstream `nijez/open-voice-bridge`：`device-profiles/xiaomi-arn9.json`、`device-profiles/xiaomi-rc003.json`
