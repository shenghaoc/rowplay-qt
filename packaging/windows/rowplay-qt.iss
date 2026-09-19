; SPDX-License-Identifier: GPL-3.0-or-later
;
; Inno Setup script for the Windows x64 installer (Phase 9, ADR 0012).
; Compiled by tools/package/windows.ps1 with:
;   ISCC.exe /DAppVersion=<cargo version> /DStageDir=<windeployqt output> /DOutDir=<dist> rowplay-qt.iss
; The staging directory holds rowplay-qt.exe plus everything windeployqt put
; next to it (Qt DLLs, plugins, QML modules, the VC++ runtime installer).

#ifndef AppVersion
  #error Pass /DAppVersion=x.y.z /DStageDir=<dir> /DOutDir=<dir> (tools/package/windows.ps1 does)
#endif

[Setup]
; Fixed per application, never per version: it is how Windows tells an
; upgrade from a second program. Generated once with uuidgen.
AppId={{EAABF5B7-1D6F-40A2-A67F-B66FCFB1F775}
AppName=rowplay
AppVersion={#AppVersion}
AppVerName=rowplay {#AppVersion}
AppPublisher=shenghaoc
AppPublisherURL=https://github.com/shenghaoc/rowplay-qt
AppSupportURL=https://github.com/shenghaoc/rowplay-qt/issues
AppUpdatesURL=https://github.com/shenghaoc/rowplay-qt/releases
AppCopyright=GPL-3.0-or-later. Not affiliated with Concept2.
DefaultDirName={autopf}\rowplay-qt
DefaultGroupName=rowplay
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\rowplay-qt.exe
UninstallDisplayName=rowplay
LicenseFile={#StageDir}\LICENSE
OutputDir={#OutDir}
OutputBaseFilename=rowplay-qt-{#AppVersion}-windows-x86_64-setup
SetupIconFile={#SourcePath}\..\..\assets\icon\rowplay-qt.ico
Compression=lzma2
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Per-user by default (no elevation prompt); the dialog lets an admin pick
; an all-users install.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
WizardStyle=modern

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#StageDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\rowplay"; Filename: "{app}\rowplay-qt.exe"; IconFilename: "{app}\rowplay-qt.ico"
Name: "{autodesktop}\rowplay"; Filename: "{app}\rowplay-qt.exe"; IconFilename: "{app}\rowplay-qt.ico"; Tasks: desktopicon

[Run]
#ifexist StageDir + "\vc_redist.x64.exe"
; windeployqt --compiler-runtime stages the Microsoft VC++ runtime installer;
; the Rust and Qt binaries link it dynamically.
Filename: "{app}\vc_redist.x64.exe"; Parameters: "/install /quiet /norestart"; StatusMsg: "Installing the Microsoft Visual C++ runtime..."; Flags: waituntilterminated
#endif
Filename: "{app}\rowplay-qt.exe"; Description: "{cm:LaunchProgram,rowplay}"; Flags: nowait postinstall skipifsilent
