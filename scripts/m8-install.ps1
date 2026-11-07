# M8 install / update / uninstall on Windows with the real default paths
# (%LOCALAPPDATA%\Programs\Explorer2, Start menu, Apps & Features).
#   pwsh scripts/m8-install.ps1 <release dir> <work dir>
param([Parameter(Mandatory)] [string]$Bin, [Parameter(Mandatory)] [string]$Work)
$ErrorActionPreference = 'Continue'
$script:fail = 0
function Pass($m) { Write-Host "PASS  $m" }
function Fail($m) { Write-Host "FAIL  $m"; $script:fail = 1 }
function Check($m, [scriptblock]$cond) { if (& $cond) { Pass $m } else { Fail $m } }

New-Item -ItemType Directory -Force $Work | Out-Null
$env:EXPLORER2_CONFIG_DIR = Join-Path $Work 'cfg'
$dl = Join-Path $Work 'Downloads'   # the copy a user downloaded
New-Item -ItemType Directory -Force $dl | Out-Null
Copy-Item (Join-Path $Bin 'explorer2.exe'), (Join-Path $Bin 'explorer2-recover.exe') $dl
$self = Join-Path $Bin 'examples\r001_selftest.exe'
$prog = Join-Path $env:LOCALAPPDATA 'Programs\Explorer2'
$menu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Explorer 2.0'
$unKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Explorer2'
$areas = @('HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Explorer2',
  'HKCU\Software\Classes\Directory', 'HKCU\Software\Classes\Drive',
  'HKCU\Software\Classes\CLSID\{52205FD8-5DFB-447D-801A-D0B52F2E83E1}')
function Snap([string]$name) {
  $lines = foreach ($k in $areas) {
    $f = Join-Path $Work ("$name-" + ($k -replace '[\\:{}]', '_') + '.reg')
    & reg.exe export $k $f /y 2>$null | Out-Null
    if ($LASTEXITCODE -eq 0) { "$k = " + (Get-FileHash $f).Hash } else { "$k = (absent)" }
  }
  $lines += "program dir = $(Test-Path $prog)"
  $lines += "start menu = $(Test-Path $menu)"
  return $lines
}
function RunGui([string]$exe, [string[]]$argv) {
  # GUI-subsystem exe: Start-Process -Wait with redirected output
  $o = Join-Path $Work ("out-" + [guid]::NewGuid().ToString('N') + '.txt')
  $p = Start-Process -FilePath $exe -ArgumentList $argv -Wait -PassThru -NoNewWindow -RedirectStandardOutput $o
  $text = Get-Content $o -Raw
  Write-Host "    > $(Split-Path $exe -Leaf) $($argv -join ' ') (exit $($p.ExitCode)): $text"
  return @{ code = $p.ExitCode; text = $text }
}
function Instances { @(Get-CimInstance Win32_Process -Filter "Name='explorer2.exe'") }

$before = Snap 'before'
Write-Host "before:"; $before | ForEach-Object { Write-Host "    $_" }

Write-Host "== an integration switched on from the downloaded copy"
& $self features | Out-Host
$cmd0 = (Get-ItemProperty 'HKCU:\Software\Classes\Directory\shell\explorer2open\command').'(default)'
Write-Host "    folder verb before install: $cmd0"

Write-Host "== install"
$r = RunGui (Join-Path $dl 'explorer2.exe') @('--install', '--quiet')
Check "[I1] --install ok" { $r.code -eq 0 -and $r.text -match 'installiert' }
Check "[I1b] program files in %LOCALAPPDATA%\Programs\Explorer2" { (Test-Path "$prog\explorer2.exe") -and (Test-Path "$prog\explorer2-recover.exe") }
$un = Get-ItemProperty $unKey -ErrorAction SilentlyContinue
Write-Host "    Apps & Features: $($un.DisplayName) $($un.DisplayVersion) | $($un.UninstallString)"
Check "[I2] 'Apps & Features' entry with version and uninstaller" { $un.DisplayName -eq 'Explorer 2.0' -and $un.DisplayVersion -match '^\d+\.\d+\.\d+' -and $un.UninstallString -eq ('"' + "$prog\explorer2-recover.exe" + '" --uninstall') -and $un.EstimatedSize -gt 1000 }
$sh = New-Object -ComObject WScript.Shell
$lnkApp = Join-Path $menu 'Explorer 2.0.lnk'
$lnkRec = Join-Path $menu 'Explorer 2.0 - Windows-Standard wiederherstellen.lnk'
Check "[I3] Start menu: 'Explorer 2.0' points to the installed exe" { (Test-Path $lnkApp) -and $sh.CreateShortcut($lnkApp).TargetPath -eq "$prog\explorer2.exe" }
Check "[I3b] Start menu: 'Windows-Standard wiederherstellen' points to the recovery tool" { (Test-Path $lnkRec) -and $sh.CreateShortcut($lnkRec).TargetPath -eq "$prog\explorer2-recover.exe" }
$cmd1 = (Get-ItemProperty 'HKCU:\Software\Classes\Directory\shell\explorer2open\command').'(default)'
Write-Host "    folder verb after install: $cmd1"
Check "[I4] the active integration now points to the installed exe" { $cmd1 -eq ('"' + "$prog\explorer2.exe" + '" "%1"') }

Write-Host "== start from the Start menu, update while it runs"
Start-Process -FilePath $lnkApp
$sw = [Diagnostics.Stopwatch]::StartNew()
do { Start-Sleep -Milliseconds 300; $inst = @(Instances | Where-Object { $_.ExecutablePath -eq "$prog\explorer2.exe" }) } while ($inst.Count -eq 0 -and $sw.ElapsedMilliseconds -lt 10000)
Check "[I5] the Start menu entry starts the installed copy" { $inst.Count -ge 1 }
# a "new version": a few bytes appended (an overlay does not change how the exe runs)
[System.IO.File]::AppendAllText((Join-Path $dl 'explorer2.exe'), "update-$PID")
$r = RunGui (Join-Path $dl 'explorer2.exe') @('--install', '--quiet')
Check "[I6] update over the running app works (exe renamed aside)" { $r.code -eq 0 -and (Test-Path "$prog\explorer2.exe") -and @(Get-ChildItem $prog -Filter '*.old-*').Count -ge 1 }
Instances | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
Start-Sleep -Milliseconds 800

Write-Host "== uninstall the way Windows does it (Apps & Features)"
$quiet = (Get-ItemProperty $unKey).QuietUninstallString
Write-Host "    $quiet"
$p = Start-Process -FilePath 'cmd.exe' -ArgumentList '/c', $quiet -Wait -PassThru -NoNewWindow
Check "[U1] uninstaller exit code 0" { $p.ExitCode -eq 0 }
$sw = [Diagnostics.Stopwatch]::StartNew()
while ((Test-Path $prog) -and $sw.ElapsedMilliseconds -lt 20000) { Start-Sleep -Milliseconds 300 }
Check "[U2] program folder removed (by the temp copy)" { -not (Test-Path $prog) }
$after = Snap 'after'
$diff = Compare-Object $before $after
if ($diff) { $diff | ForEach-Object { Write-Host "    $($_.SideIndicator) $($_.InputObject)" } }
Check "[U3] registry + Start menu exactly as before (incl. the integration switched on before install)" { -not $diff }
Check "[U4] journal gone" { -not (Test-Path "$env:EXPLORER2_CONFIG_DIR\integration-journal.txt") }
Start-Sleep -Seconds 6
Check "[U5] the temp copy of the uninstaller removed itself" { @(Get-ChildItem $env:TEMP -Filter 'explorer2-uninstall-*' -ErrorAction SilentlyContinue).Count -eq 0 }
$r = RunGui (Join-Path $dl 'explorer2-recover.exe') @('--status')
Check "[U6] nothing active any more" { $r.code -eq 0 }
exit $script:fail
