# V2-B fix diagnosis: start every cargo-xwin variant and report its exit code; then show the
# Windows Error Reporting / Application Error events and a cdb stack for the crashing pack exe.
$ErrorActionPreference = 'Continue'
Get-ChildItem pack/variants/*.exe | ForEach-Object {
  $a = if ($_.Name -like 'hello*') { @() } else { @('--version') }
  $out = & $_.FullName @a 2>&1 | Out-String
  Write-Host ("variant {0}: exit 0x{1:X8} out='{2}'" -f $_.Name, $LASTEXITCODE, $out.Trim())
}
Get-WinEvent -FilterHashtable @{ LogName = 'Application'; Id = 1000, 1001 } -MaxEvents 6 -ErrorAction SilentlyContinue |
  ForEach-Object { Write-Host "--- event $($_.Id) $($_.TimeCreated)"; Write-Host $_.Message }
$cdb = Get-ChildItem -Recurse -Filter cdb.exe -Path "${env:ProgramFiles(x86)}\Windows Kits\10\Debuggers\x64" -ErrorAction SilentlyContinue | Select-Object -First 1
if ($cdb) {
  Write-Host "--- cdb $($cdb.FullName)"
  & $cdb.FullName -g -G -lines -c ".lastevent; r; kn 40; lm m yamdv; q" pack/yamdv.exe --version 2>&1 | Select-Object -Last 90 | ForEach-Object { Write-Host $_ }
} else { Write-Host 'cdb not found' }
$real = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'YAMDV\logs'
Get-ChildItem -Filter 'crash-*.txt' -Path $real -ErrorAction SilentlyContinue | ForEach-Object { Write-Host "--- $($_.Name)"; Get-Content $_.FullName }
