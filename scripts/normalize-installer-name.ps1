# Normalize the NSIS installer file name to ASCII lowercase.
#
# Tauri names the installer "<productName>_<version>_<arch>-setup.exe" and
# productName is the user-facing brand name ("声键"), which puts CJK characters
# in the download URL / CI artifact path. Keep the in-product name, but ship the
# file as "shengjian_<version>_<arch>-setup.exe".
#
# Usage:
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/normalize-installer-name.ps1 -Version 0.1.0
# Prints the repository-relative path of the normalized installer (forward slashes).

param(
    [string]$Version,
    [string]$Arch = "x64"
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$configPath = Join-Path $repoRoot "apps\shengjian-desktop\src-tauri\tauri.conf.json"
$tauri = Get-Content -LiteralPath $configPath -Raw -Encoding utf8 | ConvertFrom-Json
if (-not $tauri.productName) {
    throw "productName is missing in $configPath"
}
if (-not $Version) {
    $Version = $tauri.version
    if (-not $Version) {
        throw "version is missing in $configPath; pass -Version explicitly"
    }
}

$bundleDir = Join-Path $repoRoot "apps\shengjian-desktop\src-tauri\target\release\bundle\nsis"
$built = Join-Path $bundleDir ("{0}_{1}_{2}-setup.exe" -f $tauri.productName, $Version, $Arch)
$target = Join-Path $bundleDir ("shengjian_{0}_{1}-setup.exe" -f $Version, $Arch)

if (Test-Path -LiteralPath $built) {
    Move-Item -LiteralPath $built -Destination $target -Force
} elseif (-not (Test-Path -LiteralPath $target)) {
    throw "installer not found: $built (run 'pnpm tauri build' first)"
}

$relative = "apps/shengjian-desktop/src-tauri/target/release/bundle/nsis/$(Split-Path -Leaf $target)"
Write-Output $relative
