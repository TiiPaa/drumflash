# Build automatique pour Flash Drum VST
#
# Ce script:
# 1. Verifie que Rust est disponible
# 2. Compile le plugin avec le nih-plug vendore
# 3. Regenere le bundle VST3 dans drum-pattern-vst/build/
# 4. Installe le bundle si demande

param(
    [switch]$Install = $false,
    [switch]$Debug = $false,
    # [255] Install elsewhere than the system VST3 folder (CI tests).
    [string]$InstallRoot = "",
    # [258] Skip the test run before -Install (tests run by default).
    [switch]$SkipTests = $false
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
# [258] Keep local absolute paths (workspace, cargo registry) out of the
# shipped binary's panic locations. CARGO_ENCODED_RUSTFLAGS is 0x1F-separated,
# so the space in "Drum Flash" survives (plain RUSTFLAGS is split on spaces).
$rustflagsSep = [char]31
$env:CARGO_ENCODED_RUSTFLAGS = "--remap-path-prefix=$PSScriptRoot=drum-pattern-vst$rustflagsSep--remap-path-prefix=$($env:USERPROFILE)\.cargo\registry\src=cargo-registry$rustflagsSep--remap-path-prefix=$($env:USERPROFILE)\.cargo\git\checkouts=cargo-git"
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

if ($Install -and -not $SkipTests) {
    # [258] Never install a build whose tests fail.
    Write-Color "Yellow" "Tests avant installation (desactivable avec -SkipTests)..."
    cargo test --lib
    if ($LASTEXITCODE -ne 0) {
        Write-Color "Red" "ERREUR: tests en echec, installation annulee (rien n'a ete modifie)."
        exit 1
    }
    Write-Host ""
}

Write-Color "Yellow" "[3/4] Regeneration du bundle VST3..."
New-Item -ItemType Directory -Force -Path $contentDir | Out-Null
Copy-Item -Path $sourceDll -Destination $destFile -Force
Copy-Item -Path $sourceDragHelper -Destination $destDragHelper -Force

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
    if (-not $InstallRoot) { $InstallRoot = "C:\Program Files\Common Files\VST3" }
    $destPath = Join-Path $InstallRoot $vst3File

    if (-not (Test-Path $InstallRoot)) {
        New-Item -ItemType Directory -Force -Path $InstallRoot | Out-Null
    }

    $installedDll = Join-Path $destPath "Contents\x86_64-win\$vst3File"
    if (Test-Path $installedDll) {
        # [250] Fail fast on a locked DLL (Studio One or antivirus holding it)
        # BEFORE touching anything.
        try {
            $fs = [System.IO.File]::Open($installedDll, 'Open', 'ReadWrite', 'None')
            $fs.Close()
        } catch {
            Write-Color "Red" "DLL verrouille (Studio One ouvert ?) : rien n'a ete modifie."
            Write-Host "Ferme Studio One puis relance .\build.ps1 -Install"
            exit 2
        }
    }

    # [255] Atomic install: stage next to the destination, verify the staged
    # copy against the compiled artifacts, then swap by rename. The installed
    # bundle is never half-deleted; a failed swap rolls back.
    $staging = "$destPath.new"
    $old = "$destPath.old"
    Remove-Item $staging, $old -Recurse -Force -ErrorAction SilentlyContinue
    Copy-Item -Path $bundleDir -Destination $staging -Recurse -Force

    $stagedDll = Join-Path $staging "Contents\x86_64-win\$vst3File"
    $stagedHelper = Join-Path $staging "Contents\x86_64-win\drum-pattern-midi-drag-helper.exe"
    $stagedOk = (Test-Path $stagedDll) -and (Test-Path $stagedHelper) -and `
        ((Get-FileHash $stagedDll).Hash -eq (Get-FileHash $sourceDll).Hash) -and `
        ((Get-FileHash $stagedHelper).Hash -eq (Get-FileHash $sourceDragHelper).Hash)
    if (-not $stagedOk) {
        Remove-Item $staging -Recurse -Force -ErrorAction SilentlyContinue
        Write-Color "Red" "ERREUR: la copie stagee est incomplete ou differe des artefacts compiles. Rien n'a ete modifie."
        exit 1
    }

    if (Test-Path $destPath) { Rename-Item $destPath $old }
    try {
        Rename-Item $staging $destPath -ErrorAction Stop
    } catch {
        if ((Test-Path $old) -and -not (Test-Path $destPath)) {
            Rename-Item $old $destPath
        }
        throw
    }
    Remove-Item $old -Recurse -Force -ErrorAction SilentlyContinue
    Write-Color "Green" "Plugin installe dans: $destPath"
} else {
    Write-Color "Yellow" "[4/4] Installation ignoree"
    Write-Host "Relance avec .\build.ps1 -Install pour copier dans le dossier VST3 systeme."
}

# [258] Archive the debug symbols under the build id, AFTER a successful
# install (or a successful bundle without -Install): an archived PDB must
# always match a bundle that actually shipped. The .pdb is NOT shipped inside
# the bundle (it stays out of the VST3), but each build overwrites
# target/<profile>/*.pdb, so without this a crash dump from an older build
# can no longer be symbolised. See task [186]: a stripped-looking stack cost
# an hour. PDBs are kept for 30 days.
$sourcePdb = Join-Path $targetDir "drum_pattern_vst.pdb"
if (Test-Path $sourcePdb) {
    $symbolDir = Join-Path $PSScriptRoot "build\symbols"
    New-Item -ItemType Directory -Force -Path $symbolDir | Out-Null
    Copy-Item -Path $sourcePdb -Destination (Join-Path $symbolDir "drum_pattern_vst-$buildId.pdb") -Force
    Get-ChildItem $symbolDir -Filter "drum_pattern_vst-*.pdb" |
        Where-Object { $_.LastWriteTime -lt (Get-Date).AddDays(-30) } |
        Remove-Item -Force -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Color "Green" "Build termine."
Write-Host "Bundle pret: $bundleDir"
