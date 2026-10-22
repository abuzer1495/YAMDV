# YAMDV V2-B owner test pack (prompt 012 section 5, PROCESS A-8). Ports the CI Windows test job:
# every driver script, the golden compares, M0/M1 (also on the baseline build), T7 and T4,
# the headless export checks and the export-progress script (BUG-11). Writes test-results.txt
# (PASS / FAIL / SKIP + numbers), failure logs and diffs to failures\, screenshots to screens\.
# Nothing here uses the network. The owner's own settings are never read or written: every
# app start gets a temporary APPDATA / LOCALAPPDATA.
param([switch]$IncludeSystem)
$ErrorActionPreference = 'Continue'
$root = $PSScriptRoot
Set-Location $root
$env:YAMDV_ROOT = $root
Get-ChildItem -Recurse -File $root | Unblock-File -ErrorAction SilentlyContinue
$rel = Join-Path $root 'yamdv.exe'
$dev = Join-Path $root 'yamdv-dev.exe'
$base = Join-Path $root 'baseline\yamdv-dev.exe'
$xt = Join-Path $root 'xtask.exe'
$res = Join-Path $root 'test-results.txt'
$fails = Join-Path $root 'failures'
$screens = Join-Path $root 'screens'
$m = Join-Path $root 'results'
foreach ($d in $fails, $screens, $m) {
  if (Test-Path $d) { Remove-Item -Recurse -Force $d }
  New-Item -ItemType Directory -Force -Path $d | Out-Null
}
$tmp = Join-Path $env:TEMP ("yamdv-v2b-test-" + (Get-Date -Format 'yyyyMMdd-HHmmss'))
New-Item -ItemType Directory -Force -Path $tmp | Out-Null
$origApp = $env:APPDATA; $origLocal = $env:LOCALAPPDATA
$script:gateFail = @()
$script:counts = @{ PASS = 0; FAIL = 0; SKIP = 0 }

function Out-Result([string]$status, [string]$name, [string]$detail) {
  $line = '{0,-5} {1,-34} {2}' -f $status, $name, $detail
  Add-Content -Path $res -Value $line -Encoding UTF8
  $c = switch ($status) { 'PASS' { 'Green' } 'FAIL' { 'Red' } default { 'Yellow' } }
  Write-Host $line -ForegroundColor $c
  if ($script:counts.ContainsKey($status)) { $script:counts[$status]++ }
}
function Note([string]$text) { Add-Content -Path $res -Value $text -Encoding UTF8; Write-Host $text }
function Use-Profile([string]$name) {
  $p = (New-Item -ItemType Directory -Force -Path (Join-Path $tmp "prof-$name")).FullName
  $env:APPDATA = $p; $env:LOCALAPPDATA = $p
}
function Restore-Profile { $env:APPDATA = $origApp; $env:LOCALAPPDATA = $origLocal }

Add-Type -TypeDefinition @"
using System; using System.Runtime.InteropServices;
public static class YW {
  [DllImport("user32.dll")] public static extern uint GetDpiForSystem();
  [DllImport("user32.dll", SetLastError=true)] public static extern IntPtr SendMessageTimeoutW(
    IntPtr h, uint msg, UIntPtr w, IntPtr l, uint flags, uint timeout, out UIntPtr result);
  public static bool Alive(IntPtr h) {
    UIntPtr r; return SendMessageTimeoutW(h, 0, UIntPtr.Zero, IntPtr.Zero, 2, 100, out r) != IntPtr.Zero;
  }
}
"@

# ---- header --------------------------------------------------------------------------------
$dpi = [YW]::GetDpiForSystem()
$os = Get-CimInstance Win32_OperatingSystem
$ver = (& $rel --version 2>$null | Out-String).Trim()
Set-Content -Path $res -Encoding UTF8 -Value @(
  'YAMDV V2-B owner test results (prompt 012, A-8)',
  "date      $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss zzz')",
  "os        $($os.Caption) $($os.Version) build $($os.BuildNumber)",
  "version   $ver",
  "dpi       $dpi (CI renders at 96)",
  "system    $(if ($IncludeSystem) { 'install/uninstall checks included' } else { 'install/uninstall checks skipped (no --include-system)' })",
  '')
Write-Host ''
Write-Host 'This takes about 20-25 minutes. Please do not touch the mouse or keyboard until it ends.' -ForegroundColor Cyan
Write-Host 'Yaklasik 20-25 dakika surer. Bitene kadar lutfen fareye ve klavyeye dokunmayin.' -ForegroundColor Cyan
Write-Host 'Starting in 10 s (Ctrl+C to stop) / 10 sn icinde basliyor (durdurmak icin Ctrl+C)...'
Start-Sleep -Seconds 10
foreach ($f in $rel, $dev, $base, $xt) { if (-not (Test-Path $f)) { Out-Result 'FAIL' 'pack' "missing $f" } }
# 96 DPI: the scripts and goldens expect 96 DPI. On another system DPI the GUI runs of the
# dev-tools build get the per-process "DPIUNAWARE" compatibility layer (Windows then scales the
# window as a bitmap; nothing is changed system-wide). Measurements run without it.
$compat = ''
if ($dpi -ne 96) { $compat = 'DPIUNAWARE'; Note "note      system DPI is ${dpi}: driver and golden runs use __COMPAT_LAYER=DPIUNAWARE" }

# ---- driver scripts (CI "in-window driver scripts") -----------------------------------------
Note ''; Note '== driver scripts (tests\driver\*.yds, dev-tools build, 30 s each; export-progress 300 s)'
$doc = Join-Path $root 'tests\driver\a.md'
$env:YAMDV_SNAPSHOT_DIR = $screens
if ($compat) { $env:__COMPAT_LAYER = $compat }
$hasPdf = [bool](Get-CimInstance Win32_Printer -Filter "Name='Microsoft Print to PDF'" -ErrorAction SilentlyContinue)
$large = Join-Path $root 'tests\fixtures\large-1mb.md'
if (-not (Test-Path $large)) { & $xt gen-large $large | Out-Null }
foreach ($s in Get-ChildItem (Join-Path $root 'tests\driver\*.yds')) {
  $ep = $s.BaseName -eq 'export-progress'
  if ($ep -and -not $hasPdf) { Out-Result 'SKIP' "driver/$($s.Name)" 'Microsoft Print to PDF is not installed'; continue }
  Use-Profile $s.BaseName
  $log = "$($s.FullName).log"
  Remove-Item -Force $log -ErrorAction SilentlyContinue
  $sw = [Diagnostics.Stopwatch]::StartNew()
  $p = Start-Process $dev -ArgumentList '--script', "`"$($s.FullName)`"", "`"$doc`"" -PassThru
  $null = $p.Handle
  $limit = if ($ep) { 300000 } else { 30000 }
  $polls = 0; $hung = 0; $jobPolls = 0
  if ($ep) {
    # Run 1 check from outside: while a worker (a second yamdv-dev process) runs, the window
    # must answer WM_NULL within 100 ms every time.
    while (-not $p.HasExited -and $sw.ElapsedMilliseconds -lt $limit) {
      Start-Sleep -Milliseconds 100
      $p.Refresh()
      $h = $p.MainWindowHandle
      if ($h -eq [IntPtr]::Zero) { continue }
      $workers = @(Get-Process -Name 'yamdv-dev' -ErrorAction SilentlyContinue).Count - 1
      $ok = [YW]::Alive($h)
      $polls++
      if ($workers -gt 0) { $jobPolls++; if (-not $ok) { $hung++ } }
    }
  }
  $done = $p.WaitForExit([Math]::Max(1, $limit - $sw.ElapsedMilliseconds))
  $secs = [Math]::Round($sw.Elapsed.TotalSeconds, 1)
  Restore-Profile
  if (-not $done) { $p.Kill(); Out-Result 'FAIL' "driver/$($s.Name)" "timed out after $secs s"; if (Test-Path $log) { Copy-Item $log $fails }; continue }
  if (Test-Path $log) { Copy-Item $log (Join-Path $m "$($s.Name).log") }
  $extra = ''
  if ($ep) { $extra = "; WM_NULL polls $polls, during the job $jobPolls, no answer within 100 ms: $hung" }
  $left = @(Get-Process -Name 'yamdv-dev' -ErrorAction SilentlyContinue).Count
  if ($ep -and $left -gt 0) { $extra += "; $left yamdv-dev process(es) left" }
  if ($p.ExitCode -eq 0 -and $hung -eq 0 -and -not ($ep -and ($left -gt 0 -or $jobPolls -eq 0))) {
    Out-Result 'PASS' "driver/$($s.Name)" "$secs s$extra"
  } else {
    Out-Result 'FAIL' "driver/$($s.Name)" "exit $($p.ExitCode), $secs s$extra"
    if (Test-Path $log) { Copy-Item $log $fails }
  }
  Get-Process -Name 'yamdv-dev' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
}
Remove-Item Env:YAMDV_SNAPSHOT_DIR -ErrorAction SilentlyContinue
Get-ChildItem (Join-Path $root 'tests\driver') -Filter '*.tmp.*' -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue

# ---- goldens (report only: a 96-DPI, same-fonts reference; the owner gate is the screens\) ----
Note ''; Note '== goldens (xtask golden / golden-shell / acg-matrix; report only, diffs in failures\)'
function XRun([string]$name, [string[]]$a, [string]$outDir) {
  $sw = [Diagnostics.Stopwatch]::StartNew()
  $o = & $xt @a 2>&1 | Out-String
  $code = $LASTEXITCODE
  Set-Content -Path (Join-Path $m "$name.txt") -Value $o -Encoding UTF8
  $secs = [Math]::Round($sw.Elapsed.TotalSeconds, 1)
  if ($code -eq 0) { Out-Result 'PASS' $name "$secs s" }
  else {
    $why = (($o -split "`n") | Where-Object { $_ -match 'error|fail' } | Select-Object -Last 1)
    Out-Result 'FAIL' $name "exit $code, $secs s (not a gate): $why"
    Copy-Item (Join-Path $m "$name.txt") $fails
    if ($outDir -and (Test-Path $outDir)) { Copy-Item -Recurse $outDir (Join-Path $fails $name) }
  }
}
XRun 'golden' @('golden', '--exe', $dev, '--out', (Join-Path $m 'golden')) (Join-Path $m 'golden')
XRun 'golden-shell' @('golden-shell', '--exe', $dev, '--out', (Join-Path $m 'golden-shell')) (Join-Path $m 'golden-shell')
XRun 'acg-matrix' @('acg-matrix', '--exe', $dev, '--out', (Join-Path $m 'acg')) ''
Remove-Item Env:__COMPAT_LAYER -ErrorAction SilentlyContinue

# ---- headless export checks (CI v1d-checks.ps1) ----------------------------------------------
Note ''; Note '== headless export (--export-html / --export-pdf)'
$typ = Join-Path $root 'tests\corpus\002\typical.md'
function Run([string]$exe, [string[]]$a, [string]$tag) {
  $p = Start-Process $exe -ArgumentList $a -PassThru -RedirectStandardOutput "$m\$tag.out" -RedirectStandardError "$m\$tag.err"
  $null = $p.Handle
  if (-not $p.WaitForExit(300000)) { $p.Kill(); return 99 }
  return $p.ExitCode
}
Use-Profile 'headless'
$html = "$m\typical.html"
$c = Run $rel @('--export-html', "`"$typ`"", '-o', "`"$html`"", '--theme', 'github', '--mode', 'light') 'export-html'
$why = @()
if ($c -ne 0) { $why += "exit $c" }
if (Test-Path $html) {
  $t = Get-Content $html -Raw
  if ($t -match '(?i)<script') { $why += '<script' }
  if ($t -notmatch "default-src 'none'; img-src data:") { $why += 'CSP' }
  $srcs = @([regex]::Matches($t, ' src="([^"]*)"') | ForEach-Object { $_.Groups[1].Value })
  if (@($srcs | Where-Object { -not $_.StartsWith('data:image/') }).Count -gt 0) { $why += 'non-data src' }
} else { $why += 'no file' }
if ((Run $rel @('--export-html', 'nope-missing.md') 'export-html-missing') -ne 1) { $why += 'missing input exit != 1' }
if ((Run $rel @('--export-html') 'export-html-usage') -ne 2) { $why += 'usage exit != 2' }
if ($why.Count -eq 0) { Out-Result 'PASS' 'export-html (headless)' "$((Get-Item $html).Length) bytes, $($srcs.Count) images" } else { Out-Result 'FAIL' 'export-html (headless)' ($why -join '; ') }
if ($hasPdf) {
  $pdf = "$m\typical.pdf"
  $c = Run $rel @('--export-pdf', "`"$typ`"", '-o', "`"$pdf`"") 'export-pdf'
  $head = if (Test-Path $pdf) { [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($pdf), 0, 4) } else { '' }
  $pages = if ((Get-Content "$m\export-pdf.out" -Raw) -match 'pages ([0-9]+)') { [int]$Matches[1] } else { 0 }
  if ($c -eq 0 -and $head -eq '%PDF' -and $pages -ge 2) { Out-Result 'PASS' 'export-pdf (headless)' "$pages pages" } else { Out-Result 'FAIL' 'export-pdf (headless)' "exit $c, header '$head', pages $pages" }
} else { Out-Result 'SKIP' 'export-pdf (headless)' 'Microsoft Print to PDF is not installed' }
Restore-Profile

if ($IncludeSystem) {
  # Same as CI: scratch profile + scratch HKCU subkey (YAMDV_TEST_HKCU, dev-tools build only).
  $prof = (New-Item -ItemType Directory -Force -Path "$tmp\install").FullName
  $env:APPDATA = (New-Item -ItemType Directory -Force -Path "$prof\Roaming").FullName
  $env:LOCALAPPDATA = (New-Item -ItemType Directory -Force -Path "$prof\Local").FullName
  $env:YAMDV_TEST_HKCU = 'Software\YAMDV-Test\owner'
  $hive = "HKCU:\$($env:YAMDV_TEST_HKCU)"
  $inst = "$env:LOCALAPPDATA\Programs\YAMDV\yamdv.exe"
  $lnk = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\YAMDV.lnk"
  $un = "$hive\Software\Microsoft\Windows\CurrentVersion\Uninstall\YAMDV"
  $why = @()
  foreach ($round in 1, 2) { if ((Run $dev @('--install', '--no-launch') "install-$round") -ne 0) { $why += "install $round exit" } }
  if (-not (Test-Path $inst)) { $why += 'exe missing' }
  if (-not (Test-Path $lnk)) { $why += 'shortcut missing' }
  $want = ($ver -split ' ')[-1]
  if (Test-Path $un) {
    $v = Get-ItemProperty $un
    if ($v.DisplayVersion -ne $want) { $why += "DisplayVersion $($v.DisplayVersion)" }
    if ($v.UninstallString -ne "`"$inst`" --uninstall") { $why += 'UninstallString' }
  } else { $why += 'uninstall entry missing' }
  if (-not (Test-Path "$hive\Software\Classes\YAMDV.Markdown\shell\open\command")) { $why += 'ProgID missing' }
  New-Item -ItemType Directory -Force -Path "$env:APPDATA\YAMDV" | Out-Null
  Set-Content "$env:APPDATA\YAMDV\settings.ini" "[general]`r`n"
  if ((Run $dev @('--uninstall', '--remove-settings') 'uninstall') -ne 0) { $why += 'uninstall exit' }
  foreach ($gone in $inst, $lnk, "$env:APPDATA\YAMDV") { if (Test-Path $gone) { $why += "left $gone" } }
  if (Test-Path $un) { $why += 'left the uninstall entry' }
  Remove-Item -Recurse -Force 'HKCU:\Software\YAMDV-Test' -ErrorAction SilentlyContinue
  Remove-Item Env:YAMDV_TEST_HKCU
  Restore-Profile
  if ($why.Count -eq 0) { Out-Result 'PASS' 'install/uninstall (scratch)' '' } else { Out-Result 'FAIL' 'install/uninstall (scratch)' ($why -join '; ') }
} else { Out-Result 'SKIP' 'install/uninstall' 'run with --include-system' }

# ---- measurements (CI measure.ps1: M0, M1, T7, T4) -------------------------------------------
Note ''; Note '== measurements (xtask memprobe, window 1000x700; gates: M0/M1 vs baseline <= +0.3 MiB, T7 = 0)'
$size = @('--size', '1000x700')
$m1x = @('--doc', $typ, '--arg', '--line', '--arg', '822')
function Probe([string]$label, [string]$exe, [string[]]$extra, [bool]$retry) {
  Use-Profile "probe-$label"
  $json = "$m\$label.json"
  $o = & $xt memprobe $exe --json $json --label $label @size @extra 2>&1 | Out-String
  $code = $LASTEXITCODE
  if ($code -ne 0 -and $retry) {
    if (Test-Path $json) { Move-Item -Force $json "$m\$label.a1.json" }
    $o += & $xt memprobe $exe --json $json --label $label @size @extra 2>&1 | Out-String
    $code = $LASTEXITCODE
  }
  Restore-Profile
  Set-Content -Path "$m\$label.txt" -Value $o -Encoding UTF8
  $j = if (Test-Path $json) { (Get-Content $json -Raw | ConvertFrom-Json).median } else { $null }
  return [pscustomobject]@{ code = $code; m = $j; out = $o }
}
function Mib($r) { if ($r.m) { [double]$r.m.private_ws_mib } else { [double]::NaN } }
$probes = @(
  @{ l = 'M0'; x = @('--runs', '5', '--budget-pws-mib', '4.8') },
  @{ l = 'M1'; x = ($m1x + @('--runs', '5', '--budget-pws-mib', '8.0')) })
foreach ($pr in $probes) {
  $r = Probe "$($pr.l)-release" $rel $pr.x $true
  $st = if ($r.code -eq 0) { 'PASS' } else { 'FAIL' }
  Out-Result $st "$($pr.l) release (budget)" ("PWS {0:N2} MiB" -f (Mib $r))
  $a = Probe "$($pr.l)-dev" $dev $pr.x $false
  $b = Probe "$($pr.l)-baseline" $base $pr.x $false
  $d = (Mib $a) - (Mib $b)
  # Gate (A-8): the same build flavour before and after this phase on this PC; one retry.
  if (-not ($d -le 0.3)) {
    $a = Probe "$($pr.l)-dev" $dev $pr.x $false
    $b = Probe "$($pr.l)-baseline" $base $pr.x $false
    $d = (Mib $a) - (Mib $b)
  }
  $txt = "V2-B {0:N2} MiB, baseline {1:N2} MiB, delta {2:+0.00;-0.00} MiB (gate <= +0.3)" -f (Mib $a), (Mib $b), $d
  if ($d -le 0.3) { Out-Result 'PASS' "$($pr.l) vs baseline (gate)" $txt } else { Out-Result 'FAIL' "$($pr.l) vs baseline (gate)" $txt; $script:gateFail += $pr.l; Copy-Item "$m\$($pr.l)-*.txt" $fails }
}
foreach ($t in @(@{ l = 'T7-empty'; x = @() }, @{ l = 'T7-doc'; x = @('--doc', $typ) })) {
  $r = Probe $t.l $rel ($t.x + @('--runs', '1', '--cpu-ms', '30000', '--budget-cpu-ms', '1')) $false
  $cpu = if ($r.m) { "idle CPU $($r.m.idle_cpu_us) us in 30 s (budget 1 ms)" } else { 'no result' }
  if ($r.code -eq 0) { Out-Result 'PASS' "$($t.l) (gate)" $cpu } else { Out-Result 'FAIL' "$($t.l) (gate)" $cpu; $script:gateFail += $t.l; Copy-Item "$m\$($t.l).txt" $fails }
}
# T4: autoscroll top -> bottom -> top, dev-tools build; run 1 cold, runs 2-4 warm (median p99).
$t4docs = @(
  @{ n = 'typical'; f = $typ },
  @{ n = 'large'; f = $large },
  @{ n = 'math'; f = (Join-Path $root 'tests\corpus\002\math-heavy.md') },
  @{ n = 'mermaid'; f = (Join-Path $root 'tests\corpus\002\mermaid-heavy.md') })
foreach ($d in $t4docs) {
  $p99s = @()
  foreach ($tag in 'cold', 'warm1', 'warm2', 'warm3') {
    $log = "$m\T4-$($d.n)-$tag.log"
    $null = Probe "T4-$($d.n)-$tag" $dev @('--doc', $d.f, '--runs', '1', '--wait-title', 'SCROLL_DONE', '--settle-ms', '500', '--arg', '--autoscroll', '--arg', '--perf-log', '--arg', $log) $false
    $v = if (Test-Path $log) { ((Get-Content $log) | Where-Object { $_ -like 'p99_ms *' }) } else { $null }
    $p99s += if ($v) { [double](($v -split ' ')[1]) } else { [double]::NaN }
  }
  $warm = @($p99s[1..3] | Where-Object { -not [double]::IsNaN($_) } | Sort-Object)
  $med = if ($warm.Count) { $warm[[math]::Floor(($warm.Count - 1) / 2)] } else { [double]::NaN }
  $txt = "cold p99 {0:N2} ms, warm median p99 {1:N2} ms (CI target <= 16)" -f $p99s[0], $med
  if ($warm.Count -eq 0) { Out-Result 'FAIL' "T4 $($d.n) (reported)" "no perf log" } else { Out-Result 'PASS' "T4 $($d.n) (reported)" $txt }
}

# ---- summary --------------------------------------------------------------------------------
Get-Process -Name 'yamdv-dev', 'yamdv' -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "$root*" } | Stop-Process -Force -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
Note ''
Note ("summary   PASS {0}  FAIL {1}  SKIP {2}" -f $script:counts.PASS, $script:counts.FAIL, $script:counts.SKIP)
if ($script:gateFail.Count -gt 0) { Note "gates     FAILED: $($script:gateFail -join ', ')" } else { Note 'gates     M0/M1 vs baseline and T7: green' }
Note "screens   $(@(Get-ChildItem $screens -Filter *.png -ErrorAction SilentlyContinue).Count) PNG files in screens\ (no golden compare; please look at them)"
Write-Host ''
Write-Host "Results: $res" -ForegroundColor Cyan
