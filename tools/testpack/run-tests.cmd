@echo off
rem YAMDV owner test pack (A-8): runs the CI Windows test job locally and writes test-results.txt.
rem Usage: run-tests.cmd [--include-system]   (--include-system also runs --install/--uninstall
rem against a scratch HKCU subkey and a scratch profile; nothing else touches the registry.)
setlocal
cd /d "%~dp0"
set "YARGS="
if /i "%~1"=="--include-system" set "YARGS=-IncludeSystem"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0run-tests.ps1" %YARGS%
echo.
echo Done - please send test-results.txt (and the failures folder if it is not empty).
echo Bitti - lutfen test-results.txt dosyasini (failures klasoru bos degilse onu da) gonderin.
pause
