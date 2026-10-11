param(
  [Parameter(Mandatory = $true)][string]$Source,
  [Parameter(Mandatory = $true)][string]$OutFile,
  [int]$Width,
  [int]$Height
)

# Resamples a PNG to an exact size and flattens it to 24-bit RGB.
#
# Two details matter here. Store art is uploaded with no alpha channel at all,
# because a partially transparent edge is a rendering hazard rather than
# something the Store can composite for you. And GDI+ bicubic scaling samples
# the destination canvas at the borders, so scaling an opaque source onto a
# freshly created 32bppARGB bitmap leaves the outermost pixels partly
# transparent even when every source pixel is fully opaque -- the 300x300 tile
# came out of a naive resize with corner alpha 220.
#
# Drawing onto an opaque white surface first gives the resampler something real
# to sample at the edges, and saving as Format24bppRgb removes the channel
# outright rather than merely filling it with 255.

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

if (-not (Test-Path $Source)) { throw "Source not found: $Source" }

$src = [System.Drawing.Bitmap]::FromFile((Resolve-Path $Source))
if (-not $Width)  { $Width  = $src.Width }
if (-not $Height) { $Height = $src.Height }

$dst = New-Object System.Drawing.Bitmap($Width, $Height, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
$g = [System.Drawing.Graphics]::FromImage($dst)
try {
  $g.InterpolationMode  = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $g.PixelOffsetMode    = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
  $g.SmoothingMode      = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
  $g.CompositingMode    = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
  # Opaque backing, so the resampler's edge taps land on real pixels.
  $g.Clear([System.Drawing.Color]::FromArgb(255, 255, 255, 255))
  $g.DrawImage($src, 0, 0, $Width, $Height)
}
finally { $g.Dispose() }

New-Item -ItemType Directory -Path (Split-Path $OutFile) -Force | Out-Null
$dst.Save($OutFile, [System.Drawing.Imaging.ImageFormat]::Png)

$check = [System.Drawing.Bitmap]::FromFile((Resolve-Path $OutFile))
$fmt = $check.PixelFormat
$corner = $check.GetPixel(0, 0)
Write-Host ("{0}  {1}x{2}  {3}  角落=({4},{5},{6})" -f $OutFile, $check.Width, $check.Height, $fmt, $corner.R, $corner.G, $corner.B)
$check.Dispose()

$dst.Dispose()
$src.Dispose()