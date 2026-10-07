@echo off
setlocal
set "SH_VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
for /f "usebackq delims=" %%i in (`"%SH_VSWHERE%" -latest -products * -prerelease -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "SH_VS_DIR=%%i"
if not defined SH_VS_DIR exit /b 1
set "PATH=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer;%PATH%"
if not exist target\core-layouts mkdir target\core-layouts
call "%SH_VS_DIR%\Common7\Tools\VsDevCmd.bat" -no_logo -arch=x86 -host_arch=x64
if errorlevel 1 exit /b 1
cl /nologo /std:c11 /W4 /WX /c /I port port\layout_check.c /Fo:target\core-layouts\disk32-x86.obj
if errorlevel 1 exit /b 1
call "%SH_VS_DIR%\Common7\Tools\VsDevCmd.bat" -no_logo -arch=x64 -host_arch=x64
if errorlevel 1 exit /b 1
cl /nologo /std:c11 /W4 /WX /c /I port port\layout_check.c /Fo:target\core-layouts\disk32-x64.obj
exit /b %errorlevel%
