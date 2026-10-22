# V1-D checks on windows-latest (prompt 008 §6): headless --export-html / --export-pdf and
# --install / --uninstall against a scratch LOCALAPPDATA/APPDATA and a scratch HKCU subkey
# (YAMDV_TEST_HKCU, dev-tools build only). Results land in ./m/v1d.
$ErrorActionPreference = 'Stop'
$out = (New-Item -ItemType Directory -Force -Path m/v1d).FullName
$rel = (Resolve-Path 'target/release/yamdv.exe').Path
$dev = (Resolve-Path 'target/dev/release/yamdv.exe').Path
$typ = (Resolve-Path 'tests/corpus/002/typical.md').Path
$fail = @()
function Run([string]$exe, [string[]]$a, [string]$tag) {
  $p = Start-Process $exe -ArgumentList $a -PassThru -RedirectStandardOutput "$out/$tag.out" -RedirectStandardError "$out/$tag.err"
  $null = $p.Handle
  if (-not $p.WaitForExit(300000)) { $p.Kill(); Write-Host "::error::$tag timed out"; return 99 }
  Write-Host "$tag exit $($p.ExitCode): $(Get-Content "$out/$tag.out" -Raw)$(Get-Content "$out/$tag.err" -Raw)"
  return $p.ExitCode
}

# Headless HTML (R-EXP-05): exit 0, file exists, no <script, CSP, every src is a data: URI.
$html = "$out/typical.html"
if ((Run $rel @('--export-html', "`"$typ`"", '-o', "`"$html`"", '--theme', 'github', '--mode', 'light') 'export-html') -ne 0) { $fail += 'export-html exit' }
if (Test-Path $html) {
  $t = Get-Content $html -Raw
  if ($t -match '(?i)<script') { $fail += 'export-html <script' }
  if ($t -notmatch "default-src 'none'; img-src data:") { $fail += 'export-html CSP' }
  $srcs = [regex]::Matches($t, ' src="([^"]*)"') | ForEach-Object { $_.Groups[1].Value }
  $bad = @($srcs | Where-Object { -not $_.StartsWith('data:image/') })
  if ($bad.Count -gt 0) { $fail += "export-html non-data src ($($bad.Count))" }
  Write-Host "export-html: $((Get-Item $html).Length) bytes, $($srcs.Count) embedded images"
} else { $fail += 'export-html missing file' }
if ((Run $rel @('--export-html', 'nope-missing.md') 'export-html-missing') -ne 1) { $fail += 'export-html missing input exit != 1' }
if ((Run $rel @('--export-html') 'export-html-usage') -ne 2) { $fail += 'export-html usage exit != 2' }

# Headless PDF (R-EXP-05, FUT-39 print path): skipped with a note when the printer is missing.
if (Get-CimInstance Win32_Printer -Filter "Name='Microsoft Print to PDF'" -ErrorAction SilentlyContinue) {
  $pdf = "$out/typical.pdf"
  if ((Run $rel @('--export-pdf', "`"$typ`"", '-o', "`"$pdf`"") 'export-pdf') -ne 0) { $fail += 'export-pdf exit' }
  $head = if (Test-Path $pdf) { [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($pdf), 0, 4) } else { '' }
  if ($head -ne '%PDF') { $fail += 'export-pdf not a PDF' }
  $pages = if ((Get-Content "$out/export-pdf.out" -Raw) -match 'pages ([0-9]+)') { [int]$Matches[1] } else { 0 }
  if ($pages -lt 2) { $fail += "export-pdf pages $pages" }
} else {
  Write-Host '::notice::export-pdf skipped: Microsoft Print to PDF is not installed on this runner'
}

# --install / --uninstall (R-SYS-05) in a scratch profile and a scratch HKCU subkey.
$saveApp = $env:APPDATA; $saveLocal = $env:LOCALAPPDATA
$prof = (New-Item -ItemType Directory -Force -Path "$env:RUNNER_TEMP/v1d-install").FullName
$env:APPDATA = (New-Item -ItemType Directory -Force -Path "$prof/Roaming").FullName
$env:LOCALAPPDATA = (New-Item -ItemType Directory -Force -Path "$prof/Local").FullName
$env:YAMDV_TEST_HKCU = 'Software\YAMDV-Test\ci'
$hive = "HKCU:\$($env:YAMDV_TEST_HKCU)"
$inst = "$env:LOCALAPPDATA\Programs\YAMDV\yamdv.exe"
$lnk = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\YAMDV.lnk"
$un = "$hive\Software\Microsoft\Windows\CurrentVersion\Uninstall\YAMDV"
foreach ($round in 1, 2) {
  if ((Run $dev @('--install', '--no-launch') "install-$round") -ne 0) { $fail += "install round $round exit" }
}
if (-not (Test-Path $inst)) { $fail += 'install: exe missing' }
if (-not (Test-Path $lnk)) { $fail += 'install: shortcut missing' }
if (Test-Path $un) {
  $v = Get-ItemProperty $un
  if ($v.DisplayVersion -ne '1.0.0') { $fail += "install: DisplayVersion $($v.DisplayVersion)" }
  if ($v.UninstallString -ne "`"$inst`" --uninstall") { $fail += "install: UninstallString $($v.UninstallString)" }
  if (-not $v.EstimatedSize) { $fail += 'install: EstimatedSize' }
} else { $fail += 'install: uninstall entry missing' }
if (-not (Test-Path "$hive\Software\Classes\YAMDV.Markdown\shell\open\command")) { $fail += 'install: ProgID missing' }
New-Item -ItemType Directory -Force -Path "$env:APPDATA\YAMDV" | Out-Null
Set-Content "$env:APPDATA\YAMDV\settings.ini" "[general]`r`n"
if ((Run $dev @('--uninstall', '--remove-settings') 'uninstall') -ne 0) { $fail += 'uninstall exit' }
foreach ($gone in $inst, $lnk, "$env:APPDATA\YAMDV") { if (Test-Path $gone) { $fail += "uninstall left $gone" } }
if (Test-Path $un) { $fail += 'uninstall left the uninstall entry' }
if (Test-Path "$hive\Software\Classes\YAMDV.Markdown") { $fail += 'uninstall left the ProgID' }
Remove-Item -Recurse -Force "HKCU:\Software\YAMDV-Test" -ErrorAction SilentlyContinue
Remove-Item Env:YAMDV_TEST_HKCU
$env:APPDATA = $saveApp; $env:LOCALAPPDATA = $saveLocal

if ($fail.Count -gt 0) { Write-Host "::error::v1d checks failed: $($fail -join '; ')"; exit 1 }
Write-Host 'v1d checks green'
