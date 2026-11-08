# Device profiles

- `schema.json`：Profile v1 schema。
- `xiaomi-rc003.json`：已核对事实；Windows/macOS 均为 implemented。
- `xiaomi-rc001.json`：草案，`support.status = research`，不得用于运行时匹配。

Profile 只描述设备事实，不是驱动。只有真实设备证据、adapter、测试和平台真机门都通过后，才能改成 `implemented`。

当前阻塞：RC001 与 upstream `xiaomi-arn9` 的关系未确认；现有 Windows 代码把 `"ARN9"` 映射到 `Rc003`，需要真实 GATT `0x2A24` 与 HID VID/PID 复核。该事项已于 2026-11-04 **deferred**，完整记录见 [docs/device-identity.md](../docs/device-identity.md)。
