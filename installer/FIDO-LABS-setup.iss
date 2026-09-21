; FIDO-LABS v3.0 - Installeur Windows Inno Setup
; Compilé avec Inno Setup 6.x ou Inno Setup Compiler (iscc.exe)

[Setup]
AppName=FIDO-LABS v3
AppVersion=3.0.0
AppPublisher=FIDO-LABS
AppPublisherURL=https://fido-labs.com
AppSupportURL=https://fido-labs.com/support
AppUpdatesURL=https://fido-labs.com/updates
DefaultDirName={autopf}\FIDO-LABS
DefaultGroupName=FIDO-LABS v3
AllowNoIcons=yes
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE.txt
OutputDir=..\dist
OutputBaseFilename=FIDO-LABS-Setup-v3.0.0
Compression=lzma
SolidCompression=yes
SetupIconFile=..\src-tauri\icons\icon.ico
UninstallDisplayIcon={app}\fido-labs.exe
UninstallDisplayName=FIDO-LABS v3.0
SetupLogging=yes

[Languages]
Name: "fr"; MessagesFile: "compiler:Languages\French.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Créer une icône sur le Bureau"; GroupDescription: "Options supplémentaires"; Flags: unchecked
Name: "autostart"; Description: "Lancer au démarrage de Windows"; GroupDescription: "Options de démarrage"

[Dirs]
Name: "{app}\Cases"
Name: "{app}\Exports"
Name: "{app}\Logs"

[Files]
Source: "..\target\release\fido-labs.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\*.dll"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\index.html"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\FIDO-LABS v3"; Filename: "{app}\fido-labs.exe"; WorkingDir: "{app}"; IconFilename: "{app}\fido-labs.exe"
Name: "{group}\FIDO-LABS v3 (Documentation)"; Filename: "{app}\README.md"; WorkingDir: "{app}"
Name: "{group}\Désinstaller FIDO-LABS v3"; Filename: "{uninstallexe}"
Name: "{autodesktop}\FIDO-LABS v3"; Filename: "{app}\fido-labs.exe"; Tasks: desktopicon; WorkingDir: "{app}"

[Run]
Filename: "{app}\fido-labs.exe"; Description: "Lancer FIDO-LABS maintenant"; Flags: nowait postinstall skipifsuspen

[UninstallDelete]
Type: files; Name: "{app}\Logs\*.*"
Type: filesandallsubdirs; Name: "{app}\Cases\*.*"

[Registry]
Root: HKCU; Subkey: "Software\FIDO-LABS"; ValueType: string; ValueName: "InstallPath"; ValueData: "{app}"; Tasks: autostart
Root: HKCU; Subkey: "Software\FIDO-LABS"; ValueType: string; ValueName: "Version"; ValueData: "3.0.0"

[Code]
procedure InitializeWizard();
begin
  // Page de bienvenue personnalisée avec information éthique
  MsgBox('FIDO-LABS v3.0 - Outil forensique numérique' + #13#10 +
         '' + #13#10 +
         '⚠️ AVERTISSEMENT ÉTHIQUE :' + #13#10 +
         'Cet outil ne doit être utilisé que par des investigateurs numériques' + #13#10 +
         'en possession d''une autorisation judiciaire valide.' + #13#10 +
         'Toute utilisation non autorisée est illégale.',
         mbInformation, MB_OK);
end;

function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;
  if CurPageID = wpReady then begin
    // Afficher le chemin d'installation choisi
    Log('FIDO-LABS Installation: ' + ExpandConstant('{app}'));
    Log('Architecture: ' + GetArchitectureName);
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then begin
    Log('Désinstallation de FIDO-LABS v3.0 en cours...');
  end;
end;
