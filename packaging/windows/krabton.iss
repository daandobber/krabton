; Krabton's Windows installer (Inno Setup 6).
; Build the app first (cargo build --release -p krabton), then:
;   iscc /DMyAppVersion=0.67.0 packaging\windows\krabton.iss
; The installer lands in dist\.

#ifndef MyAppVersion
  #define MyAppVersion "0.67.0"
#endif

[Setup]
AppId={{8A3C1F2E-5B7D-4E2A-9C61-0F3B7E5D2A41}
AppName=Krabton
AppVersion={#MyAppVersion}
AppVerName=Krabton {#MyAppVersion}
AppPublisher=Krabton contributors
AppPublisherURL=https://github.com/daandobber/krabton
AppSupportURL=https://github.com/daandobber/krabton/issues
DefaultDirName={autopf}\Krabton
DefaultGroupName=Krabton
DisableProgramGroupPage=yes
; Installs for the current user without asking for admin rights; the
; dialog offers an install for everyone.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=..\..\dist
OutputBaseFilename=Krabton-Setup-{#MyAppVersion}
SetupIconFile=..\..\crates\krabton\assets\icon.ico
UninstallDisplayIcon={app}\krabton.ico
UninstallDisplayName=Krabton
LicenseFile=..\..\COPYING
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
ChangesAssociations=yes

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "krabfiles"; Description: "Open .krab sets with Krabton"; GroupDescription: "File types:"

[Files]
Source: "..\..\target\release\krabton.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\crates\krabton\assets\icon.ico"; DestDir: "{app}"; DestName: "krabton.ico"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion
Source: "..\..\COPYING"; DestDir: "{app}"; DestName: "COPYING.txt"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Krabton"; Filename: "{app}\krabton.exe"; IconFilename: "{app}\krabton.ico"; Comment: "Make music"
Name: "{autodesktop}\Krabton"; Filename: "{app}\krabton.exe"; IconFilename: "{app}\krabton.ico"; Tasks: desktopicon

[Registry]
Root: HKA; Subkey: "Software\Classes\.krab"; ValueType: string; ValueName: ""; ValueData: "Krabton.Set"; Flags: uninsdeletevalue; Tasks: krabfiles
Root: HKA; Subkey: "Software\Classes\Krabton.Set"; ValueType: string; ValueName: ""; ValueData: "Krabton Set"; Flags: uninsdeletekey; Tasks: krabfiles
Root: HKA; Subkey: "Software\Classes\Krabton.Set\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\krabton.ico"; Tasks: krabfiles
Root: HKA; Subkey: "Software\Classes\Krabton.Set\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\krabton.exe"" ""%1"""; Tasks: krabfiles

[Run]
Filename: "{app}\krabton.exe"; Description: "{cm:LaunchProgram,Krabton}"; Flags: nowait postinstall skipifsilent
