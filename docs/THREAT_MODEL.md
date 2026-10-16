# Threat model (skeleton, S-TEST-04)

Status: **skeleton created in Phase 001**. Phase 012 completes it. Each entry point gets a STRIDE
table (Spoofing, Tampering, Repudiation, Information disclosure, Denial of service, Elevation of
privilege). The mitigation column names the requirement IDs from `docs/SECURITY.md`. Its status is
`planned` until the code exists and a test covers it.

Assets: user documents and files next to them, user settings, the integrity of the user's session
(no code execution, no network use), and the availability of the viewer.

Trust boundary: everything that comes from outside the process is untrusted. That covers document
bytes, file names, dropped data, clipboard data, the command line, IPC messages and INI files.

## 1. Document content (Markdown, HTML subset, math, Mermaid, images)

| STRIDE | Threat | Mitigations (planned) | Status |
|---|---|---|---|
| S | Link text that disguises its real target | S-LINK-01…04, R-LINK-05 | planned (003) |
| T | Document edits files through links or paths | S-FILE-*, S-LINK-* | planned (003/005) |
| R | — (single-user local app) | n/a | — |
| I | Remote resources leak that the document was opened | S-NET-01/02, S-RES-* | S-NET-03 binary check active (001) |
| D | Parser, layout, regex or Mermaid blow-ups | S-PARSE-01…08, S-TEST-01 fuzzing | planned (003/009/010/012) |
| E | Script or HTML execution; running links | S-HTML-*, S-EXEC-*, S-LINK-* | planned (003) |

## 2. Dropped files

| STRIDE | Threat | Mitigations (planned) | Status |
|---|---|---|---|
| S | Shortcuts (`.lnk`) or reparse points that disguise the target | S-FILE-* | planned (002/005) |
| T | — | — | — |
| I | UNC or WebDAV paths that cause network access | S-FILE-*, S-NET-02 | planned |
| D | Huge or special files (devices, pipes) | S-FILE-*, S-RES-* | planned |
| E | A drop from a lower-integrity process (UIPI) | S-UI-* | planned |

## 3. Clipboard

| STRIDE | Threat | Mitigations (planned) | Status |
|---|---|---|---|
| T | Malicious HTML or RTF paste into the editor | S-HTML-*, R-EDIT-* | planned (006/008) |
| D | Huge clipboard payloads | S-RES-* | planned |

## 4. Command line

| STRIDE | Threat | Mitigations | Status |
|---|---|---|---|
| T | Paths and flags crafted by a caller | strict parser; unknown flag → exit 2 | **partial (001: `--help`/`--version`/usage error)** |
| E | Elevated launch | S-PROC-06 manifest `asInvoker` | manifest done (001); banner planned (004/005) |
| — | DLL planting / search order | S-PROC-01/02 | **implemented (001)** |

## 5. IPC (single instance, Phase 005)

| STRIDE | Threat | Mitigations (planned) | Status |
|---|---|---|---|
| S | Another user or process impersonates the instance | S-IPC-01…04 | planned (005) |
| T/D | Malformed or oversized payloads | S-IPC-*, fuzzing (S-TEST-01) | planned |

## 6. Config files (`settings.ini`, `state.ini`, `custom-theme.ini`)

| STRIDE | Threat | Mitigations (planned) | Status |
|---|---|---|---|
| T | Hostile values (paths, colors, sizes) | S-PRIV-*, INI fuzzing | planned (004) |
| I | Leaking recent-file history | S-PRIV-* | planned |
| D | Huge or invalid INI | bounded parser | planned |

## Process-wide mitigations already in place (Phase 001)

The following are implemented in `crates/yamdv-platform-win/src/hardening.rs`. Each step's failure
is logged.

- S-PROC-01 `SetDefaultDllDirectories`
- S-PROC-02 `SetDllDirectoryW("")` and `SetSearchPathMode`
- S-PROC-03 heap termination on corruption
- S-PROC-04 ImageLoad and StrictHandle policies (DynamicCode deferred, ADR-0001)

S-PROC-05 build hardening is verified by `cargo xtask check-imports` in CI.
