<#
.SYNOPSIS
    Script de actualizacion automatica para yt-dlp.exe en Melodyfy.

.DESCRIPTION
    Descarga la version mas reciente de yt-dlp.exe desde GitHub Releases
    (https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe)
    y reemplaza de forma segura el ejecutable local.
#>

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

# Directorio base del script
$scriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
if (-not $scriptDir) { $scriptDir = (Get-Location).Path }

$targetExe   = Join-Path $scriptDir "yt-dlp.exe"
$backupExe   = Join-Path $scriptDir "yt-dlp.exe.bak"
$tempExe     = Join-Path $scriptDir "yt-dlp-temp.exe"
$downloadUrl = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "   Actualizador de yt-dlp - Melodyfy     " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# 1. Detectar version actual
$currentVersion = "No instalado"
if (Test-Path -LiteralPath $targetExe) {
    try {
        $currentVersion = & $targetExe --version 2>$null
    } catch {
        $currentVersion = "Desconocida / Inaccesible"
    }
}
Write-Host "[*] Version actual: $currentVersion" -ForegroundColor Yellow

# 2. Configurar protocolos de red TLS 1.2
try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
} catch {
    Write-Host "[!] Advertencia: no se pudo forzar TLS 1.2 explicitamente." -ForegroundColor Yellow
}

# 3. Limpiar temporal previo si existe
if (Test-Path -LiteralPath $tempExe) {
    Remove-Item -LiteralPath $tempExe -Force -ErrorAction SilentlyContinue
}

# 4. Descargar archivo
Write-Host "[*] Descargando la version mas reciente desde GitHub..." -ForegroundColor Green
Write-Host "    URL: $downloadUrl" -ForegroundColor DarkGray

try {
    $webClient = New-Object System.Net.WebClient
    $webClient.Headers.Add("User-Agent", "Melodyfy-Updater/1.0")
    $webClient.DownloadFile($downloadUrl, $tempExe)
    $webClient.Dispose()

    # 5. Validar tamano (> 5 MB)
    if (-not (Test-Path -LiteralPath $tempExe)) {
        throw "El archivo temporal descargado no existe."
    }

    $fileSize = (Get-Item -LiteralPath $tempExe).Length
    if ($fileSize -lt 5MB) {
        throw "El archivo descargado es demasiado pequeno ($fileSize bytes). Es probable que la descarga haya fallado."
    }

    # 6. Validar ejecutable
    Write-Host "[*] Verificando integridad del archivo descargado..." -ForegroundColor Cyan
    $newVersion = & $tempExe --version 2>$null
    if (-not $newVersion) {
        throw "El ejecutable descargado no responde correctamente al comando --version."
    }
    Write-Host "[+] Nueva version obtenida: $newVersion" -ForegroundColor Green

    # 7. Backup y reemplazo atomico
    if (Test-Path -LiteralPath $targetExe) {
        if (Test-Path -LiteralPath $backupExe) {
            Remove-Item -LiteralPath $backupExe -Force -ErrorAction SilentlyContinue
        }
        Move-Item -LiteralPath $targetExe -Destination $backupExe -Force
    }

    Move-Item -LiteralPath $tempExe -Destination $targetExe -Force

    # 8. Limpiar backup si todo fue exitoso
    if (Test-Path -LiteralPath $backupExe) {
        Remove-Item -LiteralPath $backupExe -Force -ErrorAction SilentlyContinue
    }

    Write-Host ""
    Write-Host "[OK] yt-dlp.exe se actualizo correctamente a la version: $newVersion" -ForegroundColor Green
    Write-Host "==========================================" -ForegroundColor Cyan
}
catch {
    Write-Host ""
    Write-Host "[ERROR] Ocurrio un error durante la actualizacion: $_" -ForegroundColor Red

    # Restaurar backup si es necesario
    if ((-not (Test-Path -LiteralPath $targetExe)) -and (Test-Path -LiteralPath $backupExe)) {
        Write-Host "[*] Restaurando copia de seguridad previa..." -ForegroundColor Yellow
        Move-Item -LiteralPath $backupExe -Destination $targetExe -Force
    }

    if (Test-Path -LiteralPath $tempExe) {
        Remove-Item -LiteralPath $tempExe -Force -ErrorAction SilentlyContinue
    }

    exit 1
}
