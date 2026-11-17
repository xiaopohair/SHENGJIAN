$ErrorActionPreference = "Stop"
$appRoot = Split-Path -Parent $PSScriptRoot
$repoRoot = Split-Path -Parent (Split-Path -Parent $appRoot)
$helperRoot = Join-Path $repoRoot "helper\rc003"
$sourceHelper = Join-Path $helperRoot "target\release\shengjian-helper.exe"
$sourceGadget = Join-Path $helperRoot "vendor\frida-gadget.dll"
$sourceWinUHid = Join-Path $helperRoot "vendor\winuhid.dll"
$dest = Join-Path $appRoot "src-tauri\resources\rc003"
$licenseDest = Join-Path $appRoot "src-tauri\resources\licenses"

if (-not (Test-Path -LiteralPath $sourceHelper)) {
    throw "helper binary missing; run: cd helper/rc003 && cargo build --release"
}
if (-not (Test-Path -LiteralPath $sourceGadget)) {
    throw "frida gadget missing; run: cd helper/rc003 && python vendor/fetch_frida_gadget.py"
}
if (-not (Test-Path -LiteralPath $sourceWinUHid)) {
    throw "WinUHid client missing: helper/rc003/vendor/winuhid.dll（虚拟键盘后端的客户端）"
}

New-Item -ItemType Directory -Force -Path $dest | Out-Null
New-Item -ItemType Directory -Force -Path $licenseDest | Out-Null
Copy-Item -LiteralPath $sourceHelper -Destination (Join-Path $dest "shengjian-helper.exe") -Force
Copy-Item -LiteralPath $sourceGadget -Destination (Join-Path $dest "frida-gadget.dll") -Force
# 必须与助手同目录：助手用 LoadLibraryW 按"可执行文件所在目录优先"加载。
Copy-Item -LiteralPath $sourceWinUHid -Destination (Join-Path $dest "WinUHid.dll") -Force
# 官方驱动包（含官方 CA 证书）与安装脚本：设置页的「驱动管理」直接调用它
$sourceDriverDir = Join-Path $helperRoot "vendor\winuhid-driver"
if (-not (Test-Path -LiteralPath $sourceDriverDir)) {
    throw "WinUHid driver package missing: helper/rc003/vendor/winuhid-driver"
}
$driverDest = Join-Path $dest "winuhid-driver"
if (Test-Path -LiteralPath $driverDest) { Remove-Item -LiteralPath $driverDest -Recurse -Force }
Copy-Item -LiteralPath $sourceDriverDir -Destination $driverDest -Recurse -Force

$licenseFiles = @{
    "LICENSE" = (Join-Path $repoRoot "LICENSE")
    "LICENSE-OFL.txt" = (Join-Path $repoRoot "brand\LICENSE-OFL.txt")
    "ATTRIBUTION.md" = (Join-Path $repoRoot "ATTRIBUTION.md")
    "THIRD_PARTY_NOTICES.md" = (Join-Path $repoRoot "THIRD_PARTY_NOTICES.md")
    "COPYING.frida-gadget" = (Join-Path $helperRoot "vendor\frida-gadget-COPYING")
    "COPYING.winuhid" = (Join-Path $helperRoot "vendor\winuhid-LICENSE")
}
foreach ($entry in $licenseFiles.GetEnumerator()) {
    if (-not (Test-Path -LiteralPath $entry.Value)) {
        throw "license file missing: $($entry.Value)"
    }
    Copy-Item -LiteralPath $entry.Value -Destination (Join-Path $licenseDest $entry.Key) -Force
}
Write-Output "prepared RC003 helper and license resources in $dest"
