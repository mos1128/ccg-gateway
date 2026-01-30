@echo off
setlocal enabledelayedexpansion

echo ========================================
echo   CCG Gateway Development Environment
echo ========================================
echo.

:: Check required tools
echo [Check] Verifying required tools...
where pnpm > nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] pnpm not found. Please install: npm install -g pnpm
    pause
    exit /b 1
)

where cargo > nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] cargo not found. Please install Rust: https://rustup.rs/
    pause
    exit /b 1
)
echo [OK] Environment check passed
echo.

:: Start Tauri dev (automatically starts frontend + backend)
echo [Starting] Launching Tauri development environment...
echo [INFO] First run may take longer to compile dependencies
echo [INFO] Frontend will start automatically (http://localhost:7786)
echo.
set RUST_LOG=info,ccg_gateway=debug,ccg_gateway_lib=debug
tauri dev

if %errorlevel% neq 0 (
    echo.
    echo [ERROR] Tauri development environment failed to start
    pause
    exit /b 1
)
echo.
echo ========================================
echo   Development environment exited
echo ========================================
pause
