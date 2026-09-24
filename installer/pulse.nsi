Unicode true
!include "MUI2.nsh"

!define APP_NAME "Pulse"
!define APP_DESC "任务栏系统监控"
!define APP_VERSION "0.1.0"
!define APP_PUBLISHER "Pulse"
!define APP_EXE "Pulse.exe"
!define APP_KEY "Pulse"

Name "${APP_NAME} ${APP_VERSION}"
BrandingText "${APP_NAME} ${APP_VERSION}"
OutFile "PulseSetup.exe"
InstallDir "$LOCALAPPDATA\Programs\${APP_NAME}"
InstallDirRegKey HKCU "Software\${APP_KEY}" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma

!define MUI_ICON "pulse.ico"
!define MUI_UNICON "pulse.ico"
!define MUI_ABORTWARNING
!define MUI_WELCOMEFINISHPAGE_BITMAP "welcome.bmp"
!define MUI_UNWELCOMEFINISHPAGE_BITMAP "welcome.bmp"
!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "运行 ${APP_NAME}"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"
  File "..\target\release\${APP_EXE}"

  CreateDirectory "$INSTDIR\lib"
  SetOutPath "$INSTDIR\lib"
  File "..\lib\LhmNative.dll"
  File "..\lib\PawnIO_setup.exe"

  CreateDirectory "$INSTDIR\assets"
  SetOutPath "$INSTDIR\assets"
  File "pulse.ico"

  SetOutPath "$INSTDIR"

  ; 传感器驱动 PawnIO：没有才装（需要管理员，会弹一次 UAC）。
  ; 它是 CPU 封装温度与内存 (SPD) 温度的数据来源。
  ReadRegStr $0 HKLM "SYSTEM\CurrentControlSet\Services\PawnIO" "ImagePath"
  StrCmp $0 "" 0 pawnio_done
    DetailPrint "安装 PawnIO 传感器驱动（需要管理员权限）..."
    ClearErrors
    ExecShellWait "runas" "$INSTDIR\lib\PawnIO_setup.exe" "-install -silent"
    IfErrors 0 pawnio_done
      DetailPrint "PawnIO 未安装（已取消或权限不足），CPU / 内存温度将显示 —。"
  pawnio_done:

  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\assets\pulse.ico" 0
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk" "$INSTDIR\Uninstall.exe"
  CreateShortcut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\assets\pulse.ico" 0

  ; 任务栏常驻监控，装完默认开机自启动
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${APP_KEY}" "$INSTDIR\${APP_EXE}"

  WriteUninstaller "$INSTDIR\Uninstall.exe"

  WriteRegStr HKCU "Software\${APP_KEY}" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_KEY}" "DisplayName" "${APP_NAME} - ${APP_DESC}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_KEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_KEY}" "Publisher" "${APP_PUBLISHER}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_KEY}" "DisplayIcon" "$INSTDIR\${APP_EXE}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_KEY}" "UninstallString" "$INSTDIR\Uninstall.exe"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_KEY}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\lib\LhmNative.dll"
  Delete "$INSTDIR\lib\PawnIO_setup.exe"
  Delete "$INSTDIR\assets\pulse.ico"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR\lib"
  RMDir "$INSTDIR\assets"
  RMDir "$INSTDIR"

  Delete "$DESKTOP\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"

  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${APP_KEY}"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_KEY}"
  DeleteRegKey HKCU "Software\${APP_KEY}"
SectionEnd
