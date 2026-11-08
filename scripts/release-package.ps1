# Release packaging (Windows): builds Explorer2-Setup.exe around the release
# build of the app, smoke-tests it (silent install into a temp folder ->
# files + version + Apps & Features entry -> silent uninstall) and writes the
# download files plus SHA256SUMS.txt into <out dir>.
#   pwsh scripts/release-package.ps1 <release dir> <out dir> <work dir>
# <release dir> must hold explorer2.exe + explorer2-recover.exe (cargo build --release).
param([Parameter(Mandatory)] [string]$Bin, [Parameter(Mandatory)] [string]$Out,
      [Parameter(Mandatory)] [string]$Work)
$ErrorActionPreference = 'Continue'
$script:fail = 0
function Pass($m) { Write-Host "PASS  $m" }
function Fail($m) { Write-Host "FAIL  $m"; $script:fail = 1 }
function Check($m, [scriptblock]$cond) { if (& $cond) { Pass $m } else { Fail $m } }

$Bin = (Resolve-Path $Bin).Path
New-Item -ItemType Directory -Force $Out, $Work | Out-Null
$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
Write-Host "version $version"

# the payload = exactly the files that get released as the portable ZIP
$payload = Join-Path $Work 'payload'
New-Item -ItemType Directory -Force $payload | Out-Null
Copy-Item (Join-Path $Bin 'explorer2.exe'), (Join-Path $Bin 'explorer2-recover.exe') $payload
$env:EXPLORER2_SETUP_PAYLOAD = $payload
cargo build --release -p explorer2-setup
if ($LASTEXITCODE) { throw "setup build failed" }
Remove-Item Env:EXPLORER2_SETUP_PAYLOAD

$setup = Join-Path $Out "Explorer2-Setup-$version-x64.exe"
Copy-Item (Join-Path $Bin 'Explorer2-Setup.exe') $setup -Force
Write-Host ("setup: {0:N1} MB" -f ((Get-Item $setup).Length / 1MB))

# --- smoke test: silent install into a temp folder, then uninstall --------
$env:EXPLORER2_CONFIG_DIR = Join-Path $Work 'cfg'
$env:EXPLORER2_INSTALL_DIR = Join-Path $Work 'Programs\Explorer2'
$env:EXPLORER2_STARTMENU_DIR = Join-Path $Work 'StartMenu\Explorer 2.0'
$prog = $env:EXPLORER2_INSTALL_DIR
$unKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Explorer2'
function RunGui([string]$exe, [string[]]$argv) {
  $o = Join-Path $Work ("out-" + [guid]::NewGuid().ToString('N') + '.txt')
  $p = Start-Process -FilePath $exe -ArgumentList $argv -Wait -PassThru -NoNewWindow -RedirectStandardOutput $o
  $text = Get-Content $o -Raw
  Write-Host "    > $(Split-Path $exe -Leaf) $($argv -join ' ') (exit $($p.ExitCode)): $text"
  return @{ code = $p.ExitCode; text = $text }
}

$r = RunGui $setup @('--quiet')
Check "[S1] setup --quiet installs (exit 0)" { $r.code -eq 0 -and $r.text -match 'installiert' }
Check "[S2] both programs in the install folder, identical to the build" {
  foreach ($n in 'explorer2.exe', 'explorer2-recover.exe') {
    $a = Join-Path $prog $n
    if (-not (Test-Path $a)) { return $false }
    if ((Get-FileHash $a).Hash -ne (Get-FileHash (Join-Path $payload $n)).Hash) { return $false }
  }
  $true
}
$v = RunGui (Join-Path $prog 'explorer2.exe') @('--version')
Check "[S3] installed app reports version $version" { $v.text -match [regex]::Escape("explorer2 $version") }
$un = Get-ItemProperty $unKey -ErrorAction SilentlyContinue
Check "[S4] 'Apps & Features' entry $($un.DisplayName) $($un.DisplayVersion)" { $un.DisplayName -eq 'Explorer 2.0' -and $un.DisplayVersion -eq $version }
Check "[S5] Start menu shortcuts" { @(Get-ChildItem $env:EXPLORER2_STARTMENU_DIR -Filter *.lnk -ErrorAction SilentlyContinue).Count -ge 2 }
Check "[S6] temp unpack folder removed" { @(Get-ChildItem $env:TEMP -Directory -Filter 'Explorer2-Setup-*' -ErrorAction SilentlyContinue).Count -eq 0 }
$r = RunGui $setup @('--quiet')
Check "[S7] running the setup again updates in place (exit 0)" { $r.code -eq 0 }

$u = RunGui (Join-Path $prog 'explorer2-recover.exe') @('--uninstall', '--quiet')
Check "[S8] uninstall (exit 0)" { $u.code -eq 0 }
for ($i = 0; $i -lt 40 -and (Test-Path $prog); $i++) { Start-Sleep -Milliseconds 250 }
Check "[S9] program folder, Start menu and Apps & Features entry gone" {
  -not (Test-Path $prog) -and -not (Test-Path $env:EXPLORER2_STARTMENU_DIR) -and -not (Test-Path $unKey)
}
Remove-Item Env:EXPLORER2_CONFIG_DIR, Env:EXPLORER2_INSTALL_DIR, Env:EXPLORER2_STARTMENU_DIR

# --- portable ZIP + checksums ---------------------------------------------
$zip = Join-Path $Out "Explorer2-$version-x64-portable.zip"
Remove-Item $zip -ErrorAction SilentlyContinue
Compress-Archive -Path (Join-Path $payload '*') -DestinationPath $zip
$sums = Get-ChildItem $Out -File | Where-Object Name -ne 'SHA256SUMS.txt' | Sort-Object Name |
  ForEach-Object { "{0}  {1}" -f (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower(), $_.Name }
$sums | Set-Content (Join-Path $Out 'SHA256SUMS.txt') -Encoding ascii
Get-ChildItem $Out | ForEach-Object { Write-Host ("  {0,-45} {1,8:N1} MB" -f $_.Name, ($_.Length / 1MB)) }

if ($script:fail) { throw "release packaging: smoke test failed" }
Write-Host "release packaging ok"
