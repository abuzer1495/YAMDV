# V2-B fix plan (startup crash, CI back on; A-9)

Branch `fix/v2b-startup-crash`. Owner pack run: every driver script exit 0xC0000409 after ~0.6 s; `--version` empty.

1. A-9 amendment, URLs → abuzer1495/YAMDV, startup smoke step in CI (`.github/scripts/smoke.ps1`).
2. Root cause from CI logs (crash note) + static analysis; fix + regression check.
3. PR, full CI green (32 driver scripts, goldens, gates).
4. Merge, tag `phase-v2b-fix`, rebuild test pack, `gh release upload test-v2b --clobber` (draft).
5. Report `docs/reports/v2b_fix_REPORT.md`.

## CHECKPOINT
- 2026-10-22 23:15 (UTC+3): step 1 committed and pushed; waiting for first CI run.
