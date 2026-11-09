; Krabton's Windows installer (Inno Setup 6).
; Build the app first (cargo build --release -p krabton), then:
;   iscc /DMyAppVersion=0.137.0 packaging\windows\krabton.iss
; The installer lands in dist\.

#ifndef MyAppVersion
  #define MyAppVersion "0.137.0"
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
; An update from inside Krabton runs without questions: it starts Krabton again itself.
Filename: "{app}\krabton.exe"; Flags: nowait skipifnotsilent runasoriginaluser

[UninstallDelete]
Type: files; Name: "{app}\packs-choice.txt"

[Code]
var
  PacksPage: TInputOptionWizardPage;
  PackIds: TStringList;

procedure AddPack(Id, Title: String);
begin
  PackIds.Add(Id);
  PacksPage.Add(Title);
end;

procedure InitializeWizard();
begin
  PackIds := TStringList.Create;
  PacksPage := CreateInputOptionPage(wpSelectTasks, 'Sound packs', 'Which sounds should Krabton fetch?',
    'Krabton plays without these. What you tick downloads the first time it starts, while you work. The Packs window in the Krabton menu fetches or removes them any time.',
    False, False);
  AddPack('piano', 'Grand piano (750 MB)');
  AddPack('orchestra', 'Orchestra: strings, winds, brass, percussion, organ (2.3 GB)');
  AddPack('soloists', 'Soloists and band: cello, bass, saxes, guitars, world, electric pianos (1.2 GB)');
  AddPack('drums', 'Drum kits (440 MB)');
  AddPack('soundfonts', 'SoundFonts: General MIDI sets (360 MB)');
  AddPack('voices', 'Voices for the Singer (470 MB)');
  AddPack('tools', 'Princess Sounds, ffmpeg and yt-dlp for films and MP3s (135 MB)');
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  I: Integer;
  Choice: String;
begin
  // An update from inside Krabton is silent and changes nothing about the packs.
  if (CurStep = ssPostInstall) and (not WizardSilent) then
  begin
    Choice := '';
    for I := 0 to PackIds.Count - 1 do
      if PacksPage.Values[I] then
        Choice := Choice + PackIds.Strings[I] + ',';
    SaveStringToFile(ExpandConstant('{app}\packs-choice.txt'), Choice, False);
  end;
end;
