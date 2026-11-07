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
$ROWH = 24; $LISTX = 330
# click targets by name (redesign R0): the app writes every named element
# (accessible-label / Text) with its client-area position to this file
$env:EXPLORER2_UI_DUMP = Join-Path $Work 'ui-dump.txt'
function UiBox([string]$label, $which = 1) {
  $lines = @(Get-Content $env:EXPLORER2_UI_DUMP -Encoding UTF8 -ErrorAction SilentlyContinue | Select-Object -Skip 1 |
    Where-Object { ($_ -split "`t")[0] -ceq $label })
  if ($lines.Count -eq 0) { return $null }
  $k = if ($which -eq 'last') { $lines.Count - 1 } else { [int]$which - 1 }
  if ($k -ge $lines.Count) { return $null }
  $f = $lines[$k] -split "`t"
  return @{ X = [int]$f[1]; Y = [int]$f[2]; W = [int]$f[3]; H = [int]$f[4] }
}
function UiClick([string]$label, $which = 1, [string]$button = 'left') {
  $b = $null
  for ($i = 0; $i -lt 30 -and $null -eq $b; $i++) { $b = UiBox $label $which; if ($null -eq $b) { Start-Sleep -Milliseconds 100 } }
  if ($null -eq $b) { Write-Host "UI   target not found: '$label'"; return $false }
  Click ($b.X + [int]($b.W / 2)) ($b.Y + [int]($b.H / 2)) $button; return $true
}
# first row = below the "Name" column header (falls back to the old layout)
function ListTop { $b = UiBox 'Name'; if ($null -eq $b) { 140 } else { $b.Y + $b.H } }
function Pt([int]$x, [int]$y) {
  $p = New-Object W+POINT; $p.X = [int]($x * $script:scale); $p.Y = [int]($y * $script:scale)
  [void][W]::ClientToScreen($script:hwnd, [ref]$p); return $p
}
function RowY([int]$i) { (ListTop) + $ROWH * $i + 12 }
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
Keys '%{ENTER}'
# title like "a Properties" / "Eigenschaften von a" (the shell hides known extensions).
# The shell builds the dialog asynchronously and sometimes slowly: poll, and make sure it
# is closed again - a late dialog would steal the focus from the following steps (CI #60)
$propRe = '(^|\|)(a Properties|Eigenschaften von a)(\.txt)?\|'
$sw = [Diagnostics.Stopwatch]::StartNew()
do { Start-Sleep -Milliseconds 250; $titles = [W]::Titles([uint32]$p.Id) } while ($titles -notmatch $propRe -and $sw.ElapsedMilliseconds -lt 10000)
Write-Host "windows of the app after $($sw.ElapsedMilliseconds) ms: $titles"
Check "[W11] Alt+Enter opens the properties dialog" { $titles -match $propRe }
$sw.Restart()
do {
  foreach ($t in 'Properties', 'Eigenschaften') { [W]::CloseTitled([uint32]$p.Id, $t) }
  Start-Sleep -Milliseconds 300; $titles = [W]::Titles([uint32]$p.Id)
} while ($titles -match $propRe -and $sw.ElapsedMilliseconds -lt 5000)
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
$failBefore = $script:fail; $script:fail = 0
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
  if ($script:fail) { Write-Host "----- trace (integration)"; Write-Host (Log) }
} else { Fail "[W41] $selftest missing (build the r001_selftest example)" }
$script:fail = $script:fail -bor $failBefore

# M7.1: folder verbs + context menu, switched on/off in the app; the real
# ShellExecute path ("open folder" of any program) must start Explorer 2.0
$failBefore = $script:fail; $script:fail = 0
$cfg5 = "$Work\cfg-m71"; New-Item -ItemType Directory -Force $cfg5 | Out-Null
$env:EXPLORER2_CONFIG_DIR = $cfg5
$launched = "$Work\launched.log"; Remove-Item $launched -ErrorAction SilentlyContinue
$env:EXPLORER2_TRACE_FILE = $launched   # inherited by instances that Windows starts
$classes = 'HKCU\Software\Classes'
function SnapM7([string]$name) {
  # Windows PowerShell 5.1 turns reg.exe's stderr ("key not found") into a
  # terminating error under 'Stop' (CI #62) -> local 'Continue'
  $ErrorActionPreference = 'Continue'
  foreach ($k in 'Directory', 'Drive', 'Folder') {
    $f = "$Work\m7-$name-$k.reg"
    & reg.exe export "$classes\$k" $f /y 2>$null | Out-Null
    if ($LASTEXITCODE -eq 0) { "$k=" + (Get-FileHash $f).Hash } else { "$k=(absent)" }
  }
}
function Instances { @(Get-CimInstance Win32_Process -Filter "Name='explorer2.exe'" | Where-Object { $_.ProcessId -ne $p.Id }) }
function KillInstances { Instances | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue } }
function Launched([string]$pattern, [int]$ms = 8000) {
  $sw = [Diagnostics.Stopwatch]::StartNew()
  while ($sw.ElapsedMilliseconds -lt $ms) {
    if ((Test-Path $launched) -and ((Get-Content $launched -Raw) -match $pattern)) { return $true }
    Start-Sleep -Milliseconds 200
  }
  return $false
}
$m7before = SnapM7 'before'
Write-Host "classes before: $($m7before -join ', ')"
$target = New-Item -ItemType Directory -Force "$R\m71 ziel"
Set-Content "$target\drin.txt" "x"
$script:log = "$Work\win-e2e-m71.log"
$p = Start-Process -FilePath $Exe -ArgumentList "`"$R`"" -PassThru -RedirectStandardError $script:log
try {
  if (WaitFor 'done tab=1 ' 15000) {
    Start-Sleep -Seconds 1
    $p.Refresh(); $script:hwnd = $p.MainWindowHandle
    [void][W]::ShowWindow($script:hwnd, 3); Start-Sleep -Milliseconds 800
    Focus
    Keys '^+p'; Keys 'ordner mit explorer'; Keys '{ENTER}'; Start-Sleep -Milliseconds 800
    Check "[W42] palette switches 'open folders with Explorer 2.0' on" { (Last 'integration feature folder-open on') -match 'n=8' }
    $cmd = (Get-ItemProperty "HKCU:\Software\Classes\Directory\shell\explorer2open\command" -ErrorAction SilentlyContinue).'(default)'
    $def = (Get-ItemProperty "HKCU:\Software\Classes\Directory\shell" -ErrorAction SilentlyContinue).'(default)'
    Write-Host "registered: default verb '$def', command $cmd"
    Check "[W42b] our verb is the default verb and points to this exe" { $def -eq 'explorer2open' -and $cmd -eq ('"' + $Exe + '" "%1"') }
    Check "[W42c] the recovery tool was copied to LOCALAPPDATA" { Test-Path "$env:LOCALAPPDATA\Explorer2\explorer2-recover.exe" }
    # what any program does to open a folder (ShellExecute, default verb)
    Start-Process -FilePath $target.FullName
    Check "[W42d] ShellExecute on a folder starts Explorer 2.0 there" { Launched ([regex]::Escape("start=$($target.FullName)")) }
    KillInstances
    Start-Process -FilePath 'C:\'
    Check "[W42e] a drive root arrives intact (`"C:\`" quoting)" { Launched 'start=C:\\\s' }
    KillInstances
    Keys '^+p'; Keys 'im kontextmenu ein'; Keys '{ENTER}'; Start-Sleep -Milliseconds 800
    Check "[W42f] palette switches the context menu entry on" { (Last 'integration feature context-menu on') -match 'n=9' }
    $sh = New-Object -ComObject Shell.Application
    $item = $sh.Namespace($R).ParseName((Split-Path $target.FullName -Leaf))
    $verbs = @($item.Verbs() | ForEach-Object { $_.Name })
    Write-Host "verbs: $($verbs -join ' | ')"
    Check "[W42g] 'In Explorer 2.0 öffnen' is in the folder's context menu" { $verbs -match 'Explorer 2\.0' }
    Remove-Item $launched -ErrorAction SilentlyContinue
    $item.InvokeVerb('explorer2')
    Check "[W42h] the context menu entry opens the folder in Explorer 2.0" { Launched ([regex]::Escape("start=$($target.FullName)")) }
    KillInstances
    Check "[W42i] Folder (Control Panel, Recycle Bin …) is not touched" { (SnapM7 'mid')[2] -eq $m7before[2] }
    # both off again -> exactly as before
    Keys '^+p'; Keys 'ordner mit explorer'; Keys '{ENTER}'; Start-Sleep -Milliseconds 800
    Keys '^+p'; Keys 'im kontextmenu ein'; Keys '{ENTER}'; Start-Sleep -Milliseconds 800
    Check "[W42j] both switched off without errors" { (Last 'integration feature folder-open off') -match 'errors=0' -and (Last 'integration feature context-menu off') -match 'errors=0' }
    $after = SnapM7 'after'
    Check "[W42k] registry exactly as before ($($after -join ', '))" { -not (Compare-Object $m7before $after) }
    Check "[W42l] journal empty again" { -not (Test-Path "$cfg5\integration-journal.txt") }
    $p.Refresh()
    Check "[W42m] app still running, no panic" { -not $p.HasExited -and (Log) -notmatch '(?i)panic' }
  } else { Fail "[W42] M7.1 start did not load" }
} finally {
  KillInstances
  if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
  # never leave the runner with our verbs (later steps open folders)
  $rec = Join-Path (Split-Path $Exe) 'explorer2-recover.exe'
  if (Test-Path $rec) { & $rec --yes | Out-Null }
  Remove-Item Env:\EXPLORER2_TRACE_FILE -ErrorAction SilentlyContinue
}
if ($script:fail) {
  Write-Host "----- trace (M7.1)"; Write-Host (Log)
  if (Test-Path $launched) { Write-Host "----- launched instances"; Get-Content $launched | Write-Host }
}
$script:fail = $script:fail -bor $failBefore

# M7.2: Win+E opens Explorer 2.0 (real key press; only where the runner's shell
# handles Win+E at all - checked first with Windows' own Explorer)
$failBefore = $script:fail; $script:fail = 0
function PressWinE {
  [W]::keybd_event(0x5B, 0, 0, [UIntPtr]::Zero); [W]::keybd_event(0x45, 0, 0, [UIntPtr]::Zero)
  [W]::keybd_event(0x45, 0, 2, [UIntPtr]::Zero); [W]::keybd_event(0x5B, 0, 2, [UIntPtr]::Zero)
}
$shellApp = New-Object -ComObject Shell.Application
function ExplorerWindows { @($shellApp.Windows()).Count }
function CloseExplorerWindows { foreach ($w in @($shellApp.Windows())) { try { $w.Quit() } catch {} } }
$clsidKey = 'HKCU\Software\Classes\CLSID\{52205FD8-5DFB-447D-801A-D0B52F2E83E1}'
function SnapWinE([string]$name) {
  $ErrorActionPreference = 'Continue'
  $f = "$Work\m72-$name.reg"
  & reg.exe export $clsidKey $f /y 2>$null | Out-Null
  if ($LASTEXITCODE -eq 0) { (Get-FileHash $f).Hash } else { '(absent)' }
}
CloseExplorerWindows; Start-Sleep -Milliseconds 500
$n0 = ExplorerWindows; PressWinE; Start-Sleep -Seconds 4
$baseline = (ExplorerWindows) -gt $n0
Write-Host "Win+E baseline: Explorer windows $n0 -> $(ExplorerWindows) (handled by the shell: $baseline)"
CloseExplorerWindows; Start-Sleep -Milliseconds 500
$winEBefore = SnapWinE 'before'
$cfg6 = "$Work\cfg-m72"; New-Item -ItemType Directory -Force $cfg6 | Out-Null
$env:EXPLORER2_CONFIG_DIR = $cfg6
Remove-Item $launched -ErrorAction SilentlyContinue
$env:EXPLORER2_TRACE_FILE = $launched
$script:log = "$Work\win-e2e-m72.log"
$p = Start-Process -FilePath $Exe -ArgumentList "`"$R`"" -PassThru -RedirectStandardError $script:log
try {
  if (WaitFor 'done tab=1 ' 15000) {
    Start-Sleep -Seconds 1
    $p.Refresh(); $script:hwnd = $p.MainWindowHandle
    [void][W]::ShowWindow($script:hwnd, 3); Start-Sleep -Milliseconds 800
    Focus
    Keys '^+p'; Keys 'win{+}e'; Keys '{ENTER}'; Start-Sleep -Milliseconds 800
    Check "[W43] palette switches 'Win+E opens Explorer 2.0' on" { (Last 'integration feature win-e on') -match 'n=2' }
    $cmd = (Get-ItemProperty "HKCU:\Software\Classes\CLSID\{52205FD8-5DFB-447D-801A-D0B52F2E83E1}\shell\opennewwindow\command" -ErrorAction SilentlyContinue)
    Write-Host "registered: $($cmd.'(default)') DelegateExecute='$($cmd.DelegateExecute)'"
    Check "[W43b] Win+E verb points to this exe (empty DelegateExecute)" { $cmd.'(default)' -eq ('"' + $Exe + '"') -and $cmd.DelegateExecute -eq '' }
    if ($baseline) {
      Remove-Item $launched -ErrorAction SilentlyContinue
      $n1 = ExplorerWindows
      KillInstances
      PressWinE
      # @(...): PS 5.1 unwraps a one-element result and CimInstance.Count is $null (CI #66)
      # started by explorer.exe (the shell), not by us: our environment (and so
      # EXPLORER2_TRACE_FILE) is not inherited -> look at the process itself
      $sw = [Diagnostics.Stopwatch]::StartNew()
      do { Start-Sleep -Milliseconds 250; $inst = @(Instances) } while ($inst.Count -eq 0 -and $sw.ElapsedMilliseconds -lt 8000)
      $inst | ForEach-Object { Write-Host "instance: pid=$($_.ProcessId) parent=$($_.ParentProcessId) cmd=$($_.CommandLine)" }
      Check "[W43c] Win+E starts Explorer 2.0 (without a folder)" { $inst.Count -ge 1 -and ($inst | Where-Object { $_.CommandLine -match 'explorer2\.exe"?\s*$' }) }
      Start-Sleep -Milliseconds 800
      Check "[W43d] ... and no Windows Explorer window" { (ExplorerWindows) -le $n1 }
      KillInstances; CloseExplorerWindows
    } else {
      Write-Host "NOTE  the runner's shell does not handle Win+E (no Explorer window in the baseline) - registry checked only"
    }
    Keys '^+p'; Keys 'win{+}e'; Keys '{ENTER}'; Start-Sleep -Milliseconds 800
    Check "[W43e] switched off without errors" { (Last 'integration feature win-e off') -match 'errors=0' }
    $winEAfter = SnapWinE 'after'
    Check "[W43f] CLSID key exactly as before ($winEBefore)" { $winEAfter -eq $winEBefore }
    if ($baseline) {
      $n2 = ExplorerWindows; PressWinE; Start-Sleep -Seconds 4
      Check "[W43g] off again: Win+E opens Windows' Explorer as before" { (ExplorerWindows) -gt $n2 }
      CloseExplorerWindows
    }
    $p.Refresh()
    Check "[W43h] app still running, no panic" { -not $p.HasExited -and (Log) -notmatch '(?i)panic' }
  } else { Fail "[W43] M7.2 start did not load" }
} finally {
  KillInstances
  if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
  $rec = Join-Path (Split-Path $Exe) 'explorer2-recover.exe'
  if (Test-Path $rec) { & $rec --yes | Out-Null }
  Remove-Item Env:\EXPLORER2_TRACE_FILE -ErrorAction SilentlyContinue
}
if ($script:fail) {
  Write-Host "----- trace (M7.2)"; Write-Host (Log)
  if (Test-Path $launched) { Write-Host "----- launched instances"; Get-Content $launched | Write-Host }
}
$script:fail = $script:fail -bor $failBefore

# ---- S1.4 search (W44): index of a test folder, search field, delete a hit
$failSearch = $script:fail
$sr = New-Item -ItemType Directory -Force "$Work\suche-root"
New-Item -ItemType Directory -Force "$sr\a\b" | Out-Null
1..12 | ForEach-Object { Set-Content "$sr\a\datei$_.txt" "x$_" }
Set-Content "$sr\a\b\Tief.PDF" "t"
$cfg7 = "$Work\cfg-search"; New-Item -ItemType Directory -Force $cfg7 | Out-Null
$env:EXPLORER2_CONFIG_DIR = $cfg7; $env:EXPLORER2_INDEX = 'on'
$env:EXPLORER2_INDEX_ROOTS = $sr.FullName; $env:EXPLORER2_INDEX_DELAY_MS = '300'
$script:log = "$Work\win-e2e-search.log"
$p = Start-Process -FilePath $Exe -ArgumentList "`"$sr`"" -PassThru -RedirectStandardError $script:log
try {
  if (WaitFor 'done tab=1 ' 15000) {
    # root, a, b, 12 files, Tief.PDF
    Check "[W44] index of the folder built in the background" { WaitFor 'phase=Ready entries=16 origin=built' 15000 }
    Start-Sleep -Seconds 1
    $p.Refresh(); $script:hwnd = $p.MainWindowHandle
    [void][W]::ShowWindow($script:hwnd, 3); Start-Sleep -Milliseconds 800
    Focus
    Keys '^f'; Keys 'datei1'; Start-Sleep -Milliseconds 1200
    Check "[W44b] Ctrl+F + typing: index search (datei1, datei10-12 = 4 hits)" { WaitFor 'search found query="datei1" total=4 shown=4 walked=false' 5000 }
    Keys '^a'; Keys 'ext:pdf'; Start-Sleep -Milliseconds 1200
    Check "[W44c] ext:pdf finds Tief.PDF (no case on Windows)" { WaitFor 'search found query="ext:pdf" total=1 ' 5000 }
    Keys '{DOWN}'; Keys '{HOME}'; Keys '{DELETE}'; Start-Sleep -Milliseconds 1500
    Check "[W44d] Delete on a hit: into the recycle bin" { -not (Test-Path "$sr\a\b\Tief.PDF") }
    Check "[W44e] the results follow the index (0 hits)" { WaitFor 'search found query="ext:pdf" total=0 ' 8000 }
    Keys '^f'; Keys '{ESC}'
    Check "[W44f] Esc: back to the folder" { WaitFor 'search pane=0 closed' 3000 }
    # S1.4c quick search: Ctrl+Shift+F, exact name first, Ctrl+Enter shows it in its folder
    Keys '^+f'; Keys 'datei1'; Start-Sleep -Milliseconds 1000
    Check "[W45] quick search: datei1.txt first of 4" { WaitFor 'quick search query="datei1" total=4 shown=4 first=Some\("datei1.txt"\)' 4000 }
    Keys '^{ENTER}'; Start-Sleep -Milliseconds 1500
    Check "[W45b] Ctrl+Enter shows it selected in its folder" { (WaitFor 'show in folder .*datei1\.txt' 3000) -and (WaitFor 'selection pane=0 .*focus=datei1\.txt' 3000) }
    $p.Refresh()
    Check "[W44g] app still running, no panic" { -not $p.HasExited -and (Log) -notmatch '(?i)panic' }
  } else { Fail "[W44] search start did not load" }
} finally {
  if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
  Remove-Item Env:\EXPLORER2_INDEX_ROOTS, Env:\EXPLORER2_INDEX_DELAY_MS -ErrorAction SilentlyContinue
  $env:EXPLORER2_INDEX = 'off'
}
if ($script:fail -ne $failSearch) { Write-Host "----- trace (search)"; Write-Host (Log) }

# ---- UI gallery (redesign R0): the real Windows look (Segoe UI, shell icons), light + dark
$gal = New-Item -ItemType Directory -Force "$Work\gallery"
foreach ($theme in @('light', 'dark')) {
  $cfgG = "$Work\cfg-gallery-$theme"; New-Item -ItemType Directory -Force $cfgG | Out-Null
  Set-Content "$cfgG\settings.ini" "theme = $theme`nsystem_accent = false"
  $env:EXPLORER2_CONFIG_DIR = $cfgG
  $script:log = "$Work\win-gallery-$theme.log"
  $p = Start-Process -FilePath $Exe -ArgumentList "`"$sr`"" -PassThru -RedirectStandardError $script:log
  try {
    if (WaitFor 'done tab=1 ' 15000) {
      Start-Sleep -Seconds 1
      $p.Refresh(); $script:hwnd = $p.MainWindowHandle
      [void][W]::ShowWindow($script:hwnd, 3); Start-Sleep -Milliseconds 800
      Focus
      Click $LISTX (RowY 0); Keys '{RIGHT}'
      Shot "gallery\$theme-01-ordner"
      Keys '^f'; Keys 'datei'; Start-Sleep -Milliseconds 1200; Shot "gallery\$theme-02-suche"
      Keys '{ESC}'; Start-Sleep -Milliseconds 600
      Keys '^+p'; Start-Sleep -Milliseconds 500; Shot "gallery\$theme-03-befehlspalette"; Keys '{ESC}'
      if (UiClick 'Einstellungen') { Start-Sleep -Milliseconds 800; Shot "gallery\$theme-04-einstellungen" }
      Click $LISTX (RowY 1) 'right'; Start-Sleep -Milliseconds 1200; Shot "gallery\$theme-05-kontextmenue"; Keys '{ESC}'
    }
  } finally {
    if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
  }
}
Write-Host "gallery: $(@(Get-ChildItem "$gal\*.png").Count) screenshots"
$script:log = $mainLog
if ($script:fail) { Write-Host "----- trace"; Write-Host (Log) }
exit $script:fail
