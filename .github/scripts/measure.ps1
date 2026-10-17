# Phase 002 measurements and soft-budget gate on windows-latest (§4.9, Appendix D §D.3, P-05/P-09).
# Results land in ./m (uploaded as the "measurements" artifact). Any exceeded soft budget fails CI.
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path m | Out-Null
$m = (Resolve-Path m).Path
$failed = @()
$rel = 'target/release/yamdv.exe'
$dev = 'target/dev/release/yamdv.exe'
$typ = (Resolve-Path 'tests/corpus/002/typical.md').Path
& cargo xtask gen-large "$m/large-1mb.md"
$large = "$m/large-1mb.md"
$size = @('--size', '1000x700')

function Probe([string]$label, [string]$exe, [string[]]$extra) {
  Write-Host "== memprobe $label"
  & cargo xtask memprobe $exe --json "$m/$label.json" --label $label @size @extra
  if ($LASTEXITCODE -ne 0) { $script:failed += $label; Write-Host "::error::$label failed or over budget" }
}

Get-CimInstance Win32_OperatingSystem | Select-Object Caption, Version, BuildNumber | Format-List | Out-File "$m/host.txt"
Get-CimInstance Win32_VideoController | Select-Object Name, CurrentHorizontalResolution, CurrentVerticalResolution | Format-List | Out-File -Append "$m/host.txt"
Get-ChildItem target/release/*.exe | Select-Object FullName, Length | Format-Table -AutoSize | Out-File "$m/sizes.txt"

# M0 + T2 + M7, M1 (typical.md, Phase 003) + T1 + M7, M2 + T1L + M7
Probe 'M0' $rel @('--runs', '5', '--budget-pws-mib', '4.8', '--budget-commit-ratio', '3', '--budget-first-paint-ms', '100', '--screenshot', "$m/M0.png")
Probe 'M1' $rel @('--doc', $typ, '--runs', '5', '--budget-pws-mib', '8.0', '--budget-commit-ratio', '3', '--budget-first-paint-ms', '150', '--screenshot', "$m/M1.png")
Probe 'M2' $rel @('--doc', $large, '--runs', '5', '--budget-pws-mib', '16', '--budget-commit-ratio', '3', '--budget-first-paint-ms', '300', '--screenshot', "$m/M2.png")
# M6 (P-10, F-09): Ctrl+O shows the dialog in the out-of-process host; cancel, wait 2 s, no trim.
# Budget: PWS <= M1 + 1 MiB and commit <= M1 commit + 1.5 MiB; module count unchanged; T-DLG reported.
$m1j = (Get-Content "$m/M1.json" -Raw | ConvertFrom-Json).median
Probe 'M6' $rel @('--doc', $typ, '--runs', '3', '--open-dialog', '--settle-ms', '2000', '--budget-pws-mib', "$($m1j.private_ws_mib + 1)", '--budget-commit-mib', "$($m1j.private_usage_mib + 1.5)")
$m6 = (Get-Content "$m/M6.json" -Raw | ConvertFrom-Json).median
Write-Host "T-DLG = $($m6.t_dlg_ms) ms (soft 400); modules before/after = $($m6.modules_before_dialog)/$($m6.module_count)"
if ($m6.t_dlg_ms -gt 400) { Write-Host "::warning::T-DLG $($m6.t_dlg_ms) ms > 400 ms (soft, report only)" }
# T7: idle CPU 30 s on the empty state and on a document
Probe 'T7-empty' $rel @('--runs', '1', '--cpu-ms', '30000', '--budget-cpu-ms', '1')
Probe 'T7-doc' $rel @('--doc', $typ, '--runs', '1', '--cpu-ms', '30000', '--budget-cpu-ms', '1')
# T4 (F-10): autoscroll top -> bottom -> top (dev-tools build). No external warm-up pass any more:
# the app warms its fonts in idle time after the first paint. Per document, run 1 is "cold"
# (reported against <= 33 ms), runs 2-4 are "warm" (gate: median p99 <= 16 ms).
function T4Run([string]$n, [string]$f, [string]$tag) {
  $log = "$m/T4-$n-$tag.log"
  Probe "T4-$n-$tag" $dev @('--doc', $f, '--runs', '1', '--wait-title', 'SCROLL_DONE', '--settle-ms', '500', '--arg', '--autoscroll', '--arg', '--perf-log', '--arg', $log)
  if (-not (Test-Path $log)) { $script:failed += "T4-$n-$tag-log"; return $null }
  $p99 = [double](((Get-Content $log) | Where-Object { $_ -like 'p99_ms *' }) -split ' ')[1]
  $frames = [int](((Get-Content $log) | Where-Object { $_ -like 'frames *' }) -split ' ')[1]
  Write-Host "T4-$n-$tag frames = $frames, p99 = $p99 ms"
  if ($frames -lt 10) { $script:failed += "T4-$n-$tag-frames" }
  return $p99
}
foreach ($d in @(@{n='typical'; f=$typ}, @{n='large'; f=$large})) {
  $cold = T4Run $d.n $d.f 'cold'
  $p99s = @()
  foreach ($r in 1..3) { $v = T4Run $d.n $d.f "warm$r"; if ($null -ne $v) { $p99s += $v } }
  $med = ($p99s | Sort-Object)[[math]::Floor(($p99s.Count - 1) / 2)]
  Write-Host "T4-$($d.n) cold p99 = $cold ms; warm median p99 = $med ms (runs: $($p99s -join ', '))"
  "T4-$($d.n) cold_p99_ms $cold warm_median_p99_ms $med runs $($p99s -join ' ')" | Out-File -Append "$m/T4-summary.txt"
  if ($cold -gt 33) { Write-Host "::warning::T4 cold p99 $cold ms > 33 ms target (report only)" }
  if ($med -gt 16) { $failed += "T4-$($d.n)"; Write-Host "::error::T4 warm median p99 $med ms > 16 ms" }
}
# Subsystem breakdown (mem-stats build), informational
foreach ($d in @(@{n='M1'; f=$typ}, @{n='M2'; f=$large})) {
  Probe "memstats-$($d.n)" $dev @('--doc', $d.f, '--runs', '1', '--arg', '--mem-stats', '--arg', "$m/memstats-$($d.n).app.json")
}
# T3 (prompt 005 §4): second launch -> file visible in the existing instance (target <= 50 ms,
# max <= 80 ms; reported, warning only). Clock: Start-Process of the 2nd exe until a window whose
# title names the forwarded file exists (polled every 2 ms).
Add-Type -TypeDefinition @"
using System; using System.Text; using System.Runtime.InteropServices;
public static class T3W {
  delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowTextW(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  public static bool Has(string needle) {
    bool found = false;
    EnumWindows((h, l) => { var sb = new StringBuilder(512); GetWindowTextW(h, sb, 512);
      if (IsWindowVisible(h) && sb.ToString().Contains(needle)) { found = true; return false; } return true; }, IntPtr.Zero);
    return found;
  }
}
"@
$second = (Resolve-Path 'tests/driver/b.md').Path
$t3s = @()
$origApp = $env:APPDATA; $origLocal = $env:LOCALAPPDATA
foreach ($r in 1..5) {
  $prof = (New-Item -ItemType Directory -Force -Path "$m/t3-profile").FullName
  $env:APPDATA = $prof; $env:LOCALAPPDATA = $prof
  $p1 = Start-Process $rel -ArgumentList $typ -PassThru
  $null = $p1.WaitForInputIdle(10000); Start-Sleep -Milliseconds 1500
  $sw = [Diagnostics.Stopwatch]::StartNew()
  $p2 = Start-Process $rel -ArgumentList $second -PassThru
  while (-not [T3W]::Has('b.md') -and $sw.ElapsedMilliseconds -lt 5000) { Start-Sleep -Milliseconds 2 }
  $t = $sw.Elapsed.TotalMilliseconds
  $null = $p2.WaitForExit(5000)
  $forwarded = $p2.HasExited
  Stop-Process -Id $p1.Id -Force -ErrorAction SilentlyContinue; $null = $p1.WaitForExit(10000)
  $env:APPDATA = $origApp; $env:LOCALAPPDATA = $origLocal
  if ($t -ge 5000 -or -not $forwarded) { $failed += "T3-run$r"; Write-Host "::error::T3 run $($r): no forwarded window" ; continue }
  $t3s += [math]::Round($t, 1)
}
if ($t3s.Count -gt 0) {
  $t3 = ($t3s | Sort-Object)[[math]::Floor(($t3s.Count - 1) / 2)]
  Write-Host "T3 median = $t3 ms (runs: $($t3s -join ', '))"
  "T3 median_ms $t3 runs $($t3s -join ' ')" | Out-File "$m/T3.txt"
  if ($t3 -gt 80) { Write-Host "::warning::T3 $t3 ms > 80 ms (includes process start of the 2nd launch; report only)" }
}
# 150 % scale screenshot is covered by the goldens (scale 1.5).
Get-ChildItem $m | Format-Table Name, Length -AutoSize
if ($failed.Count -gt 0) { Write-Host "::error::gates failed: $($failed -join ', ')"; exit 1 }
Write-Host 'all measurement gates green'
