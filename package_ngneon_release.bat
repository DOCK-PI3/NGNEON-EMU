@echo off
setlocal EnableExtensions

set "ROOT=%~dp0"
if "%ROOT:~-1%"=="\" set "ROOT=%ROOT:~0,-1%"
set "OUT=%ROOT%\NGNEON-EMU"
set "EXE=%ROOT%\target\release\ngneon-emu.exe"
set "SDL_RELEASE=%ROOT%\target\release\SDL2.dll"
set "SDL_ROOT=%ROOT%\SDL2.dll"
set "MEDIA_ROOT=%ROOT%\media"
set "MEDIA_RELEASE=%ROOT%\target\release\media"
set "CONFIGURATOR=%ROOT%\configurator"

echo.
echo === NGNEON-EMU release packager ===
echo Project: "%ROOT%"
echo Output : "%OUT%"
echo.

echo [INFO] Building release...
pushd "%ROOT%" >nul
cargo build --release --workspace
if errorlevel 1 (
    popd >nul
    echo [ERROR] Release build failed.
    exit /b 1
)
popd >nul

if exist "%CONFIGURATOR%\package.json" (
    echo [INFO] Building configurator with pnpm...
    call corepack pnpm --dir "%CONFIGURATOR%" install
    if errorlevel 1 (
        echo [ERROR] Configurator pnpm install failed.
        exit /b 1
    )
    call corepack pnpm --dir "%CONFIGURATOR%" build
    if errorlevel 1 (
        echo [ERROR] Configurator build failed.
        exit /b 1
    )
)

if not exist "%EXE%" (
    echo [ERROR] Missing executable: "%EXE%"
    exit /b 1
)

if not exist "%OUT%" mkdir "%OUT%"
if not exist "%OUT%\screenshots" mkdir "%OUT%\screenshots"

echo [INFO] Cleaning build-only files from package folder...
del /q "%OUT%\*.d" "%OUT%\*.rlib" "%OUT%\*.lib" "%OUT%\*.pdb" 2>nul
del /q "%OUT%\compare_rom_banks.exe" "%OUT%\diagnose_rom.exe" "%OUT%\make_test_neo.exe" "%OUT%\probe_rom.exe" "%OUT%\profile_resampler.exe" "%OUT%\render_test_neo.exe" "%OUT%\scan_cmc50_xor.exe" 2>nul

echo [INFO] Copying executable...
copy /y "%EXE%" "%OUT%\ngneon-emu.exe" >nul
if errorlevel 1 exit /b 1

echo [INFO] Copying SDL2.dll...
if exist "%SDL_RELEASE%" (
    copy /y "%SDL_RELEASE%" "%OUT%\SDL2.dll" >nul
) else if exist "%SDL_ROOT%" (
    copy /y "%SDL_ROOT%" "%OUT%\SDL2.dll" >nul
) else (
    echo [ERROR] SDL2.dll not found in target\release or project root.
    exit /b 1
)
if errorlevel 1 exit /b 1

if exist "%ROOT%\configurar-ngneon.bat" (
    echo [INFO] Copying configurator launcher...
    copy /y "%ROOT%\configurar-ngneon.bat" "%OUT%\configurar-ngneon.bat" >nul
    if errorlevel 1 exit /b 1
)

if exist "%CONFIGURATOR%\" (
    echo [INFO] Copying configurator\ ...
    if exist "%OUT%\configurator" rmdir /s /q "%OUT%\configurator"
    mkdir "%OUT%\configurator"
    if exist "%CONFIGURATOR%\dist" robocopy "%CONFIGURATOR%\dist" "%OUT%\configurator\dist" /E /R:2 /W:1 /NFL /NDL /NP >nul
    if errorlevel 8 exit /b 1
    copy /y "%CONFIGURATOR%\server.mjs" "%OUT%\configurator\server.mjs" >nul
    if errorlevel 1 exit /b 1
    copy /y "%CONFIGURATOR%\package.json" "%OUT%\configurator\package.json" >nul
    if errorlevel 1 exit /b 1
    copy /y "%CONFIGURATOR%\pnpm-lock.yaml" "%OUT%\configurator\pnpm-lock.yaml" >nul
    if errorlevel 1 exit /b 1
    copy /y "%CONFIGURATOR%\pnpm-workspace.yaml" "%OUT%\configurator\pnpm-workspace.yaml" >nul
    if errorlevel 1 exit /b 1
    if exist "%CONFIGURATOR%\README.md" copy /y "%CONFIGURATOR%\README.md" "%OUT%\configurator\README.md" >nul
)

if exist "%ROOT%\bios\" (
    echo [INFO] Copying bios\ ...
    robocopy "%ROOT%\bios" "%OUT%\bios" /E /R:2 /W:1 /NFL /NDL /NP >nul
    if errorlevel 8 exit /b 1
) else (
    echo [INFO] Source folder not found, creating empty bios\
    if not exist "%OUT%\bios" mkdir "%OUT%\bios"
)

if exist "%ROOT%\config\" (
    echo [INFO] Copying config\ ...
    robocopy "%ROOT%\config" "%OUT%\config" /E /R:2 /W:1 /NFL /NDL /NP >nul
    if errorlevel 8 exit /b 1
) else (
    echo [INFO] Source folder not found, creating empty config\
    if not exist "%OUT%\config" mkdir "%OUT%\config"
)

if exist "%MEDIA_ROOT%\" (
    echo [INFO] Copying media\ ...
    robocopy "%MEDIA_ROOT%" "%OUT%\media" /E /R:2 /W:1 /NFL /NDL /NP >nul
    if errorlevel 8 exit /b 1
) else if exist "%MEDIA_RELEASE%\" (
    echo [INFO] Copying target\release\media\ ...
    robocopy "%MEDIA_RELEASE%" "%OUT%\media" /E /R:2 /W:1 /NFL /NDL /NP >nul
    if errorlevel 8 exit /b 1
) else (
    echo [INFO] Source folder not found, creating empty media\
    if not exist "%OUT%\media" mkdir "%OUT%\media"
)

echo [INFO] Creating empty roms\ folder with ROM format readme...
if exist "%OUT%\roms" rmdir /s /q "%OUT%\roms"
mkdir "%OUT%\roms"
(
    echo NGNEON-EMU ROM folder
    echo.
    echo Put your game ROMs in this folder.
    echo.
    echo Supported formats:
    echo   .neo  - Geolith-style single-file Neo Geo ROM images.
    echo   .zip  - Neo Geo MAME/FBNeo-style ZIP sets. Merged parent sets are supported for many games;
    echo           clone subfolders inside merged ZIPs are ignored when the parent set is present.
    echo.
    echo BIOS files go in the bios folder, normally as neogeo.zip or aes.zip.
    echo Cartridge/browser artwork goes in the media folder as PNG files with the same base name as the ROM.
    echo.
    echo Examples:
    echo   roms\aof.neo
    echo   roms\aof.zip
    echo   media\aof.png
) > "%OUT%\roms\readme.txt"
if errorlevel 1 exit /b 1

if exist "%ROOT%\saves\" (
    echo [INFO] Copying saves\ ...
    robocopy "%ROOT%\saves" "%OUT%\saves" /E /R:2 /W:1 /NFL /NDL /NP >nul
    if errorlevel 8 exit /b 1
) else (
    echo [INFO] Source folder not found, creating empty saves\
    if not exist "%OUT%\saves" mkdir "%OUT%\saves"
)

if exist "%ROOT%\README.md" copy /y "%ROOT%\README.md" "%OUT%\README.md" >nul

echo [INFO] Making copied config portable...
set "NGNEON_PACKAGE_CONFIG=%OUT%\config\ngneon.conf"
powershell -NoProfile -ExecutionPolicy Bypass -Command "& { $p = $env:NGNEON_PACKAGE_CONFIG; if (Test-Path -LiteralPath $p) { $lines = @(Get-Content -LiteralPath $p); $map = [ordered]@{ rom_path=''; bios_dir='bios'; media_dir='media'; gamepad='off'; ra_token=''; ra_password=''; ra_username='' }; foreach ($k in $map.Keys) { $pattern = '^' + [regex]::Escape($k) + '='; $value = $k + '=' + $map[$k]; $found = $false; for ($i = 0; $i -lt $lines.Count; $i++) { if ($lines[$i] -match $pattern) { $lines[$i] = $value; $found = $true } }; if (-not $found) { $lines += $value } }; Set-Content -LiteralPath $p -Value $lines -Encoding ASCII } }"
if errorlevel 1 (
    echo [ERROR] Could not update copied config.
    exit /b 1
)

echo.
echo [OK] Package ready: "%OUT%"
echo.
echo Runtime files copied:
echo   ngneon-emu.exe
echo   SDL2.dll
echo   configurar-ngneon.bat
echo   bios\ config\ media\ roms\ saves\ screenshots\ configurator\
echo.
echo Note: roms\ is intentionally packaged empty except for roms\readme.txt.
echo.
echo Build-only files intentionally excluded: *.rlib, *.lib, *.d, *.pdb and helper diagnostic executables.
exit /b 0
