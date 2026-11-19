# 第三方组件与许可

本项目自身以 **GPL-3.0-only** 发布，完整文本见 [LICENSE](LICENSE)。

## Frida Gadget

- 组件：`frida-gadget`（Windows x86_64）
- 版本：`17.18.0`
- 随包文件：`resources/hid-host/frida-gadget.dll`
- 来源：https://github.com/frida/frida/releases/tag/17.18.0
- 完整性：
  - 压缩包 SHA-256：`2e549d3b77bc939b83b9a368655e90b7e566bd56b635a10fc3fab68d8dbe173d`
  - 解压后 SHA-256：`350beb0e801dc7dc39d21512960d1048b9f21dc72c0ee5ced5cf5d9dc8ea6687`
- 许可：**wxWindows Library Licence, Version 3.1**
- 许可全文：本仓库 `helper/hid-host/vendor/frida-gadget-COPYING`，并随安装包 `resources/licenses/COPYING.frida-gadget` 分发
- 说明：该许可的 exception notice 允许以二进制形式随产品分发；本产品未修改 Gadget 本身。

版本与摘要的唯一事实来源：`helper/hid-host/vendor/frida-gadget.lock.json`。

## WinUHid（虚拟 HID 键盘客户端）

- 组件：`WinUHid.dll`（Windows x86_64 客户端；对应驱动是 root 枚举的 `Root\WinUHid`，UMDF2 + 系统内置 VHF）
- 版本：`1.0.0.0`
- 随包文件：`resources/hid-host/WinUHid.dll`（必须与 `shengjian-helper.exe` 同目录，助手用 `LoadLibraryW` 加载）
- 来源：https://github.com/cgutman/WinUHid（上游没有 release 资产；当前副本取自 MoteDeck 0.2.8 随包，同一份上游构建）
- 完整性：SHA-256 `2990e3583149333f1566a1ad3715a87ffa4cac1345798c50ce30ba659140cfc7`
- 许可：**MIT License**（Copyright (c) 2025 Cameron Gutman）
- 许可全文：本仓库 `helper/hid-host/vendor/winuhid-LICENSE`，并随安装包 `resources/licenses/COPYING.winuhid` 分发
- 说明：MIT 允许以二进制形式随产品分发，需保留版权与许可声明；与项目自身的 GPL-3.0-only 兼容。
- **驱动包随包分发但不自动安装**：`WinUHidDriver.inf` / `.dll` / `.cat` / `WinUHidCertificate.cer`
  位于 `resources/hid-host/winuhid-driver/`，只有用户在设置页点击安装驱动时，才由提权助手执行
  `install-driver.ps1`（信任官方 CA、放驱动、创建 root 设备并绑定驱动）。驱动包与客户端 DLL
  同为上游 MIT 许可。驱动缺失时助手回 `K no driver_unavailable`，主程序自动回落报告层。

版本与摘要的唯一事实来源：`helper/hid-host/vendor/winuhid.lock.json`。

## Maple Mono NF CN（品牌字标轮廓）

- 组件：`Maple Mono NF CN Bold`（Maple Mono 字族，Copyright (c) 2022 subframe7536）
- 版本：未固定。字标在构建时用本机安装的 Maple Mono NF CN Bold 提取轮廓，生成参数记录在 `brand/tools/wordmark-maple.json`；
  运行时与随包分发都不依赖该字体。
- 来源：https://github.com/subframe7536/maple-font
- 用途：只用于生成「声键 / SHENGJIAN」字标轮廓（`brand/logo/*.svg`），已转曲为路径。
- 随包情况：**不随包分发字体文件**；许可声明随安装包分发到 `resources/licenses/LICENSE-OFL.txt`。
- 许可：**SIL Open Font License 1.1**
- 许可全文：本仓库 `brand/LICENSE-OFL.txt`

## 自动更新使用的 Rust crate

应用内更新只新增了三个直接依赖；它们静态链接进桌面宿主，不单独随包分发文件：

- `ureq` `3.5.2` — MIT OR Apache-2.0 — https://github.com/algesten/ureq
- `native-tls` `0.2.18` — MIT OR Apache-2.0 — https://github.com/sfackler/rust-native-tls
- `sha2` `0.10.9` — MIT OR Apache-2.0 — https://github.com/RustCrypto/hashes

完整依赖树与版本以 `apps/shengjian-desktop/src-tauri/Cargo.lock` 为准；各 crate 的许可全文随
Cargo 源码包提供。其余 Rust 依赖（Tauri、serde 等）沿用项目原有分发方式。

## 项目代码来源

代码来源、设备 profile 参考和命名清理说明见 [ATTRIBUTION.md](ATTRIBUTION.md)。

## 分发要求

- 发布二进制时同时提供本文件、[ATTRIBUTION.md](ATTRIBUTION.md)、`LICENSE`、Frida Gadget 许可全文和品牌字标 OFL 声明（`brand/LICENSE-OFL.txt`，安装包内为 `resources/licenses/LICENSE-OFL.txt`）。
- 发布 Release 时附上对应源码（GitHub tag 的源码归档即可）。
