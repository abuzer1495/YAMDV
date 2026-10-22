#!/bin/bash
# Builds YAMDV-v2b-test.zip (+ .sha256) with cargo-xwin (prompt 012 §5.5, A-8).
# Usage: tools/testpack/build-pack.sh <baseline-dev-exe> [out-dir]
set -euo pipefail
cd "$(dirname "$0")/../.."
base=${1:?baseline dev-tools yamdv.exe (main before this phase)}
out=${2:-target/pack}
T=x86_64-pc-windows-msvc
cargo xwin build --release --locked --target $T -p yamdv-app -p xtask
cargo xwin build --release --locked --target $T -p yamdv-app --features dev-tools,mem-stats --target-dir target/dev
d=$out/YAMDV-v2b-test
rm -rf "$out" && mkdir -p "$d/baseline" "$d/tests"
cp target/$T/release/yamdv.exe "$d/yamdv.exe"
cp target/dev/$T/release/yamdv.exe "$d/yamdv-dev.exe"
cp target/$T/release/xtask.exe "$d/xtask.exe"
cp "$base" "$d/baseline/yamdv-dev.exe"
cp tools/testpack/run-tests.cmd tools/testpack/run-tests.ps1 tools/testpack/OWNER_TEST.md "$d/"
for t in driver fixtures golden corpus; do cp -r "tests/$t" "$d/tests/"; done
find "$d/tests" \( -name '*.log' -o -name '*.tmp.*' \) -delete
# CRLF for the Windows scripts.
for f in "$d/run-tests.cmd" "$d/run-tests.ps1"; do sed -i 's/\r\?$/\r/' "$f"; done
(cd "$out" && rm -f YAMDV-v2b-test.zip && python3 -c 'import shutil; shutil.make_archive("YAMDV-v2b-test", "zip", ".", "YAMDV-v2b-test")' &&
  sha256sum YAMDV-v2b-test.zip > YAMDV-v2b-test.zip.sha256 && cat YAMDV-v2b-test.zip.sha256)
