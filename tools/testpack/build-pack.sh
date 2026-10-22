#!/bin/bash
# Builds YAMDV-v2b-test.zip (+ .sha256) from the MSVC-built exes of the CI job "pack exes (MSVC)"
# (artifact `pack-exes`: yamdv.exe, yamdv-dev.exe, xtask.exe, baseline-dev.exe, COMMIT).
# V2-B fix (A-9): exes linked by cargo-xwin/lld-link exited 0xC0000409 on every Windows start, so
# this script no longer builds anything itself and refuses exes without the MSVC Rich header.
# Usage: tools/testpack/build-pack.sh <run-id | pack-exes-dir> [out-dir]
set -euo pipefail
cd "$(dirname "$0")/../.."
src=${1:?CI run id (gh run download ... -n pack-exes) or a directory with the pack exes}
out=${2:-target/pack}
if [[ $src =~ ^[0-9]+$ ]]; then
  dl=$(mktemp -d)
  gh run download "$src" -n pack-exes -D "$dl"
  src=$dl
fi
for f in yamdv.exe yamdv-dev.exe xtask.exe baseline-dev.exe; do
  [[ -f $src/$f ]] || { echo "missing $src/$f" >&2; exit 1; }
  # MSVC link.exe writes a "Rich" header into the DOS stub; lld-link does not.
  head -c 1024 "$src/$f" | grep -qa Rich || { echo "$f was not linked by MSVC link.exe (no Rich header)" >&2; exit 1; }
done
d=$out/YAMDV-v2b-test
rm -rf "$out" && mkdir -p "$d/baseline" "$d/tests"
cp "$src/yamdv.exe" "$d/yamdv.exe"
cp "$src/yamdv-dev.exe" "$d/yamdv-dev.exe"
cp "$src/xtask.exe" "$d/xtask.exe"
cp "$src/baseline-dev.exe" "$d/baseline/yamdv-dev.exe"
[[ -f $src/COMMIT ]] && echo "exes built by CI from commit $(tr -d '\r\n' < "$src/COMMIT")"
cp tools/testpack/run-tests.cmd tools/testpack/run-tests.ps1 tools/testpack/OWNER_TEST.md "$d/"
for t in driver fixtures golden corpus; do cp -r "tests/$t" "$d/tests/"; done
find "$d/tests" \( -name '*.log' -o -name '*.tmp.*' \) -delete
# CRLF for the Windows scripts.
for f in "$d/run-tests.cmd" "$d/run-tests.ps1"; do sed -i 's/\r\?$/\r/' "$f"; done
(cd "$out" && rm -f YAMDV-v2b-test.zip && python3 -c 'import shutil; shutil.make_archive("YAMDV-v2b-test", "zip", ".", "YAMDV-v2b-test")' &&
  sha256sum YAMDV-v2b-test.zip > YAMDV-v2b-test.zip.sha256 && cat YAMDV-v2b-test.zip.sha256)
