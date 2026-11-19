# 声键 0.9.6 发布说明

日期：2026-11-19

> 本文件在 `v0.9.6` tag 推送时由 release workflow 逐字发布（`body_path`）。

## 本版重点：应用内自动更新

0.9.6 是**首个带更新器的版本**。安装版启动后会检查 GitHub Releases，发现新版本可在后台
下载安装包，校验 SHA-256 后由用户确认重启安装。

- 检查：启动后 24 小时限频；关于页和设置页都可手动检查，支持预发布版本筛选。
- 下载：在后台下载到 `%LOCALAPPDATA%\Shengjian\Updates`，边下边算 SHA-256；大小或哈希与
  Release 的 `manifest.json` 不一致会直接丢弃，不会进入可安装状态。
- 安装：关于页横幅显示「下载更新 / 重启并安装」，安装前二次确认；确认后应用退出，由
  NSIS 的 `/P /UPDATE /R` 关闭旧实例、覆盖文件并重新启动。
- 设置：新增「设置 → 软件更新」，可分别关闭自动检查、自动下载，并控制是否包含预发布版本。

## ⚠️ 从 0.9.5 及更早版本升级

**0.9.5 及更早版本没有更新器，无法自动发现 0.9.6。** 请先手动下载并安装一次 0.9.6；
从 0.9.6 起，后续新版本才能自动升级。

## 修复

- NSIS 安装包的 preinstall / uninstall hook 仍指向改名前的 `resources\rc003` 与
  `Shengjian RC003 Helper`，覆盖安装时停不掉正在运行的 helper、改名也找错路径。
  已改为 `resources\hid-host` 与 `Shengjian HID Host`，并保留旧路径作为兼容分支。
- 自动更新安装时，preinstall hook 会先停掉计划任务并把被占用的 helper exe/dll 改名挪开，
  避免文件锁导致升级失败。

## 说明与边界

- 只有 NSIS 安装版支持应用内升级；便携版和开发版只提供手动下载入口。
- 更新校验的信任锚是 HTTPS + Release 内 `manifest.json` 的 SHA-256；当前安装包仍未签名，
  更新器暂不校验 Authenticode。
- 便携版与安装版共用数据目录：设置、方案、宏、历史保留在 `%LOCALAPPDATA%\Shengjian`；
  已下载的更新包位于其 `Updates/` 子目录。

## Release 资产

- `shengjian_0.9.6_x64-setup.exe` — NSIS 安装包（安装版，支持后续自动升级）
- `shengjian_0.9.6_x64-portable.zip` — 便携版（手动更新）
- `manifest.json` / `portable-manifest.json` — 大小与 SHA-256
- `SHA256SUMS` — 两个产物的校验和
- 许可证与第三方声明文件

校验请以下载后的 `SHA256SUMS` 为准，本文不硬编码哈希。
