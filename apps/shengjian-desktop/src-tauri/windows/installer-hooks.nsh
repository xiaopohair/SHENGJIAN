!macro NSIS_HOOK_PREUNINSTALL
  ; Ask the bundled helper to remove its elevated scheduled task. The helper
  ; is still present here because this hook runs before file deletion.
  ${If} ${FileExists} "$INSTDIR\resources\rc003\shengjian-helper.exe"
    ExecShell "runas" "$INSTDIR\resources\rc003\shengjian-helper.exe" "--remove-task"
    ; ExecShell is async; give the elevated helper time to delete the task.
    Sleep 4000
  ${EndIf}
!macroend
