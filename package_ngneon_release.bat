@echo off
setlocal EnableExtensions

set "ROOT=%~dp0"
if "%ROOT:~-1%"=="\" set "ROOT=%ROOT:~0,-1%"
set "OUT=%ROOT%\NGNEON-EMU"
set "EXE=%ROOT%\target\release\ngneon-emu.exe"
set "FRONTEND_EXE=%ROOT%\target\release\ngneon-frontend.exe"
set "SDL_RELEASE=%ROOT%\target\release\SDL2.dll"
set "SDL_ROOT=%ROOT%\SDL2.dll"
set "VCRT_RELEASE=%ROOT%\target\release\VCRUNTIME140.dll"
set "VCRT_ROOT=%ROOT%\VCRUNTIME140.dll"
set "VCRT_SYSTEM=%SystemRoot%\System32\VCRUNTIME140.dll"
set "CONFIG_EXAMPLE=%ROOT%\config\ngneon.example.conf"

if /i "%NGNEON_ALLOW_PACKAGE_CLEAN%"=="1" goto package_guard_done
if not exist "%OUT%" goto package_guard_done

echo [INFO] Checking existing package for user data...
powershell -NoProfile -ExecutionPolicy Bypass -Command "& { $out='%OUT%'; $template='%CONFIG_EXAMPLE%'; $hasUserData=$false; $protected=@('roms','media','saves','screenshots','logs'); foreach($name in $protected) { $dir=Join-Path $out $name; if(Test-Path -LiteralPath $dir) { foreach($file in Get-ChildItem -LiteralPath $dir -File -Recurse) { if($file.FullName -ne (Join-Path $out 'roms\readme.txt')) { $hasUserData=$true } } } }; function Read-Conf([string]$path) { $map=@{}; if(Test-Path -LiteralPath $path) { foreach($line in Get-Content -LiteralPath $path) { if($line -match '^\s*([^#=][^=]*)=(.*)$') { $map[$matches[1].Trim()]=$matches[2].Trim() } } }; return $map }; $expected=Read-Conf $template; $expected['rom_path']='roms'; $expected['bios_dir']='bios'; $expected['media_dir']='media'; $expected['gamepad']='off'; $expected['cpu_overclock']='off'; $expected['sprite_limit']='96'; $expected['disable_adpcm_wrap']='off'; $expected['memcard']='on'; $expected['memcard_write_protect']='off'; $expected['ra_token']=''; $expected['ra_password']=''; $expected['ra_username']=''; $current=Read-Conf (Join-Path $out 'config\ngneon.conf'); foreach($key in $expected.Keys) { if(-not $current.ContainsKey($key) -or $current[$key] -ne $expected[$key]) { $hasUserData=$true } }; if($hasUserData) { exit 3 } }"
if errorlevel 3 (
    echo [ERROR] Existing NGNEON-EMU contains ROMs, media, saves, logs, screenshots, or modified config.
    echo [ERROR] Packaging stopped so those files are not deleted.
    echo [INFO] To intentionally create a clean package, run first:
    echo        set NGNEON_ALLOW_PACKAGE_CLEAN=1
    exit /b 3
)

:package_guard_done

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

if not exist "%EXE%" (
    echo [ERROR] Missing executable: "%EXE%"
    exit /b 1
)

if not exist "%FRONTEND_EXE%" (
    echo [ERROR] Missing native frontend: "%FRONTEND_EXE%"
    exit /b 1
)

if not exist "%CONFIG_EXAMPLE%" (
    echo [ERROR] Missing config template: "%CONFIG_EXAMPLE%"
    exit /b 1
)

if not exist "%OUT%" mkdir "%OUT%"
if exist "%OUT%\screenshots" rmdir /s /q "%OUT%\screenshots"
mkdir "%OUT%\screenshots"

echo [INFO] Cleaning stale generated package folders...
if exist "%OUT%\target" rmdir /s /q "%OUT%\target"
if exist "%OUT%\runtime" rmdir /s /q "%OUT%\runtime"
if exist "%OUT%\configurator" rmdir /s /q "%OUT%\configurator"

echo [INFO] Cleaning build-only files from package folder...
del /q "%OUT%\*.d" "%OUT%\*.rlib" "%OUT%\*.lib" "%OUT%\*.pdb" 2>nul
del /q "%OUT%\compare_rom_banks.exe" "%OUT%\diagnose_rom.exe" "%OUT%\make_test_neo.exe" "%OUT%\probe_rom.exe" "%OUT%\profile_resampler.exe" "%OUT%\render_test_neo.exe" "%OUT%\scan_cmc50_xor.exe" 2>nul

echo [INFO] Copying executable...
copy /y "%EXE%" "%OUT%\ngneon-emu.exe" >nul
if errorlevel 1 exit /b 1

echo [INFO] Copying native frontend...
copy /y "%FRONTEND_EXE%" "%OUT%\ngneon-frontend.exe" >nul
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

echo [INFO] Copying VCRUNTIME140.dll...
if exist "%VCRT_RELEASE%" (
    copy /y "%VCRT_RELEASE%" "%OUT%\VCRUNTIME140.dll" >nul
) else if exist "%VCRT_ROOT%" (
    copy /y "%VCRT_ROOT%" "%OUT%\VCRUNTIME140.dll" >nul
) else if exist "%VCRT_SYSTEM%" (
    copy /y "%VCRT_SYSTEM%" "%OUT%\VCRUNTIME140.dll" >nul
) else (
    echo [ERROR] VCRUNTIME140.dll not found in target\release, project root, or System32.
    exit /b 1
)
if errorlevel 1 exit /b 1

if exist "%ROOT%\configurar-ngneon.bat" (
    echo [INFO] Copying compatibility launcher...
    copy /y "%ROOT%\configurar-ngneon.bat" "%OUT%\configurar-ngneon.bat" >nul
    if errorlevel 1 exit /b 1
)

if exist "%ROOT%\iniciar-ngneon.bat" (
    echo [INFO] Copying native frontend launcher...
    copy /y "%ROOT%\iniciar-ngneon.bat" "%OUT%\iniciar-ngneon.bat" >nul
    if errorlevel 1 exit /b 1
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
    if exist "%OUT%\config" rmdir /s /q "%OUT%\config"
    mkdir "%OUT%\config"
    robocopy "%ROOT%\config" "%OUT%\config" /E /XF ngneon*.conf /R:2 /W:1 /NFL /NDL /NP >nul
    if errorlevel 8 exit /b 1
) else (
    echo [INFO] Source folder not found, creating empty config\
    if not exist "%OUT%\config" mkdir "%OUT%\config"
)

if exist "%ROOT%\themes\" (
    echo [INFO] Copying themes\ without overwriting user themes...
    if not exist "%OUT%\themes" mkdir "%OUT%\themes"
    robocopy "%ROOT%\themes" "%OUT%\themes" /E /XC /XN /XO /R:2 /W:1 /NFL /NDL /NP >nul
    if errorlevel 8 exit /b 1
) else (
    echo [INFO] Source folder not found, creating empty themes\
    if not exist "%OUT%\themes" mkdir "%OUT%\themes"
)

echo [INFO] Creating empty media\ folder...
if exist "%OUT%\media" rmdir /s /q "%OUT%\media"
mkdir "%OUT%\media"

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
    echo Start the native library with iniciar-ngneon.bat.
    echo.
    echo Examples:
    echo   roms\aof.neo
    echo   roms\aof.zip
    echo   media\aof.png
) > "%OUT%\roms\readme.txt"
if errorlevel 1 exit /b 1

echo [INFO] Creating empty saves\ folder...
if exist "%OUT%\saves" rmdir /s /q "%OUT%\saves"
mkdir "%OUT%\saves"

echo [INFO] Creating empty logs\ folder...
if exist "%OUT%\logs" rmdir /s /q "%OUT%\logs"
mkdir "%OUT%\logs"

if exist "%ROOT%\README.md" copy /y "%ROOT%\README.md" "%OUT%\README.md" >nul
if exist "%ROOT%\CHANGELOG.md" copy /y "%ROOT%\CHANGELOG.md" "%OUT%\CHANGELOG.md" >nul

echo [INFO] Creating portable config from ngneon.example.conf...
set "NGNEON_PACKAGE_CONFIG=%OUT%\config\ngneon.conf"
copy /y "%CONFIG_EXAMPLE%" "%NGNEON_PACKAGE_CONFIG%" >nul
if errorlevel 1 (
    echo [ERROR] Could not copy config template.
    exit /b 1
)
powershell -NoProfile -ExecutionPolicy Bypass -Command "& { $p = $env:NGNEON_PACKAGE_CONFIG; $lines = @(Get-Content -LiteralPath $p); $map = [ordered]@{ rom_path='roms'; bios_dir='bios'; media_dir='media'; gamepad='off'; cpu_overclock='off'; sprite_limit='96'; disable_adpcm_wrap='off'; memcard='on'; memcard_write_protect='off'; ra_token=''; ra_password=''; ra_username='' }; foreach ($k in $map.Keys) { $pattern = '^' + [regex]::Escape($k) + '='; $value = $k + '=' + $map[$k]; $found = $false; for ($i = 0; $i -lt $lines.Count; $i++) { if ($lines[$i] -match $pattern) { $lines[$i] = $value; $found = $true } }; if (-not $found) { $lines += $value } }; Set-Content -LiteralPath $p -Value $lines -Encoding ASCII }"
if errorlevel 1 (
    echo [ERROR] Could not update copied config.
    exit /b 1
)

echo.
echo [OK] Package ready: "%OUT%"
echo.
echo Runtime files copied:
echo   ngneon-frontend.exe
echo   ngneon-emu.exe
echo   SDL2.dll
echo   VCRUNTIME140.dll
echo   iniciar-ngneon.bat
echo   configurar-ngneon.bat
echo   bios\ config\ themes\ media\ roms\ saves\ screenshots\ logs\
echo.
echo Note: roms\ is intentionally packaged empty except for roms\readme.txt.
echo Note: media\ and saves\ are intentionally packaged empty.
echo Note: existing themes\ files are preserved; only missing bundled themes are copied.
echo.
echo Build-only files intentionally excluded: *.rlib, *.lib, *.d, *.pdb and helper diagnostic executables.
exit /b 0
