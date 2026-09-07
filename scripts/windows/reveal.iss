; Inno Setup script for the Reveal Windows installer.
; Driven by the release workflow, which sets these environment variables:
;   REVEAL_VERSION  three-part version, e.g. 0.3.2
;   REVEAL_EXE      absolute path to the built reveal.exe
;   REVEAL_DIST     absolute path to the directory the installer is written to

#define Version GetEnv("REVEAL_VERSION")
#define ExePath GetEnv("REVEAL_EXE")
#define DistDir GetEnv("REVEAL_DIST")

#if Version == ""
  #define Version "0.0.0"
#endif

[Setup]
AppId={{8F5E9C1A-4B2D-4E7A-9F3C-1A2B3C4D5E6F}
AppName=Reveal
AppVersion={#Version}
AppPublisher=knst0
AppPublisherURL=https://github.com/knst0/reveal
DefaultDirName={autopf}\Reveal
DefaultGroupName=Reveal
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\reveal.exe
LicenseFile=..\..\LICENSE
OutputDir={#DistDir}
OutputBaseFilename=reveal-{#Version}-x86_64-pc-windows-msvc-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
ChangesAssociations=yes

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "addtopath"; Description: "Add Reveal to PATH"; GroupDescription: "Integration:"

[Files]
Source: "{#ExePath}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Reveal"; Filename: "{app}\reveal.exe"
Name: "{group}\{cm:UninstallProgram,Reveal}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Reveal"; Filename: "{app}\reveal.exe"; Tasks: desktopicon

[Registry]
Root: HKA; Subkey: "Software\Classes\Applications\reveal.exe\shell\open\command"; \
  ValueType: string; ValueName: ""; ValueData: """{app}\reveal.exe"" ""%1"""; Flags: uninsdeletekey
Root: HKA; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; \
  ValueData: "{olddata};{app}"; Tasks: addtopath; \
  Check: NeedsAddPath('{app}')

[Run]
Filename: "{app}\reveal.exe"; Description: "{cm:LaunchProgram,Reveal}"; Flags: nowait postinstall skipifsilent

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
  Result := Pos(';' + ExpandConstant(Param) + ';', ';' + OrigPath + ';') = 0;
end;
