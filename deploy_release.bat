@echo off
setlocal EnableDelayedExpansion
cd /d "%~dp0"

echo =====================================================================
echo  MELODYFY - BUILD, GIT PUSH ^& GITHUB RELEASE (EXE + APK)
echo =====================================================================

set "TAG_VER=%~1"
if "%TAG_VER%"=="" set "TAG_VER=v2.1.0"

:: 1. Configurar entorno MSVC si existe Visual Studio Insiders
if exist "C:\Program Files\Microsoft Visual Studio\18\Insiders\VC\Auxiliary\Build\vcvars64.bat" (
    call "C:\Program Files\Microsoft Visual Studio\18\Insiders\VC\Auxiliary\Build\vcvars64.bat" >nul
)
if exist "C:\Program Files (x86)\Windows Kits\10\Include\10.0.28000.0\km\crt" (
    set "INCLUDE=C:\Program Files (x86)\Windows Kits\10\Include\10.0.28000.0\km\crt;%~dp0melodyfy-rust\target\include;%INCLUDE%;C:\Program Files (x86)\Windows Kits\10\Include\10.0.28000.0\um;C:\Program Files (x86)\Windows Kits\10\Include\10.0.28000.0\shared"
    set "LIB=%LIB%;%~dp0melodyfy-rust\target\libs;C:\Program Files (x86)\Windows Kits\10\Lib\10.0.28000.0\um\x64;C:\Program Files (x86)\Windows Kits\10\Lib\10.0.28000.0\km\x64"
)

:: 2. Compilar Desktop Rust (.exe con logo.ico incrustado)
echo.
echo [1/5] Compilando Melodyfy-Rust.exe...
pushd melodyfy-rust
cargo build --release
if errorlevel 1 (
    popd
    echo [ERROR] Fallo la compilacion Desktop Rust.
    exit /b 1
)
popd
copy /Y "melodyfy-rust\target\release\melodyfy-rust.exe" "Melodyfy-Rust.exe" >nul

:: 3. Compilar Android Mobile (.apk)
echo.
echo [2/5] Compilando Melodyfy-Mobile.apk...
pushd melodyfy-mobile
call flutter build apk --release
if errorlevel 1 (
    popd
    echo [ERROR] Fallo la compilacion Android APK.
    exit /b 1
)
popd
copy /Y "melodyfy-mobile\build\app\outputs\flutter-apk\app-release.apk" "Melodyfy-Mobile.apk" >nul

:: 4. Empaquetar ZIP portable usando tar.exe nativo de Windows (sin PowerShell)
echo.
echo [3/5] Empaquetando Melodyfy-Rust-Windows-x64.zip...
set "DIST_DIR=melodyfy-rust\target\dist_pkg"
if exist "%DIST_DIR%" rmdir /s /q "%DIST_DIR%"
mkdir "%DIST_DIR%\assets"
copy /Y "Melodyfy-Rust.exe" "%DIST_DIR%\" >nul
if exist "yt-dlp.exe" copy /Y "yt-dlp.exe" "%DIST_DIR%\" >nul
copy /Y "README.md" "%DIST_DIR%\" >nul
if exist "assets\logo.jpg" copy /Y "assets\logo.jpg" "%DIST_DIR%\assets\" >nul
if exist "assets\logo.ico" copy /Y "assets\logo.ico" "%DIST_DIR%\assets\" >nul

if exist "Melodyfy-Rust-Windows-x64.zip" del /f /q "Melodyfy-Rust-Windows-x64.zip"
pushd "%DIST_DIR%"
tar -a -c -f "..\..\..\Melodyfy-Rust-Windows-x64.zip" *
popd
rmdir /s /q "%DIST_DIR%"

:: 5. Subir cambios a GitHub
echo.
echo [4/5] Subiendo codigo a GitHub (origin/main)...
git add -A
git commit -m "release(%TAG_VER%): Melodyfy Rust Desktop (.exe) & Android Mobile (.apk)"
git push origin main

:: 6. Publicar o actualizar GitHub Release con APK + EXE + ZIP
echo.
echo [5/5] Publicando Release %TAG_VER% en GitHub...
gh release view "%TAG_VER%" >nul 2>&1
if !ERRORLEVEL! EQU 0 (
    gh release upload "%TAG_VER%" "Melodyfy-Mobile.apk" "Melodyfy-Rust.exe" "Melodyfy-Rust-Windows-x64.zip" --clobber
) else (
    gh release create "%TAG_VER%" "Melodyfy-Mobile.apk" "Melodyfy-Rust.exe" "Melodyfy-Rust-Windows-x64.zip" --title "Melodyfy %TAG_VER% - Android APK & Windows Desktop" --notes "Release automatico con Melodyfy-Mobile.apk (Android) y Melodyfy-Rust.exe (Windows Desktop)."
)

echo.
echo =====================================================================
echo  DEPLOY COMPLETADO: https://github.com/UISTALIN2773/Melodify/releases
echo =====================================================================
