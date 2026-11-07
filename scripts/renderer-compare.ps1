# Skia vs FemtoVG on Windows: start time, robustness (no GL / Mesa GL) and a
# screenshot of the same view per renderer. Screenshots are printed as
# base64 lines ("PNG-BASE64 <label> <data>") so they can be read from the log.
# Usage: pwsh -File scripts/renderer-compare.ps1 <exe with both renderers> <mesa dir with opengl32.dll> <workdir>
param([string]$Exe, [string]$MesaDir, [string]$Work)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class R {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, Rr, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, UIntPtr e);
}
"@
New-Item -ItemType Directory -Force $Work | Out-Null
$data = New-Item -ItemType Directory -Force "$Work\data"
foreach ($d in 'Bilder', 'Projekte 2026', 'Überweisungen') { New-Item -ItemType Directory -Force "$data\$d" | Out-Null }
foreach ($f in 'Bericht Q3 – final.docx', 'Ärger mit Umlauten äöü ß.txt', 'foto10.png', 'foto2.png', 'Makefile',
               'sehr langer Dateiname der abgeschnitten werden sollte weil er nicht in die Spalte passt.pdf') {
  Set-Content -LiteralPath "$data\$f" 'x'
}
# app copy next to Mesa's opengl32.dll = machine with a (software) OpenGL 4.x driver
$mesaExe = "$Work\mesa\explorer2.exe"
New-Item -ItemType Directory -Force "$Work\mesa" | Out-Null
Copy-Item "$MesaDir\*.dll" "$Work\mesa"; Copy-Item $Exe $mesaExe

function Run([string]$label, [string]$app, [hashtable]$vars, [bool]$shot) {
  $log = "$Work\$label.log"
  $vars['EXPLORER2_TRACE'] = '1'; $vars['EXPLORER2_CONFIG_DIR'] = "$Work\cfg-$label"
  foreach ($k in $vars.Keys) { [Environment]::SetEnvironmentVariable($k, $vars[$k]) }
  $p = Start-Process -FilePath $app -ArgumentList "`"$data`"" -PassThru -RedirectStandardError $log
  Start-Sleep -Seconds 5
  $alive = -not $p.HasExited
  if ($alive -and $shot) {
    $p.Refresh(); $h = $p.MainWindowHandle
    [R]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero); [R]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
    [void][R]::SetForegroundWindow($h); Start-Sleep -Milliseconds 700
    $r = New-Object R+RECT; [void][R]::GetWindowRect($h, [ref]$r)
    # list area of the window: below the toolbar, right of the sidebar
    $w = 760; $hh = 260; $x = $r.L + 218; $y = $r.T + 110
    $bmp = New-Object System.Drawing.Bitmap $w, $hh
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($x, $y, 0, 0, (New-Object System.Drawing.Size $w, $hh))
    $file = "$Work\$label.png"; $bmp.Save($file, [System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose()
    Write-Host "PNG-BASE64 $label $([Convert]::ToBase64String([IO.File]::ReadAllBytes($file)))"
  }
  if ($alive) { Stop-Process -Id $p.Id -Force }
  foreach ($k in $vars.Keys) { [Environment]::SetEnvironmentVariable($k, $null) }
  Start-Sleep -Milliseconds 300
  $text = Get-Content $log -Raw
  $m = [regex]::Match($text, 'first paint tab=1 .*? at=([\d.]+)ms')
  $cold = if ($m.Success) { [double]::Parse($m.Groups[1].Value, [cultureinfo]::InvariantCulture) } else { -1 }
  $gl = if ($text -match 'Failed to initialize') { 'GL-init-failed(fallback)' } else { 'ok' }
  $rend = [regex]::Match($text, 'gpu_renderer=(\w+)').Groups[1].Value
  $panic = $text -match '(?i)panic'
  "RESULT label=$label alive=$alive panic=$panic cold_ms=$cold gl=$gl gpu_renderer=$rend"
  if (-not $alive -or $panic) { Write-Host "----- [$label] trace (app died / panicked)"; Write-Host $text }
}

foreach ($rd in 'skia', 'femtovg') {
  # real GL path (Mesa llvmpipe, OpenGL 4.x): picture + start time (3 starts, best)
  $colds = @()
  foreach ($n in 1..3) {
    $res = Run "$rd-mesa-$n" $mesaExe @{ EXPLORER2_RENDERER = 'gpu'; EXPLORER2_GPU_RENDERER = $rd } ($n -eq 1)
    Write-Host $res
    $colds += [double]([regex]::Match($res, 'cold_ms=([\d.-]+)').Groups[1].Value)
  }
  Write-Host "SUMMARY $rd mesa cold start best=$(($colds | Measure-Object -Minimum).Minimum) ms all=$($colds -join ', ')"
  # robustness: forced GPU on a machine without usable OpenGL (CI runner) must still work
  Write-Host (Run "$rd-nogl" $Exe @{ EXPLORER2_RENDERER = 'gpu'; EXPLORER2_GPU_RENDERER = $rd } $true)
}
# reference: software renderer
Write-Host (Run "software" $Exe @{ EXPLORER2_RENDERER = 'software' } $true)
