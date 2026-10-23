# V2-C plan (prompt 013): CI back, v1.1.0, tooltips/popovers/banners, fullscreen viewers, folding

## Goals (restated)
- §2 setup: official A-9, roadmap lines, P-40…P-42, Font dialog-kind cleanup, v1.1.0 draft release from a
  green main, test-v2b deleted (it no longer exists on abuzer1495/YAMDV: nothing to delete).
- §3: tooltip component, anchored popover (external-link Ask), banners, fullscreen viewers (image, Mermaid,
  table), heading folding, sup/sub/footnote-ref inline objects, BUG-08 minimum diagram scale.
- Tests: 3 driver scripts (tooltips-banners, viewers, folding), 4 goldens, pure unit tests.

## v1.1.0 route
a5ebb9a (the V2-B fix merge) carries "[skip ci]" in its squash message, so its push and the tag
`phase-v2b-fix` ran no CI, and a v1.1.0 tag on it would be skipped as well. So the docs-only setup
(report, A-9, roadmap, P-40…P-42) is merged first as its own squash commit with a clean message; its main
push run is the "CI green on main", then `v1.1.0` is tagged on it (tag run → draft release job).
The branch then continues (merge of origin/main, no history rewrite) with the V2-C features.

## Risks
- Scope is large for one phase; order: pure logic + tests first, then platform pieces, then scripts/goldens.
- T7 with a viewer open: no timers unless animating.

## CHECKPOINT
- 09:19: d9573cc v2b_fix report + PROCESS A-8 pack wording (first commit). Prompt 013 stored (cmp identical). A-9 official text, roadmap lines, P-40…P-42 applied, spec-check green.
- 09:20: CI: PR runs upload yamdv-exe (7 days, A-9 §5); pack-exes/pack-smoke only on workflow_dispatch (A-9 §3). Next: setup PR squash-merge with a clean message → main push run → tag v1.1.0.
- 09:22: setup PR #81 squash-merged as bd6acc6; main push run 39910926460 running. Font dialog-host kind removed (ipc 3 now Unknown, dialog_host, sys::font_dialog, window arm, cli).
