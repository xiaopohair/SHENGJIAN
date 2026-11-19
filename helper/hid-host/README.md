# hid-host（全按键支持 helper）

Windows 提权助手：小米遥控器返回键/音量± 的增强捕获、Chromecast 在 HOGP 驱动进程内的
按键报告读取与转发，以及虚拟键盘（WinUHid）和驱动管理。迁移自原始 Windows 实现。

## 构建

```powershell
cd helper/hid-host
cargo build --release
```

产物：`target/release/shengjian-helper.exe`。

## 运行边界

- 独立 Cargo workspace，不参与顶层 `cargo test --workspace`；在 `helper/hid-host` 内单独跑
  `cargo test`。
- 只负责需要管理员权限或必须进入驱动进程的能力；基础语音路径不依赖它。
- 计划任务名 `Shengjian HID Host`；安装/卸载/重启用 `--install-task` / `--remove-task`，
  桌面端入口在「设置 → 音频及语音」。
- 资源与安装步骤见 [docs/hid-host-setup.md](../../docs/hid-host-setup.md)；
  自动更新时安装包 preinstall hook 会先停任务并挪开被占用的 exe/dll。
