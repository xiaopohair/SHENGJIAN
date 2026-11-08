# 桌面端可访问性与键盘验收报告（0.1.0）

- 日期：2026-11-07
- 对象：`apps/shengjian-desktop` 的四批次 UI 重构
- 方法：Tauri dev（WebView2）+ Chrome DevTools Protocol
  - `Accessibility.getFullAXTree` 检查每个页面的无障碍树
  - `Input.dispatchKeyEvent` 执行 Tab / Shift+Tab / Enter / Escape / 方向键
  - 临时破坏 `settings.json` 验证设置加载失败时的诊断入口
- 限制：本报告以 **AX 树 + 键盘链路**验收为主；Windows 讲述人人工朗读已由用户确认通过；未运行 axe-core 自动化扫描。

## 1. 结论

- 7 个主页面 + 首次配置 + 顶栏命令搜索 + 确认对话框均具备可用的 landmark、标题层级、按钮/表单名称。
- 关键控件 unnamed 数量：**0**。
- 装饰性 SVG 图标已从 AX 树隐藏；AX 树中 `image` 节点数量为 **0**。
- 键盘主链路：窗口控制 → 跳过链接 → 折叠/展开侧栏 → 主导航 → 顶栏搜索 → 页面操作 → 页面内容，顺序稳定。
- Ctrl+K 展开顶栏搜索、方向键选择、Enter 执行、Escape/失焦折叠均通过。
- 顶栏连接阶段使用 `role=status` + `aria-live=polite`；电量/输入法为普通文本，避免轮询时反复播报。
- 首次配置、确认对话框、顶栏搜索均有焦点管理；关闭/折叠后焦点回到触发元素。
- 设置加载失败时，设置页自动切到“数据与诊断”，配置备份/诊断/健康检查仍可用。

## 2. 页面无障碍树

| 页面 | Landmarks | H1 | 未命名关键控件 |
| --- | --- | --- | --- |
| 连接 | banner / complementary / main / navigation | 连接遥控器 | 0 |
| 语音 | 同上 | 语音输入与测试 | 0 |
| 按键 | 同上 | 按键与方案 | 0 |
| 宏 | 同上 | 宏 | 0 |
| 历史 | 同上 | 历史 | 0（Tabs 4 / tabpanel 1） |
| 设置 | 同上 | 设置 | 0（Tabs 5 / tabpanel 1） |
| 关于 | 同上 | 关于 | 0 |

补充：

- banner 名称：`应用标题栏`
- complementary 名称：`应用侧栏`
- 主导航名称：`主导航`
- 设置分区 tablist 名称：`设置分区`
- 每个页面恰有一个 `h1`；卡片使用 `h2`。

## 3. 表单与编辑器

- 按键编辑器：39 个 textbox/combobox/checkbox/switch，未命名 **0**；矩阵选择器可读出“返回 · 单击”等完整名称。
- 宏编辑器：8 个表单控件，未命名 **0**；名称/说明/快捷键按键/修饰键均有名称。
- 设置页：主题/强调色/应用语言均可读出 label；分区改用 Tabs：5 个 `role=tab`，1 个 `role=tabpanel`，方向键/Home/End 切换。
- 历史 Tabs：4 个 `role=tab`，1 个 `role=tabpanel`；方向键可切换并同步 `aria-selected` 与面板。
- 顶栏命令搜索：combobox 名 `搜索命令或页面`，`aria-autocomplete=list`、`aria-haspopup=listbox`、`aria-expanded=true`；listbox 名 `命令面板`；option 首项 `aria-selected=true`。

## 4. 键盘链路

### 4.1 标题栏与主框架

从“最小化”按钮开始的 Tab 顺序：

1. 最小化
2. 最大化
3. 关闭
4. 跳到主内容
5. 连接
6. 语音
7. 按键
8. 宏
9. 历史
10. 设置
11. 关于
12. 折叠/展开侧栏
13. 顶栏搜索
14. 页面主操作（扫描设备 / 重试 …）

验证：

- 窗口最小化、最大化、还原按钮均可用；最大化后按钮名称从“最大化”变为“还原”。
- 窗口保留 `WS_THICKFRAME`，无边框后仍可原生缩放。
- 跳过链接激活后焦点落到 `main-content`，页面滚动到主内容。
- `Ctrl+K` 展开顶栏搜索；输入框自动获得焦点。

### 4.2 顶栏命令搜索

- 点击搜索图标或 `Ctrl+K` 展开；输入焦点进入搜索框。
- `ArrowDown` 移动选中项；`Enter` 执行；`Escape` 折叠并归还焦点。
- 实测从“连接”移动一次到“语音”，Enter 后主视图切到语音页。
- 输入框失焦后自动折叠为图标；点击结果使用 `mousedown.prevent`，不会先失焦。
- 空结果有“没有匹配的命令”文本。

### 4.3 确认对话框

- 键盘 Enter 触发删除方案 → `alertdialog` 打开，初始焦点在“取消”。
- Tab 在“取消/删除”之间循环；Escape 关闭。
- 关闭后焦点恢复到触发删除的按钮。

### 4.4 首次配置

- `role=dialog` + `aria-modal` + `aria-labelledby`。
- 打开后焦点落在“跳过”；Tab 在“开始配置/跳过”之间循环。
- Escape 关闭向导。
- 标题使用 h2，不再与页面 h1 冲突。

### 4.5 设置加载失败

临时破坏 `%LOCALAPPDATA%\Shengjian\settings.json` 后：

- 设置页 `activeSection` 自动切到“数据与诊断”。
- 顶部显示错误 Notice。
- “配置备份”“诊断”“健康检查”三张卡片仍渲染，可打开日志目录、查看日志、导出诊断包。
- 恢复 settings.json 后重新加载正常。

## 5. 已修复的问题

1. Lucide 装饰图标暴露为无名 `image` 节点 → 在 `main.ts` 统一加 `aria-hidden`，并监听后续渲染。
2. ConfirmDialog 只有 Esc/焦点初始，没有 Tab 圈定和焦点归还 → 已补。
3. Onboarding 缺 dialog 语义、无焦点圈定 → 已补。
4. TopbarSearch 缺 combobox 语义、失焦折叠和焦点归还 → 已补。
5. 标题栏缺少可读名称 → 补 `应用标题栏` banner label。
6. 三处图标按钮/输入框缺少可访问名称 → 已补 aria-label。
7. Tabs 缺 `aria-label` / Home / End → 已补。
8. 设置页分区缺 `aria-current` → 改用共享 Tabs（tablist/tab/tabpanel + 方向键）。
9. 侧栏分组标题对比度 2.31:1 → 改用 `--text-muted`，浅色 4.70:1、深色 6.68:1。
10. 浅色错误文字、危险按钮、自定义强调色对比度 → 统一 token 修复。

## 6. 人工确认

- **2026-11-08：Windows 讲述人朗读、真实 RC003 状态播报、Windows 高对比度主题与 200% 缩放人工验收，用户确认通过，未报告失败项。**

## 7. 复现命令

```powershell
cd apps/shengjian-desktop
pnpm build
pnpm test
```

运行时通过 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223` 启动 Tauri dev，再用 CDP 的 `Accessibility.getFullAXTree` 和 `Input.dispatchKeyEvent` 复查。
