param(
  [Parameter(Mandatory = $true)][string]$Source,
  [Parameter(Mandatory = $true)][string]$OutFile
)

# Writes an opaque, multi-size icon.ico.
#
# The existing icon.ico carries the vector mark's rounded corners as
# transparency. Windows composites that against whatever is behind it, which on
# a taskbar reads as a dark corner, so every asset is made opaque first and the
# system applies its own mask.
#
# A multi-size .ico cannot be produced through System.Drawing.Icon, which holds
# a single image, so the ICONDIR and ICONDIRENTRY structures are written by hand.
#
# Everything goes through a BinaryWriter over a MemoryStream: that class has a
# fixed little-endian layout for the integer writes regardless of host, and it
# keeps PowerShell from inferring byte[] as Object[] the way array concatenation
# does.
#
# Comments are deliberately ASCII: Windows PowerShell 5.1 reads a .ps1 file as
# ANSI unless it carries a BOM.
#
# Usage:
#   powershell -File tools/New-OpaqueIco.ps1 -Source icons\icon-1024.png -OutFile icons\icon.ico

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

if (-not (Test-Path $Source)) { throw "Source not found: $Source" }

$sizes = @(256, 128, 64, 48, 32, 16)

# Builds a fully opaque master: the mark over its own gradient extended to the
# corners. Filling the rounded area with a flat colour instead leaves a dark
# frame around a coloured icon, which is the same artefact by another route, so
# the corner colour is sampled from the nearest opaque pixel of the same row.
function Get-OpaqueMaster([string]$path) {
  $src = New-Object System.Drawing.Bitmap($path)
  $size = $src.Width
  $bmp = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)

  $top = New-Object 'System.Drawing.Color[]' $size
  $left = New-Object 'System.Drawing.Color[]' $size

  for ($x = 0; $x -lt $size; $x++) {
    for ($y = 0; $y -lt $size; $y++) {
      if ($src.GetPixel($x, $y).A -ge 250) { $top[$x] = $src.GetPixel($x, $y); break }
    }
  }
  for ($y = 0; $y -lt $size; $y++) {
    for ($x = 0; $x -lt $size; $x++) {
      if ($src.GetPixel($x, $y).A -ge 250) { $left[$y] = $src.GetPixel($x, $y); break }
    }
  }

  for ($y = 0; $y -lt $size; $y++) {
    for ($x = 0; $x -lt $size; $x++) {
      if ($src.GetPixel($x, $y).A -ge 250) { continue }
      $bmp.SetPixel($x, $y, [System.Drawing.Color]::FromArgb(255,
        [int](($top[$x].R + $left[$y].R * 3) / 4),
        [int](($top[$x].G + $left[$y].G * 3) / 4),
        [int](($top[$x].B + $left[$y].B * 3) / 4)))
    }
  }

  $g = [System.Drawing.Graphics]::FromImage($bmp)
  try {
    $g.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceOver
    $g.DrawImage($src, 0, 0, $size, $size)
  }
  finally { $g.Dispose() }

  $src.Dispose()
  return $bmp
}

$src = Get-OpaqueMaster $Source
$payloads = New-Object System.Collections.ArrayList

foreach ($size in $sizes) {
  $bmp = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  try {
    $g.InterpolationMode  = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.PixelOffsetMode    = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.SmoothingMode      = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $g.DrawImage($src, 0, 0, $size, $size)
  }
  finally { $g.Dispose() }

  # Force alpha opaque: resampling a rounded silhouette leaves partially
  # transparent pixels along the arc, and those are the artefact.
  $rect = New-Object System.Drawing.Rectangle 0, 0, $size, $size
  $data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadWrite, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  try {
    $absStride = [Math]::Abs($data.Stride)
    $rows = New-Object byte[] ($absStride * $size)
    [System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $rows, 0, $rows.Length)
    for ($i = 3; $i -lt $rows.Length; $i += 4) { $rows[$i] = 255 }
  }
  finally { $bmp.UnlockBits($data) }
  $bmp.Dispose()

  $stride = $size * 4
  $xor = New-Object byte[] ($stride * $size)
  # BMP rows run bottom-up.
  for ($row = 0; $row -lt $size; $row++) {
    [Array]::Copy($rows, ($size - 1 - $row) * $stride, $xor, $row * $stride, $stride)
  }

  # AND mask: one bit per pixel, rows padded to a 4-byte boundary. All zero
  # means opaque, which the forced alpha above already guarantees.
  $maskStride = [int]([Math]::Ceiling($size / 32) * 4)
  $mask = New-Object byte[] ($maskStride * $size)

  $payloadSize = 40 + $xor.Length + $mask.Length
  [void]$payloads.Add(@{
    Size = $size
    Length = $payloadSize
    Xor = $xor
    Mask = $mask
  })
}

$src.Dispose()

$ms = New-Object System.IO.MemoryStream
$w = New-Object System.IO.BinaryWriter($ms)

$w.Write([UInt16]0)          # idReserved
$w.Write([UInt16]1)          # idType: icon
$w.Write([UInt16]$sizes.Count)

$offset = 6 + (16 * $sizes.Count)
foreach ($p in $payloads) {
  $dim = if ($p.Size -ge 256) { 0 } else { $p.Size }
  $w.Write([byte]$dim)
  $w.Write([byte]$dim)
  $w.Write([byte]0)          # colour count
  $w.Write([byte]0)          # reserved
  $w.Write([UInt16]1)        # colour planes
  $w.Write([UInt16]32)       # bits per pixel
  $w.Write([UInt32]$p.Length)
  $w.Write([UInt32]$offset)
  $offset += $p.Length
}

foreach ($p in $payloads) {
  # BITMAPINFOHEADER: 40 bytes, bottom-up, 32bpp BI_RGB.
  $w.Write([UInt32]40)
  $w.Write([UInt32]$p.Size)
  $w.Write([UInt32]($p.Size * 2))
  $w.Write([UInt16]1)
  $w.Write([UInt16]32)
  $w.Write([UInt32]0)
  $w.Write([UInt32]$p.Xor.Length)
  $w.Write([UInt32]0)        # biXPelsPerMeter
  $w.Write([UInt32]0)        # biYPelsPerMeter
  $w.Write([UInt32]0)        # biClrUsed
  $w.Write([UInt32]0)        # biClrImportant

  $w.Write([byte[]]$p.Xor)
  $w.Write([byte[]]$p.Mask)
}

$w.Flush()
[System.IO.File]::WriteAllBytes($OutFile, $ms.ToArray())
$w.Dispose()
$ms.Dispose()

Write-Host "Wrote $OutFile ($($sizes.Count) sizes)"