//! In-app automatic updates.
//!
//! The update **check** lives in the frontend (`src/lib/update.ts`), because it only
//! needs the GitHub Releases API and the app's own version. This module owns the part
//! that must not happen in the webview:
//!
//! 1. fetch the release `manifest.json` (version / file / size / SHA-256),
//! 2. stream the NSIS installer to `%LOCALAPPDATA%\Shengjian\Updates`,
//! 3. verify size + SHA-256 against the manifest,
//! 4. hand off to the installer with `/P /UPDATE /R` and exit.
//!
//! The installer (Tauri's NSIS template) understands `/UPDATE`: it skips the
//! uninstall/reinstall choice, closes the running app through the Restart Manager,
//! replaces the files, and `/R` starts the new version again. This is the same
//! hand-off the official Tauri updater plugin performs; only the transport is ours, so
//! the existing release assets (`manifest.json`, `SHA256SUMS`, NSIS installer) keep
//! working without a second signing key.
//!
//! Trust model: the manifest and the installer are fetched from the same GitHub
//! Release over HTTPS. The SHA-256 protects against corrupted/truncated downloads,
//! not against a compromised GitHub release. `manifest.json.signed` is recorded for
//! diagnostics but is **not** independently re-verified here (the current CI builds
//! are unsigned); the module never installs bytes that do not match the manifest.

use crate::commands::CommandError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Release assets live under `<repo>/releases/download/<tag>/<asset>`.
const RELEASE_DOWNLOAD_BASE: &str = "https://github.com/xiaopohair/SHENGJIAN/releases/download";
const INSTALLER_PREFIX: &str = "shengjian_";
const INSTALLER_SUFFIX: &str = "_x64-setup.exe";
const USER_AGENT: &str = concat!("shengjian-desktop/", env!("CARGO_PKG_VERSION"));
const PROGRESS_EVENT: &str = "update-download-progress";
const PROGRESS_INTERVAL: Duration = Duration::from_millis(120);
/// Sanity bound: the current installer is ~14 MB. Refuse a manifest that would make
/// the app fill the disk if the release metadata is ever wrong.
const MAX_INSTALLER_BYTES: u64 = 512 * 1024 * 1024;

/// Latest successful download. `install_update` only accepts a path this module
/// produced and verified, never a path supplied by the webview.
static DOWNLOADED: Mutex<Option<DownloadedUpdate>> = Mutex::new(None);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSupport {
    /// Whether this build may replace itself through the NSIS installer.
    pub supported: bool,
    /// `installed` | `portable` | `development`.
    pub kind: String,
    /// Optional machine-readable detail for diagnostics.
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadedUpdate {
    pub tag: String,
    pub version: String,
    pub path: String,
    pub size_bytes: u64,
    pub sha256: String,
    /// Whether the release manifest reports a valid Authenticode signature. The
    /// current CI builds are unsigned (`false`); kept for diagnostics and for a
    /// future signature policy.
    pub signed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDownloadProgress {
    pub tag: String,
    pub downloaded: u64,
    pub total: u64,
}

/// The slim, machine-readable manifest uploaded with every release by
/// `scripts/prepare-release.ps1` (see `docs/release-process.md`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseManifest {
    version: String,
    /// Repository-relative build path in the local manifest; only the file name is
    /// meaningful for the uploaded asset (`.../shengjian_<version>_x64-setup.exe`).
    file: String,
    size_bytes: u64,
    sha256: String,
    #[serde(default)]
    signed: bool,
}

impl ReleaseManifest {
    fn file_name(&self) -> Result<&str, CommandError> {
        Path::new(&self.file)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                CommandError::new(
                    "update-manifest",
                    "release manifest has no installer file",
                    false,
                )
            })
    }

    /// Reject a manifest that does not describe the tag we asked for. This turns a
    /// wrong/partial release into a clear error instead of downloading something
    /// that may not match the version the UI offered.
    fn validate(&self, tag: &str) -> Result<(), CommandError> {
        let expected = tag.trim_start_matches('v');
        if self.version.trim_start_matches('v') != expected {
            return Err(CommandError::new(
                "update-manifest",
                &format!(
                    "release manifest version {} does not match tag {tag}",
                    self.version
                ),
                false,
            ));
        }
        let file_name = self.file_name()?;
        let expected_file = format!("{INSTALLER_PREFIX}{expected}{INSTALLER_SUFFIX}");
        if file_name != expected_file {
            return Err(CommandError::new(
                "update-manifest",
                &format!("release manifest points at unexpected installer {file_name}"),
                false,
            ));
        }
        if self.size_bytes == 0 {
            return Err(CommandError::new(
                "update-manifest",
                "release manifest has an empty installer",
                false,
            ));
        }
        if self.size_bytes > MAX_INSTALLER_BYTES {
            return Err(CommandError::new(
                "update-manifest",
                "release manifest installer is unreasonably large",
                false,
            ));
        }
        if self.sha256.len() != 64 || !self.sha256.chars().all(|value| value.is_ascii_hexdigit()) {
            return Err(CommandError::new(
                "update-manifest",
                "release manifest has an invalid SHA-256",
                false,
            ));
        }
        Ok(())
    }
}

/// `update_support` command: can this build update itself?
#[tauri::command]
pub fn update_support(app: AppHandle) -> UpdateSupport {
    current_update_support(&app)
}

fn current_update_support(app: &AppHandle) -> UpdateSupport {
    let product = app
        .config()
        .product_name
        .clone()
        .unwrap_or_else(|| "声键".to_owned());
    let install_root = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|base| base.join(&product));
    let exe = std::env::current_exe().ok();
    classify_update_support(exe.as_deref(), install_root.as_deref())
}

/// Pure classifier (unit-testable): installed builds live in
/// `%LOCALAPPDATA%\\<productName>`; portable builds unpack into
/// `shengjian_<version>_<arch>`; everything else is a development build.
fn classify_update_support(exe: Option<&Path>, install_root: Option<&Path>) -> UpdateSupport {
    let Some(exe) = exe else {
        return unsupported("development", "cannot resolve current executable");
    };
    let parent = exe.parent();
    if let Some(parent) = parent {
        let default_root = install_root.is_some_and(|root| paths_equal(parent, root));
        // NSIS can install to a custom directory; the uninstaller next to the exe is
        // the reliable marker that `/UPDATE` has an installation to update.
        if default_root || parent.join("uninstall.exe").is_file() {
            return UpdateSupport {
                supported: true,
                kind: "installed".to_owned(),
                reason: None,
            };
        }
    }
    let portable = parent
        .and_then(|path| path.file_name())
        .map(|name| {
            let name = name.to_string_lossy().to_ascii_lowercase();
            name.starts_with("shengjian_") && !name.starts_with("shengjian-desktop")
        })
        .unwrap_or(false);
    if portable {
        unsupported("portable", "portable builds update manually")
    } else {
        unsupported("development", "development builds update manually")
    }
}

fn unsupported(kind: &str, reason: &str) -> UpdateSupport {
    UpdateSupport {
        supported: false,
        kind: kind.to_owned(),
        reason: Some(reason.to_owned()),
    }
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    let normalize = |path: &Path| -> String {
        path.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_ascii_lowercase()
    };
    let canonical_left = left.canonicalize().ok();
    let canonical_right = right.canonicalize().ok();
    match (canonical_left, canonical_right) {
        (Some(left), Some(right)) => normalize(&left) == normalize(&right),
        _ => normalize(left) == normalize(right),
    }
}

fn updates_dir() -> Result<PathBuf, CommandError> {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    Ok(base.join("Shengjian").join("Updates"))
}

fn valid_tag(tag: &str) -> bool {
    if tag.is_empty() || tag.len() > 80 || tag.contains("..") {
        return false;
    }
    let version = tag.strip_prefix('v').unwrap_or(tag);
    !version.is_empty()
        && version
            .chars()
            .next()
            .is_some_and(|value| value.is_ascii_digit())
        && version
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, '.' | '-' | '+'))
}

fn manifest_url(tag: &str) -> String {
    format!("{RELEASE_DOWNLOAD_BASE}/{tag}/manifest.json")
}

fn installer_url(tag: &str, file_name: &str) -> String {
    format!("{RELEASE_DOWNLOAD_BASE}/{tag}/{file_name}")
}

fn http_agent(timeout: Duration) -> ureq::Agent {
    // ureq defaults to Rustls even when only `native-tls` is compiled in, so the
    // provider must be selected explicitly. Native TLS uses Windows SChannel,
    // which respects the machine's certificate store and proxy settings.
    let tls = ureq::tls::TlsConfig::builder()
        .provider(ureq::tls::TlsProvider::NativeTls)
        .build();
    ureq::Agent::config_builder()
        .user_agent(USER_AGENT)
        .https_only(true)
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_global(Some(timeout))
        .max_redirects(5)
        .tls_config(tls)
        .build()
        .new_agent()
}

fn fetch_manifest(tag: &str) -> Result<ReleaseManifest, CommandError> {
    let url = manifest_url(tag);
    let mut response = http_agent(Duration::from_secs(45))
        .get(&url)
        .call()
        .map_err(|error| {
            CommandError::new(
                "update-manifest",
                &format!("cannot fetch release manifest: {error}"),
                true,
            )
        })?;
    let manifest = response
        .body_mut()
        .read_json::<ReleaseManifest>()
        .map_err(|error| {
            CommandError::new(
                "update-manifest",
                &format!("release manifest is not valid JSON: {error}"),
                false,
            )
        })?;
    manifest.validate(tag)?;
    Ok(manifest)
}

/// `download_update` command.
#[tauri::command]
pub async fn download_update(
    app: AppHandle,
    tag: String,
) -> Result<DownloadedUpdate, CommandError> {
    let support = current_update_support(&app);
    if !support.supported {
        return Err(CommandError::new(
            "update-unsupported",
            "this build cannot update itself",
            false,
        ));
    }
    let app_for_task = app.clone();
    let tag_for_task = tag.clone();
    let joined = tauri::async_runtime::spawn_blocking(move || {
        download_blocking(&app_for_task, &tag_for_task)
    })
    .await;
    let downloaded = match joined {
        Ok(Ok(downloaded)) => downloaded,
        Ok(Err(error)) => {
            crate::platform::note(format!(
                "update download failed tag={tag} code={} message={}",
                error.code, error.message
            ));
            return Err(error);
        }
        Err(error) => {
            let error = CommandError::new(
                "update-download",
                &format!("update download task failed: {error}"),
                true,
            );
            crate::platform::note(format!("update download task failed tag={tag}: {error:?}"));
            return Err(error);
        }
    };
    crate::platform::note(format!(
        "update download ready tag={tag} version={} bytes={}",
        downloaded.version, downloaded.size_bytes
    ));
    if let Ok(mut slot) = DOWNLOADED.lock() {
        *slot = Some(downloaded.clone());
    }
    Ok(downloaded)
}

fn download_blocking(app: &AppHandle, tag: &str) -> Result<DownloadedUpdate, CommandError> {
    if !valid_tag(tag) {
        return Err(CommandError::new(
            "update-manifest",
            "invalid release tag",
            false,
        ));
    }
    let manifest = fetch_manifest(tag)?;
    let file_name = manifest.file_name()?.to_owned();
    let directory = updates_dir()?;
    fs::create_dir_all(&directory).map_err(|error| {
        CommandError::new(
            "update-download",
            &format!("cannot create update directory: {error}"),
            true,
        )
    })?;
    let target = directory.join(&file_name);

    // A previous run may already have downloaded this exact installer. Re-check the
    // hash instead of trusting the file name, then reuse it.
    if target.is_file() && verify_file(&target, &manifest).is_ok() {
        emit_progress(Some(app), tag, manifest.size_bytes, manifest.size_bytes);
        return Ok(DownloadedUpdate {
            tag: tag.to_owned(),
            version: manifest.version.clone(),
            path: target.to_string_lossy().into_owned(),
            size_bytes: manifest.size_bytes,
            sha256: manifest.sha256.to_ascii_lowercase(),
            signed: manifest.signed,
        });
    }

    let partial = directory.join(format!("{file_name}.part"));
    let url = installer_url(tag, &file_name);
    let result = download_to(Some(app), tag, &url, &manifest, &partial);
    if result.is_err() {
        let _ = fs::remove_file(&partial);
    }
    result?;
    // Windows `rename` replaces an existing file, but remove the stale installer
    // explicitly so a failed replacement cannot leave the old bytes behind.
    let _ = fs::remove_file(&target);
    fs::rename(&partial, &target).map_err(|error| {
        let _ = fs::remove_file(&partial);
        CommandError::new(
            "update-download",
            &format!("cannot store downloaded installer: {error}"),
            true,
        )
    })?;
    // Drop installers from older versions so the update directory stays small.
    prune_old_installers(&directory, &file_name);
    Ok(DownloadedUpdate {
        tag: tag.to_owned(),
        version: manifest.version.clone(),
        path: target.to_string_lossy().into_owned(),
        size_bytes: manifest.size_bytes,
        sha256: manifest.sha256.to_ascii_lowercase(),
        signed: manifest.signed,
    })
}

fn download_to(
    app: Option<&AppHandle>,
    tag: &str,
    url: &str,
    manifest: &ReleaseManifest,
    partial: &Path,
) -> Result<(), CommandError> {
    let mut response = http_agent(Duration::from_secs(15 * 60))
        .get(url)
        .call()
        .map_err(|error| {
            CommandError::new(
                "update-download",
                &format!("cannot download installer: {error}"),
                true,
            )
        })?;
    let mut reader = response
        .body_mut()
        .with_config()
        .limit(manifest.size_bytes.saturating_add(1))
        .reader();
    let mut file = File::create(partial).map_err(|error| {
        CommandError::new(
            "update-download",
            &format!("cannot create installer file: {error}"),
            true,
        )
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    let mut downloaded = 0u64;
    let mut last_emit = Instant::now() - PROGRESS_INTERVAL;
    loop {
        let read = reader.read(&mut buffer).map_err(|error| {
            CommandError::new(
                "update-download",
                &format!("installer download interrupted: {error}"),
                true,
            )
        })?;
        if read == 0 {
            break;
        }
        downloaded = downloaded.saturating_add(read as u64);
        if downloaded > manifest.size_bytes {
            return Err(CommandError::new(
                "update-verify",
                "downloaded installer is larger than the release manifest says",
                false,
            ));
        }
        file.write_all(&buffer[..read]).map_err(|error| {
            CommandError::new(
                "update-download",
                &format!("cannot write installer: {error}"),
                true,
            )
        })?;
        hasher.update(&buffer[..read]);
        if last_emit.elapsed() >= PROGRESS_INTERVAL {
            emit_progress(app, tag, downloaded, manifest.size_bytes);
            last_emit = Instant::now();
        }
    }
    file.sync_all().map_err(|error| {
        CommandError::new(
            "update-download",
            &format!("cannot flush installer: {error}"),
            true,
        )
    })?;
    if downloaded != manifest.size_bytes {
        return Err(CommandError::new(
            "update-verify",
            &format!(
                "downloaded installer is {downloaded} bytes, expected {}",
                manifest.size_bytes
            ),
            true,
        ));
    }
    let actual = to_hex(&hasher.finalize());
    if !actual.eq_ignore_ascii_case(&manifest.sha256) {
        return Err(CommandError::new(
            "update-verify",
            "installer SHA-256 does not match the release manifest",
            false,
        ));
    }
    emit_progress(app, tag, downloaded, manifest.size_bytes);
    Ok(())
}

fn emit_progress(app: Option<&AppHandle>, tag: &str, downloaded: u64, total: u64) {
    let Some(app) = app else {
        return;
    };
    let _ = app.emit(
        PROGRESS_EVENT,
        UpdateDownloadProgress {
            tag: tag.to_owned(),
            downloaded,
            total,
        },
    );
}

fn verify_file(path: &Path, manifest: &ReleaseManifest) -> Result<(), CommandError> {
    let metadata = fs::metadata(path).map_err(|error| {
        CommandError::new(
            "update-verify",
            &format!("cannot stat installer: {error}"),
            true,
        )
    })?;
    if metadata.len() != manifest.size_bytes {
        return Err(CommandError::new(
            "update-verify",
            "existing installer size does not match the release manifest",
            false,
        ));
    }
    let actual = sha256_file(path)?;
    if !actual.eq_ignore_ascii_case(&manifest.sha256) {
        return Err(CommandError::new(
            "update-verify",
            "existing installer SHA-256 does not match the release manifest",
            false,
        ));
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, CommandError> {
    let mut file = File::open(path).map_err(|error| {
        CommandError::new(
            "update-verify",
            &format!("cannot open installer: {error}"),
            true,
        )
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            CommandError::new(
                "update-verify",
                &format!("cannot read installer: {error}"),
                true,
            )
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(to_hex(&hasher.finalize()))
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(value, "{byte:02x}");
    }
    value
}

fn prune_old_installers(directory: &Path, keep: &str) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == keep || !name.starts_with(INSTALLER_PREFIX) {
            continue;
        }
        if name.ends_with(INSTALLER_SUFFIX) || name.ends_with(&format!("{INSTALLER_SUFFIX}.part")) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// `install_update` command: verify the pending installer again, hand it to NSIS in
/// passive update mode, and exit so `/UPDATE` can replace the running files. `/R`
/// makes the installer start the new version when it finishes.
#[tauri::command]
pub fn install_update(app: AppHandle) -> Result<(), CommandError> {
    let support = current_update_support(&app);
    if !support.supported {
        return Err(CommandError::new(
            "update-unsupported",
            "this build cannot update itself",
            false,
        ));
    }
    let pending = DOWNLOADED
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
        .ok_or_else(|| {
            CommandError::new("update-not-ready", "no downloaded update is ready", false)
        })?;
    let path = PathBuf::from(&pending.path);
    let directory = updates_dir()?;
    let in_updates_dir = path
        .parent()
        .map(|parent| paths_equal(parent, &directory))
        .unwrap_or(false);
    if !in_updates_dir || !path.is_file() {
        return Err(CommandError::new(
            "update-not-ready",
            "downloaded installer is missing",
            false,
        ));
    }
    let metadata = fs::metadata(&path).map_err(|error| {
        CommandError::new(
            "update-install",
            &format!("cannot stat installer: {error}"),
            true,
        )
    })?;
    if metadata.len() != pending.size_bytes {
        return Err(CommandError::new(
            "update-verify",
            "installer changed after download completed",
            false,
        ));
    }
    let actual = sha256_file(&path)?;
    if !actual.eq_ignore_ascii_case(&pending.sha256) {
        return Err(CommandError::new(
            "update-verify",
            "installer changed after download completed",
            false,
        ));
    }

    let mut command = std::process::Command::new(&path);
    command.args(["/P", "/UPDATE", "/R"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    crate::platform::note(format!(
        "update install start tag={} path={}",
        pending.tag, pending.path
    ));
    command.spawn().map_err(|error| {
        let error = CommandError::new(
            "update-install",
            &format!("cannot start installer: {error}"),
            true,
        );
        crate::platform::note(format!(
            "update install failed tag={} message={}",
            pending.tag, error.message
        ));
        error
    })?;
    // The installer takes over from here: `/UPDATE` makes it close this process
    // through the Restart Manager, swap the files, and `/R` relaunches the app.
    app.exit(0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_release_and_prerelease_tags_only() {
        for tag in ["v0.10.0", "v0.10.0-beta.1", "0.10.0", "v1.0.0+build.2"] {
            assert!(valid_tag(tag), "{tag} should be accepted");
        }
        for tag in ["", "v", "../v0.1.0", "v0.1.0/../../x", "v 0.1.0", "release"] {
            assert!(!valid_tag(tag), "{tag} should be rejected");
        }
    }

    #[test]
    fn manifest_requires_matching_tag_and_installer_name() {
        let manifest = ReleaseManifest {
            version: "0.10.0".to_owned(),
            file: "apps/shengjian-desktop/src-tauri/target/release/bundle/nsis/shengjian_0.10.0_x64-setup.exe".to_owned(),
            size_bytes: 1024,
            sha256: "a".repeat(64),
            signed: false,
        };
        assert!(manifest.validate("v0.10.0").is_ok());
        assert!(manifest.validate("v0.10.1").is_err());

        let wrong = ReleaseManifest {
            file: "C:/tmp/evil.exe".to_owned(),
            ..manifest
        };
        assert!(wrong.validate("v0.10.0").is_err());
    }

    #[test]
    fn sha256_hex_matches_known_vector() {
        assert_eq!(
            to_hex(&Sha256::digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn support_classifier_distinguishes_install_layouts() {
        let installed = classify_update_support(
            Some(Path::new(
                r"C:\Users\me\AppData\Local\声键\shengjian-desktop.exe",
            )),
            Some(Path::new(r"C:\Users\me\AppData\Local\声键")),
        );
        assert!(installed.supported);
        assert_eq!(installed.kind, "installed");

        let portable = classify_update_support(
            Some(Path::new(
                r"D:\apps\shengjian_0.10.0_x64\shengjian-desktop.exe",
            )),
            Some(Path::new(r"C:\Users\me\AppData\Local\声键")),
        );
        assert!(!portable.supported);
        assert_eq!(portable.kind, "portable");

        let development = classify_update_support(
            Some(Path::new(
                r"D:\repo\apps\shengjian-desktop\src-tauri\target\debug\shengjian-desktop.exe",
            )),
            Some(Path::new(r"C:\Users\me\AppData\Local\声键")),
        );
        assert!(!development.supported);
        assert_eq!(development.kind, "development");

        // 自定义安装目录：靠 exe 旁边的 uninstall.exe 认出来（NSIS 才有）。
        let custom = std::env::temp_dir().join(format!(
            "shengjian-custom-install-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&custom).expect("temp install dir");
        File::create(custom.join("uninstall.exe")).expect("uninstaller marker");
        let custom_support = classify_update_support(
            Some(&custom.join("shengjian-desktop.exe")),
            Some(Path::new(r"C:\Users\me\AppData\Local\声键")),
        );
        let _ = fs::remove_dir_all(&custom);
        assert!(custom_support.supported);
        assert_eq!(custom_support.kind, "installed");
    }

    /// 真实网络的烟测：`cargo test -- --ignored fetch_manifest_from_github`。
    /// 校验 GitHub Release 的资产地址、跳转、TLS 与 manifest 字段名都和代码一致。
    #[test]
    #[ignore = "hits the GitHub Releases API"]
    fn fetch_manifest_from_github() {
        let manifest = fetch_manifest("v0.9.5").expect("release manifest should be reachable");
        assert_eq!(manifest.version, "0.9.5");
        assert!(manifest.validate("v0.9.5").is_ok());
        assert!(!manifest.signed);
    }

    /// 真实网络的端到端烟测（会下载约 13 MB）：
    /// `cargo test -- --ignored download_and_verify_release_installer`。
    #[test]
    #[ignore = "downloads a release installer"]
    fn download_and_verify_release_installer() {
        let manifest = fetch_manifest("v0.9.5").expect("release manifest should be reachable");
        let file_name = manifest.file_name().expect("installer name");
        let directory =
            std::env::temp_dir().join(format!("shengjian-update-test-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("temp directory");
        let partial = directory.join(format!("{file_name}.part"));
        let url = installer_url("v0.9.5", file_name);
        download_to(None, "v0.9.5", &url, &manifest, &partial).expect("download should succeed");
        let actual = sha256_file(&partial).expect("hash should read");
        assert!(actual.eq_ignore_ascii_case(&manifest.sha256));
        let _ = fs::remove_dir_all(&directory);
    }
}
