param(
  [string]$Root = (Split-Path $PSScriptRoot -Parent),
  [string]$MsixVersion = ''
)

$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path $Root).Path
Set-Location $Root

function Stop-Release([string]$Message) {
  throw "Release readiness check failed: $Message"
}

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

$appVersion = [string]$package.version
if ($appVersion -notmatch '^\d+\.\d+\.\d+$') {
  Stop-Release "package.json version '$appVersion' must use major.minor.patch."
}

$versions = [ordered]@{
  'package-lock.json root' = [string]$lockVersions[0]
  'package-lock.json package' = [string]$lockVersions[1]
  'src-tauri/Cargo.toml' = [regex]::Match($cargoToml, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
  'src-tauri/Cargo.lock' = [regex]::Match(
    $cargoLock, '(?ms)^name = "fm-radio"\r?\nversion = "([^"]+)"').Groups[1].Value
  'src-tauri/tauri.conf.json' = [string]$tauri.version
}

foreach ($entry in $versions.GetEnumerator()) {
  if ($entry.Value -ne $appVersion) {
    Stop-Release "$($entry.Key) has version '$($entry.Value)'; update it to '$appVersion'."
  }
}

if (-not $identity -or $identity.GetAttribute('Version') -notmatch '^\d+\.\d+\.\d+\.\d+$') {
  Stop-Release 'msix/Package.appxmanifest must use a four-part numeric version.'
}
$releaseVersion = $identity.GetAttribute('Version')
$manifestAppVersion = ($releaseVersion -split '\.')[0..2] -join '.'
if ($manifestAppVersion -ne $appVersion) {
  Stop-Release "msix/Package.appxmanifest has version '$releaseVersion'; expected '$appVersion.0'."
}

if ($MsixVersion) {
  if ($MsixVersion -notmatch '^\d+\.\d+\.\d+\.\d+$') {
    Stop-Release "MSIX version '$MsixVersion' must use four numeric components."
  }
  if ($MsixVersion -ne $releaseVersion) {
    Stop-Release "MSIX version '$MsixVersion' does not match manifest version '$releaseVersion'."
  }
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
$isExistingReleaseMsixBuild = $latestTag -and
  [version]$releaseVersion -eq $latestTag.Version -and
  $MsixVersion -eq $releaseVersion
if ($latestTag -and [version]$releaseVersion -lt $latestTag.Version) {
  Stop-Release "version '$releaseVersion' is lower than the latest release tag '$($latestTag.Name)'."
}
if ($latestTag -and [version]$releaseVersion -eq $latestTag.Version -and -not $isExistingReleaseMsixBuild) {
  Stop-Release "version '$releaseVersion' has already been released as '$($latestTag.Name)'; only an MSIX build of that exact version is allowed."
}

$readmePath = Join-Path $Root 'README.md'
$readme = Get-Content $readmePath -Raw
if ($readme -notmatch '\]\(docs/CHANGELOG\.md\)') {
  Stop-Release 'README.md must link to docs/CHANGELOG.md.'
}
if ($readme -notmatch "(?m)^\u5F53\u524D\u7248\u672C\uFF1A\*\*v?$([regex]::Escape($releaseVersion))\*\*\s*$") {
  Stop-Release "README.md must show the current version as 'current version: **v$releaseVersion**'."
}

$changelogPath = Join-Path $Root 'docs\CHANGELOG.md'
if (-not (Test-Path $changelogPath)) {
  Stop-Release 'docs/CHANGELOG.md is missing.'
}
$changelog = Get-Content $changelogPath -Raw
$releaseEntry = [regex]::Match(
  $changelog,
  "(?ms)^## \[$([regex]::Escape($releaseVersion))\](?:[ \t]+-[^\r\n]*)?\r?\n(.*?)(?=^## |\z)")
$releaseNotes = $releaseEntry.Groups[1].Value
if (-not $releaseEntry.Success -or
    $releaseNotes -notmatch '(?m)^\s*-\s+\S' -or
    $releaseNotes -match '(?i)\b(TODO|TBD)\b') {
  Stop-Release "docs/CHANGELOG.md must contain a non-empty '## [$releaseVersion]' release entry."
}

if ($latestTag -and -not $isExistingReleaseMsixBuild) {
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

if ($isExistingReleaseMsixBuild) {
  Write-Host "MSIX rebuild readiness checks passed for existing v$releaseVersion."
} elseif ($latestTag) {
  Write-Host "Release readiness checks passed for v$releaseVersion (previous tag: $($latestTag.Name))."
} else {
  Write-Host "Initial release readiness checks passed for v$releaseVersion."
}
