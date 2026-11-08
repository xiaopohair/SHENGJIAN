# 多平台结构

## 结论

**保持 monorepo，不拆成多个 Git 仓库。**

当前是“一个仓库、多个独立构建单元”：

```text
crates/                      # 平台无关 core workspace
apps/shengjian-desktop/ # 共享 Tauri/Vue 前端 + Windows host
helper/rc003/                # Windows 专用提权 helper
apps/shengjian-windows-host/
```

未来增加 macOS/Linux 时，继续在同一个仓库增加平台实现，而不是复制仓库。

## 分层

```text
contract / domain / app / testkit
        ↑
platform adapters (windows, future macos/linux)
        ↑
host services (macro runtime, enhanced capture, diagnostics, helper)
        ↑
Tauri commands
        ↑
Vue UI
```

- core 不依赖 Windows/Tauri
- platform adapter 实现 core ports
- desktop command 层只面向 `HostPlatform`，不 import Windows 类型
- Windows 专属实现放在 `apps/shengjian-desktop/src-tauri/src/platform/windows.rs`

## 已经完成

- `HostPlatform` / `HostCapabilities` / `HostError` / `MacroRunStatusView`
- `AppState` 持有 `Arc<dyn HostPlatform>`，不再持有 `WindowsPlatform`
- Tauri 命令不再直接依赖 `shengjian_windows` 或 Windows 宏类型
- Windows 专属依赖改为 `[target.'cfg(windows)'.dependencies]`
- UI 从 `host_capabilities` 获取平台标识
- Windows 实现封装：
  - 宏运行时
  - 增强捕获
  - helper 状态/安装/启动
  - 诊断日志目录
  - 平台诊断事件

## 后续平台接入

### 新增平台 crate

```text
crates/shengjian-macos/
crates/shengjian-macos-adapter/
```

实现现有 ports：

- `RemoteLinkPort`
- `AudioSinkPort`
- `ActionSinkPort`
- `ButtonSourcePort`
- `VoiceSourcePort`
- `SettingsStorePort`
- `ForegroundPort`

### 新增 host 实现

```text
apps/shengjian-desktop/src-tauri/src/platform/macos.rs
```

实现 `HostPlatform`；不支持的能力返回空实现/错误，并由 `HostCapabilities` 让 UI 隐藏入口。

### UI 能力化

Windows 专属入口（RC003 增强捕获/helper）应由 capability 控制，而不是硬编码显示。

### 暂不做

- 不在 core 中放平台分支
- 不复制一套 Vue UI
- 不把 helper 做成跨平台
- 不在没有真实平台实现前拆仓库

## 当前限制

- 桌面宿主仍然只能编译 Windows；`platform/mod.rs` 目前对非 Windows 明确报错
- macOS/Linux adapter 尚未实现
- 宏库路径、helper、托盘/自启动仍需按平台补实现或能力降级
