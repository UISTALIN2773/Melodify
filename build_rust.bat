@echo off
if exist "C:\Program Files\Microsoft Visual Studio\18\Insiders\VC\Auxiliary\Build\vcvars64.bat" (
    call "C:\Program Files\Microsoft Visual Studio\18\Insiders\VC\Auxiliary\Build\vcvars64.bat" >nul
)
set "INCLUDE=C:\Program Files (x86)\Windows Kits\10\Include\10.0.28000.0\km\crt;%~dp0melodyfy-rust\target\include;%INCLUDE%;C:\Program Files (x86)\Windows Kits\10\Include\10.0.28000.0\um;C:\Program Files (x86)\Windows Kits\10\Include\10.0.28000.0\shared"
set "LIB=%LIB%;%~dp0melodyfy-rust\target\libs;C:\Program Files (x86)\Windows Kits\10\Lib\10.0.28000.0\um\x64;C:\Program Files (x86)\Windows Kits\10\Lib\10.0.28000.0\km\x64"
cd /d "%~dp0\melodyfy-rust"
echo Compilando Melodyfy Rust (Perfil Release Optimizado Full Rust)...
cargo build --release
if %ERRORLEVEL% EQU 0 (
    echo Copiando ejecutables a la raiz...
    copy /Y "target\release\melodyfy-rust.exe" "..\Melodyfy-Rust.exe"
    copy /Y "target\release\melodyfy-rust.exe" "..\Melodyfy-Mobile.exe"
    echo.
    echo =======================================================
    echo  Compilacion exitosa Full Rust!
    echo  Ejecutables listos:
    echo  - Melodyfy-Rust.exe ^(Desktop^)
    echo  - Melodyfy-Mobile.exe ^(Mobile 9:16^)
    echo =======================================================
) else (
    echo.
    echo [ERROR] Fallo la compilacion.
)
