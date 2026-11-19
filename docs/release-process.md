# 发布流程

## 当前路线

- GitHub 公共仓库（GPL-3.0-only）
- 只发 **x64**
- 本地包**未签名**；CI 配置 SignPath 后由 workflow 签名
- 安装版支持应用内自动更新（0.9.6 起；检查 → 后台下载 → SHA-256 校验 → 确认后重启安装）；便携版手动下载
- 0.9.5 及更早版本没有更新器，升级到 0.9.6 需手动安装一次；从 0.9.6 起后续版本可自动升级
- helper/gadget 随包分发，首次启用需要一次 UAC
- 卸载会清理 helper 计划任务

## 本地发布

发布前如需调整「推荐应用」或「语音输入工具」，先编辑仓库根目录的
`config/app-presets.json` / `config/voice-tool-presets.json`（说明见
[config-catalogs.md](config-catalogs.md)）；`pnpm tauri build` 会通过
`prepare:config` 自动复制进安装包资源。安装目录里的
`resources/config/*.json` 也可直接修改，重启生效。

```powershell
cd apps/shengjian-desktop
pnpm tauri build

cd ../..
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/prepare-release.ps1 -Version 0.9.6
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-release.ps1 -Version 0.9.6
```

安装包先由 Tauri 按 `productName` 产出 `声键_<version>_x64-setup.exe`，再由
`scripts/normalize-installer-name.ps1` 归一化为 ASCII 小写 `shengjian_<version>_x64-setup.exe`；
`prepare-release.ps1` 会自动调用该脚本，`manifest.json` 的 `file` 字段指向归一化后的路径。
产品名、版本与安装包路径都从 `src-tauri/tauri.conf.json` 派生，品牌改名后不需要改脚本。
本地构建也可以直接运行 `pnpm build:installer`（`tauri build` + 归一化）。

产物：

- `release/0.9.6/manifest.json`
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
`%LOCALAPPDATA%\Shengjian`。全按键支持 helper 的计划任务指向便携目录，移动目录后需要重新安装；
删除目录前以管理员身份运行 `resources\hid-host\shengjian-helper.exe --remove-task`。

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
Get-AuthenticodeSignature "apps/shengjian-desktop/src-tauri/target/release/bundle/nsis/shengjian_0.9.6_x64-setup.exe"
```

## GitHub 仓库

仓库已创建：<https://github.com/xiaopohair/SHENGJIAN>。首次推送：

```powershell
git remote add origin https://github.com/xiaopohair/SHENGJIAN.git
git push -u origin main
git push origin v0.9.6
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

### manifest.json 的注意点

仓库里的 `release/<version>/manifest.json` 记录的是**本地**构建结果；
CI 会重新生成同名文件并上传，所以仓库版本与线上资产可能不同，**以线上资产为准**。

字段固定为 `version` / `file` / `sizeBytes` / `sha256` / `builtAtUtc` / `signed`，
由 `prepare-release.ps1` 写入、`verify-release.ps1` 校验、release workflow 读取 `file`。

> 早期版本还写过一个 `published` 字段，它恒为 `false`、CI 不改写、代码里也没有消费者
> （App 的更新检查读的是 GitHub API 的 `published_at`），容易误判发布状态，
> 已在 0.4.0 之后从脚本与产物中移除。判断发布状态请用 `gh release view`。

## 发布

推荐由 tag 触发 `.github/workflows/release.yml`：

```powershell
git tag v0.9.6
git push origin v0.9.6
```

workflow 会：

1. 校验 tag 与 package.json / tauri.conf.json 版本
2. 从 `tauri.conf.json` 读取 `productName`，构建后调用 `scripts/normalize-installer-name.ps1` 归一化为 `shengjian_<version>_x64-setup.exe`（不写死产品名），Release 标题只写版本号 `<version>`（不带产品名）
3. 构建 helper、下载并校验 Frida gadget
4. 构建未签名 NSIS 安装包
5. 配置 SignPath 时签 app exe 与安装包
6. 用签名后的 exe 运行 `scripts/build-portable.ps1 -SkipBuild`，组装便携 zip
7. 生成 `manifest.json`、`portable-manifest.json` 与 `SHA256SUMS`（安装包 + 便携 zip）
8. 创建 GitHub Release，上传安装包、便携 zip、manifest、校验和、许可证

也可手工上传 Release 资产：

- `shengjian_0.9.6_x64-setup.exe`
- `shengjian_0.9.6_x64-portable.zip`
- `release/0.9.6/manifest.json`
- `release/0.9.6/portable-manifest.json`
- `SHA256SUMS`（手工发布时需要自己生成，本地脚本不产此文件）
- `LICENSE`
- `ATTRIBUTION.md`
- `THIRD_PARTY_NOTICES.md`
- `brand/LICENSE-OFL.txt`
- `helper/hid-host/vendor/frida-gadget-COPYING`

Release 正文来自 `docs/release-notes-<version>.md`（0.9.6 为 [release-notes-0.9.6.md](release-notes-0.9.6.md)）。
正文在 tag 推送时就已定型，之后修改需用 `gh release edit` 同步，见[发布说明与 SHA-256](#发布说明与-sha-256)。

## 发布前检查

### 本地校验命令（三个 Cargo workspace + 前端，一个都不能漏）

仓库里有**三个独立的 Cargo workspace**：根 workspace、`apps/shengjian-desktop/src-tauri`（桌面宿主）、
`helper/hid-host`。`cargo fmt --all` / `cargo check --workspace` / `cargo test --workspace` 只覆盖**当前所在的那个**，
在根目录跑完不代表桌面 crate 和 helper 也干净——2026-11 就因为漏跑桌面 crate 的 `cargo fmt`，
CI 的 `Desktop Rust format` 连红三次。

```powershell
# 1) 根 workspace
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace

# 2) 桌面宿主（独立 workspace）
cd apps/shengjian-desktop/src-tauri
cargo fmt -- --check
cargo test
cd ../../..

# 3) hid-host 全按键支持 helper（独立 workspace）
cd helper/hid-host
cargo test
cargo build --release
cd ../..

# 4) 前端
cd apps/shengjian-desktop
pnpm lint
pnpm format:check
pnpm test
pnpm build
pnpm config:check
```

对应 CI 的三个作业：`core`（第 1 步）、`desktop`（第 2、4 步）、helper（第 3 步）。

- 安装、启动、各型号遥控器扫描/连接（至少 RC003）
- 语音 streaming/draining/idle
- OK single/double/long
- Profile 自动切换
- `run-macro` 实体按键
- 返回键增强捕获
- 品牌：安装包文件名、安装向导、开始菜单/桌面快捷方式、窗口标题、托盘提示均为「声键」
- 品牌：深色界面/窗口图标用亮松青，浅色界面/窗口图标用松青·深（`#0F7A63`）；托盘与侧栏使用深色底应用图标
- 卸载后计划任务清理、数据保留
- Release 资产齐全：安装包、便携 zip、`SHA256SUMS`、`manifest.json`、`portable-manifest.json`、许可证
- 自动更新：从第二个带更新器的版本起，用上一版安装包验证“关于页发现新版本 → 显示下载进度 →
  重启并安装后版本号更新、设置/方案/宏保留”；首个带更新器版本（0.9.6）用测试构建/本地 manifest
  验证下载、SHA-256 校验与覆盖安装。下载被中断或哈希不符时停在「可下载」状态并给出错误，不会启动安装
- 自动更新：便携版与 `tauri dev` 只显示「查看更新」，不出现应用内安装入口
- 首个带更新器的版本（0.9.6）：发布说明与 README 必须写清 0.9.5 及更早用户需手动安装一次；
  安装后才能自动升级后续版本
- 发布说明里**没有**硬编码的 `SHA-256` / 大小，校验信息指向 `SHA256SUMS` 资产；
  若确实写死了数值，已用 `gh release edit v<version> --notes-file ...` 同步线上正文
- Release 里 `SHA256SUMS` 与 `manifest.json` 的哈希相互一致
  （`gh release download v<version> --pattern SHA256SUMS --dir $env:TEMP --clobber`）
- Release 用户能看到 SmartScreen 未签名提示和 SHA-256 校验说明
- 启用签名后：两个 SignPath 签名请求审批通过，`Get-AuthenticodeSignature` 对 exe 与安装包均为 `Valid`

## 许可

- 项目：GPL-3.0-only，源码在 GitHub tag
- Frida Gadget：wxWindows Library Licence
  - 版本与摘要见 `helper/hid-host/vendor/frida-gadget.lock.json`
  - 许可全文随包分发到 `resources/licenses/COPYING.frida-gadget`
- 发布 Release 时同时上传 `LICENSE`、`ATTRIBUTION.md`、`THIRD_PARTY_NOTICES.md`

## 应用内自动更新

> 从 **0.9.6** 起生效。0.9.5 及更早版本没有更新器，用户升级到 0.9.6 仍需手动下载安装一次；
> 0.9.6 之后的新版本才进入自动检查/下载/安装流程。

安装版（NSIS，`%LOCALAPPDATA%\声键`）支持应用内升级；便携版与 `tauri dev` 构建不支持，
界面会提示手动下载。整条链路不依赖 Tauri 官方 updater 的独立签名密钥，直接复用每次
Release 已经上传的资产：

1. 前端按现有 GitHub Releases API 检查版本（可筛选预发布），`update_support` 判断当前
   是不是安装版；
2. 需要下载时，Rust 侧取 `releases/download/<tag>/manifest.json`，校验 `version`、安装包
   文件名、`sizeBytes` 与 `sha256`；
3. 安装包流式下载到 `%LOCALAPPDATA%\Shengjian\Updates`，边下边算 SHA-256；大小或哈希
   不符立即删除，不进入可安装状态；
4. 用户确认「重启并安装」后，以 `/P /UPDATE /R` 启动 NSIS 安装包并退出应用。安装包的
   `/UPDATE` 模式跳过卸载/重装选择，通过 Restart Manager 关闭正在运行的实例并覆盖文件，
   `/R` 负责重新启动新版本；
5. 安装包 preinstall hook 会先停掉 `Shengjian HID Host` 计划任务并把被占用的 helper
   exe/dll 改名挪开，避免覆盖失败（同时兼容 0.9.3 之前的 `resources\rc003` 布局）。

注意事项：

- **信任锚是 HTTPS + Release 内的 manifest SHA-256**，不是第二把更新签名密钥。能改
  GitHub Release 的人也能同时改 manifest；这一点与手动下载安装包相同。后续若要更强
  的保证，可再引入 Tauri updater 的签名密钥或校验 Authenticode。
- 未启用 SignPath 时 CI 产物**未签名**，应用内安装不会经过 SmartScreen 的「未知发布者」提示；
  启用签名后安装包带 Authenticode 签名，但更新器仍只按 manifest 的 SHA-256 校验，不校验签名。
  校验失败一律拒绝安装，manifest 里的 `signed` 字段保留给后续签名策略。
- `manifest.json` 是自动更新的必需资产：`prepare-release.ps1` 生成、workflow 上传，
  字段为 `version` / `file` / `sizeBytes` / `sha256` / `builtAtUtc` / `signed`。
  删掉它或改字段名会让应用内更新失败。
- 便携版没有 NSIS 安装记录，`/UPDATE` 无法正确替换，因此明确不做应用内升级。
- 2026-11-19 实测：用 v0.9.6 发布资产从 0.9.5 执行 `/P /UPDATE /R`，安装后版本为 0.9.6，
  安装版被应用识别为可自动升级环境；v0.9.6 的 manifest / 安装包 / SHA256SUMS 已互相校验。
- 升级后若全按键支持服务仍跑着旧 helper，重启应用（或设置页「重启服务」）即可加载新文件。

## 后续（需要额外授权）

- 代码签名证书与签名流水线
- ARM64 包
- 更强的更新签名（Tauri updater 密钥 / Authenticode 校验）
