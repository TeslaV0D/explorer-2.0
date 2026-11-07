# M8.3b one window on Windows (named pipe): a second start hands its folder
# to the running window (new tab, minimized window restored) and exits; a
# hung (suspended) instance does not block it; --new-instance opts out.
#   pwsh scripts/m8-instance.ps1 <release dir> <work dir>
param([Parameter(Mandatory)] [string]$Bin, [Parameter(Mandatory)] [string]$Work)
$ErrorActionPreference = 'Continue'
$script:fail = 0
function Pass($m) { Write-Host "PASS  $m" }
function Fail($m) { Write-Host "FAIL  $m"; $script:fail = 1 }
function Check($m, [scriptblock]$cond) { if (& $cond) { Pass $m } else { Fail $m } }

Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class I {
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("ntdll.dll")] public static extern int NtSuspendProcess(IntPtr h);
  [DllImport("ntdll.dll")] public static extern int NtResumeProcess(IntPtr h);
}
"@

$exe = Join-Path $Bin 'explorer2.exe'
$cfg = Join-Path $Work 'cfg'
$dirA = New-Item -ItemType Directory -Force (Join-Path $Work 'ordner a')
New-Item -ItemType Directory -Force $cfg | Out-Null
$env:EXPLORER2_CONFIG_DIR = $cfg
$env:EXPLORER2_TRACE = '1'
$log = Join-Path $Work 'trace-main.log'
function Trace([string]$pat) { @(Select-String -Path $log -Pattern $pat -SimpleMatch -ErrorAction SilentlyContinue) }
function WaitTrace([string]$pat, [int]$ms = 10000) {
  $sw = [Diagnostics.Stopwatch]::StartNew()
  while ($sw.ElapsedMilliseconds -lt $ms) { if ((Trace $pat).Count) { return $true }; Start-Sleep -Milliseconds 200 }
  return $false
}
function Second([string[]]$argv, [string]$name, [int]$ms = 15000) {
  $o = Join-Path $Work "second-$name.log"
  $sw = [Diagnostics.Stopwatch]::StartNew()
  $q = Start-Process -FilePath $exe -ArgumentList $argv -PassThru -RedirectStandardError $o
  $done = $q.WaitForExit($ms)
  $r = @{ proc = $q; done = $done; ms = $sw.ElapsedMilliseconds; log = (Get-Content $o -Raw -ErrorAction SilentlyContinue) }
  Write-Host "    second start ($name): exited=$done after $($r.ms) ms"
  return $r
}
function Instances { @(Get-CimInstance Win32_Process -Filter "Name='explorer2.exe'") }

$p = Start-Process -FilePath $exe -ArgumentList "`"$Work`"" -PassThru -RedirectStandardError $log
try {
  Check "[N1] the first window serves the named pipe" { WaitTrace 'instance: serving \\.\pipe\explorer2-' }
  Start-Sleep -Seconds 1
  $p.Refresh(); $hwnd = $p.MainWindowHandle
  [void][I]::ShowWindow($hwnd, 6)   # minimize
  Start-Sleep -Milliseconds 500
  $r = Second @("`"$($dirA.FullName)`"") 'folder'
  Check "[N2] a second start with a folder hands over and exits quickly" { $r.done -and $r.proc.ExitCode -eq 0 -and $r.log -match 'hand over Done' -and $r.ms -lt 4000 }
  Check "[N3] the running window opened it in a new tab" { (WaitTrace "instance: open $($dirA.FullName)" 3000) -and (Trace 'tab new').Count -ge 1 }
  Start-Sleep -Milliseconds 500
  Check "[N4] the minimized window was restored" { -not [I]::IsIconic($hwnd) }
  Get-Content $log | Select-String 'instance:' | ForEach-Object { Write-Host "    $_" }
  Check "[N5] still exactly one Explorer 2.0 process" { (Instances).Count -eq 1 }

  # a hung instance must not leave the user without a window
  [void][I]::NtSuspendProcess($p.Handle)
  $o = Join-Path $Work 'second-hung.log'
  $q = Start-Process -FilePath $exe -ArgumentList "`"$($dirA.FullName)`"" -PassThru -RedirectStandardError $o
  Start-Sleep -Seconds 7
  $q.Refresh()
  $hung = Get-Content $o -Raw -ErrorAction SilentlyContinue
  Check "[N6] hung instance: the second start opens its own window after the wait" { -not $q.HasExited -and $hung -match 'hand over NoAnswer' -and $hung -match 'done tab=' }
  if (-not $q.HasExited) { Stop-Process -Id $q.Id -Force }
  [void][I]::NtResumeProcess($p.Handle)
  Start-Sleep -Milliseconds 800

  $o = Join-Path $Work 'second-new.log'
  $q = Start-Process -FilePath $exe -ArgumentList '--new-instance', "`"$($dirA.FullName)`"" -PassThru -RedirectStandardError $o
  Start-Sleep -Seconds 3
  $q.Refresh()
  $txt = Get-Content $o -Raw -ErrorAction SilentlyContinue
  Check "[N7] --new-instance: own window, the flag is no folder" { -not $q.HasExited -and $txt -notmatch 'hand over' -and $txt -match [regex]::Escape("start=$($dirA.FullName)") }
  if (-not $q.HasExited) { Stop-Process -Id $q.Id -Force }
  $p.Refresh()
  Check "[N8] the first window survived all of it (no panic)" { -not $p.HasExited -and (Get-Content $log -Raw) -notmatch '(?i)panic' }
} finally {
  Instances | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
}
Start-Sleep -Milliseconds 500
$r = Second @("`"$($dirA.FullName)`"") 'after' 5000
Check "[N9] with no window open, a start is a normal window (pipe freed with the process)" { -not $r.done -and $r.log -match 'hand over NoInstance' }
Instances | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
exit $script:fail
