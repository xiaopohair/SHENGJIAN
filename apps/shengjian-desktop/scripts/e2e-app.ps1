param(
  [int]$Port = 9223
)

$ErrorActionPreference = "Stop"

# Product name and version live in the Tauri config; read them so the script
# follows a rebrand instead of hardcoding an install directory.
$configPath = Join-Path $PSScriptRoot "..\src-tauri\tauri.conf.json"
$config = Get-Content -LiteralPath $configPath -Raw -Encoding UTF8 | ConvertFrom-Json
$productName = $config.productName
$version = $config.version

$exe = Join-Path $env:LOCALAPPDATA "$productName\shengjian-desktop.exe"
if (-not (Test-Path $exe)) {
  throw "$productName $version is not installed at $exe. Run the $version installer before E2E."
}

# The deterministic E2E run owns the desktop process. Stop any manually opened instance
# so WebView2 starts with the CDP port enabled.
Get-Process shengjian-desktop -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1

$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port"
$process = Start-Process -FilePath $exe -PassThru

try {
  $deadline = (Get-Date).AddSeconds(45)
  $ready = $false
  while ((Get-Date) -lt $deadline) {
    try {
      Invoke-RestMethod "http://127.0.0.1:$Port/json/version" -TimeoutSec 2 | Out-Null
      $ready = $true
      break
    } catch {
      Start-Sleep -Milliseconds 500
    }
  }
  if (-not $ready) {
    throw "$productName started but WebView2 CDP port $Port is not available."
  }

  # Keep the Playwright webServer process alive while the desktop app is running.
  while (-not $process.HasExited) {
    Start-Sleep -Seconds 1
  }
} finally {
  if (-not $process.HasExited) {
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
  }
}
