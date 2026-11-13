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
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);
}
"@

New-Item -ItemType Directory -Force $Work | Out-Null
$env:EXPLORER2_CONFIG_DIR = Join-Path $Work 'cfg'
$env:EXPLORER2_TRACE = '1'
$env:EXPLORER2_TRACE_KEYS = '1'   # where every key lands (diagnosis, CI #75/#76)
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
function Title([IntPtr]$h) { $sb = New-Object System.Text.StringBuilder 256; [void][S]::GetWindowText($h, $sb, 256); $sb.ToString() }
$script:stolen = @()
# keys only count while our window is in front (a late dialog or another
# window must not swallow the rounds, CI #75)
function EnsureFront {
  $fg = [S]::GetForegroundWindow()
  if ($fg -ne $hwnd) {
    $script:stolen += (Title $fg)
    Focus
  }
}
function Stats {
  $p.Refresh()
  [pscustomobject]@{
    priv = $p.PrivateMemorySize64; handles = $p.HandleCount; threads = $p.Threads.Count
    gdi = [S]::GetGuiResources($p.Handle, 0); user = [S]::GetGuiResources($p.Handle, 1)
  }
}
function Navs { @(Select-String -Path $log -Pattern 'navigate tab=' -SimpleMatch).Count }
function Cycle([int]$i) {
  EnsureFront
  # 20 folders, then files: back to the top before Down passes the last folder
  if ($i % 19 -eq 0) { K '{HOME}' }
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
if ($script:stolen.Count) {
  $names = ($script:stolen | Group-Object | ForEach-Object { "'$($_.Name)' x$($_.Count)" }) -join ', '
  Write-Host "    focus taken $($script:stolen.Count)x by: $names (re-focused)"
}
Check "[S1] the rounds really ran ($($n3 - $n1) navigations)" { ($n3 - $n1) -ge (2 * $Rounds) }
if (($n3 - $n1) -lt (2 * $Rounds)) {
  # the keys reached our window but did nothing: show what the app did last
  $lines = @(Get-Content $log)
  $lastNav = ($lines | Select-String 'navigate tab=' | Select-Object -Last 1)
  $at = if ($lastNav) { $lastNav.LineNumber } else { 0 }
  Write-Host "    ----- trace around the last navigation (line $at of $($lines.Count))"
  $lines[[math]::Max(0, $at - 15)..[math]::Min($lines.Count - 1, $at + 60)] | ForEach-Object { Write-Host "    | $_" }
  Write-Host "    ----- last 40 trace lines"
  $lines | Select-Object -Last 40 | ForEach-Object { Write-Host "    | $_" }
  $shot = Join-Path $Work 'stress-fail.png'
  Add-Type -AssemblyName System.Drawing
  $b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
  $bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
  [System.Drawing.Graphics]::FromImage($bmp).CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size)
  $bmp.Save($shot); Write-Host "    screenshot: $shot"
}
Check "[S2] memory flat: phase B +$gb MiB without the image cache (limit +6; phase A +$ga)" { $gb -lt 6 }
Check "[S2b] the image cache stays in its budget ($([math]::Round($tc / 1MB, 1)) MiB, limit 96)" { $tc -le 96MB }
Check "[S3] no handle leak (phase B +$($c.handles - $b.handles), limit +40)" { ($c.handles - $b.handles) -le 40 }
Check "[S4] no thread leak (+$($c.threads - $a.threads), limit +4)" { ($c.threads - $a.threads) -le 4 }
Check "[S5] no GDI/USER object leak (icons, menus: +$($c.gdi - $a.gdi) / +$($c.user - $a.user), limit +30)" { ($c.gdi - $a.gdi) -le 30 -and ($c.user - $a.user) -le 30 }
Check "[S7] nothing else came to the front (the rounds only navigate, never open a file)" { $script:stolen.Count -eq 0 }
$p.Refresh()
Check "[S6] process alive, no panic" { -not $p.HasExited -and (Get-Content $log -Raw) -notmatch '(?i)panic' }
if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }

# D-028: hidden window drops the folder listing. The same release the close
# button uses, driven by EXPLORER2_TEST_PARK (the navigation run above stays
# untouched). EXPLORER2_INDEX=off so the index does not hide the drop.
Write-Host "== [S8] idle memory: drop the folder listing (4 park/resume cycles)"
$big = Join-Path $Work 'park-big'
New-Item -ItemType Directory -Force $big | Out-Null
foreach ($i in 1..20000) {
  [IO.File]::WriteAllBytes((Join-Path $big ('f{0:d5}.dat' -f $i)), [byte[]]::new(0))
}
$ctl = Join-Path $Work 'park-cmd'
Set-Content -Path $ctl -Value '' -Encoding ascii
$plog = Join-Path $Work 'trace-park.log'
$env:EXPLORER2_INDEX = 'off'
$env:EXPLORER2_TEST_PARK = $ctl
$env:EXPLORER2_CONFIG_DIR = Join-Path $Work 'cfg-park'
$pp = Start-Process -FilePath $Exe -ArgumentList "`"$big`"" -PassThru -RedirectStandardError $plog
function PCount([string]$pat) { @(Select-String -Path $plog -Pattern $pat -SimpleMatch).Count }
function PWait([string]$pat, [int]$want) {
  foreach ($i in 1..80) {
    if ((PCount $pat) -ge $want) { return $true }
    Start-Sleep -Milliseconds 250
  }
  Write-Host "    timeout: wanted $want x '$pat', have $(PCount $pat)"
  return $false
}
$hi = @(); $lo = @()
if (PWait 'done tab=' 1) {
  Start-Sleep -Milliseconds 500
  $pp.Refresh(); $hwnd = $pp.MainWindowHandle
  [void][S]::ShowWindow($hwnd, 3)
  [S]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero); [S]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
  [void][S]::SetForegroundWindow($hwnd)
  Start-Sleep -Milliseconds 200
  K '{DOWN}'
  Start-Sleep -Milliseconds 300
} else { Fail "[S8] the big folder loaded" }
for ($n = 1; $n -le 4; $n++) {
  Set-Content -Path $ctl -Value 'park' -Encoding ascii
  if (-not (PWait 'parked entries=' $n)) { Fail "[S8] park $n"; break }
  Start-Sleep -Milliseconds 400
  $pp.Refresh(); $lo += $pp.PrivateMemorySize64
  $before = PCount 'done tab='
  Set-Content -Path $ctl -Value 'resume' -Encoding ascii
  if (-not (PWait 'resumed' $n)) { Fail "[S8] resume $n"; break }
  $back = $false
  foreach ($i in 1..40) {
    if ((PCount 'done tab=') -gt $before) { $back = $true; break }
    Start-Sleep -Milliseconds 250
  }
  if (-not $back) { Fail "[S8] reload $n"; break }
  Start-Sleep -Milliseconds 400
  $pp.Refresh(); $hi += $pp.PrivateMemorySize64
}
$parkLine = @(Select-String -Path $plog -Pattern 'parked entries=' | Select-Object -First 1)
Write-Host "    $($parkLine.Line)"
$ent = 0; $byt = 0; $sel = 0
if ($parkLine.Line -match 'entries=(\d+) bytes=(\d+) selected=(\d+)') {
  $ent = [int]$Matches[1]; $byt = [int]$Matches[2]; $sel = [int]$Matches[3]
}
Check "[S8] park dropped the listing ($ent entries, $byt bytes)" { $ent -ge 15000 -and $byt -ge 1000000 }
Check "[S9] the focused name is kept (selected=$sel)" { $sel -ge 1 }
if ($hi.Count -ge 4 -and $lo.Count -ge 4) {
  $drop = [math]::Round(($hi[0] - $lo[0]) / 1MB, 1)
  $grow = [math]::Round(($hi[3] - $hi[0]) / 1MB, 1)
  $pgrow = [math]::Round(($lo[3] - $lo[0]) / 1MB, 1)
  Write-Host "    drop on first park: $drop MiB | shown growth: $grow MiB | parked growth: $pgrow MiB"
  Check "[S10] parking frees memory ($drop MiB, at least 2)" { ($hi[0] - $lo[0]) -ge 2MB }
  Check "[S11] reloading does not leak (shown +$grow MiB, parked +$pgrow MiB, limit +16)" { ($hi[3] - $hi[0]) -lt 16MB -and ($lo[3] - $lo[0]) -lt 16MB }
} else { Fail "[S10] park/resume cycles did not finish" }
$pp.Refresh()
Check "[S12] process alive, no panic" { -not $pp.HasExited -and (Get-Content $plog -Raw) -notmatch '(?i)panic' }
if (-not $pp.HasExited) { Stop-Process -Id $pp.Id -Force }
exit $script:fail
