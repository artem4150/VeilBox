
; build\installer\veilbox.iss

[Setup]
SourceDir=..\..
AppName=VeilBox
AppVersion=1.0.0
DefaultDirName={autopf}\VeilBox
DefaultGroupName=VeilBox
DisableProgramGroupPage=yes
OutputDir=build\dist
OutputBaseFilename=VeilBoxSetup
Compression=lzma
SolidCompression=yes
SetupIconFile=C:\Users\artem\Desktop\VeilBox\VeilBox\build\windows\icon.ico
UninstallDisplayIcon={app}\VeilBox.exe
WizardStyle=modern
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
PrivilegesRequired=admin

[Files]
; Берём wailsapp.exe, но устанавливаем под именем VeilBox.exe
Source: "bin\wailsapp.exe"; DestDir: "{app}"; DestName: "VeilBox.exe"; Flags: ignoreversion
Source: "core\*"; DestDir: "{app}\core"; Flags: recursesubdirs createallsubdirs ignoreversion

[Dirs]
Name: "{localappdata}\VeilBox"

[Icons]
Name: "{autoprograms}\VeilBox"; Filename: "{app}\VeilBox.exe"; WorkingDir: "{app}"
Name: "{autodesktop}\VeilBox";  Filename: "{app}\VeilBox.exe"; Tasks: desktopicon; WorkingDir: "{app}"

[Tasks]
Name: "desktopicon"; Description: "Создать ярлык на рабочем столе"; GroupDescription: "Ярлыки:"; Flags: unchecked

[Run]
Filename: "{app}\VeilBox.exe"; Description: "Запустить VeilBox"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: filesandordirs; Name: "{app}\core"
