# 可编辑配置：推荐应用与语音输入工具

「打开应用」列表里的**推荐应用**、设置/连接/向导里的**语音输入工具**都不再写死在代码里，
而是由仓库根目录的 JSON 配置决定。发布前直接改这两个文件即可，不需要改 Rust。

## 文件位置

| 文件 | 作用 |
|---|---|
| `config/app-presets.json` | 推荐应用（含安装探测、AUMID、默认聚焦、便携导出映射） |
| `config/voice-tools.json` | 语音输入工具列表、名称、默认热键、行为类型 |
| `config/button-profiles/` | 推荐按键方案的**源目录**（一方案一文件，见下） |
| `config/button-profiles.json` | 推荐按键方案的**聚合产物**（生成文件，不要手改） |
| `config/recommended-macros.json` | 推荐宏（只读；供推荐/复制方案引用，用户宏优先） |

构建时 `apps/shengjian-desktop/scripts/prepare-config.ps1` 会把它们复制到
`apps/shengjian-desktop/src-tauri/resources/config/`，再由 Tauri 打进安装包。

运行时读取顺序：

1. 安装目录 `resources/config/<name>.json`
2. 安装目录 `config/<name>.json`
3. 都缺失或解析失败 → 使用 `shengjian-domain` 里内嵌的同一份 JSON（`include_str!`），
   不会崩溃，启动日志会注明使用了内嵌兜底

所以：**发布前改仓库里的 `config/*.json`；装好之后也能直接改安装目录里的
`resources/config/*.json`，重启程序生效。** 改坏 JSON 只会回退到内嵌默认值。

## `app-presets.json`

```jsonc
{
  "schemaVersion": 1,
  "apps": [
    {
      "id": "wechat",                 // 稳定 id：写入按键动作/方案匹配/旧配置，发布后不要改
      "name": "微信",                  // 界面显示名（不翻译）
      "exeNames": ["WeChat.exe"],     // 进程/窗口匹配与安装探测
      "installPaths": ["%ProgramFiles%\\Tencent\\WeChat\\WeChat.exe"], // %VAR% 模板
      "shortcutNames": ["微信"],       // 开始菜单快捷方式名（自定义安装目录兜底）
      "relaunchWhenHidden": false,    // 托盘隐藏时是否允许重新拉起
      "hidden": false,                // true = 仍可解析旧映射，但不进推荐列表
      "selfApp": false,               // 遥控器宿主自身
      "alwaysInstalled": false,       // 恒视为已安装
      "aumid": "OpenAI.Codex_...!App",// 打包应用（UWP/MSIX）的 AppsFolder id
      "focus": {                      // 可选：打开后默认聚焦方式
        "strategy": "appShortcut",    // appShortcut | autoComposer | openOnly
        "shortcut": ["left_control", "l"], // KeyCode serde 名
        "click": { "x": 410.0, "y": 37.0 } // 可选：快捷键后点击的窗口内坐标（96 DPI）
      },
      "portableBundle": "com.tencent.xinWeChat" // 可选：便携组合动作导出到 macOS 的 bundle id
    }
  ]
}
```

- 数组顺序就是「推荐」列表顺序；`hidden: true` 的条目不出现在列表，但旧按键动作仍能解析。
- 没有 `focus` 的条目默认 `autoComposer`（自动找输入框），与旧行为一致。
- `id` 是持久化标识：发布后改 id 会让旧按键动作/方案匹配找不到它（不会清空配置）。

## `voice-tools.json`

```jsonc
{
  "schemaVersion": 1,
  "tools": [
    {
      "id": "wechat",                    // 持久化到 settings：发布后不要改
      "label": "微信输入法",              // 下拉/状态栏显示名（不翻译）
      "kind": "wechat",                  // wechat | doubao | vokie | chatterfly | custom
      "defaultHotkey": ["ControlLeft", "MetaLeft"], // KeyboardEvent.code；空=无默认热键
      "ime": {                           // 可选：该工具本身是 TSF 输入法时的身份
        "clsid": "604A99E3-6D90-4571-824D-2639BD572F6C",
        "profile": "C16C250B-CF1C-4C58-B068-12AEDBED610F",
        "names": ["Chatterfly"]          // GUID 缺失/失效时按显示名自动发现（简中）
      }
    }
  ]
}
```

- `kind` 决定 Windows 专属行为，代码里目前有这几种适配器：
  - `wechat`：切换/观测微信输入法（WeType）
  - `doubao`：切换豆包输入法 + 右 Alt 松手保险
  - `chatterfly`：切换/观测 Chatterfly 输入法（TSF），走通用 IME 救回
  - `vokie`：只观测，不切换输入法
  - `custom`：只有自定义热键，无 IME 切换/观测/救回
- `ime`（可选）：给"本身是输入法"的工具配置 TSF 身份。
  - 优先用 `clsid` + `profile` 固定 GUID（格式带不带 `{}`/大小写都可）；
  - 固定 GUID 未提供或在本机读不到显示名时，按 `names` 在已安装的简体中文
    输入法里按显示名自动发现（大小写/空白不敏感，允许包含匹配）；
  - 启动时会解析并写诊断日志：`catalog ime_profile tool_id=… source=config|discovered outcome=resolved`；
  - 因此**重装/升级后 GUID 变了**：改这两个值，或删掉 `clsid`/`profile` 只留
    `names` 走自动发现，不需要改 Rust；解析失败会回退代码内置的实测常量；
  - 只有 `wechat` / `doubao` / `chatterfly` 这三种"自建输入法"行为会使用它；
    `vokie`（桌面程序）和 `custom` 即使配了也会被跳过。
- 新增一个输入法：加一条 JSON，`kind` 选 `custom`（或复用已有行为），填 `defaultHotkey`；
  如果它本身是输入法并且要切输入法/观测，再补 `ime` 身份。
- 改 `id` 同样会让旧设置里的 id 变成未知 id：配置不会丢，但默认热键与行为降为 custom。

### 如何从注册表查 TSF 身份（CLSID / Profile GUID）

TSF 输入法注册在 `CTF\TIP` 下，路径结构固定：

```
HKLM\SOFTWARE\Microsoft\CTF\TIP\{CLSID}\LanguageProfile\0x{LANGID}\{ProfileGUID}
                                   └─ 第 1 个值 ─┘              └─ 第 2 个值 ─┘
```

- **CLSID**：`TIP\{…}` 这一层目录名（输入法 COM 组件）
- **Profile GUID**：`LanguageProfile\0x…\{…}` 最里层目录名（具体输入配置）
- **LANGID**：中间层，`0x00000804` = 简体中文
- 同时要查 3 个位置：64 位视图、`WOW6432Node`（32 位输入法）、`HKCU`（每用户注册）

#### 方法 A：`reg query` 一步搜索（最快）

```bat
reg query "HKLM\SOFTWARE\Microsoft\CTF\TIP" /s /f "Chatterfly"
reg query "HKLM\SOFTWARE\WOW6432Node\Microsoft\CTF\TIP" /s /f "Chatterfly"
reg query "HKCU\SOFTWARE\Microsoft\CTF\TIP" /s /f "Chatterfly"
```

本机 Chatterfly 的实际输出（64 位与 WOW6432Node 都有，各 2 处匹配）：

```
HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\CTF\TIP\{604A99E3-6D90-4571-824D-2639BD572F6C}\LanguageProfile\0x00000804\{C16C250B-CF1C-4C58-B068-12AEDBED610F}
    Description    REG_SZ    Chatterfly
    IconFile       REG_SZ    C:\WINDOWS\system32\ChatterflyTSF.ime
```

拆出来就是配置要的两个值：

| 配置字段 | 取注册表路径的哪一段 | Chatterfly 实测值 |
|---|---|---|
| `ime.clsid` | `TIP\` 后的目录名 | `604A99E3-6D90-4571-824D-2639BD572F6C` |
| `ime.profile` | `LanguageProfile\0x00000804\` 后的目录名 | `C16C250B-CF1C-4C58-B068-12AEDBED610F` |
| `ime.names` | `Description` 值 | `Chatterfly` |

#### 方法 B：PowerShell 列出全部输入法身份

```powershell
$roots = @(
  'HKLM:\SOFTWARE\Microsoft\CTF\TIP',
  'HKLM:\SOFTWARE\WOW6432Node\Microsoft\CTF\TIP',
  'HKCU:\Software\Microsoft\CTF\TIP'
)
$profiles = foreach ($root in ($roots | Where-Object { Test-Path $_ })) {
  Get-ChildItem $root | ForEach-Object {
    $clsid = $_.PSChildName
    $langRoot = Join-Path $_.PSPath 'LanguageProfile'
    if (-not (Test-Path $langRoot)) { return }
    Get-ChildItem $langRoot | ForEach-Object {
      $langid = $_.PSChildName
      Get-ChildItem $_.PSPath | ForEach-Object {
        $props = Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue
        [pscustomobject]@{
          Description = $props.Description
          CLSID       = $clsid
          LangID      = $langid
          ProfileGUID = $_.PSChildName
          Enable      = $props.Enable
        }
      }
    }
  }
}
$profiles | Where-Object { $_.Description -like '*Chatterfly*' } | Format-List
# 全部输入法：$profiles | Sort-Object Description | Format-Table -AutoSize
```

期望看到（`Enable = 1` 才表示已启用）：

```
Description : Chatterfly
CLSID       : {604A99E3-6D90-4571-824D-2639BD572F6C}
LangID      : 0x00000804
ProfileGUID : {C16C250B-CF1C-4C58-B068-12AEDBED610F}
Enable      : 1
```

> 同一输入法在 64 位视图和 `WOW6432Node` 各注册一次时，上面的脚本会输出两行、
> 但 CLSID / Profile GUID 相同，属正常现象。

#### 方法 C：系统语言列表（只能拿到 GUID，没有显示名）

```powershell
Get-WinUserLanguageList | Select-Object LanguageTag, InputMethodTips
```

`InputMethodTips` 的格式是 `langid:{CLSID}{ProfileGUID}`，例如：

```
0804:{86598FB9-66A2-463E-B9C2-AEB906D477AD}{607FDF85-FCC8-4DBD-A365-41296F980C9C}   # 微信输入法
0804:{604A99E3-6D90-4571-824D-2639BD572F6C}{C16C250B-CF1C-4C58-B068-12AEDBED610F}   # Chatterfly
```

不含 Description，需与注册表结果对照（或按你已知的输入法名字排除）。

#### 查不到时

- 检查是否装了 32 位版：一定要查 `WOW6432Node`；本机 Chatterfly 在两个视图都注册
- `Enable = 1` 才是启用状态；`Enable` 缺失/为 0 时 TSF 不会切过去
- `Description` 可能随语言/版本变化，配置的 `names` 用包含匹配兜底
- 配置里的 GUID 带不带 `{}`、大小写混写都可以（解析时会去掉非十六进制字符）
- 改完配置重启程序，看诊断日志
  `%LOCALAPPDATA%\Shengjian\Logs\shengjian-diagnostic.log` 里的
  `catalog ime_profile … source=config|discovered outcome=resolved|failed`；
  若显示 `failed`，先核对路径里的 CLSID/Profile 是否抄错、`Enable` 是否为 1

## 推荐按键方案：源目录 + 聚合文件

推荐方案是**只读**的：不会写进用户方案库，用户点「复制并使用」后才生成一份可编辑副本。
同「推荐应用」一样随安装包下发、启动读取、非法时回退内嵌副本。

源码侧采用**一方案一文件**，方便 review、减少多人改同一文件的冲突：

```text
config/button-profiles/
├── catalog.json                        { "schemaVersion": 1, "bundleVersion": "2026.11.1" }
├── preset.desktop.shortcuts.json       一个文件 = 一条推荐方案
└── preset.netease.controls.json
```

单条方案文件就是方案对象本身（不带 `profiles` 外壳），文件名必须等于方案 id：

```jsonc
// config/button-profiles/preset.netease.controls.json
{
  "id": "preset.netease.controls",     // 稳定 id，必须 preset. 前缀，且与文件名一致
  "name": "网易云音乐按键",
  "deviceModel": null,                 // 复制后归属；null = 通用
  "compatibleModels": ["RC003"],       // 适配型号；空 = 全部
  "appTargets": ["netease_music"],     // 前台应用匹配（预设 id / exe）
  "autoApply": true,
  "holdRepeat": "long",                // 可选
  "bindings": {                        // 只允许可编辑按钮（is_profile_button）
    "ok": { "single": { "type": "mouse", "kind": "click", "direction": null, "amount": 1 } }
  }
}
```

运行时与安装包只认聚合文件 `config/button-profiles.json`（`include_str!` 内嵌兜底也用它）：

```jsonc
{
  "schemaVersion": 1,
  "bundleVersion": "2026.11.1",   // 版本变化时按键页显示「有更新」
  "profiles": [ /* 由目录按文件名升序合并 */ ]
}
```

**聚合文件是生成物，不要手改。** 改完目录后跑：

```powershell
cd apps/shengjian-desktop
pnpm config:build     # 目录 → 聚合文件（并 prettier 排版）
pnpm config:check     # 校验目录 + 聚合是否一致 + 其余三个配置
```

- 顺序 = 文件名升序（界面里推荐方案的排列顺序；想调顺序改文件名前缀）
- CI 会跑 `pnpm config:check`：**改了目录忘记 `config:build` 会直接失败**，
  并列出「目录里有但聚合文件没有」之类的差异
- 单条方案的校验规则与运行时一致（见下），不合法时 `config:build` 拒绝生成

### 通用规则

- `compatibleModels` 为空 = 所有型号；非空时只有当前设备型号匹配才会列出/允许复制
- 未连接设备（型号未知）时只显示「通用」方案
- 不支持 `isDefault`（默认方案只属于用户数据）；`run_macro` 引用的宏必须存在于
  `recommended-macros.json`，否则该条预置被拒绝（目录级回退内嵌）
- 复制：生成 `preset-copy-<源id>-<毫秒>` 的普通方案，名称加「（我的）」，
  保留绑定/应用目标/重复设置；用户在前端保存后才会落盘并激活

### 用脚本把「我的方案」转成推荐方案

应用导出的 `button-profiles.json` 是**用户方案库**（`schemaVersion: 5` + `activeManualProfileId`），
和推荐方案不是同一个结构：id 前缀、`isDefault`、`compatibleModels`、`bundleVersion` 都不一样。
手动搬字段容易漏掉 `isDefault`（一条就会让整份目录被拒），因此提供转换脚本：

```powershell
cd apps/shengjian-desktop

# 1) 在应用「按键」页做好方案、真机验证后点「导出」；先看看文件里有哪些方案
pnpm config:import-profile --list --input "%USERPROFILE%\Desktop\button-profiles.json"

# 2) 预览转换结果（不写盘）
pnpm config:import-profile --input "%USERPROFILE%\Desktop\button-profiles.json" `
  --profile "豆包语音输入（我的）" --id preset.rc003.doubao --models RC003 --dry-run

# 3) 写入 config/button-profiles/preset.rc003.doubao.json 并重建聚合文件
#    （同名 id 会覆盖原文件，并自动递增 catalog.json 里的 bundleVersion）
pnpm config:import-profile --input "%USERPROFILE%\Desktop\button-profiles.json" `
  --profile "豆包语音输入（我的）" --id preset.rc003.doubao --models RC003

# 4) 校验
pnpm config:check
```

脚本做这些事：

- 自动剔除 `isDefault` / `createdAt`，补 `compatibleModels` 与 `bundleVersion`
- 按 `--id` 生成 `preset.` 前缀 id，写成 `<id>.json`；同名 id 覆盖原文件，不会重复追加
- 写盘前跑一遍与 `button_presets::validate_catalog` 等价的校验（id 前缀/重复、
  `isDefault`、空 name、不可编辑按键、`run-macro` 引用是否存在、动作类型等），
  不合法直接报原因并**不写盘**
- `--check` 同时检查聚合一致性与 `app-presets.json` / `voice-tools.json` /
  `recommended-macros.json` 的结构性问题；CI 也会跑 `pnpm config:check`

已安装的机器想直接试效果，用 `--out` 直接写安装目录（只写聚合文件，不动源目录）：

```powershell
pnpm config:import-profile --input ... --id preset.xxx `
  --out "$env:LOCALAPPDATA\声键\resources\config\button-profiles.json"
```

## `recommended-macros.json`（推荐宏）

```jsonc
{
  "schemaVersion": 1,
  "bundleVersion": "2026.11.1",
  "macros": [
    // 与 macros.json 的 MacroDefinition 同结构
  ]
}
```

- 只读，随包下发；推荐方案里的 `run_macro` 可以引用随包发布的宏
- 同 `macro_id` 的用户宏优先；推荐宏不能被用户库删除/覆盖
- 启动先加载推荐宏、再加载推荐方案；任一推荐方案引用的宏不存在时整份目录回退内嵌
- 推荐宏目录为空是合法状态（当前默认）
- 运行时：`run_macro` 先在用户宏库查找，找不到再查推荐宏；按键页编辑动作的
  「运行其他宏」下拉会合并推荐宏（用户同名优先），但**宏页不列出推荐宏**、
  运行历史也不记录推荐宏
- 推荐宏在每次动作执行开始时做快照；更新配置后下一次执行自动生效

## 校验与回退

两个文件在启动时都会做轻量校验：非空、id 唯一、focus strategy / tool kind 合法；
`voice-tools.json` 的 `ime` 块必须至少提供固定 GUID 对或发现名之一。
校验失败时整体回退到内嵌默认配置（对应 `crates/shengjian-domain/src/app_catalog.rs`、
`voice_tools.rs` 里的 `DEFAULT_*_JSON`）。单元测试覆盖了解析与拒绝非法配置。

## 相关代码

- `crates/shengjian-domain/src/app_catalog.rs` / `voice_tools.rs`：配置类型、加载、查询
- `apps/shengjian-desktop/src-tauri/src/catalog.rs`：启动时读取安装目录配置
- `apps/shengjian-desktop/scripts/prepare-config.ps1`：构建时复制进资源目录
- `crates/shengjian-windows/src/app_launcher.rs` / `button_mapping.rs` / `app_icons.rs`：
  推荐应用的启动、探测、默认聚焦、图标
- `crates/shengjian-windows/src/voice_tool.rs` + `ble.rs` / `ime.rs`：输入法行为适配器；
  `ime.rs` 负责 TSF 身份解析（固定 GUID / 按显示名发现）与切换/观测
- `crates/shengjian-domain/src/button_presets.rs` / `recommended_macros.rs`：推荐方案/推荐宏目录
- `apps/shengjian-desktop/src/components/RecommendedProfilesDialog.vue`：推荐方案对话框与复制并使用
