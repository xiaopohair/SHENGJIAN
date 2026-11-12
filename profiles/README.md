# Device profiles

- `schema.json`：Profile v1 schema。
- `xiaomi-rc003.json`：小米蓝牙语音遥控器 2 Pro（RC003）；Windows implemented，macOS planned。
- `xiaomi-arn9.json`：小米蓝牙语音遥控器 普通款（ARN9）；Windows 真机验收 implemented（2026-11-12）。
- `xiaomi-rc001.json`：草案，`support.status = research`，不得用于运行时匹配。

Profile 只描述设备事实，不是驱动。只有真实设备证据、adapter、测试和平台真机门都通过后，才能改成 `implemented`。

## 型号识别注意

RC003 与 ARN9 在 Windows 上的**广播名完全一样**（都叫「小米蓝牙语音遥控器」），HID VID/PID 也相同（10007 / 12984）。因此：

- 不能靠广播名判定型号；
- 唯一可靠区分是 GATT Device Information 的型号号（`0x2A24`）：`RC003` / `ARN9`；
- `xiaomi-arn9.json` 的 `identity.bluetoothNames` 故意留空，避免名字匹配把两台设备判错。

证据与验收记录见 [docs/device-identity.md](../docs/device-identity.md)。
