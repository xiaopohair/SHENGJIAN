# 全按键支持 helper（hid-host）本地 setup

`hid-host` 是提权助手：小米遥控器的返回键、音量± 在 Windows HID→VK 映射阶段被系统丢弃，
常规 Raw Input 收不到；Chromecast 的按键报告则需要它在 HOGP 驱动进程内读取后转发。
桌面宿主已支持：

- 设置 → 音频及语音 → 全按键支持服务：安装/更新与开关
- helper、`frida-gadget.dll`、`WinUHid.dll` 与驱动包随安装包放在 `resources\hid-host`
- 设置页「安装/更新」会以 UAC 启动包内 helper 的 `--install-task`
- 启用后下发 `enhanced_capture` 目标 usage `80,81,f1`
- 启动时 / 保存设置时触发计划任务 `Shengjian HID Host`

## 推荐安装

1. 安装当前版本（0.9.6 或更新）的安装包
2. 设置 → 音频及语音 → 全按键支持服务，点击「安装/更新」，确认 UAC
3. 在同一分组打开「全按键支持」开关
4. 连接页设备卡显示「全按键支持 · 已安装」

日志应出现：

```text
hid_bridge event=helper_authenticated
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
cd helper/hid-host
cargo build --release
python vendor\fetch_frida_gadget.py
```

`pnpm prepare:helper`（Tauri `beforeBuildCommand` 会自动执行）会复制到
`apps/shengjian-desktop/src-tauri/resources/hid-host/`。

## 手工 fallback

```powershell
# 管理员 PowerShell
.\target\release\shengjian-helper.exe --install-task
```

## 当前边界

- 卸载钩子会通过 UAC 调用包内 helper `--remove-task`；若 UAC 被拒绝，任务可能残留
- 计划任务指向安装目录内 helper；卸载后任务删除，重装/重新安装 helper 可恢复
- 自动更新：安装包 preinstall hook 会先停掉计划任务，并把被占用的 helper exe / dll 改名挪开，
  避免覆盖失败；应用重启后按「全按键支持」开关重新拉起新 helper。
## 改名（2026-11-17）：rc003-helper → hid-host

组件语义早就超出「RC003 的桥」了（三款遥控器、按键合成、虚拟 HID 设备、驱动管理），
所以统一改名为 `hid-host`：

| 旧 | 新 |
|---|---|
| 计划任务 `Shengjian RC003 Helper` | `Shengjian HID Host` |
| 目录 `resources/rc003/` | `resources/hid-host/` |
| 运行时目录 `%ProgramData%\Shengjian\rc003-helper` | `%ProgramData%\Shengjian\hid-host` |
| 模块 `rc003_bridge` / `Rc003Bridge` | `hid_bridge` / `HidBridge` |
| 注入脚本 `rc003_agent.js` | `hid_agent.js` |
| 日志前缀 `rc003_bridge event=` | `hid_bridge event=` |

**升级迁移**（自动，不需要用户操作）：

- 应用启动时尽力清理旧目录 `resources/rc003/`；
- 助手 `--install-task` 会注册新任务后**删掉旧任务**，并清理旧运行时目录；
- 应用判断「是否已安装」时新旧任务名都认，所以装过旧版的机器不会误报「未安装」；
- 卸载时两个名字都删。

历史日志里仍是 `rc003_bridge event=` 前缀（0.9.2 及以前），排障时按两个前缀都搜。