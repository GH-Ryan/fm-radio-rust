param(
  # Derived from the script's own location rather than hardcoded, so the
  # repository can be cloned or moved without editing this line.
  [string]$Root = (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent),
  # The default certificate lives beside the package source rather than in the
  # tree: *.pfx is gitignored, and a signing certificate must not be committed.
  [string]$CertPath = '',
  [string]$CertPassword = 'password'
)

# Builds the Store MSIX from a clean staging directory.
#
# The version comes from VERSION; there is no version argument. Build-Release
# calls this with no parameters, so a release always produces both the
# executable and the package without a second command.
#
# Staging lives under src-tauri/target/ rather than in msix/, so every
# intermediate is covered by the existing Cargo ignore rule and `cargo clean`
# removes it. The staged copies are deleted after signing: the only files that
# survive are the two deliverables in release/.
#
# winapp is not used for packaging. It writes its own priconfig that indexes via
# a pri.resfiles manifest listing only the asset directory, and a list-driven
# index discards the per-language candidates in Strings\: every string resource
# collapses to the config default and the other translations vanish. The
# package still builds and still validates; ms-resource:AppName simply resolves
# to nothing, so the Start menu falls back to the raw identifier.
#
# So the stage is assembled by hand, makepri runs with msix\priconfig.xml, and
# makeappx packs the result. Staging is a fresh directory rather than building
# in place because the folder indexer walks everything under the root, and any
# .msix or .pfx sitting there would be indexed along with the payload.
#
# Comments and messages are deliberately ASCII: Windows PowerShell 5.1 reads a
# .ps1 file as ANSI unless it carries a BOM.

$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path $Root).Path
Set-Location $Root

$versionPath = Join-Path $Root 'VERSION'
if (-not (Test-Path $versionPath)) {
  throw 'VERSION is missing. Create it with tools/build/Set-Version.ps1.'
}
$version = (Get-Content $versionPath -Raw).Trim()
if ($version -notmatch '^\d+\.\d+\.\d+\.\d+$') {
  throw "VERSION '$version' must use four numeric parts."
}

& (Join-Path $PSScriptRoot 'Test-ReleaseReadiness.ps1') -Root $Root

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

$makepri = Join-Path $sdk 'makepri.exe'
$makeappx = Join-Path $sdk 'makeappx.exe'
# Resolved inside the x64 folder specifically. Searching the whole bin tree
# finds the arm64 copy first, which fails with "not a valid application for
# this OS platform".
$signtool = Join-Path $sdk 'signtool.exe'

$mpkg = Join-Path $Root 'msix'
$stage = Join-Path $Root 'src-tauri\target\msix-stage'
$manifest = Join-Path $mpkg 'Package.appxmanifest'
$material = Join-Path $mpkg 'PackageAssets'
$strings = Join-Path $mpkg 'Strings'
$exe = Join-Path $Root 'src-tauri\target\release\fm-radio.exe'
$config = Join-Path $mpkg 'priconfig.xml'
$cert = if ($CertPath) { $CertPath } else { Join-Path $mpkg 'dev-cert.pfx' }

foreach ($p in @($makepri, $makeappx, $signtool, $manifest, $material, $strings, $exe, $config, $cert)) {
  if (-not (Test-Path $p)) { throw "Missing: $p" }
}

Write-Host '1) Assemble clean staging directory'
Remove-Item $stage -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $stage -Force | Out-Null
Copy-Item $exe (Join-Path $stage 'fm-radio.exe') -Force

# makeappx looks for AppxManifest.xml by name; the repo keeps it as
# Package.appxmanifest, which is what winapp expects to be pointed at. The
# Executable attribute carries winapp's $targetnametoken$ placeholder, which
# winapp substitutes while packing and makeappx does not: left alone, package
# validation fails with "the file name \"$targetnametoken$.exe\" ... doesn't
# exist in the package". The staged copy gets the real name instead.
$manifestXml = (Get-Content $manifest -Raw) -replace '\$targetnametoken\$', 'fm-radio'
$manifestXml = [regex]::Replace($manifestXml, '(?<=<Identity[^>]*Version=")[0-9.]+(?=")', $version, 'None')
[IO.File]::WriteAllText(
  (Join-Path $stage 'AppxManifest.xml'),
  $manifestXml,
  (New-Object Text.UTF8Encoding $false))

Copy-Item $material (Join-Path $stage 'PackageAssets') -Recurse -Force
Copy-Item $strings (Join-Path $stage 'Strings') -Recurse -Force
Get-ChildItem $stage -Force | ForEach-Object { Write-Host "   $($_.Name)" }

# Both .resw files must parse. An invalid comment yields PRI224, which reads
# like a config fault, so the XML is validated up front instead.
Write-Host '2) Validate .resw'
foreach ($resw in Get-ChildItem (Join-Path $stage 'Strings') -Recurse -Filter *.resw) {
  try {
    $doc = New-Object System.Xml.XmlDocument
    $doc.Load($resw.FullName)
    $appName = ($doc.root.data | Where-Object { $_.name -eq 'AppName' }).value
    Write-Host "   OK  $($resw.FullName.Replace("$stage\", ''))  AppName='$appName'"
  } catch {
    throw "$($resw.Name) is not valid XML: $($_.Exception.Message)"
  }
}

Write-Host '3) Build resources.pri'
& $makepri new /pr $stage /cf $config /of (Join-Path $stage 'resources.pri') /o | Out-Null
if (-not (Test-Path (Join-Path $stage 'resources.pri'))) { throw 'makepri did not produce resources.pri' }

# Confirm both languages made it in, rather than trusting the exit code.
& $makepri dump /if (Join-Path $stage 'resources.pri') /of (Join-Path $stage 'pri-dump.xml') /o | Out-Null
$dump = [IO.File]::ReadAllText((Join-Path $stage 'pri-dump.xml'), [Text.Encoding]::UTF8)
$langs = [regex]::Match($dump, '<Language>([^<]*)</Language>').Groups[1].Value
Write-Host "   languages: $langs"
foreach ($l in @('zh-CN', 'en-US')) {
  if ($langs -notmatch [regex]::Escape($l)) { throw "resources.pri is missing $l" }
}
Remove-Item (Join-Path $stage 'pri-dump.xml') -Force

$raw = Get-Content (Join-Path $stage 'AppxManifest.xml') -Raw
$name = [regex]::Match($raw, 'Name="([^"]+)"').Groups[1].Value
$packedVersion = [regex]::Match($raw, 'Version="([0-9.]+)"').Groups[1].Value

# Pack outside the staging directory: makeappx walks the folder it packs, and
# an output file inside it would be indexed as payload.
$packDir = Join-Path $env:TEMP 'fm-radio-msix'
Remove-Item $packDir -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $packDir -Force | Out-Null
$msix = Join-Path $packDir ("{0}_{1}_x64.msix" -f $name, $packedVersion)

Write-Host '4) Pack'
& $makeappx pack /d $stage /p $msix /o | Select-Object -Last 2
if (-not (Test-Path $msix)) { throw 'makeappx did not produce the package.' }

Write-Host '5) Sign'
& $signtool sign /fd SHA256 /a /f $cert /p $CertPassword $msix | Select-Object -Last 2

# Deliver into the one stable directory the build as a whole writes to.
$release = Join-Path $Root 'release'
New-Item -ItemType Directory -Path $release -Force | Out-Null
$delivered = Join-Path $release (Split-Path $msix -Leaf)
Copy-Item $msix $delivered -Force

$deliveredExe = Join-Path $release 'fm-radio.exe'
if (-not (Test-Path $deliveredExe)) { Copy-Item $exe $deliveredExe -Force }

# The checksum list covers the two deliverables, so a downloaded pair can be
# verified without a separate command.
$sums = Get-ChildItem $release -File |
  Where-Object { $_.Extension -in '.exe', '.msix' } |
  Sort-Object Name |
  ForEach-Object { "$((Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower())  $($_.Name)" }
$sums | Set-Content (Join-Path $release 'SHA256SUMS.txt') -Encoding ascii

# Staging and the temporary pack directory are regenerable; keeping them would
# leave a second copy of the executable and the package in the tree.
Remove-Item $stage -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item $packDir -Recurse -Force -ErrorAction SilentlyContinue

$item = Get-Item $delivered
Write-Host ''
Write-Host ("Done: {0}  ({1:N2} MB)" -f $delivered, ($item.Length / 1MB))
Write-Host ("Identity: {0} {1}" -f $name, $packedVersion)
Write-Host ("Checksums: {0}" -f (Join-Path $release 'SHA256SUMS.txt'))
