param(
    [string]$Version = "0.1.0"
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot

# The installer is produced as "<productName>_<version>_<arch>-setup.exe" and
# normalized to ASCII "shengjian_<version>_<arch>-setup.exe" so download URLs and
# CI artifact paths stay ASCII. Both steps derive the product name from
# tauri.conf.json, so a brand rename never requires editing this script.
$normalized = & (Join-Path $PSScriptRoot "normalize-installer-name.ps1") -Version $Version
if (-not $normalized) {
    throw "installer not found for version $Version (run 'pnpm tauri build' first)"
}
$installerRelative = ($normalized | Select-Object -Last 1).Trim()
$installer = Join-Path $repoRoot ($installerRelative -replace "/", "\")
if (-not (Test-Path -LiteralPath $installer)) {
    throw "installer not found: $installer"
}

$file = Get-Item -LiteralPath $installer
$hash = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash
$signature = Get-AuthenticodeSignature -LiteralPath $installer
$releaseDir = Join-Path $repoRoot "release\$Version"
New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null

$manifest = [ordered]@{
    version = $Version
    file = $installerRelative
    sizeBytes = $file.Length
    sha256 = $hash
    builtAtUtc = $file.LastWriteTimeUtc.ToString("o")
    signed = ($signature.Status -eq "Valid")
}
$manifestPath = Join-Path $releaseDir "manifest.json"
$manifestJson = $manifest | ConvertTo-Json -Depth 5
[IO.File]::WriteAllText($manifestPath, $manifestJson + "`n", (New-Object System.Text.UTF8Encoding($false)))
Write-Output "wrote $manifestPath"
Write-Output "sha256=$hash size=$($file.Length)"
