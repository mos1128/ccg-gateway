@echo off
title CCG Gateway

echo ========================================
echo   CCG Gateway Startup Script
echo ========================================
echo.

cd /d "%~dp0"

:: Load .env file
set GATEWAY_PORT=7788
set UI_PORT=3000
if exist ".env" (
    for /f "usebackq tokens=1,* delims==" %%a in (".env") do (
        if "%%a"=="GATEWAY_PORT" set GATEWAY_PORT=%%b
        if "%%a"=="UI_PORT" set UI_PORT=%%b
    )
)

:: Init frontend dependencies
if not exist "frontend\node_modules" (
    echo [Frontend] Installing dependencies...
    cd frontend
    pnpm install
    cd ..
)

echo.
echo ========================================
echo   Starting Services
echo ========================================
echo.

:: Start backend
echo [Backend] Starting... (port %GATEWAY_PORT%)
start "CCG Gateway - Backend" cmd /k "cd /d %~dp0backend && uv run uvicorn app.main:app --host 127.0.0.1 --port %GATEWAY_PORT% --reload"

timeout /t 3 /nobreak >nul

:: Start frontend
echo [Frontend] Starting... (port %UI_PORT%)
start "CCG Gateway - Frontend" cmd /k "cd /d %~dp0frontend && pnpm dev"

timeout /t 5 /nobreak >nul

echo.
echo ========================================
echo   Services Started
echo ========================================
echo.
echo   Press any key to open browser...
pause >nul

start http://localhost:%UI_PORT%

echo.
echo   To stop services, run stop.bat
echo.
pause
