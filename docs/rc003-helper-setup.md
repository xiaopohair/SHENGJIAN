# RC003 增强捕获 helper（本地 setup）

RC003 的返回键、音量± 在 Windows HID→VK 映射阶段被系统丢弃，常规 Raw Input
收不到。桌面宿主已支持：

- 连接页 `启用 RC003 增强捕获（返回键、音量键）`
- helper 与 `frida-gadget.dll` 随安装包放在 `resources\rc003`
- 连接页“安装/更新 helper”会以 UAC 启动包内 helper 的 `--install-task`
- 启用后下发 `enhanced_capture` 目标 usage `80,81,f1`
- 启动时 / 保存设置时触发计划任务 `Shengjian RC003 Helper`

## 推荐安装

1. 安装 0.1.0 包
2. 连接页勾选“启用 RC003 增强捕获（返回键、音量键）”
3. 点击“安装/更新 helper”，确认 UAC
4. helper 状态显示“已安装”

日志应出现：

```text
rc003_bridge event=helper_authenticated
enhanced_capture event=ownership_resumed usages=80,81,f1
```

按返回键应出现：

```text
map_edges count=1 detail=Back=true
button.edge edge.button=back
```

## 开发构建资源

打包前需要准备 helper 资源：

```powershell
cd helper/rc003
cargo build --release
python vendor\fetch_frida_gadget.py
```

`pnpm prepare:helper`（Tauri `beforeBuildCommand` 会自动执行）会复制到
`apps/shengjian-desktop/src-tauri/resources/rc003/`。

## 手工 fallback

```powershell
# 管理员 PowerShell
.\target\release\shengjian-helper.exe --install-task
```

## 当前边界

- 卸载钩子会通过 UAC 调用包内 helper `--remove-task`；若 UAC 被拒绝，任务可能残留
- 计划任务指向安装目录内 helper；卸载后任务删除，重装/重新安装 helper 可恢复
