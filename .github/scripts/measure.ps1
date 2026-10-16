# Phase 002 measurements and soft-budget gate on windows-latest (§4.9, Appendix D §D.3, P-05/P-09).
# Results land in ./m (uploaded as the "measurements" artifact). Any exceeded soft budget fails CI.
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path m | Out-Null
$m = (Resolve-Path m).Path
$failed = @()
$rel = 'target/release/yamdv.exe'
$dev = 'target/dev/release/yamdv.exe'
$typ = (Resolve-Path 'tests/corpus/002/typical-basic.md').Path
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

# M0 + T2 + M7, M1b + T1b + M7, M2 + T1L + M7
Probe 'M0' $rel @('--runs', '5', '--budget-pws-mib', '4.8', '--budget-commit-ratio', '3', '--budget-first-paint-ms', '100', '--screenshot', "$m/M0.png")
Probe 'M1b' $rel @('--doc', $typ, '--runs', '5', '--budget-pws-mib', '8.0', '--budget-commit-ratio', '3', '--budget-first-paint-ms', '150', '--screenshot', "$m/M1b.png")
Probe 'M2' $rel @('--doc', $large, '--runs', '5', '--budget-pws-mib', '16', '--budget-commit-ratio', '3', '--budget-first-paint-ms', '300', '--screenshot', "$m/M2.png")
# M6: M1b, open + cancel Ctrl+O, wait 2 s; budget M1b + 1 MiB
$m1b = (Get-Content "$m/M1b.json" -Raw | ConvertFrom-Json).median.private_ws_mib
Probe 'M6' $rel @('--doc', $typ, '--runs', '3', '--open-dialog', '--settle-ms', '2000', '--budget-pws-mib', "$($m1b + 1)")
# T7: idle CPU 30 s on the empty state and on a document
Probe 'T7-empty' $rel @('--runs', '1', '--cpu-ms', '30000', '--budget-cpu-ms', '1')
Probe 'T7-doc' $rel @('--doc', $typ, '--runs', '1', '--cpu-ms', '30000', '--budget-cpu-ms', '1')
# T4: autoscroll top -> bottom -> top (dev-tools build); 3 runs per document, gate on the median p99 <= 16 ms
# One untimed warm-up pass first: the runner's system font cache is cold for the faces that
# typical-basic uses (bold, italic, mono, fallback). Reported in 002_REPORT.md section 5.
& cargo xtask memprobe $dev --doc $typ --runs 1 --wait-title SCROLL_DONE --settle-ms 200 --arg --autoscroll --arg --perf-log --arg "$m/T4-warmup.log" | Out-Null
foreach ($d in @(@{n='typical'; f=$typ}, @{n='large'; f=$large})) {
  $p99s = @()
  foreach ($r in 1..3) {
    $log = "$m/T4-$($d.n)-$r.log"
    Probe "T4-$($d.n)-$r" $dev @('--doc', $d.f, '--runs', '1', '--wait-title', 'SCROLL_DONE', '--settle-ms', '500', '--arg', '--autoscroll', '--arg', '--perf-log', '--arg', $log)
    if (-not (Test-Path $log)) { $failed += "T4-$($d.n)-$r-log"; continue }
    $p99 = [double](((Get-Content $log) | Where-Object { $_ -like 'p99_ms *' }) -split ' ')[1]
    $frames = [int](((Get-Content $log) | Where-Object { $_ -like 'frames *' }) -split ' ')[1]
    Write-Host "T4-$($d.n)-$r frames = $frames, p99 = $p99 ms"
    if ($frames -lt 10) { $failed += "T4-$($d.n)-$r-frames" }
    $p99s += $p99
  }
  $med = ($p99s | Sort-Object)[[math]::Floor(($p99s.Count - 1) / 2)]
  Write-Host "T4-$($d.n) median p99 = $med ms (runs: $($p99s -join ', '))"
  "T4-$($d.n) median_p99_ms $med runs $($p99s -join ' ')" | Out-File -Append "$m/T4-summary.txt"
  if ($med -gt 16) { $failed += "T4-$($d.n)"; Write-Host "::error::T4 median p99 $med ms > 16 ms" }
}
# Subsystem breakdown (mem-stats build), informational
foreach ($d in @(@{n='M1b'; f=$typ}, @{n='M2'; f=$large})) {
  Probe "memstats-$($d.n)" $dev @('--doc', $d.f, '--runs', '1', '--arg', '--mem-stats', '--arg', "$m/memstats-$($d.n).app.json")
}
# 150 % scale screenshot is covered by the goldens (scale 1.5).
Get-ChildItem $m | Format-Table Name, Length -AutoSize
if ($failed.Count -gt 0) { Write-Host "::error::gates failed: $($failed -join ', ')"; exit 1 }
Write-Host 'all measurement gates green'
