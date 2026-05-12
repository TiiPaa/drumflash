# Script de compatibilite.
# Le chemin d'installation officiel est maintenant .\install.ps1,
# qui appelle drum-pattern-vst\build.ps1 -Install.

$ErrorActionPreference = "Stop"
& (Join-Path $PSScriptRoot "install.ps1")
exit $LASTEXITCODE
