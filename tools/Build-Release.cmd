@echo off
setlocal

cd /d "%~dp0.."
if errorlevel 1 exit /b 1

powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Test-ReleaseReadiness.ps1"
if errorlevel 1 exit /b 1

call "%~dp0Setup-Msvc.cmd"
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

echo.
echo Release build completed: src-tauri\target\release\fm-radio.exe
exit /b 0
