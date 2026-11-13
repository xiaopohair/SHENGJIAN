<picture>
  <source media="(prefers-color-scheme: dark)" srcset="brand/logo/shengjian-lockup-horizontal-dark.svg">
  <img src="brand/logo/shengjian-lockup-horizontal-light.svg" height="56" alt="声键（SHENGJIAN）">
</picture>

# 声键（SHENGJIAN）· Voice remote for Windows

[English](README.en.md) | 简体中文

**声音，就是快捷键。** · **Your voice is the shortcut.**

`语音输入` · `按键映射` · `宏与方案`

仓库：<https://github.com/xiaopohair/SHENGJIAN>

声键（SHENGJIAN）是一款面向 Windows 的语音输入应用：把一支蓝牙语音遥控器变成 Windows 输入设备，开口即输入，按键即执行。它支持语音输入、自定义快捷键、自定义宏与按键映射方案，可搭配微信输入法、豆包输入法、Vokie、ChatterFly 等主流 AI 语音输入法使用，也能在 ChatGPT、Claude、DeepSeek Harness 等主流 Agent 应用中直接语音输入、用宏触发操作，为 vibe coding 与日常办公打造个性化的语音工作流。

品牌定位、Slogan、LOGO 与视觉规范见 [brand/BRAND.md](brand/BRAND.md)。

## 当前状态

- 版本：`0.6.0`（方案按设备型号隔离；新增 ARN9 支持；三款遥控器各有产品图与实测热区）
- 已支持设备：RC003（小米蓝牙语音遥控器 2 Pro）、RC001（小米蓝牙语音遥控器 2）、ARN9（小米蓝牙语音遥控器）；Windows x64
- GitHub Release 0.6.0：<https://github.com/xiaopohair/SHENGJIAN/releases/tag/v0.6.0>（CI 构建、未签名），安装包 `shengjian_0.6.0_x64-setup.exe`；校验值以 Release 内的 `SHA256SUMS` 为准
- 代码签名：已接入 **SignPath Foundation** 免费开源签名（release workflow 内置，配置后自动启用）；见 [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md) 与 [docs/signing.md](docs/signing.md)
- 真机已验证：RC003（连接、语音、按键手势、方案自动切换、运行宏、返回键增强捕获）；ARN9（连接、ATVV 语音链路、导航键、音量/返回增强捕获）
- 本地发布清单：`release/0.6.0/manifest.json`（记录本地构建结果，线上资产以 Release 为准）
- 发布说明：`docs/release-notes-0.6.0.md`
- GitHub 仓库已创建并推送：<https://github.com/xiaopohair/SHENGJIAN>；发布由 `v*` tag 触发的 Release workflow 生成
- 不包含自动更新器

## 功能

### 桌面宿主

`apps/shengjian-desktop`：Tauri 2 + Vue 3，in-process 使用 core。

- 界面：无边框自定义标题栏（左侧品牌图标与产品名称）、圆角主内容面板、启动居中、`Ctrl+K` 命令面板
- 启动 splash：品牌启动画面覆盖初始化过程，设置与主题就绪后切换到主窗口，避免白屏闪烁；启动画面文案跟随应用语言设置，未设置时跟随系统语言
- 连接页：扫描、连接、断开、多设备切换、记住上次设备、状态、型号、固件版本、电量、连接详情、当前设备音频/语音工具覆盖、RC003 增强捕获与 helper
- 托盘：显示/隐藏、打开设置、打开日志目录、退出；tooltip 显示连接/电量；中英文菜单随语言即时切换；托盘图标随任务栏明暗使用亮松青 / 墨色两版
- 语音页：voice/audio 状态、实时电平、语音测试、语音输入工具、自定义语音热键、语音记录开关
- 按键页：方案列表、编辑、激活、导入/全部方案导出；宽屏工作台 + 表格视图，未保存导航确认
- 方案编辑器：应用目标（支持选择程序/用当前前台）、当前前台填充、快捷键捕获、自动方案冲突提示、保存校验、按键级按住连发、方案语音工具覆盖、复制方案、12 键（含 TV、菜单）；常用语入口按平台能力显示
- 宏页：创建/编辑/删除、8 种动作、三栏编辑器（步骤列表 / 当前步骤 / 动作参数）、拖动排序、步骤复制、步骤超时、快捷键捕获与清除、宏属性抽屉（名称与说明）、测试/停止、分步运行结果与失败原因、导入/导出、中文校验提示
- 历史页：语音事件、按键事件、宏运行、语音记录四个标签；语音/按键按本次运行保留，宏运行/语音记录持久保存
- 设置页：全局音频、主题、强调色、应用语言（中文/English/跟随系统）、开机自启、启动后隐藏到系统托盘、关闭行为、配置备份/恢复（设置、方案、宏库、语音记录、外观偏好、上次设备）、日志查看/诊断包、健康检查、恢复默认、分区选项卡；设置加载失败时仍可进入数据与诊断
- 可访问性：页面 landmark/标题层级、表单 label、Tabs/命令面板/确认框焦点圈定、装饰图标隐藏；详见 `docs/ui-accessibility-report.md`
- 关于页：品牌标记（随明暗切换亮松青 / 松青·深）、应用版本、平台/构建/签名、运行信息、本机能力、支持的遥控器与平台状态、组件与许可；可打开日志目录、重新运行首次配置；启动后台检查 GitHub Releases（24 小时限频），发现新版本时用**关于页置顶更新横幅**（版本对比 + 查看更新 + 忽略此版本）、品牌色 Toast、侧栏「关于」脉动红点和托盘菜单提示，支持预发布筛选，不自动下载安装
- 配对帮助：关于页支持型号行和首次配置扫描页均可打开；按 RC003 / RC001 / ARN9 三个型号展示组合键、Windows 蓝牙步骤与 TV 初始化提示（ARN9 无 TV 键，不显示该提示）
- 托盘、首次配置向导（含语音输入工具选择）、单实例
- 方案/宏导入导出、诊断日志入口

### Core / 平台

- `shengjian-contract`：设备身份、逻辑事件、快照、动作、错误
- `shengjian-domain`：ATVV、ADPCM/PCM、语音管线、手势、宏、设置、统计、转写
- `shengjian-app`：端口、`CoreEngine`、事件流、用例
- `shengjian-windows`：BLE/GATT、WASAPI、Raw Input、SendInput、IME/TSF、UIA、focus、应用启动、RC003 bridge
- `shengjian-windows-adapter`：把 Windows 平台翻译成 core ports
- `shengjian-testkit`：假平台与集成测试
- `apps/shengjian-windows-host`：headless scan/run/migrate
- `helper/rc003`：独立 workspace 的提权增强捕获 helper

## 项目结构

```text
crates/                       # core、contract、domain、testkit、Windows 平台与 adapter
apps/shengjian-windows-host/
apps/shengjian-desktop/  # Tauri 2 + Vue 3 桌面宿主
  src-tauri/src/host.rs       # 平台无关 HostPlatform 抽象
  src-tauri/src/platform/     # 平台实现（当前 windows）
helper/rc003/                 # RC003 返回/音量键增强捕获 helper（独立 workspace）
profiles/                     # DeviceProfile schema 与设备事实
Testing/hardware-scripts/     # 硬件信号脚本夹具
contracts/ipc/                # Windows runtime IPC 契约夹具
brand/                        # 声键品牌规范、LOGO、应用图标与社交预览
docs/                         # 架构、路线、发布流程
release/                      # 本地发布 manifest 与校验和
scripts/                      # 发布清单/校验脚本
```

多平台结构与后续接入计划见 [docs/multi-platform.md](docs/multi-platform.md)。当前保持 monorepo，不拆分为多个 Git 仓库。

依赖方向：

```text
shengjian-contract
  ← shengjian-domain
  ← shengjian-app
  ← shengjian-testkit

shengjian-windows
  → shengjian-app ports
  ← shengjian-windows-adapter
```

## 构建与运行

### 桌面开发

```powershell
cd apps/shengjian-desktop
pnpm install
pnpm tauri dev
```

### 桌面测试

```powershell
cd apps/shengjian-desktop
pnpm test
pnpm build

cd src-tauri
cargo check
cargo test
```

### 桌面安装包

```powershell
cd apps/shengjian-desktop
pnpm build:installer
```

`pnpm build:installer` = `tauri build` + 文件名归一化。Tauri 会先按 `productName` 产出 `声键_<version>_x64-setup.exe`，随后 `scripts/normalize-installer-name.ps1` 把它改名为 ASCII 小写的发布文件名：

`apps/shengjian-desktop/src-tauri/target/release/bundle/nsis/shengjian_0.6.0_x64-setup.exe`

只跑 `pnpm tauri build` 也能构建，但产出的是中文文件名；发布、CI 与下载链接统一用 `shengjian_<version>_x64-setup.exe`。

安装包包含：

- 桌面 app
- `resources/rc003/shengjian-helper.exe`
- `resources/rc003/frida-gadget.dll`
- `resources/licenses/`：GPL、ATTRIBUTION、第三方许可、Frida Gadget 许可全文、品牌字标 OFL 声明（`LICENSE-OFL.txt`）

### 便携版（免安装 zip）

```powershell
cd apps/shengjian-desktop
pnpm build:portable
```

产物：`apps/shengjian-desktop/src-tauri/target/release/portable/shengjian_<version>_x64-portable.zip`，校验信息写入 `release/<version>/portable-manifest.json`。

解压后运行 `shengjian-desktop.exe` 即可，无需安装。需要系统已安装 WebView2 Runtime（Windows 11 自带；Windows 10 多数机器已有）。设置、宏、历史与日志仍保存在 `%LOCALAPPDATA%\Shengjian`；RC003 helper 的计划任务指向当前目录，移动目录后需重新安装，删除目录前请以管理员身份运行 `resources\rc003\shengjian-helper.exe --remove-task`。便携版与安装版共用单实例标识和数据目录，不能同时运行。

### Headless host

```powershell
cargo run -p shengjian-windows-host -- scan
cargo run -p shengjian-windows-host -- run --seconds 20
```

### 核心测试

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
```

## RC003 增强捕获 helper

返回键/音量±在 Windows HID→VK 映射阶段会被系统丢弃，必须由提权 helper 接管。

首次使用：

1. 连接页勾选“启用 RC003 增强捕获（返回键、音量键）”
2. 点击“安装/更新 helper”，确认 UAC
3. 保存设置
4. helper 计划任务由 app 启动/停止

细节见 [docs/rc003-helper-setup.md](docs/rc003-helper-setup.md)。

## 验证

- 根 workspace：`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace` 通过（**684 passed**，其中 `shengjian-windows` 平台库 494）
- 桌面 Rust：**14 passed**
- 桌面前端：`pnpm lint`、`pnpm format:check`、`pnpm test`（**66 passed**）、`pnpm build` 通过
- 本地构建：`shengjian_0.6.0_x64-setup.exe` 生成，`prepare-release.ps1` / `verify-release.ps1` 通过
- 真机安装 / 启动 / 卸载：安装目录、快捷方式、卸载项、版本号清理正确；用户数据 `%LOCALAPPDATA%\Shengjian` 保留
- 安装向导：欢迎页侧栏、页头品牌图、中文界面与「声键 安装」标题正常
- 便携版：zip 解压到含空格/中文路径后启动正常；splash 窗口出现，主窗口就绪后自动关闭
- 真机 RC003：
  - scan → connect `ready`
  - 语音 `streaming → draining → idle`
  - OK `single / double / long`
  - 方案前台自动切换
  - 实体按键「运行宏」（`run-macro`，下键打开 Notepad）
  - 返回键增强捕获 `back` edge
- 真机 ARN9（小米蓝牙语音遥控器，无 TV 键）：
  - 型号识别为 ARN9（不再被误判成 RC003），方案与 RC003 相互隔离
  - ATVV 语音链路：2.49 秒会话、36480 采样、peak −2.7 dBFS 无削波，识别结果入库
  - 导航六键（上/下/左/右/Home/菜单）映射正确
  - 音量 ± 与返回键走增强捕获（`0x80` / `0x81` / `0xF1`）

## 本地发布

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/prepare-release.ps1 -Version 0.6.0
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-release.ps1 -Version 0.6.0
```

两个脚本从 `src-tauri/tauri.conf.json` 读取 `productName` 再拼安装包路径，品牌改名后不需要改脚本。
`prepare-release.ps1` 写 `release/<version>/manifest.json`（`version` / `file` / `sizeBytes` / `sha256` /
`builtAtUtc` / `signed`），`verify-release.ps1` 用它逐项比对本地安装包。

GitHub Release 0.6.0（CI 构建）：

```text
version=0.6.0
installer=shengjian_0.6.0_x64-setup.exe
portable=shengjian_0.6.0_x64-portable.zip
sha256/size：见 Release 内的 SHA256SUMS 与 manifest.json
signed=false
```

本地构建与 CI 构建不是逐字节相同的，大小与哈希都会不同，因此**发布说明不硬编码校验值**，
统一指向 Release 里的 `SHA256SUMS`；仓库里的 `release/<version>/manifest.json` 只记录本地构建结果。
详见 [docs/release-process.md](docs/release-process.md#发布说明与-sha-256)。

## GitHub 发布准备

路线：**public 仓库、x64、手动下载、不自动更新**；通过 SignPath Foundation 做免费开源签名。

- `.github/workflows/release.yml`：`v*` tag 触发构建与 Release；配置 SignPath 变量/密钥后自动签名
- Release 标题为「声键 &lt;version&gt;」；Tauri 先按 `productName` 产出 `声键_<version>_x64-setup.exe`，workflow 随后调用 `scripts/normalize-installer-name.ps1` 归一化为发布文件名 `shengjian_<version>_x64-setup.exe`
- 便携包：签名后用 `scripts/build-portable.ps1 -SkipBuild` 组装 `shengjian_<version>_x64-portable.zip`，与安装包一起发布
- 签名说明：[CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md)、[docs/signing.md](docs/signing.md)
- 资产：安装包、便携 zip、`manifest.json`、`portable-manifest.json`、`SHA256SUMS`、`LICENSE`、`ATTRIBUTION.md`、`THIRD_PARTY_NOTICES.md`、`brand/LICENSE-OFL.txt`、Frida 许可全文
- 首次安装 helper 需要一次 UAC
- 未启用签名时仍会触发 SmartScreen；Release 提供 SHA-256 供校验
- 安全说明：[SECURITY.md](SECURITY.md)
- 发布流程：[docs/release-process.md](docs/release-process.md)

仓库已推送至 <https://github.com/xiaopohair/SHENGJIAN>；Release 列表：<https://github.com/xiaopohair/SHENGJIAN/releases>（已发布 `v0.4.1`、`v0.5.0`、`v0.5.1`、`v0.6.0`）。

## 已知限制

- 仅 Windows x64；ARM64 包未提供
- 发布包未签名；CI 签名需仓库公开并通过 SignPath 审核
- 无自动更新
- helper 首次安装需要 UAC
- RC001 身份映射仍为 `research`（不进入运行时型号匹配）；ARN9 已于 2026-11-12 解冻并接入，见 [docs/device-identity.md](docs/device-identity.md)
- 无第二个客户端 IPC；桌面宿主为 in-process

## Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org). 详见 [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md)；隐私政策见 [PRIVACY.md](PRIVACY.md)。

## 许可证与来源

项目以 **GPL-3.0-only** 发布，见 [LICENSE](LICENSE)。

来源、设备配置参考和命名清理见 [ATTRIBUTION.md](ATTRIBUTION.md)。
第三方组件与 Frida Gadget 许可见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
