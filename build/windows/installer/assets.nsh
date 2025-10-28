; build/windows/installer/assets.nsh

!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_BITMAP "build/windows/installer/header.bmp"
!define MUI_HEADERIMAGE_RIGHT
!define MUI_WELCOMEFINISHPAGE_BITMAP "build/windows/installer/sidebar.bmp"
!define MUI_ICON  "build/windows/installer/logo.ico"
!define MUI_UNICON "build/windows/installer/logo.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\VeilBox.exe"
BrandingText "VeilBox Installer"
BGGradient 203040 0A1328 FFFFFF

Section "-VeilBoxCore"
  ; Берём core из build\bin (рабочая папка makensis у Wails)
  IfFileExists "core\*.*" +2 0
    DetailPrint "WARNING: build\\bin\\core not found. Did you pre-copy it?"

  SetOutPath "$INSTDIR\core"
  DetailPrint "Copying build\\bin\\core -> $INSTDIR\\core"
  File /r "core\*.*"
  DetailPrint "Core done."
SectionEnd
