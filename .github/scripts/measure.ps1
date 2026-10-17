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
# F-20 (prompt 006): M1 opens typical.md in place (memprobe sets the working directory to the
# corpus folder) at its image section (`--line 822`), so both 800x600 PNGs are decoded and held.
$m1line = @('--arg', '--line', '--arg', '822')
Probe 'M1' $rel (@('--doc', $typ, '--runs', '5') + $m1line + @('--budget-pws-mib', '8.0', '--budget-commit-ratio', '3', '--budget-first-paint-ms', '150', '--screenshot', "$m/M1.png"))
Probe 'M2' $rel @('--doc', $large, '--runs', '5', '--budget-pws-mib', '16', '--budget-commit-ratio', '3', '--budget-first-paint-ms', '300', '--screenshot', "$m/M2.png")
# V1-C (prompt 007 §5): M3 = PWS of math-heavy.md / mermaid-heavy.md (<= 9.6 MiB each);
# T1M = their first paint (reported).
$mh = (Resolve-Path 'tests/corpus/002/math-heavy.md').Path
$mm = (Resolve-Path 'tests/corpus/002/mermaid-heavy.md').Path
foreach ($d in @(@{n='M3-math'; f=$mh}, @{n='M3-mermaid'; f=$mm})) {
  Probe $d.n $rel @('--doc', $d.f, '--runs', '5', '--budget-pws-mib', '9.6', '--screenshot', "$m/$($d.n).png")
  $j = (Get-Content "$m/$($d.n).json" -Raw | ConvertFrom-Json).median
  Write-Host "T1M $($d.n): first paint $($j.first_paint_ms) ms, PWS $($j.private_ws_mib) MiB"
  "T1M $($d.n) first_paint_ms $($j.first_paint_ms) pws_mib $($j.private_ws_mib)" | Out-File -Append "$m/T1M.txt"
}
# Prompt 007 §4: one block's display list offscreen to PNG at 2x (the V1-D export path).
$bp = "$m/block-mermaid-2x.png"
& $dev --render-png $bp --block 2 --width 1000 --scale 2 (Resolve-Path 'tests/corpus/002/mermaid.md').Path
if ($LASTEXITCODE -ne 0 -or -not (Test-Path $bp) -or (Get-Item $bp).Length -lt 1000) { $failed += 'block-png'; Write-Host "::error::block PNG snapshot failed" } else { Write-Host "block PNG: $((Get-Item $bp).Length) bytes" }
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
  Probe "T4-$n-$tag" $dev @('--doc', $f, '--runs', '1', '--wait-title', 'SCROLL_DONE', '--settle-ms', '500', '--arg', '--autoscroll', '--arg', '--perf-log', '--arg', $log) | Out-Host
  if (-not (Test-Path $log)) { $script:failed += "T4-$n-$tag-log"; return $null }
  $p99 = [double](((Get-Content $log) | Where-Object { $_ -like 'p99_ms *' }) -split ' ')[1]
  $frames = [int](((Get-Content $log) | Where-Object { $_ -like 'frames *' }) -split ' ')[1]
  Write-Host "T4-$n-$tag frames = $frames, p99 = $p99 ms"
  if ($frames -lt 10) { $script:failed += "T4-$n-$tag-frames" }
  return [double]$p99
}
# F-21 (prompt 006): one attempt = median p99 of 3 warm runs. If attempt 1 exceeds 16 ms, one
# automatic retry runs in the same job; T4 fails only when both consecutive attempts exceed 16 ms.
function T4Attempt([string]$n, [string]$f, [int]$a) {
  [double[]]$p99s = @()
  foreach ($r in 1..3) { $v = T4Run $n $f "a$a-warm$r"; if ($null -ne $v) { $p99s += [double]$v } }
  if ($p99s.Count -eq 0) { return [double]9999 }
  return [double]($p99s | Sort-Object)[[math]::Floor(($p99s.Count - 1) / 2)]
}
foreach ($d in @(@{n='typical'; f=$typ}, @{n='large'; f=$large}, @{n='math'; f=$mh}, @{n='mermaid'; f=$mm})) {
  $cold = T4Run $d.n $d.f 'cold'
  # T4Run's only pipeline output is the p99 (Probe output goes to the host), so the median and
  # the 16 ms gate compare numbers (in V1-A run 2 the Probe text leaked in and the gate compared strings).
  $med1 = T4Attempt $d.n $d.f 1
  $med2 = 'not run'
  if ($med1 -gt 16) { Write-Host "::warning::T4-$($d.n) attempt 1 warm median p99 $med1 ms > 16 ms; retrying"; $med2 = T4Attempt $d.n $d.f 2 }
  Write-Host "T4-$($d.n) cold p99 = $cold ms; warm median p99 attempt 1 = $med1 ms, attempt 2 = $med2"
  "T4-$($d.n) cold_p99_ms $cold warm_median_p99_ms attempt1 $med1 attempt2 $med2" | Out-File -Append "$m/T4-summary.txt"
  if ($cold -gt 33) { Write-Host "::warning::T4 cold p99 $cold ms > 33 ms target (report only)" }
  if ($med1 -gt 16 -and $med2 -is [double] -and $med2 -gt 16) { $failed += "T4-$($d.n)"; Write-Host "::error::T4 warm median p99 $med1 / $med2 ms > 16 ms on two consecutive attempts" }
}
# Subsystem breakdown (mem-stats build), informational
foreach ($d in @(@{n='M1'; f=$typ; x=$m1line}, @{n='M2'; f=$large; x=@()})) {
  Probe "memstats-$($d.n)" $dev (@('--doc', $d.f, '--runs', '1', '--arg', '--mem-stats', '--arg', "$m/memstats-$($d.n).app.json") + $d.x)
}
# F-20: the image cache bytes of the M1 scenario (decoded + LRU), from the mem-stats dump.
$ms1 = if (Test-Path "$m/memstats-M1.app.json") { Get-Content "$m/memstats-M1.app.json" -Raw | ConvertFrom-Json } else { [pscustomobject]@{ image_decoded_bytes = 0; image_lru_bytes = 0 } }
Write-Host "M1 image cache: decoded $($ms1.image_decoded_bytes) B, LRU $($ms1.image_lru_bytes) B"
"M1 image_decoded_bytes $($ms1.image_decoded_bytes) image_lru_bytes $($ms1.image_lru_bytes)" | Out-File "$m/M1-images.txt"
if ($ms1.image_decoded_bytes -le 0) { Write-Host "::warning::M1 decoded no images (F-20)" }
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
# V1-B (prompt 006 §2.7): M5 = memory after 200 edits (F3 and F2, typical.md, PWS <= 9.6 MiB) and
# T5 = key -> caret painted p99 (F3 on large-1mb.md <= 25 ms, F2 on typical.md <= 40 ms). Both run
# the dev-tools build (the driver types one character per step) on copies, never on the corpus.
$ed = (New-Item -ItemType Directory -Force -Path "$m/edit").FullName
Copy-Item -Recurse -Force tests/corpus/002/* $ed
Copy-Item -Force $large "$ed/t5-large.md"
$perf = (Resolve-Path tests/perf).Path
foreach ($k in 'split', 'live') {
  Copy-Item -Force "$ed/typical.md" "$ed/m5-$k.md"
  Probe "M5-$k" $dev @('--doc', "$ed/m5-$k.md", '--runs', '1', '--settle-ms', '16000', '--arg', '--script', '--arg', "$perf/m5-$k.yds", '--budget-pws-mib', '9.6')
}
function T5([string]$name, [string]$doc) {
  $saveApp = $env:APPDATA; $saveLocal = $env:LOCALAPPDATA
  $prof = (New-Item -ItemType Directory -Force -Path "$m/prof-$name").FullName
  $env:APPDATA = $prof; $env:LOCALAPPDATA = $prof
  $s = "$perf/$name.yds"
  $p = Start-Process $dev -ArgumentList '--script', $s, $doc -PassThru
  $null = $p.Handle
  $done = $p.WaitForExit(180000)
  $env:APPDATA = $saveApp; $env:LOCALAPPDATA = $saveLocal
  if (-not $done) { $p.Kill(); $script:failed += $name; Write-Host "::error::$name timed out"; return }
  $log = if (Test-Path "$s.log") { Get-Content "$s.log" -Raw } else { '' }
  if (Test-Path "$s.log") { Copy-Item "$s.log" "$m/$name.log" }
  $p99 = if ($log -match '"t5_p99_ms":([0-9.]+)') { $Matches[1] } else { 'n/a' }
  $n = if ($log -match '"t5_n":([0-9.]+)') { $Matches[1] } else { 'n/a' }
  Write-Host "$name p99 = $p99 ms ($n samples)"
  "$name t5_p99_ms $p99 samples $n" | Out-File -Append "$m/T5.txt"
  if ($p.ExitCode -ne 0) { $script:failed += $name; Write-Host "::error::$name over budget or failed (exit $($p.ExitCode))" }
}
T5 't5-split' "$ed/t5-large.md"
T5 't5-live' "$ed/typical.md"
# 150 % scale screenshot is covered by the goldens (scale 1.5).
Get-ChildItem $m | Format-Table Name, Length -AutoSize
if ($failed.Count -gt 0) { Write-Host "::error::gates failed: $($failed -join ', ')"; exit 1 }
Write-Host 'all measurement gates green'
