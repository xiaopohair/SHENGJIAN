# 声键 0.3.0 发布说明

日期：2026-11-09

`0.3.0` 增加了更新检查与按型号的配对帮助，并集中优化了连接页、顶栏和设备状态的信息展示。

## 亮点

- **更新检查**：
  - 启动后延迟检查 GitHub Releases，24 小时内最多自动检查一次
  - 发现新版本时显示 Toast、关于页红点和托盘菜单提示
  - 点击提示直接打开 Release 页面
  - 支持预发布版筛选；不自动下载、不自动安装
- **配对帮助**：
  - 关于页「支持的遥控器」按型号进入
  - 首次配置扫描页可直接打开
  - 支持 RC003 / RC001 切换，展示产品图、配对组合键、Windows 蓝牙步骤和 TV 初始化提示
  - 产品图上直接圈出主页键、菜单键和 TV 键
- **连接页与顶栏**：
  - 顶栏、连接页摘要、设备状态和覆盖设置统一显示完整产品名
  - 设备状态重新布局，长产品名单独一行，不再拆行
  - 设备覆盖的语音输入工具立即保存，顶栏同步显示当前设备的生效值
  - 断开连接后不再残留上一次的固件版本和电源通知状态
- **界面修复**：
  - 命令面板层级修复，不再被页面内容遮挡
  - 连接页无提示时不再多出一层空白间距
  - 最小窗口尺寸调整为 `1000 × 640`

## 安装

以下为 GitHub Release CI 构建产物。

```text
shengjian_0.3.0_x64-setup.exe
大小：11326700 bytes
SHA-256：B6FB79FCBC5B17952762C9D45DD8B42AA5FC72B5F51ADA2F98FB1501B381ABA5
```

便携版：

```text
shengjian_0.3.0_x64-portable.zip
大小：14792199 bytes
SHA-256：23F6DEA7F74D9499B3DC5ADDE330CAF63697450A1D80DDE82D9CDAEAA9E9AA48
```

SHA-256 同时写入 `release/0.3.0/SHA256SUMS`，可用 `scripts/verify-release.ps1 -Version 0.3.0` 复核。

GitHub Release：<https://github.com/xiaopohair/SHENGJIAN/releases/tag/v0.3.0>。安装包当前**未签名**，SmartScreen 可能提示；后续通过 SignPath Foundation 签名，见 `CODE_SIGNING_POLICY.md` 与 `docs/signing.md`。

从 0.2.0 覆盖安装会保留 `%LOCALAPPDATA%\Shengjian` 下的设置、方案、宏、转写历史和日志；本次发布已在本机完成覆盖升级验证。

## 已知限制

- 仅 Windows x64；未提供 ARM64 包。
- 0.3.0 为未签名发布包；无自动更新/自动安装。
- 便携版依赖系统 WebView2 Runtime；移动目录后 RC003 helper 计划任务需重新安装。
- RC001 / ARN9 设备身份映射仍未进入运行时匹配，见 `docs/device-identity.md`。
- 无第二个客户端 IPC；桌面宿主为 in-process。

## 许可与归属

- 项目代码：**GPL-3.0-only**，见 `LICENSE`。
- 品牌：「声键」名称与标志由项目作者保留；品牌手册文本 CC BY 4.0；字标轮廓来自 Maple Mono NF CN Bold（SIL OFL 1.1，见 `brand/LICENSE-OFL.txt`，安装包内为 `resources/licenses/LICENSE-OFL.txt`）。
- 第三方组件：Frida Gadget（wxWindows Library Licence），见 `THIRD_PARTY_NOTICES.md`。
- 来源与命名清理说明见 `ATTRIBUTION.md`。
