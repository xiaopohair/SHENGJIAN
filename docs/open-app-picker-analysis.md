# 「打开应用」选择程序：现状分析与重新设计建议

- 日期：2026-11-13
- 状态：**分析文档（未改代码）**，供方案评审
- 范围：按键方案动作「打开应用」、方案设置「应用目标」、组合动作「打开应用 / 聚焦应用输入框」这三类界面里的「选择程序」
- 结论摘要：见 §1；推荐方案：**B（应用目录 + 应用身份规范化）**，分 P0/P1 两阶段；不建议现在做 C（契约层结构化 target）

---

## 1. 结论摘要

1. 现在的「选择程序」本质是**让用户用 Windows 文件对话框去 Program Files 里找一个 .exe**。它把三个不同概念塞进同一个字符串：
   - **要启动什么**（launch target）——启动链路按字符串猜：预设 id / 完整路径 / 裸 exe 名 / AUMID；
   - **方案对谁生效**（match key）——前台匹配只认可执行文件名（basename 小写等值）；
   - **聚焦配置以谁为键**（focus key）——聚焦档案以同一个 target 字符串为 key。
   三者对字符串形态的要求不同，但 UI 都叫它「应用」，用户无从判断该填哪种。

2. 同一功能有 **4 个选择入口、3 份扩展名常量、2 套存值策略**，且 `ProfileEditor.vue` 是**无生产引用的死代码**（只有一份单测在跑）。前端完全不认识「预设应用 / 友好名」，摘要里直接显示原始 target 字符串。仓库里其实已有「预设应用表 + 安装探测 + 已安装应用枚举 + 原生选择器」四类后端能力，但**没有任何 Tauri 命令接线**。

3. 最短的修复不是"再做一个更好的文件对话框"，而是**把选择结果收敛成「应用身份」**：预设 id / UWP AUMID / 完整路径（三种规范形态之一），由后端一个解析函数派生出「显示名、可执行名、能否启动、能否自动匹配、能否便携导出」，选择器提供「预设 → 已安装应用 → 浏览文件」三层入口。

4. 建议分两步交付：
   - **P0（修缺陷 + 收敛）**：共享 `AppPicker` 组件；统一存值规则（预设命中存 id，否则存**完整路径**，不再存裸 exe 名）；友好名展示；空值/可启动性校验；删除死代码；错误不再静默。
   - **P1（应用目录）**：新增 `list_launchable_apps` / `describe_app_targets` 两个 Tauri 命令，复用 `probe_preset_apps`、`scan_registered_apps`、`pick_custom_app`；把已安装应用、UWP 应用、最近使用接进选择器；方案「应用目标」由同一身份派生。
   - **不做 C**：contract 层把 target 改成枚举，需要跨 5 个 crate、导入导出、便携格式、旧数据迁移全改，用户收益与 B 相同（方案文件甚至没有 per-profile 版本号，无法渐进迁移，见 §2.5）。

---

## 2. 现状（代码事实）

### 2.1 一条字符串，四条消费链路

```text
UI「选择程序」──(字符串)──┐
                          ├─> LogicalAction::OpenApp { app } ─> adapter ─> WindowsPlatformActions::open_app
                          │                                   └─> app_launcher::activate_or_launch(target)   ← 启动
                          ├─> 聚焦档案 focus_profiles[target] / 预设默认聚焦                                     ← 聚焦
宏步骤 open_application ──┤
                          └─> 就绪判据 application_readiness(target)                                            ← 等待
方案 ButtonProfile.app_targets（另一个字符串列表）──> matches_executable(前台 exe basename)                  ← 自动匹配
```

同一字符串在四条链路上被解读；**字符串形态不同，四条链路的行为都不同**（见 §2.3 的表）。

补充链路细节（2026-11-13 从代码核实）：

- 桌面端实际执行路径是 `LogicalAction::OpenApp` → `WindowsPlatformActions::open_app`（`shengjian-windows-adapter/src/lib.rs:483-490`）→ `WindowsPlatform::test_app_focus` → `launch_app_and_focus_now`（`shengjian-windows/src/button_mapping.rs:735-739`）→ `activate_or_launch`。
- 桌面端**从不调用** `set_button_mappings`（只有 CLI host 与 examples 调用），所以遗留的 `focus_profiles` / `applications` 在桌面运行时是空的；`is_button_profile_host_available()` 恒为 false（`button_mapping.rs:285`）。聚焦只剩「预设 id → 产品默认聚焦」这一条分支。
- `LogicalAction::OpenApp.focus` 字段执行期被忽略（adapter `:581` 只取 `app`），前端也不写（`ProfileBindingEditor.vue:291-292`）。

### 2.2 四个「选择程序」入口

| 界面 | 入口 | 对话框 | 扩展名常量来源 | 存进去的值 | 失败处理 |
|---|---|---|---|---|---|
| 按键方案 · 动作「打开应用」 | `ProfileBindingEditor.vue:367-380`，按钮在 `:420`（工作台/表格两种视图共用，`ProfileWorkbench.vue:329-337`、`:443-452`） | Tauri `plugin-dialog` `open()` | 本地副本 `:44-56` | `resolved \|\| selected` → **裸 exe 名** | 取消静默 return；异常 `emit("error")` |
| 按键方案 · 方案设置「应用目标」 | `ProfileSettingsDrawer.vue:47-68`，按钮在 `:145-148` | 同上 | 本地副本 `:29-41` | `resolved \|\| basename` 追加进 `appTargets` → **裸 exe 名** | 取消 return；异常 emit |
| 宏 · 「打开应用 / 聚焦应用输入框」步骤 | `macro-editor/MacroActionPanel.vue:67-81`，按钮在 `:187-189` | 同上 | 复用 `lib/runnable.ts` | `patchParameters({ target: selected })` → **完整路径**（`:75-76` 注释说明了理由） | **`catch {}` 静默吞掉** |
| （死代码）`ProfileEditor.vue` | `:247-284`，模板 `:301-317`、`:397-411` | 同上 | 复用 `lib/runnable.ts` | profile 级存 exe 名、按键级存**完整路径** | emit |

- `ProfileEditor.vue` 全仓库只被 `src/components/ProfileEditor.test.ts:4` 引用，**无任何页面/组件 import**。
- 扩展名常量实际有 3 份前端拷贝：`lib/runnable.ts:5-17`、`ProfileBindingEditor.vue:44-56`、`ProfileSettingsDrawer.vue:29-41`；后端还有第 4 份 `crates/shengjian-windows/src/app_launcher.rs:598-604`（注释自称"与前端 runnable.ts 保持一致"）。
- 选择器**零测试覆盖**：`ButtonsPage.test.ts:8-11` 只 mock 了 `open → null` 做导入隔离，没有任何用例点击「选择程序」；`ProfileEditor.test.ts` 只测死组件且不触发选择；`e2e/desktop.spec.ts:69-78` 无法操作原生对话框。

### 2.3 后端 target 形态与分支（`app_launcher::activate_or_launch`）

分支顺序固定：注册项前缀 → 预设 id（**大小写敏感**精确匹配）→ 自定义路径（要求含 `\`/`/` **且**扩展名在白名单）→ 裸可执行名（不含分隔符且扩展名在白名单）→ 其余报错。

| target 形态 | 判定 | 启动行为 | 就绪判据 | 预设默认聚焦 | 便携导出 |
|---|---|---|---|---|---|
| 预设 id（`wechat`/`chrome`…） | `preset_app`（`:245-247`） | 完整激活语义：已运行→前置；AUMID 合并；托盘/relaunch；候选路径/开始菜单解析（`:750-1030`） | 可用（`:3006`） | ✅ Chrome/网易云快捷键（`button_mapping.rs:72-93`） | ✅ `portable_macros.rs:221-232` |
| 完整路径 `.exe` | `is_custom_path_target`（`:608-615`，要求含 `\` 或 `/`） | 存在性检查→按 exe 名找已运行实例→`launch_explicit`（`:1032-1050`） | 可用（`:3010`） | ❌ | ❌ 导出报错 |
| `.lnk` 完整路径 | 同上 | `resolve_lnk` 解析出 exe/参数/工作目录后启动（`:1051-1076`、`:1114`） | 可用 | ❌ | ❌ |
| 裸 exe 名（`notepad.exe`） | `is_bare_executable_name`（`:619-627`） | `resolve_bare_executable`：App Paths→PATH→开始菜单（`:634-658`）；解析不到则 ShellExecute 裸名（`:1098-1099`） | **不可用**（`:3022` 返回 `known=false`）；窗口可见性枚举也没有裸名分支（`:1781-1813`），`wait_for_target_foreground` 恒 false（`:2659-2682`） | ❌（`"chrome.exe" ≠ "chrome"`） | ❌ |
| 其它可运行扩展（`.bat/.cmd/.msi/.ps1/.vbs/.js/.url/.com/.scr`） | `is_custom_path_target` | 只 `launch_path`（`:1076`）：不探测已运行实例、不会前置 | 不可用 | ❌ | ❌ |
| `shell:AppsFolder\…` AUMID | `registered_apps::is_registered_target`（`registered_apps.rs:11-19`） | `launch_registered_app`（`:299` 起） | 可用（`:2989`） | ❌ | ❌ |
| URI（`ms-settings:`、`https://…`） | 无分支 | 报错 | — | — | — |
| 其它（如中文名「微信」、空串） | 都不命中 | 运行时 `Err("未知预设应用：{id}")`（`:1101`）；空串在桌面端更早被 adapter 拒为 `empty-app-target`（adapter `:484-486`），**两处错误码不一致** | 不可用 | ❌ | ❌ |

补充（都是「选择程序」可能产出的值）：

- **`.lnk` 解析失败会被静默降级**：`target_executable_name` 对 .lnk 解析失败时不返回 `None`，而是落到 `file_name()` 得到 `"xx.lnk"`（`app_launcher.rs:258-269`）；前端 `resolved || selected` 无从感知。结果既匹配不到前台 exe 名，启动也走裸名分支大概率失败。
- **多 exe 名预设被压成一种写法**：`target_executable_name` 只取 `exe_names.first()`（`:255`）；微信 `WeChat.exe`/`Weixin.exe`、计算器 `calc.exe`/`CalculatorApp.exe`（`:74-84`、`:123-130`）只有第一个能被 UI 存下。
- **自动匹配（方案 appTargets）是另一套规则**：`ButtonProfile::matches_executable`（`shengjian-contract/src/button_profile.rs:155-160`）拿**前台可执行文件的 basename**（`shengjian-windows-adapter/src/lib.rs:881-896`）与 `app_targets`（规范化时统一小写，`button_profile.rs:192-199`）做**小写等值比较**。路径、AUMID、`.bat` 这类目标永远匹配不上；UWP 前台会得到 `ApplicationFrameHost.exe`。
- **WPS 模块区分能力是死代码**：`registered_apps.rs:21-42` 的窗口类辅助函数全仓库无调用者，`appTargets` 实际无法区分同一 exe 的不同模块。

### 2.4 仓库里已有、但从未接线的后端能力

| 能力 | 位置 | 现状 |
|---|---|---|
| 预设应用表 + 安装探测（10 个定稿预设，含中文名） | `app_launcher.rs:45-56`（`DEFAULT_PRESET_IDS`）、`:65-224`（`PRESET_APPS`）、`:399-423`（`probe_preset_apps`） | 仅 `examples/preset_apps_probe.rs:9` 与单测调用，**无 Tauri 命令**；`app_launcher.rs:13` 注释提到的 `list_preset_apps` 命令**并不存在** |
| 原生应用选择器（返回 `{name, path}`） | `app_launcher.rs:3115-3179`（`pick_custom_app`）、`:462-470`（`CustomAppPick`） | **全仓库零调用点**；`:465-473` 注释「命令返回 / 作为 `OpenApp.target` 持久化」已过期 |
| 已安装应用枚举（Windows AppsFolder，返回 `{name, shell:AppsFolder\AUMID}`，15s 超时 / 2000 上限） | `registered_apps.rs:89-129`（`scan_registered_apps`）、`:63-86`（`normalize_library`） | 仅 `examples/registered_apps_probe.rs:3`；`normalize_library` 唯一生产调用是旧 `ButtonMappings::normalized()` |
| 开始菜单 `.lnk` 索引 + 快捷方式解析 | `app_launcher.rs:286-302`（`start_menu_shortcuts`）、`:1114`（`resolve_lnk`） | 仅内部使用 |
| 前台应用识别 | `foreground_executable` 命令；`app_launcher.rs:497-512` 还有 AUMID 版本 | UI 只用了 exe 名（每 3s 轮询，`ButtonsPage.vue` 约 `:228-234`） |
| 旧应用库 / 聚焦档案（桌面端事实不可达） | `send_input.rs:863-870`（`applications` / `focus_profiles`） | 桌面端从不调用 `set_button_mappings`；Vue/TS 无引用 → 死字段 |
| WPS 窗口类辅助（区分文字/演示） | `registered_apps.rs:21-42` | 全仓库无调用者 |

`apps/shengjian-desktop/src-tauri/src/main.rs:165-216` 注册的命令里，与"应用"相关的只有 `foreground_executable` 和 `resolve_app_target`。

### 2.5 持久化、兼容边界与跨平台后果

- 方案文档 `ButtonProfilesDocument` 为 `schemaVersion 5`（`shengjian-contract/src/button_profile.rs:11`），存储是「索引 + 每方案一文件」，路径 `%LOCALAPPDATA%\Shengjian\profiles.json` + `profiles\*.json`（`shengjian-windows-adapter/src/lib.rs:1017-1059`）。
- **每个方案文件没有自己的 schemaVersion**；导入命令不校验版本，`normalized()` 无条件把版本写回 5（`shengjian-desktop/src-tauri/src/commands.rs:418-429`、`button_profile.rs:282`）。所以任何 target 形状变更都无法按文件渐进迁移，只能全局 bump + 兼容读旧串——这是不推荐方案 C 的直接原因之一。
- 旧 `button-mappings.json` 被新旧两套模型复用：旧标签 `open_app`/`chord`、新标签 `open-app`/`label+keys`（`send_input.rs:556` vs `model.rs:229-240`）；按代码推断旧文件在新 store 下会 `mapping-store-invalid`（未真机验证）。与本议题相邻，重构时应顺手确认迁移路径。
- `LogicalAction::OpenApp { app, focus }`（`model.rs:236-240`）里的 `focus: FocusStrategy`（`:195-202`）是**死字段**：前端从不设置，adapter 执行时忽略（`:581`），聚焦仍走遗留 `WindowsPlatform` 的档案表。
- **便携格式（macOS 互通）只认预设 id**：`portable_macros.rs:522-531` 把 target 映射成 bundle id，映射失败直接导出报错；白名单只有 11 个预设（`:221-232`）。所以宏里选一个自定义路径的程序，**便携导出会失败**——当前的"存完整路径"选择有跨平台成本。
- 方案里同一应用可能出现三种字符串而互不感知：动作 `app`、方案 `app_targets`、聚焦档案 key。宏步骤 target 与按键动作 target 也不统一（宏存完整路径）。

### 2.6 前端展示与交互现状

- 展示：`ProfileWorkbench.vue:174-199` 的 `summary()` 对 `open-app` 直接 `String(action.app || "打开应用")`（`:183`），矩阵单元格、绑定面板页脚、搜索索引都用它；抽屉与宏面板的输入框原样回填 target（`ProfileBindingEditor.vue:412-424`、`ProfileSettingsDrawer.vue:138-140`、`MacroActionPanel.vue:181-186`）。没有友好名、没有图标、没有"这是微信"的映射。
- 「用当前前台」：只有方案设置抽屉有（`ProfileSettingsDrawer.vue:149-151`），把 `foregroundApp`（裸 exe 名）追加进 appTargets；去重大小写敏感，而文件选择器那侧去重大小写不敏感。
- 可复用 UI 基础：`TopbarSearch.vue` 是现成的、可访问的 combobox/listbox 实现（`role=combobox`、`aria-activedescendant`、↑↓/Enter/Esc、失焦折叠，`:104-143`）；`ConfirmDialog.vue` 是唯一的共享对话框且带焦点圈定/归还；`ui/` 下**没有** Popover/Menu/通用 Dialog/Listbox，两个抽屉（`ProfileSettingsDrawer`、`macro-editor/MacroSettingsDrawer`）是手写骨架且都缺 Tab 焦点圈定。
- a11y/i18n 细节：动作卡片缺 `aria-pressed`（`ProfileBindingEditor.vue:386-402`，宏面板有）；icon-only 选择按钮的 aria 名没有上下文（读屏只听到"选择程序…"）；`ProfileSettingsDrawer.vue:116`、`ButtonsPage.vue:635` 的 `aria-label` 硬编码中文；「选择程序」与「选择程序…」是两条不同英文（`i18n.ts:64` 与 `:436`）；`i18n.test.ts:15-20` 会强制每个 `tr("…")` 字面量都有英文条目。

---

## 3. 问题清单（按严重度）

### P0 —— 配了但不工作 / 无法配置

1. **动作 target 被降级成裸 exe 名**（`ProfileBindingEditor.vue:375-376` 调 `resolveAppTarget`，后端 `target_executable_name` 只取 `file_name`，`app_launcher.rs:253-270`）。
   后果：便携安装（不在 App Paths / PATH / 开始菜单）的程序选完打不开，报错只在运行时出现；预设应用被选成 `chrome.exe` 后，**预设 id 丢失** → Chrome/网易云的默认聚焦、AUMID 合并、托盘恢复/relaunch 语义、窗口可见性等待（`target_window_visibility` 无裸名分支）全部失效；同名 exe 有歧义。
2. **UI 只能选文件**：UWP/Store 应用（ChatGPT、计算器）永远选不到；普通用户要在 Program Files 里翻目录；无法复用后端已有的预设表与已安装枚举。
3. **无任何校验**：空 app 可以保存（`ButtonMappings::normalized()` 对 OpenApp 不做校验；桌面端新模型同样不校验，`send_input.rs:1098-1122` 只校验快捷键/滚动/鼠标/宏）；路径失效/应用卸载后无提示；宏面板选择失败被 `catch {}` 静默吞掉（`MacroActionPanel.vue:78-80`）。
4. **同一应用多种字符串且链路不一致**：动作存 exe 名、宏存完整路径、appTargets 存 exe 名。此外：
   - 宏里对自定义应用做「聚焦应用输入框」必然 `MissingDependency`（`macro_runtime.rs:1055-1071` 查不到聚焦档案）；
   - 宏里手输/选择裸 exe 名后，「等待应用」的就绪判据 `known=false` 会被当作 Done（`macro_runtime.rs:980-986`、`:1023-1029`），**静默假成功**；
   - 自定义路径的宏无法便携导出（硬报错）。

### P1 —— 能工作但难用 / 易错

5. 展示原始字符串：用户看不到"微信"这样的友好名，长路径在表格/摘要里溢出，选完无法确认选对。
6. **方案「应用目标」与动作「打开应用」共用文案「选择程序」**，但保存的东西不同（前者要 exe 名用于匹配，后者要启动目标）。用户看不懂也更记不住。
7. **多 exe 名与别名问题**：预设只取 `exe_names.first()`，微信升级后可能变成 `Weixin.exe`；appTargets 又是单值等值匹配（`button_profile.rs:155-160`）→ 方案可能永远不再自动生效。
8. 已安装应用 / 最近使用缺失，每次配置都得重新浏览目录；「用当前前台」只有一处、且只有当前值没有候选。
9. **非 exe 可运行目标**（`.bat/.cmd/.msi/.ps1/.url` 等）能选、能启动，但无法判就绪、不会前置、appTargets 也匹配不上（前台是 `cmd.exe`/`msiexec.exe`/浏览器）。UI 没有任何提示。
10. 旧数据里的裸 exe 名（0.4.x 起保存的）无法被 UI 识别/修复，没有"这个目标可能有问题"的提示；`.lnk` 解析失败留下的 `"xx.lnk"` 更是完全不可用。

### P2 —— 技术债 / 整洁性

11. 4 份选择实现 + 3-4 份扩展名常量 + 死组件 `ProfileEditor.vue` 及其测试；错误上报三套；去重大小写两套。
12. 契约里 `OpenApp.focus` 死字段；聚焦档案在桌面端事实不可达（`is_button_profile_host_available()` 恒 false）；没有聚焦 UI（更大议题，见 §6.6）。
13. a11y/i18n 细节（见 §2.6）。
14. 选择器零测试覆盖，原生对话框不可 E2E（改为自绘列表后可自动化）。
15. 空串 target 的错误码两处不一致（`empty-app-target` vs `未知预设应用`），不利于 i18n 与问题定位。

---

## 4. 设计目标与约束

**目标**

- 用户在 3 秒内选到常见应用（预设/已安装列表），不用记路径。
- 选完能"看到选的是什么、现在能不能启动"，保存前就能发现问题。
- **一个应用只有一种身份表示**：写入时规范化，读取时能描述；启动、匹配、聚焦、等待四条链路对同一个应用得到一致语义。
- 旧配置零迁移成本：读取兼容、写入升级、不批量改写用户数据。
- 界面不暴露内部 target 形态，但保留查看/高级编辑能力。

**约束**

- target 是跨 crate（contract / domain / windows / adapter / desktop）与跨平台便携格式的**字符串契约**，本次不改契约模型。
- Windows Shell/COM 能力必须留在 `shengjian-windows`，不能把 HWND/COM 带进 contract/app（`docs/architecture.md:43`）。
- 扫描必须在后台线程、有界超时、可缓存，不阻塞 UI/引擎（现有 `scan_registered_apps` 已是 15s 超时 + 互斥）。
- 新增文案走 `tr()` 并同时补英文（`i18n.test.ts` 会拦）；`trDynamic` 不校验，不能用于新界面文案。
- 选择器必须可用键盘完整操作（方向键/Enter/Esc，焦点归还），并在 280px 侧栏 / 440px 抽屉 / ~600px 宏面板三种宽度下可用。

---

## 5. 方案对比

| | A 收敛式（最小） | **B 应用目录式（推荐）** | C 契约结构化 |
|---|---|---|---|
| 做法 | 4 处 → 1 个组件；存完整路径；友好名；校验；删死代码 | A + 应用目录命令（预设/已安装/最近使用）+ 身份规范化 + 匹配键派生 | contract 里 `OpenAppTarget` 枚举（Preset/Path/Aumid）；迁移旧数据与便携格式 |
| 用户收益 | 便携应用能用了；不再显示原始串 | 预设/已安装/UWP 直接选；自动匹配一键派生；跨平台导出可提示 | 同 B（用户几乎无感） |
| 改动面 | 前端 + 1 个小命令 | 前端 + `shengjian-windows` 两个解析/列表函数 + 2 个 Tauri 命令 | 5 个 crate + schema/导入导出/便携格式 + 迁移 |
| 风险 | 低 | 中（扫描性能/名称质量/去重） | 高（方案文件无 per-profile 版本、导入不校验版本；兼容性只能全局 bump） |
| 估算 | ~1 天 | ~3-4 天（P0+P1） | ~2 周起，且不可回退 |
| 结论 | 可作为 P0 单独交付 | **推荐**，P0/P1 可拆开上线 | 现在不做；把 B 的 `describe_app_target` 当作将来迁移的垫脚石 |

**为什么推荐 B 而不是 C**：target 字符串已经是 `LogicalAction`、`ButtonProfile.app_targets`、`MacroParameters.target`、便携 `.shengjian` 格式（`portable_macros.rs`）、遗留迁移层（`legacy.rs`）五处的公共表示；改枚举等于同时改这五处，还要写双向迁移。而且方案文件**没有 per-profile 版本号**、导入还不校验版本，结构化改造无法渐进迁移（§2.5）。B 用一个纯解析函数把"字符串的语义"集中到一处，成本小一个数量级，而用户看到的改进与 C 相同。等将来真的需要类型安全（比如引入窗口类/AUMID 匹配、图标缓存），B 的 `describe_app_target` 就是 C 的迁移基础。

---

## 6. 推荐方案 B 详细设计

### 6.1 应用身份与规范化规则

**概念**：`AppIdentity`（后端解析结果，不落盘）

```text
kind:            preset | path | aumid | legacy-name | invalid
target:          写入配置的规范字符串（预设 id / 完整路径 / shell:AppsFolder\AUMID）
displayName:     预设中文名 / 系统显示名 / 文件名（用于界面）
executableName:  派生出的 exe 名（用于 appTargets 匹配与就绪判据；AUMID 可能为空）
launchable:      预设已安装 / 路径存在 / AUMID 已安装 / 裸名可解析
portable:        target 是否在便携导出白名单内（portable_macros.rs:221-232）
readiness:       复用 application_readiness 的只读结果（known/runningProcess/visibleWindow）
issue:           未安装 / 路径不存在 / 无法解析 / 不支持自动匹配（AUMID 无 exe）/ 非 exe 目标
```

**写入规则（canonicalize，按优先级）：**

1. 选择结果的可执行文件命中 `PRESET_APPS` 的 `exe_names`（忽略大小写）→ 存**预设 id**（最佳：默认聚焦、AUMID 合并、便携导出、就绪判据全部生效）。输出必须是**精确大小写**的预设 id（后端匹配大小写敏感）。
2. 来自开始菜单/AppsFolder 的条目 → 存 `shell:AppsFolder\<AUMID>`。
3. 来自文件浏览/开始菜单 `.lnk` → 存**绝对路径**；`.lnk` 能解析出目标 exe 时存解析后的 exe 路径（启动链路本来就绕开 shell 的异步 .lnk 链路，`app_launcher.rs:743-748`）；**解析失败不静默存 `xx.lnk`**，而是提示"快捷方式无法解析，可能指向已卸载的程序"，让用户改选。
4. 手填的裸文件名 → 允许保存但**明确标记为旧式名称**，并提示"按可执行文件名解析，建议重新选择应用"；不再由「选择程序」产生这种值。
5. 其它可运行扩展（`.bat/.cmd/.msi/.ps1/.url` 等）→ 允许但标记"仅启动：不支持自动匹配/不会前置"，UI 给出说明。

**读取规则**：旧值不迁移；`describe_app_target` 按形态推断 kind 并给出 `issue`，界面据此显示徽标与"修复"入口（重新选择即写入规范值）。

**规则表**

| 来源 | 存储 target | 显示名 | 自动匹配键 | 便携导出 | 就绪判据 |
|---|---|---|---|---|---|
| 预设应用（10 个） | 预设 id（精确大小写） | 中文名（如 微信） | `executableName`（多别名表，P2） | ✅ | ✅ |
| 开始菜单 UWP/Store | `shell:AppsFolder\…` | 系统显示名（缓存） | 解析出的 exe 名；无则提示不支持 | ❌ | ✅ |
| Win32 程序 / .lnk / 浏览 | 绝对路径（优先 .exe） | 文件名（优先 lnk 名） | 可执行文件名 | ❌（导出时提示） | ✅ |
| 非 exe 可运行文件 | 绝对路径 | 文件名 | 无（标记仅启动） | ❌ | ❌ |
| 旧式裸名 | 原样（读取兼容） | 文件名 + "旧式"徽标 | 该名称 | ❌ | ❌（可提示修复） |

### 6.2 后端改动

全部落在 `shengjian-windows`（平台能力）+ `shengjian-desktop/src-tauri`（命令），**不碰 contract/domain 模型**。

**纯函数（`app_launcher.rs` 内新增，便于单测）**

- `describe_app_target(&str) -> AppTargetInfo`
  复用已有判定：`preset_app`、`is_registered_target`、`is_custom_path_target`、`is_bare_executable_name`、`resolve_lnk`、`resolve_bare_executable`、`exe_resolvable`、`registered_target_is_installed`、`application_readiness`。
- `canonical_app_target(raw: &str) -> CanonicalAppTarget`
  预设反查（exe_names → id）→ AUMID → 路径规范化（绝对化 + .lnk 解析，失败给出错误而非降级）→ 裸名保留并标记。
- `list_launchable_apps(refresh: bool) -> Vec<LaunchableApp>`
  - 预设：`probe_preset_apps()` 的结果（含未安装标记，默认只展示已安装，可切换"显示全部"）；
  - 已安装：`scan_registered_apps()`（AppsFolder，覆盖 UWP 与开始菜单注册项）+ `start_menu_shortcuts()` 索引解析（Win32 程序）；
  - 去重：与预设 `exe_names` 相同的条目归并到预设项；同 exe 路径/同 AUMID 去重；
  - 排序：预设定稿顺序（`DEFAULT_PRESET_IDS`）→ 最近使用 → 名称；
  - 缓存：进程内 `Mutex<Option<(Instant, Vec<...>)>>`，TTL 60s，`refresh=true` 强制重扫；扫描失败返回结构化错误（可重试），预设部分先返回不阻塞首屏；
  - 稳定性：单次扫描沿用 2000 项 / 15s 上限；日志只记计数/耗时/来源种类，不记个人路径。
- `app_match_key(&str) -> Option<String>`：从 target 派生 appTargets 用的 exe 名（预设取 `exe_names`，路径取 basename，AUMID 取解析结果）。

**Tauri 命令（`apps/shengjian-desktop/src-tauri/src/commands.rs` + `main.rs` 注册）**

- `list_launchable_apps(refresh?) -> Result<Vec<LaunchableApp>, CommandError>`（`spawn_blocking`）
- `describe_app_targets(targets: Vec<String>) -> Vec<AppTargetInfo>`（纯读、同步即可；界面加载后批量描述）
- `canonicalize_app_target(raw: String) -> Result<AppTargetInfo, CommandError>`（手填校验用）
- 保留现有 `resolve_app_target`（其他调用方/兼容），但新代码不再用它把路径降级为 exe 名；给它加 `deprecated` 注释并指向 `app_match_key`。

### 6.3 前端改动

**新组件 `src/components/app-picker/AppPickerDialog.vue`**

```text
┌─ 选择应用 ──────────────────────────────────────────── × ─┐
│ 🔍 搜索应用…                                              │
│ 推荐                                                     │
│  ▸ 微信                        WeChat.exe        [已安装] │
│  ▸ 当前前台：Chrome            chrome.exe                 │
│ 已安装应用                                               │
│  ▸ 微信                        WeChat.exe                 │
│  ▸ 计算器                      Microsoft.WindowsCalc…      │
│  ▸ Chrome 浏览器               chrome.exe                 │
│ 其他                                                     │
│  ▸ 浏览文件…                                             │
│  ▸ 手动填写 target（高级）                                │
└──────────────────────────────────────────────────────────┘
```

- **键盘模型**：复用 `TopbarSearch.vue` 的 combobox/listbox/`aria-activedescendant`/↑↓/Enter/Esc 实现；对话框本体照抄 `ConfirmDialog.vue` 的焦点圈定与关闭归还焦点。
- **宽度**：`min(560px, calc(100vw - 48px))`，列表最大高度 ~320px；从 280px 侧栏按钮打开时也渲染为居中浮层（不内联），避免挤压。
- **状态**：加载中 / 空 / 扫描失败（重试按钮）/ 未安装徽标 / "旧式目标"徽标 / "仅启动"徽标 / "该应用不支持自动匹配"提示。
- **两种模式**（同一个组件，参数区分）：
  - `mode="launch"`（动作/宏）：选中的是启动身份，保存规范 target；
  - `mode="match"`（方案 appTargets）：选中的是**应用身份**，保存 `app_match_key`（exe 名）；AUMID 无 exe 名或非 exe 目标时禁用确认并说明原因。
- **「用当前前台」** 并入列表首项（`foregroundApp` 作为推荐项），与其它入口行为一致（大小写统一用同一个 `mergeAppTarget` 纯函数）。

**共享逻辑（composable）`src/composables/useAppPicker.ts`**

- 封装：打开/关闭、加载 `list_launchable_apps`（缓存句柄）、搜索过滤、选中回调、错误转 toast；
- 替代 4 份 `chooseApp()`；业务组件只传 `mode` 与 `onPicked`。

**展示组件 `AppTargetBadge.vue`**

- 输入 target → 调 `describe_app_targets` 的结果缓存 → 输出「友好名 + 副标题（exe 名/路径）+ 状态点 + tooltip 完整 target」；
- 用于：绑定面板输入框/选中态、矩阵单元格摘要、页脚 `selectedSummary`、方案设置抽屉、宏动作面板、`ProfileWorkbench` 搜索索引（搜索应匹配友好名 + exe 名 + target）。

**接线与清理**

- `ProfileBindingEditor.vue`、`ProfileSettingsDrawer.vue`、`MacroActionPanel.vue` 改为使用 `AppPickerDialog` + `AppTargetBadge`；`ProfileEditor.vue` 与其测试**删除**。
- 扩展名常量收敛到 `lib/runnable.ts`（前端唯一来源）；后端常量保留但补一条一致性检查（Rust 测试里读取该 TS 文件比对，或至少在注释中标注版本号）。
- 错误统一：选择失败/描述失败走全局 toast（`ToastHost`）或页面 `Notice`，不再 `catch {}`。
- i18n 新增 key（中文原文 / 英文建议）：
  - 选择应用 / Choose app
  - 搜索应用… / Search apps…
  - 推荐 / Suggested
  - 已安装应用 / Installed apps
  - 未安装 / Not installed
  - 浏览文件… / Browse files…
  - 手动填写 target（高级） / Enter a target manually (advanced)
  - 该应用不支持自动匹配 / This app can't be matched automatically
  - 应用不可用：{reason} / App unavailable: {reason}
  - 旧式目标，建议重新选择 / Legacy target; please pick the app again
  - 仅支持启动，不会自动切换方案 / Launch only; no automatic profile switching
- a11y：选择按钮带上下文 `aria-label`（如「主页·单击 的应用」）；动作卡片补 `aria-pressed`；抽屉/浮层的硬编码 `aria-label` 改 `trDynamic`。

### 6.4 数据与兼容策略

- **不升 schemaVersion**：target 仍是字符串，`app_targets` 仍是小写 exe 名。（方案文件没有 per-profile 版本号，升版本需全局兼容读旧串；本方案不需要。）
- **读取兼容**：旧值原样保留；`describe_app_target` 推断；界面显示状态与"修复"入口，**不批量改写**用户配置（避免误改/丢信息）。
- **写入升级**：用户重新选择或编辑保存该动作时写入规范值（幂等）。
- **宏同步**：宏的 `open_application` / `focus_input` / `wait_for_application` target 走同一选择器与同一规范化；`focus_input` 对预设应用改为存 id 后可命中预设默认聚焦（修复"自定义应用永远 MissingDependency"的一部分）。
- **便携导出**：对非预设 target 保持现有的硬失败行为，但 UI 在导出前给出清单提示（"以下步骤引用了自定义程序，无法导出到 macOS"），不改变格式。
- **不引入新持久化**：「最近使用」由现有配置（方案动作 + 宏库 + 当前前台）在前端聚合即可，最多 8 项，不落新文件、不涉及隐私。

### 6.5 分阶段交付与验收

**P0 —— 收敛与修复（可独立上线，约 1-1.5 天）**

- [ ] `describe_app_targets` / `canonicalize_app_target` 命令 + `AppPickerDialog`（预设硬编码列表 + 浏览文件 + 手动）
- [ ] 统一存值规则；友好名徽标；空值阻止保存；可启动性提示；错误 toast
- [ ] 删除 `ProfileEditor.vue` + 测试；扩展名常量收敛；i18n/a11y 修正
- 验收：便携应用（如 `D:\Apps\xxx\x.exe`）选完能启动；预设 Chrome/网易云重新选择后默认聚焦恢复；摘要显示中文名；旧值不丢失；`pnpm lint/test/build` + `cargo test --workspace` 通过。

**P1 —— 应用目录（约 2 天）**

- [ ] `list_launchable_apps`（预设/开始菜单/UWP/最近使用，缓存 + 刷新 + 去重 + 分组）
- [ ] `mode="match"` 派生 exe 名；AUMID 无 exe 名/非 exe 目标的明确提示
- [ ] 加载/空/失败/重试态；扫描性能与日志
- 验收：UWP（计算器、ChatGPT）可从列表选择并启动；已安装应用列表 500ms 内出现（缓存命中）；扫描失败可重试且不影响预设选择；同名/重复项不重复出现。

**P2 —— 可选增强**

- 图标（Shell 图标是额外 STA/COM 工作，收益有限，先不做）
- 自动匹配扩展：把 `ForegroundPort` 从"exe basename"扩展为 `{exe, aumid, window_class}`，支持 AUMID/窗口类匹配（同时解决微信新旧 exe 名、WPS 模块区分；`foreground_app_user_model_id` / 窗口类读取函数已存在，见 `app_launcher.rs:497-560`）
- 聚焦档案统一（见 §6.6）
- 旧数据批量修复向导

### 6.6 与本次相关但建议单独立项的两件事

1. **聚焦档案**：契约 `OpenApp.focus` 死字段、桌面端无聚焦档案 UI、`focus_profiles` 在桌面端不可达、预设默认聚焦靠遗留 `WindowsPlatform` 的 `preset_app` 分支。本次只做到"预设应用存 id，默认聚焦继续生效"；把聚焦档案迁到 contract / 做学习 UI 是独立议题。
2. **appTargets 匹配语义**：当前只按 exe basename 等值。建议后续扩展为多别名（预设 `exe_names` 全量）+ AUMID + 窗口类；本次先用 `app_match_key` 保证"选什么存什么"是一致的，不改变匹配函数。

---

## 7. 风险与对策

| 风险 | 影响 | 对策 |
|---|---|---|
| AppsFolder 扫描慢/卡 | 打不开选择器 | 后台线程 + 15s 超时 + 60s 缓存 + 先返回预设 + 失败可重试（现有 `scan_registered_apps` 已具备超时/互斥） |
| 系统显示名质量差（如 `Microsoft.WindowsCalculator_8wekyb3d8bbwe!App`） | 列表难读 | 用 `SIGDN_NORMALDISPLAY`（现有实现已用）+ 预设优先 + 去重/过滤系统项；AUMID 只在无显示名时兜底 |
| AUMID 目标没有 exe 名 | 无法自动匹配/就绪判据弱 | `describe` 明确提示；匹配模式禁用并说明；P2 扩展匹配 |
| 预设在用户机器上装了但探测不到（自定义目录） | 选不了/显示未安装 | 保留"浏览文件"兜底；`canonicalize` 命中 exe 名即归一为预设 id（探测失败也能享受预设语义） |
| 旧数据批量改写误伤 | 配置损坏 | 只读推断 + 单条"修复"，不批量迁移 |
| 扫描结果里的路径/名称进入日志 | 隐私 | 日志只记计数/耗时/来源种类，不记路径（沿用现有 `gatt_note` 脱敏约定） |
| 便携导出遇到自定义路径 | 用户困惑 | 导出前清单提示；不改变现有硬失败语义 |
| 裸名/`.lnk` 降级值遗留 | 用户以为配好了、实际不工作 | `describe` 标记"旧式/不可解析"，提供一键修复；启动失败时错误里带修复入口 |
| 非 exe 目标（bat/msi/url） | 不判就绪、不前置、不参与自动匹配 | 选择器标记"仅启动"；自动匹配模式禁用确认 |
| 宏的等待应用假成功 | 宏误判完成 | 统一 target 规范化后 readiness 可解析；对旧裸名标记并提示 |
| 新增 Tauri 命令权限 | 安全 | 只读、不接受任意 URI；输入用 `normalize_library` 同款校验（长度/控制字符/引号/扩展名） |

---

## 8. 已拍板的取舍（2026-11-13）

> 决策：①做「已安装应用」扫描（P1）②选择器用**居中模态 Dialog** ③旧数据**标记 + 一键修复**、不批量改写 ④宏 target **同步规范化**。以下方案按此实施。

1. **选择器形态**：居中模态 Dialog（已定）。
2. **是否做"已安装应用"扫描**（P1）：推荐做（后端能力已在，用户收益最大）；若想更保守，P0 只上预设 + 浏览也能解决最痛的 bug。
3. **是否保留"手动填写"**：推荐以"高级"折叠保留（AUMID/特殊路径用户），但保存时用 `canonicalize` 校验并标记旧式裸名。
4. **方案「应用目标」是否也换成同一选择器**：推荐是（保存派生 exe 名），否则用户仍要面对两套语义。
5. **旧数据是否自动修复**：推荐"标记 + 一键修复"，不批量改写。
6. **宏 target 是否同步规范化**：推荐同步（同一组件/规则）；注意这会让部分宏的 target 从完整路径变成预设 id，行为更优但属于可见变更，需要 release note。
7. **P2 匹配扩展是否纳入本次**：建议单独立项（会改 `ForegroundPort` 契约）。

---

## 9. 验收清单

**单元测试（Rust）**

- `describe_app_target`：预设 id / 已安装预设 / 未安装预设 / 完整路径存在 / 不存在 / `.lnk` 可解析 / `.lnk` 不可解析 / AUMID / 裸名可解析 / 裸名不可解析 / 非 exe 目标 / 中文名（invalid）。
- `canonical_app_target`：exe 路径命中预设 → id；AUMID → 原样；`.lnk` → 解析后 exe；`.lnk` 解析失败 → 报错；裸名 → 保留并标记。
- `app_match_key`：各形态派生结果；AUMID 无 exe 名 / 非 exe 目标 → `None`。
- `list_launchable_apps`：去重（预设 vs 开始菜单同名）、排序、未安装过滤、缓存命中不重复扫描（可用注入的扫描函数测试）。
- 既有测试会被本次改动影响，需同步更新（清单见下）。

**前端测试（Vitest）**

- 选择器：mock `list_launchable_apps` / `open`；断言选中后保存值形态（launch=规范 target，match=exe 名）、取消不改动、失败走 toast、加载/空/失败态、键盘上下/Enter/Esc。
- `AppTargetBadge`：友好名/副标题/tooltip/状态徽标。
- `useAppPicker`：刷新、搜索过滤、当前前台合并去重（大小写）。
- `i18n.test.ts`：新增 `tr()` key 全部有英文。
- 删除 `ProfileEditor.test.ts`。

**E2E（自绘列表后可测）**

- 按键方案：新建 → 选择动作「打开应用」→ 打开选择器 → 搜索"微信" → 选中 → 摘要显示"微信" → 保存 → 重新载入仍显示"微信"。
- 方案设置：应用目标 → 选一个预设 → 显示 exe 名 + 徽标。
- 宏编辑器：打开应用步骤 → 选择器 → 选中 → target 为预设 id/规范路径；取消不改动。

**真机手工验收**

- 便携应用（不在 App Paths/PATH/开始菜单）通过浏览选择后，按键能启动并前置。
- 预设 Chrome / 网易云通过选择器选择后，"打开应用 + 聚焦"的默认行为仍生效。
- UWP（计算器 / ChatGPT）从已安装列表选择并启动；重复触发不新开实例。
- 方案自动切换：选微信后 `appTargets` 自动带 `wechat.exe`，切到微信前台生效。
- 旧配置（0.4.x 保存的裸 exe 名）打开编辑器不丢值，显示"旧式目标"并可一键修复。

**会被本次改动影响的既有测试（需同步更新）**

- `app_launcher.rs`：`target_executable_name_resolves_preset_and_custom_path`、`custom_path_targets_are_recognized`、`bare_executable_names_are_recognized`、`preset_ids_are_unique_and_nonempty`、`preset_table_includes_requested_apps`、`legacy_presets_stay_resolvable_but_hidden`、`installed_preset_apps_expose_launch_targets`、`lnk_resolution_round_trips`、`activate_or_launch_rejects_unknown_non_path`
- `button_mapping.rs`：`open_app_action_launches_instead_of_tap`、`open_app_with_focus_profile_forwards_strategy`、`preset_focus_profiles_use_known_shortcuts`、`open_app_effective_profile_prefers_explicit_preset_override`、`open_app_preset_requests_auto_composer`、`open_app_custom_without_profile_stays_open_only`
- `focus.rs`：`focus_profile_map_keeps_every_strategy_and_caps_size`
- `send_input.rs`：`legacy_preset_focus_profiles_clear_defaults_but_keep_learned_assets`、`normalized_keeps_focus_profiles_even_without_payload`
- `shengjian-contract`：`normalization_dedupes_ids_and_app_targets`；`profiles.rs`：`auto_profile_applies_on_matching_foreground_and_clears_afterward`；`legacy.rs`：`migrates_legacy_button_profile_document`
- `macro_runtime.rs`：`sequential_steps_complete`（只断言调用顺序，未断言 target 内容，适合补断言）

---

## 10. 证据索引（file:line）

| 事实 | 位置 |
|---|---|
| 动作 target 字符串 + `focus` 死字段 | `crates/shengjian-contract/src/model.rs:236-240` |
| adapter 忽略 focus | `crates/shengjian-windows-adapter/src/lib.rs:581` |
| 前台匹配只认 exe basename | `crates/shengjian-windows-adapter/src/lib.rs:881-896` |
| appTargets 规范化/匹配 | `crates/shengjian-contract/src/button_profile.rs:155-160`、`:192-199` |
| 方案文件无 per-profile 版本、导入不校验 | `apps/shengjian-desktop/src-tauri/src/commands.rs:418-429`；`crates/shengjian-contract/src/button_profile.rs:282` |
| 便携导出只认预设 id | `crates/shengjian-domain/src/portable_macros.rs:221-232`、`:522-531` |
| target 启动分支 | `crates/shengjian-windows/src/app_launcher.rs:750-1101` |
| 路径/裸名判定 | 同文件 `:608-627`、`:634-658` |
| 裸名无窗口可见性分支 | 同文件 `:1781-1813`；置前等待 `:2659-2682` |
| exe 名归一（前端存值来源） | 同文件 `:253-270`；命令 `apps/shengjian-desktop/src-tauri/src/commands.rs:315` |
| 预设表/探测/选择器/扫描 | 同文件 `:45-56`、`:65-224`、`:399-423`、`:3115-3179`；`registered_apps.rs:11-19`、`:89-129` |
| 预设默认聚焦 / 档案 key | `crates/shengjian-windows/src/button_mapping.rs:51-93`；`send_input.rs:865-870`；`focus.rs:851-871` |
| 聚焦 host 不可达 | `crates/shengjian-windows/src/button_mapping.rs:285` |
| 宏就绪/聚焦依赖 | `crates/shengjian-windows/src/macro_runtime.rs:965-1071` |
| 4 个选择入口 | `ProfileBindingEditor.vue:367-380`；`ProfileSettingsDrawer.vue:47-68`；`MacroActionPanel.vue:67-81`；`ProfileEditor.vue:247-284`（死） |
| 原始字符串展示 | `ProfileWorkbench.vue:174-199`（`:183`） |
| 无生产引用的 ProfileEditor | `src/components/ProfileEditor.test.ts:4`（唯一引用） |
| combobox/listbox 可复用实现 | `TopbarSearch.vue:104-143` |
| 对话框焦点管理可复用实现 | `ui/ConfirmDialog.vue:29-64` |
| i18n 强制覆盖测试 | `src/lib/i18n.test.ts:15-20` |

---

## 11. 实施记录（2026-11-13）

按 §8 的四项决策已完成 P0+P1（**未 commit、未改版本号**）。

### 后端（Rust）

- `crates/shengjian-windows/src/app_launcher.rs`：`AppTargetInfo` / `AppTargetKind` / `AppTargetIssue`、`LaunchableApp` / `LaunchableAppSource` / `LaunchableAppsResult`；`describe_app_target(s)`、`canonical_app_target`、`app_match_key`、`list_launchable_apps`（60s 进程缓存；扫描失败降级为已安装预设 + `scanError` token；不 panic）。判定逻辑与 OS 探测分离（`classify_app_target` + 可注入 `AppTargetFacts`），单元测试不依赖真机；非 Windows 提供 stub。
- `crates/shengjian-windows/src/registered_apps.rs`：`scan_registered_app_identities`（同一次 STA 枚举内顺带解析 exe，不再为每个 `.lnk` 起线程）、`describe_registered_app`、`AppScanError`（稳定 token `app_scan_busy/timeout/failed`）。
- `crates/shengjian-domain/src/portable_macros.rs`：`preset_is_portable`（便携导出白名单的唯一来源）。
- `apps/shengjian-desktop/src-tauri`：三个新命令 `list_launchable_apps` / `describe_app_targets` / `canonicalize_app_target` 注册进 `invoke_handler`；`resolve_app_target` 标记 deprecated。
- 规范化规则：exe 命中预设 `exe_names` → 预设 id（便携副本且预设未安装时保留真实路径，避免启动打不开）；AUMID 原样；`.lnk` → 解析后的 exe 路径，解析失败 → 保留 lnk + `lnk_unresolved`；裸名可解析 → 真实路径；空/未知 → invalid + issue。

### 前端（Vue / TypeScript）

- 新增 `components/app-picker/AppPickerDialog.vue`（居中模态；搜索 + 分组 + 状态徽标 + 浏览文件 + 手动填写校验；键盘 ↑↓/Enter/Esc、焦点圈定与归还、aria combobox/listbox）、`AppTargetBadge.vue`（友好名 + 副标题 + 问题徽标 + 「修复」）、`composables/useAppPicker.ts`（列表加载/刷新、describe 批量缓存与 in-flight 去重、错误 toast）、`lib/appTargets.ts`（merge/去重/状态分类纯函数）。
- 接线：`ProfileBindingEditor`（launch 模式 + 旧值修复）、`ProfileSettingsDrawer`（match 模式写入 exe 名、`mergeAppTarget`、「用当前前台」同源、Esc 让位）、`macro-editor/MacroActionPanel`（launch 模式写 `parameters.target`，不再静默吞错）；`ProfileWorkbench` 摘要与搜索优先使用友好名。
- 清理：删除死代码 `ProfileEditor.vue` 与其测试；扩展名常量统一到 `lib/runnable.ts`；`i18n.ts` 新增 44 条中英词条；`resolveAppTarget` 仅保留 deprecated 导出（无新调用方）。
- 旧数据处理：`legacy` 标记 + `issue` 徽标 + 「修复」入口，重新选择即写入规范值，不做批量改写。

### 验证（本机）

| 检查 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | 通过 |
| `cargo test --workspace` | **715 passed / 0 failed / 16 ignored**（其中 `shengjian-windows` 525 passed；新增 31 项 app target/目录/缓存/契约序列化测试） |
| `apps/shengjian-desktop/src-tauri` `cargo check` | 通过 |
| `pnpm lint` / `pnpm test` / `pnpm build` / `pnpm format:check` | 18 files / **99 tests** 全通过；build 通过 |
| 只读探针 | `registered_apps_probe` = 120 项；`preset_apps_probe` = 10 预设（7 installed） |

### 遗留（P2，未做）

- 图标、AUMID/窗口类匹配扩展（需把 `ForegroundPort` 从 exe basename 扩展为身份）、聚焦档案统一、旧数据批量修复向导（见 §6.5/§6.6）。
- 真机人工验收需补：便携应用启动与前置、预设默认聚焦、UWP 列表去重、旧值「修复」交互。

---

## 附：一句话给评审

**这不是"选择程序的对话框不好用"，而是"应用在系统里有三种身份，界面却只让用户交一个字符串"。** 先把身份规范化和友好名做出来（P0），再把应用目录接上（P1），是当前投入产出比最高的路径。
