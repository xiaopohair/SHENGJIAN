# 声键 0.2.0 发布说明

日期：2026-11-09

`0.2.0` 在 0.1.0 的基础上补齐了启动体验、便携分发与首次配置，并集中修复了设备状态与主题相关的问题。

## 亮点

- **启动体验**：新增品牌 splash 窗口，初始化完成后再显示主窗口，避免白屏；文案跟随应用语言，首次启动跟随系统语言。
- **免安装便携版**：新增 `shengjian_0.2.0_x64-portable.zip`，解压即可运行；与安装版共用单实例标识和 `%LOCALAPPDATA%\Shengjian` 数据目录。
- **连接页**：
  - 设备状态显示型号、固件版本、电量、重连次数与电源通知
  - 支持 RC001 / RC003 产品图与线稿，连接状态和主题自动切换
  - 扫描候选会从连接快照回填型号，修复「型号一直未知」
  - 当前设备可单独覆盖音频输入设备、语音输入工具与增益
- **首次配置**：
  - 五步向导：连接遥控器 → 扫描设备 → 语音测试 → 按键测试 → 完成
  - 语音测试支持波形覆盖层、成功/失败结果，以及语音输入工具、音频输入设备选择
  - 按键测试逐个显示 11 个按键的可用状态
  - 集成 RC003 增强捕获开关与 helper 安装/更新；开关立即生效
  - 可从「关于」重新运行，不再刷新页面、不再闪深色主题
- **界面与托盘**：
  - 无边框标题栏、圆角主面板、侧栏折叠按钮与导航对齐
  - 主内容滚动条无操作时隐藏
  - 深/浅色主题使用不同托盘图标，托盘图标随主题即时刷新
  - 修复深色主题被整体反色的问题
- **文案**：统一「设置 / 方案 / 音频输入设备 / 语音输入工具 / RC003 增强捕获服务」等用词，清理首次配置和设置页冗余提示。

## 安装

```text
shengjian_0.2.0_x64-setup.exe
大小：11019233 bytes
SHA-256：F16910F1FB0D767BB95108A849716F95EFC6C86B157EC367A124E846C421767C
```

便携版：

```text
shengjian_0.2.0_x64-portable.zip
大小：14483536 bytes
SHA-256：45186F1200797988CB4A033595C16159F4181C6BD2E049140F9BCD8D1E026DA1
```

SHA-256 同时写入 `release/0.2.0/SHA256SUMS`，可用 `scripts/verify-release.ps1 -Version 0.2.0` 复核。

GitHub Release：<https://github.com/xiaopohair/SHENGJIAN/releases/tag/v0.2.0>。安装包当前**未签名**，SmartScreen 可能提示；后续通过 SignPath Foundation 签名，见 `CODE_SIGNING_POLICY.md` 与 `docs/signing.md`。

RC003 的返回键 / 音量键增强捕获需要一个提权 helper：首次在连接页或首次配置中勾选并点击「安装/更新」时确认一次 UAC。

## 已知限制

- 仅 Windows x64；未提供 ARM64 包。
- 0.2.0 为未签名发布包；无自动更新。
- 便携版依赖系统 WebView2 Runtime；移动目录后 RC003 helper 计划任务需重新安装。
- RC001 / ARN9 设备身份映射仍未进入运行时匹配，见 `docs/device-identity.md`。
- 无第二个客户端 IPC；桌面宿主为 in-process。

## 许可与归属

- 项目代码：**GPL-3.0-only**，见 `LICENSE`。
- 品牌：「声键」名称与标志由项目作者保留；品牌手册文本 CC BY 4.0；字标轮廓来自 Maple Mono NF CN Bold（SIL OFL 1.1，见 `brand/LICENSE-OFL.txt`，安装包内为 `resources/licenses/LICENSE-OFL.txt`）。
- 第三方组件：Frida Gadget（wxWindows Library Licence），见 `THIRD_PARTY_NOTICES.md`。
- 来源与命名清理说明见 `ATTRIBUTION.md`。
