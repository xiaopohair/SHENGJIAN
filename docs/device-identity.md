# 设备身份：RC001 / RC003 / ARN9

状态：**deferred（2026-11-04 用户决定先跳过）**

## 结论先行

- 当前已真机验收的设备是 **RC003（小米蓝牙语音遥控器 2 Pro）**。
- **RC001（小米蓝牙语音遥控器 2）** 仍保持 `profiles/xiaomi-rc001.json` 的 `research` 状态，不进入运行时匹配。
- **ARN9** 是 upstream 使用的普通款型号标识；不能仅凭名称或现有代码认定它等于 RC003。
- 现有代码 `remote_model_from_model_number` 把 `"ARN9"` 映射为 `Rc003`，与 upstream `xiaomi-arn9.json` 冲突；**暂不改动**，等待真机证据。

## 三个标识

| 标识 | 产品/含义 | 当前状态 |
| --- | --- | --- |
| RC001 | 小米蓝牙语音遥控器 2，标准款 | Windows 基础支持存在；profile `research` |
| RC003 | 小米蓝牙语音遥控器 2 Pro | Windows 真机验收通过；当前连接设备 |
| ARN9 | Xiaomi Bluetooth Voice Remote，普通款 | upstream macOS 已验收；无 TV 键；Windows planned |

## 为什么无法直接合并/区分

1. 蓝牙名重叠：都可能显示“小米蓝牙语音遥控器”。
2. HID 身份重叠：upstream 记录 ARN9 与 RC003 同为 `VID 10007 / PID 12984`。
3. 型号号冲突：现有 Windows 代码把 `ARN9` 当作 `Rc003`，upstream 却把 ARN9 定义为普通款。
4. RC001 与 ARN9 是否同一硬件，尚未确认。

## 需要什么证据才能解冻

在真实设备上采集：

- GATT Model Number String（`0x2A24`）
- HID product string / report descriptor
- 是否存在 TV 键
- ATVV capability / 语音变体
- 设备实际广播名与型号号组合

## 解冻后的处理顺序

1. 用证据确认 ARN9 归属：RC001 还是独立 profile。
2. 修正 `remote_model_from_model_number` 的 `"ARN9" -> Rc003`（如果需要）。
3. 更新 `profiles/xiaomi-rc001.json` / 新增 `xiaomi-arn9.json`。
4. 为 RC001/ARN9 分别跑 Windows 真机验收。
5. 只有通过后才把 profile 改为 `implemented` 并进入运行时匹配。

## 当前不做什么

- 不修改现有 RC003 映射。
- 不把 RC001 profile 升为 `implemented`。
- 不用单元测试或名称猜测替代真机证据。
- 不阻塞 host/core 的其他工作。

## 参考

- `profiles/xiaomi-rc001.json`
- `profiles/xiaomi-rc003.json`
- `crates/shengjian-windows/src/lib.rs` 中的 `remote_model_from_model_number`
- upstream `nijez/open-voice-bridge`：`device-profiles/xiaomi-arn9.json`、`device-profiles/xiaomi-rc003.json`
