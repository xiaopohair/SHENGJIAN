# 安全说明

## 报告问题

请通过 GitHub 仓库的 **Security advisory** 私下报告安全问题，不要在公开 issue 中贴日志中的设备路径、令牌或个人信息。

## 安装边界

- 安装包未签名；Windows SmartScreen 可能提示未知发布者。请只用官方 Release 页面并核对 SHA-256。
- RC003 返回键/音量键增强捕获需要一次 UAC 安装 `Shengjian RC003 Helper` 计划任务。
- helper 以最高权限运行，但只在 RC003 报告层做目标 usage 拦截；源码在 `helper/rc003`。
- 未实现自动更新；请手动下载新版本并校验。

## 日志

诊断日志位于 `%LOCALAPPDATA%\Shengjian\Logs`，不包含语音内容或 HID 路径。提交日志前请确认已移除个人信息。
