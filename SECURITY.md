# 安全说明

## 报告问题

请通过 GitHub 仓库的 **Security advisory** 私下报告安全问题，不要在公开 issue 中贴日志中的设备路径、令牌或个人信息。

## 安装边界

- 默认未签名；未配置 SignPath 时 CI 产物没有 Authenticode 签名，Windows SmartScreen 可能提示未知发布者。请只用官方 Release 页面并核对 SHA-256。
- 安装版支持应用内更新：只从 `github.com/xiaopohair/SHENGJIAN` 的 Release 取 `manifest.json`
  与安装包，下载后必须与 manifest 的 `sizeBytes` / `sha256` 完全一致才会进入可安装状态；
  安装前由用户确认，应用退出后由 NSIS 覆盖升级。便携版与开发版不做应用内升级。
- 更新校验的信任锚是 HTTPS + Release 内的 SHA-256；若 GitHub 账号或 Release 被攻破，
  校验值也可能被一起替换，这一点与手动下载相同。当前更新器不校验 Authenticode；
  后续可再加 Tauri updater 的独立签名密钥或 Authenticode 校验。
- 全按键支持 helper 的返回键/音量键增强捕获需要一次 UAC 安装 `Shengjian HID Host` 计划任务。
- helper 以最高权限运行，但只对目标 usage 做按键拦截；源码在 `helper/hid-host`。
- 更新包与日志同放在 `%LOCALAPPDATA%\Shengjian` 下；安装包在升级成功后由下一次下载清理。

## 日志

诊断日志位于 `%LOCALAPPDATA%\Shengjian\Logs`，不包含语音内容或 HID 路径。提交日志前请确认已移除个人信息。
