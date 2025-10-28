@echo off
setlocal enableextensions

rem Чистим и создаём build\bin\core
if exist build\bin\core rmdir /s /q build\bin\core
mkdir build\bin\core

rem Копируем содержимое корневого core/ (sing-box.exe, wintun.dll, и т.д.)
xcopy /e /i /y core build\bin\core

echo [pre-nsis] Copied core -> build\bin\core
endlocal
