param(
  # Derived from the script's own location rather than hardcoded, so the
  # repository can be cloned or moved without editing this line.
  [string]$Root = (Split-Path $PSScriptRoot -Parent),
  # The default certificate lives beside this script rather than in the tree:
  # *.pfx is gitignored, and a signing certificate must not be committed.
  [string]$CertPath = '',
  [string]$CertPassword = 'password',
  # Every Store submission needs a strictly higher version than the last, and
  # the version committed in the manifest is only a placeholder. Rewritten on
  # the staged copy only, so the tree keeps its committed value.
  [ValidatePattern('^\d+\.\d+\.\d+\.\d+$')]
  [string]$Version = ''
)

# Builds the Store MSIX from a clean staging directory.
#
# winapp is not used for packaging. It writes its own priconfig that indexes via
# a pri.resfiles manifest listing only Assets\, and a list-driven index discards
# the per-language candidates in Strings\: every string resource collapses to
# the config default and the other translations vanish. The package still builds
# and still validates; ms-resource:AppName simply resolves to nothing, so the
# Start menu falls back to the raw identifier.
#
# So the stage is assembled by hand, makepri runs with msix\priconfig.xml, and
# makeappx packs the result. Staging is a fresh directory rather than building
# in place because the folder indexer walks everything under the root, and any
# .msix or .pfx sitting there would be indexed along with the payload.
#
# Signing uses msix\dev-cert.pfx, or the certificate named by -CertPath. It is
# not trusted on this machine, which only affects local install; Partner Center
# validates the signature rather than the chain.
#
# Comments and messages are deliberately ASCII: Windows PowerShell 5.1 reads a
# .ps1 file as ANSI unless it carries a BOM, so a non-ASCII character anywhere
# corrupts the parse rather than merely looking wrong.

$ErrorActionPreference = 'Stop'

if (-not $Version) {
  throw 'Pass -Version in major.minor.patch.revision format, for example -Version "0.2.0.0".'
}

& (Join-Path $PSScriptRoot 'Test-ReleaseReadiness.ps1') -Root $Root -MsixVersion $Version

# Locate the newest installed SDK rather than pinning a version. The SDK build
# changes with Windows updates and differs between machines, and a hardcoded
# path turns that into a missing-file error that reads like a broken script.
$sdkRoot = 'C:\Program Files (x86)\Windows Kits\10\bin'
$sdk = (Get-ChildItem $sdkRoot -Directory -ErrorAction SilentlyContinue |
        Where-Object { Test-Path (Join-Path $_.FullName 'x64\makepri.exe') } |
        Sort-Object Name -Descending |
        Select-Object -First 1).FullName
if (-not $sdk) { throw "No Windows SDK with makepri.exe found under $sdkRoot" }
$sdk = Join-Path $sdk 'x64'

$makepri  = Join-Path $sdk 'makepri.exe'
$makeappx = Join-Path $sdk 'makeappx.exe'
# Resolved inside the x64 folder specifically. Searching the whole bin tree
# finds the arm64 copy first, which fails with "not a valid application for
# this OS platform".
$signtool = Join-Path $sdk 'signtool.exe'

# The package source -- manifest, assets, localised strings and the resource
# index configuration -- lives in msix/, separate from the Tauri crate in
# src-tauri/. The staging directory and the certificate sit beside them, since
# neither is part of the application.
$mpkg    = Join-Path $Root 'msix'
$build   = Join-Path $mpkg 'build'
$cert    = if ($CertPath) { $CertPath } else { Join-Path $mpkg 'dev-cert.pfx' }
$manifest = Join-Path $mpkg 'Package.appxmanifest'
$exe      = Join-Path $Root 'src-tauri\target\release\fm-radio.exe'
$config   = Join-Path $mpkg 'priconfig.xml'

foreach ($p in @($makepri, $makeappx, $signtool, $manifest, $exe, $config, $cert)) {
  if (-not (Test-Path $p)) { throw "Missing: $p" }
}

Write-Host '1) Assemble clean staging directory'
Remove-Item $build -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $build -Force | Out-Null
Copy-Item $exe "$build\fm-radio.exe" -Force

# makeappx looks for AppxManifest.xml by name; the repo keeps it as
# Package.appxmanifest, which is what winapp expects to be pointed at. The
# Executable attribute carries winapp's $targetnametoken$ placeholder, which
# winapp substitutes while packing and makeappx does not: left alone, package
# validation fails with "the file name \"$targetnametoken$.exe\" ... doesn't
# exist in the package". The staged copy gets the real name instead.
$manifestXml = (Get-Content $manifest -Raw) -replace '\$targetnametoken\$', 'fm-radio'
if ($Version) {
  $manifestXml = [regex]::Replace(
    $manifestXml, '(?<=<Identity[^>]*Version=")[0-9.]+(?=")', $Version, 'None')
}
[IO.File]::WriteAllText("$build\AppxManifest.xml", $manifestXml, (New-Object Text.UTF8Encoding $false))

# msix\Assets\ holds only what Windows reads from inside the package, so it is
# copied whole. The listing screenshots are in assets\ and are never part of a
# package -- they are uploaded to Partner Center by hand.
Copy-Item (Join-Path $mpkg 'Assets') $build -Recurse -Force
Copy-Item (Join-Path $mpkg 'Strings') $build -Recurse -Force
Get-ChildItem $build -Force | ForEach-Object { Write-Host "   $($_.Name)" }

# Both .resw files must parse. An invalid comment yields PRI224, which reads
# like a config fault, so the XML is validated up front instead.
Write-Host '2) Validate .resw'
foreach ($resw in Get-ChildItem "$build\Strings" -Recurse -Filter *.resw) {
  try {
    $doc = New-Object System.Xml.XmlDocument
    $doc.Load($resw.FullName)
    $appName = ($doc.root.data | Where-Object { $_.name -eq 'AppName' }).value
    Write-Host "   OK  $($resw.FullName.Replace("$build\", ''))  AppName='$appName'"
  } catch {
    throw "$($resw.Name) is not valid XML: $($_.Exception.Message)"
  }
}

Write-Host '3) Build resources.pri'
& $makepri new /pr $build /cf $config /of "$build\resources.pri" /o | Out-Null
if (-not (Test-Path "$build\resources.pri")) { throw 'makepri did not produce resources.pri' }

# Confirm both languages made it in, rather than trusting the exit code.
& $makepri dump /if "$build\resources.pri" /of "$build\pri-dump.xml" /o | Out-Null
$dump = [IO.File]::ReadAllText("$build\pri-dump.xml", [Text.Encoding]::UTF8)
$langs = [regex]::Match($dump, '<Language>([^<]*)</Language>').Groups[1].Value
Write-Host "   languages: $langs"
foreach ($l in @('zh-CN', 'en-US')) {
  if ($langs -notmatch [regex]::Escape($l)) { throw "resources.pri is missing $l" }
}
Remove-Item "$build\pri-dump.xml" -Force

$raw = Get-Content "$build\AppxManifest.xml" -Raw
$name = [regex]::Match($raw, 'Name="([^"]+)"').Groups[1].Value
$version = [regex]::Match($raw, 'Version="([0-9.]+)"').Groups[1].Value
$msix = Join-Path $build ("{0}_{1}_x64.msix" -f $name, $version)

Write-Host '4) Pack'
& $makeappx pack /d $build /p $msix /o | Select-Object -Last 2

Write-Host '5) Sign'
& $signtool sign /fd SHA256 /a /f $cert /p $CertPassword $msix | Select-Object -Last 2

$item = Get-Item $msix
Write-Host ''
Write-Host ("Done: {0}  ({1:N2} MB)" -f $msix, ($item.Length / 1MB))
Write-Host ("Identity: {0} {1}" -f $name, $version)