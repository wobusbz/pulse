@echo off
rem Builds PulseSetup.exe from the release binary. Run after `cargo build --release`.
cd /d "%~dp0"

set "NSIS=C:\Program Files (x86)\NSIS\makensis.exe"
if not exist "%NSIS%" set "NSIS=makensis.exe"

"%NSIS%" pulse.nsi
