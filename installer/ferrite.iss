; Ferrite — Windows Inno Setup installer (optional alternative to MSI)
;
; Builds a per-machine installer from the release binary with Start Menu shortcut,
; optional file associations (OpenWithProgids, matching wix/main.wxs), Explorer
; context menu entries, and optional PATH integration.
;
; Manual build (from repo root, after cargo build --release):
;   powershell -File installer\build.ps1
;   — or —
;   "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" /DMyAppVersion=0.3.0 installer\ferrite.iss
;
; Output: installer\Output\ferrite-windows-x64-setup.exe

#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif

#define MyAppName "Ferrite"
#define MyAppPublisher "OlaProeis"
#define MyAppURL "https://github.com/OlaProeis/Ferrite"
#define MyAppExeName "ferrite.exe"
#define MyAppId "{{F3BB1E73-ED17-4000-A000-000000000002}"

[Setup]
AppId={#MyAppId}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf64}\Ferrite
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
OutputDir=Output
OutputBaseFilename=ferrite-windows-x64-setup
SetupIconFile=..\assets\icons\windows\app.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName}
VersionInfoVersion={#MyAppVersion}.0
VersionInfoCompany={#MyAppPublisher}
VersionInfoDescription={#MyAppName} Setup
VersionInfoProductName={#MyAppName}
VersionInfoProductVersion={#MyAppVersion}
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
CloseApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Additional shortcuts:"; Flags: unchecked
Name: "assocmd"; Description: "Markdown (.md, .markdown)"; GroupDescription: "File associations (Open With / Default Apps):"; Flags: checked
Name: "assoctxt"; Description: "Plain text (.txt)"; GroupDescription: "File associations (Open With / Default Apps):"; Flags: checked
Name: "assocjson"; Description: "JSON (.json)"; GroupDescription: "File associations (Open With / Default Apps):"; Flags: checked
Name: "assocyaml"; Description: "YAML (.yaml, .yml)"; GroupDescription: "File associations (Open With / Default Apps):"; Flags: checked
Name: "assoctoml"; Description: "TOML (.toml)"; GroupDescription: "File associations (Open With / Default Apps):"; Flags: checked
Name: "assoccsv"; Description: "CSV (.csv, .tsv)"; GroupDescription: "File associations (Open With / Default Apps):"; Flags: checked
Name: "contextmenufiles"; Description: "&Open with Ferrite (any file)"; GroupDescription: "Explorer context menu:"; Flags: unchecked
Name: "contextmenufolders"; Description: "Open &Folder with Ferrite (directories)"; GroupDescription: "Explorer context menu:"; Flags: unchecked
Name: "addtopath"; Description: "Add Ferrite to the system &PATH"; GroupDescription: "Other:"; Flags: unchecked

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; Tasks: desktopicon

[Registry]
; --- Markdown ---
Root: HKLM; Subkey: "Software\Classes\Ferrite.md"; ValueType: string; ValueData: "Markdown Document"; Flags: uninsdeletekey; Tasks: assocmd
Root: HKLM; Subkey: "Software\Classes\Ferrite.md\DefaultIcon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: assocmd
Root: HKLM; Subkey: "Software\Classes\Ferrite.md\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: assocmd
Root: HKLM; Subkey: "Software\Classes\.md\OpenWithProgids"; ValueType: string; ValueName: "Ferrite.md"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocmd
Root: HKLM; Subkey: "Software\Classes\.markdown\OpenWithProgids"; ValueType: string; ValueName: "Ferrite.md"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocmd

; --- Plain text ---
Root: HKLM; Subkey: "Software\Classes\Ferrite.txt"; ValueType: string; ValueData: "Text Document"; Flags: uninsdeletekey; Tasks: assoctxt
Root: HKLM; Subkey: "Software\Classes\Ferrite.txt\DefaultIcon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: assoctxt
Root: HKLM; Subkey: "Software\Classes\Ferrite.txt\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: assoctxt
Root: HKLM; Subkey: "Software\Classes\.txt\OpenWithProgids"; ValueType: string; ValueName: "Ferrite.txt"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoctxt

; --- JSON ---
Root: HKLM; Subkey: "Software\Classes\Ferrite.json"; ValueType: string; ValueData: "JSON Document"; Flags: uninsdeletekey; Tasks: assocjson
Root: HKLM; Subkey: "Software\Classes\Ferrite.json\DefaultIcon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: assocjson
Root: HKLM; Subkey: "Software\Classes\Ferrite.json\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: assocjson
Root: HKLM; Subkey: "Software\Classes\.json\OpenWithProgids"; ValueType: string; ValueName: "Ferrite.json"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocjson

; --- YAML ---
Root: HKLM; Subkey: "Software\Classes\Ferrite.yaml"; ValueType: string; ValueData: "YAML Document"; Flags: uninsdeletekey; Tasks: assocyaml
Root: HKLM; Subkey: "Software\Classes\Ferrite.yaml\DefaultIcon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: assocyaml
Root: HKLM; Subkey: "Software\Classes\Ferrite.yaml\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: assocyaml
Root: HKLM; Subkey: "Software\Classes\.yaml\OpenWithProgids"; ValueType: string; ValueName: "Ferrite.yaml"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocyaml
Root: HKLM; Subkey: "Software\Classes\.yml\OpenWithProgids"; ValueType: string; ValueName: "Ferrite.yaml"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocyaml

; --- TOML ---
Root: HKLM; Subkey: "Software\Classes\Ferrite.toml"; ValueType: string; ValueData: "TOML Document"; Flags: uninsdeletekey; Tasks: assoctoml
Root: HKLM; Subkey: "Software\Classes\Ferrite.toml\DefaultIcon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: assoctoml
Root: HKLM; Subkey: "Software\Classes\Ferrite.toml\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: assoctoml
Root: HKLM; Subkey: "Software\Classes\.toml\OpenWithProgids"; ValueType: string; ValueName: "Ferrite.toml"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoctoml

; --- CSV ---
Root: HKLM; Subkey: "Software\Classes\Ferrite.csv"; ValueType: string; ValueData: "CSV Document"; Flags: uninsdeletekey; Tasks: assoccsv
Root: HKLM; Subkey: "Software\Classes\Ferrite.csv\DefaultIcon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: assoccsv
Root: HKLM; Subkey: "Software\Classes\Ferrite.csv\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: assoccsv
Root: HKLM; Subkey: "Software\Classes\.csv\OpenWithProgids"; ValueType: string; ValueName: "Ferrite.csv"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoccsv
Root: HKLM; Subkey: "Software\Classes\.tsv\OpenWithProgids"; ValueType: string; ValueName: "Ferrite.csv"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoccsv

; --- Windows Default Apps registration (per selected extension) ---
Root: HKLM; Subkey: "Software\OlaProeis\Ferrite\Capabilities"; ValueName: "ApplicationDescription"; ValueType: string; ValueData: "A fast, lightweight text editor for Markdown, JSON, and more"; Flags: uninsdeletekey; Check: AnyAssocTaskSelected
Root: HKLM; Subkey: "Software\OlaProeis\Ferrite\Capabilities"; ValueName: "ApplicationName"; ValueType: string; ValueData: "{#MyAppName}"; Check: AnyAssocTaskSelected
Root: HKLM; Subkey: "Software\OlaProeis\Ferrite\Capabilities\FileAssociations"; ValueName: ".md"; ValueType: string; ValueData: "Ferrite.md"; Flags: uninsdeletevalue; Tasks: assocmd
Root: HKLM; Subkey: "Software\OlaProeis\Ferrite\Capabilities\FileAssociations"; ValueName: ".markdown"; ValueType: string; ValueData: "Ferrite.md"; Flags: uninsdeletevalue; Tasks: assocmd
Root: HKLM; Subkey: "Software\OlaProeis\Ferrite\Capabilities\FileAssociations"; ValueName: ".txt"; ValueType: string; ValueData: "Ferrite.txt"; Flags: uninsdeletevalue; Tasks: assoctxt
Root: HKLM; Subkey: "Software\OlaProeis\Ferrite\Capabilities\FileAssociations"; ValueName: ".json"; ValueType: string; ValueData: "Ferrite.json"; Flags: uninsdeletevalue; Tasks: assocjson
Root: HKLM; Subkey: "Software\OlaProeis\Ferrite\Capabilities\FileAssociations"; ValueName: ".yaml"; ValueType: string; ValueData: "Ferrite.yaml"; Flags: uninsdeletevalue; Tasks: assocyaml
Root: HKLM; Subkey: "Software\OlaProeis\Ferrite\Capabilities\FileAssociations"; ValueName: ".yml"; ValueType: string; ValueData: "Ferrite.yaml"; Flags: uninsdeletevalue; Tasks: assocyaml
Root: HKLM; Subkey: "Software\OlaProeis\Ferrite\Capabilities\FileAssociations"; ValueName: ".toml"; ValueType: string; ValueData: "Ferrite.toml"; Flags: uninsdeletevalue; Tasks: assoctoml
Root: HKLM; Subkey: "Software\OlaProeis\Ferrite\Capabilities\FileAssociations"; ValueName: ".csv"; ValueType: string; ValueData: "Ferrite.csv"; Flags: uninsdeletevalue; Tasks: assoccsv
Root: HKLM; Subkey: "Software\OlaProeis\Ferrite\Capabilities\FileAssociations"; ValueName: ".tsv"; ValueType: string; ValueData: "Ferrite.csv"; Flags: uninsdeletevalue; Tasks: assoccsv
Root: HKLM; Subkey: "Software\RegisteredApplications"; ValueName: "Ferrite"; ValueType: string; ValueData: "Software\OlaProeis\Ferrite\Capabilities"; Flags: uninsdeletevalue; Check: AnyAssocTaskSelected

; --- Explorer context menu ---
Root: HKLM; Subkey: "Software\Classes\*\shell\OpenWithFerrite"; ValueType: string; ValueData: "Open with Ferrite"; Flags: uninsdeletekey; Tasks: contextmenufiles
Root: HKLM; Subkey: "Software\Classes\*\shell\OpenWithFerrite"; ValueName: "Icon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: contextmenufiles
Root: HKLM; Subkey: "Software\Classes\*\shell\OpenWithFerrite\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: contextmenufiles
Root: HKLM; Subkey: "Software\Classes\Directory\shell\OpenWithFerrite"; ValueType: string; ValueData: "Open Folder with Ferrite"; Flags: uninsdeletekey; Tasks: contextmenufolders
Root: HKLM; Subkey: "Software\Classes\Directory\shell\OpenWithFerrite"; ValueName: "Icon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: contextmenufolders
Root: HKLM; Subkey: "Software\Classes\Directory\shell\OpenWithFerrite\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: contextmenufolders
Root: HKLM; Subkey: "Software\Classes\Directory\Background\shell\OpenWithFerrite"; ValueType: string; ValueData: "Open Folder with Ferrite"; Flags: uninsdeletekey; Tasks: contextmenufolders
Root: HKLM; Subkey: "Software\Classes\Directory\Background\shell\OpenWithFerrite"; ValueName: "Icon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: contextmenufolders
Root: HKLM; Subkey: "Software\Classes\Directory\Background\shell\OpenWithFerrite\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%V"""; Tasks: contextmenufolders

; --- Install marker (Start Menu shortcut key path analogue) ---
Root: HKCU; Subkey: "Software\OlaProeis\Ferrite"; ValueName: "installed"; ValueType: dword; ValueData: "1"; Flags: uninsdeletekey

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch {#MyAppName}"; Flags: nowait postinstall skipifsilent unchecked

[Code]
function AnyAssocTaskSelected: Boolean;
begin
  Result :=
    IsTaskSelected('assocmd') or
    IsTaskSelected('assoctxt') or
    IsTaskSelected('assocjson') or
    IsTaskSelected('assocyaml') or
    IsTaskSelected('assoctoml') or
    IsTaskSelected('assoccsv');
end;

function AddToPath(Param: string): string;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(
    HKEY_LOCAL_MACHINE,
    'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
    'Path',
    OrigPath)
  then
    Result := Param
  else
    Result := OrigPath + ';' + Param;
end;

function RemoveFromPath(Param: string): string;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(
    HKEY_LOCAL_MACHINE,
    'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
    'Path',
    OrigPath)
  then
    Exit;

  if Pos(';' + Param + ';', ';' + OrigPath + ';') > 0 then
    StringChangeEx(OrigPath, ';' + Param, '', True)
  else if Pos(Param + ';', OrigPath) = 1 then
    StringChangeEx(OrigPath, Param + ';', '', True)
  else if Copy(OrigPath, Length(OrigPath) - Length(Param), Length(Param) + 1) = ';' + Param then
    StringChangeEx(OrigPath, ';' + Param, '', False);

  Result := OrigPath;
end;

function NeedsAddPath(Param: string): Boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(
    HKEY_LOCAL_MACHINE,
    'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
    'Path',
    OrigPath)
  then
    Result := True
  else
    Result := Pos(';' + Param + ';', ';' + OrigPath + ';') = 0;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    if IsTaskSelected('addtopath') and NeedsAddPath(ExpandConstant('{app}')) then
    begin
      RegWriteExpandStringValue(
        HKEY_LOCAL_MACHINE,
        'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
        'Path',
        AddToPath(ExpandConstant('{app}')));
      RegWriteDWordValue(
        HKEY_LOCAL_MACHINE,
        'Software\OlaProeis\Ferrite',
        'PathAdded',
        1);
    end;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  AppPath: string;
  PathAdded: Cardinal;
begin
  if CurUninstallStep = usPostUninstall then
  begin
    if RegQueryDWordValue(
      HKEY_LOCAL_MACHINE,
      'Software\OlaProeis\Ferrite',
      'PathAdded',
      PathAdded) and (PathAdded = 1) then
    begin
      AppPath := ExpandConstant('{app}');
      if RegValueExists(
        HKEY_LOCAL_MACHINE,
        'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
        'Path') then
      begin
        RegWriteExpandStringValue(
          HKEY_LOCAL_MACHINE,
          'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
          'Path',
          RemoveFromPath(AppPath));
      end;
      RegDeleteValue(HKEY_LOCAL_MACHINE, 'Software\OlaProeis\Ferrite', 'PathAdded');
    end;
  end;
end;
