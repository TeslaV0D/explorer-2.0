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
# M5.6d2: libraries – an own test library with two folders
$libDir = Join-Path $env:APPDATA 'Microsoft\Windows\Libraries'
New-Item -ItemType Directory -Force $libDir | Out-Null
$bibA = New-Item -ItemType Directory -Force "$R\bib-a"; $bibB = New-Item -ItemType Directory -Force "$R\bib-b"
Set-Content "$bibA\in-a.txt" "a"
$libXml = @"
<?xml version="1.0" encoding="UTF-8"?>
<libraryDescription xmlns="http://schemas.microsoft.com/windows/2009/library">
  <name>e2ebib</name><version>1</version><isLibraryPinned>true</isLibraryPinned>
  <searchConnectorDescriptionList>
    <searchConnectorDescription><isDefaultSaveLocation>true</isDefaultSaveLocation><simpleLocation><url>$bibA</url></simpleLocation></searchConnectorDescription>
    <searchConnectorDescription><simpleLocation><url>$bibB</url></simpleLocation></searchConnectorDescription>
  </searchConnectorDescriptionList>
</libraryDescription>
"@
Set-Content -Encoding UTF8 "$libDir\e2ebib.library-ms" $libXml
Keys '^l'; Keys 'Bibliotheken'; Keys '{ENTER}'; Start-Sleep -Milliseconds 2500
Check "[W23] 'Bibliotheken' lists the test library with 2 folders" { (Last 'address=') -match 'address=::bibliotheken\s*$' -and (Last 'order') -match '"e2ebib' -and (Last 'kinds') -match '2 Ordner' }
Keys 'e2eb'; Keys '{ENTER}'; Start-Sleep -Milliseconds 2000
Check "[W24] a library with several folders lists them" { (Last 'address=') -match 'address=::bibliothek:e2ebib\s*$' -and (Last 'order') -match 'bib-a' -and (Last 'order') -match 'bib-b' }
Keys '{HOME}'; Keys '{ENTER}'; Start-Sleep -Milliseconds 1500
Check "[W25] opening a library folder shows its files" { (Last 'address=') -match '\\bib-a\s*$' -and (Last 'order') -match 'in-a\.txt' }
Keys '%{LEFT}'; Start-Sleep -Milliseconds 1500; Keys '{BACKSPACE}'; Start-Sleep -Milliseconds 1500
Check "[W26] back to the library, up to 'Bibliotheken'" { (Last 'address=') -match 'address=::bibliotheken\s*$' }
Keys '^l'; Keys 'shell:DocumentsLibrary'; Keys '{ENTER}'; Start-Sleep -Milliseconds 2000
Check "[W27] shell:DocumentsLibrary opens the Documents library" { (Last 'navigate tab') -match 'User ::bibliothek:Documents ' -or (Log) -match 'redirect ::bibliothek:Documents' }
Remove-Item -Force "$libDir\e2ebib.library-ms" -ErrorAction SilentlyContinue
# M5.7: icon view with real thumbnails – a red picture must show up red in its tile
$pics = New-Item -ItemType Directory -Force "$R\bilder"
$bm = New-Object System.Drawing.Bitmap 120, 80
$gr = [System.Drawing.Graphics]::FromImage($bm); $gr.Clear([System.Drawing.Color]::FromArgb(255, 220, 20, 20)); $gr.Dispose()
$bm.Save("$pics\rot.png", [System.Drawing.Imaging.ImageFormat]::Png); $bm.Dispose()
Set-Content "$pics\text.txt" "t"
Keys '^l'; Keys "$pics"; Keys '{ENTER}'; Start-Sleep -Milliseconds 1500
Keys '^+2'; Start-Sleep -Milliseconds 3000
Check "[W28] Ctrl+Shift+2 = large icons" { (Last 'view mode') -match 'mode=large' }
Check "[W28b] thumbnails are loaded for the visible tiles" { (Log) -match 'thumbs loaded n=\d+' }
$geo = Last 'geometry pane=0'; Write-Host "geometry: $geo"
if ($geo -match ' x=([\d.]+) y=([\d.]+) .* item_w=([\d.]+)') {
  $gx = [double]$Matches[1]; $gy = [double]$Matches[2]
  # tile 0 = rot.png (files sort by name); image area: 2 px margin + 4 px padding, 96 px high
  $c = Pt ([int]($gx + 2 + 62)) ([int]($gy + 6 + 48))
  $shot = New-Object System.Drawing.Bitmap 1, 1
  $g2 = [System.Drawing.Graphics]::FromImage($shot); $g2.CopyFromScreen($c.X, $c.Y, 0, 0, (New-Object System.Drawing.Size 1, 1)); $g2.Dispose()
  $px = $shot.GetPixel(0, 0); $shot.Dispose()
  Write-Host "tile 0 center pixel: $px"
  Shot "w28-icons"
  $tl = Last 'thumbs loaded'; Write-Host "thumbs: $tl"
  if ($tl -match 'previews=0 ') {
    # GitHub's Windows Server runners have no picture thumbnail provider: the
    # shell hands out icons, the tile must then show the (non-background) icon
    Write-Host "NOTE  no thumbnail provider on this machine - checking the icon fallback"
    Check "[W28c] the picture's tile shows the shell image (icon fallback)" { -not ($px.R -gt 240 -and $px.G -gt 240 -and $px.B -gt 240) }
  } else {
    Check "[W28c] the picture's tile shows its red thumbnail" { $px.R -gt 180 -and $px.G -lt 90 -and $px.B -lt 90 }
  }
} else { Fail "[W28c] no geometry trace" }
Click ([int]($gx + 2 + 128 + 62)) ([int]($gy + 60))
Check "[W29] click on a tile selects it" { (Last 'selection pane') -match 'focus=text\.txt' }
Keys '{LEFT}'
Check "[W29b] arrow keys move in the grid" { (Last 'selection pane') -match 'focus=rot\.png' }
# M6.1b: rubber band from the empty area right of the two tiles over both
$a = Pt ([int]($gx + 600)) ([int]($gy + 300)); [void][W]::SetCursorPos($a.X, $a.Y); Start-Sleep -Milliseconds 100
[W]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero)
for ($i = 1; $i -le 12; $i++) {
  $b = Pt ([int]($gx + 600 - 590 * $i / 12)) ([int]($gy + 300 - 290 * $i / 12)); [void][W]::SetCursorPos($b.X, $b.Y); Start-Sleep -Milliseconds 50
}
[W]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 600
Check "[W34] rubber band selects both tiles" { (Last 'band done') -match 'hits=2' -and (Last 'selection pane') -match 'count=2 ' }
Keys '^+6'; Start-Sleep -Milliseconds 800
# M6.2: inspector (rows in $pics: 0 rot.png, 1 text.txt)
Keys '%p'; Start-Sleep -Milliseconds 1000
Click $LISTX (RowY 0); Start-Sleep -Milliseconds 2000
Write-Host "inspector: $(Last 'inspector done')"
Check "[W35] inspector: picture size + the shell's preview image" { (Last 'inspector done') -match 'image=true' -and (Last 'inspector done') -match 'Abmessungen=120 × 80 Pixel' }
Keys '%p'; Start-Sleep -Milliseconds 500
Check "[W35b] Alt+P closes the inspector" { (Last 'inspector open') -match 'false' }
# M6.3: command palette
Keys '^+p'; Start-Sleep -Milliseconds 600
Keys 'neuer ordner'; Start-Sleep -Milliseconds 400
Keys '{ENTER}'; Start-Sleep -Milliseconds 1200
Check "[W36] palette: 'neuer ordner' + Enter creates a folder" { (Last 'palette run') -match 'new-folder' -and (Test-Path "$pics\Neuer Ordner") }
Keys '{ESC}'; Start-Sleep -Milliseconds 400
Check "[W30] Ctrl+Shift+6 = details again" { (Last 'view mode') -match 'mode=details' }
if ($shared) { Remove-SmbShare -Name 'e2efreigabe' -Force -ErrorAction SilentlyContinue }

$p.Refresh()
Check "[W13] app still running" { -not $p.HasExited }
Check "[W14] no panic" { (Log) -notmatch '(?i)panic' }
if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
$mainLog = $script:log

# M6.1a: folder options from settings.ini (second start: real registry, real attributes)
$o = New-Item -ItemType Directory -Force "$R\optionen"
Set-Content "$o\brief.txt" "b"; Set-Content "$o\daten.unbekanntxyz" "u"
Set-Content "$o\geschuetzt.dat" "g"; attrib +h +s "$o\geschuetzt.dat"
$cfg2 = "$Work\cfg-opt"; New-Item -ItemType Directory -Force $cfg2 | Out-Null
Set-Content "$cfg2\settings.ini" "show_extensions = false`r`nshow_protected = false`r`nshow_hidden = true"
$script:log = "$Work\win-e2e-opt.log"; $env:EXPLORER2_CONFIG_DIR = $cfg2
$p = Start-Process -FilePath $Exe -ArgumentList "`"$o`"" -PassThru -RedirectStandardError $script:log
if (WaitFor 'done tab=1 ' 15000) {
  Start-Sleep -Seconds 1
  $p.Refresh(); $script:hwnd = $p.MainWindowHandle
  [void][W]::ShowWindow($script:hwnd, 3); Start-Sleep -Milliseconds 800
  Focus
  Write-Host "shown: $(Last 'shown')"
  Check "[W31] known extension hidden (.txt), unknown kept" { (Last 'shown') -match '"brief"' -and (Last 'shown') -match '"daten\.unbekanntxyz"' }
  Check "[W31b] protected system file hidden although 'show hidden' is on" { (Last 'shown') -notmatch 'geschuetzt' }
  # rows: 0 brief, 1 daten.unbekanntxyz
  Click $LISTX (RowY 0)
  Keys '{F2}'
  Check "[W32] F2 edits the stem only" { (Last 'rename start') -match 'edit=brief hidden_ext=txt' }
  Keys 'neu'; Keys '{ENTER}'; Start-Sleep -Milliseconds 1000
  Check "[W32b] rename keeps the hidden extension (brief.txt -> neu.txt)" { (Test-Path "$o\neu.txt") -and -not (Test-Path "$o\neu") }
  $p.Refresh()
  Check "[W33] app still running" { -not $p.HasExited }
} else { Fail "[W31] second start did not load" }
if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
Check "[W33b] no panic" { (Log) -notmatch '(?i)panic' }
if ($script:fail) { Write-Host "----- trace (options)"; Write-Host (Log) }
$optFail = $script:fail

# M6.4: colour scheme follows Windows live (real registry; restored afterwards)
function Pixel([int]$x, [int]$y) {
  $c = Pt $x $y
  $b = New-Object System.Drawing.Bitmap 1, 1
  $g = [System.Drawing.Graphics]::FromImage($b); $g.CopyFromScreen($c.X, $c.Y, 0, 0, (New-Object System.Drawing.Size 1, 1)); $g.Dispose()
  $px = $b.GetPixel(0, 0); $b.Dispose(); return $px
}
function Near($px, [int]$r, [int]$g, [int]$b) { [math]::Abs($px.R - $r) -le 3 -and [math]::Abs($px.G - $g) -le 3 -and [math]::Abs($px.B - $b) -le 3 }
function Palette([int[]]$rgb) { # 8 shades, RGBA; only Light 2 / Accent / Dark 1 matter
  $bytes = New-Object byte[] 32
  for ($i = 0; $i -lt 8; $i++) { $bytes[$i * 4] = $rgb[0]; $bytes[$i * 4 + 1] = $rgb[1]; $bytes[$i * 4 + 2] = $rgb[2] }
  return ,$bytes
}
$pers = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize'
$acc = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Accent'
$origLight = (Get-ItemProperty $pers -Name AppsUseLightTheme -ErrorAction SilentlyContinue).AppsUseLightTheme
$accExisted = Test-Path $acc
$origPal = (Get-ItemProperty $acc -Name AccentPalette -ErrorAction SilentlyContinue).AccentPalette
if (-not (Test-Path $pers)) { New-Item -Path $pers -Force | Out-Null }
if (-not $accExisted) { New-Item -Path $acc -Force | Out-Null }
Set-ItemProperty $pers -Name AppsUseLightTheme -Value 1 -Type DWord
Set-ItemProperty $acc -Name AccentPalette -Value (Palette @(0, 120, 212)) -Type Binary
$th = New-Item -ItemType Directory -Force "$R\farben"
Set-Content "$th\a.txt" "a"; Set-Content "$th\b.txt" "b"
$cfg3 = "$Work\cfg-theme"; New-Item -ItemType Directory -Force $cfg3 | Out-Null
$script:log = "$Work\win-e2e-theme.log"; $env:EXPLORER2_CONFIG_DIR = $cfg3
$p = Start-Process -FilePath $Exe -ArgumentList "`"$th`"" -PassThru -RedirectStandardError $script:log
try {
  if (WaitFor 'done tab=1 ' 15000) {
    Start-Sleep -Seconds 1
    $p.Refresh(); $script:hwnd = $p.MainWindowHandle
    [void][W]::ShowWindow($script:hwnd, 3); Start-Sleep -Milliseconds 800
    Focus
    Write-Host "theme: $(Last 'theme why=init')"
    Check "[W37] start: light system, accent from AccentPalette" { (Last 'theme why=init') -match 'choice=system dark=false .*system_dark=false system_accent=#0078d4' }
    Check "[W37b] title bar set" { (Log) -match 'theme title bar dark=false' }
    $bg = Pixel $LISTX (RowY 6); Write-Host "list background: $bg"
    Check "[W37c] list background light" { Near $bg 250 250 250 }
    Set-ItemProperty $pers -Name AppsUseLightTheme -Value 0 -Type DWord
    Check "[W38] Windows switched to dark -> the app follows live" { WaitFor 'theme why=system choice=system dark=true' 5000 }
    Start-Sleep -Milliseconds 800
    $bg = Pixel $LISTX (RowY 6); Write-Host "list background: $bg"; Shot "w38-dark"
    Check "[W38b] list background dark (#191919)" { Near $bg 25 25 25 }
    Check "[W38c] title bar dark" { WaitFor 'theme title bar dark=true' 3000 }
    Set-ItemProperty $acc -Name AccentPalette -Value (Palette @(16, 124, 16)) -Type Binary
    Check "[W39] accent changed in Windows -> selection follows live" { WaitFor 'system_accent=#107c10' 5000 }
    Click $LISTX (RowY 0); Start-Sleep -Milliseconds 600
    $sel = Pixel ($LISTX + 150) (RowY 0); Write-Host "selection: $sel"
    Check "[W39b] the selection is green now" { $sel.G -gt $sel.R + 20 -and $sel.G -gt $sel.B + 20 }
    Set-ItemProperty $pers -Name AppsUseLightTheme -Value 1 -Type DWord
    Check "[W40] back to light" { WaitFor 'theme why=system choice=system dark=false' 5000 }
    $p.Refresh()
    Check "[W40b] app still running, no panic" { -not $p.HasExited -and (Log) -notmatch '(?i)panic' }
  } else { Fail "[W37] theme start did not load" }
} finally {
  if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
  # restore the runner's registry exactly
  if ($null -eq $origLight) { Remove-ItemProperty $pers -Name AppsUseLightTheme -ErrorAction SilentlyContinue }
  else { Set-ItemProperty $pers -Name AppsUseLightTheme -Value $origLight -Type DWord }
  if (-not $accExisted) { Remove-Item $acc -Recurse -Force -ErrorAction SilentlyContinue }
  elseif ($null -eq $origPal) { Remove-ItemProperty $acc -Name AccentPalette -ErrorAction SilentlyContinue }
  else { Set-ItemProperty $acc -Name AccentPalette -Value $origPal -Type Binary }
}
if ($script:fail -and -not $optFail) { Write-Host "----- trace (theme)"; Write-Host (Log) }
$themeFail = $script:fail

# R-001: a real integration (registry + environment + file) is undone by the app's button
$selftest = Join-Path (Split-Path $Exe) 'examples\r001_selftest.exe'
if (Test-Path $selftest) {
  $cfg4 = "$Work\cfg-integration"; New-Item -ItemType Directory -Force $cfg4 | Out-Null
  $env:EXPLORER2_CONFIG_DIR = $cfg4
  $lnk = "$Work\r001-startmenu.lnk"
  & reg.exe add 'HKCU\Software\Explorer2R001Probe' /v keep /t REG_SZ /d 'original value' /f | Out-Null
  & $selftest full $lnk | Out-Host
  $verbKey = 'HKCU:\Software\Classes\Directory\shell\open\command'
  Check "[W41] integration applied (folder verb, environment, file)" { (Get-ItemProperty $verbKey -ErrorAction SilentlyContinue).'(default)' -match 'explorer2' -and (Test-Path $lnk) }
  $script:log = "$Work\win-e2e-integration.log"
  $p = Start-Process -FilePath $Exe -ArgumentList "`"$R`"" -PassThru -RedirectStandardError $script:log
  try {
    if (WaitFor 'done tab=1 ' 15000) {
      Start-Sleep -Seconds 1
      $p.Refresh(); $script:hwnd = $p.MainWindowHandle
      [void][W]::ShowWindow($script:hwnd, 3); Start-Sleep -Milliseconds 800
      Focus
      Keys '^+p'; Keys 'integrationen zur'; Keys '{ENTER}'
      Check "[W41b] palette: the reset asks first" { WaitFor 'confirm integration reset' 3000 }
      Keys '{ENTER}'
      Check "[W41c] reset ran without errors" { (WaitFor 'integration reset restored' 5000) -and (Last 'integration reset restored') -match 'errors=0 journal=Ok' }
      Write-Host "reset: $(Last 'integration reset restored')"
      Check "[W41d] folder verb gone again" { -not (Test-Path $verbKey) }
      Check "[W41e] overwritten value restored exactly" { (Get-ItemProperty 'HKCU:\Software\Explorer2R001Probe').keep -eq 'original value' }
      Check "[W41f] environment variable and file removed" { $null -eq (Get-ItemProperty 'HKCU:\Environment').EXPLORER2_R001 -and -not (Test-Path $lnk) }
      Check "[W41g] journal gone, status: nothing active" { -not (Test-Path "$cfg4\integration-journal.txt") -and (Last 'integration status') -match 'entries=0 active_points=0' }
      $p.Refresh()
      Check "[W41h] app still running, no panic" { -not $p.HasExited -and (Log) -notmatch '(?i)panic' }
    } else { Fail "[W41] integration start did not load" }
  } finally {
    if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
    & reg.exe delete 'HKCU\Software\Explorer2R001Probe' /f 2>$null | Out-Null
  }
  if ($script:fail -and -not $themeFail) { Write-Host "----- trace (integration)"; Write-Host (Log) }
} else { Fail "[W41] $selftest missing (build the r001_selftest example)" }
$script:log = $mainLog
if ($script:fail) { Write-Host "----- trace"; Write-Host (Log) }
exit $script:fail
