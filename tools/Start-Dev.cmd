@echo off
setlocal

call "%~dp0Setup-Msvc.cmd"
if errorlevel 1 (
  exit /b 1
)

cd /d "%~dp0.."
if errorlevel 1 exit /b 1

call npm.cmd run tauri dev
exit /b %errorlevel%
