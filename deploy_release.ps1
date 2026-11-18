param(
    [string]$TagVer = "v2.0.0"
)

$ErrorActionPreference = "Continue"
Set-Location $PSScriptRoot

Write-Host "=====================================================================" -ForegroundColor Cyan
Write-Host "  MELODYFY RUST - AUTO-DEPLOY & GITHUB RELEASE PUBLISHER ($TagVer)" -ForegroundColor Green
Write-Host "=====================================================================" -ForegroundColor Cyan

# 1. Verify or build binaries
if (-not (Test-Path "Melodyfy-Rust.exe")) {
    Write-Host "`n[1/4] Compilando ejecutables Release optimizados..." -ForegroundColor Yellow
    cmd /c build_rust.bat
} else {
    Write-Host "`n[1/4] Ejecutables Release listos (Melodyfy-Rust.exe y Melodyfy-Mobile.exe)." -ForegroundColor Green
}

if (-not (Test-Path "Melodyfy-Rust.exe")) {
    Write-Host "[ERROR] No se encontro Melodyfy-Rust.exe" -ForegroundColor Red
    exit 1
}

# 2. Package portable ZIP
Write-Host "`n[2/4] Empaquetando Melodyfy-Rust-Windows-x64.zip..." -ForegroundColor Yellow
$dist = Join-Path $PSScriptRoot "melodyfy-rust\target\dist_pkg"
if (Test-Path $dist) { Remove-Item $dist -Recurse -Force }
New-Item -ItemType Directory -Force -Path $dist | Out-Null

Copy-Item "Melodyfy-Rust.exe" $dist -Force
Copy-Item "Melodyfy-Mobile.exe" $dist -Force
if (Test-Path "iniciar_melodyfy_rust.bat") { Copy-Item "iniciar_melodyfy_rust.bat" $dist -Force }
if (Test-Path "yt-dlp.exe") { Copy-Item "yt-dlp.exe" $dist -Force }
Copy-Item "README.md" $dist -Force
if (Test-Path "assets") { Copy-Item "assets" (Join-Path $dist "assets") -Recurse -Force }

$zipPath = Join-Path $PSScriptRoot "Melodyfy-Rust-Windows-x64.zip"
if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
Compress-Archive -Path "$dist\*" -DestinationPath $zipPath -Force
Remove-Item $dist -Recurse -Force
Write-Host " -> Paquete creado: Melodyfy-Rust-Windows-x64.zip" -ForegroundColor Green

# 3. Git commit & push
Write-Host "`n[3/4] Subiendo codigo fuente y workflow CI/CD a GitHub (origin/main)..." -ForegroundColor Yellow
git add -A
git commit -m "release($TagVer): Melodyfy Rust - Official Logo, Playlists, Offline Downloads, DSP Speed, Radio & Auto-Release"
git push origin main

# 4. Publish GitHub Release with binaries attached
Write-Host "`n[4/4] Publicando Release $TagVer en GitHub con ejecutables adjuntos..." -ForegroundColor Yellow
$notes = @"
## 🎧 Melodyfy Rust $TagVer — Reproductor Nativo Libre, Rapido y Sin Anuncios (<25 MB RAM)

### ✨ Novedades incluidas en esta version:
- 🎨 **Logo Oficial Melodyfy Rust** integrado en icono de ventana, barra de tareas, vistas de Escritorio, Mini-Player, Karaoke y Movil 9:16.
- 🚀 **Motor Hibrido InnerTube en Rust Puro + yt-dlp**: Busqueda instantanea (~200 ms) y cache de streams con pre-carga automatica (0 ms).
- 📂 **Gestor Completo de Playlists Personalizadas**: Crea tus propias listas y anade canciones con un clic.
- 💾 **Modo Descargas Offline (Sin DRM)**: Descarga canciones en .m4a y escuchalas sin internet.
- 🎤 **Modo Karaoke Sincronizado (LRCLIB)**: Letras en tiempo real con visualizador de espectro de 24 barras y ajuste de sincronizacion (-0.5s / +0.5s).
- ⚡ **Efectos de Velocidad DSP**: 1.0x Normal, 1.25x Nightcore, 1.5x Fast y 0.85x Slowed Chill.
- 📻 **Radio Infinita (Auto-Mix)** y **🌙 Temporizador de Sueno (Sleep Timer)**.

### 📥 Archivos de Descarga Directa:
- **Melodyfy-Rust-Windows-x64.zip**: Paquete portable completo (incluye version Desktop, version Mobile 9:16 y motor yt-dlp.exe).
- **Melodyfy-Rust.exe**: Ejecutable directo de Escritorio.
- **Melodyfy-Mobile.exe**: Ejecutable directo en formato Smartphone (9:16).
"@

gh release view $TagVer >$null 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "Actualizando Release $TagVer existente..." -ForegroundColor Cyan
    gh release upload $TagVer "Melodyfy-Rust.exe" "Melodyfy-Mobile.exe" "Melodyfy-Rust-Windows-x64.zip" --clobber
} else {
    Write-Host "Creando nuevo Release $TagVer en GitHub..." -ForegroundColor Cyan
    gh release create $TagVer "Melodyfy-Rust.exe" "Melodyfy-Mobile.exe" "Melodyfy-Rust-Windows-x64.zip" `
        --title "Melodyfy Rust $TagVer - Reproductor Nativo Libre, Rapido y Sin Anuncios" `
        --notes $notes
}

Write-Host "`n=====================================================================" -ForegroundColor Cyan
Write-Host "  AUTO-DEPLOY Y GITHUB RELEASE ($TagVer) COMPLETADOS CON EXITO!" -ForegroundColor Green
Write-Host "  https://github.com/UISTALIN2773/Melodify/releases" -ForegroundColor Yellow
Write-Host "=====================================================================" -ForegroundColor Cyan
