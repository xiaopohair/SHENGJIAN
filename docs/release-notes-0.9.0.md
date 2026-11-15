# 声键 0.9.0 发布说明

日期：待发布

> **状态：草稿。** 本文件在 `v0.9.0` tag 推送时由 release workflow 逐字发布
> （`body_path`），发布前请补全下面的「变更 / 修复 / 验证」并删掉本段。

## 不兼容变更

本次有 3 处会让**已有配置或旧版本数据**不再按老路径生效，升级前请对照检查。

### 1. 语音输入工具清单改名

- 旧：`config/voice-tools.json`
- 新：`config/voice-tool-presets.json`

安装目录下的 `resources/config/voice-tools.json` **不再被读取**。如果你在已装机器上
直接改过这个文件（自定义工具名、默认热键、输入法 TSF 身份等），升级后请把文件改名成
`voice-tool-presets.json`；不处理也不会崩，程序会回退到安装包内嵌的默认清单，行为等同于没改过。

### 2. 推荐按键方案的源目录改名（仅仓库侧）

- 旧：`config/button-profiles/`
- 新：`config/recommended-button-profiles/`

只影响仓库里维护推荐方案的人（以及脚本 `pnpm config:build` / `config:check` 的默认目录），
已安装用户不受影响——运行时装进包里的仍然是聚合文件 `config/button-profiles.json`。

### 3. 移除「旧版单文件方案库」的一次性导入

0.6.0 起，首次启动会把 0.5.0 之前留下的单文件方案库（`button-profiles.json`）导入新的
`profiles/` 目录布局，并把旧文件改名成 `.migrated.bak`。这段兼容逻辑已删除。

- 影响范围：只在「从 **0.4.x 或更早** 直接跳到本版本」的机器上才会体现
- 表现：旧文件不再被自动导入，程序从默认方案开始；旧文件**不会被删除**，仍在原处
- 手动恢复：把该文件按新布局手工整理，或用 `pnpm config:import-profile` 转成推荐方案
- 已经跑过 0.5～0.8 任意版本的机器：早在当时就完成了导入，本次无影响

## 推荐方案集合更新

`config/button-profiles.json` 的 `bundleVersion` 升到 `2026.11.4`，按键页会提示「有更新」：

- 新增 5 条推荐方案：Claude Code 按键、Codex 按键、微信按键、WorkBuddy 按键、WPS 演示放映
- 删除示例方案 `preset.netease.controls`（网易云音乐示例）与 `preset.desktop.shortcuts`（桌面快捷键示例），两者都只用于早期功能演示
- 推荐方案的 `appTargets` 现在一律写 exe 文件名（如 `wechat.exe`），不再写应用预设 id

## 变更

（待补）

## 修复

（待补）

## 验证

（待补）
