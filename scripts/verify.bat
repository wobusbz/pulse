@echo off
rem Runs the same steps as the CI + packaging flow, locally, so you can verify
rem a release before pushing / tagging.
setlocal
cd /d "%~dp0\.."

echo === 1/6  cargo fmt --check ===
cargo fmt --all -- --check || goto :fail

echo.
echo === 2/6  cargo check --all-targets ===
cargo check --all-targets || goto :fail

echo.
echo === 3/6  cargo clippy --all-targets ===
cargo clippy --all-targets
if errorlevel 1 echo   (clippy 有告警，CI 里是非阻断，这里也继续)

echo.
echo === 4/6  cargo test ===
cargo test || goto :fail

echo.
echo === 5/6  cargo build --release ===
cargo build --release || goto :fail

echo.
echo === 6/6  installer\build.bat (NSIS) ===
call installer\build.bat || goto :fail

echo.
echo ============================================
echo  OK
echo    target\release\Pulse.exe
echo    installer\PulseSetup.exe
echo ============================================
exit /b 0

:fail
echo.
echo ============================================
echo  FAILED (step above returned an error)
echo ============================================
exit /b 1
