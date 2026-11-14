# 推荐按键方案源目录

一个文件 = 一条推荐方案，文件名必须等于方案 id（`preset.` 前缀）。

```text
catalog.json                      { "schemaVersion": 1, "bundleVersion": "2026.11.1" }
preset.desktop.shortcuts.json     { "id": "preset.desktop.shortcuts", ... }
preset.netease.controls.json      { "id": "preset.netease.controls", ... }
```

改完这里要重建聚合文件（运行时会读它，`include_str!` 内嵌兜底也用它）：

```powershell
cd apps/shengjian-desktop
pnpm config:build   # 目录 → ../../config/button-profiles.json
pnpm config:check   # 校验目录、聚合一致性、其余配置目录
```

- 聚合文件 `config/button-profiles.json` 是生成物，**不要手改**
- 推荐方案顺序 = 文件名升序
- 改 `catalog.json` 的 `bundleVersion` 后，用户按键页会提示「有更新」
- 从应用「按键」页导出的方案请用 `pnpm config:import-profile` 转换，不要手抄字段

完整规则见仓库 `docs/config-catalogs.md`。
