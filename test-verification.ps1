# Verification rapide du plugin Flash Drum

$ErrorActionPreference = "Stop"

$bundlePath = Join-Path $PSScriptRoot "drum-pattern-vst\build\drum-pattern-vst.vst3"
$bundleBinary = Join-Path $bundlePath "Contents\x86_64-win\drum-pattern-vst.vst3"
$installedBinary = "C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3\Contents\x86_64-win\drum-pattern-vst.vst3"
$sourceLib = Join-Path $PSScriptRoot "drum-pattern-vst\src\lib.rs"
$expectedClass = "DrumFlashPlugin1"

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "  Flash Drum - Verification" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

$ok = $true

function Check-Path($label, $path) {
    if (Test-Path $path) {
        Write-Host "OK: $label" -ForegroundColor Green
        return $true
    }

    Write-Host "ERREUR: $label introuvable: $path" -ForegroundColor Red
    return $false
}

$ok = (Check-Path "bundle local" $bundlePath) -and $ok
$ok = (Check-Path "binaire bundle local" $bundleBinary) -and $ok
$ok = (Check-Path "binaire installe" $installedBinary) -and $ok

if (Test-Path $installedBinary) {
    $info = Get-Item $installedBinary
    $hash = Get-FileHash $installedBinary
    Write-Host "Binaire installe: $($info.LastWriteTime)" -ForegroundColor Gray
    Write-Host "SHA-256: $($hash.Hash)" -ForegroundColor Gray

    if (Test-Path $bundleBinary) {
        $bundleHash = Get-FileHash $bundleBinary
        if ($hash.Hash -eq $bundleHash.Hash) {
            Write-Host "OK: binaire installe identique au bundle local" -ForegroundColor Green
        } else {
            Write-Host "ATTENTION: le bundle local et le binaire installe different. Relance build.ps1 -Install." -ForegroundColor Yellow
        }
    } else {
        Write-Host "INFO: bundle local introuvable, comparaison impossible." -ForegroundColor Gray
    }
}

if (Test-Path $sourceLib) {
    $sourceText = Get-Content -Path $sourceLib -Raw
    if ($sourceText.Contains($expectedClass)) {
        Write-Host "OK: class ID source attendu ($expectedClass)" -ForegroundColor Green
    } else {
        Write-Host "ATTENTION: class ID source attendu non trouve ($expectedClass)." -ForegroundColor Yellow
    }
}

Write-Host ""
if ($ok) {
    Write-Host "Verification fichier OK. Test manuel Studio One requis pour le multi-out." -ForegroundColor Green
} else {
    Write-Host "Verification echouee. Lance: .\install.ps1" -ForegroundColor Red
    exit 1
}
