# Device profiles

- `schema.json`：Profile v1 schema。
- `xiaomi-rc003.json`：小米蓝牙语音遥控器 2 Pro（RC003）；Windows implemented，macOS planned。
- `xiaomi-arn9.json`：小米蓝牙语音遥控器 普通款（ARN9）；Windows 真机验收 implemented（2026-11-12）。
- `xiaomi-rc001.json`：小米蓝牙语音遥控器 2（RC001）；2026-11-15 真机确认型号 `RC001`、12 键（含 TV），Windows `implemented`。
- `google-chromecast.json`：Google Chromecast Remote 的设备事实研究，`support.status = research`；
  2026-11-18 真机探测（见 [docs/chromecast-remote.md](../docs/chromecast-remote.md)）：身份与 GATT 已探明，
  **语音（ATVV）实测可用**，但键盘/Consumer 的 HID 上报在该原型固件上始终为零。该 profile **不参与运行时匹配**；
  产品里的 Chromecast 支持走 `CHROMECAST` 型号路径与默认按键方案（helper 在 HOGP 驱动进程内读取 3 字节报告）。

Profile 只描述设备事实，不是驱动。只有真实设备证据、adapter、测试和平台真机门都通过后，才能改成 `implemented`。

## 型号识别注意

RC003 与 ARN9 在 Windows 上的**广播名完全一样**（都叫「小米蓝牙语音遥控器」），HID VID/PID 也相同（10007 / 12984）。因此：

- 不能靠广播名判定型号；
- 唯一可靠区分是 GATT Device Information 的型号号（`0x2A24`）：`RC003` / `ARN9`；
- `xiaomi-arn9.json` 的 `identity.bluetoothNames` 故意留空，避免名字匹配把两台设备判错。

证据与验收记录见 [docs/device-identity.md](../docs/device-identity.md)。
