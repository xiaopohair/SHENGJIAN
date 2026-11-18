<p align="center">
  <img src="brand/logo/shengjian-app-icon.svg" width="112" alt="SHENGJIAN">
</p>

<h1 align="center">SHENGJIAN · Voice Remote for Windows</h1>

<p align="center">
  <a href="README.md">简体中文</a> | <strong>English</strong>
</p>

<p align="center">
  <strong>Your voice is the shortcut.</strong>
</p>

<p align="center">
  <code>Voice input</code> · <code>Button mapping</code> · <code>Macros & profiles</code>
</p>

Repository: <https://github.com/xiaopohair/SHENGJIAN>

SHENGJIAN is a voice-input app for Windows that turns a Bluetooth voice remote into a Windows input device — speak to type, press to act. Voice input, custom hotkeys, macros, and button-mapping profiles are built in. It works with mainstream AI voice-input tools such as WeChat Input, Doubao Input, Vokie, and ChatterFly, and lets you dictate and run macros in agent apps such as ChatGPT, Claude, and DeepSeek Harness — a personalized voice workflow for vibe coding and everyday work.

Brand positioning, slogan, logo, and visual guidelines: [brand/BRAND.md](brand/BRAND.md).

## Current status

- Version: `0.9.5` (0.9.4 added the Google Chromecast TV Remote; 0.9.5 fixes two global voice regressions introduced in 0.9.4 — the helper agent swallowed every 3-byte I/O, and the WeChat IME was routed to the virtual-keyboard path)
- Supported devices: RC003 (Xiaomi Bluetooth Voice Remote 2 Pro), RC001 (Xiaomi Bluetooth Voice Remote 2), ARN9 (Xiaomi Bluetooth Voice Remote), **Google Chromecast TV Remote** (model `CHROMECAST`, 14 keys); Windows x64
- GitHub Release 0.9.5: <https://github.com/xiaopohair/SHENGJIAN/releases/tag/v0.9.5> (CI-built, unsigned); installer `shengjian_0.9.5_x64-setup.exe`, portable `shengjian_0.9.5_x64-portable.zip`; verify with the `SHA256SUMS` asset in the release
- Code signing: integrated with **SignPath Foundation** free open-source signing (built into the release workflow; enables automatically once configured); see [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md) and [docs/signing.md](docs/signing.md)
- Verified on physical devices: RC003 (connection, voice, button gestures, automatic profile switching, Run macro, enhanced Back-key capture); ARN9 (connection, ATVV voice link, navigation keys, volume/Back capture); RC001 (GATT identity, 12-key layout incl. TV, two live voice sessions, TV key opens the common-phrases panel); Chromecast (connection, all 14 keys mapped and injected, ATVV voice at 16 kHz, mute / volume / Enter, pairing guide and profile-page hotspots)
- Local release manifest: `release/<version>/manifest.json` (produced by a local build; the release asset is authoritative)
- Release notes: `docs/release-notes-0.9.5.md` (includes the 0.9.4 post-mortem)
- GitHub repository: <https://github.com/xiaopohair/SHENGJIAN>; releases are produced by the Release workflow triggered by `v*` tags
- No auto-updater

## Device support

| Model | Product | Keys | Key channel | Voice | Default button profile |
| --- | --- | --- | --- | --- | --- |
| `RC003` | Xiaomi Bluetooth Voice Remote 2 Pro | 12 (incl. TV / Menu) | Raw Input + enhanced capture | ATVV | Arrows / OK / Back / Volume ± |
| `RC001` | Xiaomi Bluetooth Voice Remote 2 | 12 (incl. TV / Menu) | Raw Input + enhanced capture | ATVV | same as above |
| `ARN9` | Xiaomi Bluetooth Voice Remote | 11 (no TV) | Raw Input + enhanced capture | ATVV | same as above |
| `CHROMECAST` | Google Chromecast TV Remote | 14 (incl. Mute / YouTube / Netflix / input source) | read by the helper inside the HOGP driver | ATVV | OK→Enter, Back→Esc, arrows, Home→Ctrl+T, Power→F11, Volume ±/Mute→system volume |

## Features

### Desktop host

`apps/shengjian-desktop`: Tauri 2 + Vue 3, consuming the core in-process.

- UI: frameless custom title bar (drag region + window controls, the brand appears once in the sidebar), rounded main content panel, centered on startup, `Ctrl+K` command palette; the top bar shows connection status, device name, battery, current input tool, and voice/audio status pills
- Startup splash: a branded splash screen covers initialization and hands off to the main window once settings and theme are ready, avoiding a blank flash; its copy follows the in-app language setting, falling back to the system language
- Connection page: device info card (custom nickname, product name · model, battery / connected time / firmware, Bluetooth / voice / full-key-support pills, accent gradient edge), scan/connect/disconnect and multi-device switching, remember last device; “Settings” opens the device settings drawer (device name, per-device audio toggle, gain, voice output device, voice input tool); “Forget device” disconnects and then unpairs it from Windows; the scan list marks each remote with a relative last-connected time (3 days ago / 1 minute ago) and remembers its model per device token; gain and voice tool show the *effective* value
- Tray: show/hide, open settings, open logs directory, quit; tooltip shows four status lines (button profile / remote / input tool / battery); Chinese/English menus switch instantly with the language; the tray uses the dark-plate app icon, which stays readable on both light and dark taskbars
- Voice page: voice/audio status, real-time level, voice test, voice input tool, custom voice hotkey, voice recordings toggle
- Buttons page: profile list, edit, activate; wide-screen workbench + table view with unsaved-navigation confirmation; import/export merged into the “More actions” menu (whole library as one file / single or multiple files, real per-file progress, import conflict preview)
- Recommended apps, button profiles, voice input tools, and recommended macros are driven by `config/*.json` (shipped with the installer; editable before a release or inside the install directory — see [docs/config-catalogs.md](docs/config-catalogs.md)); presets are read-only and become editable user profiles once copied
- Profile editor: app target (picker for preset / installed / UWP / browse, or use the current foreground), fill from current foreground, hotkey capture, automatic profile conflict hints, save validation, per-button hold repeat, profile-level voice tool override, duplicate profile; the key set follows the model (12 keys incl. TV and Menu for the three Xiaomi remotes, 14 keys incl. Mute / YouTube / Netflix / input source for Chromecast); common phrases (built in, edited on the Settings page, opened as a list on key press)
- Device test: the top-bar command palette opens “voice test / key test” at any time — read-only, never writes settings; voice text lands in an in-app native test pad (the IME keeps working and the main window does not reload), and the key tab lists the buttons expected for the connected model with detected/missing marks
- Top-bar listening switch: a “Listening / Paused” pill pauses or resumes button listening at any time (voice and Bluetooth stay connected); returning from Settings syncs the input tool and gain shown in the top bar to the effective values
- Reconnect: when disconnected, any key press on the remote reconnects the last remote; a user-initiated disconnect is respected, while an app restart still tries the last device
- Macros page: unified toolbar (“Try run / Save / Add / Properties / More actions” — duplicate, delete, and import/export moved into the more menu); a new macro is persisted as a draft right away; three-column editor (steps / current step / action parameters), drag reorder (pointer-following ghost + accent insertion line), step duplicate and timeout, shortcut capture, macro properties drawer, test/stop, per-step run results, single/multi-file import/export with per-item progress, Chinese validation messages
- History page: four tabs — voice events, button events, macro runs, and voice recordings; voice/button events are retained for the current run, while macro runs and voice recordings are persisted
- Settings page: global audio, theme, accent color, application language (Chinese/English/follow system), launch at login, hide to tray on startup, close behavior, configuration backup/restore (settings, profiles, macro library, voice recordings, appearance preferences, last device), log viewing/diagnostic bundle, health check, restore defaults, sectioned tabs; Data & diagnostics are still reachable if settings fail to load
- Accessibility: page landmarks/heading hierarchy, form labels, focus trapping for tabs/command palette/confirmation dialogs, decorative icons hidden; see `docs/ui-accessibility-report.md`
- About page: dark-plate app icon, app version, platform/build/signing, runtime information, local capabilities, supported remotes and platform status, components and licenses; can open the logs directory and re-run first-run setup; checks GitHub Releases in the background at startup (24-hour limit) and surfaces updates via a pinned update banner on the About page (version comparison + view release + ignore this version), a branded toast, a pulsing dot on the About nav item, and the tray menu, with optional pre-release filtering and no automatic download/install
- Pairing help: available from each supported model row in About and from the first-run scan step; covers four models — RC003 / RC001 / ARN9 / Google Chromecast TV Remote — with the key combination and Windows Bluetooth steps (Home + Menu for the three Xiaomi remotes, Back + Home for Chromecast, whose Bluetooth name is `Chromecast Remote`; ARN9 and Chromecast have no TV key, so the TV initialization note is hidden)
- Tray, first-run setup wizard (including voice input tool selection), single instance
- Profile/macro import and export, diagnostic log entry points

### Core / platform

- `shengjian-contract`: device identity, logical events, snapshots, actions, errors
- `shengjian-domain`: ATVV, ADPCM/PCM, voice pipeline, gestures, macros, settings, statistics, transcription
- `shengjian-app`: ports, `CoreEngine`, event stream, use cases
- `shengjian-windows`: BLE/GATT, WASAPI, Raw Input, SendInput, IME/TSF, UIA, focus, application launch, RC003 bridge
- `shengjian-windows-adapter`: translates the Windows platform into core ports
- `shengjian-testkit`: fake platform and integration tests
- `apps/shengjian-windows-host`: headless scan/run/migrate
- `helper/hid-host`: elevated helper in its own workspace (enhanced capture, key reading and forwarding, virtual keyboard, driver management; renamed from `rc003-helper` in 0.9.3)

## Project structure

```text
crates/                       # core, contract, domain, testkit, Windows platform and adapter
apps/shengjian-windows-host/
apps/shengjian-desktop/  # Tauri 2 + Vue 3 desktop host
  src-tauri/src/host.rs       # platform-agnostic HostPlatform abstraction
  src-tauri/src/platform/     # platform implementations (currently windows)
helper/hid-host/              # elevated helper: enhanced capture / key reading and forwarding / virtual keyboard (own workspace)
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

`pnpm build:installer` builds the elevated helper first, then runs `tauri build`, then normalizes the filename. Tauri first produces `声键_<version>_x64-setup.exe` from `productName`; `scripts/normalize-installer-name.ps1` then renames it to the ASCII lowercase release filename:

`apps/shengjian-desktop/src-tauri/target/release/bundle/nsis/shengjian_0.9.5_x64-setup.exe`

Building the helper first matters: the installer copies `helper/hid-host/target/release/shengjian-helper.exe`, so skipping that step silently ships a stale helper.

Running only `pnpm tauri build` also builds, but produces a Chinese filename; releases, CI, and download links consistently use `shengjian_<version>_x64-setup.exe`.

Installer contents:

- desktop app
- `resources/hid-host/shengjian-helper.exe`
- `resources/hid-host/frida-gadget.dll`
- `resources/licenses/`: GPL, ATTRIBUTION, third-party licenses, full Frida Gadget license, brand wordmark OFL notice (`LICENSE-OFL.txt`)

### Portable build (zip, no installer)

```powershell
cd apps/shengjian-desktop
pnpm build:portable
```

Output: `apps/shengjian-desktop/src-tauri/target/release/portable/shengjian_<version>_x64-portable.zip`; verification metadata is written to `release/<version>/portable-manifest.json`.

Extract the zip and run `shengjian-desktop.exe`; no installation is required. The system must have the WebView2 Runtime installed (included in Windows 11; present on most Windows 10 machines). Settings, macros, history, and logs still live in `%LOCALAPPDATA%\Shengjian`. The helper scheduled task points at the current folder, so reinstall it after moving the folder; before deleting the folder, run `resources\hid-host\shengjian-helper.exe --remove-task` as administrator. The portable and installed builds share the single-instance identifier and data directory, so they cannot run at the same time.

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

## Privileged helper (enhanced capture / key reading)

The helper does three things. The Back key and Volume ± keys of the Xiaomi remotes are dropped by Windows at the HID→VK mapping stage, so it takes them over (enhanced capture). The Chromecast remote's button reports never reach the OS Raw Input layer at all, so it reads them inside the HOGP driver process and forwards them. It also provides the virtual keyboard (WinUHid) and driver management.

First use (Xiaomi remotes):

1. Open **Settings → Audio & voice** and click install/update under “Full-key support service” (confirm the UAC prompt)
2. Enable the “Full-key support” toggle in the same group (it applies per PC, not per remote)
3. The helper scheduled task is started/stopped by the app; the “Full-key support · installed” pill on the connection card shows the service state

Chromecast only needs the **key reading** path and does not require the “full-key support” toggle; both capabilities share the same helper and the same resource directory.

Details: [docs/hid-host-setup.md](docs/hid-host-setup.md), [docs/chromecast-remote.md](docs/chromecast-remote.md).

## Verification

- Root workspace: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace` pass (**787 passed**); `cargo clippy --workspace --all-targets` reports 0 errors
- Desktop Rust: `cargo fmt -- --check`, `cargo test` pass (**24 passed**)
- Desktop frontend: `pnpm lint`, `pnpm format:check`, `pnpm exec vitest run` (**242 passed**), `pnpm build`, `pnpm config:check` pass
- Local build: the installer is produced from `productName` and normalized by `scripts/normalize-installer-name.ps1`; this release was built by CI (`shengjian_0.9.5_x64-setup.exe`), and the `prepare-release.ps1` / `verify-release.ps1` flow is unchanged
- Physical install / launch / uninstall: install directory, shortcuts, uninstall entry, and version cleanup are correct; user data `%LOCALAPPDATA%\Shengjian` is preserved
- Installer wizard: welcome sidebar, header brand image, Chinese UI, and the “声键 安装” title are correct
- Portable build: extracted to a path with spaces and Chinese characters and launched correctly; the splash window appeared and closed once the main window was ready
- Physical RC003:
  - scan → connect `ready`
  - voice `streaming → draining → idle`
  - OK `single / double / long`
  - automatic profile switching by foreground app
  - physical button Run macro (`run-macro`; Down key opens Notepad)
  - Back-key enhanced capture `back` edge
- Physical ARN9 (Xiaomi Bluetooth Voice Remote, no TV key):
  - the model is identified as ARN9 (no longer misdetected as RC003) and its profiles stay isolated from RC003
  - ATVV voice link: a 2.49 s session, 36,480 samples, peak −2.7 dBFS with no clipping, transcription stored
  - six navigation keys (up/down/left/right/Home/Menu) map correctly
  - Volume ± and Back go through enhanced capture (`0x80` / `0x81` / `0xF1`)
- Physical Google Chromecast TV Remote:
  - shows as Google Chromecast TV Remote · `CHROMECAST` after connecting (firmware 0.0.1, battery reads correctly)
  - all 14 keys produce semantic edges and are injected through the model's default profile (OK→Enter, Back→Esc, arrows, Home→Ctrl+T, Power→F11, Volume ±/Mute→system volume)
  - ATVV voice at a full 16 kHz (`voice_audio_rate result=full`)
  - pairing guide and the profile page's product image / hotspots line up correctly

## Local release

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/prepare-release.ps1 -Version 0.9.5
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-release.ps1 -Version 0.9.5
```

Both scripts read `productName` from `src-tauri/tauri.conf.json` and then assemble the installer path, so renaming the brand does not require script changes.
`prepare-release.ps1` writes `release/<version>/manifest.json` (`version` / `file` / `sizeBytes` / `sha256` /
`builtAtUtc` / `signed`); `verify-release.ps1` checks the local installer against it.

GitHub Release 0.9.5 (CI build):

```text
version=0.9.5
installer=shengjian_0.9.5_x64-setup.exe
portable=shengjian_0.9.5_x64-portable.zip
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
- Release notes body comes from `docs/release-notes-<version>.md` (`body_path`), so a tagged version needs that file
- Portable: after signing, `scripts/build-portable.ps1 -SkipBuild` assembles `shengjian_<version>_x64-portable.zip` and publishes it with the installer
- Signing docs: [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md), [docs/signing.md](docs/signing.md)
- Assets: installer, portable zip, `manifest.json`, `portable-manifest.json`, `SHA256SUMS`, `LICENSE`, `ATTRIBUTION.md`, `THIRD_PARTY_NOTICES.md`, `brand/LICENSE-OFL.txt`, full Frida license
- First helper installation requires one UAC approval
- SmartScreen still applies when signing is not enabled; Releases provide SHA-256 for verification
- Security: [SECURITY.md](SECURITY.md)
- Release process: [docs/release-process.md](docs/release-process.md)

The repository has been pushed to <https://github.com/xiaopohair/SHENGJIAN>; releases: <https://github.com/xiaopohair/SHENGJIAN/releases> (`v0.9.4` and `v0.9.5` are the latest published).

## Known limitations

- Windows x64 only; no ARM64 package
- Release packages are unsigned; CI signing requires the repository to be public and pass SignPath review
- No auto-update
- First helper installation requires UAC
- The common-phrases overlay only selects and inserts: phrases and their order are edited on the Settings page, not inside the panel; it is an in-app overlay that never takes focus, so it does not replace the system IME candidate window
- The Chromecast remote's keys depend on the privileged helper (its reports never reach the OS Raw Input layer); without the helper only voice works
- No second-client IPC; the desktop host is in-process

## Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org). See [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md) for details, and [PRIVACY.md](PRIVACY.md) for the privacy policy.

## License and attribution

The project is released under **GPL-3.0-only**; see [LICENSE](LICENSE).

See [ATTRIBUTION.md](ATTRIBUTION.md) for provenance, device profile references, and naming cleanup.
See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for third-party components and the Frida Gadget license.
