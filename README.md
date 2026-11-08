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

- 版本：`0.1.0`（首个公开版本）
- GitHub Release 0.1.0：已发布（CI 构建、未签名），安装包 `shengjian_0.1.0_x64-setup.exe`；清单见 `release/0.1.0/manifest.json`
- 代码签名：已接入 **SignPath Foundation** 免费开源签名（release workflow 内置，配置后自动启用）；见 [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md) 与 [docs/signing.md](docs/signing.md)
- 真机 RC003 已验证：连接、语音、按键手势、profile 自动切换、`run-macro`、返回键增强捕获
- 本地发布清单：`release/0.1.0/manifest.json`
- 发布说明：`docs/release-notes-0.1.0.md`
- GitHub 仓库已创建并推送：<https://github.com/xiaopohair/SHENGJIAN>；发布由 `v*` tag 触发的 Release workflow 生成
- 不包含自动更新器

## 功能

### 桌面宿主

`apps/shengjian-desktop`：Tauri 2 + Vue 3，in-process 使用 core。

- 界面：无边框自定义标题栏、`Ctrl+K` 命令面板
- 连接页：扫描、连接、断开、多设备切换、记住上次设备、状态、电量、连接详情、当前设备音频/语音工具覆盖、RC003 增强捕获与 helper
- 托盘：显示/隐藏、打开设置、打开日志目录、退出；tooltip 显示连接/电量；中英文菜单随语言即时切换；图标随任务栏明暗使用亮松青 / 松青·深两版
- 语音页：voice/audio 状态、实时电平、语音测试、语音输入工具、自定义语音热键、转写历史开关
- 按键页：profile 列表、编辑、激活、导入/导出；宽屏列表 + 右侧编辑器，未保存导航确认
- Profile 编辑器：应用目标（支持选择程序/用当前前台）、当前前台填充、快捷键捕获、自动方案冲突提示、保存校验、按住连发、Profile 语音工具覆盖、复制方案、从模板新建、11 键（含 TV）；常用语入口按平台能力显示
- 宏页：创建/编辑/删除、8 种动作、多步骤拖拽排序/复制/超时、版本历史与单版本删除、复制、测试/取消/重试、分步运行结果、搜索、导入/导出、中文校验提示；宽屏列表 + 右侧编辑器
- 历史页：语音事件、按键事件、宏运行、转写记录四个标签；语音/按键按本次运行保留，宏运行/转写持久保存
- 设置页：全局音频、主题、强调色、应用语言（中文/English/跟随系统）、开机自启、关闭行为、配置备份/恢复（settings、profiles、宏库、转写历史、外观偏好、上次设备）、日志查看/诊断包、健康检查、恢复默认、分区选项卡；设置加载失败时仍可进入数据与诊断
- 可访问性：页面 landmark/标题层级、表单 label、Tabs/命令面板/确认框焦点圈定、装饰图标隐藏；详见 `docs/ui-accessibility-report.md`
- 关于页：品牌标记（随明暗切换亮松青 / 松青·深）、应用版本、平台/构建/签名、运行信息、本机能力、支持的遥控器与平台状态、组件与许可；可打开日志目录、重新运行首次配置
- 托盘、首次配置向导（含语音输入工具选择）、单实例
- Profile/宏导入导出、诊断日志入口

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

`apps/shengjian-desktop/src-tauri/target/release/bundle/nsis/shengjian_0.1.0_x64-setup.exe`

只跑 `pnpm tauri build` 也能构建，但产出的是中文文件名；发布、CI 与下载链接统一用 `shengjian_<version>_x64-setup.exe`。

安装包包含：

- 桌面 app
- `resources/rc003/shengjian-helper.exe`
- `resources/rc003/frida-gadget.dll`
- `resources/licenses/`：GPL、ATTRIBUTION、第三方许可、Frida Gadget 许可全文、品牌字标 OFL 声明（`LICENSE-OFL.txt`）

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

- 根 workspace：`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace` 通过（**689 passed**，其中 `shengjian-windows` 平台库 490）
- 桌面 Rust：**12 passed**
- 桌面前端：**21 passed**
- 桌面 E2E（真实安装包 + WebView2 CDP）：**4 passed**
- 本地构建：`shengjian_0.1.0_x64-setup.exe` 生成，`prepare-release.ps1` / `verify-release.ps1` 通过
- 真机安装 / 启动 / 卸载：安装目录、快捷方式、卸载项、版本号清理正确；用户数据 `%LOCALAPPDATA%\Shengjian` 保留
- 安装向导：欢迎页侧栏、页头品牌图、中文界面与「声键 安装」标题正常
- 真机 RC003：
  - scan → connect `ready`
  - 语音 `streaming → draining → idle`
  - OK `single / double / long`
  - Profile 前台自动切换
  - 实体按键 `run-macro`（下键打开 Notepad）
  - 返回键增强捕获 `back` edge

## 本地发布

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/prepare-release.ps1 -Version 0.1.0
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-release.ps1 -Version 0.1.0
```

两个脚本从 `src-tauri/tauri.conf.json` 读取 `productName` 再拼安装包路径，品牌改名后不需要改脚本。

GitHub Release 0.1.0（CI 构建）：

```text
version=0.1.0
size=10785931 bytes
sha256=60A7FA3CB06B64E729F67FFADD7D0409A37F2BAB9157BCF8BFC95BDDF82C86ED
signed=false
published=true
```

0.1.0 的 SHA-256 同步写入 `docs/release-notes-0.1.0.md` 与 `release/0.1.0/SHA256SUMS`。该清单对应 GitHub Release 的 CI 构建产物；本地重新构建的包大小/哈希可能不同。

## GitHub 发布准备

路线：**public 仓库、x64、手动下载、不自动更新**；通过 SignPath Foundation 做免费开源签名。

- `.github/workflows/release.yml`：`v*` tag 触发构建与 Release；配置 SignPath 变量/密钥后自动签名
- Release 标题为「声键 &lt;version&gt;」；Tauri 先按 `productName` 产出 `声键_<version>_x64-setup.exe`，workflow 随后调用 `scripts/normalize-installer-name.ps1` 归一化为发布文件名 `shengjian_<version>_x64-setup.exe`
- 签名说明：[CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md)、[docs/signing.md](docs/signing.md)
- 资产：安装包、`manifest.json`、`SHA256SUMS`、`LICENSE`、`ATTRIBUTION.md`、`THIRD_PARTY_NOTICES.md`、`brand/LICENSE-OFL.txt`、Frida 许可全文
- 首次安装 helper 需要一次 UAC
- 未启用签名时仍会触发 SmartScreen；Release 提供 SHA-256 供校验
- 安全说明：[SECURITY.md](SECURITY.md)
- 发布流程：[docs/release-process.md](docs/release-process.md)

仓库已推送至 <https://github.com/xiaopohair/SHENGJIAN>；发布由 tag 触发 `.github/workflows/release.yml` 生成，当前 tag 为 `v0.1.0`。

## 已知限制

- 仅 Windows x64；ARM64 包未提供
- 0.1.0 为未签名发布包；CI 签名需仓库公开并通过 SignPath 审核
- 无自动更新
- helper 首次安装需要 UAC
- RC001/ARN9 身份映射仍 deferred，见 [docs/device-identity.md](docs/device-identity.md)
- 无第二个客户端 IPC；桌面宿主为 in-process

## Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org). 详见 [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md)；隐私政策见 [PRIVACY.md](PRIVACY.md)。

## 许可证与来源

项目以 **GPL-3.0-only** 发布，见 [LICENSE](LICENSE)。

来源、设备 profile 参考和命名清理见 [ATTRIBUTION.md](ATTRIBUTION.md)。
第三方组件与 Frida Gadget 许可见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
