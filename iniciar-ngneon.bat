@echo off
setlocal EnableExtensions
cd /d "%~dp0"

set "FRONTEND=%CD%\ngneon-frontend.exe"
if not exist "%FRONTEND%" set "FRONTEND=%CD%\target\release\ngneon-frontend.exe"

if not exist "%FRONTEND%" (
  if exist "%CD%\Cargo.toml" (
    echo [INFO] Compilando el frontend nativo de NGNEON...
    cargo build --release -p frontend --bin ngneon-frontend
    if errorlevel 1 goto build_error
    set "FRONTEND=%CD%\target\release\ngneon-frontend.exe"
  )
)

if not exist "%FRONTEND%" (
  echo [ERROR] No se encontro ngneon-frontend.exe.
  pause
  exit /b 1
)

start "" "%FRONTEND%"
exit /b 0

:build_error
echo [ERROR] No se pudo compilar ngneon-frontend.exe.
pause
exit /b 1
