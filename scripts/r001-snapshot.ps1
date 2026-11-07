# R-001 mandatory test (Windows): snapshot -> integration on -> reset -> snapshot identical.
# The snapshot is taken with Windows' own reg.exe, independent of our code.
#   pwsh scripts/r001-snapshot.ps1 <release dir with explorer2-recover.exe> <work dir>
param([Parameter(Mandatory)] [string]$Bin, [Parameter(Mandatory)] [string]$Work)
$ErrorActionPreference = 'Continue'
$script:fail = 0
function Pass($m) { Write-Host "PASS  $m" }
function Fail($m) { Write-Host "FAIL  $m"; $script:fail = 1 }
function Check($m, [scriptblock]$cond) { if (& $cond) { Pass $m } else { Fail $m } }

$recover = Join-Path $Bin 'explorer2-recover.exe'
$self = Join-Path $Bin 'examples\r001_selftest.exe'
New-Item -ItemType Directory -Force $Work | Out-Null
$cfg = Join-Path $Work 'cfg'
$env:EXPLORER2_CONFIG_DIR = $cfg
$journal = Join-Path $cfg 'integration-journal.txt'
$mirror = Join-Path $cfg 'integration-journal.mirror.txt'
$file = Join-Path $Work 'startmenu\Explorer 2.0 - Wiederherstellen.lnk'
New-Item -ItemType Directory -Force (Split-Path $file) | Out-Null
# the registry areas our integrations touch (+ the foreign probe value we overwrite)
$keys = @(
  'HKCU\Software\Classes\Directory', 'HKCU\Software\Classes\Folder', 'HKCU\Software\Classes\Drive',
  'HKCU\Software\Classes\CLSID\{52205FD8-5DFB-447D-801A-D0B52F2E83E1}',
  'HKCU\Environment', 'HKCU\Software\Explorer2', 'HKCU\Software\Explorer2R001Probe')

function Snap([string]$name) {
  $lines = foreach ($k in $keys) {
    $f = Join-Path $Work ("$name-" + ($k -replace '[\\:{}]', '_') + '.reg')
    & reg.exe export $k $f /y 2>$null | Out-Null
    if ($LASTEXITCODE -eq 0) { "$k = " + (Get-FileHash $f).Hash } else { "$k = (absent)" }
  }
  $lines += 'file = ' + $(if (Test-Path $file) { (Get-FileHash $file).Hash } else { '(absent)' })
  return $lines
}
function Same([string]$what, $before, [string]$name) {
  $after = Snap $name
  $diff = Compare-Object $before $after
  if ($diff) {
    Fail "$what - snapshot differs:"
    $diff | ForEach-Object { Write-Host "    $($_.SideIndicator) $($_.InputObject)" }
    # show the content of the differing exports
    foreach ($k in $keys) {
      $a = Join-Path $Work ("before-" + ($k -replace '[\\:{}]', '_') + '.reg')
      $b = Join-Path $Work ("$name-" + ($k -replace '[\\:{}]', '_') + '.reg')
      if ((Test-Path $b) -and -not ((Test-Path $a) -and ((Get-FileHash $a).Hash -eq (Get-FileHash $b).Hash))) {
        Write-Host "    --- $k after:"; Get-Content $b | Select-Object -First 40 | ForEach-Object { Write-Host "      $_" }
      }
    }
  } else { Pass "$what - snapshot identical ($($before.Count) areas)" }
}
function Run([string]$exe, [string[]]$argv) {
  $o = & $exe @argv 2>&1 | Out-String
  Write-Host ("    > " + (Split-Path $exe -Leaf) + " " + ($argv -join ' ') + " (exit $LASTEXITCODE)")
  $o.Trim() -split "`n" | ForEach-Object { Write-Host "      $_" }
  return @{ code = $LASTEXITCODE; text = $o }
}
function Verb { (Get-ItemProperty 'HKCU:\Software\Classes\Directory\shell\open\command' -ErrorAction SilentlyContinue).'(default)' }

# a foreign value that the plan overwrites (must come back exactly)
& reg.exe add 'HKCU\Software\Explorer2R001Probe' /v keep /t REG_SZ /d 'original value' /f | Out-Null
$before = Snap 'before'
Write-Host "snapshot before:"; $before | ForEach-Object { Write-Host "    $_" }

Write-Host "== 1) integration on -> reset from the journal"
$r = Run $self @('full', $file)
Check "[R1] selftest plan applied" { $r.code -eq 0 }
Check "[R1b] folder verb points to explorer2" { (Verb) -match 'explorer2' }
Check "[R1c] user environment variable set" { (Get-ItemProperty 'HKCU:\Environment').EXPLORER2_R001 -ne $null }
Check "[R1d] foreign value overwritten, file written" { (Get-ItemProperty 'HKCU:\Software\Explorer2R001Probe').keep -match 'explorer2' -and (Test-Path $file) }
Check "[R1e] journal + mirror written" { (Test-Path $journal) -and (Test-Path $mirror) }
$r = Run $recover @('--status')
Check "[R1f] --status reports active integration (exit 1)" { $r.code -eq 1 -and $r.text -match 'Journal' }
$r = Run $recover @('--yes')
Check "[R1g] reset ok (exit 0)" { $r.code -eq 0 -and $r.text -match 'wiederhergestellt' }
Same "[R1h] reset from journal" $before 'after1'
Check "[R1i] journal removed" { -not (Test-Path $journal) -and -not (Test-Path $mirror) }

Write-Host "== 2) primary journal damaged -> mirror"
$r = Run $self @('full', $file)
$t = Get-Content $journal -Raw; Set-Content $journal $t.Substring(0, [int]($t.Length / 2)) -NoNewline
$r = Run $recover @('--yes')
Check "[R2] reset ok with a damaged primary" { $r.code -eq 0 }
Same "[R2b] reset via the mirror" $before 'after2'

Write-Host "== 3) no journal at all -> sweep of the known points"
$r = Run $self @('manifest')
Remove-Item $journal, $mirror -Force
$r = Run $recover @('--status')
Check "[R3] --status still sees our integration without a journal" { $r.code -eq 1 -and $r.text -match 'Integrationsstelle' }
$r = Run $recover @('--yes')
Check "[R3b] reset ok" { $r.code -eq 0 }
Same "[R3c] reset without journal" $before 'after3'

Write-Host "== 3b) the M7 features (folder verbs, context menu) - with and without journal"
$r = Run $self @('features')
Check "[R5] features switched on" { $r.code -eq 0 -and (Get-ItemProperty 'HKCU:\Software\Classes\Directory\shell').'(default)' -eq 'explorer2open' }
$r = Run $recover @('--yes')
Check "[R5b] reset ok" { $r.code -eq 0 }
Same "[R5c] features reset from the journal" $before 'after5'
$r = Run $self @('features')
Remove-Item $journal, $mirror -Force
$r = Run $recover @('--yes')
Check "[R5d] reset ok without journal" { $r.code -eq 0 }
Same "[R5e] features reset without journal (manifest sweep)" $before 'after5b'

Write-Host "== 4) nothing to do"
$r = Run $recover @('--yes')
Check "[R4] second reset: nothing to do (exit 0)" { $r.code -eq 0 -and $r.text -match 'nichts zu tun' }
$r = Run $recover @('--status')
Check "[R4b] --status: nothing active (exit 0)" { $r.code -eq 0 }
$r = Run $recover @('--bogus')
Check "[R4c] unknown argument = usage (exit 2)" { $r.code -eq 2 }

& reg.exe delete 'HKCU\Software\Explorer2R001Probe' /f | Out-Null
exit $script:fail
