!macro VeilBoxKillProcess processName
  ClearErrors
  nsExec::ExecToLog '"$SYSDIR\taskkill.exe" /F /T /IM "${processName}"'
  Pop $0
!macroend

!macro VeilBoxStopRunningComponents
  DetailPrint "Stopping running VeilBox processes..."
  !insertmacro VeilBoxKillProcess "vailbox.exe"
  !insertmacro VeilBoxKillProcess "VeilBox.exe"
  !insertmacro VeilBoxKillProcess "xray.exe"
  !insertmacro VeilBoxKillProcess "amneziawg.exe"
  !insertmacro VeilBoxKillProcess "awg.exe"
  Sleep 1500
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro VeilBoxStopRunningComponents
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro VeilBoxStopRunningComponents
!macroend
