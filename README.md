<p align="center">
  <img src="docs/branding/logo.png" alt="VeilBox" width="120" />
</p>

<h1 align="center">VeilBox — лёгкий VPN-клиент для Windows (Wails + sing-box)</h1>

<p align="center">
  <a href="https://go.dev/"><img alt="Go" src="https://img.shields.io/badge/Go-1.22%2B-00ADD8?logo=go"></a>
  <a href="https://wails.io/"><img alt="Wails" src="https://img.shields.io/badge/Wails-2.x-8A2BE2"></a>
  <img alt="Windows" src="https://img.shields.io/badge/Windows-10%2B-0078D6?logo=windows">
  <img alt="License" src="https://img.shields.io/badge/license-MIT-green">
</p>

> <strong>Коротко:</strong> GUI на Wails, ядро — sing-box., кэш и логи лежат в профиле пользователя: <code>%LOCALAPPDATA%\VeilBox</code>. Инсталлятор — Inno Setup.

---

## 📸 Скриншоты



<p align="center">
  <img src="docs/screenshots/main.png" alt="Главное окно" width="840">
</p>

<p align="center">
  <img src="docs/screenshots/tray.png" alt="Иконка в трее и меню" width="420">
</p>

---

## ✨ Возможности

* VLESS Reality gRPC (proxy/tun) с готовыми шаблонами конфигов
* Запуск ядра <strong>без консольного окна</strong> (CREATE_NO_WINDOW)
* Работа <strong>без прав админа</strong>: кэш/логи в <code>%LOCALAPPDATA%\VeilBox</code>
* Трей-иконка, кнопки Connect/Disconnect, вывод логов в UI
* Установщик Inno Setup, чистая деинсталляция

---

## 🗂 Структура

```
VeilBox/
├─ app.go
├─ runner.go                # скрытый запуск sing-box, рабочая dir = %LOCALAPPDATA%\VeilBox
├─ logs.go                  # RingBuffer для логов (использует app.go)
├─ tray_windows.go
├─ proxy_windows.go
├─ main.go
├─ core/                    # ядро: sing-box.exe, wintun.dll
├─ embed_templates/
│  ├─ vless_reality_grpc_proxy.json
│  └─ vless_reality_grpc_tun.json
├─ build/
│  ├─ windows/              # ассеты инсталлятора (иконки/картинки)
│  └─ installer/
│     └─ veilbox.iss        # скрипт Inno Setup (SourceDir=..\..)
└─ docs/
   ├─ branding/logo.png
   └─ screenshots/
      ├─ main.png
      └─ tray.png
```

---

## 🧰 Требования

* <strong>Go</strong> 1.22+
* <strong>Node.js</strong> 18+ (для фронта Wails)
* <strong>Microsoft Build Tools (MSVC)</strong>
* <strong>WebView2 Runtime</strong> (режим download в <code>wails.json</code> уже включён)
* <strong>Inno Setup</strong> (для сборки инсталлятора)

Проверка окружения:

```powershell
wails doctor
```

---

## 🛠 Сборка приложения

```powershell
# из корня репозитория
wails build -clean

# результат: build/bin/VeilBox.exe (или wailsapp.exe — см. ниже раздел Инсталлятор)
```

> Если бинарник называется <code>wailsapp.exe</code>, можно либо переименовать его в <code>VeilBox.exe</code>,
> либо оставить как есть — в инсталляторе предусмотрен <code>DestName: VeilBox.exe</code>.

---

## 📦 Сборка установщика (Inno Setup)

Скрипт: <code>build/installer/veilbox.iss</code>

Важные моменты:

* В начале файла установлена базовая директория:

  ```ini
  SourceDir=..\..
  ```
* Иконка берётся из: <code>build\windows\icon.ico</code>
* Папка данных пользователя будет создана: <code>{localappdata}\VeilBox</code>
* Ядро копируется в <code>{app}\core</code>

Пример ключевых секций:

```ini
[Setup]
SourceDir=..\..
SetupIconFile=build\windows\icon.ico
OutputDir=build\dist
OutputBaseFilename=VeilBoxSetup
PrivilegesRequired=admin

[Files]
; Вариант A: если после сборки есть build\bin\VeilBox.exe
Source: "build\bin\VeilBox.exe"; DestDir: "{app}"; Flags: ignoreversion

; Вариант B: если билд даёт wailsapp.exe — устанавливаем как VeilBox.exe
; Source: "build\bin\wailsapp.exe"; DestDir: "{app}"; DestName: "VeilBox.exe"; Flags: ignoreversion

; Ядро
Source: "core\*"; DestDir: "{app}\core"; Flags: recursesubdirs createallsubdirs ignoreversion

[Dirs]
Name: "{localappdata}\VeilBox"

[Icons]
Name: "{autoprograms}\VeilBox"; Filename: "{app}\VeilBox.exe"
Name: "{autodesktop}\VeilBox";  Filename: "{app}\VeilBox.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\VeilBox.exe"; Description: "Запустить VeilBox"; Flags: nowait postinstall skipifsilent
```

Сборка:

1. Открой <code>build/installer/veilbox.iss</code> в <strong>Inno Setup Compiler</strong>
2. <strong>Compile</strong> → <code>build/dist/VeilBoxSetup.exe</code>

---

## ⚙️ Как это работает под капотом

* <code>runner.go</code> сохраняет активный конфиг в:

  ```
  %LOCALAPPDATA%\VeilBox\sb_config.json
  ```
* Запускает <code>{app}\core\sing-box.exe</code> с рабочей директорией:

  ```
  %LOCALAPPDATA%\VeilBox
  ```

  Благодаря этому <code>cache.db</code> и логи пишутся туда же, и не нужны права администратора.
* Процесс запускается с флагами <code>CREATE_NO_WINDOW</code> + <code>HideWindow</code>, поэтому <strong>чёрного окна нет</strong>.
* Логи ядра читаются пайпами и прокидываются в UI; в проекте есть <code>RingBuffer</code> для последних N строк.

---

## 🧪 Режим разработки

```powershell
wails dev
```

Полная сборка:

```powershell
wails build -clean
```

---

## 🧯 Траблшутинг

**<code>FATAL ... open cache.db: Access is denied</code>**
Кэш создавался в <code>C:\Program Files...</code>. В новой версии рабочая папка ядра — <code>%LOCALAPPDATA%\VeilBox</code>.
Проверь, что используешь актуальный <code>runner.go</code>.
Путь к кэшу можно также явно указать в JSON:

```json
"experimental": { "cache_file": "%LOCALAPPDATA%\\VeilBox\\cache.db" }
```

**Появляется чёрное консольное окно**
Убедись, что sing-box запускается именно кодом из <code>runner.go</code>, где установлены: <code>SysProcAttr{ HideWindow: true, CreationFlags: CREATE_NO_WINDOW }</code>.

**Ярлык в Пуск/на рабочем столе не работает**
Проверь, что в инсталляторе конечный бинарь называется <code>VeilBox.exe</code>.
Если билд даёт <code>wailsapp.exe</code>, используй опцию:

```ini
DestName: "VeilBox.exe"
```

---

## 🧾 Лицензия

MIT — см. файл <code>LICENSE</code>.

---

## 🙌 Благодарности

* <a href="https://wails.io/">Wails</a> — за отличный фреймворк для гибридных десктоп-приложений
* <a href="https://sing-box.sagernet.org/">sing-box</a> — за мощное VPN-ядро
