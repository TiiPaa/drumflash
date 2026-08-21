# Build automatique pour Flash Drum VST
#
# Ce script:
# 1. Verifie que Rust est disponible
# 2. Compile le plugin avec le nih-plug vendore
# 3. Regenere le bundle VST3 dans drum-pattern-vst/build/
# 4. Installe le bundle si demande

param(
    [switch]$Install = $false,
    [switch]$Debug = $false
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false

$pluginName = "drum-pattern-vst"
$vst3File = "$pluginName.vst3"
$targetDir = if ($Debug) { Join-Path $PSScriptRoot "target\debug" } else { Join-Path $PSScriptRoot "target\release" }
$bundleRoot = Join-Path $PSScriptRoot "build"
$bundleDir = Join-Path $bundleRoot $vst3File
$contentDir = Join-Path $bundleDir "Contents\x86_64-win"
$sourceDll = Join-Path $targetDir "drum_pattern_vst.dll"
$sourceDragHelper = Join-Path $targetDir "drum-pattern-midi-drag-helper.exe"
$destFile = Join-Path $contentDir $vst3File
$destDragHelper = Join-Path $contentDir "drum-pattern-midi-drag-helper.exe"
$tempDir = Join-Path $PSScriptRoot ".codex-tmp"
$buildId = Get-Date -Format "yyyyMMdd-HHmmss"

function Write-Color($color, $message) {
    Write-Host $message -ForegroundColor $color
}

Write-Color "Cyan" "========================================="
Write-Color "Cyan" "  Flash Drum VST - Build"
Write-Color "Cyan" "========================================="
Write-Host ""

Write-Color "Yellow" "[1/4] Verification de l'installation Rust..."
$rustCheck = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $rustCheck) {
    Write-Color "Red" "ERREUR: Rust n'est pas installe."
    Write-Host "Installe-le depuis: https://rustup.rs/"
    exit 1
}
Write-Color "Green" "Rust trouve: $(cargo --version)"
Write-Host ""

Write-Color "Yellow" "[2/4] Compilation du plugin..."
New-Item -ItemType Directory -Force -Path $tempDir | Out-Null
$env:TEMP = $tempDir
$env:TMP = $tempDir
$env:DRUM_PATTERN_BUILD_ID = $buildId
Write-Host "Build ID: $buildId"

if ($Debug) {
    cargo build
} else {
    cargo build --release
}

if ($LASTEXITCODE -ne 0) {
    Write-Color "Red" "ERREUR: Compilation echouee."
    exit 1
}

if (-not (Test-Path $sourceDll)) {
    Write-Color "Red" "ERREUR: DLL plugin introuvable: $sourceDll"
    exit 1
}

if (-not (Test-Path $sourceDragHelper)) {
    Write-Color "Red" "ERREUR: Helper drag MIDI introuvable: $sourceDragHelper"
    exit 1
}

Write-Color "Green" "Compilation reussie."
Write-Host ""

Write-Color "Yellow" "[3/4] Regeneration du bundle VST3..."
New-Item -ItemType Directory -Force -Path $contentDir | Out-Null
Copy-Item -Path $sourceDll -Destination $destFile -Force
Copy-Item -Path $sourceDragHelper -Destination $destDragHelper -Force

# Archive the debug symbols under the build id. The .pdb is NOT shipped inside
# the bundle (it stays out of the VST3), but each build overwrites
# target/release/*.pdb, so without this a crash dump from an older build can no
# longer be symbolised. See task [186]: a stripped-looking stack cost an hour.
$symbolDir = Join-Path $PSScriptRoot "build/symbols"
$sourcePdb = Join-Path $PSScriptRoot "target/release/drum_pattern_vst.pdb"
if (Test-Path $sourcePdb) {
    New-Item -ItemType Directory -Force -Path $symbolDir | Out-Null
    Copy-Item -Path $sourcePdb -Destination (Join-Path $symbolDir "drum_pattern_vst-$buildId.pdb") -Force
    # Keep the 10 most recent so the folder does not grow without bound.
    Get-ChildItem $symbolDir -Filter "drum_pattern_vst-*.pdb" |
        Sort-Object LastWriteTime -Descending |
        Select-Object -Skip 10 |
        Remove-Item -Force -ErrorAction SilentlyContinue
}

$dllInfo = Get-Item $sourceDll
$bundleInfo = Get-Item $destFile
$helperInfo = Get-Item $destDragHelper

Write-Color "Green" "Bundle VST3 mis a jour."
Write-Host "DLL source  : $($dllInfo.LastWriteTime)"
Write-Host "Bundle VST3 : $($bundleInfo.LastWriteTime)"
Write-Host "Drag helper : $($helperInfo.LastWriteTime)"
Write-Host ""

if ($Install) {
    Write-Color "Yellow" "[4/4] Installation du plugin..."
    $vst3Path = "C:\Program Files\Common Files\VST3"
    $destPath = Join-Path $vst3Path $vst3File

    if (-not (Test-Path $vst3Path)) {
        New-Item -ItemType Directory -Force -Path $vst3Path | Out-Null
    }

    if (Test-Path $destPath) {
        Remove-Item -Path $destPath -Recurse -Force
    }

    Copy-Item -Path $bundleDir -Destination $destPath -Recurse -Force
    Write-Color "Green" "Plugin installe dans: $destPath"
} else {
    Write-Color "Yellow" "[4/4] Installation ignoree"
    Write-Host "Relance avec .\build.ps1 -Install pour copier dans le dossier VST3 systeme."
}

Write-Host ""
Write-Color "Green" "Build termine."
Write-Host "Bundle pret: $bundleDir"
