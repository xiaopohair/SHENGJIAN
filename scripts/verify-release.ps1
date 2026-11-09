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

$portableManifestPath = Join-Path $repoRoot "release\$Version\portable-manifest.json"
if (Test-Path -LiteralPath $portableManifestPath) {
    $portable = Get-Content -LiteralPath $portableManifestPath -Raw -Encoding utf8 | ConvertFrom-Json
    $portableFile = Join-Path $repoRoot ($portable.file -replace "/", "\")
    if (-not (Test-Path -LiteralPath $portableFile)) {
        throw "portable zip not found: $portableFile"
    }
    $portableItem = Get-Item -LiteralPath $portableFile
    $portableHash = (Get-FileHash -LiteralPath $portableFile -Algorithm SHA256).Hash
    if ($portableItem.Length -ne [int64]$portable.sizeBytes) {
        throw "portable size mismatch: expected $($portable.sizeBytes), got $($portableItem.Length)"
    }
    if ($portableHash -ne $portable.sha256) {
        throw "portable sha256 mismatch: expected $($portable.sha256), got $portableHash"
    }
    Write-Output "portable $Version verified: sha256=$portableHash size=$($portableItem.Length)"
}
