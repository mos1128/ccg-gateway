@echo off
title CCG Gateway - Backend
cd /d "%~dp0"

:: Load .env file
set GATEWAY_PORT=7788
if exist ".env" (
    for /f "usebackq tokens=1,* delims==" %%a in (".env") do (
        if "%%a"=="GATEWAY_PORT" set GATEWAY_PORT=%%b
    )
)

cd backend
uv run uvicorn app.main:app --host 127.0.0.1 --port %GATEWAY_PORT% --reload --reload-exclude .venv
