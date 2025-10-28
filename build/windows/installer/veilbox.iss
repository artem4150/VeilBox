; build\installer\veilbox.iss

[Setup]
AppName=VeilBox
AppVersion=1.0.0
DefaultDirName={autopf}\VeilBox
DefaultGroupName=VeilBox
DisableProgramGroupPage=yes
OutputDir=build\dist
OutputBaseFilename=VeilBoxSetup
Compression=lzma
SolidCompression=yes
SetupIconFile=build\windows\icon.ico
UninstallDisplayIcon={app}\VeilBox.exe
WizardStyle=modern
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
; Если не нужен UAC — можно поставить: PrivilegesRequired=lowest
PrivilegesRequired=admin

[Files]
; Главный exe (убедиcь, что он свежесобранный: wails build → build\bin\VeilBox.exe)
; Если VeilBox.exe лежит в корне — меняем Source соответственно.
Source: "VeilBox.exe"; DestDir: "{app}"; Flags: ignoreversion
; Вся папка core рекурсивно
Source: "core\*"; DestDir: "{app}\core"; Flags: recursesubdirs createallsubdirs ignoreversion

; Если нужны иконки/ресурсы — добавляй похожими строками:
; Source: "build\windows\installer\header.bmp"; DestDir: "{app}\assets"; Flags: ignoreversion
; Source: "build\windows\installer\sidebar.bmp"; DestDir: "{app}\assets"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\VeilBox"; Filename: "{app}\VeilBox.exe"; WorkingDir: "{app}"
Name: "{autodesktop}\VeilBox"; Filename: "{app}\VeilBox.exe"; Tasks: desktopicon; WorkingDir: "{app}"

[Tasks]
Name: "desktopicon"; Description: "Создать ярлык на рабочем столе"; GroupDescription: "Ярлыки:"; Flags: unchecked

[Run]
; Автозапуск после установки
Filename: "{app}\VeilBox.exe"; Description: "Запустить VeilBox"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; На всякий случай удаляем папку core при деинсталляции
Type: filesandordirs; Name: "{app}\core"
