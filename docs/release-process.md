# 发布流程

## 当前路线

- GitHub 公共仓库（GPL-3.0-only）
- 只发 **x64**
- 本地包**未签名**；CI 配置 SignPath 后由 workflow 签名
- 手动下载，**不做自动更新**
- helper/gadget 随包分发，首次启用需要一次 UAC
- 卸载会清理 helper 计划任务

## 本地发布

```powershell
cd apps/shengjian-desktop
pnpm tauri build

cd ../..
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/prepare-release.ps1 -Version 0.1.0
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-release.ps1 -Version 0.1.0
```

安装包先由 Tauri 按 `productName` 产出 `声键_<version>_x64-setup.exe`，再由
`scripts/normalize-installer-name.ps1` 归一化为 ASCII 小写 `shengjian_<version>_x64-setup.exe`；
`prepare-release.ps1` 会自动调用该脚本，`manifest.json` 的 `file` 字段指向归一化后的路径。
产品名、版本与安装包路径都从 `src-tauri/tauri.conf.json` 派生，品牌改名后不需要改脚本。
本地构建也可以直接运行 `pnpm build:installer`（`tauri build` + 归一化）。

产物：

- `release/0.1.0/manifest.json`
- `release/0.1.0/SHA256SUMS`
- 安装包位于 Tauri `target` 目录，不复制进仓库

> 本地命令产出的是**未签名**包。代码签名只在 GitHub Actions 的 release workflow 中执行，
> 且需要先在仓库/ SignPath 侧完成配置，见 [signing.md](signing.md)。

### 便携版（免安装 zip）

```powershell
cd apps/shengjian-desktop
pnpm build:portable
```

产物：

- `apps/shengjian-desktop/src-tauri/target/release/portable/shengjian_<version>_x64-portable.zip`
- `release/<version>/portable-manifest.json`（大小、SHA-256、exe 签名状态）

`scripts/build-portable.ps1 -SkipBuild` 可跳过构建、直接复用 `target/release/shengjian-desktop.exe`，
适合 CI 在签名后组装便携包。便携版依赖系统 WebView2 Runtime；设置、宏、历史与日志仍写入
`%LOCALAPPDATA%\Shengjian`。RC003 helper 的计划任务指向便携目录，移动目录后需要重新安装；
删除目录前以管理员身份运行 `resources\rc003\shengjian-helper.exe --remove-task`。

## 代码签名（SignPath）

本项目接入 [SignPath Foundation](https://signpath.org) 的免费开源代码签名，公开政策见
[CODE_SIGNING_POLICY.md](../CODE_SIGNING_POLICY.md)，配置步骤见 [signing.md](signing.md)。

- 签名在 `v*` tag 触发的 [release.yml](../.github/workflows/release.yml) 中执行；
- 未配置 `SIGNPATH_API_TOKEN` 时自动跳过，产出未签名包；
- 配置后每次 Release 产生两个签名请求（app exe、NSIS 安装包），需要在 SignPath 中**人工审批**；
- 审批通过后，release 里的 `manifest.json` / `SHA256SUMS` 基于已签名的安装包生成；
- 签名失败或超时会终止 workflow，不会发布半签名产物。

本地校验：

```powershell
Get-AuthenticodeSignature "apps/shengjian-desktop/src-tauri/target/release/shengjian-desktop.exe"
Get-AuthenticodeSignature "apps/shengjian-desktop/src-tauri/target/release/bundle/nsis/shengjian_0.1.0_x64-setup.exe"
```

## GitHub 仓库

仓库已创建：<https://github.com/xiaopohair/SHENGJIAN>。首次推送：

```powershell
git remote add origin https://github.com/xiaopohair/SHENGJIAN.git
git push -u origin main
git push origin v0.1.0
```

也可以使用 SSH：`git@github.com:xiaopohair/SHENGJIAN.git`。

## 发布

推荐由 tag 触发 `.github/workflows/release.yml`：

```powershell
git tag v0.1.0
git push origin v0.1.0
```

workflow 会：

1. 校验 tag 与 package.json / tauri.conf.json 版本
2. 从 `tauri.conf.json` 读取 `productName`，构建后调用 `scripts/normalize-installer-name.ps1` 归一化为 `shengjian_<version>_x64-setup.exe`（不写死产品名），Release 标题为「声键 &lt;version&gt;」
3. 构建 helper、下载并校验 Frida gadget
4. 构建未签名 NSIS 安装包
5. 配置 SignPath 时签 app exe 与安装包
6. 用签名后的 exe 运行 `scripts/build-portable.ps1 -SkipBuild`，组装便携 zip
7. 生成 `manifest.json`、`portable-manifest.json` 与 `SHA256SUMS`（安装包 + 便携 zip）
8. 创建 GitHub Release，上传安装包、便携 zip、manifest、校验和、许可证

也可手工上传 Release 资产：

- `shengjian_0.1.0_x64-setup.exe`
- `shengjian_0.1.0_x64-portable.zip`
- `release/0.1.0/manifest.json`
- `release/0.1.0/portable-manifest.json`
- `release/0.1.0/SHA256SUMS`
- `LICENSE`
- `ATTRIBUTION.md`
- `THIRD_PARTY_NOTICES.md`
- `brand/LICENSE-OFL.txt`
- `helper/rc003/vendor/frida-gadget-COPYING`

Release 正文来自 `docs/release-notes-<version>.md`（0.1.0 为 [release-notes-0.1.0.md](release-notes-0.1.0.md)）。

## 发布前检查

- 安装、启动、RC003 扫描/连接
- 语音 streaming/draining/idle
- OK single/double/long
- Profile 自动切换
- `run-macro` 实体按键
- 返回键增强捕获
- 品牌：安装包文件名、安装向导、开始菜单/桌面快捷方式、窗口标题、托盘提示均为「声键」
- 品牌：深色界面/窗口图标用亮松青，浅色界面/窗口图标用松青·深（`#0F7A63`）；托盘深色任务栏用亮松青、浅色任务栏用墨色
- 卸载后计划任务清理、数据保留
- `docs/release-notes-<version>.md` 已填写安装包 SHA-256（不再是占位符）
- Release 用户能看到 SmartScreen 未签名提示和 SHA-256 校验说明
- 启用签名后：两个 SignPath 签名请求审批通过，`Get-AuthenticodeSignature` 对 exe 与安装包均为 `Valid`

## 许可

- 项目：GPL-3.0-only，源码在 GitHub tag
- Frida Gadget：wxWindows Library Licence
  - 版本与摘要见 `helper/rc003/vendor/frida-gadget.lock.json`
  - 许可全文随包分发到 `resources/licenses/COPYING.frida-gadget`
- 发布 Release 时同时上传 `LICENSE`、`ATTRIBUTION.md`、`THIRD_PARTY_NOTICES.md`

## 后续（需要额外授权）

- 代码签名证书与签名流水线
- ARM64 包
- 自动更新 endpoint、签名密钥、updater
