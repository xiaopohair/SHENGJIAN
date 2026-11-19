# Code signing policy

[中文说明](docs/signing.md)

## Policy statement

Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org).

This project is open source under GPL-3.0-only (see [LICENSE](LICENSE)). The source code and the build
scripts that produce the signed artifacts live in [xiaopohair/SHENGJIAN](https://github.com/xiaopohair/SHENGJIAN).
The user-facing product name is **声键 (SHENGJIAN)**; crate names use the technical prefix `shengjian`.

## Signed artifacts

Every release is built on GitHub-hosted runners by
[`.github/workflows/release.yml`](.github/workflows/release.yml) from the tagged commit.
The signing requests are created by the same workflow:

| Artifact | Path |
| --- | --- |
| Desktop application | `apps/shengjian-desktop/src-tauri/target/release/shengjian-desktop.exe` |
| NSIS installer | `apps/shengjian-desktop/src-tauri/target/release/bundle/nsis/声键_<version>_x64-setup.exe` |

The installer file name is derived from `productName` in
`apps/shengjian-desktop/src-tauri/tauri.conf.json` at workflow runtime, so it follows the
product name without hardcoding it in the release scripts.

## Team roles

- **Authors** (may modify the source without additional review): the repository owner.
- **Reviewers** (review changes from non-committers): the repository owner.
- **Approvers** (approve every signing request): the repository owner.

> Committers and reviewers: [@xiaopohair](https://github.com/xiaopohair);
> Approvers: [@xiaopohair](https://github.com/xiaopohair).

## Privacy

This program will not transfer any information to other networked systems unless specifically
requested by the user or the person installing or operating it, except for automatic update checks
and installer downloads from GitHub Releases.

- The application communicates with the Bluetooth remote paired by the user.
- Automatic updates make HTTPS requests to `api.github.com` and `github.com` to check for a new
  release and download its installer. These requests contain no account, device identifier, voice,
  or transcript data. Automatic checks/downloads can be disabled under Settings → Software updates;
  manual checks and opening external links still use the network.
- No telemetry, analytics, or crash reporting is built in.
- Settings, button profiles, macros, history, and downloaded update packages are stored locally under
  `%LOCALAPPDATA%\Shengjian`.
- Configuration import/export and diagnostic bundles only run when the user explicitly triggers them.

## System changes

- The installer installs the application per user under `%LOCALAPPDATA%\声键` and does not
  require administrator rights.
- The optional full-key support helper (`hid-host`) is installed only after the user explicitly
  clicks “Install helper” in the app, and only that action triggers the UAC prompt and creates the
  `Shengjian HID Host` scheduled task.
- The uninstaller removes the application directory; user data under `%LOCALAPPDATA%\Shengjian` is
  kept unless the user removes it, and the helper scheduled task is removed during uninstall.

## Update verification and signing

- In-app updates verify the installer against the `sizeBytes` / `sha256` in the release
  `manifest.json`; they do not currently verify the Authenticode signature of the installer.
- CI signing (when enabled) still signs the application executable and the NSIS installer, so manual
  downloads and Windows trust see the signature. The updater's trust anchor is the HTTPS download
  plus the release manifest SHA-256.
- A stronger update-signing scheme (Authenticode verification or a separate updater signing key) is
  listed as future work in [docs/release-process.md](docs/release-process.md).

## Signing request rules

- Every signing request is approved manually by an approver listed above.
- Only binaries built from this repository by the release workflow are submitted for signing.
- Third-party components that ship inside the installer (the Frida gadget and WinUHid client used by
  the `hid-host` helper) are open-source components included unmodified from their upstream releases;
  they are not signed with this project's certificate.
