<p align="center">
  <img src="brand/logo/shengjian-app-icon.svg" width="112" alt="声键">
</p>

<h1 align="center">声键 · Windows 语音遥控器</h1>

<p align="center">
  <a href="README.en.md">English</a> | 简体中文
</p>

<p align="center">
  <strong>声音，就是快捷键。</strong>
</p>

<p align="center">
  <code>语音输入</code> · <code>按键映射</code> · <code>宏与方案</code>
</p>

仓库：<https://github.com/xiaopohair/SHENGJIAN>

声键（SHENGJIAN）是一款面向 Windows 的语音输入应用：把一支蓝牙语音遥控器变成 Windows 输入设备，开口即输入，按键即执行。它支持语音输入、自定义快捷键、自定义宏与按键映射方案，可搭配微信输入法、豆包输入法、Vokie、ChatterFly 等主流 AI 语音输入法使用，也能在 ChatGPT、Claude、DeepSeek Harness 等主流 Agent 应用中直接语音输入、用宏触发操作，为 Vibe Coding 与日常办公打造个性化的语音工作流。

品牌定位、Slogan、LOGO 与视觉规范见 [brand/BRAND.md](brand/BRAND.md)。

## 当前状态

- 版本：`0.9.7`（0.9.7 修复按键测试触发方案动作；0.9.6 新增应用内自动更新；0.9.4 新增 Google Chromecast TV Remote 支持；0.9.5 修复 0.9.4 的两处语音全局失效——后台组件误吞 3 字节 I/O、微信输入法被排到虚拟键盘路径）
- 已支持设备：RC003（小米蓝牙语音遥控器 2 Pro）、RC001（小米蓝牙语音遥控器 2）、ARN9（小米蓝牙语音遥控器）、
  **Google Chromecast TV Remote**（型号 `CHROMECAST`，14 键）；Windows x64
- GitHub Release 0.9.7：<https://github.com/xiaopohair/SHENGJIAN/releases/tag/v0.9.7>（CI 构建、未签名），
  安装包 `shengjian_0.9.7_x64-setup.exe`、便携包 `shengjian_0.9.7_x64-portable.zip`；校验值以 Release 内的 `SHA256SUMS` 为准
- 代码签名：已接入 **SignPath Foundation** 免费开源签名（release workflow 内置，配置后自动启用）；见 [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md) 与 [docs/signing.md](docs/signing.md)
- 真机已验证：RC003（连接、语音、按键手势、方案自动切换、运行宏、返回键增强捕获）；ARN9（连接、ATVV 语音链路、导航键、音量/返回增强捕获）；RC001（GATT 身份、12 键布局含 TV、两次真实语音会话、TV 键唤起常用语面板）；Chromecast（连接、14 键映射与注入、ATVV 语音 16 kHz、静音 / 音量 / 回车、配对指南与方案页热区）
- 本地发布清单：`release/<version>/manifest.json`（本地构建时生成，线上资产以 Release 为准）
- 发布说明：`docs/release-notes-0.9.7.md`；0.9.6 的自动更新说明与 0.9.5 的事故复盘见对应 release notes
- GitHub 仓库已创建并推送：<https://github.com/xiaopohair/SHENGJIAN>；发布由 `v*` tag 触发的 Release workflow 生成
- 自动更新（0.9.6 起）：安装版启动后自动检查 GitHub Releases，发现新版本可在后台下载安装包，校验 SHA-256 后由用户确认重启安装；便携版与开发版不支持应用内升级，仍需手动下载。0.9.5 及更早版本没有更新器，升级到 0.9.6 仍需手动安装一次

## 设备支持

| 型号 | 产品 | 按键数 | 按键通道 | 语音 | 默认按键方案 |
| --- | --- | --- | --- | --- | --- |
| `RC003` | 小米蓝牙语音遥控器 2 Pro | 12（含 TV / 菜单） | Raw Input + 增强捕获 | ATVV | 方向 / OK / 返回 / 音量± |
| `RC001` | 小米蓝牙语音遥控器 2 | 12（含 TV / 菜单） | Raw Input + 增强捕获 | ATVV | 同上 |
| `ARN9` | 小米蓝牙语音遥控器 | 11（无 TV） | Raw Input + 增强捕获 | ATVV | 同上 |
| `CHROMECAST` | Google Chromecast TV Remote | 14（含静音 / YouTube / Netflix / 信源） | 助手在 HOGP 驱动内读取 | ATVV | 确定→回车、返回→Esc、方向键、主页→Ctrl+T、电源→F11、音量±/静音→系统音量 |

## 功能

### 桌面宿主

`apps/shengjian-desktop`：Tauri 2 + Vue 3，in-process 使用 core。

- 界面：无边框自定义标题栏（拖拽区 + 窗口控制，品牌只在侧栏出现一次）、圆角主内容面板、启动居中、`Ctrl+K` 命令面板；顶栏集中显示连接状态、设备名、电量、当前输入法与语音/音频状态胶囊
- 启动 splash：品牌启动画面覆盖初始化过程，设置与主题就绪后切换到主窗口，避免白屏闪烁；启动画面文案跟随应用语言设置，未设置时跟随系统语言
- 连接页：设备信息卡（自定义设备昵称、产品名 · 型号、电量 / 已连接时长 / 固件、蓝牙/语音/全按键支持状态胶囊、顶部强调色渐变边）、扫描连接断开与多设备切换、记住上次设备；「设置」打开右侧设备设置抽屉（设备名称、独立音频开关、增益、语音输出设备、语音输入工具）；「忘记设备」先断开再解除系统配对，扫描列表按相对时间（3 天前 / 1 分钟前）标出上次连接，型号按设备令牌记忆；增益与输入法展示「实际生效」值
- 托盘：显示/隐藏、打开设置、打开日志目录、退出；tooltip 显示按键方案 / 遥控器 / 输入法 / 电量四行状态；中英文菜单随语言即时切换；托盘使用深色底应用图标，深浅任务栏下都清晰
- 语音页：voice/audio 状态、实时电平、语音测试、语音输入工具、自定义语音热键、语音记录开关
- 按键页：方案列表、编辑、激活；宽屏工作台 + 表格视图，未保存导航确认；导入 / 导出合并进「更多操作」菜单（单文件整库 / 单个·多个文件、逐文件真实进度、导入冲突预览）
- 推荐应用、推荐按键方案、语音输入工具、推荐宏由 `config/*.json` 配置驱动（随安装包下发，可在发布前或安装目录中编辑，见 [docs/config-catalogs.md](docs/config-catalogs.md)）；推荐方案只读，复制后成为可编辑的用户方案
- 方案编辑器：应用目标（支持应用选择器：预设/已安装/UWP/浏览文件；用当前前台）、当前前台填充、快捷键捕获、自动方案冲突提示、保存校验、按键级按住连发、方案语音工具覆盖、复制方案、按键数按型号（小米三款 12 键含 TV 与菜单，Chromecast 14 键含静音 / YouTube / Netflix / 信源）；常用语（内置，设置页维护多条，按键后弹出列表选择）
- 设备测试：顶栏命令面板可随时打开「语音测试 / 按键测试」，只读、不写设置，按键测试期间也不执行当前方案动作——语音落字用应用内原生测试板（输入法能正常落字、主窗口不重载），按键按型号列出应有按键并标出已检测/缺失
- 顶栏监听开关：一枚「监听中 / 已暂停」胶囊，可随时暂停或恢复按键监听（语音与蓝牙连接不受影响）；从设置页返回时顶栏的输入法与增益会同步为「实际生效」值
- 断连恢复：未连接时按遥控器任意键会自动连回上次那台；用户主动「断开」后不再自动连，重启时仍会尝试连接上次设备
- 宏页：导航栏统一为「试运行 / 保存 / 添加 / 属性 / 更多操作」（复制宏、删除宏、导入导出收进更多菜单，编辑工具栏已取消）；新建宏直接落盘成草稿；三栏编辑器（步骤列表 / 当前步骤 / 动作参数）、拖动排序（跟随指针的鬼影 + 强调色插入线）、步骤复制与超时、快捷键点选、宏属性抽屉、测试/停止、分步运行结果、单/多文件导入导出与按条进度、中文校验提示
- 历史页：语音事件、按键事件、宏运行、语音记录四个标签；语音/按键按本次运行保留，宏运行/语音记录持久保存
- 设置页：全局音频、主题、强调色、应用语言（中文/English/跟随系统）、开机自启、启动后隐藏到系统托盘、关闭行为、软件更新（自动检查 / 自动下载 / 预发布筛选）、配置备份/恢复（设置、方案、宏库、语音记录、外观偏好、上次设备）、日志查看/诊断包、健康检查、恢复默认、分区选项卡；设置加载失败时仍可进入数据与诊断
- 可访问性：页面 landmark/标题层级、表单 label、Tabs/命令面板/确认框焦点圈定、装饰图标隐藏；详见 `docs/ui-accessibility-report.md`
- 关于页：深色底应用图标、应用版本、平台/构建/签名、运行信息、本机能力、支持的遥控器与平台状态、组件与许可；可打开日志目录、重新运行首次配置；启动后台检查 GitHub Releases（24 小时限频），发现新版本时用**关于页置顶更新横幅**（版本对比 + 下载进度 + 重启并安装 + 忽略此版本）、品牌色 Toast、侧栏「关于」脉动红点和托盘菜单提示，支持预发布筛选；安装版自动下载安装包并校验 SHA-256，安装前弹出确认，重启后由 NSIS 完成覆盖升级
- 配对帮助：关于页支持型号行和首次配置扫描页均可打开；按 RC003 / RC001 / ARN9 / Google Chromecast TV Remote 四个型号展示组合键与 Windows 蓝牙步骤（小米三款为「主页 + 菜单」，Chromecast 为「返回 + 主页」，其蓝牙名为 `Chromecast Remote`；ARN9 与 Chromecast 无 TV 键，不显示 TV 初始化提示）
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
- `helper/hid-host`：独立 workspace 的提权助手（增强捕获、按键读取与转发、虚拟键盘、驱动管理；0.9.3 由 `rc003-helper` 改名）

## 项目结构

```text
crates/                       # core、contract、domain、testkit、Windows 平台与 adapter
apps/shengjian-windows-host/
apps/shengjian-desktop/  # Tauri 2 + Vue 3 桌面宿主
  src-tauri/src/host.rs       # 平台无关 HostPlatform 抽象
  src-tauri/src/platform/     # 平台实现（当前 windows）
helper/hid-host/              # 提权助手：增强捕获 / 按键读取转发 / 虚拟键盘（独立 workspace）
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

`apps/shengjian-desktop/src-tauri/target/release/bundle/nsis/shengjian_0.9.7_x64-setup.exe`

只跑 `pnpm tauri build` 也能构建，但产出的是中文文件名；发布、CI 与下载链接统一用 `shengjian_<version>_x64-setup.exe`。

安装包包含：

- 桌面 app
- `resources/hid-host/shengjian-helper.exe`
- `resources/hid-host/frida-gadget.dll`
- `resources/hid-host/WinUHid.dll` 与 `resources/hid-host/winuhid-driver/`（驱动由用户按需安装）
- `resources/licenses/`：GPL、ATTRIBUTION、第三方许可（含 WinUHid）、Frida Gadget 许可全文、品牌字标 OFL 声明（`LICENSE-OFL.txt`）

### 便携版（免安装 zip）

```powershell
cd apps/shengjian-desktop
pnpm build:portable
```

产物：`apps/shengjian-desktop/src-tauri/target/release/portable/shengjian_<version>_x64-portable.zip`，校验信息写入 `release/<version>/portable-manifest.json`。

解压后运行 `shengjian-desktop.exe` 即可，无需安装。需要系统已安装 WebView2 Runtime（Windows 11 自带；Windows 10 多数机器已有）。设置、宏、历史与日志仍保存在 `%LOCALAPPDATA%\Shengjian`；助手的计划任务指向当前目录，移动目录后需重新安装，删除目录前请以管理员身份运行 `resources\hid-host\shengjian-helper.exe --remove-task`。便携版与安装版共用单实例标识和数据目录，不能同时运行。便携版不支持应用内自动更新，新版本需重新下载 zip 覆盖。

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

## 提权助手（增强捕获 / 按键读取）

助手做三件事：小米遥控器的返回键与音量±在 Windows 的 HID→VK 映射阶段会被系统丢弃，
需要它接管（增强捕获）；Chromecast 的按键报告完全不进系统的 Raw Input 层，需要它在 HOGP 驱动进程内读取后转发；
以及虚拟键盘（WinUHid）与驱动管理。

首次使用（小米遥控器）：

1. 打开「设置 → 音频及语音」，在「全按键支持服务」里点击安装 / 更新（确认 UAC）
2. 在同一分组里打开「全按键支持」开关（按电脑生效而不是按遥控器）
3. 助手计划任务由 app 启动/停止；连接页设备卡上的「全按键支持 · 已安装」胶囊显示服务状态

Chromecast 只是**读取按键**，不需要「全按键支持」开关；它的按键走同一条助手通道，
所以这两个能力共用同一个 helper 与同一份资源目录。

细节见 [docs/hid-host-setup.md](docs/hid-host-setup.md)、[docs/chromecast-remote.md](docs/chromecast-remote.md)。

## 验证

- 根 workspace：`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace` 通过（**789 passed**）；`cargo clippy --workspace --all-targets` 0 error
- 桌面 Rust：`cargo fmt -- --check`、`cargo test` 通过（**28 passed，2 ignored**；两个 ignored 是需要真实 GitHub 网络的更新烟测）
- 桌面前端：`pnpm lint`、`pnpm format:check`、`pnpm exec vitest run`（**254 passed**）、`pnpm build`、`pnpm config:check` 通过
- 本地构建：安装包按 `productName` 生成后由 `scripts/normalize-installer-name.ps1` 归一化；本版本发布包由 CI 产出（`shengjian_0.9.7_x64-setup.exe`），`prepare-release.ps1` / `verify-release.ps1` 流程未改动
- 自动更新：Release manifest 拉取、约 13 MB 安装包下载与 SHA-256 校验已实测；2026-11-19 用发布资产完成 0.9.5 → 0.9.6 覆盖安装，`/P /UPDATE /R` 重启后版本为 0.9.6，安装版被正确识别为可自动升级环境
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
- 真机 Google Chromecast TV Remote：
  - 连接后显示为 Google Chromecast TV Remote · `CHROMECAST`（固件 0.0.1、电量读取正常）
  - 14 个按键全部产生语义边沿并按型号默认方案注入（确定→回车、返回→Esc、方向键、主页→Ctrl+T、电源→F11、音量±/静音→系统音量）
  - ATVV 语音：16 kHz 满速，`voice_audio_rate result=full`
  - 配对指南与方案页产品图 / 热区对位正确

## 本地发布

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/prepare-release.ps1 -Version 0.9.7
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-release.ps1 -Version 0.9.7
```

两个脚本从 `src-tauri/tauri.conf.json` 读取 `productName` 再拼安装包路径，品牌改名后不需要改脚本。
`prepare-release.ps1` 写 `release/<version>/manifest.json`（`version` / `file` / `sizeBytes` / `sha256` /
`builtAtUtc` / `signed`），`verify-release.ps1` 用它逐项比对本地安装包。

GitHub Release 0.9.7（CI 构建）：

```text
version=0.9.7
installer=shengjian_0.9.7_x64-setup.exe
portable=shengjian_0.9.7_x64-portable.zip
sha256/size：见 Release 内的 SHA256SUMS 与 manifest.json
signed=false
```

本地构建与 CI 构建不是逐字节相同的，大小与哈希都会不同，因此**发布说明不硬编码校验值**，
统一指向 Release 里的 `SHA256SUMS`；仓库里的 `release/<version>/manifest.json` 只记录本地构建结果。
详见 [docs/release-process.md](docs/release-process.md#发布说明与-sha-256)。

## GitHub 发布准备

路线：**public 仓库、x64、安装版应用内自动更新、便携版手动下载**；通过 SignPath Foundation 做免费开源签名。

- `.github/workflows/release.yml`：`v*` tag 触发构建与 Release；配置 SignPath 变量/密钥后自动签名
- Release 标题只写版本号 `<version>`（不带产品名）；Tauri 先按 `productName` 产出 `声键_<version>_x64-setup.exe`，workflow 随后调用 `scripts/normalize-installer-name.ps1` 归一化为发布文件名 `shengjian_<version>_x64-setup.exe`
- 便携包：签名后用 `scripts/build-portable.ps1 -SkipBuild` 组装 `shengjian_<version>_x64-portable.zip`，与安装包一起发布
- 签名说明：[CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md)、[docs/signing.md](docs/signing.md)
- 资产：安装包、便携 zip、`manifest.json`、`portable-manifest.json`、`SHA256SUMS`、`LICENSE`、`ATTRIBUTION.md`、`THIRD_PARTY_NOTICES.md`、`brand/LICENSE-OFL.txt`、Frida 许可全文
- 首次安装 helper 需要一次 UAC
- 未启用签名时仍会触发 SmartScreen；Release 提供 SHA-256 供校验
- 安全说明：[SECURITY.md](SECURITY.md)
- 发布流程：[docs/release-process.md](docs/release-process.md)

仓库已推送至 <https://github.com/xiaopohair/SHENGJIAN>；Release 列表：<https://github.com/xiaopohair/SHENGJIAN/releases>（最新已发布 `v0.9.7`）。

## 已知限制

- 仅 Windows x64；ARM64 包未提供
- 当前发布包未签名；配置 SignPath 后 CI 可签名
- 自动更新只覆盖 NSIS 安装版；便携版与开发版仍需手动下载。更新校验用 Release manifest 的 SHA-256，未引入第二把更新签名密钥（见 [docs/release-process.md](docs/release-process.md#应用内自动更新)）
- 0.9.5 及更早版本没有更新器；升级到首个带更新器的版本（0.9.6）仍需手动安装一次
- helper 首次安装需要 UAC
- 常用语面板只做「选择 + 插入」：文案与顺序在设置页维护，面板内不能编辑；面板是应用内浮层，不抢焦点，因此也不会替代系统的输入法候选框
- 无第二个客户端 IPC；桌面宿主为 in-process

## Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org). 详见 [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md)；隐私政策见 [PRIVACY.md](PRIVACY.md)。

## 许可证与来源

项目以 **GPL-3.0-only** 发布，见 [LICENSE](LICENSE)。

来源、设备配置参考和命名清理见 [ATTRIBUTION.md](ATTRIBUTION.md)。
第三方组件与 Frida Gadget 许可见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
