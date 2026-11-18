param(
    [string]$TagVer = "v2.1.0"
)

$ErrorActionPreference = "Continue"
Set-Location $PSScriptRoot

Write-Host "=====================================================================" -ForegroundColor Cyan
Write-Host "  MELODYFY - AUTO-DEPLOY DESKTOP (.exe) & MOBILE ANDROID (.apk) ($TagVer)" -ForegroundColor Green
Write-Host "=====================================================================" -ForegroundColor Cyan

# 1. Verify / Build Desktop .exe
if (-not (Test-Path "Melodyfy-Rust.exe")) {
    Write-Host "`n[1/5] Compilando Melodyfy-Rust.exe..." -ForegroundColor Yellow
    cmd /c build_rust.bat
} else {
    Write-Host "`n[1/5] Melodyfy-Rust.exe listo." -ForegroundColor Green
}

# 2. Build Android Mobile .apk
Write-Host "`n[2/5] Compilando Melodyfy-Mobile.apk (Android Release)..." -ForegroundColor Yellow
Push-Location "melodyfy-mobile"
C:\dev\flutter\bin\flutter.bat build apk --release
Pop-Location
if (Test-Path "melodyfy-mobile\build\app\outputs\flutter-apk\app-release.apk") {
    Copy-Item "melodyfy-mobile\build\app\outputs\flutter-apk\app-release.apk" "Melodyfy-Mobile.apk" -Force
    Write-Host " -> APK compilado: Melodyfy-Mobile.apk" -ForegroundColor Green
} else {
    Write-Host "[ERROR] No se pudo compilar Melodyfy-Mobile.apk" -ForegroundColor Red
    exit 1
}

# 3. Package Windows Portable ZIP
Write-Host "`n[3/5] Empaquetando Melodyfy-Rust-Windows-x64.zip..." -ForegroundColor Yellow
$dist = Join-Path $PSScriptRoot "melodyfy-rust\target\dist_pkg"
if (Test-Path $dist) { Remove-Item $dist -Recurse -Force }
New-Item -ItemType Directory -Force -Path $dist | Out-Null

Copy-Item "Melodyfy-Rust.exe" $dist -Force
if (Test-Path "iniciar_melodyfy_rust.bat") { Copy-Item "iniciar_melodyfy_rust.bat" $dist -Force }
if (Test-Path "yt-dlp.exe") { Copy-Item "yt-dlp.exe" $dist -Force }
Copy-Item "README.md" $dist -Force
if (Test-Path "assets") { Copy-Item "assets" (Join-Path $dist "assets") -Recurse -Force }

$zipPath = Join-Path $PSScriptRoot "Melodyfy-Rust-Windows-x64.zip"
if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
Compress-Archive -Path "$dist\*" -DestinationPath $zipPath -Force
Remove-Item $dist -Recurse -Force

# 4. Git commit & push
Write-Host "`n[4/5] Subiendo codigo fuente (Desktop + Android Mobile) a GitHub (origin/main)..." -ForegroundColor Yellow
git add -A
git commit -m "release($TagVer): Melodyfy Mobile Android (.apk) with ForegroundService MediaSession + Desktop Rust"
git push origin main

# 5. Publish GitHub Release with .apk + .exe + .zip
Write-Host "`n[5/5] Publicando Release $TagVer en GitHub con APK y EXE adjuntos..." -ForegroundColor Yellow
$notes = @"
## 🎧 Melodyfy $TagVer — Desktop Windows (.exe) & Mobile Android (.apk)

### 📱 Melodyfy Mobile (Android `.apk` Nativo):
- **Arquitectura Mobile Real (`melodyfy-mobile/`)**: Servicio de audio en segundo plano (`ForegroundService` + `MediaSession`) con controles en pantalla de bloqueo, barra de notificaciones y Bluetooth.
- **Motor InnerTube HTTP Puro**: Búsqueda y streaming directo desde YouTube sin binarios Python ni `yt-dlp`.
- **Logo Oficial Melodyfy Rust**: Icono de launcher adaptativo Android y branding completo.
- **Todas las funciones**: Playlists personalizadas, Descargas Offline `.m4a` sin DRM, Letras Karaoke LRCLIB, Velocidad DSP (`1.0x`, `1.25x Nightcore`, `1.5x Fast`, `0.85x Slowed`), Radio Infinita y Temporizador de Sueño.

### 🖥️ Melodyfy Desktop (Windows x64):
- **`Melodyfy-Rust.exe`** y **`Melodyfy-Rust-Windows-x64.zip`** (<25 MB RAM, WinRT MediaPlayer).
"@

gh release view $TagVer >$null 2>&1
if ($LASTEXITCODE -eq 0) {
    gh release upload $TagVer "Melodyfy-Mobile.apk" "Melodyfy-Rust.exe" "Melodyfy-Rust-Windows-x64.zip" --clobber
} else {
    gh release create $TagVer "Melodyfy-Mobile.apk" "Melodyfy-Rust.exe" "Melodyfy-Rust-Windows-x64.zip" `
        --title "Melodyfy $TagVer - Android APK & Windows Desktop" `
        --notes $notes
}

Write-Host "`n=====================================================================" -ForegroundColor Cyan
Write-Host "  AUTO-DEPLOY COMPLETADO: APK + EXE PUBLICADOS EN GITHUB ($TagVer)!" -ForegroundColor Green
Write-Host "  https://github.com/UISTALIN2773/Melodify/releases" -ForegroundColor Yellow
Write-Host "=====================================================================" -ForegroundColor Cyan
