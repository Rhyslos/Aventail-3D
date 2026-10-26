; The Windows setup program (decision 0169), built with Inno Setup by
; installer/windows/build-setup.ps1, which passes every name in from
; crates/app/src/branding.rs and crates/app/Cargo.toml (RENAMING.md):
;   AppName, AppId, AppVersion, ExeName, Extension, SourceDir, OutputDir,
;   ReleasesUrl, IconFile (the logo, assets/icon.ico, decision 0171).
;
; It installs for the person running it only (no admin password), into
; a folder they choose on the first install (their own Programs folder
; suggested; any folder they may write to, on any drive), with a Start
; menu entry, an optional desktop
; icon, an entry in Windows' installed apps (to uninstall), and project
; files opening in the app. Run quietly by an update, it closes the app if
; it is still open, and opens it again afterwards unless given /NOLAUNCH.

#ifndef AppName
  #error Build this with installer/windows/build-setup.ps1
#endif

[Setup]
AppId={#AppId}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisherURL={#ReleasesUrl}
AppUpdatesURL={#ReleasesUrl}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
PrivilegesRequired=lowest
DisableProgramGroupPage=yes
; The folder is asked on the first install (decision 0170); updates and
; reinstalls go back into the folder chosen then.
DisableDirPage=auto
UsePreviousAppDir=yes
DisableReadyPage=no
DisableWelcomePage=yes
OutputDir={#OutputDir}
OutputBaseFilename={#AppId}-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\{#ExeName}
SetupIconFile={#IconFile}
UninstallDisplayName={#AppName}
CloseApplications=yes
RestartApplications=no
ChangesAssociations=yes

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\{#ExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\NOTICE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\licenses\*"; DestDir: "{app}\licenses"; Flags: ignoreversion recursesubdirs

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#ExeName}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#ExeName}"; Tasks: desktopicon

[Registry]
; Double-clicking a project opens it in the app (for this person only).
Root: HKA; Subkey: "Software\Classes\.{#Extension}"; ValueType: string; ValueName: ""; ValueData: "{#AppId}.project"; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\{#AppId}.project"; ValueType: string; ValueName: ""; ValueData: "{#AppName} project"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\{#AppId}.project\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#ExeName},0"
Root: HKA; Subkey: "Software\Classes\{#AppId}.project\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#ExeName}"" ""%1"""

[Run]
; Offered on the last page; run quietly too (an update), unless /NOLAUNCH.
Filename: "{app}\{#ExeName}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall; Check: ShouldLaunch

[Code]
{ Whether to open the app after installing: not when the app asked for a
  quiet install at its next close (Later), with /NOLAUNCH. }
function ShouldLaunch: Boolean;
var
  I: Integer;
begin
  Result := True;
  for I := 1 to ParamCount do
    if CompareText(ParamStr(I), '/NOLAUNCH') = 0 then
      Result := False;
end;
