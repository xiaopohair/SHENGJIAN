param(
    [string]$Version = "0.1.0"
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$manifestPath = Join-Path $repoRoot "release\$Version\manifest.json"
if (-not (Test-Path -LiteralPath $manifestPath)) {
    throw "manifest not found: $manifestPath (run scripts/prepare-release.ps1 first)"
}

$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json
$installer = Join-Path $repoRoot $manifest.file
if (-not (Test-Path -LiteralPath $installer)) {
    throw "installer not found: $installer"
}

$file = Get-Item -LiteralPath $installer
$hash = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash
if ($file.Length -ne [int64]$manifest.sizeBytes) {
    throw "size mismatch: expected $($manifest.sizeBytes), got $($file.Length)"
}
if ($hash -ne $manifest.sha256) {
    throw "sha256 mismatch: expected $($manifest.sha256), got $hash"
}
Write-Output "release $Version verified: sha256=$hash size=$($file.Length)"
