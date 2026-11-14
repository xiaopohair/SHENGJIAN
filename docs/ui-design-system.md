# 前端设计系统与页面迁移约定

状态：四批次已完成（2026-11-07），0.1.0 品牌收口（2026-11-08），进入后续可选项
日期：2026-11-07（2026-11-08 更新品牌标记）
范围：`apps/shengjian-desktop/src`

本文件是 UI 重构期间的设计契约。页面迁移必须遵守这里的组件、布局和文案规则，避免再次出现每页一套样式。

## 实施状态

- 批次 0：壳层固定、页面居中、宏字段宽度、token/对比度、i18n、错误码、标题栏主题同步
- 批次 1：tokens v2、`components/ui`、页面壳层、Toast/ConfirmDialog/PageHeader
- 批次 2：7 个页面 + Onboarding 已迁移；按键/宏改为宽屏分栏
- 批次 3：构建/测试/实机 1440 浅深色、1000/860 断点、主题切换、未保存导航守卫回归
- 追加：无边框自定义标题栏（`TitleBar.vue`）、`Ctrl+K` 命令面板、完整 AX/键盘验收、设置加载失败诊断降级
- 追加（2026-11-14）：§4.1 用户反馈标准（Toast / Notice / ConfirmDialog / 字段提示的选型与层级阶梯）
- 可访问性验收结果见 `docs/ui-accessibility-report.md`

§4.1 标准的已知落地差距（代码尚未符合，按此清单收敛）：

1. `ToastHost` 当前 `z-index: 130`，低于弹窗（300）与抽屉（240），弹窗内的错误提示看不见 → 应改 400。
2. `ConnectionPage` 电量低使用 `error`，标准要求 `warning`。
3. `pushToast` 无上限、无去重，标准要求同时最多 3 条 + 同消息 5s 节流。
4. `lib/toast.ts` / `ToastHost.vue` 无任何测试覆盖。
5. 按键增强服务的反馈：按键页用 Toast、设置页用 Notice，需按标准统一为「长流程 Notice + 结果 Toast」。

后续可选：真实 NVDA/讲述人逐句回归、axe-core、高对比度主题与 200% 缩放专项。



## 1. 视觉方向

**精炼的桌面工具**：保留 7 页结构和青绿色强调色；深色和浅色同等优先；信息密度适中；工具感、可读性、久用不累优先于装饰。

- 不使用大面积渐变、玻璃拟态、重阴影。
- 卡片圆角 12px，1px 边框 + 极轻阴影。
- 主内容面板顶部与标题栏贴合，1px 边框 + 四周 12px 圆角，左/右/下留 8px 间距，侧栏底色从间距中露出；浅色主题面板背景用 `--bg-panel`（白色）。
- 正文 14px，辅助 12–13px；页面标题 26px。
- 所有颜色走 token；页面里不写十六进制颜色。
- 动效只用于状态反馈，120–240ms；遵守 `prefers-reduced-motion`。

## 2. Token

在 `src/styles/tokens.css` 中定义，浅色/深色成对。常用：

- 表面：`--bg`、`--bg-panel`、`--bg-subtle`、`--bg-raised`、`--bg-elevated`、`--bg-hover`、`--bg-sidebar`
- 边框：`--border`、`--border-strong`
- 文本：`--text`、`--text-secondary`、`--text-muted`、`--text-disabled`、`--text-on-accent`
- 强调：`--accent`、`--accent-hover`、`--accent-soft`、`--accent-soft-strong`
- 语义：`--danger`、`--danger-text`、`--danger-soft`、`--success`、`--success-text`、`--warning`、`--warning-text`、`--info`、`--info-text`
- 圆角：`--radius-sm/md/lg/xl/pill`
- 间距：`--space-1..8`
- 字号：`--text-2xs/xs/sm/base/lg/xl/2xl/3xl`
- 阴影：`--shadow-sm/md/lg`
- 动效：`--duration-fast/base/slow`、`--ease-out`
- 布局：`--page-max` 统一页面最大宽度（1180px）

不要新增与这些语义重复的变量。确需新增时先改 `tokens.css` 并在交付说明中列出。

## 3. 共享布局类（global.css）

- `.page`：所有页面统一使用 `--page-max`（1180），居中，padding 32；不再使用 `.wide/.narrow` 变体。
- 主内容滚动条默认透明；滚动或指针靠近右缘时显示，约 800ms 无操作后隐藏。
- `.stack` / `.stack-sm` / `.stack-lg`：纵向 flex，gap 16/8/24。
- `.metric-grid` + `.metric` + `.metric-label` + `.metric-value`：状态摘要。
- `.split-layout`：`minmax(280px,340px) + 1fr`，<=1120 自动单列。用于列表 + 编辑器。
- `.form-grid` + `.span-2`：两列表单，<=900 单列。
- `.settings-content`：设置页内容区；设置分区使用共享 `.ui-tabs`（与历史页一致），不再使用左侧导航。
- `.toolbar` / `.toolbar-actions` / `.row-actions` / `.footer-actions`
- `.editor-actions`：编辑器底部粘性操作条。
- `.mono` / `.muted` / `.text-truncate` / `.sr-only`

## 4. 共享组件（src/components/ui）

所有组件都使用 scoped CSS，颜色/间距来自 token。

**新界面先在这里找组件，找不到再考虑写页面级布局。** 现有清单：`Button`、`IconButton`、`Card`、
`SectionHeader`、`Field`、`TextInput`、`SelectInput`、`Checkbox`、`Switch`、`RangeInput`、`ToggleGroup`、
`Tabs`、`Notice`、`StatusPill`、`EmptyState`、`LoadingState`、`ConfirmDialog`、`SystemActionGrid`、`ChordInput`。

弹窗类界面没有独立基础组件，直接照 [`RecommendedProfilesDialog.vue`](../apps/shengjian-desktop/src/components/RecommendedProfilesDialog.vue)
的骨架写（遮罩 + 标题栏 + 主体 + 页脚），数值见第 5 节第 19 条。

组合键一律用点选面板，**不做键盘直接采集**（原因见第 5 节第 20 条）：

- [`HotkeyPickerDialog.vue`](../apps/shengjian-desktop/src/components/HotkeyPickerDialog.vue)：语音热键（存 `KeyboardEvent.code` 列表，修饰键分左右）
- [`ChordPickerDialog.vue`](../apps/shengjian-desktop/src/components/ChordPickerDialog.vue)：宏动作 / 按键动作的快捷键（存 `MacroChord`，修饰键分左右）
- 触发入口统一是 [`ChordInput.vue`](../apps/shengjian-desktop/src/components/ui/ChordInput.vue) 的「选择」按钮

### Button

`Button.vue`

```vue
<Button variant="primary" size="md" :loading="busy" @click="save">保存</Button>
<Button variant="secondary">取消</Button>
<Button variant="ghost">更多</Button>
<Button variant="danger">删除</Button>
<Button variant="danger-solid">确认删除</Button>
```

- variant: `primary | secondary | ghost | danger | danger-solid`
- size: `sm | md`
- 默认 slot 放文字；图标直接放在文字前/后，组件自动处理 gap。
- 默认 `type="button"`。

### IconButton

```vue
<IconButton label="编辑方案" @click="edit"><Pencil :size="15" /></IconButton>
<IconButton label="删除" variant="danger"><Trash2 :size="15" /></IconButton>
```

- `label` 必填，同时写入 `aria-label` 和 `title`。

### Card / SectionHeader

```vue
<Card class="stack">
  <SectionHeader title="键位方案" hint="按前台应用自动切换" />
  ...
</Card>
```

- Card `padding`: `none | sm | md | lg`，默认 `md`。
- SectionHeader `title`、`hint` 会走 `tr()`；`actions` slot 放按钮。

### Notice / EmptyState

```vue
<Notice kind="error">{{ message }}</Notice>
<Notice kind="warning">遥控器电量低，建议充电。</Notice>
<EmptyState title="还没有方案" description="新建一个方案开始绑定按键。">
  <template #icon><Keyboard :size="20" /></template>
  <template #actions><Button variant="primary">新建方案</Button></template>
</EmptyState>
```

- Notice kind: `info | success | warning | error`。
- EmptyState 标题/描述走 `tr()`；不要再用页面内 `.history-empty` 等旧类。

### Field / TextInput / SelectInput / Checkbox / Switch / RangeInput

```vue
<Field label="名称">
  <TextInput v-model="draft.name" maxlength="80" />
</Field>

<Field label="音频端点">
  <SelectInput v-model="endpointId">
    <option value="">未设置</option>
    <option v-for="e in endpoints" :key="e.id" :value="e.id">{{ e.name }}</option>
  </SelectInput>
</Field>

<Checkbox v-model="autoApply">前台匹配时自动应用</Checkbox>
<Switch v-model="enabled" label="启用增强捕获" />
<RangeInput v-model="gain" :min="0" :max="24" :step="1" />
```

- Field 是横向 label + control；窄屏自动单列。
- TextInput/SelectInput 使用 `v-model`，宽度 100%。
- 不要再在页面里写 `select { border... }`、`input[type=range]` 等重复样式。

### StatusPill

```vue
<StatusPill :label="statusLabel(phase)" tone="success" dot />
```

- tone: `neutral | success | warning | danger | info | accent`。
- label 由调用方传入已翻译文本（通常是 `statusLabel(...)`）。

### Tabs

```vue
<Tabs v-model="activeTab" :items="tabs" />
```

- items: `{ id, label, count? }[]`。
- 组件内已实现 `role=tablist/tab`、roving tabindex、左右方向键、Home/End。
- 页面面板需要 `:id="'panel-' + tab.id"` 和 `role="tabpanel"`、`:aria-labelledby="'tab-' + tab.id"`。

### TitleBar

`TitleBar.vue` 是应用级无边框标题栏：

- 左侧显示品牌图标与产品名称；保留 drag region（`data-tauri-drag-region`）和右侧窗口控制。
- 右侧最小化/最大化（还原）/关闭，调用 Tauri window API。
- 双击 drag region 切换最大化；`onResized` 同步最大化状态。
- Tauri 配置 `decorations: false`；capabilities 需要 window 的 minimize/toggle-maximize/close/is-maximized/start-dragging 权限。
- 标题栏高度使用 `--titlebar-height` token（36px）；z-index 高于内容浮层，保证窗口控制始终可用。
- 标题栏下边框透明；主体与标题栏间距为 0，与主面板的分隔交给主面板边框。

### TopbarSearch

`TopbarSearch.vue`：

- 靠右显示搜索图标；点击展开 320px 输入框，失去焦点自动折叠。
- 输入时显示下方命令建议；方向键选择、Enter 执行、Esc 折叠。
- `role=combobox` + listbox/option 语义；`Ctrl+K` 由 `App.vue` 聚焦/展开。
- 命令定义在 `App.vue` 的 `paletteCommands`，覆盖 7 页导航、扫描遥控器、打开语音测试。
- 展开态绝对定位，不挤压顶栏设备状态；窄窗口输入框收窄到 280px。

### BrandMark

`BrandMark.vue`：标题栏 12px、侧栏 24px、关于页 84px 的品牌标记。

- 跟随 `document.documentElement.dataset.theme`：深色用 `shengjian-mark.svg`（亮松青），浅色用 `shengjian-mark-on-light.svg`（松青·深 `#0F7A63`）。
- 组件内部用 Vue 响应式切 `src`，不要再用两张 `<img>` + CSS `display` 做主题切换（已验证会因 scoped CSS 优先级失效）。
- 托盘：深色任务栏用亮松青、浅色任务栏用墨色；窗口图标深/浅仍用亮松青 / 松青·深，见 `brand/BRAND.md` §4.6。

### 顶栏全局状态

顶栏从左到右：连接阶段、设备名、电量、当前语音输入工具、语音/音频异常状态；搜索在右侧。版本号移到侧栏品牌下方；侧栏折叠按钮在品牌行右侧，折叠后在 Logo 下方。

### ConfirmDialog / confirmAction

```ts
import { confirmAction } from "../lib/confirm";

if (!(await confirmAction({
  title: "删除方案？",
  message: `将删除“${profile.name}”。此操作不可恢复。`,
  confirmLabel: "删除",
  danger: true,
}))) return;
```

- 全局只挂载一次在 `App.vue`，页面不要自己写遮罩。
- 替换所有 `window.confirm`。

## 4.1 用户反馈标准

界面上「告诉用户发生了什么」有四个通道。**四者能力重叠，本文是唯一选型标准**；新增任何反馈，
先按本节判定，不要凭手感决定。

### 一句话原则

> **用户不需要做任何事 → Toast（自己会走）。用户必须看到、必须做点什么，或者必须等 → 留在页面上（Notice / ConfirmDialog / 字段提示）。**

Toast 会自动消失，所以它是「知其发生过」的通道，不能承担任何「用户还得处理」的职责。

### 判定顺序（按顺序问，命中即定，不许跳步）

| # | 问题 | 命中 → |
|---|---|---|
| 1 | 需要用户明确同意才能继续吗？（删除、卸载、覆盖、丢弃未保存） | `confirmAction({ danger: true })` |
| 2 | 用户要做一件事，或要等一个结果吗？（去装依赖、去授权、等 UAC、等轮询） | 页内 `Notice`，必要时内联按钮 |
| 3 | 是某个字段/表单项本身有问题吗？（保存校验、取值非法） | 字段旁 `settings-help` + 页面级 `Notice` |
| 4 | 都不是，只是告知「刚才那件事成了/没成」？ | `pushToast` |

四问都不命中，说明你其实没有要告诉用户的事 —— 不要加提示。

### 通道职责（唯一职责，不许越界）

| 通道 | API | 生命周期 | 位置 | 唯一职责 |
|---|---|---|---|---|
| Toast | `pushToast(msg, kind, options)` | 自动消失（默认 4.5s） | 顶部居中浮层 | 告知一次性结果 |
| Notice | `<Notice kind="info\|success\|warning\|error">` | 常驻到条件解除 | 页面内文流 | 说明当前状态与下一步 |
| ConfirmDialog | `confirmAction({ ... })` | 阻塞 | 居中模态 | 取得明确同意 |
| 字段提示 | 字段旁 `settings-help` / `StatusPill` | 随值变化 | 行内 | 说明字段语义或校验问题 |

### kind 语义（四个通道共用一套）

| kind | 判据 | 例 |
|---|---|---|
| `success` | 用户发起的动作按预期完成 | 诊断包已导出、方案列表已更新 |
| `info` | 中性进度或状态，无褒贬 | 已请求管理员授权，请在弹出的窗口确认 |
| `warning` | 环境/前置条件不满足，**不是操作失败**，用户可继续或忽略 | 遥控器电量低、按键增强服务未安装、安装未完成所以开关保持关闭 |
| `error` | 操作确实失败了 | 无法加载应用列表、无法解析应用目标 |
| `accent` | 需要主动吸引注意，但既非错误也非警告 | 发现新版本（目前仅此一处） |

**最常见的错用**：把「环境预警」写成 `error`。电量低、服务没装、依赖缺失都是 `warning`；
只有「用户点了、但没做成」才是 `error`。

### 硬规则

**MUST**

1. 同一功能在所有页面用同一通道。长流程（等 UAC / 轮询 / 装依赖）一律 `Notice`，
   结束后最多再用一条 Toast 说结果。
2. `Notice` 需要引导用户去别处操作时，把按钮放进 Notice 内联（`Button size="sm"`），
   一个 Notice 最多一个主行动。不要让用户自己去找入口。
3. Toast 带 `actionLabel` 时，动作只能是「带我去看」（跳转 / 展开 / 聚焦），
   不能是不可逆操作；关闭路径由 `ToastHost` 自带，不要自己再画一个。
4. 错误文案必须给出可执行的下一步，不能只有判断（「请先为方案选择一个程序」+ 内联「选择程序」）。
5. 高频来源（目标解析失败、列表刷新失败）必须在调用侧节流，同一消息 5 秒内只推一次。
6. 文案遵循第 5 节第 6/7 条：不出现路径、命令、字段名、错误码。

**MUST NOT**

1. 不用 Toast 承载需要用户处理的信息。
2. 不用 Toast 承载表单校验错误（它会在用户修好前消失）。
3. 不用 Toast + 「撤销」替代 `confirmAction`。
4. 不同页面用不同通道表达同一件事。
5. 同一时刻推多于 3 条 Toast（超出时丢弃最旧的）。

### 场景对照表（新增场景先在这里找同类）

| 场景 | 通道 | kind | 备注 |
|---|---|---|---|
| 保存方案成功 | Toast | success | |
| 保存校验失败（缺程序/缺快捷键等） | 页内 Notice + 内联动作 | error | 停在页面上直到修好 |
| 导入方案成功 / 有冲突 | 弹窗内 Notice | success / warning | 弹窗开着时不用 Toast |
| 按键增强：请求 UAC、等待轮询 | 卡片内 Notice | info | 长流程 |
| 按键增强：安装/卸载完成 | Toast | success | 结果 |
| 按键增强：服务未安装 | 卡片内 Notice | warning | 常驻状态 |
| 遥控器电量低 | Toast | warning | 不是 error |
| 按语音键但前台是声键（平台会跳过输入法切换） | Toast | warning | 说明原因 + 下一步：切到目标应用 |
| 发现新版本 | Toast | accent + 「查看」 | 20s，唯一带动作的 Toast |
| 诊断包导出 / 恢复默认设置 | Toast | success | |
| 无法加载应用列表 / 解析目标 | Toast | error | 节流 5s |
| 删除方案 / 卸载服务 / 覆盖导入 | ConfirmDialog | — | `danger: true` |

### 层级标准

弹层层级是全局约定，按此阶梯取值，不要自造数字：

| 层 | z-index | 内容 |
|---|---|---|
| 内容 | 0 | 页面主体 |
| 页内粘性元素 | 40 | 工具栏、批量操作条 |
| 引导向导 | 50 | 首次启动向导 |
| 顶栏下拉 | 90 | 搜索建议 |
| 标题栏 | 200 | 自绘标题栏 |
| 抽屉 | 240 | 方案属性抽屉 |
| 模态弹窗 | 300 | 应用选择器、导入导出、推荐方案、配对帮助 |
| **Toast** | **400** | 必须高于所有弹层 |
| 系统级 | 500 | 预留（原生对话框等） |

Toast 位于标题栏 + 顶栏之下的顶部居中位置；容器 `pointer-events: none`，只有卡片接收事件。

### 交付前自查（6 条）

1. 每条新提示都过一遍「判定顺序」四问了吗？
2. `kind` 是 `error` 的，都是「用户点了但没做成」吗？
3. 需要用户行动的提示，是不是留在了页面上？
4. 引导性提示带内联动作了吗？
5. 同一功能在其他页面的通道一致吗？
6. 高频来源节流了吗？

## 5. 页面迁移规则

1. 每个页面根节点使用 `<section class="page">`；按键/宏可加 `wide`。
2. 页面标题统一用 `PageHeader`（eyebrow/title/description/actions slot）。
3. 主内容统一用 `Card` + `SectionHeader`；不要再写 `.card` 大块样式。
4. 页面级 scoped CSS 只保留该页独有的布局（列表行、矩阵、步骤卡、日志预览等），控件样式必须来自共享组件。
5. 不允许出现裸 `window.confirm`、裸 `<select>`/`<input>` 样式（矩阵内的高密度控件除外，可使用共享 `.ui-input` 风格类；但优先用组件）。
6. 面向用户不显示 UUID、`macroId`、`profile id`、`control n`、`versions=1`、原始错误码。ID 放进“详情/复制”二级操作。
   例外：确实需要用户填写/核对标识的编辑弹窗可显示该字段本身（如推荐方案 id），但不要附带实现说明。
7. **界面里不写开发流程**：不出现仓库路径（`config/button-profiles/`）、命令（`pnpm config:build`）、
   JSON 字段名、“去掉默认标记与时间戳”这类实现解释、以及任何“给开发者看的”提示。
   面向用户只说用户能做的动作和结果（导出到哪里、之后能用来做什么）。
8. 快捷键统一显示 `Ctrl + N`、`Win + Shift + S`；不再出现 `control n`。
9. 日期时间使用 `Intl.DateTimeFormat(uiLocale.value, ...)`。
10. 用户反馈（Toast / Notice / ConfirmDialog / 字段提示）一律按 **§4.1 用户反馈标准** 判定；
    本文件不在别处重复这套规则，避免两份标准漂移。
11. 破坏性操作一律 `confirmAction({ danger: true })`。
12. 所有 icon-only 按钮必须有 `aria-label`（`IconButton` 已处理）。
13. 文案：中文源文案 + `tr()`；不要新造硬编码英文（技术字段除外）。
14. 不修改 `lib/i18n.ts`。需要新 key 时在交付消息里列出“中文原文 + 英文建议”，由 Lead 统一补。
15. 不修改 `styles/tokens.css`、`styles/global.css`、`App.vue`、`components/ui/*`、`lib/*`。页面只改自己的 `.vue` 文件。
16. 保留现有脚本业务逻辑（Tauri invoke、自动保存、轮询、录音、事件监听），只调整模板、样式、展示文案和必要的只读展示逻辑。
17. **复用优先，不另造视觉语言**：新界面必须先用 `components/ui/*` 和已有页面/弹窗的区块样式；
    需要新布局时，先找一个同类现有界面照抄骨架（列表类弹窗照 `RecommendedProfilesDialog`），
    不要自创小标题排版、分隔方式、圆角/宽度体系。
18. **不用内联样式**：除运行时计算值（坐标、缩放、比例的 `:style` 绑定）外，一律写 scoped CSS
    并走 token；`style="..."` 静态内联样式禁止。现有例外只有产品图热区/缩放这类动态几何
    （`DevicePairingHelp.vue`、`ProfileWorkbench.vue`、`BrandMark.vue` 的高度参数）。
19. 新弹窗对齐既有骨架数值：`width: min(620px, calc(100vw - 48px))`、`--radius-xl`、
    遮罩取 §4.1 层级阶梯的「模态弹窗」层（`z-index: 300`）、头部 `1px` 分隔线 + 标题前图标 +
    字重 650、页脚 `padding: var(--space-3) var(--space-5)`。
20. **组合键一律点选，禁止键盘直接采集**。语音热键、宏动作快捷键、按键动作快捷键，全部走弹出
    点选面板（`HotkeyPickerDialog` / `ChordPickerDialog`），由用户点修饰键 + 主键组合；
    不要实现「点一下然后按键盘识别」的录制模式。原因是踩过两次坑：
    - **输入法会抢它自己的和弦**：输入法的键盘钩子先于本应用安装，只在自身是活动输入法时
      拦截自己的语音和弦并「吞掉 + 重放」。现场表现是按下后没有任何反应，或只录到半个组合
      （`Ctrl+Win` 是 Chatterfly 的默认和弦，实测钩子只收到 Win、Ctrl 一条边沿都没有）。
    - **系统也会抢**：`Win` 类组合会被 Shell 先行处理，Win 键能到钩子、`Ctrl` 到不了；
      即使钩子层能吞键（`key_gate` 的 shortcut capture），也仍然晚于上游钩子/驱动。
    点选面板能确定地选到任意组合（含左右修饰键、F1–F12、方向键、标点），且**不会触发任何程序**；
    上游 SayAll 最终也是把「自由录制」用 feature flag 关掉，只保留固定/点选组合。
    「注入副本」这一层的机制说明在 `crates/shengjian-windows/src/key_gate.rs` 的
    `injected_edge_is_passthrough` 注释里（本仓库没有 `Bugs/` 目录，别引用上游路径）。

## 6. 响应式

窗口 `minWidth = 1200`（`apps/shengjian-desktop/src-tauri/tauri.conf.json`），桌面壳里视口宽度永远 ≥ 1200，因此**低于 1200px 的 `max-width` 断点是不可达的死代码**。断点必须落在可达区间：

- `<=1400`：中等档（动作/绑定网格列数压缩）。
- `<=1320`：整页级分栏改单列（语音页两栏、About hero、配对弹层、方案矩阵、`.split-layout`）。默认 1280 窗口即生效。
- `<=1260`：紧凑档（表单单列、行布局重排、页头/工具栏堆叠、内边距收窄）。

新增断点前先确认它 > 1200；做不到就用容器查询（`container-type: inline-size`），或先评估窗口 `minWidth`。不要再用 560/620/700/720/820/900/980/1120 这类小于 minWidth 的值。

## 7. 交付要求

- 页面 `pnpm build` 必须通过（Lead 也会跑）。
- 新增文案在消息中列出；不要直接改 i18n。
- 交付消息包含：改了哪些文件、页面结构、仍需 Lead 处理的点（新 i18n key、共享样式缺口、行为差异）。
- 不提交 git，不打包，不改版本号。
