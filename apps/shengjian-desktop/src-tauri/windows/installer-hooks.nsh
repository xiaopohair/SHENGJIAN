; ---------------------------------------------------------------------------
; 全按键支持助手与运行中的文件
; ---------------------------------------------------------------------------
; Windows 会锁住正在运行的 exe / dll，安装包无法覆盖它们。助手通常以管理员
; 权限运行（安装包是普通权限，结束不掉它），于是升级时会在
; resources\hid-host\shengjian-helper.exe 上失败（“无法打开要写入的文件”）。
; 先让计划任务停掉助手；若仍有进程占着（手动启动 / 提权实例），就把旧文件改名
; 挪开——Windows 允许重命名正在运行的 exe / dll，安装包随后写入新文件。
;
; 0.9.3 把资源目录从 resources\rc003 改名为 resources\hid-host、计划任务从
; "Shengjian RC003 Helper" 改名为 "Shengjian HID Host"。这里两套路径和任务名都处理，
; 保证从任何更早版本升级都不会在旧文件上卡住——自动更新走的正是这条路。

!macro ShengjianMoveAsideIfLocked target
  ${If} ${FileExists} "${target}"
    ClearErrors
    Delete "${target}"
    ${If} ${Errors}
      Push $0
      Push $1
      Push $2
      Push $3
      Push $4
      Push $5
      Push $6
      ${GetTime} "" "L" $0 $1 $2 $3 $4 $5 $6
      ; GetTime 输出：$5 年、$6 月、$0 日、$3 时、$2 分
      Rename "${target}" "${target}.$5$6$0$3$2.old"
      ; 旧文件此刻仍被占用，安排到下次重启时清掉。
      Delete /REBOOTOK "${target}.$5$6$0$3$2.old"
      Pop $6
      Pop $5
      Pop $4
      Pop $3
      Pop $2
      Pop $1
      Pop $0
      ClearErrors
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREINSTALL
  nsExec::ExecToLog '"$SYSDIR\schtasks.exe" /End /TN "Shengjian HID Host"'
  nsExec::ExecToLog '"$SYSDIR\schtasks.exe" /End /TN "Shengjian RC003 Helper"'
  nsExec::ExecToLog '"$SYSDIR\taskkill.exe" /F /IM shengjian-helper.exe'
  Sleep 500

  !insertmacro ShengjianMoveAsideIfLocked "$INSTDIR\resources\hid-host\shengjian-helper.exe"
  !insertmacro ShengjianMoveAsideIfLocked "$INSTDIR\resources\hid-host\frida-gadget.dll"
  !insertmacro ShengjianMoveAsideIfLocked "$INSTDIR\resources\hid-host\WinUHid.dll"
  ; 0.9.3 之前的布局（覆盖安装旧版本时仍可能存在）。
  !insertmacro ShengjianMoveAsideIfLocked "$INSTDIR\resources\rc003\shengjian-helper.exe"
  !insertmacro ShengjianMoveAsideIfLocked "$INSTDIR\resources\rc003\frida-gadget.dll"
!macroend

; ---------------------------------------------------------------------------
; 卸载
; ---------------------------------------------------------------------------

!macro NSIS_HOOK_PREUNINSTALL
  ; Ask the bundled helper to remove its elevated scheduled task. The helper
  ; is still present here because this hook runs before file deletion.
  ${If} ${FileExists} "$INSTDIR\resources\hid-host\shengjian-helper.exe"
    ExecShell "runas" "$INSTDIR\resources\hid-host\shengjian-helper.exe" "--remove-task"
    ; ExecShell is async; give the elevated helper time to delete the task.
    Sleep 4000
  ${EndIf}
  ; 0.9.3 之前升级上来的目录里可能还留着旧助手与旧任务名。
  ${If} ${FileExists} "$INSTDIR\resources\rc003\shengjian-helper.exe"
    ExecShell "runas" "$INSTDIR\resources\rc003\shengjian-helper.exe" "--remove-task"
    Sleep 4000
  ${EndIf}
!macroend
