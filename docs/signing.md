# Windows 代码签名（SignPath）

本项目使用 **SignPath Foundation** 的免费开源代码签名：证书由 SignPath Foundation 持有，
构建与签名请求来自 GitHub Actions，发布前需要人工审批。

- 仓库：<https://github.com/xiaopohair/SHENGJIAN>
- 面向 SignPath 审核的公开政策：[CODE_SIGNING_POLICY.md](../CODE_SIGNING_POLICY.md)
- 签名用的 artifact 配置：[.signpath/artifact-configuration.xml](../.signpath/artifact-configuration.xml)
- 发布工作流：[.github/workflows/release.yml](../.github/workflows/release.yml)

## 为什么是两段签名

SignPath 支持 `msi`、`zip`、`nuget` 等复合格式的深度签名，但 **不支持 NSIS 安装包内部文件的深度签名**。
而 Tauri 在 `tauri bundle` 阶段会给 app exe 写入 bundle type。因此发布流程是：

```text
tauri build --no-bundle          # 编译 exe（未打补丁）
tauri bundle                     # 写入 bundle type，产出安装包 A（丢弃）
SignPath: 签 app exe             # 第 1 个签名请求
tauri bundle                     # 用已签名的 exe 重新打包出安装包 B
SignPath: 签安装包 B             # 第 2 个签名请求
verify + prepare release
```

已验证（自签名证书实验）：对已签名的 exe 再跑 `tauri bundle`，exe 的文件哈希与
Authenticode 签名都保持不变，因此两段式是安全的。

## 前置条件

1. 仓库公开（SignPath Foundation 需要可验证的开源项目；本项目 GPL-3.0-only，符合 OSI 许可）。
2. 项目已经有至少一个 Release。
3. 在 [SignPath Foundation](https://signpath.org/apply) 提交申请；通过后在 SignPath 组织里：
   - 在组织设置中添加 **GitHub.com** 作为 trusted build system，并关联到本项目；
   - 安装 [SignPath GitHub App](https://github.com/apps/signpath) 并授权目标仓库；
   - 项目成员开启 MFA（SignPath 与 GitHub 都要）。

## SignPath 侧配置

在 SignPath 控制台创建/确认以下对象，slug 要填到 GitHub 仓库变量里：

| SignPath 对象 | 建议 slug | 说明 |
| --- | --- | --- |
| Project | `shengjian` | 关联本仓库与 GitHub trusted build system |
| Signing policy | `release` | 只允许从 `v*` tag 的 release 构建发起签名；每次请求人工审批 |
| Artifact configuration | `windows-exe` | 内容粘贴 `.signpath/artifact-configuration.xml` |

Artifact configuration 会校验 `ProductVersion`、`FileVersion`，并给 PE 文件做 Authenticode 签名；
每次请求通过参数 `version` 传入版本号。

安装包文件名：Tauri 先按 `productName` 产出 `声键_<version>_x64-setup.exe`，release workflow 随后调用
`scripts/normalize-installer-name.ps1` 归一化为 ASCII 小写 `shengjian_<version>_x64-setup.exe` 再提交签名与上传。

> **注意（品牌改名后的 ProductName）**：PE 版本资源里的 `ProductName` 来自 `tauri.conf.json` 的
> `productName`，当前是中文「声键」。`.signpath/artifact-configuration.xml` 不再约束 `product-name`
> （只按 `path="*.exe"` 匹配），避免 ASCII 约束与中文 PE 元数据冲突。如果 SignPath 项目要求 metadata
> 完全匹配，请在控制台把 product-name 填成与 PE 完全一致的 `声键`。

## GitHub 侧配置

在仓库 **Settings → Secrets and variables → Actions** 中添加：

| 名称 | 类型 | 值 |
| --- | --- | --- |
| `SIGNPATH_API_TOKEN` | secret | SignPath 用户 API token（需要该项目的提交权限） |
| `SIGNPATH_ORGANIZATION_ID` | variable | SignPath 组织 ID |
| `SIGNPATH_PROJECT_SLUG` | variable | `shengjian` |
| `SIGNPATH_SIGNING_POLICY_SLUG` | variable | `release` |
| `SIGNPATH_ARTIFACT_CONFIGURATION_SLUG` | variable | `windows-exe` |

**兼容性**：没有配置 `SIGNPATH_API_TOKEN` 时，release workflow 会跳过所有签名步骤，照常产出未签名安装包。
配置后自动切换为签名流程，不需要改 YAML。

## 发布时的行为

推送 `v*` tag 后：

1. 构建 exe（`tauri build --no-bundle`）。
2. 打一次未签名安装包，让 Tauri 把 bundle type 写进 exe。
3. 上传 exe → 提交第 1 个签名请求 → 在 SignPath 里人工审批 → 下载签名后的 exe 并替换。
4. 重新 `tauri bundle`，得到包含签名 exe 的安装包。
5. 上传安装包 → 提交第 2 个签名请求 → 人工审批 → 替换为签名后的安装包。
6. `Get-AuthenticodeSignature` 校验两个文件必须为 `Valid`，然后生成 `manifest.json` / `SHA256SUMS` 并创建 Release。

任何一个签名请求被拒绝或超时，工作流会失败，不会产出“半签名”的 Release。

## 本地验证

```powershell
Get-AuthenticodeSignature "apps/shengjian-desktop/src-tauri/target/release/shengjian-desktop.exe" |
  Format-List Status, StatusMessage, SignerCertificate

Get-AuthenticodeSignature "apps/shengjian-desktop/src-tauri/target/release/bundle/nsis/shengjian_0.2.0_x64-setup.exe" |
  Format-List Status, StatusMessage, SignerCertificate
```

签名后 `Status` 应为 `Valid`，`SignerCertificate.Subject` 应包含 `SignPath Foundation`。

未签名构建的 `Status` 是 `NotSigned`，属于预期。

## 说明与预期

- **SmartScreen 不是签完立刻消失**：从 2024 起微软取消了 EV 证书的即时信誉，OV/EV 都要按发布累积；
  SignPath 签发的是 OV 级别证书，前几个版本仍可能提示，持续用同一身份签名会逐步建立信誉。
- 签名不改变安装包内容与行为，只增加签名块；`SHA256SUMS` 在签名完成后生成。
- 不要把 API token 写进仓库或日志；token 只放在 GitHub secret 中，且只对需要的项目授权。
- 首次申请需要说明 RC003 helper 与随包分发的 Frida gadget（开源上游组件，用于按键增强捕获，
  不是漏洞利用工具）；公开政策里已经写明系统改动与隐私边界。
