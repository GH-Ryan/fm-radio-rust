param(
  [string]$Source = '',
  [string]$Root = (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent)
)

# Regenerates every bitmap that Windows reads from the one branding source.
#
# The source is branding/app-icon-1024.png, a 1024x1024 render of
# branding/app-icon.svg. Nothing downstream should be edited by hand: the Tauri
# icons, the package assets and the Store listing logo are all derived here.
#
# The opaque fill and the multi-size .ico are handled by the two helper scripts
# in this directory; the small Tauri PNGs are downscaled from the 310px package
# logo, which is already opaque, rather than from the transparent master.
#
# Comments are deliberately ASCII: Windows PowerShell 5.1 reads a .ps1 file as
# ANSI unless it carries a BOM.

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

if (-not $Source) { $Source = Join-Path $Root 'branding\app-icon-1024.png' }
if (-not (Test-Path $Source)) { throw "Branding source not found: $Source" }

$storeAssets = Join-Path $PSScriptRoot 'New-StoreAssets.ps1'
$opaqueIco = Join-Path $PSScriptRoot 'New-OpaqueIco.ps1'
$packageAssets = Join-Path $Root 'msix\PackageAssets'
$icons = Join-Path $Root 'src-tauri\icons'
$listing = Join-Path $Root 'assets'

Write-Host '1) Package assets and Store listing logo'
& powershell -NoProfile -ExecutionPolicy Bypass -File $storeAssets -Source $Source -OutDir $packageAssets -ListingDir $listing
if ($LASTEXITCODE -ne 0) { throw 'Package asset generation failed.' }

Write-Host '2) Tauri application icon (multi-size .ico)'
& powershell -NoProfile -ExecutionPolicy Bypass -File $opaqueIco -Source $Source -OutFile (Join-Path $icons 'icon.ico')
if ($LASTEXITCODE -ne 0) { throw 'Icon generation failed.' }

Write-Host '3) Tauri PNG icons'
$big = Join-Path $packageAssets 'Square310x310Logo.png'
if (-not (Test-Path $big)) { throw "Expected generated asset missing: $big" }

$sizes = [ordered]@{
  '32x32.png'      = 32
  '128x128.png'    = 128
  '128x128@2x.png' = 256
}

foreach ($entry in $sizes.GetEnumerator()) {
  $size = $entry.Value
  $src = [System.Drawing.Image]::FromFile($big)
  $bmp = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  try {
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $g.DrawImage($src, 0, 0, $size, $size)
    $bmp.Save((Join-Path $icons $entry.Key), [System.Drawing.Imaging.ImageFormat]::Png)
  }
  finally {
    $g.Dispose()
    $bmp.Dispose()
    $src.Dispose()
  }
  Write-Host ("   {0,-16} {1}x{1}" -f $entry.Key, $size)
}

Write-Host 'Branding assets regenerated.'
