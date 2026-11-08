# Windows adapters

把 `shengjian-windows` 的平台状态翻译成 `shengjian-app` 的端口契约。

## RemoteLinkPort

- `PlatformRemote`：适配器只依赖的窄接口，便于无 BLE 硬件测试。
- `WindowsRemoteLink`：
  - `scan` -> `Vec<RemoteCandidate>`（型号、profile id、稳定 token）
  - `connect` -> `ConnectionSnapshot`（phase、remote、generation）
  - `disconnect` -> 清理 last remote
  - `PlatformError` -> 稳定的 `EngineError { domain, code, retryable }`
- `WindowsRemoteLink::live()`：用真实 `WindowsPlatform` 构造。

## AudioSinkPort

- `PlatformAudio`：列表/选择/增益/快照的窄接口，可无 WASAPI 硬件测试。
- `WindowsAudioSink`：
  - `list_endpoints` -> `AudioEndpoint`（recommended = CABLE 候选）
  - `select_endpoint` -> `AudioSnapshot`
  - `set_gain_db` -> 钳制到 0–24 dB 后回读快照
  - phase/generation/错误映射到 contract
- `WindowsAudioSink::live()`：用真实 `WindowsPlatform` 构造。

## ActionSinkPort

- `PlatformActions`：动作执行的窄接口，可无 SendInput/UIA 硬件测试。
- `WindowsPlatformActions`：把 contract 动作映射到现有平台 API。
- `WindowsActionSink`：实现 `ActionSinkPort`
  - `Shortcut` -> `test_shortcut`
  - `OpenApp` -> `test_app_focus`
  - `FocusInput` -> `test_focus_input`
  - `RunMacro` -> `MacroRuntime::start`
  - `Mouse` -> `test_scroll` / `test_mouse_action`
  - `CommonPhrase` -> 当前 `CommonPhraseHost`
  - `Disabled` -> 空操作
- 鼠标点击当前映射为左键；滚动只允许上下；金额 1–100。

## ButtonSourcePort

- `PlatformButtonSource`：`subscribe_button_edges` 的窄接口。
- `WindowsButtonSource`：平台 `RemoteButton` + pressed -> contract `ButtonEdge`，时间戳在适配器侧补。
- `CoreEngine::wire_button_source` 把它接到 `ButtonService`；Raw Input/HID 的平台语义边沿由此进入新 core。
- 平台原有 `ButtonMappingRuntime` 仍保留；后续阶段再消除两套手势/映射语义。

## VoiceSourcePort

- `WindowsVoiceSource`：轮询平台 `ConnectionSnapshot.voice_state`。
- `CoreEngine` ticker 将其 reconcile 到 `VoiceService`，产生 `voice.changed` 事件；重复状态为 no-op。

## ForegroundPort + 键位方案

- `WindowsForeground`：只读前台可执行文件名（不含路径/窗口标题）。
- `CoreEngine::set_button_profiles` / `activate_button_profile` / `sync_foreground` 组合手动激活与 `auto_apply`。
- 覆盖层按 `(button, trigger)` 继承，只有 10 个普通键可被方案覆盖。

## SettingsStorePort

- `WindowsSettingsStore`：默认 `%LOCALAPPDATA%\Shengjian\settings.json`。
- 写入 `SettingsDocument`（`global` + `devices.<id>.<platform>`）；读取时支持同路径旧扁平 `AppSettings`，自动转成 `global`。
- `load()` 会按 `selectedRemoteId` + `windows` 解析设备覆盖，返回有效扁平设置。
- 缺省文件返回默认文档；损坏文件返回稳定的 `settings-invalid`。
- 写入使用临时文件 + rename，失败时 copy 兜底。

## 旧模型迁移

- `migrate_button_mappings(&ButtonMappings) -> ButtonMappingSet`
- `migrate_button_profiles_document(&ButtonProfilesDocument) -> core ButtonProfilesDocument`

把平台动作映射到 contract 动作；鼠标点击目前收敛为左键，聚焦档案/应用路径等旧字段不进入 core。文件读取和切换由宿主完成。

## 一键组装

```rust
let engine = shengjian_windows_adapter::live_core_engine(event_sink)?;
```

`live_core_engine` 会创建**一个共享的 `WindowsPlatform`**，把 remote/audio/action/button source/settings 全部接上，并返回 `CoreEngine`。它会启动平台后台线程（BLE/audio/input）和 core 的 gesture deadline ticker，应保证每进程只创建一次。

## 真机 smoke

```powershell
cargo run -p shengjian-windows-adapter --example live_smoke
cargo run -p shengjian-windows-adapter --example live_smoke -- --connect <remote-id>
```

会启动真实平台、扫描已配对遥控器、尝试连接、打印音频端点/增益/事件，然后断开。没有真机时只做扫描，不伪造连接成功。

尚未实现：键位方案覆盖层与前台自动应用。

```rust
let platform = std::sync::Arc::new(shengjian_windows::WindowsPlatform::default());
let link = std::sync::Arc::new(
    shengjian_windows_adapter::WindowsRemoteLink::new(platform.clone()),
);
let audio = std::sync::Arc::new(
    shengjian_windows_adapter::WindowsAudioSink::new(platform.clone()),
);
let actions = std::sync::Arc::new(
    shengjian_windows_adapter::WindowsActionSink::new(
        std::sync::Arc::new(
            shengjian_windows_adapter::WindowsPlatformActions::new(platform),
        ),
    ),
);
// 也可以直接用 live_core_engine 一次装配。
```
