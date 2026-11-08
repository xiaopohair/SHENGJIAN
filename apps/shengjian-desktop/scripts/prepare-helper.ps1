$ErrorActionPreference = "Stop"
$appRoot = Split-Path -Parent $PSScriptRoot
$repoRoot = Split-Path -Parent (Split-Path -Parent $appRoot)
$helperRoot = Join-Path $repoRoot "helper\rc003"
$sourceHelper = Join-Path $helperRoot "target\release\shengjian-helper.exe"
$sourceGadget = Join-Path $helperRoot "vendor\frida-gadget.dll"
$dest = Join-Path $appRoot "src-tauri\resources\rc003"
$licenseDest = Join-Path $appRoot "src-tauri\resources\licenses"

if (-not (Test-Path -LiteralPath $sourceHelper)) {
    throw "helper binary missing; run: cd helper/rc003 && cargo build --release"
}
if (-not (Test-Path -LiteralPath $sourceGadget)) {
    throw "frida gadget missing; run: cd helper/rc003 && python vendor/fetch_frida_gadget.py"
}

New-Item -ItemType Directory -Force -Path $dest | Out-Null
New-Item -ItemType Directory -Force -Path $licenseDest | Out-Null
Copy-Item -LiteralPath $sourceHelper -Destination (Join-Path $dest "shengjian-helper.exe") -Force
Copy-Item -LiteralPath $sourceGadget -Destination (Join-Path $dest "frida-gadget.dll") -Force

$licenseFiles = @{
    "LICENSE" = (Join-Path $repoRoot "LICENSE")
    "LICENSE-OFL.txt" = (Join-Path $repoRoot "brand\LICENSE-OFL.txt")
    "ATTRIBUTION.md" = (Join-Path $repoRoot "ATTRIBUTION.md")
    "THIRD_PARTY_NOTICES.md" = (Join-Path $repoRoot "THIRD_PARTY_NOTICES.md")
    "COPYING.frida-gadget" = (Join-Path $helperRoot "vendor\frida-gadget-COPYING")
}
foreach ($entry in $licenseFiles.GetEnumerator()) {
    if (-not (Test-Path -LiteralPath $entry.Value)) {
        throw "license file missing: $($entry.Value)"
    }
    Copy-Item -LiteralPath $entry.Value -Destination (Join-Path $licenseDest $entry.Key) -Force
}
Write-Output "prepared RC003 helper and license resources in $dest"
