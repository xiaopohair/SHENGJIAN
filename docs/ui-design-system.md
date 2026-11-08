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
- 可访问性验收结果见 `docs/ui-accessibility-report.md`

后续可选：真实 NVDA/讲述人逐句回归、axe-core、高对比度主题与 200% 缩放专项。



## 1. 视觉方向

**精炼的桌面工具**：保留 7 页结构和青绿色强调色；深色和浅色同等优先；信息密度适中；工具感、可读性、久用不累优先于装饰。

- 不使用大面积渐变、玻璃拟态、重阴影。
- 卡片圆角 12px，1px 边框 + 极轻阴影。
- 正文 14px，辅助 12–13px；页面标题 26px。
- 所有颜色走 token；页面里不写十六进制颜色。
- 动效只用于状态反馈，120–240ms；遵守 `prefers-reduced-motion`。

## 2. Token

在 `src/styles/tokens.css` 中定义，浅色/深色成对。常用：

- 表面：`--bg`、`--bg-subtle`、`--bg-raised`、`--bg-elevated`、`--bg-hover`、`--bg-sidebar`
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

- 只保留 drag region（`data-tauri-drag-region`）和右侧窗口控制；品牌移到侧栏顶部。
- 右侧最小化/最大化（还原）/关闭，调用 Tauri window API。
- 双击 drag region 切换最大化；`onResized` 同步最大化状态。
- Tauri 配置 `decorations: false`；capabilities 需要 window 的 minimize/toggle-maximize/close/is-maximized/start-dragging 权限。
- 标题栏高度使用 `--titlebar-height` token；z-index 高于内容浮层，保证窗口控制始终可用。

### TopbarSearch

`TopbarSearch.vue`：

- 靠右显示搜索图标；点击展开 320px 输入框，失去焦点自动折叠。
- 输入时显示下方命令建议；方向键选择、Enter 执行、Esc 折叠。
- `role=combobox` + listbox/option 语义；`Ctrl+K` 由 `App.vue` 聚焦/展开。
- 命令定义在 `App.vue` 的 `paletteCommands`，覆盖 7 页导航、扫描遥控器、打开语音测试。
- 展开态绝对定位，不挤压顶栏设备状态；窄窗口输入框收窄到 280px。

### BrandMark

`BrandMark.vue`：侧栏 24px、关于页 84px 的品牌标记。

- 跟随 `document.documentElement.dataset.theme`：深色用 `shengjian-mark.svg`（亮松青），浅色用 `shengjian-mark-on-light.svg`（松青·深 `#0F7A63`）。
- 组件内部用 Vue 响应式切 `src`，不要再用两张 `<img>` + CSS `display` 做主题切换（已验证会因 scoped CSS 优先级失效）。
- 托盘/窗口图标同样走「亮松青 / 松青·深」两版，见 `brand/BRAND.md` §4.6。

### 顶栏全局状态

从左到右：折叠侧栏、连接阶段、设备名、电量、当前语音输入工具、语音/音频异常状态；搜索在右侧。版本号移到侧栏品牌下方。

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

## 5. 页面迁移规则

1. 每个页面根节点使用 `<section class="page">`；按键/宏可加 `wide`。
2. 页面标题统一用 `PageHeader`（eyebrow/title/description/actions slot）。
3. 主内容统一用 `Card` + `SectionHeader`；不要再写 `.card` 大块样式。
4. 页面级 scoped CSS 只保留该页独有的布局（列表行、矩阵、步骤卡、日志预览等），控件样式必须来自共享组件。
5. 不允许出现裸 `window.confirm`、裸 `<select>`/`<input>` 样式（矩阵内的高密度控件除外，可使用共享 `.ui-input` 风格类；但优先用组件）。
6. 面向用户不显示 UUID、`macroId`、`profile id`、`control n`、`versions=1`、原始错误码。ID 放进“详情/复制”二级操作。
7. 快捷键统一显示 `Ctrl + N`、`Win + Shift + S`；不再出现 `control n`。
8. 日期时间使用 `Intl.DateTimeFormat(uiLocale.value, ...)`。
9. 错误提示优先用 `Notice`；瞬时成功/失败用 `pushToast`。
10. 破坏性操作一律 `confirmAction({ danger: true })`。
11. 所有 icon-only 按钮必须有 `aria-label`（`IconButton` 已处理）。
12. 文案：中文源文案 + `tr()`；不要新造硬编码英文（技术字段除外）。
13. 不修改 `lib/i18n.ts`。需要新 key 时在交付消息里列出“中文原文 + 英文建议”，由 Lead 统一补。
14. 不修改 `styles/tokens.css`、`styles/global.css`、`App.vue`、`components/ui/*`、`lib/*`。页面只改自己的 `.vue` 文件。
15. 保留现有脚本业务逻辑（Tauri invoke、自动保存、轮询、录音、事件监听），只调整模板、样式、展示文案和必要的只读展示逻辑。

## 6. 响应式

真实桌面窗口最小 860，常用 1000/1280/1440/最大化。

- `<=1120`：列表 + 编辑器分栏改为单列。
- `<=1000`：侧栏保持用户选择，可手动折叠为 64px 图标栏。
- `<=900`：表单单列。
- `<=720`：页面 padding 收窄（浏览器调试用）。

不要继续使用 `@media (max-width:760px)` 作为主要断点。

## 7. 交付要求

- 页面 `pnpm build` 必须通过（Lead 也会跑）。
- 新增文案在消息中列出；不要直接改 i18n。
- 交付消息包含：改了哪些文件、页面结构、仍需 Lead 处理的点（新 i18n key、共享样式缺口、行为差异）。
- 不提交 git，不打包，不改版本号。
