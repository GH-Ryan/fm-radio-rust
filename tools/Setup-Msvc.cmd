@echo off

set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if not exist "%VSWHERE%" (
  echo Error: Visual Studio Installer was not found at "%VSWHERE%".
  exit /b 1
)

set "VSINSTALL="
for /f "usebackq tokens=*" %%i in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSINSTALL=%%i"
if not defined VSINSTALL (
  echo Error: MSVC x64/x86 Build Tools are not installed.
  exit /b 1
)

set "VSDEVCMD=%VSINSTALL%\Common7\Tools\VsDevCmd.bat"
if not exist "%VSDEVCMD%" (
  echo Error: Visual Studio developer environment was not found at "%VSDEVCMD%".
  exit /b 1
)

call "%VSDEVCMD%" -arch=amd64
if errorlevel 1 (
  echo Error: Failed to initialize the Visual Studio developer environment.
  exit /b 1
)

where cl.exe >nul 2>nul
if errorlevel 1 (
  echo Error: MSVC compiler cl.exe is unavailable after environment setup.
  exit /b 1
)

where link.exe >nul 2>nul
if errorlevel 1 (
  echo Error: MSVC linker link.exe is unavailable after environment setup.
  exit /b 1
)
