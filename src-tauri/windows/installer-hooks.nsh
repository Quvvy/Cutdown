; Runs after Cutdown files are installed. Downloads ffmpeg/ffprobe to %LOCALAPPDATA%\Cutdown\ffmpeg.
!macro NSIS_HOOK_POSTINSTALL
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
