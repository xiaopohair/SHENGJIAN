# 声键 0.5.1 发布说明

日期：2026-11-12

`0.5.1` 是 0.5.0 的收尾版本：新增「启动后隐藏到系统托盘」设置，并把转写历史相关文案统一为「语音记录」。

## 亮点

- **新增：启动后隐藏到系统托盘**
  - 位于「设置 → 启动与窗口」，紧跟在「开机自动启动」之后，默认关闭
  - 打开后启动时不弹出主窗口，应用只在系统托盘常驻；点托盘菜单的「显示主窗口」「打开设置」，或再次双击图标都能打开主窗口
  - 与「关闭窗口时隐藏到托盘」分工明确：前者管启动时，后者管关闭窗口时
  - 复用已有设置字段，无需数据迁移；升级后默认行为仍是启动即显示主窗口

- **文案统一：转写历史 → 语音记录**
  - 设置项、历史页标签与空状态、页面说明、备份与恢复默认提示、错误提示统一为「语音记录」
  - 英文文案同步为 Voice recordings
  - 数据含义不变：仍然只保存转写文字、不保存音频

- **工程质量**
  - 前端测试可在 Node 26 上运行：Vitest worker 关闭实验性 Web Storage，CI 使用的 Node 24 行为不变
  - Prettier 改为 `endOfLine: "auto"`，Windows 上 CRLF 检出不再误报格式问题

## 验证

- `cargo fmt --all -- --check` 通过
- `cargo test --workspace`：**670 passed**（其中 `shengjian-windows` 平台库 493）
- 桌面 Rust：`cargo test` **14 passed**
- 桌面前端：`pnpm lint`、`pnpm format:check`、`pnpm test`（**61 passed**）、`pnpm build` 通过
- 本地构建：`shengjian_0.5.1_x64-setup.exe` 生成，`prepare-release.ps1` / `verify-release.ps1` 通过

## 安装

Release 资产：

```text
shengjian_0.5.1_x64-setup.exe      安装包（NSIS，未签名）
shengjian_0.5.1_x64-portable.zip   便携版（免安装 zip）
SHA256SUMS                         两个产物的 SHA-256
manifest.json                      安装包大小与 SHA-256
portable-manifest.json             便携版大小与 SHA-256
```

下载后用随包的 `SHA256SUMS` 校验：

```powershell
Get-FileHash shengjian_0.5.1_x64-setup.exe -Algorithm SHA256
# 与 SHA256SUMS 中对应的行比对
```

校验值以 Release 里的 `SHA256SUMS` 与 `manifest.json` 为准，本文不硬编码，
避免与实际上传的构建产物不一致。

安装包未签名，Windows SmartScreen 可能提示；首次安装 RC003 helper 需要一次 UAC。覆盖安装会保留 `%LOCALAPPDATA%\Shengjian` 下的设置、方案、宏、语音记录与日志。

> 便携版请解压到普通目录（桌面、D 盘目录等）后运行，不要直接在受限目录里运行。

## 升级说明

- 覆盖安装会保留 `%LOCALAPPDATA%\Shengjian` 下的设置、方案、宏、语音记录与日志
- 新增的「启动后隐藏到系统托盘」默认关闭，从 0.5.0 升级后启动行为不变
- 「语音记录」只是界面文案变更，原转写历史数据继续沿用，无需迁移

## 已知限制

- 仅 Windows x64；未提供 ARM64 包
- 0.5.1 为未签名发布包
- 无自动更新
- 方案尚未按设备型号隔离（RC003 与 RC001 共用同一份方案列表），
  计划见 `docs/plan-device-model-profiles.md`
- 「聚焦应用输入框」需要先在按键方案里配置过对应应用的聚焦方式，否则步骤会报缺少依赖
- RC001 / ARN9 设备身份映射仍为 deferred
