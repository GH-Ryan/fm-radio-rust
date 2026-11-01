param(
  [string]$Root = (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent)
)

# Validates that every versioned file agrees with VERSION.
#
# VERSION is the single source of truth for the four-part release version; the
# three-part application version is derived from its first three segments. This
# check exists to catch a hand edit that bypassed tools/build/Set-Version.ps1,
# not as the place versions are set.
#
# Comments and messages are deliberately ASCII: Windows PowerShell 5.1 reads a
# .ps1 file as ANSI unless it carries a BOM.

$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path $Root).Path
Set-Location $Root

function Stop-Release([string]$Message) {
  throw "Release readiness check failed: $Message"
}

$versionPath = Join-Path $Root 'VERSION'
if (-not (Test-Path $versionPath)) {
  Stop-Release 'VERSION is missing. Create it with tools/build/Set-Version.ps1.'
}
$releaseVersion = (Get-Content $versionPath -Raw).Trim()
if ($releaseVersion -notmatch '^\d+\.\d+\.\d+\.\d+$') {
  Stop-Release "VERSION '$releaseVersion' must use four numeric parts."
}
if (($releaseVersion -split '\.')[3] -ne '0') {
  Stop-Release "VERSION '$releaseVersion' has a non-zero revision; the Store requires the fourth part to be 0. Bump the third part instead."
}
$appVersion = ($releaseVersion -split '\.')[0..2] -join '.'

$package = Get-Content (Join-Path $Root 'package.json') -Raw | ConvertFrom-Json
$tauri = Get-Content (Join-Path $Root 'src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json
$lockVersions = @(& node -e "const lock = require('./package-lock.json'); console.log(lock.version); console.log(lock.packages[''].version)")
if ($LASTEXITCODE -ne 0 -or $lockVersions.Count -ne 2) {
  Stop-Release 'could not read package-lock.json versions with Node.js.'
}
$cargoToml = Get-Content (Join-Path $Root 'src-tauri\Cargo.toml') -Raw
$cargoLock = Get-Content (Join-Path $Root 'src-tauri\Cargo.lock') -Raw
$manifest = [xml](Get-Content (Join-Path $Root 'msix\Package.appxmanifest') -Raw)
$identity = $manifest.SelectSingleNode("//*[local-name()='Identity']")

$versions = [ordered]@{
  'package.json'                 = [string]$package.version
  'package-lock.json root'       = [string]$lockVersions[0]
  'package-lock.json package'    = [string]$lockVersions[1]
  'src-tauri/Cargo.toml'         = [regex]::Match($cargoToml, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
  'src-tauri/Cargo.lock'         = [regex]::Match($cargoLock, '(?ms)^name = "fm-radio"\r?\nversion = "([^"]+)"').Groups[1].Value
  'src-tauri/tauri.conf.json'    = [string]$tauri.version
}

foreach ($entry in $versions.GetEnumerator()) {
  if ($entry.Value -ne $appVersion) {
    Stop-Release "$($entry.Key) has version '$($entry.Value)'; expected '$appVersion'. Run tools/build/Set-Version.ps1 -Version $releaseVersion."
  }
}

if (-not $identity) {
  Stop-Release 'msix/Package.appxmanifest has no Identity element.'
}
$manifestVersion = $identity.GetAttribute('Version')
if ($manifestVersion -ne $releaseVersion) {
  Stop-Release "msix/Package.appxmanifest has version '$manifestVersion'; expected '$releaseVersion'."
}

$readmePath = Join-Path $Root 'README.md'
$readme = Get-Content $readmePath -Raw -Encoding UTF8
if ($readme -notmatch '\]\(docs/CHANGELOG\.md\)') {
  Stop-Release 'README.md must link to docs/CHANGELOG.md.'
}
$label = [regex]::Unescape('\u5F53\u524D\u7248\u672C\uFF1A')
$readmePattern = '(?m)^' + [regex]::Escape($label) + '\*\*v?' + [regex]::Escape($releaseVersion) + '\*\*\s*$'
if ($readme -notmatch $readmePattern) {
  Stop-Release "README.md must show the current version as '$label**v$releaseVersion**' on its own line."
}

$changelogPath = Join-Path $Root 'docs\CHANGELOG.md'
if (-not (Test-Path $changelogPath)) {
  Stop-Release 'docs/CHANGELOG.md is missing.'
}
$changelog = Get-Content $changelogPath -Raw -Encoding UTF8
$releaseEntry = [regex]::Match(
  $changelog,
  "(?ms)^## \[$([regex]::Escape($releaseVersion))\](?:[ \t]+-[^\r\n]*)?\r?\n(.*?)(?=^## |\z)")
$releaseNotes = $releaseEntry.Groups[1].Value
if (-not $releaseEntry.Success -or
    $releaseNotes -notmatch '(?m)^\s*-\s+\S' -or
    $releaseNotes -match '(?i)\b(TODO|TBD)\b') {
  Stop-Release "docs/CHANGELOG.md must contain a non-empty '## [$releaseVersion]' release entry."
}

$tags = @(& git tag --list)
if ($LASTEXITCODE -ne 0) {
  Stop-Release 'could not read Git tags.'
}
$releaseTags = foreach ($tag in $tags) {
  if ($tag -match '^(?:v)?(\d+\.\d+\.\d+(?:\.\d+)?)$') {
    $tagVersion = $Matches[1]
    if ($tagVersion.Split('.').Count -eq 3) {
      $tagVersion += '.0'
    }
    [pscustomobject]@{ Name = $tag; Version = [version]$tagVersion }
  }
}
$latestTag = $releaseTags | Sort-Object Version -Descending | Select-Object -First 1
if ($latestTag -and [version]$releaseVersion -lt $latestTag.Version) {
  Stop-Release "version '$releaseVersion' is lower than the latest release tag '$($latestTag.Name)'."
}
if ($latestTag -and [version]$releaseVersion -eq $latestTag.Version) {
  Stop-Release "version '$releaseVersion' has already been released as '$($latestTag.Name)'; bump VERSION with tools/build/Set-Version.ps1."
}

if ($latestTag) {
  foreach ($path in @('README.md', 'docs/CHANGELOG.md')) {
    $baselineEntry = @(& git ls-tree $latestTag.Name -- $path)
    if ($LASTEXITCODE -ne 0) {
      Stop-Release "could not compare $path with Git tag '$($latestTag.Name)'."
    }
    if ($baselineEntry.Count -gt 0) {
      $baselineContent = @(& git show "$($latestTag.Name):$path")
      if ($LASTEXITCODE -ne 0) {
        Stop-Release "could not read $path from Git tag '$($latestTag.Name)'."
      }
      if (($baselineContent -join "`n").TrimEnd() -eq
          (Get-Content (Join-Path $Root $path) -Raw).TrimEnd()) {
        Stop-Release "$path has not been updated since Git tag '$($latestTag.Name)'."
      }
    }
  }
}

if ($latestTag) {
  Write-Host "Release readiness checks passed for v$releaseVersion (previous tag: $($latestTag.Name))."
} else {
  Write-Host "Initial release readiness checks passed for v$releaseVersion."
}
