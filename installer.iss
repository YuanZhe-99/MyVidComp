; Inno Setup script for MyVidComp on Windows.
;
; The files come from the staged package folder, so build that first:
;   scripts/package-gui-windows.ps1 -Platform x64 -DownloadFfmpeg
;   scripts/package-gui-windows-installer.ps1 -Platform x64
;
; Compiling it by hand takes the same two switches the script passes:
;   iscc /DAppVersion=0.1.4 installer.iss                 (x64)
;   iscc /DAppVersion=0.1.4 /DARM64 installer.iss         (ARM64)
;
; AppId is this application's own GUID and must never change: Windows uses it to
; recognise an upgrade, and a new one would install a second copy alongside the
; first rather than replacing it.
;
; The version is passed in rather than written here. It already lives in
; Cargo.toml, gui/pubspec.yaml and gui/windows/runner/Runner.rc, and the release
; workflow checks the tag against Cargo.toml, so a fourth hand-edited copy would
; only be one more thing to forget.

#ifndef AppVersion
  #error Pass the version in, for example: iscc /DAppVersion=0.1.4 installer.iss
#endif

[Setup]
AppId={{78F9D601-311C-4038-B446-23D80DED8A7D}
AppName=MyVidComp
AppVersion={#AppVersion}
AppPublisher=yuanzhe
AppPublisherURL=https://github.com/YuanZhe-99/MyVidComp
DefaultDirName={autopf}\MyVidComp
DefaultGroupName=MyVidComp
UninstallDisplayIcon={app}\MyVidComp.exe
OutputDir=dist
#ifdef ARM64
OutputBaseFilename=MyVidComp_{#AppVersion}_arm64_Setup
#else
OutputBaseFilename=MyVidComp_{#AppVersion}_Setup
#endif
VersionInfoVersion={#AppVersion}.0
VersionInfoCompany=MyVidComp
VersionInfoDescription=MyVidComp Installer
VersionInfoProductName=MyVidComp
VersionInfoProductVersion={#AppVersion}
Compression=lzma2
SolidCompression=yes
#ifdef ARM64
ArchitecturesAllowed=arm64
ArchitecturesInstallIn64BitMode=arm64
#else
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
#endif
WizardStyle=modern
SetupIconFile=gui\windows\runner\resources\app_icon.ico
; Installs per user, so no administrator prompt. The application writes only to
; its own settings directory and to the video folder the user chooses.
PrivilegesRequired=lowest

[Languages]
; English only. Inno Setup ships a fixed set of translations under Languages\,
; and Chinese is not among them: it is a community file downloaded separately,
; which a clean CI runner does not have.
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
; The staged package, not the raw Flutter output: the media tools, the READMEs
; and the checksum file are added at staging time, so this is what makes the
; installer carry the same tree as the zip.
#ifdef ARM64
Source: "dist\MyVidComp-windows-arm64\*"; DestDir: "{app}"; Flags: recursesubdirs ignoreversion
#else
Source: "dist\MyVidComp-windows-x64\*"; DestDir: "{app}"; Flags: recursesubdirs ignoreversion
#endif

[Icons]
Name: "{group}\MyVidComp"; Filename: "{app}\MyVidComp.exe"
Name: "{group}\{cm:UninstallProgram,MyVidComp}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\MyVidComp"; Filename: "{app}\MyVidComp.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\MyVidComp.exe"; Description: "{cm:LaunchProgram,MyVidComp}"; Flags: nowait postinstall skipifsilent
