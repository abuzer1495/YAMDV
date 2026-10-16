# Phase 001 measurements on windows-latest (§4.1, §4.7, Appendix D §D.3).
# Results land in ./m (uploaded as the "measurements" artifact).
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path m | Out-Null
$m = (Resolve-Path m).Path

function Probe([string]$exe, [string]$label, [string[]]$extra) {
  Write-Host "== memprobe $label"
  & cargo xtask memprobe $exe --json "$m/$label.json" --label $label @extra
  if ($LASTEXITCODE -ne 0) { Write-Host "::warning::memprobe $label failed ($LASTEXITCODE)" }
}

Get-CimInstance Win32_OperatingSystem | Select-Object Caption, Version, BuildNumber | Format-List | Out-File "$m/host.txt"
Get-CimInstance Win32_VideoController | Select-Object Name, CurrentHorizontalResolution, CurrentVerticalResolution | Format-List | Out-File -Append "$m/host.txt"
Get-ChildItem target/release/*.exe, target/seg/release/*.exe | Select-Object FullName, Length | Format-Table -AutoSize | Out-File "$m/sizes.txt"

foreach ($v in @(@{dir='target/release'; tag='default'}, @{dir='target/seg/release'; tag='segheap'})) {
  $d = $v.dir; $t = $v.tag
  Probe "$d/yamdv.exe" "skeleton-$t" @('--runs', '5', '--screenshot', "$m/skeleton-$t.png")
  foreach ($s in 'a', 'b', 'c') {
    $exe = "$d/spike-$s.exe"
    Probe $exe "spike-$s-$t-idle" @('--runs', '3', '--screenshot', "$m/spike-$s-$t-100.png", '--arg', '--log', '--arg', "$m/spike-$s-$t-idle.log")
    Probe $exe "spike-$s-$t-scroll" @('--runs', '3', '--wait-title', 'SCROLL_DONE', '--arg', '--autoscroll', '--arg', '--log', '--arg', "$m/spike-$s-$t-scroll.log")
    if ($t -eq 'default') {
      Probe $exe "spike-$s-$t-150" @('--runs', '1', '--screenshot', "$m/spike-$s-$t-150.png", '--arg', '--scale', '--arg', '1.5', '--arg', '--log', '--arg', "$m/spike-$s-$t-150.log")
    }
  }
}
Get-ChildItem $m | Format-Table Name, Length -AutoSize
