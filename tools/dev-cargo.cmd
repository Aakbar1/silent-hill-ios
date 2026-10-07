@echo off
setlocal
set "SH_VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if not exist "%SH_VSWHERE%" (
  echo Visual Studio Installer vswhere.exe is unavailable. 1>&2
  exit /b 1
)
for /f "usebackq delims=" %%i in (`"%SH_VSWHERE%" -latest -products * -prerelease -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "SH_VS_DIR=%%i"
if not defined SH_VS_DIR (
  echo No installed x64 MSVC toolset found. No tools were installed. 1>&2
  exit /b 1
)
set "PATH=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer;%PATH%"
call "%SH_VS_DIR%\Common7\Tools\VsDevCmd.bat" -no_logo -arch=x64 -host_arch=x64
if errorlevel 1 exit /b 1
cargo %*
exit /b %errorlevel%
