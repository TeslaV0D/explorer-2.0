# M8.3 crash report + start guard on Windows: a real Windows exception
# (RaiseException → unhandled exception filter), a panic, the safe start after
# two dead starts, and a GPU probe that took the process down.
#   pwsh scripts/m8-crash.ps1 <release dir> <work dir>
param([Parameter(Mandatory)] [string]$Bin, [Parameter(Mandatory)] [string]$Work)
$ErrorActionPreference = 'Continue'
$script:fail = 0
function Pass($m) { Write-Host "PASS  $m" }
function Fail($m) { Write-Host "FAIL  $m"; $script:fail = 1 }
function Check($m, [scriptblock]$cond) { if (& $cond) { Pass $m } else { Fail $m } }

$exe = Join-Path $Bin 'explorer2.exe'
$cfg = Join-Path $Work 'cfg'
$crash = Join-Path $cfg 'crash'
New-Item -ItemType Directory -Force $cfg | Out-Null
$env:EXPLORER2_CONFIG_DIR = $cfg

# a crashing start must end on its own (no "has stopped working" dialog)
function RunCrash([string]$kind) {
  $env:EXPLORER2_TEST_CRASH = $kind
  $p = Start-Process -FilePath $exe -PassThru
  $done = $p.WaitForExit(20000)
  Remove-Item Env:EXPLORER2_TEST_CRASH
  if (-not $done) { Stop-Process -Id $p.Id -Force; return $null }
  return '{0:x8}' -f $p.ExitCode
}
function NewReports { @(Get-ChildItem $crash -Filter 'crash-*.txt' -ErrorAction SilentlyContinue | Where-Object { $_.Name -notlike '*.seen.txt' }) }
function StartTraced([string]$log) {
  $env:EXPLORER2_TRACE_FILE = $log
  $p = Start-Process -FilePath $exe -PassThru
  Remove-Item Env:EXPLORER2_TRACE_FILE
  return $p
}
function Trace([string]$log, [string]$pat) { @(Select-String -Path $log -Pattern $pat -SimpleMatch -ErrorAction SilentlyContinue) }

Write-Host "== native Windows exception at start"
$code = RunCrash 'native'
$rep = @(NewReports)
Write-Host "    exit code: $code, reports: $($rep.Count)"
if ($rep.Count) { Get-Content $rep[0].FullName -TotalCount 12 | ForEach-Object { Write-Host "    | $_" } }
Check "[C1] the process ends by itself (no error dialog)" { $null -ne $code }
Check "[C2] report: Windows exception with code and exe offset" { $rep.Count -eq 1 -and (Select-String -Path $rep[0].FullName -Pattern 'Cause: Windows exception' -SimpleMatch) -and (Select-String -Path $rep[0].FullName -Pattern '0xe0002001' -SimpleMatch) -and (Select-String -Path $rep[0].FullName -Pattern 'Exe base' -SimpleMatch) }

Write-Host "== panic at start"
$code = RunCrash 'panic'
$rep = @(NewReports | Where-Object { Select-String -Path $_.FullName -Pattern 'Cause: Panic' -SimpleMatch })
Write-Host "    exit code: $code"
Check "[C3] a panic writes a report with message and the last trace lines" { $null -ne $code -and $rep.Count -eq 1 -and (Select-String -Path $rep[0].FullName -Pattern 'test crash requested' -SimpleMatch) -and (Select-String -Path $rep[0].FullName -Pattern 'start guard' -SimpleMatch) }
Check "[C4] both dead starts counted" { (Get-Content (Join-Path $cfg 'start-guard.txt') -Raw) -match 'failed=1' }

Write-Host "== third start: safe"
$log = Join-Path $Work 'trace-safe.log'
$p = StartTraced $log
Start-Sleep -Seconds 3
Get-Content $log -ErrorAction SilentlyContinue | Select-String 'start guard|renderer choice|notice' | ForEach-Object { Write-Host "    $_" }
Check "[C5] safe start: software renderer" { (Trace $log 'safe=true').Count -ge 1 -and (Trace $log 'reason="safe start').Count -ge 1 }
Check "[C6] notice about 2 reports + safe start" { (Trace $log 'notice reports=2 safe=true').Count -ge 1 }
Start-Sleep -Seconds 4
Check "[C7] stable after 5 s: guard removed" { -not (Test-Path (Join-Path $cfg 'start-guard.txt')) }
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500

Write-Host "== GPU probe that took the process down"
Set-Content (Join-Path $cfg 'gpu-probe.txt') "pid=999999`nfailed=0`n"
$log = Join-Path $Work 'trace-probe.log'
$p = StartTraced $log
Start-Sleep -Seconds 3
Check "[C8] normal start again (no notice), GPU marked unusable after a probe crash" { (Trace $log 'safe=false').Count -ge 1 -and (Trace $log 'notice ').Count -eq 0 -and (Trace $log 'gpu probe crashed last time').Count -ge 1 -and (Select-String -Path (Join-Path $cfg 'settings.ini') -Pattern 'gpu_probe = unusable' -SimpleMatch) }
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
exit $script:fail
