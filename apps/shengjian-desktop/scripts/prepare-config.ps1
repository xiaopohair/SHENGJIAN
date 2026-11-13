$ErrorActionPreference = "Stop"
$appRoot = Split-Path -Parent $PSScriptRoot
$repoRoot = Split-Path -Parent (Split-Path -Parent $appRoot)
$source = Join-Path $repoRoot "config"
$dest = Join-Path $appRoot "src-tauri\resources\config"

New-Item -ItemType Directory -Force -Path $dest | Out-Null
foreach ($name in @("app-presets.json", "voice-tools.json")) {
    $file = Join-Path $source $name
    if (-not (Test-Path -LiteralPath $file)) {
        throw "config source missing: $file"
    }
    Copy-Item -LiteralPath $file -Destination (Join-Path $dest $name) -Force
}
Write-Output "prepared config resources in $dest"
