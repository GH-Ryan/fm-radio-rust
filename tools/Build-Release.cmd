@echo off
setlocal

cd /d "%~dp0.."
if errorlevel 1 exit /b 1

rem One entry point, no switches: a release build produces the executable, the
rem signed MSIX and the checksum list on every run.
rem
rem The frontend must be built first. Tauri embeds dist/ at compile time, so the
rem reverse order silently packs the previous frontend.

powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0build\Test-ReleaseReadiness.ps1"
if errorlevel 1 exit /b 1

call "%~dp0build\Setup-Msvc.cmd"
if errorlevel 1 exit /b 1

call npm.cmd run build
if errorlevel 1 (
  echo Error: Frontend build failed.
  exit /b 1
)

call cargo build --manifest-path src-tauri\Cargo.toml --release --features custom-protocol
if errorlevel 1 (
  echo Error: Tauri release build failed.
  exit /b 1
)

rem Start from a clean delivery directory. Old versions must not linger beside
rem the current one or the checksum list would describe a mixed set.
if exist "release" (
  del /q "release\*.exe" "release\*.msix" "release\SHA256SUMS.txt" 2>nul
) else (
  mkdir "release"
)

copy /y "src-tauri\target\release\fm-radio.exe" "release\fm-radio.exe" >nul
if errorlevel 1 (
  echo Error: could not copy fm-radio.exe into release\.
  exit /b 1
)

powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0build\New-MsixPackage.ps1"
if errorlevel 1 (
  echo Error: MSIX packaging failed.
  exit /b 1
)

echo.
echo Release build completed:
echo   release\fm-radio.exe
echo   release\*.msix
echo   release\SHA256SUMS.txt
exit /b 0
