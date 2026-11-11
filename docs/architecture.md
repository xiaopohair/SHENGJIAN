# 核心架构

## 分层

```text
shengjian-contract
  ├─ DeviceProfile / Catalog
  ├─ 逻辑按钮 / 触发 / 动作
  ├─ Snapshot / Event / Error / ID / Generation
  └─ 无平台 I/O

shengjian-domain
  ├─ ATVV / ADPCM / PCM / voice / pipeline
  ├─ macros / settings / statistics / transcripts
  └─ 最小手势策略

shengjian-app
  ├─ ports：RemoteLink / AudioSink / ActionSink / SettingsStore / EventSink
  ├─ ConnectionService / VoiceService / ButtonService
  ├─ AudioService / SettingsService / DeviceCatalog
  ├─ CoreEngine（ports 组装 + 统一快照）
  └─ EventPublisher（seq + generation）

shengjian-windows
  ├─ BLE/GATT、WASAPI、Raw Input/HID
  ├─ SendInput、IME/TSF、UIA/focus、应用启动
  └─ 平台实现；不依赖 app

shengjian-windows-adapter
  ├─ PlatformRemote 窄接口（便于无硬件测试）
  ├─ WindowsRemoteLink -> RemoteLinkPort
  └─ 平台状态/错误/型号 -> contract

shengjian-testkit
  ├─ FakeRemoteLink / FakeAudioSink / FakeActionSink
  ├─ InMemorySettingsStore / RecordingSink
  └─ 集成测试

helper/rc003
  └─ 独立原生 helper（提权计划任务 / 增强捕获）
```

平台适配器只实现 `ports.rs`；它们不应该把 `HWND`、COM、WinRT、HID usage 或 UI 类型带进 `shengjian-contract`/`shengjian-app`。

## 当前边界

- `shengjian-windows-adapter` 已接 `RemoteLinkPort`：scan/connect/disconnect/snapshot 会映射成 contract，包含型号、设备 profile id、稳定 token、generation 和结构化错误。
- 同一 adapter 已接 `AudioSinkPort`：endpoint 列表、选择、增益、phase/generation/错误映射；音频数据本身仍由平台层/BLE 管道处理。
- 同一 adapter 已接 `ActionSinkPort`：快捷键、打开应用、聚焦、宏、滚动/移动/点击、常用语；映射到现有 `test_shortcut`/`test_app_focus`/`test_focus_input`/`MacroRuntime`/`test_scroll`/`test_mouse_action`，不重写注入逻辑。
- `shengjian-app::CoreEngine` 已负责组装 ports、服务、Catalog 和统一事件流；测试使用 fake ports，真实组合可用 `live_core_engine(...)` 一次装配共享的 `WindowsPlatform`。
- `WindowsButtonSource` 已把 Raw Input/HID 的平台语义边沿映射为 contract `ButtonEdge`，`CoreEngine::wire_button_source` 接到 `ButtonService`；`ButtonService` 内含完整手势识别和 deadline ticker（单击/双击/长按/连发/锁屏释放延迟/OK 上下文点击）。平台原有的手势/映射引擎仍保留，后续阶段再消除重复语义。
- `WindowsVoiceSource` 轮询平台 `ConnectionSnapshot.voice_state`，`CoreEngine` ticker 将其 reconcile 到 `VoiceService`；真实语音键会出现在 core 事件流。
- `AudioService::sync` 轮询 `AudioSinkPort::snapshot`，ticker 在流式/失效状态变化时发 `audio.changed`。
- `WindowsSettingsStore` 提供中性命名空间下的文件读写（默认 `%LOCALAPPDATA%\Shengjian\settings.json`）。
- core 键位方案：`ButtonProfilesDocument` + `ButtonProfileService` 做手动/前台自动选择与按 `(button, trigger)` 继承的覆盖层；`ForegroundPort` 由 Windows adapter 用只读前台可执行文件名实现。旧版方案/映射文件（遗留 `button-profiles.json` 形状与平台 `ButtonMappings`）可通过 adapter 的 `migrate_*` 函数转成 core 模型；这些遗留类型已内联在 adapter 的 `legacy` 模块里，原 `shengjian-profiles` crate 已删除（它不在运行链路上）。
- 设置命名空间：`SettingsDocument` 保留 global `AppSettings`，设备覆盖放在 `devices.<id>.<platform>.*`；`WindowsSettingsStore` 已写命名空间文档，并兼容读取同路径的旧扁平文件作为 `global`。
- `DeviceCatalog::load_dir` / `resolve` 支持运行时 profile 加载，并优先用 `device_id`、其次型号/蓝牙名匹配；RC001/ARN9 的真实身份仍未确认。
- `CoreEngine::disconnect` 在断连时强制释放 voice 与按键手势状态，`shutdown` 是幂等退出路径。
- `shengjian-windows` 仍保留其原有平台内部结构（`WindowsPlatform`、`ble.rs`、`audio.rs` 等），adapter 只做翻译，不重写平台逻辑。
- `helper/rc003` 是独立 workspace，不参与顶层 `cargo test --workspace`；它只通过原有的桥接/计划任务边界与平台层交互。
- 桌面外壳在 `apps/shengjian-desktop`（Tauri 2 + Vue 3），通过 `HostPlatform` 使用 core；core 不依赖 Tauri/Vue/Windows UI。
- `apps/shengjian-windows-host` 是 headless 宿主：start live engine、scan/run、JSON 事件、legacy settings/mappings/profiles 迁移；`run --profiles` 每 500ms 检测前台变化，仅在 profile 变化时重应用映射。它依赖 core，反向不允许。
- `WindowsRemoteLink` 在 adapter 内部保留平台 device path，contract 的 `RemoteId` 只暴露 opaque token；core 事件不含蓝牙地址，raw-input 诊断只记录路径存在性。
- `ButtonProfilesStorePort` / `ButtonMappingStorePort` 负责 core 按键配置持久化；`live_core_engine` 启动自动加载，`set_button_mappings` / `set_button_profiles` / 手动激活时写回；显式导入默认 transient，需显式持久化。

## 命名清理

原产品命名已统一替换为「声键 / Shengjian」命名：crate 使用 `shengjian-*`，环境变量使用 `SHENGJIAN_*`，helper 为 `shengjian-helper`，日志/临时文件/计划任务/命名事件使用 `shengjian-*`，便携组合动作格式为 `shengjian-transfer` / `.shengjian`。上游参考项目（`GetSayAll/remote-mic-app-windows`、`HD838A/remote-mic-app`）的仓库名与外部类型名保持原样，见 `ATTRIBUTION.md`。

便携格式的 ID/扩展名更改是有意的不兼容变更。

## 下一步

1. 复核 RC001/ARN9 设备身份，通过后再启用 profile 运行时匹配（见 `device-identity.md`）。
   方案按设备型号配置分两步：第一步只做模型结构并让 RC003 生效，计划见 `plan-device-model-profiles.md`。
2. 决定是否把 `helper/rc003` 并入顶层 workspace，统一 `cargo test --workspace` 覆盖。
3. 对外发布前实跑 GitHub Actions（tag → release workflow），并视授权启用 SignPath 签名与 ARM64。
