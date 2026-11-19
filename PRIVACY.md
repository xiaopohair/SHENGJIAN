# Privacy policy / 隐私政策

声键（SHENGJIAN）是本地优先的 Windows 桌面工具。

- 不要求账号，不包含遥测、分析或崩溃上报。
- 不收集、上传语音内容或转写文本；语音由用户选择的本地输入工具处理。
- 设置、键位方案、宏、历史记录、日志和已下载的更新包默认只保存在本机 `%LOCALAPPDATA%\Shengjian`。
- 自动更新会访问 GitHub Releases（`api.github.com`、`github.com`，下载重定向到 `release-assets.githubusercontent.com`）检查版本，并在发现新版本后下载安装包；请求只包含常规 HTTP 头，不包含账号、设备标识、语音或转写内容。可在「设置 → 软件更新」关闭自动检查/自动下载；手动检查更新、打开外部链接仍会联网。
- 只有在用户主动打开日志目录、导出诊断包或访问外部链接时，其他数据才会离开应用。
- 安装包内的第三方组件（Frida Gadget、WinUHid 等）及其许可说明见 `THIRD_PARTY_NOTICES.md`。

除上述自动更新请求外，本程序不会向其他联网系统传输任何信息，除非用户或安装/操作人员明确要求。

Apart from the update check/download described above, this program does not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it. The update requests carry no account, device identifier, voice, or transcript data, and automatic update checks/downloads can be disabled under Settings → Software updates.
