@echo off
rem Builds the installer from the release binary. Run after `cargo build --release`.
rem Version comes from Cargo.toml unless APP_VERSION is already set in the env.
setlocal
cd /d "%~dp0"

set "NSIS=C:\Program Files (x86)\NSIS\makensis.exe"
if not exist "%NSIS%" set "NSIS=makensis.exe"

if not defined APP_VERSION (
  for /f "tokens=3 delims= " %%v in ('findstr /r /c:"^version = " ..\Cargo.toml') do set "APP_VERSION=%%~v"
)

set "VERARG="
if defined APP_VERSION set "VERARG=/DAPP_VERSION=%APP_VERSION%"

echo Building installer (version=%APP_VERSION%) ...
"%NSIS%" %VERARG% pulse.nsi
