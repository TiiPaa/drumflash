# Installation rapide du plugin Flash Drum
#
# Usage: .\install.ps1
# Delegue au build officiel du plugin pour appliquer le patch Studio One,
# compiler, regenerer le bundle et installer le VST3 systeme.

$ErrorActionPreference = "Stop"

$pluginDir = Join-Path $PSScriptRoot "drum-pattern-vst"
$buildScript = Join-Path $pluginDir "build.ps1"

if (-not (Test-Path $buildScript)) {
    Write-Host "ERREUR: build.ps1 introuvable: $buildScript" -ForegroundColor Red
    exit 1
}

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "  Flash Drum - Build/Install" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""

Push-Location $pluginDir
try {
    & $buildScript -Install
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
} finally {
    Pop-Location
}

Write-Host ""
Write-Host "Installation terminee." -ForegroundColor Green
Write-Host "Dans Studio One: rescan VST3, charge le plugin, verifie le build affiche dans l'UI." -ForegroundColor Cyan
