# setup_dependencies.ps1 - Melodyfy Dependencies Setup
# Descarga yt-dlp.exe si no se encuentra en el directorio raíz del proyecto.

[CmdletBinding()]
param()

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $ScriptDir) {
    $ScriptDir = Get-Location
}

$YtDlpPath = Join-Path $ScriptDir "yt-dlp.exe"
$DownloadUrl = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"

Write-Host "=============================================" -ForegroundColor Cyan
Write-Host "   Melodyfy - Verificación de Dependencias   " -ForegroundColor Cyan
Write-Host "=============================================" -ForegroundColor Cyan

if (Test-Path -LiteralPath $YtDlpPath) {
    $fileSizeMB = [math]::Round(((Get-Item -LiteralPath $YtDlpPath).Length / 1MB), 2)
    Write-Host "[OK] yt-dlp.exe ya existe en la raíz ($fileSizeMB MB): $YtDlpPath" -ForegroundColor Green
    exit 0
}

Write-Host "[INFO] yt-dlp.exe no encontrado en: $YtDlpPath" -ForegroundColor Yellow
Write-Host "[INFO] Descargando desde release oficial de GitHub..." -ForegroundColor Yellow
Write-Host "URL: $DownloadUrl" -ForegroundColor Gray

try {
    # Intentar descargar con curl.exe si está disponible en el sistema
    $curlCmd = Get-Command "curl.exe" -ErrorAction SilentlyContinue
    if ($curlCmd) {
        Write-Host "[INFO] Usando curl.exe para la descarga..." -ForegroundColor Cyan
        & curl.exe -L --fail --show-error --progress-bar -o "$YtDlpPath" "$DownloadUrl"
    } else {
        Write-Host "[INFO] Usando Invoke-WebRequest para la descarga..." -ForegroundColor Cyan
        $ProgressPreference = 'SilentlyContinue' # Acelera la descarga en PowerShell 5.1
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $YtDlpPath -UseBasicParsing
    }

    if (Test-Path -LiteralPath $YtDlpPath) {
        $size = (Get-Item -LiteralPath $YtDlpPath).Length
        if ($size -gt 1048576) { # Al menos 1MB
            $fileSizeMB = [math]::Round(($size / 1MB), 2)
            Write-Host "[ÉXITO] yt-dlp.exe descargado correctamente ($fileSizeMB MB)." -ForegroundColor Green
        } else {
            Remove-Item -LiteralPath $YtDlpPath -Force -ErrorAction SilentlyContinue
            throw "El archivo descargado está incompleto o corrupto (tamaño: $size bytes)."
        }
    } else {
        throw "No se pudo generar el archivo yt-dlp.exe en el destino."
    }
} catch {
    Write-Host "[ERROR] Falló la descarga de yt-dlp.exe: $_" -ForegroundColor Red
    exit 1
}

Write-Host "[LISTO] Todas las dependencias están configuradas." -ForegroundColor Green
exit 0
