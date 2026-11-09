param(
    [string]$Version,
    [string]$Arch = "x64",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$appRoot = Join-Path $repoRoot "apps\shengjian-desktop"
$srcTauri = Join-Path $appRoot "src-tauri"
$configPath = Join-Path $srcTauri "tauri.conf.json"
$tauri = Get-Content -LiteralPath $configPath -Raw -Encoding utf8 | ConvertFrom-Json
if (-not $tauri.productName) {
    throw "productName is missing in $configPath"
}
if (-not $Version) {
    $Version = [string]$tauri.version
    if (-not $Version) {
        throw "version is missing in $configPath; pass -Version explicitly"
    }
}

if (-not $SkipBuild) {
    Push-Location $appRoot
    try {
        & pnpm tauri build --no-bundle
        if ($LASTEXITCODE -ne 0) {
            throw "pnpm tauri build --no-bundle failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
}

$exe = Join-Path $srcTauri "target\release\shengjian-desktop.exe"
if (-not (Test-Path -LiteralPath $exe)) {
    throw "application executable not found: $exe (run without -SkipBuild first)"
}
$resources = Join-Path $srcTauri "resources"
$requiredResources = @(
    "rc003\shengjian-helper.exe",
    "rc003\frida-gadget.dll",
    "licenses\LICENSE",
    "licenses\LICENSE-OFL.txt",
    "licenses\ATTRIBUTION.md",
    "licenses\THIRD_PARTY_NOTICES.md",
    "licenses\COPYING.frida-gadget"
)
foreach ($relative in $requiredResources) {
    if (-not (Test-Path -LiteralPath (Join-Path $resources $relative))) {
        throw "portable resource missing: resources\$relative (run 'pnpm prepare:helper' first)"
    }
}

$portableRoot = Join-Path $srcTauri "target\release\portable"
$stageName = "shengjian_{0}_{1}" -f $Version, $Arch
$stage = Join-Path $portableRoot $stageName
if (Test-Path -LiteralPath $stage) {
    Remove-Item -LiteralPath $stage -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $stage | Out-Null

Copy-Item -LiteralPath $exe -Destination (Join-Path $stage "shengjian-desktop.exe")
Copy-Item -LiteralPath $resources -Destination $stage -Recurse
foreach ($notice in @("LICENSE", "ATTRIBUTION.md", "THIRD_PARTY_NOTICES.md")) {
    $source = Join-Path $repoRoot $notice
    if (-not (Test-Path -LiteralPath $source)) {
        throw "notice file missing: $source"
    }
    Copy-Item -LiteralPath $source -Destination $stage
}

$readmeTemplatePath = Join-Path $PSScriptRoot "portable-readme.txt"
if (-not (Test-Path -LiteralPath $readmeTemplatePath)) {
    throw "portable readme template missing: $readmeTemplatePath"
}
$readmeTemplate = Get-Content -LiteralPath $readmeTemplatePath -Raw -Encoding utf8
$portableReadme = $readmeTemplate.Replace("{{VERSION}}", $Version)
$readmePath = Join-Path $stage "README.txt"
[IO.File]::WriteAllText($readmePath, $portableReadme + "`r`n", (New-Object System.Text.UTF8Encoding($false)))

$zipName = "shengjian_{0}_{1}-portable.zip" -f $Version, $Arch
$zipPath = Join-Path $portableRoot $zipName
if (Test-Path -LiteralPath $zipPath) {
    Remove-Item -LiteralPath $zipPath -Force
}
Compress-Archive -LiteralPath $stage -DestinationPath $zipPath -CompressionLevel Optimal

$zipItem = Get-Item -LiteralPath $zipPath
$hash = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash
$signature = Get-AuthenticodeSignature -LiteralPath $exe
$relativeZip = "apps/shengjian-desktop/src-tauri/target/release/portable/$zipName"

$releaseDir = Join-Path $repoRoot "release\$Version"
New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null
$manifest = [ordered]@{
    version = $Version
    arch = $Arch
    kind = "portable-zip"
    file = $relativeZip
    sizeBytes = $zipItem.Length
    sha256 = $hash
    builtAtUtc = $zipItem.LastWriteTimeUtc.ToString("o")
    webview2 = "system-evergreen"
    signed = ($signature.Status -eq "Valid")
}
$manifestPath = Join-Path $releaseDir "portable-manifest.json"
$manifestJson = $manifest | ConvertTo-Json -Depth 5
[IO.File]::WriteAllText($manifestPath, $manifestJson + "`n", (New-Object System.Text.UTF8Encoding($false)))

Write-Output "portable zip: $relativeZip"
Write-Output "portable manifest: release/$Version/portable-manifest.json"
Write-Output "sha256=$hash size=$($zipItem.Length)"
