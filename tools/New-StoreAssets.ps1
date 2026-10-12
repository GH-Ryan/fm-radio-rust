param(
  [Parameter(Mandatory = $true)][string]$Source,
  [Parameter(Mandatory = $true)][string]$OutDir
)

# Generates the icon set the MSIX manifest and the Store require.
#
# Everything is resampled from a 1024x1024 render of the vector source rather
# than scaled up from the largest PNG in the repo. That PNG is 256x256, and the
# Store asks for 310x310 and 620x300, so scaling up would be visibly soft at
# exactly the sizes that matter most.
#
# The vector mark has rounded corners drawn with a transparent background. That
# is right for a browser favicon and wrong here: Windows applies its own mask
# to taskbar and Start icons, so handing it an already-rounded image with
# transparent corners produces a dark corner where the transparency meets the
# taskbar background. The rounded area is therefore filled before scaling.
#
# The fill has to come from the mark itself. Filling with a flat dark colour
# instead leaves a dark frame around a coloured icon, which is the same artefact
# by another route -- so the corner colour is sampled from the nearest opaque
# pixel of the same row, and each is smoothed against its neighbours to follow
# the gradient rather than producing visible bands.
#
# Comments are deliberately ASCII: Windows PowerShell 5.1 reads a .ps1 file as
# ANSI unless it carries a BOM.
#
# Usage:
#   powershell -File tools/New-StoreAssets.ps1 -Source src-tauri\icons\icon-1024.png -OutDir assets

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

if (-not (Test-Path $Source)) { throw "Source not found: $Source" }
New-Item -ItemType Directory -Path $OutDir -Force | Out-Null

function Set-Quality($g) {
  $g.InterpolationMode  = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $g.SmoothingMode      = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
  $g.PixelOffsetMode    = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
  return $g
}

function New-Bitmap([int]$w, [int]$h) {
  $bmp = New-Object System.Drawing.Bitmap($w, $h, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $bmp.SetResolution(96, 96)
  return $bmp
}

# Builds a fully opaque master: the mark over its own gradient extended to the
# corners.
function Get-OpaqueMaster([string]$path) {
  $src = New-Object System.Drawing.Bitmap($path)
  if ($src.Width -ne $src.Height) { throw "Source must be square, got $($src.Width)x$($src.Height)" }

  $size = $src.Width
  $bmp = New-Bitmap $size $size

  # Sample the gradient along each edge, walking inwards until a pixel is
  # actually opaque. That finds where the mark's own background starts rather
  # than assuming a radius.
  $top = New-Object 'System.Drawing.Color[]' $size
  $bottom = New-Object 'System.Drawing.Color[]' $size
  $left = New-Object 'System.Drawing.Color[]' $size
  $right = New-Object 'System.Drawing.Color[]' $size

  for ($x = 0; $x -lt $size; $x++) {
    for ($y = 0; $y -lt $size; $y++) {
      if ($src.GetPixel($x, $y).A -ge 250) { $top[$x] = $src.GetPixel($x, $y); break }
    }
    for ($y = $size - 1; $y -ge 0; $y--) {
      if ($src.GetPixel($x, $y).A -ge 250) { $bottom[$x] = $src.GetPixel($x, $y); break }
    }
  }

  for ($y = 0; $y -lt $size; $y++) {
    for ($x = 0; $x -lt $size; $x++) {
      if ($src.GetPixel($x, $y).A -ge 250) { $left[$y] = $src.GetPixel($x, $y); break }
    }
    for ($x = $size - 1; $x -ge 0; $x--) {
      if ($src.GetPixel($x, $y).A -ge 250) { $right[$y] = $src.GetPixel($x, $y); break }
    }
  }

  for ($y = 0; $y -lt $size; $y++) {
    for ($x = 0; $x -lt $size; $x++) {
      $px = $src.GetPixel($x, $y)
      if ($px.A -ge 250) { continue }

      # Nearest opaque pixel on each axis, so the fill continues the gradient
      # that pixel would have had rather than the one the corner happens to
      # sit next to.
      $c = [System.Drawing.Color]::FromArgb(
        255,
        [int](($top[$x].R + $left[$y].R + $top[$x].R + $left[$y].R) / 4),
        [int](($top[$x].G + $left[$y].G + $top[$x].G + $left[$y].G) / 4),
        [int](($top[$x].B + $left[$y].B + $top[$x].B + $left[$y].B) / 4))
      $bmp.SetPixel($x, $y, $c)
    }
  }

  # Draw the mark on top. SourceOver so the antialiased rim of the rounded
  # shape blends into the fill instead of leaving a seam.
  $g = Set-Quality ([System.Drawing.Graphics]::FromImage($bmp))
  try {
    $g.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceOver
    $g.DrawImage($src, 0, 0, $size, $size)
  }
  finally { $g.Dispose() }

  $src.Dispose()
  return $bmp
}

# Force every pixel opaque. Resampling a silhouette leaves partially transparent
# pixels along its edge, and Windows composites those against the taskbar
# background, which is where the dark corner comes from. Windows applies its own
# mask anyway, so there is nothing to gain from keeping the alpha.
function Set-Opaque($bitmap) {
  $rect = New-Object System.Drawing.Rectangle 0, 0, $bitmap.Width, $bitmap.Height
  $data = $bitmap.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadWrite, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  try {
    $bytes = New-Object byte[] ([Math]::Abs($data.Stride) * $bitmap.Height)
    [System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $bytes, 0, $bytes.Length)
    for ($i = 3; $i -lt $bytes.Length; $i += 4) { $bytes[$i] = 255 }
    [System.Runtime.InteropServices.Marshal]::Copy($bytes, 0, $data.Scan0, $bytes.Length)
  }
  finally { $bitmap.UnlockBits($data) }
}

$master = Get-OpaqueMaster $Source

$square = [ordered]@{
  'StoreLogo.png'         = 50
  'Square44x44Logo.png'   = 44
  'Square71x71Logo.png'   = 71
  'Square150x150Logo.png' = 150
  'Square300x300Logo.png' = 300
  'Square310x310Logo.png' = 310
  'LockScreenLogo.png'    = 24
}

# Non-square assets centre the mark on the app's own background, which is what
# the splash screen is read against.
$wide = [ordered]@{
  'Wide310x150Logo.png' = @(310, 150)
  'SplashScreen.png'    = @(620, 300)
}

$AppBackground = [System.Drawing.ColorTranslator]::FromHtml('#0e0f13')

foreach ($entry in $square.GetEnumerator()) {
  $size = $entry.Value
  $out = Join-Path $OutDir $entry.Key

  $bmp = New-Bitmap $size $size
  $g = Set-Quality ([System.Drawing.Graphics]::FromImage($bmp))
  try {
    $g.DrawImage($master, 0, 0, $size, $size)
    Set-Opaque $bmp
    $bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
  }
  finally {
    $g.Dispose()
    $bmp.Dispose()
  }

  Write-Host ("  {0,-24} {1}x{1}" -f $entry.Key, $size)
}

foreach ($entry in $wide.GetEnumerator()) {
  $w = $entry.Value[0]
  $h = $entry.Value[1]
  $out = Join-Path $OutDir $entry.Key

  $bmp = New-Bitmap $w $h
  $g = Set-Quality ([System.Drawing.Graphics]::FromImage($bmp))
  try {
    $g.Clear($AppBackground)
    $side = [Math]::Min($w, $h)
    $g.DrawImage($master, [int](($w - $side) / 2), [int](($h - $side) / 2), $side, $side)
    Set-Opaque $bmp
    $bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
  }
  finally {
    $g.Dispose()
    $bmp.Dispose()
  }

  Write-Host ("  {0,-24} {1}x{2}" -f $entry.Key, $w, $h)
}

$master.Dispose()
Write-Host "Done -> $OutDir"