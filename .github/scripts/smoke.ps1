# Startup smoke checks (V2-B fix, A-9). Runs before the long gates so a crash at startup is
# visible in the log with its exit code and the crash note (panic location + message).
param([string]$Rel = 'target/release/yamdv.exe', [string]$Dev = 'target/dev/release/yamdv.exe')
$ErrorActionPreference = 'Continue'
$fail = 0
$root = (New-Item -ItemType Directory -Force -Path "$env:RUNNER_TEMP/smoke").FullName
function Show-Crash([string]$prof) {
  Get-ChildItem -Recurse -Filter 'crash-*.txt' -Path $prof -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Host "--- crash note $($_.Name)"; Get-Content $_.FullName
  }
}
function Run([string]$name, [string]$exe, [string[]]$argv, [int]$ms) {
  $prof = (New-Item -ItemType Directory -Force -Path "$root/prof-$name").FullName
  $env:APPDATA = $prof; $env:LOCALAPPDATA = $prof
  $so = "$root/$name.out"; $se = "$root/$name.err"
  $p = Start-Process $exe -ArgumentList $argv -PassThru -RedirectStandardOutput $so -RedirectStandardError $se
  $null = $p.Handle
  if (-not $p.WaitForExit($ms)) { $p.Kill(); Write-Host "::error::smoke $name timed out"; Show-Crash $prof; return 1 }
  $code = $p.ExitCode
  Write-Host ("smoke {0}: exit {1} (0x{2:X8})" -f $name, $code, $code)
  Write-Host "stdout: $((Get-Content $so -Raw) -replace '\s+$','')"
  $err = (Get-Content $se -Raw) -replace '\s+$', ''
  if ($err) { Write-Host "stderr: $err" }
  Show-Crash $prof
  if ($code -ne 0) { Write-Host "::error::smoke $name failed (exit $code)"; return 1 }
  return 0
}
$fail += Run 'rel-version' $Rel @('--version') 20000
$v = (Get-Content "$root/rel-version.out" -Raw)
if ($v -notmatch 'YAMDV') { Write-Host "::error::--version printed nothing"; $fail++ }
$fail += Run 'dev-version' $Dev @('--version') 20000
$doc = (Resolve-Path tests/driver/a.md).Path
$fail += Run 'dev-start' $Dev @('--script', (Resolve-Path tests/smoke/start.yds).Path, $doc) 30000
if (Test-Path tests/smoke/start.yds.log) { Get-Content tests/smoke/start.yds.log }
exit [int]($fail -gt 0)
