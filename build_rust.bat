@echo off
cd /d "%~dp0\melodyfy-rust"
echo Compilando Melodyfy Rust (Perfil Release Optimizado)...
cargo build --release
if %ERRORLEVEL% EQU 0 (
    echo.
    echo =======================================================
    echo  Compilacion exitosa!
    echo  Ejecutable: melodyfy-rust\target\release\melodyfy-rust.exe
    echo =======================================================
) else (
    echo.
    echo [ERROR] Fallo la compilacion.
)
pause
