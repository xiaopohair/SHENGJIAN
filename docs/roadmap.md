# Roadmap

## Phase 1 — 已完成：核心 + Windows 平台迁移

- [x] `shengjian-contract`：DeviceProfile、ID、逻辑动作、快照、事件、错误。
- [x] `shengjian-domain`：ATVV/ADPCM/PCM/voice/pipeline/macros/settings/statistics/transcripts。
- [x] `shengjian-app`：端口与 Connection/Voice/Button/Audio/Settings/Catalog 用例。
- [x] `shengjian-testkit`：假适配器、记录型事件 sink、集成测试。
- [x] `shengjian-windows`：原有 Windows 平台实现迁入，编译和测试通过。
- [x] `shengjian-profiles` / `shengjian-common-phrases`：迁入。
- [x] `helper/rc003`：原生 helper 迁入。
- [x] `profiles/`、`Testing/hardware-scripts/`、`contracts/ipc/`：迁入。
- [x] 清除原产品命名（crate、环境变量、日志、helper、临时文件、便携格式）。

## Phase 2 — 已完成：接上 ports

- [x] `RemoteLinkPort` adapter：`shengjian-windows-adapter` 复用现有 BLE 状态/错误映射。
- [x] `AudioSinkPort` adapter：列表/选择/增益/phase/generation 映射。
- [x] `ActionSinkPort` adapter：快捷键/打开应用/聚焦/宏/鼠标/常用语。
- [x] `ButtonSourcePort` adapter：Raw Input/HID 语义边沿 -> `ButtonService`。
- [x] `SettingsStorePort` adapter：中性命名空间文件的读写。
- [x] 基础 `CoreEngine` + `live_core_engine`：ports 组装、统一事件流、快照。
- [x] 断连/退出时释放 voice/button 核心状态。
- [x] 平台语音状态轮询到 `VoiceService`。
- [x] 平台音频状态轮询到 `AudioService`（engine ticker 发 `audio.changed`）。
- [x] 真机验证语音 streaming/draining/idle 与方向键/OK 的 `button.edge`。
- [x] 真机验证 `button.gesture`（单击/双击/长按）与 FocusInput 动作注入。
- [x] 真机验证 profile 前台自动切换（Notepad auto-apply → `foreground profile changed`）。
- [x] 现有平台测试继续通过（`shengjian-windows` 490；workspace 689）。

## Phase 3 — 补全核心语义

- [x] 完整手势：单击/双击/长按、按住连发、锁屏释放延迟、OK 上下文点击、deadline ticker。
- [x] 键位方案覆盖层与前台自动应用：contract `ButtonProfilesDocument`、`ButtonProfileService`、`ForegroundPort` + Windows 实现。
- [x] 旧平台 `ButtonMappings` / 旧版方案文件到 core 模型的迁移函数（遗留类型已内联进 adapter，`shengjian-profiles` crate 已删除）。
- [x] 宿主把旧 profile/mapping 文件读入并切换到 core 方案存储。
- [x] core 按键配置持久化：`ButtonProfilesStorePort` / `ButtonMappingStorePort`，启动自动加载、变更写回。
- [x] 隐私边界：contract `RemoteId` 使用 opaque token，事件/诊断日志不含蓝牙地址或 HID 路径。
- [x] profile/settings 高版本 schema 拒载并留 `.bak`。
- [ ] 完整映射/动作模型：快捷键、媒体键、鼠标、聚焦、宏引用、禁用（ActionSink 映射已有）。
- [x] 设置命名空间核心模型：`SettingsDocument` + `DeviceSettingsOverride` 解析/规范化。
- [x] `WindowsSettingsStore` 命名空间文档写入 + 旧扁平文件读取回退。
- [ ] `Engine` 取消/退订与 headless 入口（基础 facade 和 disconnect 生命周期已有）。
- [x] 运行时 profile loader：`ProfileCatalog::from_dir` / `DeviceCatalog::resolve`。
- [ ] RC001/ARN9 身份复核后再启用 profile 运行时匹配。**deferred（2026-11-04 跳过，见 [device-identity.md](device-identity.md)）**
- [ ] 方案按设备型号配置（第一步：模型结构 + RC003 生效）。**计划见 [plan-device-model-profiles.md](plan-device-model-profiles.md)**

## Phase 4 — 可选 shell / 客户端

- [x] headless host：scan/run/JSON events/legacy migrate。
- [x] host 显式 `--mappings`/`--profiles` 默认 transient，`--persist` 才写默认配置。
- [x] 桌面宿主 M0：Tauri 2 + Vue 3 骨架、engine 生命周期、snapshot/event cursor/scan。
- [x] 桌面宿主 M1：UI 内 scan/connect/disconnect 真机闭环。
- [x] 桌面宿主 M2：语音/音频状态、CABLE、gain、物理语音键 UI streaming/draining/idle。
- [x] 桌面宿主 M3：按键事件 reducer、profile 列表/激活/自动切换。
- [x] 桌面宿主 M4：设置页、诊断入口、CSP、单实例、退出清理。
- [x] 桌面宿主 M5：本地 NSIS 包构建、安装、启动、卸载、数据保留。
- [x] 桌面宿主 M5b：本地覆盖安装、数据保留、卸载清理回归。
- [x] 桌面产品扩展：profile 编辑器（新建/编辑/删除、应用目标、自动应用、逐键动作）。
- [x] 桌面产品扩展：托盘菜单、关闭到托盘/关闭即退出设置、开机自启动。
- [x] 桌面产品扩展：设备级音频设置覆盖（gain/endpoint）。
- [x] 桌面产品扩展：profile 原生文件导入/导出。
- [x] 桌面产品扩展：首次配置向导（4 步，可重新运行）。
- [x] 桌面产品扩展：主题设置（system/light/dark）。
- [x] 桌面产品扩展：profile 动作扩展（常用语、鼠标滚轮/移动/点击）。
- [x] 桌面产品扩展 M6：宏库存储/宏管理页/profile `run-macro` 绑定。
- [x] 桌面产品扩展 M6b：宏库原生文件导入/导出、宏测试运行。
- [x] 桌面产品扩展 M6c：宏版本递增/历史、复制宏、运行状态展示。
- [x] 桌面 M7：本地打包/安装/RC003 连接/卸载回归。
- [x] 桌面 M7b：实体语音/OK 单双击长按人工验收通过。
- [x] 桌面 M7c：实体按键触发 `run-macro` 人工验收通过。
- [x] 桌面 M7d：RC003 返回键增强捕获 helper 接入与真机验证。
- [x] 桌面 M8：helper/gadget 随 NSIS 分发、设置页管理员安装入口。
- [x] 桌面 M8b：卸载器清理计划任务；helper 安装状态自动刷新。
- [x] 桌面 M8c：monorepo 结构确认 + HostPlatform 抽象 + Windows 实现隔离。
- [x] 桌面产品扩展：语音输入工具设置（全局 + 设备覆盖）。
- [x] 桌面产品扩展：连接页电量显示。
- [x] 桌面产品扩展：宏步骤排序与历史版本删除。
- [x] 桌面产品扩展：首次配置向导接入语音输入工具。
- [x] 桌面产品扩展：宏步骤超时与分步运行结果。
- [x] 桌面产品扩展：宏保存前中文校验提示。
- [x] 桌面产品扩展：配置备份/恢复（settings + profiles + 宏库）。
- [x] 桌面产品扩展：宏最近运行历史。
- [x] 桌面产品扩展：Profile 快速模板。
- [x] 桌面产品扩展：常用语动作按 host capability 显示。
- [x] 桌面产品扩展：C1/C2/E1/A2/D1（前台目标、快捷键捕获、日志/诊断、低电量、宏历史）。
- [x] 桌面产品扩展：C3/C5/D2/E2/F1（冲突检测、动作校验、步骤拖拽/复制、设置重置、健康检查）。
- [x] 桌面产品扩展：D3/A1/D6（宏搜索、连接详情、宏取消/重试）。
- [x] 桌面产品扩展：Profile 复制方案。
- [x] 桌面 M9a：本地发布清单、SHA-256 校验与验证脚本。
- [x] 桌面 M9b：代码签名链路（SignPath Foundation 免费开源签名，已接入 release workflow；启用需公开仓库与 SignPath 审核，见 [signing.md](signing.md)）
- [x] 桌面 M5c：0.1.0 已发布到 GitHub Release；自动更新未做
- [ ] 安装器、更新器、helper 调度仍由宿主负责。

## 未决

- RC001 与 upstream `xiaomi-arn9` 的关系；当前平台层把 `"ARN9"` 映射为 `Rc003`。
- 便携格式从旧标识改为 `shengjian-transfer` 后，旧导出文件不兼容。
- `helper/rc003` 尚未并入顶层 workspace；需单独跑 `cargo test`。
