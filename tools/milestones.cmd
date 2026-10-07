@echo off
setlocal
pushd "%~dp0.."
if errorlevel 1 exit /b 1
call tools\dev-cargo.cmd build --release --locked
if errorlevel 1 (
  popd
  exit /b 1
)
python tools\milestones.py %*
set "SH_MILESTONE_EXIT=%errorlevel%"
popd
exit /b %SH_MILESTONE_EXIT%
