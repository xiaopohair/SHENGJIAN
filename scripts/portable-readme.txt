声键（SHENGJIAN）便携版 {{VERSION}}

1. 解压后直接运行 shengjian-desktop.exe，无需安装。
2. 需要系统已安装 WebView2 Runtime（Windows 11 自带；Windows 10 如缺失请安装 Evergreen Runtime）。
3. 设置、宏、历史与日志保存在 %LOCALAPPDATA%\Shengjian，删除本目录不会删除这些数据。
4. RC003 增强捕获需要 helper：连接页勾选并点“安装/更新 helper”，会请求一次 UAC。helper 计划任务指向当前目录；移动目录后需重新安装。删除本目录前请以管理员身份运行：
   resources\rc003\shengjian-helper.exe --remove-task
5. 开机自启写入当前 exe 路径；移动目录后需在设置中重新开启。
6. 若该构建未签名，SmartScreen 可能提示；请只从官方 Release 下载并校验 SHA-256。
7. 便携版与安装版共用单实例标识和 %LOCALAPPDATA%\Shengjian 数据目录，不能同时运行。

许可证与第三方声明见 resources\licenses 以及根目录 LICENSE、ATTRIBUTION.md、THIRD_PARTY_NOTICES.md。
