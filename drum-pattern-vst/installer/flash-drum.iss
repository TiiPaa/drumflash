; Flash Drum VST3 — installer script (Inno Setup 6)
;
; Build: iscc installer\flash-drum.iss   (run from drum-pattern-vst/,
; after build.ps1 produced build\drum-pattern-vst.vst3\)
; or: .\build.ps1 -Installer
;
; Version must match drum-pattern-vst/Cargo.toml.

#define AppVersion "0.2.0"

[Setup]
AppName=Flash Drum
AppVersion={#AppVersion}
AppPublisher=Flash Drum
; VST3 system folder: C:\Program Files\Common Files\VST3
; ({commoncf} resolves to the 64-bit Common Files in x64 install mode)
DefaultDirName={commoncf}\VST3\drum-pattern-vst.vst3
DisableDirPage=yes
OutputDir=..\..\dist
OutputBaseFilename=FlashDrum-Setup-{#AppVersion}
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=admin
LicenseFile=..\..\LICENSE
InfoAfterFile=..\..\THIRD-PARTY.md
Compression=lzma2
SolidCompression=yes
UninstallDisplayName=Flash Drum
WizardStyle=modern

[Files]
Source: "..\build\drum-pattern-vst.vst3\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs ignoreversion
