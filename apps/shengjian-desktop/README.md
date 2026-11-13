# 声键 · shengjian-desktop

声键（SHENGJIAN）的 Windows 桌面宿主：Tauri 2 + Vue 3，in-process 使用 `shengjian-core`。

当前版本：**0.3.1**
状态：已通过 GitHub Release 发布（CI 构建、未签名）；无自动更新。
已支持设备：RC003（小米蓝牙语音遥控器 2 Pro）、RC001（小米蓝牙语音遥控器 2）；Windows x64。

## 运行

```powershell
cd apps/shengjian-desktop
pnpm install
pnpm tauri dev
```

## 构建检查

```powershell
pnpm lint
pnpm format:check
pnpm test
pnpm build
# 需要已安装 0.3.1；会重启桌面进程并通过 WebView2 CDP 跑真实安装包 E2E（安装路径从 tauri.conf.json 的 productName 派生）
pnpm test:e2e

cd src-tauri
cargo check
cargo test
```

## 界面

- 无边框自定义标题栏：拖动区、最小化/最大化（还原）/关闭；双击拖动区最大化
- `Ctrl+K` / 顶栏搜索：搜索页面与操作，方向键选择、Enter 执行、Esc 或失焦折叠
- 侧栏顶部品牌 LOGO 与版本号；侧栏可手动折叠/展开并记住状态
- 顶栏：折叠按钮、连接阶段、设备名、电量、当前输入法、语音/音频异常状态；搜索图标靠右，点击展开输入
- 固定侧栏 + 唯一内容滚动区；所有页面统一 `--page-max` 1180px 居中，按键/宏在 1180 内分栏
- 按键页默认「遥控器工作台」，可切换「表格视图」；两者共享当前方案、按键、手势和未保存状态
- 设置页分区选项卡（与历史页一致），窄窗口自动换行；settings 加载失败时仍进入“数据与诊断”
- 可访问性验收见 `docs/ui-accessibility-report.md`

## 功能

### 连接

- 扫描已配对遥控器
- 连接/断开、多设备切换、记住上次设备
- phase、model、电量、重连次数、电源通知、已配对数量
- 当前设备音频覆盖与设备语音工具覆盖
- RC003 增强捕获开关与 helper 安装/状态
- 低电量页面提醒；托盘 tooltip 显示连接状态/电量

### 语音

- voice/audio 状态
- 语音测试：streaming → draining → idle 链路检查 + 实时 dBFS 电平
- 错误提示：已知后端错误码/消息按中英文映射
- 语音输入工具（微信输入法/豆包/Vokie/其他）
- 自定义语音热键：支持录制组合键，可恢复输入工具默认
- 转写历史开关（默认关闭，只存文字，不存音频）

### 按键 / 方案

- 方案列表、手动激活、复制方案
  - 应用目标支持“用当前前台”填充
  - 快捷键支持直接按键捕获
  - 自动方案冲突提示、保存前动作校验
  - 按键级按住连发（单击/长按）
  - 方案级语音输入工具覆盖
- 常用语动作仅在组件可用时显示；已有不可用绑定保留并标注
- 方案编辑器：
  - 名称、应用目标、自动应用
  - 12 键 × single/double/long（含 TV、菜单）
  - 动作：继承、禁用、聚焦输入、快捷键、打开应用、常用语、鼠标、运行宏
- 方案原生 JSON 导入/导出
- 应用目标支持应用选择器（预设/已安装/UWP/浏览文件）/用当前前台
- “打开应用”动作使用应用选择器：预设 → 已安装应用（含 UWP）→ 浏览文件；选中后按应用身份规范化保存，旧式目标可一键修复

### 宏

- 宏列表、新建/编辑/删除、搜索
- 8 种动作、多步骤
- 版本递增与历史、单版本删除、复制宏、最新版本选择
- 步骤上移/下移、拖拽排序、复制步骤、步骤超时、最近运行分步结果
- 宏运行历史由主进程后台记录（应用运行期间全量）到 `%LOCALAPPDATA%\Shengjian\macro-history.json`
- 保存前中文校验提示（名称/步骤/URL/快捷键/超时）
- 测试、取消、重试、运行状态
- 宏库导入/导出
- 方案绑定「运行宏」（`run-macro`）

### 设置

- 全局音频（端点/增益）
- 主题 system/light/dark、强调色（默认/蓝/紫/橙）
- 应用语言：跟随系统 / 中文 / English（所有页面，即时切换并重建托盘菜单）
- 设置项变更即时保存，无需保存按钮
- 开机自启
- 关闭行为：隐藏到托盘或退出
- 配置备份/恢复（设置、方案、宏库、转写历史、外观偏好、上次设备）
- 日志查看器、诊断包导出、健康检查、恢复默认设置、设置搜索
- 诊断日志目录

### 历史

- 语音事件、按键事件、宏运行、转写记录集中在一个页面
- 语音/按键仅为本次运行记录；宏运行/转写持久保存
- 转写默认关闭；语音页开启后只记录文字、应用和时间，不保存音频
- 支持查看和清除宏运行/转写历史

### 桌面

- 托盘菜单：显示/隐藏、打开设置、打开日志目录、退出；tooltip 显示连接/电量
- 全局操作 toast：备份、诊断导出、重置、低电量
- 单实例
- 首次配置向导（语音测试步骤可选择语音输入工具）
- 关闭到托盘
- CSP

## 安装包

```powershell
pnpm tauri build
```

输出（文件名由 `src-tauri/tauri.conf.json` 的 `productName` 派生）：

`src-tauri/target/release/bundle/nsis/shengjian_0.3.1_x64-setup.exe`

包内包含：

- `resources/rc003/shengjian-helper.exe`
- `resources/rc003/frida-gadget.dll`
- `resources/licenses/`（含品牌字标 OFL 声明 `LICENSE-OFL.txt`）

安装包当前未签名。应用安装目录为 `%LOCALAPPDATA%\声键`；用户数据位于 `%LOCALAPPDATA%\Shengjian`，卸载保留。

## RC003 返回键

返回键/音量±需要增强捕获 helper：

1. 连接页开启“启用 RC003 增强捕获（返回键、音量键）”
2. 点击“安装/更新 helper”，确认一次 UAC
3. helper 状态显示“已安装”

详见 [../../docs/rc003-helper-setup.md](../../docs/rc003-helper-setup.md)。

## 边界

- UI 不直接调用 Windows API 或读写业务文件
- core 持久化设置/方案/映射/宏
- Tauri 只负责生命周期与命令/事件翻译
- `RemoteId` 为 opaque token，不暴露蓝牙地址
