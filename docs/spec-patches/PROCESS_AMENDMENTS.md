## Amendments

**A-1 (Phase 002, prompt 002 F-01).** `spikes/` may use `unsafe` (excluded from the product; every block still has `// SAFETY:`), resolving the H14 vs E.9 conflict.

**A-2 (Phase 002, prompt 002 F-01).** Report section "0. Fix items status" and section "3b. Spec patches applied" are mandatory from Phase 002 on: `## 0. Fix items status` (table `F-ID | status | evidence`) before section 1, and `## 3b. Spec patches applied` (table `P-ID | doc | verified by spec-check (yes/no)`) after section 3. Sections with nothing to report say "None." and are never omitted.

**A-3 (Phase 003b, prompt 004 F-18).**
```text
A-3 Token economy (from Phase 003b on):
1. Read spec docs selectively: grep for the requirement IDs you are working on; never re-read whole docs or prompt files you have already read.
2. Never print large files or long command outputs into the conversation; use grep, head, tail -n 50 and quiet flags.
3. CI: run all gates locally (cross-compiled) before pushing; push at most 2–3 times per phase; wait with a single `gh run watch <id> --exit-status` — no polling loops; on failure read only `gh run view <id> --log-failed | tail -n 80`.
4. Goldens: regenerate once near the end of the phase, not after every change.
5. Fuzzing and long measurements: start them in the background, check once at the end.
6. ADRs ≤ 30 lines; the phase report ≤ 250 lines — tables, no repeated prose, never restate the prompt.
7. Fix things in one pass where possible; batch commits.
8. If something is ambiguous, choose the simplest option that meets the spec and note it in one line in the report.
9. If the phase cannot be finished, stop at a green, merged state and list exactly what is left (as in Phase 003) — never leave main red.
```
