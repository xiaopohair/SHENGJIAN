# 可编辑配置：推荐应用与语音输入工具

「打开应用」列表里的**推荐应用**、设置/连接/向导里的**语音输入工具**都不再写死在代码里，
而是由仓库根目录的 JSON 配置决定。发布前直接改这两个文件即可，不需要改 Rust。

## 文件位置

| 文件 | 作用 |
|---|---|
| `config/app-presets.json` | 推荐应用（含安装探测、AUMID、默认聚焦、便携导出映射） |
| `config/voice-tools.json` | 语音输入工具列表、名称、默认热键、行为类型 |

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
