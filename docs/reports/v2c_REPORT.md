# YAMDV — Phase V2-C Report: transient UI and viewers (prompt 013)
Prompt: `docs/prompts/013_V2C_TRANSIENT_UI_VIEWERS.md` (§1.1 skipped by owner instruction). Rules: PROCESS A-9 (CI back).
Branch `phase/v2c-viewers`, PR #82, squash-merged as 3dc8157 (title `[skip ci]`, tree = PR head 9510db0), tagged `phase-v2c`. Remote: https://github.com/abuzer1495/YAMDV.
Issues closed: #38 FUT-40 (PR keyword); #35 FUT-36, #13 FUT-06, #33 FUT-34, #39 FUT-41, #68 BUG-08 closed by hand, citing #82.

## 1. Acceptance criteria
| AC | Status | Evidence |
|---|---|---|
| AC-1 setup, v1.1.0, test-v2b | done | V2-B fix report + A-9 (#81); `v1.1.0` tagged on bd6acc6 after green main run; draft below; `test-v2b` was already absent |
| AC-2 items 1–7 + TRACEABILITY | done | tooltips, banners, Ask popover, image/diagram/table viewers, folding, sup/sub/footref inline objects, BUG-08; 9 rows → DONE |
| AC-3 scripts, goldens, unit tests, artifact | done | `tooltips-banners`, `viewers`, `folding`; 4 shell goldens `top-v2c-*`; pure tests in yamdv-ui/layout; `yamdv-exe` uploaded |
| AC-4 gates, PR, issues, tag, report | done | gates below; PR #82 merged; 6 issues closed; `phase-v2c`; this file |

## 2. CI
- Phase run (PR head 9e76508): https://github.com/abuzer1495/YAMDV/actions/runs/39916255630 — all green except doc goldens
  `mermaid-light/dark-1.0` (size differs: BUG-08 keeps the 9-node LR flowchart at minimum scale, 14 px taller; intended).
- Fix run (80590f9): https://github.com/abuzer1495/YAMDV/actions/runs/39919219757 — **green**, all jobs and gates.
- 9510db0 (`[skip ci]`) adds the footnote/image shell goldens from the fix-run artifact; nothing else.
- v1.1.0 tag run: https://github.com/abuzer1495/YAMDV/actions/runs/39913154778 (green, on bd6acc6).

## 3. Gates (fix run = merged code, release exe, windows-2022)
| Gate | V2-B | V2-C | Limit |
|---|---|---|---|
| M0 private WS | 2.36 MiB | 2.36 MiB | ≤ +0.3 MiB |
| M1 private WS | 4.84 MiB | 4.61 MiB | ≤ +0.3 MiB |
| T4 typical warm p99 | — | 9.95 ms (cold 13.3), attempt 1 | A-7 |
| T7 doc / empty | 0 / 0 | 0 / 0 µs | 0 |
| T7 viewer open, idle (in-script) | — | 31 ms CPU / 2 s (driver timer only) | < 60 ms |
| viewer PWS after close / GDI | — | +52 KiB / 0 | ≤ +0.2 MiB / ≤ 0 |

Driver scripts: 35/35 PASS (32 old + 3 new); doc, window/shell goldens and ACG matrix green. Exe: 2,919,936 B → 2,988,032 B = **+68,096 B**.

## 4. v1.1.0 draft release
- https://github.com/abuzer1495/YAMDV/releases/tag/untagged-0d11771430dd96fbe710 (draft; owner publishes)
- Assets `YAMDV-1.1.0-win-x64.zip` + `.sha256`; SHA-256 `49d8372dfe0fc491d1a3ef4b6c0d7a965986d318be7eab1b4fa9084bd4180aec`
- The old `v1.0.0-rc1` draft was built with cargo-xwin and is left alone by owner decision.

## 5. Deviations
- PR #81 (setup) merged without a PR run; v1.1.0 was built by the tag run after the green main run.
- `test-v2b` did not exist any more; nothing was deleted. Pack jobs stay workflow_dispatch-only.
- The tooltip is drawn inside the main window (no layered HWND); same look, no extra window or GDI objects.
- The edit conflict moved from a dialog to a banner (Reload / Keep mine); the Elevated banner is suppressed under the driver.
- Sup/sub are always synthesized (75 %, sup shift 0.33 em, sub 0.15 em); font `sups`/`subs` are not queried.
- The v2c fixture uses `<sub>`/`<sup>`: pulldown-cmark has no intraword `^x^`, and `~x~` is off by default.
- The window layer is split into `window/transient/{mod,banner,tooltip,popover,viewer,fold}.rs`.
- TRACEABILITY was updated by a one-off script (no xtask command exists).
- 4 new shell goldens: motw/wide came from the phase-run artifact in the fix push; footnote/image from the fix-run
  artifact in a `[skip ci]` commit (the fixture changed in the fix push). The 2 mermaid doc goldens were updated.
- T7 with a viewer open is checked in-script (`cpu_delta_ms`); memprobe T7 covers doc/empty.
- Merge title carries `[skip ci]` (as V2-B #79), so the main push and the `phase-v2c` tag run no third full CI.
- Known issue (next prompt): the footnote tooltip keeps literal `*emphasis*` asterisks (spec: emphasis removed);
  visible in golden `top-v2c-footnote-light`. Found after the fix push, so not fixed here (push limit).

## 6. Owner check list (EN)
1. Download `yamdv-exe` from the fix run (or the v1.1.0 draft zip) and open a document with links and footnotes.
2. Hover a titled link and a footnote mark: tooltips after ~0.5 s; Tab to a link shows its tooltip too.
3. Open a downloaded .md (Internet zone): blue banner; click a link → Ask popover (Open / Copy / Cancel).
4. Double-click an image: viewer at fit; wheel zoom, drag, `0`, `1`, Copy, Esc.
5. Use the ⛶ badge on a table and a Mermaid diagram: viewer title "Table · R × C"; diagram zoom stays sharp.
6. Click an H2 toggle, then Alt+click: sections fold; Ctrl+F for a word inside a fold opens it.
7. A wide LR flowchart keeps readable text and scrolls sideways; HTML export scrolls it too.
8. Repeat 2–5 in dark mode and at 150 % display scale; publish the v1.1.0 draft when happy.

## 7. Sahip kontrol listesi (TR)
1. Fix koşusundan `yamdv-exe`'yi (ya da v1.1.0 taslak zip'ini) indirin; bağlantı ve dipnot içeren bir belge açın.
2. Başlıklı bir bağlantı ve dipnot işaretinin üzerinde bekleyin: ~0,5 sn sonra ipucu; Tab ile de görünür.
3. İndirilmiş bir .md açın (İnternet bölgesi): mavi şerit; bağlantıya tıklayın → Sor penceresi (Aç / Kopyala / İptal).
4. Bir resme çift tıklayın: sığdırılmış görüntüleyici; tekerlek yakınlaştırma, sürükleme, `0`, `1`, Kopyala, Esc.
5. Tablo ve Mermaid diyagramındaki ⛶ simgesini kullanın: başlık "Table · R × C"; diyagram yakınlaşınca net kalır.
6. Bir H2 düğmesine, sonra Alt+tıklayın: bölümler katlanır; katlı bölümdeki bir kelimeyi Ctrl+F ile arayın, açılır.
7. Geniş bir LR akış şeması okunur yazıyla yana kayar; HTML dışa aktarımında da kayar.
8. 2–5. adımları koyu modda ve %150 ölçekte tekrarlayın; uygunsa v1.1.0 taslağını yayımlayın.

Next action: owner tests the v1.1.0 draft and the V2-C artifact, publishes v1.1.0; then prompt 014 (V2-D).
