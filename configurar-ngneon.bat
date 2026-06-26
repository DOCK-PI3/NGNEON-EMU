@echo off
setlocal
cd /d "%~dp0"

if exist "configurator\dist\index.html" (
  goto run
)

echo [INFO] Building NGNEON configurator...
pushd configurator
call corepack pnpm install
if errorlevel 1 goto fail
call corepack pnpm build
if errorlevel 1 goto fail
popd

:run
echo [INFO] Opening NGNEON configurator...
start "" "http://127.0.0.1:4177"
pushd configurator
call corepack pnpm start
popd
goto end

:fail
popd
echo [ERROR] No se pudo preparar el configurador.
pause

:end
endlocal
