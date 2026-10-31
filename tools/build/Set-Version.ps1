param(
  [Parameter(Mandatory = $true)]
  [ValidatePattern('^\d+\.\d+\.\d+\.\d+$')]
  [string]$Version
)

# Writes one version to every place that carries it.
#
# The repository keeps the four-part release version in VERSION. The three-part
# application version is derived from its first three segments, so increasing
# the Store revision does not move the app version. Everything else is generated
# from that single file; running this script is the only way a version should
# change.
#
# Comments are deliberately ASCII: Windows PowerShell 5.1 reads a .ps1 file as
# ANSI unless it carries a BOM.

$ErrorActionPreference = 'Stop'
$root = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
Set-Location $root

$appVersion = ($Version -split '\.')[0..2] -join '.'

# Source of truth.
[IO.File]::WriteAllText(
  (Join-Path $root 'VERSION'),
  "$Version`n",
  (New-Object Text.UTF8Encoding $false))

# package.json and package-lock.json: Node keeps the JSON valid and the
# lockfile's root entries in their original shape.
& node -e "const fs=require('fs');const v=process.argv[1];for(const p of ['package.json','package-lock.json']){const j=JSON.parse(fs.readFileSync(p,'utf8'));j.version=v;if(j.packages&&j.packages[''])j.packages[''].version=v;fs.writeFileSync(p,JSON.stringify(j,null,2)+'\n')}" $appVersion
if ($LASTEXITCODE -ne 0) { throw 'Failed to update package.json/package-lock.json.' }

# Cargo.toml: the only line that starts with `version =` is the package one.
$cargoToml = Join-Path $root 'src-tauri\Cargo.toml'
$text = [IO.File]::ReadAllText($cargoToml)
$text = ([regex]::new('(?m)^version\s*=\s*"[^"]+"')).Replace($text, "version = `"$appVersion`"", 1)
[IO.File]::WriteAllText($cargoToml, $text, (New-Object Text.UTF8Encoding $false))

# Cargo.lock: only the fm-radio package entry.
$cargoLock = Join-Path $root 'src-tauri\Cargo.lock'
$text = [IO.File]::ReadAllText($cargoLock)
$text = [regex]::Replace(
  $text,
  '(?ms)(name = "fm-radio"\r?\nversion = ")[^"]+(")',
  ('${1}' + $appVersion + '${2}'))
[IO.File]::WriteAllText($cargoLock, $text, (New-Object Text.UTF8Encoding $false))

# tauri.conf.json.
& node -e "const fs=require('fs');const p='src-tauri/tauri.conf.json';const j=JSON.parse(fs.readFileSync(p,'utf8'));j.version=process.argv[1];fs.writeFileSync(p,JSON.stringify(j,null,2)+'\n')" $appVersion
if ($LASTEXITCODE -ne 0) { throw 'Failed to update tauri.conf.json.' }

# The MSIX manifest keeps the full four-part version.
$manifest = Join-Path $root 'msix\Package.appxmanifest'
$text = [IO.File]::ReadAllText($manifest)
$text = [regex]::Replace($text, '(?<=<Identity[^>]*Version=")[0-9.]+(?=")', $Version)
[IO.File]::WriteAllText($manifest, $text, (New-Object Text.UTF8Encoding $false))

# README shows the release version. The label is built from escapes so this
# script stays pure ASCII for Windows PowerShell 5.1.
$readme = Join-Path $root 'README.md'
$text = [IO.File]::ReadAllText($readme)
$label = [regex]::Unescape('\u5F53\u524D\u7248\u672C\uFF1A')
$pattern = '(?m)^' + [regex]::Escape($label) + '\*\*v?[0-9.]+\*\*\s*$'
$replacement = $label + '**v' + $Version + '**'
$text = [regex]::Replace($text, $pattern, $replacement)
[IO.File]::WriteAllText($readme, $text, (New-Object Text.UTF8Encoding $false))

Write-Host ("Version set to {0} (application {1})." -f $Version, $appVersion)
Write-Host 'Remember to add a matching section to docs/CHANGELOG.md.'
