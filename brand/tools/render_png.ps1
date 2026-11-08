# Render every job in render_manifest.json to a 1600x1600 transparent page,
# then crop the top-left target rectangle with crop_pngs.py.
# A fixed large viewport avoids Chromium's small-window scaling quirks.
# ASCII-only (Windows PowerShell 5.1 reads UTF-8 without BOM as ANSI).

$brand = Split-Path -Parent $PSScriptRoot
$jobs = Get-Content (Join-Path $PSScriptRoot "render_manifest.json") -Raw | ConvertFrom-Json
$edge = "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
if (-not (Test-Path $edge)) { throw "Edge not found at $edge" }
$profile = Join-Path $PSScriptRoot ".edge-profile"
$work = Join-Path $PSScriptRoot ".render"
$stage = Join-Path $PSScriptRoot ".stage"
New-Item -ItemType Directory -Force -Path $work, $stage | Out-Null

$i = 0
foreach ($job in $jobs) {
  $i++
  $srcPath = Join-Path $brand ([string]$job.src)
  $srcUri = ([uri]$srcPath).AbsoluteUri
  $stagePath = Join-Path $stage ([string]$job.out).Replace('/', '\')
  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $stagePath) | Out-Null
  $htmlPath = Join-Path $work ("job-{0:00}.html" -f $i)
  $html = "<!doctype html><html><head><meta charset='utf-8'><style>html,body{margin:0;padding:0;background:transparent;overflow:hidden}img{display:block}</style></head><body><img src='$srcUri' style='width:$($job.w)px;height:$($job.h)px'></body></html>"
  Set-Content -Path $htmlPath -Value $html -Encoding ASCII
  $htmlUri = ([uri]$htmlPath).AbsoluteUri

  $edgeArgs = @(
    "--headless=new",
    "--disable-gpu",
    "--no-sandbox",
    "--disable-crash-reporter",
    "--hide-scrollbars",
    "--user-data-dir=$profile",
    "--force-device-scale-factor=1",
    "--window-size=1600,1600",
    "--default-background-color=00000000",
    "--screenshot=$stagePath",
    $htmlUri
  )
  $proc = Start-Process -FilePath $edge -ArgumentList $edgeArgs -Wait -PassThru -WindowStyle Hidden
  if (-not (Test-Path $stagePath)) { throw "Render failed (exit $($proc.ExitCode)): $($job.out)" }
  Write-Output ("[{0}/{1}] staged {2}" -f $i, $jobs.Count, $job.out)
}

$py = $env:PYTHON
if (-not $py) {
  $cmd = Get-Command python -ErrorAction SilentlyContinue
  if ($cmd) { $py = $cmd.Source }
}
if (-not $py) { throw "Python not found. Set PYTHON to your python.exe (Pillow required)." }
& $py (Join-Path $PSScriptRoot "crop_pngs.py")
Write-Output "done"

