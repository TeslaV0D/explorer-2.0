# M8.3c endurance on Windows (real keyboard): many navigations, tabs
# opened/closed, view switches, file churn in the watched folder. Private
# memory, handles, threads and GDI/USER objects (icons!) must stay flat.
#   powershell -File scripts/m8-stress.ps1 <exe> <work dir> [rounds]
param([Parameter(Mandatory)] [string]$Exe, [Parameter(Mandatory)] [string]$Work, [int]$Rounds = 120)
$ErrorActionPreference = 'Continue'
$script:fail = 0
function Pass($m) { Write-Host "PASS  $m" }
function Fail($m) { Write-Host "FAIL  $m"; $script:fail = 1 }
function Check($m, [scriptblock]$cond) { if (& $cond) { Pass $m } else { Fail $m } }

Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class S {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, UIntPtr e);
  [DllImport("user32.dll")] public static extern uint GetGuiResources(IntPtr p, uint flags);
}
"@

New-Item -ItemType Directory -Force $Work | Out-Null
$env:EXPLORER2_CONFIG_DIR = Join-Path $Work 'cfg'
$env:EXPLORER2_TRACE = '1'
$sd = Join-Path $Work 'stress'
foreach ($i in 1..20) {
  $d = New-Item -ItemType Directory -Force (Join-Path $sd ('d{0:d2}' -f $i))
  foreach ($j in 1..120) { [IO.File]::WriteAllText((Join-Path $d "f$j.e$($j % 9)"), '') }
}
$log = Join-Path $Work 'trace-stress.log'
$p = Start-Process -FilePath $Exe -ArgumentList "`"$sd`"" -PassThru -RedirectStandardError $log
Start-Sleep -Seconds 3
$p.Refresh(); $hwnd = $p.MainWindowHandle
[void][S]::ShowWindow($hwnd, 3)
function Focus {
  [S]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero); [S]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
  [void][S]::SetForegroundWindow($hwnd); Start-Sleep -Milliseconds 200
}
function K([string]$k, [int]$ms = 60) { [System.Windows.Forms.SendKeys]::SendWait($k); Start-Sleep -Milliseconds $ms }
function Stats {
  $p.Refresh()
  [pscustomobject]@{
    priv = $p.PrivateMemorySize64; handles = $p.HandleCount; threads = $p.Threads.Count
    gdi = [S]::GetGuiResources($p.Handle, 0); user = [S]::GetGuiResources($p.Handle, 1)
  }
}
function Navs { @(Select-String -Path $log -Pattern 'navigate tab=' -SimpleMatch).Count }
function Cycle([int]$i) {
  if ($i % 25 -eq 0) { Focus }
  if ($i % 20 -eq 0) { K '{HOME}' }
  K '{DOWN}'; K '{ENTER}' 150
  if ($i % 5 -eq 0) { K '^+6' 120; K '^+1' }
  if ($i % 7 -eq 0) {
    [IO.File]::WriteAllText((Join-Path $sd "churn-$i"), 'x')
    Remove-Item (Join-Path $sd "churn-$($i - 7)") -ErrorAction SilentlyContinue
  }
  K '{BACKSPACE}' 150
  K '^t'; K '^w'
}

Write-Host "== M8.3c endurance on Windows: warm-up + 2 x $Rounds rounds"
# image cache (thumbnail LRU, by design up to 96 MB) as last reported by the app
function ThumbBytes {
  $m = @(Select-String -Path $log -Pattern 'thumbs loaded n=\d+ previews=\d+ kept=\d+ bytes=(\d+)')
  if ($m.Count) { [double]$m[-1].Matches[0].Groups[1].Value } else { 0 }
}
function Show($label, $s, $t, $n) {
  Write-Host ("    {0}: private={1:n1} MiB (image cache {2:n1} MiB) handles={3} threads={4} gdi={5} user={6} navigations={7}" -f $label, ($s.priv / 1MB), ($t / 1MB), $s.handles, $s.threads, $s.gdi, $s.user, $n)
}
Focus
foreach ($i in 1..30) { Cycle $i }
Start-Sleep -Seconds 2
$a = Stats; $ta = ThumbBytes; $n1 = Navs; Show 'after warm-up' $a $ta $n1
foreach ($i in 31..(30 + $Rounds)) { Cycle $i }
Start-Sleep -Seconds 3
$b = Stats; $tb = ThumbBytes; $n2 = Navs; Show "after $Rounds rounds" $b $tb $n2
foreach ($i in (31 + $Rounds)..(30 + 2 * $Rounds)) { Cycle $i }
Start-Sleep -Seconds 3
$c = Stats; $tc = ThumbBytes; $n3 = Navs; Show "after $(2 * $Rounds) rounds" $c $tc $n3
# growth not explained by the (bounded) image cache, per phase
$ga = [math]::Round((($b.priv - $a.priv) - ($tb - $ta)) / 1MB, 1)
$gb = [math]::Round((($c.priv - $b.priv) - ($tc - $tb)) / 1MB, 1)
Write-Host "    growth without image cache: phase A +$ga MiB, phase B +$gb MiB"
Check "[S1] the rounds really ran ($($n3 - $n1) navigations)" { ($n3 - $n1) -ge (2 * $Rounds) }
Check "[S2] memory flat: phase B +$gb MiB without the image cache (limit +6; phase A +$ga)" { $gb -lt 6 }
Check "[S2b] the image cache stays in its budget ($([math]::Round($tc / 1MB, 1)) MiB, limit 96)" { $tc -le 96MB }
Check "[S3] no handle leak (phase B +$($c.handles - $b.handles), limit +40)" { ($c.handles - $b.handles) -le 40 }
Check "[S4] no thread leak (+$($c.threads - $a.threads), limit +4)" { ($c.threads - $a.threads) -le 4 }
Check "[S5] no GDI/USER object leak (icons, menus: +$($c.gdi - $a.gdi) / +$($c.user - $a.user), limit +30)" { ($c.gdi - $a.gdi) -le 30 -and ($c.user - $a.user) -le 30 }
$p.Refresh()
Check "[S6] process alive, no panic" { -not $p.HasExited -and (Get-Content $log -Raw) -notmatch '(?i)panic' }
if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
exit $script:fail
