#define MyAppName "Fluffy Injector"
#ifndef MyAppVersion
#error "Pass /DMyAppVersion=x.y.z from Cargo.toml"
#endif
#define MyAppPublisher "fluffysnaff"
#define MyAppURL "https://github.com/fluffysnaff/fluffy-injector"
#define MyAppExeName "fluffy_injector.exe"

[Setup]
AppId={{E8C4A1B2-7D3F-4A91-9B6E-5C2D8F0A4E17}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile=LICENSE
OutputDir=dist
OutputBaseFilename=FluffyInjector-{#MyAppVersion}-setup
SetupIconFile=assets\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
ChangesEnvironment=yes
CloseApplications=yes
SourceDir=..
MinVersion=10.0

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "installer\Install-FluffyInjector.ps1"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; Tasks: desktopicon

[Registry]
Root: HKA; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Flags: preservestringtype; Check: NeedsAddPath(ExpandConstant('{app}'))

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
function NeedsAddPath(Param: string): Boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKA, 'Environment', 'Path', OrigPath) then
  begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Uppercase(Param) + ';', ';' + Uppercase(OrigPath) + ';') = 0;
end;

procedure RemoveAppPath(const AppDir: string);
var
  OrigPath: string;
  Wrapped: string;
  Position: Integer;
begin
  if not RegQueryStringValue(HKA, 'Environment', 'Path', OrigPath) then
    Exit;
  Wrapped := ';' + OrigPath + ';';
  Position := Pos(';' + AppDir + ';', Wrapped);
  if Position = 0 then
    Exit;
  Delete(Wrapped, Position + 1, Length(AppDir) + 1);
  if Length(Wrapped) >= 2 then
    OrigPath := Copy(Wrapped, 2, Length(Wrapped) - 2)
  else
    OrigPath := '';
  RegWriteExpandStringValue(HKA, 'Environment', 'Path', OrigPath);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
    RemoveAppPath(ExpandConstant('{app}'));
end;
