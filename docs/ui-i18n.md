# UI 与多语言结构

## 页面

- Connection / 连接
- Voice / 语音
- Buttons / 按键
- Macros / 宏
- History / 历史
- Settings / 设置
- About / 关于
- OnboardingWizard / 首次配置

## 语言层

`src/lib/i18n.ts`：

- 中文源文案作为 key
- English 字典集中维护
- `tr(source, vars?)` 返回当前语言文案
- 支持 `{name}` 形式插值
- `setLanguagePreference` 支持：
  - `system`
  - `zh-Hans`
  - `en`
- `system` 根据 `navigator.language` 解析
- 语言切换由 Vue 响应式驱动，无需刷新页面

## 设置链路

`SettingsDocument.global.application_language`：

```text
设置页选择
  -> setLanguagePreference()
  -> 立即刷新 UI
  -> 保存设置写入 core settings
  -> 下次启动 App 读取并恢复
```

## 翻译边界

翻译：

- 页面标题、按钮、字段、提示
- 状态和选项
- 校验错误、确认对话框
- 空态、帮助文案

不翻译：

- 用户输入：profile 名称、宏名称、转写文字
- 应用名、设备名
- 协议/版本号等技术字段
- 后端返回的未映射错误码
- 原生 Windows 文件对话框（由系统语言决定）

## 已知限制

- 后端常见错误码/消息已做中英文映射；未收录的错误码会显示通用“操作失败，请重试”，不再直接暴露原始 code。
- 英文布局已按 860–900px 窗口回归；本轮设计系统重做已完成。
- 原生 Windows 文件对话框语言由系统决定。
- 原生窗口标题栏仍保留，深色/浅色主题已同步；自定义无边框标题栏留作后续可选项。

## 交互约定

- 七个主页面统一使用 `components/PageHeader.vue`：eyebrow、标题、描述、操作区；首次配置向导使用独立 dialog 结构。
- 页面底部不放置静态功能说明；说明合并到页面描述或相关卡片。
- 设置项变更后自动保存，不提供“保存设置”按钮。
- 语言切换立即生效，并立即重建托盘菜单。
- 中文界面中的 `ready`、`streaming`、`single`、`disconnected` 等状态值统一映射为中文标签。
- 页面切换前会经过 `lib/navigation.ts` 的离开守卫；按键/宏编辑器有未保存修改时会先确认。

## 本轮已落地的 UI 优化（2026-11-07）

1. 抽取 `components/ui`：Button / IconButton / Card / SectionHeader / Notice / EmptyState / Field / TextInput / SelectInput / Checkbox / Switch / RangeInput / StatusPill / Tabs / ConfirmDialog。
2. 固定侧栏 + 唯一滚动区 + 常驻顶栏；页面内容居中，宽屏 1440/2560 不再靠左。
3. 深色/浅色/三种强调色 token 化；同步 Windows 标题栏主题；修复浅色错误文字、危险按钮、侧栏分组标题对比度。
4. 按键/宏改为宽屏列表 + 编辑器分栏，窄屏单列；编辑器底部操作条常驻。
5. i18n 全覆盖检查；补齐窗口主题、错误码、空态、辅助文案。
6. 移除未使用的 `reka-ui`、`@vueuse/core` 依赖；清理旧版控件 CSS。

## 后续可选

1. 真实 NVDA/Windows 讲述人逐句回归、axe-core 自动化扫描
2. 高对比度 Windows 主题与 200% 缩放专项
3. 屏幕阅读器状态播报细化（连接/语音/宏运行）
