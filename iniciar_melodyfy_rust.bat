@echo off
cd /d "%~dp0"
if exist "melodyfy-rust\target\release\melodyfy-rust.exe" (
    start "" "melodyfy-rust\target\release\melodyfy-rust.exe"
) else (
    echo [ERROR] No se encontro el ejecutable compilado en release.
    echo Compilando ahora con cargo...
    cd melodyfy-rust
    cargo build --release
    cd ..
    start "" "melodyfy-rust\target\release\melodyfy-rust.exe"
)
exit
