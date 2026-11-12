# 来源与归属

本项目以 GPL-3.0-only 发布（见 [LICENSE](LICENSE)），是遥控器语音/按键桥接核心的独立抽取项目。

## 代码来源

- `crates/shengjian-domain/src/*` 与 `crates/shengjian-windows/src/*` 主要复制自原始 Windows 实现：
  `https://github.com/GetSayAll/remote-mic-app-windows`，源提交 `52305ce`（本轮迁移时核对的工作树提交）。
  原仓库为 GPL-3.0-only；复制后保留许可证与文件内容，仅做命名清理和 crate 拆分。
- 原始 Windows 实现参考了 macOS 产品 `https://github.com/HD838A/remote-mic-app` 的产品/协议/测试语义；本项目没有复制 SwiftUI/CoreBluetooth/CoreAudio/IOHID 代码。
- `crates/shengjian-domain/src/gestures.rs` 是新的最小单/双/长按策略；它参考现有 Windows `button_gestures` 的时序语义（双击 300ms、长按 550ms），不是逐行复制。
- `helper/rc003` 复制自原始 Windows 实现的提权 helper，保留其原生 FFI、协议和计划任务边界；本轮只重命名包/二进制与内部标识。

## 设备 Profile

- `profiles/schema.json`、`profiles/xiaomi-rc003.json`、`profiles/xiaomi-rc001.json`、`profiles/xiaomi-arn9.json` 参考 `https://github.com/nijez/open-voice-bridge` 的 `specs/device-profile.schema.json` 与 `device-profiles/*.json`，以及原始 Windows 实现的设备 profile。原项目均为 GPL-3.0-only。
- RC003 的 VID/PID、ATVV、能力事实来自上述已记录资料；RC001 的 profile 是草案，身份未与 upstream `xiaomi-arn9` 对齐，不得作为运行时匹配依据。

## 架构参考

- `https://github.com/nijez/open-voice-bridge` 的六层架构、DeviceModule/DeviceProfile 设计、Windows Python/Qt 候选的模块边界，是本项目契约/端口/设备 profile 的参考。本项目不引入其 Python/Qt 运行时代码或依赖。
- `https://github.com/HD838A/remote-mic-app` 是原始 macOS 产品与行为基准。本项目不使用其 App Logo、SwiftUI、CoreBluetooth、CoreAudio、IOHID 或 Apple 发布链。

## 命名清理

本轮将原产品相关的命名统一改为中性命名：

- crate：`shengjian-*`；
- 环境变量：`SHENGJIAN_*`；
- helper：`shengjian-helper`；
- 日志/临时文件/计划任务/命名事件：`shengjian-*`；
- 便携组合动作格式：`shengjian-transfer`（旧格式不保证兼容）。

## 品牌资产

- **「声键」名称与标志**（`brand/BRAND.md`、`brand/logo/*`、`brand/png/*`、`brand/app-icon.ico`、`brand/social/*`）：
  由项目作者保留权利；在提及本项目时可按品牌手册使用，其他用途需授权。GPL-3.0 的代码许可不构成商标或品牌授权。
- **品牌手册文本**（`brand/BRAND.md`、`brand/brand-board.html`）：CC BY 4.0。
- **字标轮廓**：来自 Maple Mono NF CN Bold（Maple Mono 字族，Copyright (c) 2022 subframe7536），
  SIL OFL 1.1；仓库只包含转曲轮廓，不附带字体文件，许可全文见 [brand/LICENSE-OFL.txt](brand/LICENSE-OFL.txt)。
- **界面字体**：使用系统字体（Microsoft YaHei UI / Segoe UI），不随包分发字体文件。
- 应用图标、托盘图标、社交预览图由 `brand/tools/` 中的脚本按 `brand/BRAND.md` 的规格生成。

## 未包含的资产

- 上游产品的 App Logo / App Icon 等专有品牌资产不随本项目复制；本项目使用自有「声键」品牌资产（见上节）。
- 原 Windows 仓库的 Tauri 外壳、Vue UI、安装器、更新器、会员/私有组件不在本项目内。

## 参考链接

- https://github.com/HD838A/remote-mic-app
- https://github.com/nijez/open-voice-bridge
- https://github.com/GetSayAll/remote-mic-app-windows
