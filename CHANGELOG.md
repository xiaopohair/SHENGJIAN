# 变更记录

## 0.9.4 — 2026-11-19

### 新增：Google Chromecast TV Remote 支持

- 识别与档案：`Chromecast Remote` 加入支持列表，显示名 **Google Chromecast TV Remote**，
  型号 `CHROMECAST`，设备 id `google-chromecast`。
- 按键：14 个键全部接入（确定 / 返回 / 方向 / 主页 / 电源 / 音量± / 静音 / YouTube / Netflix / 信源）。
  该机型的 HID 报告不出现在 Raw Input 层，按键由 helper 在 `WUDFHost` 内读取后经命名管道转发，
  新增 `C <ms> <usage>` 行传递 3 字节 Consumer 报告。
- 默认按键映射（型号基线，连上即生效）：
  确定→回车、返回→Esc、方向键、主页→Ctrl+T、电源→F11、音量±/静音→系统音量。
  YouTube / Netflix / 信源默认不绑定（无通用 Windows 语义）。
- 语音：走 ATVV，实测 16 kHz 满速（`voice_audio_rate result=full`）。
- 界面：连接页显示 Chromecast 线稿图；配对指南新增 Google 机型（配对手法「返回 + 主页」，
  蓝牙名 `Chromecast Remote`）；按键方案页使用 Chromecast 产品图并标注 14 个热区。

### 修复

- 按键映射合成只遍历小米 12 键，型号布局里的键（如静音）绑定会静默失效。
- 映射重算的判重键不含设备型号，切换设备后不重算，型号默认方案永远推不下去。
- 同一功能存在两条「推送映射」实现，行为分叉且不可观测；已合并为一条。
- 边沿被 app 层与平台注入器**各执行一次**（一次按键做两遍：静音按一下就又取消）——
  改为注入器为唯一执行者。
- 按键动作转换覆盖不足：注入层执行 8 种动作而转换器只映射 3 种，滚动 / 鼠标 / 运行宏 /
  打开应用 / 常用语 / 聚焦输入框会被静默丢弃。已补全并加回归守卫测试
  `every_action_kind_survives_conversion`。
- 语音速率把「会话墙钟」当分母，短会话被系统性低估（16 kHz 报成 9~11 kHz）；改为按首帧→末帧计算。
- 打包脚本 `build:installer` 不构建 helper，安装包会静默带上旧 helper；已把 helper 构建并入脚本。
- 版本号统一：`src-tauri/Cargo.toml` 补到 0.9.4，诊断日志 `ver=` 不再落后于界面。
- 推荐方案里的静音键名写成 `mute`（契约里是 `volume-mute`），导致内嵌目录解析失败；
  已修正，`bundleVersion` 随之升到 `2026.11.19`。
- 静音键纳入 Chromecast 方案布局后，`is_profile_button` 覆盖到全部 16 个逻辑键；
  领域测试里那条「静音不可编辑」的反例随之移除。

### 变更

- 小米三款（RC001 / RC003 / ARN9）默认方案补齐方向键、OK、返回（等价方向键 / Enter / Esc），
  原先只有音量±；Home / 菜单 / TV / 电源保持未绑定。
- `preset.chromecast-remote` 推荐方案的 `autoApply` 为 `false`（该字段按前台应用匹配，
  空 `appTargets` 永不触发；型号基线由代码内的默认方案承担）。
- 诊断开关改名 `debug_log_enabled`（原 `audio_progress_enabled` 名不副实）。
- 表格视图的按钮行去掉第二行的按键 id，仅保留键帽与名称；补齐静音 / YouTube / Netflix / 信源的键帽字形。
- 补齐界面英译：`信源`、Chromecast 配对文案、方案页产品图说明、`返回键`。

### 其他

- `shengjian-app` 增加最小日志钩子，app 层诊断可进诊断日志。
- `SHENGJIAN_DEBUG=1` 时才输出逐键映射诊断（`map_resolve` / `map_inject` / `inject_tap` / `audio_progress`）。
