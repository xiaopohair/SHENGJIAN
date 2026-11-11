# 声键 0.5.0 发布说明

日期：2026-11-12

`0.5.0` 重做了设置页与按键方案，并修正了多处布局与状态显示问题。

## 亮点

- **设置页重做**
  - 改为左侧分区导航 + 右侧内容面板，分组带图标，顶部支持搜索设置项
  - 设置项统一为「标题 + 说明 / 控件右置」的行式布局
  - 外观与语言改用分段按钮，选项全部可见、一次点击完成
  - 「音频」分区更名为「音频及语音」，拆出独立的「语音」区块
  - 修正「音频输入设备」的方向：后端枚举的是 Render（播放）端点，已更名为「音频输出设备」
  - 数据与诊断、高级两节改为行式结构，与其余分区风格统一
  - 关于页移除重复的「日志目录」「配置向导」入口（设置页已有）

- **语音页合并**
  - 删除独立的语音页，其设置并入「音频及语音」分区的「语音」区块
  - 语音热键录入抽为 `useHotkeyRecorder` 组合式函数，窗口监听与清理随组件生命周期
  - 顶栏的语音/音频运行状态保持不变

- **按键方案**
  - 新增全局方案：恒存在、排首位、默认激活、不可删除，其余方案继承它的动作
  - 方案存储改为**一个方案一个文件**（`profiles/<名称>-<id>.json` + `profiles.json` 索引），便于单独分享
  - 工具栏新增「重置」按钮：清空当前方案的全部动作，回到继承状态
  - 修复遥控器连接状态被硬编码为「RC003 · 已连接」——未连接时现在显示「未连接」且状态点为灰

- **布局与交互修复**
  - 修复语音/按键/宏/连接四页在最小窗口宽度（1200px）下错误堆叠为上下布局
  - 修复各页工具条在最小宽度下折行
  - 修复宏页动作卡片退化为每行 2 个（现为 4 个）
  - 修复宏页按钮字号偏大、标题与「未保存」标签未垂直居中
  - 删除连接页冗余的「重试」按钮（与「扫描设备」职责重叠，主按钮现可清除设置加载错误）
  - 删除按键页编辑区重复的「当前绑定 / 主页 单击」

- **代码质量**
  - 按 Vue 最佳实践整理响应式原语（原始类型改用 `shallowRef`）、组件契约（`ChordInput` 改用 `defineModel`）与模板引用（`useTemplateRef`）
  - 新增可复用的 `ToggleGroup`、`SettingRow` 组件
  - 删除不在运行链路上的 `shengjian-profiles` crate（实际实现位于 `shengjian-app` 与 `shengjian-windows-adapter`）

## 验证

- `cargo fmt --all -- --check` 通过
- `cargo test --workspace`：**670 passed**
- 桌面前端：`pnpm lint`、`pnpm format:check`、`pnpm test`（**61 passed**）、`pnpm build`
- 实机验证：设置页五个分区与搜索、按键方案全局方案与重置、未连接状态显示、最小窗口宽度布局

## 安装

Release 资产：

```text
shengjian_0.5.0_x64-setup.exe      安装包（NSIS，未签名）
shengjian_0.5.0_x64-portable.zip   便携版（免安装 zip）
SHA256SUMS                         两个产物的 SHA-256
manifest.json                      安装包大小与 SHA-256
portable-manifest.json             便携版大小与 SHA-256
```

下载后用随包的 `SHA256SUMS` 校验：

```powershell
Get-FileHash shengjian_0.5.0_x64-setup.exe -Algorithm SHA256
# 与 SHA256SUMS 中对应的行比对
```

校验值以 Release 里的 `SHA256SUMS` 与 `manifest.json` 为准，本文不硬编码，
避免与实际上传的构建产物不一致。

安装包未签名，Windows SmartScreen 可能提示；首次安装 RC003 helper 需要一次 UAC。覆盖安装会保留 `%LOCALAPPDATA%\Shengjian` 下的设置、方案、宏、转写历史与日志。

## 升级说明

- **按键方案存储格式变更**：从单文件 `button-profiles.json` 改为 `profiles/` 目录 + `profiles.json` 索引。
  旧文件保留在磁盘上但不再读取；升级后方案库以新布局为准。
- 语音页已移除，原语音页的设置项位于「设置 → 音频及语音 → 语音」。

## 已知限制

- 仅 Windows x64；未提供 ARM64 包
- 0.5.0 为未签名发布包
- 无自动更新
- 方案尚未按设备型号隔离（RC003 与 RC001 共用同一份方案列表），
  计划见 `docs/plan-device-model-profiles.md`
- 「聚焦应用输入框」需要先在按键方案里配置过对应应用的聚焦方式，否则步骤会报缺少依赖
- RC001 / ARN9 设备身份映射仍为 deferred
