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
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/prepare-release.ps1 -Version 0.3.0
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-release.ps1 -Version 0.3.0
```

安装包先由 Tauri 按 `productName` 产出 `声键_<version>_x64-setup.exe`，再由
`scripts/normalize-installer-name.ps1` 归一化为 ASCII 小写 `shengjian_<version>_x64-setup.exe`；
`prepare-release.ps1` 会自动调用该脚本，`manifest.json` 的 `file` 字段指向归一化后的路径。
产品名、版本与安装包路径都从 `src-tauri/tauri.conf.json` 派生，品牌改名后不需要改脚本。
本地构建也可以直接运行 `pnpm build:installer`（`tauri build` + 归一化）。

产物：

- `release/0.3.0/manifest.json`
- 安装包位于 Tauri `target` 目录，不复制进仓库

> 本地 `prepare-release.ps1` **只写 `manifest.json`**，不写 `SHA256SUMS`。
> `SHA256SUMS` 由 release workflow 生成，因为它要同时收录安装包和便携 zip，
> 而便携 zip 是在 CI 里签名后才组装的。

> 本地命令产出的是**未签名**包。代码签名只在 GitHub Actions 的 release workflow 中执行，
> 且需要先在仓库/ SignPath 侧完成配置，见 [signing.md](signing.md)。

> **本地构建与 CI 构建不是逐字节相同的**：同一个 commit 在本地和 GitHub Actions 上
> 编译出的安装包大小与 SHA-256 都不同。因此本地 `manifest.json` 里的 `sha256`
> 只用于本地自查，**不要**写进发布说明或对外公布。见[发布说明与 SHA-256](#发布说明与-sha-256)。

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
Get-AuthenticodeSignature "apps/shengjian-desktop/src-tauri/target/release/bundle/nsis/shengjian_0.3.0_x64-setup.exe"
```

## GitHub 仓库

仓库已创建：<https://github.com/xiaopohair/SHENGJIAN>。首次推送：

```powershell
git remote add origin https://github.com/xiaopohair/SHENGJIAN.git
git push -u origin main
git push origin v0.3.0
```

也可以使用 SSH：`git@github.com:xiaopohair/SHENGJIAN.git`。

### gh CLI

发布后的核对、追踪与修都用 [GitHub CLI](https://cli.github.com/)：

```powershell
winget install --id GitHub.cli      # 已装可跳过
gh auth login                       # 需要 repo + workflow 权限

gh run list --limit 5               # 看 tag 触发的 release workflow
gh run watch <run-id> --exit-status # 跟踪到结束
gh run view --log-failed <run-id>   # 失败时看日志
gh release view v0.4.0              # 确认资产清单
```

## 发布说明与 SHA-256

**规则：发布说明里不写死 SHA-256 和大小。**

Release 正文由 workflow 直接从 `docs/release-notes-<version>.md` 读取
（`body_path`），**在 tag 推送时逐字发布**。而同一个 commit 在本地和 GitHub Actions
上编译出的安装包不是逐字节相同的，哈希与大小都不一样。所以正文里一旦写了具体数值，
就必须在 CI 发布后回填，很容易漏掉、漏掉就会和实际下载的文件对不上。

正文只列文件名，校验信息指向 Release 资产：

```text
Release 资产：
  shengjian_0.4.0_x64-setup.exe      安装包（NSIS，未签名）
  shengjian_0.4.0_x64-portable.zip   便携版（免安装 zip）
  SHA256SUMS                         两个产物的 SHA-256
  manifest.json                      安装包大小与 SHA-256
  portable-manifest.json             便携版大小与 SHA-256

下载后用随包的 SHA256SUMS 校验：
  Get-FileHash shengjian_0.4.0_x64-setup.exe -Algorithm SHA256
```

这样说明与 CI 产物天然一致，不需要任何回填步骤。

### 如果确实要在正文写哈希

只改仓库文件**不会**更新已经发布的 Release，必须同步线上正文：

```powershell
gh release download v0.4.0 --pattern SHA256SUMS --dir $env:TEMP --clobber
gh release download v0.4.0 --pattern manifest.json --dir $env:TEMP --clobber
# 用这两个文件里的值替换正文中的占位符

gh release edit v0.4.0 --notes-file docs/release-notes-0.4.0.md
git add docs/release-notes-0.4.0.md
git commit -m "docs: 回填 0.4.0 发布说明的校验值"
git push origin main
```

0.3.x 的发布说明用的是这种写法，可以直接比对 `docs/release-notes-0.3.1.md`。

### manifest.json 的两个注意点

- 仓库里的 `release/<version>/manifest.json` 记录的是**本地**构建结果；
  CI 会重新生成同名文件并上传，所以仓库版本与线上资产可能不同，**以线上资产为准**。
- 其中的 `published` 字段由 `prepare-release.ps1` 固定写 `false`，CI 不会改写它，
  代码里也没有任何消费者（App 的更新检查读的是 GitHub API 的 `published_at`）。
  该字段只是占位，**不要依据它判断发布状态**。

## 发布

推荐由 tag 触发 `.github/workflows/release.yml`：

```powershell
git tag v0.3.0
git push origin v0.3.0
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

- `shengjian_0.3.0_x64-setup.exe`
- `shengjian_0.3.0_x64-portable.zip`
- `release/0.3.0/manifest.json`
- `release/0.3.0/portable-manifest.json`
- `SHA256SUMS`（手工发布时需要自己生成，本地脚本不产此文件）
- `LICENSE`
- `ATTRIBUTION.md`
- `THIRD_PARTY_NOTICES.md`
- `brand/LICENSE-OFL.txt`
- `helper/rc003/vendor/frida-gadget-COPYING`

Release 正文来自 `docs/release-notes-<version>.md`（0.3.0 为 [release-notes-0.3.0.md](release-notes-0.3.0.md)）。
正文在 tag 推送时就已定型，之后修改需用 `gh release edit` 同步，见[发布说明与 SHA-256](#发布说明与-sha-256)。

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
- Release 资产齐全：安装包、便携 zip、`SHA256SUMS`、`manifest.json`、`portable-manifest.json`、许可证
- 发布说明里**没有**硬编码的 `SHA-256` / 大小，校验信息指向 `SHA256SUMS` 资产；
  若确实写死了数值，已用 `gh release edit v<version> --notes-file ...` 同步线上正文
- Release 里 `SHA256SUMS` 与 `manifest.json` 的哈希相互一致
  （`gh release download v<version> --pattern SHA256SUMS --dir $env:TEMP --clobber`）
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
