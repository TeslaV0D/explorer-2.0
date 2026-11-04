# Windows E2E: drives the real app with real keyboard/mouse input (SendKeys,
# SetCursorPos/mouse_event) on the CI runner's interactive desktop and checks
# the file system, the system clipboard, the recycle bin and the trace log.
# Usage: powershell -File scripts/e2e-windows.ps1 <explorer2.exe> <workdir>
param([string]$Exe, [string]$Work)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class W {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, UIntPtr e);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  public static string Titles(uint pid) {
    var sb = new StringBuilder();
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p);
      if (p == pid && IsWindowVisible(h)) { var t = new StringBuilder(256); GetWindowText(h, t, 256); sb.Append(t.ToString()).Append("|"); }
      return true; }, IntPtr.Zero);
    return sb.ToString();
  }
  public static void CloseTitled(uint pid, string part) {
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p);
      var t = new StringBuilder(256); GetWindowText(h, t, 256);
      if (p == pid && t.ToString().Contains(part)) SendMessage(h, 0x0010, IntPtr.Zero, IntPtr.Zero); // WM_CLOSE
      return true; }, IntPtr.Zero);
  }
}
"@

$script:fail = 0
function Pass($m) { Write-Host "PASS  $m" }
function Fail($m) { Write-Host "FAIL  $m"; $script:fail = 1 }
function Check($m, [scriptblock]$cond) { if (& $cond) { Pass $m } else { Fail $m; Shot ("fail-" + ($m -replace '[^\w]', '_')) } }
function Shot($name) {
  $b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
  $bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
  $g = [System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size)
  $bmp.Save("$Work\$name.png"); $g.Dispose(); $bmp.Dispose()
}
function Log { Get-Content $script:log -Raw -ErrorAction SilentlyContinue }
function Last($pattern) { (Log) -split "`n" | Where-Object { $_ -match $pattern } | Select-Object -Last 1 }
function WaitFor($pattern, [int]$ms = 5000) {
  $t = [Diagnostics.Stopwatch]::StartNew()
  while ($t.ElapsedMilliseconds -lt $ms) { if ((Log) -match $pattern) { return $true }; Start-Sleep -Milliseconds 100 }
  return $false
}
function Keys($k) { [System.Windows.Forms.SendKeys]::SendWait($k); Start-Sleep -Milliseconds 400 }

# ---- layout (main.slint, logical px from the client origin) ----
$ROW0 = 140; $ROWH = 24; $LISTX = 330
function Pt([int]$x, [int]$y) {
  $p = New-Object W+POINT; $p.X = [int]($x * $script:scale); $p.Y = [int]($y * $script:scale)
  [void][W]::ClientToScreen($script:hwnd, [ref]$p); return $p
}
function RowY([int]$i) { $ROW0 + $ROWH * $i + 12 }
function Click([int]$x, [int]$y, [string]$button = 'left') {
  $p = Pt $x $y; [void][W]::SetCursorPos($p.X, $p.Y); Start-Sleep -Milliseconds 80
  if ($button -eq 'left') { [W]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero); [W]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero) }
  else { [W]::mouse_event(0x08, 0, 0, 0, [UIntPtr]::Zero); [W]::mouse_event(0x10, 0, 0, 0, [UIntPtr]::Zero) }
  Start-Sleep -Milliseconds 400
}
function Drag([int]$x, [int]$y1, [int]$y2) {
  $p = Pt $x $y1; [void][W]::SetCursorPos($p.X, $p.Y); Start-Sleep -Milliseconds 100
  [W]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero)               # left down
  $steps = 12
  for ($s = 1; $s -le $steps; $s++) {
    $q = Pt $x ([int]($y1 + ($y2 - $y1) * $s / $steps)); [void][W]::SetCursorPos($q.X, $q.Y); Start-Sleep -Milliseconds 60
  }
  Start-Sleep -Milliseconds 300
  [W]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero)               # left up
  Start-Sleep -Milliseconds 1200
}
function Focus {
  # the foreground lock lets a background process activate a window after an Alt press
  [W]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero); [W]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
  [void][W]::SetForegroundWindow($script:hwnd); Start-Sleep -Milliseconds 300
}
function RecycleBinNames {
  $sh = New-Object -ComObject Shell.Application
  @($sh.NameSpace(10).Items() | ForEach-Object { $_.Name })
}

# ---- fixture ----
$R = Join-Path $Work ("win-e2e-" + $PID)
$src = New-Item -ItemType Directory -Force "$R\src"; $dst = New-Item -ItemType Directory -Force "$R\dst"
New-Item -ItemType Directory -Force "$src\Ordner" | Out-Null
Set-Content "$src\a.txt" "a"; Set-Content "$src\b.txt" "b"
# rows (name sort, folders first): 0 Ordner, 1 a.txt, 2 b.txt

$script:log = "$Work\win-e2e.log"
$env:EXPLORER2_TRACE = '1'; $env:EXPLORER2_CONFIG_DIR = "$Work\cfg-e2e"
$p = Start-Process -FilePath $Exe -ArgumentList "`"$src`"" -PassThru -RedirectStandardError $script:log
if (-not (WaitFor 'done tab=1 ' 15000)) { throw "app did not load the folder" }
Start-Sleep -Seconds 1
$p.Refresh(); $script:hwnd = $p.MainWindowHandle
if ($script:hwnd -eq [IntPtr]::Zero) { throw "no main window" }
[void][W]::ShowWindow($script:hwnd, 3)  # maximize: full layout regardless of the runner's screen
Start-Sleep -Milliseconds 800
$script:scale = [W]::GetDpiForWindow($script:hwnd) / 96.0
Write-Host "hwnd=$($script:hwnd) scale=$($script:scale) screen=$([System.Windows.Forms.Screen]::PrimaryScreen.Bounds)"
Focus
Click $LISTX (RowY 1)
Check "[W1] click selects a.txt" { (Last 'selection pane') -match 'focus=a\.txt' }

Keys '{F2}'
Check "[W2] F2 opens inline rename" { (Last 'rename start') -match 'name=a\.txt' }
Keys 'neu'; Keys '{ENTER}'; Start-Sleep -Milliseconds 800
Check "[W3] rename replaces the stem (a.txt -> neu.txt)" { (Test-Path "$src\neu.txt") -and -not (Test-Path "$src\a.txt") }
Keys '^z'; Start-Sleep -Milliseconds 800
Check "[W4] Ctrl+Z undoes the rename" { (Test-Path "$src\a.txt") -and -not (Test-Path "$src\neu.txt") }

Keys '^+n'; Start-Sleep -Milliseconds 1000
Check "[W5] Ctrl+Shift+N creates a folder and starts renaming it" { (Test-Path "$src\Neuer Ordner") -and (Last 'rename start') -match 'name=Neuer Ordner' }
Keys '{ENTER}'; Start-Sleep -Milliseconds 600
$newDir = (Get-ChildItem $src -Directory | Where-Object Name -ne 'Ordner' | Select-Object -First 1).Name
Write-Host "new folder: $newDir"
# rows: 0 <new folder>, 1 Ordner, 2 a.txt, 3 b.txt  (both folder names sort before files)
Click $LISTX (RowY 2)
Keys '^c'
$clip = @([System.Windows.Forms.Clipboard]::GetFileDropList())
Check "[W6] Ctrl+C puts the file on the Windows clipboard" { $clip.Count -eq 1 -and $clip[0] -like '*\a.txt' }

Keys '^l'; Keys "$dst"; Keys '{ENTER}'; Start-Sleep -Milliseconds 1200
Keys '^v'; Start-Sleep -Milliseconds 1500
Check "[W7] Ctrl+V copies it into dst" { (Test-Path "$dst\a.txt") -and (Test-Path "$src\a.txt") }

Click $LISTX (RowY 0)
Keys '{DEL}'; Start-Sleep -Milliseconds 1500
Check "[W8] Del moves to the recycle bin" { -not (Test-Path "$dst\a.txt") -and ((RecycleBinNames) -match '^a(\.txt)?$') }
Keys '^z'; Start-Sleep -Milliseconds 2000
Check "[W9] Ctrl+Z restores it (with extension)" { Test-Path "$dst\a.txt" }

# real shell context menu: open on the row, close with Esc
Click $LISTX (RowY 0) 'right'; Start-Sleep -Milliseconds 1200; Shot "w10-menu"
Keys '{ESC}'; Start-Sleep -Milliseconds 600
Check "[W10] right-click shows the shell menu (closed with Esc)" { (Last 'context menu result') -match 'Dismissed' }

# properties dialog (shell window in our process)
Keys '%{ENTER}'; Start-Sleep -Milliseconds 2500
$titles = [W]::Titles([uint32]$p.Id)
Write-Host "windows of the app: $titles"
# title like "a Properties" / "Eigenschaften von a" (the shell hides known extensions)
Check "[W11] Alt+Enter opens the properties dialog" { $titles -match '(^|\|)(a Properties|Eigenschaften von a)(\.txt)?\|' }
foreach ($t in 'Properties', 'Eigenschaften') { [W]::CloseTitled([uint32]$p.Id, $t) }; Start-Sleep -Milliseconds 500
Focus

# real OLE drag & drop with the mouse: b.txt (row 3) onto folder "Ordner" (row 1)
Keys '%{LEFT}'; Start-Sleep -Milliseconds 1200
Drag $LISTX (RowY 3) (RowY 1)
Check "[W12] OLE drag onto a folder moves the file" { (Test-Path "$src\Ordner\b.txt") -and -not (Test-Path "$src\b.txt") }
Check "[W12b] drop went through our drop target" { (Last 'ole drop') -match 'effect=Move' }

# M5.6: recycle bin view (rows in src now: 0 Neuer Ordner, 1 Ordner, 2 a.txt)
Click $LISTX (RowY 2)
Keys '{DEL}'; Start-Sleep -Milliseconds 1500
Keys '^l'; Keys 'Papierkorb'; Keys '{ENTER}'; Start-Sleep -Milliseconds 2000
Check "[W15] address 'Papierkorb' opens the recycle bin view" { (Last 'navigate tab') -match ' ::papierkorb ' }
Check "[W15b] the deleted file is listed with its folder" { (Last 'kinds') -match [regex]::Escape("$src".Replace('\', '\\')) -or (Last 'kinds') -match 'win-e2e' }
Keys 'a.txt'; Start-Sleep -Milliseconds 500
Click 195 20   # command bar "Wiederherstellen"
Start-Sleep -Milliseconds 2500
Check "[W16] restore puts it back (real name with extension)" { (Test-Path "$src\a.txt") -and (Last 'trash Restore done') -match 'ok=true' }
Keys '^l'; Keys 'Dieser PC'; Keys '{ENTER}'; Start-Sleep -Milliseconds 1500
Check "[W17] 'Dieser PC' lists the drives with free space" { (Last 'navigate tab') -match ' ::dieser-pc ' -and (Last 'kinds') -match '(frei von|GB|MB)' }
Keys '{HOME}'; Keys '{ENTER}'; Start-Sleep -Milliseconds 1200
Check "[W17b] opening a drive shows its root" { (Last 'navigate tab') -match 'User [A-Z]:\\ at=' }

# M5.6c: network – \\localhost lists its shares, a share opens, "up" goes back to the server
$shareDir = New-Item -ItemType Directory -Force "$R\freigabe"; Set-Content "$shareDir\im-netz.txt" "n"
$shared = $true
try { New-SmbShare -Name 'e2efreigabe' -Path "$shareDir" -FullAccess 'Everyone' -Description 'E2E Freigabe' | Out-Null }
catch { Write-Host "New-SmbShare failed: $_"; $shared = $false }
Check "[W18a] test share created" { $shared }
Keys '^l'; Keys '\\localhost'; Keys '{ENTER}'; Start-Sleep -Milliseconds 2500
Check "[W18] \\localhost lists its shares (no admin shares)" { (Last 'address=') -match 'address=\\\\localhost\s*$' -and (Last 'order') -match '"e2efreigabe"' -and (Last 'order') -notmatch '\$"' }
Check "[W18b] share comment in the type column" { (Last 'kinds') -match 'E2E Freigabe' }
Keys 'e2e'; Keys '{ENTER}'; Start-Sleep -Milliseconds 2000
Check "[W19] opening a share lists its files" { (Last 'address=') -match 'address=\\\\localhost\\e2efreigabe' -and (Last 'order') -match 'im-netz\.txt' }
Keys '{BACKSPACE}'; Start-Sleep -Milliseconds 2000
Check "[W19b] up from a share goes to the server, share selected" { (Last 'address=') -match 'address=\\\\localhost\s*$' -and (Last 'selection pane') -match 'focus=e2efreigabe' }
Keys '^l'; Keys '\\gibtsnicht-e2e'; Keys '{ENTER}'
[void](WaitFor 'failed tab=.*gibtsnicht-e2e' 30000)   # name resolution may take a while
Check "[W19c] unknown server: error message, previous listing stays" { (Last 'failed tab') -match 'gibtsnicht-e2e' -and (Last 'address=') -match 'address=\\\\localhost\s*$' }
# M5.6d: address bar understands %VAR% and shell: names
# (SendKeys: % is Alt, so the literal percent sign is {%})
Keys '^l'; Keys '{%}TEMP{%}'; Keys '{ENTER}'; Start-Sleep -Milliseconds 1500
Check "[W20] %TEMP% opens the temp folder (long name)" { (Last 'address=') -match 'address=[A-Z]:\\.*\\Temp\s*$' -and (Last 'address=') -notmatch '~' }
Keys '^l'; Keys 'shell:Downloads'; Keys '{ENTER}'; Start-Sleep -Milliseconds 1500
Check "[W21] shell:Downloads opens the Downloads folder" { (Last 'address=') -match 'address=[A-Z]:\\.*\\Downloads\s*$' }
Keys '^l'; Keys 'shell:gibtsnicht'; Keys '{ENTER}'; Start-Sleep -Milliseconds 800
Check "[W22] unknown shell: name gives a clear error" { (Last 'address error') -match 'Unbekannter Shell-Ort: shell:gibtsnicht' }
Keys '{ESC}'
if ($shared) { Remove-SmbShare -Name 'e2efreigabe' -Force -ErrorAction SilentlyContinue }

$p.Refresh()
Check "[W13] app still running" { -not $p.HasExited }
Check "[W14] no panic" { (Log) -notmatch '(?i)panic' }
if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
if ($script:fail) { Write-Host "----- trace"; Write-Host (Log) }
exit $script:fail
