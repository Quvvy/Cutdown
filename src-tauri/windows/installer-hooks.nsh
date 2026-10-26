; "Clip with Cutdown" in Explorer's right-click menu for video files (per user, no admin).
; On Windows 11 it appears under "Show more options". Same extensions as fileAssociations
; in tauri.conf.json.
!define CUTDOWN_CLIP_KEY "shell\Cutdown.Clip"

!macro CUTDOWN_ADD_CLIP_MENU EXT
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.${EXT}\${CUTDOWN_CLIP_KEY}" "" "Clip with Cutdown"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.${EXT}\${CUTDOWN_CLIP_KEY}" "Icon" '"$INSTDIR\Cutdown.exe",0'
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.${EXT}\${CUTDOWN_CLIP_KEY}\command" "" '"$INSTDIR\Cutdown.exe" "%1"'
!macroend

!macro CUTDOWN_REMOVE_CLIP_MENU EXT
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.${EXT}\${CUTDOWN_CLIP_KEY}"
!macroend

!macro CUTDOWN_CLIP_MENU_ALL ACTION
  !insertmacro ${ACTION} "mp4"
  !insertmacro ${ACTION} "mkv"
  !insertmacro ${ACTION} "mov"
  !insertmacro ${ACTION} "webm"
  !insertmacro ${ACTION} "ts"
  !insertmacro ${ACTION} "avi"
  !insertmacro ${ACTION} "flv"
  !insertmacro ${ACTION} "m4v"
!macroend

; Runs after Cutdown files are installed. Downloads ffmpeg/ffprobe to %LOCALAPPDATA%\Cutdown\ffmpeg.
!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Adding Clip with Cutdown to the Explorer right-click menu..."
  !insertmacro CUTDOWN_CLIP_MENU_ALL CUTDOWN_ADD_CLIP_MENU

  DetailPrint "Installing ffmpeg and ffprobe (required for video export)..."
  DetailPrint "This downloads the latest Windows essentials build (~80 MB)."

  nsExec::ExecToLog '"$INSTDIR\Cutdown.exe" --install-dependencies'
  Pop $0

  IntCmp $0 0 ffmpeg_install_ok ffmpeg_install_warn ffmpeg_install_warn

  ffmpeg_install_warn:
    MessageBox MB_ICONEXCLAMATION|MB_OK "Cutdown was installed, but ffmpeg could not be downloaded.$\r$\n$\r$\nCheck your internet connection, then open Cutdown and choose Download ffmpeg, or install ffmpeg on PATH.$\r$\n$\r$\nInstall log: %LOCALAPPDATA%\Cutdown\install-ffmpeg.log"
    Goto ffmpeg_install_done

  ffmpeg_install_ok:
    DetailPrint "ffmpeg installed successfully."

  ffmpeg_install_done:
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro CUTDOWN_CLIP_MENU_ALL CUTDOWN_REMOVE_CLIP_MENU
!macroend
