<picture>
  <source media="(prefers-color-scheme: dark)" srcset="brand/logo/shengjian-lockup-horizontal-dark.svg">
  <img src="brand/logo/shengjian-lockup-horizontal-light.svg" height="56" alt="SHENGJIAN">
</picture>

# SHENGJIAN (声键) · Voice Remote for Windows

[简体中文](README.md) | **English**

**Your voice is the shortcut.** · **声音，就是快捷键。**

`Voice input` · `Button mapping` · `Macros & profiles`

Repository: <https://github.com/xiaopohair/SHENGJIAN>

SHENGJIAN is a voice-input app for Windows that turns a Bluetooth voice remote into a Windows input device — speak to type, press to act. Voice input, custom hotkeys, macros, and button-mapping profiles are built in. It works with mainstream AI voice-input tools such as WeChat Input, Doubao Input, Vokie, and ChatterFly, and lets you dictate and run macros in agent apps such as ChatGPT, Claude, and DeepSeek Harness — a personalized voice workflow for vibe coding and everyday work.

Brand positioning, slogan, logo, and visual guidelines: [brand/BRAND.md](brand/BRAND.md).

## Current status

- Version: `0.4.1` (buttons/connection/macros page fixes, larger minimum window height, responsive breakpoint cleanup)
- Supported devices: RC003 (Xiaomi Bluetooth Voice Remote 2 Pro) and RC001 (Xiaomi Bluetooth Voice Remote 2); Windows x64
- GitHub Release 0.4.1: <https://github.com/xiaopohair/SHENGJIAN/releases/tag/v0.4.1> (CI-built, unsigned); installer `shengjian_0.4.1_x64-setup.exe`; verify with the `SHA256SUMS` asset in the release
- Code signing: integrated with **SignPath Foundation** free open-source signing (built into the release workflow; enables automatically once configured); see [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md) and [docs/signing.md](docs/signing.md)
- Verified on a physical RC003: connection, voice, button gestures, automatic profile switching, Run macro (`run-macro`), and enhanced Back-key capture
- Local release manifest: `release/0.4.1/manifest.json` (records the local build; the release asset is authoritative)
- Release notes: `docs/release-notes-0.4.1.md`
- GitHub repository: <https://github.com/xiaopohair/SHENGJIAN>; releases are produced by the Release workflow triggered by `v*` tags
- No auto-updater

## Features

### Desktop host

`apps/shengjian-desktop`: Tauri 2 + Vue 3, consuming the core in-process.

- UI: frameless custom title bar (brand mark and product name on the left), rounded main content panel, centered on startup, `Ctrl+K` command palette
- Startup splash: a branded splash screen covers initialization and hands off to the main window once settings and theme are ready, avoiding a blank flash; its copy follows the in-app language setting, falling back to the system language
- Connection page: scan, connect, disconnect, multi-device switching, remember last device, status, model, firmware version, battery, connection details, current-device audio/voice tool overrides, RC003 enhanced capture and helper
- Tray: show/hide, open settings, open logs directory, quit; tooltip shows connection/battery; Chinese/English menus switch instantly with the language; tray icons use the light pine-green / ink variants according to taskbar light/dark
- Voice page: voice/audio status, real-time level, voice test, voice input tool, custom voice hotkey, transcription history toggle
- Buttons page: profile list, edit, activate, import/export all profiles; workbench and table views with unsaved-navigation confirmation
- Profile editor: app target (choose a program / use current foreground), fill from current foreground, hotkey capture, automatic profile conflict hints, save validation, per-button hold repeat, profile-level voice tool override, duplicate profile, 12 keys (including TV and Menu); common-phrases entry shown according to platform capability
- Macros page: create/edit/delete, 8 action types, three-column editor (steps / current step / parameters), drag reorder, step duplicate, step timeout, shortcut capture and clearing, macro properties drawer (name and description), test/stop, per-step run results with failure reasons, import/export, Chinese validation messages
- History page: four tabs — voice events, button events, macro runs, and transcription records; voice/button events are retained for the current run, while macro runs and transcriptions are persisted
- Settings page: global audio, theme, accent color, application language (Chinese/English/follow system), launch at login, close behavior, configuration backup/restore (settings, profiles, macro library, transcription history, appearance preferences, last device), log viewing/diagnostic bundle, health check, restore defaults, sectioned tabs; Data & diagnostics are still reachable if settings fail to load
- Accessibility: page landmarks/heading hierarchy, form labels, focus trapping for tabs/command palette/confirmation dialogs, decorative icons hidden; see `docs/ui-accessibility-report.md`
- About page: brand mark (light pine-green / deep pine-green depending on theme), app version, platform/build/signing, runtime information, local capabilities, supported remotes and platform status, components and licenses; can open the logs directory and re-run first-run setup; checks GitHub Releases in the background at startup (24-hour limit) and surfaces updates via a toast, an About nav dot, and the tray menu, with optional pre-release filtering and no automatic download/install
- Pairing help: available from each supported model row in About and from the first-run scan step; shows the RC003 / RC001 key combination, Windows Bluetooth steps, and the new-remote TV initialization note
- Tray, first-run setup wizard (including voice input tool selection), single instance
- Profile/macro import and export (the export action covers all profiles), diagnostic log entry points

### Core / platform

- `shengjian-contract`: device identity, logical events, snapshots, actions, errors
- `shengjian-domain`: ATVV, ADPCM/PCM, voice pipeline, gestures, macros, settings, statistics, transcription
- `shengjian-app`: ports, `CoreEngine`, event stream, use cases
- `shengjian-windows`: BLE/GATT, WASAPI, Raw Input, SendInput, IME/TSF, UIA, focus, application launch, RC003 bridge
- `shengjian-windows-adapter`: translates the Windows platform into core ports
- `shengjian-testkit`: fake platform and integration tests
- `apps/shengjian-windows-host`: headless scan/run/migrate
- `helper/rc003`: elevated enhanced-capture helper in its own workspace

## Project structure

```text
crates/                       # core, contract, domain, testkit, Windows platform and adapter
apps/shengjian-windows-host/
apps/shengjian-desktop/  # Tauri 2 + Vue 3 desktop host
  src-tauri/src/host.rs       # platform-agnostic HostPlatform abstraction
  src-tauri/src/platform/     # platform implementations (currently windows)
helper/rc003/                 # RC003 Back/Volume-key enhanced-capture helper (own workspace)
profiles/                     # DeviceProfile schema and device facts
Testing/hardware-scripts/     # hardware signal script fixtures
contracts/ipc/                # Windows runtime IPC contract fixtures
brand/                        # SHENGJIAN brand guidelines, logos, app icons, and social previews
docs/                         # architecture, roadmap, release process
release/                      # local release manifests and checksums
scripts/                      # release manifest/verification scripts
```

Multi-platform structure and plans for future platforms: [docs/multi-platform.md](docs/multi-platform.md). The project currently stays a monorepo and is not split into multiple Git repositories.

Dependency direction:

```text
shengjian-contract
  ← shengjian-domain
  ← shengjian-app
  ← shengjian-testkit

shengjian-windows
  → shengjian-app ports
  ← shengjian-windows-adapter
```

## Build and run

### Desktop development

```powershell
cd apps/shengjian-desktop
pnpm install
pnpm tauri dev
```

### Desktop tests

```powershell
cd apps/shengjian-desktop
pnpm test
pnpm build

cd src-tauri
cargo check
cargo test
```

### Desktop installer

```powershell
cd apps/shengjian-desktop
pnpm build:installer
```

`pnpm build:installer` = `tauri build` + filename normalization. Tauri first produces `声键_<version>_x64-setup.exe` from `productName`; then `scripts/normalize-installer-name.ps1` renames it to the ASCII lowercase release filename:

`apps/shengjian-desktop/src-tauri/target/release/bundle/nsis/shengjian_0.4.1_x64-setup.exe`

Running only `pnpm tauri build` also builds, but produces a Chinese filename; releases, CI, and download links consistently use `shengjian_<version>_x64-setup.exe`.

Installer contents:

- desktop app
- `resources/rc003/shengjian-helper.exe`
- `resources/rc003/frida-gadget.dll`
- `resources/licenses/`: GPL, ATTRIBUTION, third-party licenses, full Frida Gadget license, brand wordmark OFL notice (`LICENSE-OFL.txt`)

### Portable build (zip, no installer)

```powershell
cd apps/shengjian-desktop
pnpm build:portable
```

Output: `apps/shengjian-desktop/src-tauri/target/release/portable/shengjian_<version>_x64-portable.zip`; verification metadata is written to `release/<version>/portable-manifest.json`.

Extract the zip and run `shengjian-desktop.exe`; no installation is required. The system must have the WebView2 Runtime installed (included in Windows 11; present on most Windows 10 machines). Settings, macros, history, and logs still live in `%LOCALAPPDATA%\Shengjian`. The RC003 helper scheduled task points at the current folder, so reinstall it after moving the folder; before deleting the folder, run `resources\rc003\shengjian-helper.exe --remove-task` as administrator. The portable and installed builds share the single-instance identifier and data directory, so they cannot run at the same time.

### Headless host

```powershell
cargo run -p shengjian-windows-host -- scan
cargo run -p shengjian-windows-host -- run --seconds 20
```

### Core tests

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
```

## RC003 enhanced-capture helper

The Back key and Volume ± keys are dropped by Windows at the HID→VK mapping stage and must be taken over by an elevated helper.

First use:

1. On the Connection page, check "Enable RC003 enhanced capture (Back/Volume)"
2. Click "Install/update helper" and confirm the UAC prompt
3. Save settings
4. The helper scheduled task is started/stopped by the app

Details: [docs/rc003-helper-setup.md](docs/rc003-helper-setup.md).

## Verification

- Root workspace: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace` pass (**694 passed**, including 493 in the `shengjian-windows` platform library)
- Desktop Rust: **14 passed**
- Desktop frontend: `pnpm lint`, `pnpm format:check`, `pnpm test` (**61 passed**), `pnpm build` pass
- Local build: `shengjian_0.4.1_x64-setup.exe` generated; `prepare-release.ps1` / `verify-release.ps1` pass
- Physical install / launch / uninstall: install directory, shortcuts, uninstall entry, and version cleanup are correct; user data `%LOCALAPPDATA%\Shengjian` is preserved
- Installer wizard: welcome sidebar, header brand image, Chinese UI, and the "声键 安装" title are correct
- Portable build: extracted to a path with spaces and Chinese characters and launched correctly; the splash window appeared and closed once the main window was ready
- Physical RC003:
  - scan → connect `ready`
  - voice `streaming → draining → idle`
  - OK `single / double / long`
  - automatic profile switching by foreground app
  - physical button Run macro (`run-macro`; Down key opens Notepad)
  - Back-key enhanced capture `back` edge

## Local release

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/prepare-release.ps1 -Version 0.4.1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-release.ps1 -Version 0.4.1
```

Both scripts read `productName` from `src-tauri/tauri.conf.json` and then assemble the installer path, so renaming the brand does not require script changes.
`prepare-release.ps1` writes `release/<version>/manifest.json` (`version` / `file` / `sizeBytes` / `sha256` /
`builtAtUtc` / `signed`); `verify-release.ps1` checks the local installer against it.

GitHub Release 0.4.1 (CI build):

```text
version=0.4.1
installer=shengjian_0.4.1_x64-setup.exe
portable=shengjian_0.4.1_x64-portable.zip
sha256/size: see the SHA256SUMS and manifest.json assets in the release
signed=false
```

Local and CI builds are not byte-identical, so sizes and hashes differ. Release notes therefore never
hardcode checksums; they point at the `SHA256SUMS` asset instead. The `release/<version>/manifest.json`
in the repository records only the local build. See [docs/release-process.md](docs/release-process.md).

## GitHub release preparation

Route: **public repository, x64, manual download, no auto-update**; free open-source signing via SignPath Foundation.

- `.github/workflows/release.yml`: builds and releases on `v*` tags; signs automatically after SignPath variables/secrets are configured
- Release title is "声键 &lt;version&gt;"; Tauri first produces `声键_<version>_x64-setup.exe` per `productName`, and the workflow then calls `scripts/normalize-installer-name.ps1` to normalize it to the release filename `shengjian_<version>_x64-setup.exe`
- Portable: after signing, `scripts/build-portable.ps1 -SkipBuild` assembles `shengjian_<version>_x64-portable.zip` and publishes it with the installer
- Signing docs: [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md), [docs/signing.md](docs/signing.md)
- Assets: installer, portable zip, `manifest.json`, `portable-manifest.json`, `SHA256SUMS`, `LICENSE`, `ATTRIBUTION.md`, `THIRD_PARTY_NOTICES.md`, `brand/LICENSE-OFL.txt`, full Frida license
- First helper installation requires one UAC approval
- SmartScreen still applies when signing is not enabled; Releases provide SHA-256 for verification
- Security: [SECURITY.md](SECURITY.md)
- Release process: [docs/release-process.md](docs/release-process.md)

The repository has been pushed to <https://github.com/xiaopohair/SHENGJIAN>; releases are generated by the tag-triggered `.github/workflows/release.yml`. `v0.4.1` is prepared and awaiting release (the latest published release is `v0.4.0`).

## Known limitations

- Windows x64 only; no ARM64 package
- 0.4.1 is an unsigned release package; CI signing requires the repository to be public and pass SignPath review
- No auto-update
- First helper installation requires UAC
- RC001/ARN9 identity mapping is still deferred; see [docs/device-identity.md](docs/device-identity.md)
- No second-client IPC; the desktop host is in-process

## Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org). See [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md) for details, and [PRIVACY.md](PRIVACY.md) for the privacy policy.

## License and attribution

The project is released under **GPL-3.0-only**; see [LICENSE](LICENSE).

See [ATTRIBUTION.md](ATTRIBUTION.md) for provenance, device profile references, and naming cleanup.
See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for third-party components and the Frida Gadget license.
