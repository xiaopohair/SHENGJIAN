# 声键 0.1.0 发布说明

日期：2026-11-08

首个公开版本。**声音，就是快捷键。**

## 亮点

- **连接**：扫描、连接、记住设备、断线重连、多设备切换；支持 RC003（BLE / ATVV）。
- **语音**：按住遥控器语音键说话，文字落到当前光标；实时电平、语音测试、自定义语音热键。
- **按键**：11 键映射、单击 / 双击 / 长按、按住连发；按前台应用自动切换方案；宏把一串动作压成一个键。
- **桌面壳**：Tauri 2 + Vue 3，7 个页面、托盘、无边框标题栏、`Ctrl+K` 命令面板。
- **本地优先**：不要求账号；语音交给用户选择的输入工具，声键不上传语音内容，转写历史默认关闭。
- **品牌**：声键（SHENGJIAN）名称、字标与键帽标志；应用图标、安装向导、深/浅色界面与托盘图标统一。浅色主题使用松青·深（#0F7A63）标记与托盘，避免浅底颜色过浅。

## 安装

```text
shengjian_0.1.0_x64-setup.exe
大小：10785048 bytes
SHA-256：072221ABB1FAC0B1BF79FE2B93C31525B934A36E1A82AEEE364E845E22CDC2FC
```

SHA-256 同时写入 `release/0.1.0/SHA256SUMS`，可用 `scripts/verify-release.ps1 -Version 0.1.0` 复核。

安装包当前**未签名**，SmartScreen 可能提示；后续通过 SignPath Foundation 签名，见 `CODE_SIGNING_POLICY.md` 与 `docs/signing.md`。

RC003 的返回键 / 音量键增强捕获需要一个提权 helper：首次在连接页勾选并点击「安装/更新 helper」时确认一次 UAC。

## 已知限制

- 仅 Windows x64；未提供 ARM64 包。
- 本地包未签名、未发布；无自动更新。
- RC001 / ARN9 设备身份映射尚未支持，见 `docs/device-identity.md`。
- 无第二个客户端 IPC；桌面宿主为 in-process。

## 许可与归属

- 项目代码：**GPL-3.0-only**，见 `LICENSE`。
- 品牌：「声键」名称与标志由项目作者保留；品牌手册文本 CC BY 4.0；字标轮廓来自 Maple Mono NF CN Bold（SIL OFL 1.1，见 `brand/LICENSE-OFL.txt`，安装包内为 `resources/licenses/LICENSE-OFL.txt`）。
- 第三方组件：Frida Gadget（wxWindows Library Licence），见 `THIRD_PARTY_NOTICES.md`。
- 来源与命名清理说明见 `ATTRIBUTION.md`。
