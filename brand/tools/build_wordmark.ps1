# Build the Shengjian wordmark outline from Maple Mono NF CN Bold (SIL OFL).
# Extracts the two CJK glyphs, joins them with 0.12em tracking, normalizes the
# ink box to 1000 units high, and writes brand/tools/wordmark-maple.json.
#
# ASCII-only on purpose: Windows PowerShell 5.1 reads UTF-8 without BOM as ANSI.
# Requires WPF (PresentationCore) and the font installed for the current user.

Add-Type -AssemblyName PresentationCore, WindowsBase, PresentationFramework

$fontCandidates = @(
  "file:///C:/Users/$env:USERNAME/AppData/Local/Microsoft/Windows/Fonts/MapleMono-NF-CN-Bold.ttf",
  "file:///C:/Windows/Fonts/MapleMono-NF-CN-Bold.ttf"
)
$fontUri = $fontCandidates | Where-Object { Test-Path ([uri]$_).LocalPath } | Select-Object -First 1
if (-not $fontUri) {
  throw "Maple Mono NF CN Bold not found. Install the font (SIL OFL) or edit `$fontCandidates in this script."
}

$ff = ([System.Windows.Media.Fonts]::GetFontFamilies($fontUri))[0]
$tf = New-Object System.Windows.Media.Typeface($ff, [System.Windows.FontStyles]::Normal, [System.Windows.FontWeights]::Bold, [System.Windows.FontStretches]::Normal)
$ci = [System.Globalization.CultureInfo]::GetCultureInfo("zh-CN")
$size = 1000
$ch1 = [string][char]0x58F0   # sheng
$ch2 = [string][char]0x952E   # jian

function Get-Glyph([string]$ch) {
  $ft = New-Object System.Windows.Media.FormattedText($ch, $ci, [System.Windows.FlowDirection]::LeftToRight, $tf, $size, [System.Windows.Media.Brushes]::Black, 1.0)
  return $ft.BuildGeometry([System.Windows.Point]::new(0, 0))
}

$g1 = Get-Glyph $ch1
$g2 = Get-Glyph $ch2
$gap = 120.0
$dx = $g1.Bounds.Right + $gap - $g2.Bounds.Left

$minX = [Math]::Min($g1.Bounds.Left, $g2.Bounds.Left + $dx)
$maxX = [Math]::Max($g1.Bounds.Right, $g2.Bounds.Right + $dx)
$minY = [Math]::Min($g1.Bounds.Top, $g2.Bounds.Top)
$maxY = [Math]::Max($g1.Bounds.Bottom, $g2.Bounds.Bottom)
$h = $maxY - $minY
$w = $maxX - $minX
$s = 1000.0 / $h

function Fmt([double]$v) { $v.ToString("0.##", [System.Globalization.CultureInfo]::InvariantCulture) }
function PX($p, $dxg) { return (($p.X + $dxg - $minX) * $s) }
function PY($p) { return (($p.Y - $minY) * $s) }

function Emit-Geometry($geo, $dxg, $sb) {
  if ($geo -is [System.Windows.Media.GeometryGroup]) {
    foreach ($c in $geo.Children) { Emit-Geometry $c $dxg $sb }
    return
  }
  if ($geo -is [System.Windows.Media.PathGeometry]) {
    foreach ($f in $geo.Figures) {
      [void]$sb.Append("M " + (Fmt (PX $f.StartPoint $dxg)) + " " + (Fmt (PY $f.StartPoint)) + " ")
      foreach ($seg in $f.Segments) {
        $t = $seg.GetType().Name
        if ($t -eq 'PolyLineSegment') {
          foreach ($p in $seg.Points) { [void]$sb.Append("L " + (Fmt (PX $p $dxg)) + " " + (Fmt (PY $p)) + " ") }
        }
        elseif ($t -eq 'LineSegment') {
          [void]$sb.Append("L " + (Fmt (PX $seg.Point $dxg)) + " " + (Fmt (PY $seg.Point)) + " ")
        }
        elseif ($t -eq 'QuadraticBezierSegment') {
          [void]$sb.Append("Q " + (Fmt (PX $seg.Point1 $dxg)) + " " + (Fmt (PY $seg.Point1)) + " " + (Fmt (PX $seg.Point2 $dxg)) + " " + (Fmt (PY $seg.Point2)) + " ")
        }
        elseif ($t -eq 'PolyQuadraticBezierSegment') {
          $pts = $seg.Points
          for ($i = 0; $i -lt $pts.Count; $i += 2) {
            [void]$sb.Append("Q " + (Fmt (PX $pts[$i] $dxg)) + " " + (Fmt (PY $pts[$i])) + " " + (Fmt (PX $pts[$i + 1] $dxg)) + " " + (Fmt (PY $pts[$i + 1])) + " ")
          }
        }
        elseif ($t -eq 'BezierSegment') {
          [void]$sb.Append("C " + (Fmt (PX $seg.Point1 $dxg)) + " " + (Fmt (PY $seg.Point1)) + " " + (Fmt (PX $seg.Point2 $dxg)) + " " + (Fmt (PY $seg.Point2)) + " " + (Fmt (PX $seg.Point3 $dxg)) + " " + (Fmt (PY $seg.Point3)) + " ")
        }
        elseif ($t -eq 'PolyBezierSegment') {
          $pts = $seg.Points
          for ($i = 0; $i -lt $pts.Count; $i += 3) {
            [void]$sb.Append("C " + (Fmt (PX $pts[$i] $dxg)) + " " + (Fmt (PY $pts[$i])) + " " + (Fmt (PX $pts[$i + 1] $dxg)) + " " + (Fmt (PY $pts[$i + 1])) + " " + (Fmt (PX $pts[$i + 2] $dxg)) + " " + (Fmt (PY $pts[$i + 2])) + " ")
          }
        }
        else { throw "Unsupported segment type: $t" }
      }
      if ($f.IsClosed) { [void]$sb.Append("Z ") }
    }
    return
  }
  throw "Unsupported geometry type: $($geo.GetType().FullName)"
}

$sb = New-Object System.Text.StringBuilder
Emit-Geometry $g1 0 $sb
Emit-Geometry $g2 $dx $sb

$out = [ordered]@{
  d = $sb.ToString()
  width = [Math]::Round($w * $s, 2)
  height = 1000
  gapEm = 0.12
  font = "Maple Mono NF CN Bold"
  license = "SIL OFL 1.1"
}
$outPath = Join-Path $PSScriptRoot "wordmark-maple.json"
$out | ConvertTo-Json -Compress | Set-Content -Path $outPath -Encoding UTF8
Write-Output ("Wrote " + $outPath + " width=" + $out.width + " height=1000")

