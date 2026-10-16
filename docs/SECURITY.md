# APPENDIX C — Security Specification (`docs/SECURITY.md`)

**Threat model in one line:** the adversary controls the *content* of Markdown files (and the files next to them) that the user opens — e.g. a README in a cloned repository, an e-mail attachment, an AI-generated file — and may also control other files the user drags in or pastes. Other processes of the *same* user are semi-trusted; other users and the network are untrusted. Assets to protect: the user's files and their integrity, the user's credentials (including NTLM hashes), the system's integrity, and the user's privacy.

## C.1 Network (S-NET)
- **S-NET-01 MUST** No networking code in the product: no socket, HTTP, DNS, TLS or WebSocket APIs or crates (bans in S-SUP-03).
- **S-NET-02 MUST** Nothing referenced by a document is ever fetched from a network: remote images, fonts, includes, iframes → placeholders. Never pass document-derived strings to APIs that can fetch (URL monikers, `URLDownloadToFile*`, `SHCreateStreamOnFileEx` with URLs, WIC decoders created from URLs, `LoadImage` on UNC paths).
- **S-NET-03 MUST** Automated binary check (`cargo xtask check-imports`): the import and delay-import tables of `yamdv.exe` MUST NOT contain `ws2_32.dll`, `wsock32.dll`, `mswsock.dll`, `winhttp.dll`, `wininet.dll`, `urlmon.dll`, `dnsapi.dll`, `iphlpapi.dll`, `webio.dll`, `httpapi.dll`, `netapi32.dll`, `mpr.dll`, `rasapi32.dll`, `wlanapi.dll`, `winhttpcom.dll`, nor functions named `URLDownloadToFile*`, `URLOpenStream*`, `Internet*`, `WinHttp*`, `socket`, `WSA*`, `connect`, `getaddrinfo`, `GetAddrInfo*`. (System DLLs loaded *at runtime* by Windows itself — e.g. shell extensions inside the file dialog — are outside our control; `memprobe` lists loaded modules so they are visible.)
- **S-NET-04 MUST** The only outward hand-offs, both user-initiated and leaving our process: (a) a validated `http`/`https`/`mailto` URL passed to the default handler via `ShellExecuteExW` (R-LINK-04), (b) the hard-coded `ms-settings:defaultapps…` URI (R-SYS-02).
- **S-NET-05 MUST** Verification step in every release QA: with an outbound-block firewall rule for `yamdv.exe` nothing changes, and `netstat -ano` / Resource Monitor show no sockets owned by the process.

## C.2 Process hardening (S-PROC)
- **S-PROC-01 MUST** First statements in `main`: `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)`; any `LoadLibraryExW` uses `LOAD_LIBRARY_SEARCH_SYSTEM32` with a bare system DLL name. Never load DLLs from the exe folder, the current directory or a document folder (DLL planting is the #1 risk for a portable exe in `Downloads`).
- **S-PROC-02 MUST** `SetDllDirectoryW("")`, `SetSearchPathMode(BASE_SEARCH_PATH_ENABLE_SAFE_SEARCHMODE | BASE_SEARCH_PATH_PERMANENT)`.
- **S-PROC-03 MUST** `HeapSetInformation(NULL, HeapEnableTerminationOnCorruption, …)`.
- **S-PROC-04 MUST** `SetProcessMitigationPolicy`: `ProcessImageLoadPolicy` (NoRemoteImages, NoLowMandatoryLabelImages, PreferSystem32Images), `ProcessStrictHandleCheckPolicy`. **SHOULD** `ProcessDynamicCodePolicy` (ProhibitDynamicCode) if DirectWrite, the renderer, WIC, IMEs, file dialogs and printing all keep working (test and record in an ADR). **MUST NOT** enable `ProcessExtensionPointDisablePolicy` (breaks IMEs/accessibility), `ProcessSignaturePolicy` (breaks printer drivers/IMEs), `ProcessFontDisablePolicy` (breaks per-user fonts).
- **S-PROC-05 MUST** Build hardening: ASLR + high-entropy VA, DEP/NX, Control Flow Guard, CET shadow-stack compatible, static CRT, no RWX sections, `overflow-checks = true`, `panic = "abort"`. Verified by xtask (PE header flags) in CI.
- **S-PROC-06 MUST** Manifest `asInvoker`; never request elevation. If running elevated, show a banner "Running as administrator is not recommended".
- **S-PROC-07 MUST** Child processes only for: the external editor (user action); the **dialog host** — a second instance of our own `yamdv.exe`, started by absolute path (`GetModuleFileNameW`) with the internal flag `--dialog-host`, which shows exactly one native file-open/save or print dialog and exits; and the self-install/uninstall helper — nothing else. Showing a file in Explorer uses `SHOpenFolderAndSelectItems` (no process launch by us).
- **S-PROC-08 MUST** Security-relevant behavior never depends on environment variables or `PATH` (no `MD_EDITOR`-style variables; executables are resolved to absolute paths from known locations / `App Paths`).

## C.3 File system (S-FILE)
- **S-FILE-01 MUST** Writes happen only to: the open document (explicit save / autosave **in edit modes only**), the assets folder (S-FILE-05), export targets chosen in a Save dialog or via CLI `-o`, the YAMDV config/state/log/recovery folders, and the install folder during `--install`. Read mode never writes the document, with one opt-in exception: when `reading.task_toggle_in_read_mode` is on, clicking a task checkbox performs a single edit of that `[ ]`/`[x]` marker through the editor's save path (atomic save and conflict detection, available from Phase 006).
- **S-FILE-02 MUST** Reads happen only from: the opened document, resources per S-RES, YAMDV config files, OS-provided fonts/codecs, and directory listings (names/attributes only) for the Files tab and wikilink resolution.
- **S-FILE-03 MUST** Atomic save: write a hidden temp file in the same folder (`.~<name>.yamdv-<random>.tmp`, `CREATE_NEW`), `FlushFileBuffers`, then `ReplaceFileW` (keeps ACLs/attributes), falling back to `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)`. On any failure the original stays intact, the temp file is removed, the buffer stays dirty and an error is shown. Never clear the read-only attribute. If the document path is a symlink, write the final target (`GetFinalPathNameByHandleW`), never replace the link itself.
- **S-FILE-04 MUST** Path validation for every path that comes from a document, IPC, CLI or config: normalize with `GetFullPathNameW`; reject device namespace paths (`\\.\`, `\\?\GLOBALROOT`), reserved device names (`CON PRN AUX NUL COM1–9 LPT1–9`, with or without extension, any case), alternate data streams (`:` beyond the drive letter), and embedded NULs. Long paths are handled internally with `\\?\`.
- **S-FILE-05 MUST** Assets sandbox: the target folder `<doc_dir>/<assets_dir>/<doc_stem>/` must, after creation and `GetFinalPathNameByHandleW`, still be inside `<doc_dir>` (rejects `..`, absolute values, junction/symlink escapes). File names are sanitized (remove `<>:"/\|?*` and control characters, trim trailing dots/spaces, avoid reserved names, max 120 UTF-16 units) and created with `CREATE_NEW` (never overwrite).
- **S-FILE-06 MUST** Temporary files only in the document folder (atomic save) or `%TEMP%\YAMDV-<random>\` (export staging), created with `CREATE_NEW`, deleted on success, on failure and at exit.
- **S-FILE-07 MUST** Config/state parsing is defensive: ≤ 4 MB per file, ≤ 64 KB per line, unknown keys ignored, values clamped to their ranges, nothing ever executed; corrupt files renamed to `*.corrupt-<timestamp>` and defaults used.
- **S-FILE-08 MUST** File watching covers only the open document's folder (non-recursive) and the config folder; buffer overflow → full reload of the document.
- **S-FILE-09 MUST** `.lnk`, `.url` and other shell-link files are never followed or resolved automatically.
- **S-FILE-10 MUST** Data integrity: never normalize line endings, never strip/add a BOM, never re-format untouched blocks, never write in read mode (also R-EDIT-01/02).

## C.4 Resources referenced by documents (S-RES)
- **S-RES-01 MUST** Allowed image sources: relative paths, absolute local paths on local fixed/removable drives, `file:///X:/…` (local), and `data:image/(png|jpeg|gif|bmp|webp|x-icon|tiff);base64,…`.
- **S-RES-02 MUST** **Network-location blocking (prevents NTLM credential leaks):** block UNC paths (`\\server\share\…`, `//server/share/…`, `\\?\UNC\…`), `file://host/…`, WebDAV forms (`\\host@SSL\…`, `\\host@port\…`), and paths on mapped network drives (`GetDriveTypeW == DRIVE_REMOTE`) — **before** any `CreateFileW`/WIC call. Exception: if the user opened the document itself from a network location, resources on the **same server and share** are allowed. Additionally open local resources with `FILE_FLAG_OPEN_REPARSE_POINT` first; if the final component is a symlink/junction whose target is a network path, block it.
- **S-RES-03 MUST** Never fetch remote (`http`, `https`, `ftp`, …) resources; show the offline placeholder.
- **S-RES-04 MUST** Limits: image file ≤ 50 MB; `data:` URI ≤ 10 MB decoded; dimensions ≤ 16 384 × 16 384 and ≤ 64 megapixels (checked from the header **before** decoding); animated GIF ≤ 1 000 frames; decoded-image cache within the Appendix D budget.
- **S-RES-05 MUST** Referenced files are only ever *decoded as images* — never opened with another app automatically. Exports embed only files that decoded successfully as images (so `![](C:/Users/me/.ssh/id_rsa)` can never be exfiltrated into an exported HTML/PDF).
- **S-RES-06 MUST** Fonts come only from the system/user font collections via DirectWrite; documents cannot load fonts.

## C.5 Links (S-LINK)
- **S-LINK-01 MUST** External schemes allowlist: `http`, `https`, `mailto` (ASCII-case-insensitive). Every other scheme is inert (R-LINK-06).
- **S-LINK-02 MUST** Before hand-off: the URL is parsed and re-serialized; rejected if it contains control characters, unescaped spaces or quotes, or exceeds 2 048 characters. The *Ask* popover shows the real destination; internationalized host names are shown **with their punycode form** (homograph protection); a credential part (`user:pass@`) is highlighted as suspicious.
- **S-LINK-03 MUST** Local link targets: Markdown family → open in YAMDV; folders → `SHOpenFolderAndSelectItems` after confirmation; "open with default app" only after confirmation and only for this extension allowlist: `png jpg jpeg gif bmp webp tif tiff ico svg pdf txt log csv tsv json yaml yml toml xml docx xlsx pptx odt ods odp mp3 wav ogg flac mp4 webm mkv mov` — everything else (including `exe com scr pif bat cmd ps1 psm1 vbs vbe js jse wsf wsh hta cpl msc msi msp msu reg lnk url scf inf jar appref-ms application settingcontent-ms library-ms search-ms searchconnector-ms theme themepack diagcab chm hlp iso img vhd vhdx dll sys docm xlsm pptm` and extension-less files) is **blocked** (R-LINK-05). Confirmation dialogs focus **Cancel** by default.
- **S-LINK-04 MUST** Mark-of-the-Web: if the document has a `Zone.Identifier` stream with `ZoneId` ≥ 3, show a banner "This file came from the internet" and always *Ask* before any external link or local file launch, regardless of settings.

## C.6 HTML inside Markdown (S-HTML)
- **S-HTML-01 MUST** There is no HTML engine. Raw HTML is tokenized by a small bounded tokenizer and mapped to the allowlist (R-MD-14). Unknown elements are dropped but their text content is kept; these containers are dropped **with** their content: `script style iframe frame frameset object embed applet form input button select textarea template noscript link meta base title head svg math` (inline SVG/MathML → placeholder).
- **S-HTML-02 MUST** Only allowlisted attributes survive; all `on*` attributes and `javascript:`/`vbscript:`/`data:` (except image data URIs in `img src`) values are removed; `style` accepts only `color`, `background-color`, `text-align` with a strict value grammar (hex, `rgb()`, named colors; `left|center|right|justify`); every `href`/`src` goes through S-LINK / S-RES.
- **S-HTML-03 MUST** Limits: nesting depth 64, 100 000 tags per document, 8 KB per attribute value.

## C.7 Execution (S-EXEC)
- **S-EXEC-01 MUST** No eval, scripting, macros, plugins, or dynamic code loading of any kind.
- **S-EXEC-02 MUST** Launching local files: only per S-LINK-03.
- **S-EXEC-03 MUST** Processes are started only with `CreateProcessW` with `lpApplicationName` = absolute path to an existing `.exe` (verified not to be a network path or a reparse point to one), arguments quoted per the MSVC/`CommandLineToArgvW` rules (unit-tested round-trip). Never `cmd.exe`, PowerShell, `.bat`/`.cmd`/`.ps1` targets (the "BatBadBut" class of injection).
- **S-EXEC-04 MUST** `ShellExecuteExW` is used only for: validated external URLs, the constant `ms-settings:` URI, and allowlisted local files after confirmation. Never pass `SEE_MASK_NOZONECHECKS`.

## C.8 Single instance / IPC (S-IPC)
- **S-IPC-01 MUST** Single instance via a named mutex `Local\YAMDV-<hash of user SID>`; arguments forwarded with `WM_COPYDATA` to the primary window, `dwData` = magic constant, payload = versioned UTF-16 structure (magic, version, count, paths/flags), ≤ 64 KB.
- **S-IPC-02 MUST** The receiver verifies the sender's process runs as the same user (token SID comparison) and ignores anything else.
- **S-IPC-03 MUST** Forwarded paths get exactly the validation of user-opened paths; IPC can only *open documents* — never export, install, register, or change settings.
- **S-IPC-04 MUST** No named pipes, sockets, shared memory sections, COM servers, DDE, or listening endpoints of any kind.
- **S-IPC-05 MUST** Dialog-host results return through `WM_COPYDATA` with their own magic value, a versioned payload and the per-launch nonce. The main window accepts them only while it waits for a host it started, only from that host's process ID (checked with `GetWindowThreadProcessId` on the sender window), and treats returned paths exactly like user-opened paths. The host refuses to run if the owner window does not belong to a process of the same user, and it never does anything other than show one dialog.

## C.9 Registry (S-REG)
- **S-REG-01 MUST** Writes only to: `HKCU\Software\Classes\YAMDV.Markdown`, `HKCU\Software\Classes\<ext>\OpenWithProgids`, `HKCU\Software\Classes\Applications\yamdv.exe`, `HKCU\Software\Classes\SystemFileAssociations\<ext>\shell\YAMDV`, `HKCU\Software\YAMDV\Capabilities`, `HKCU\Software\RegisteredApplications` (value `YAMDV`), `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\YAMDV` (install only). Never HKLM, never `UserChoice`, never `Explorer\FileExts`, never policy keys.
- **S-REG-02 MUST** Every written key/value is listed in one place in code; `--unregister` / `--uninstall` remove exactly those; registration is idempotent.
- **S-REG-03 MUST** Registration happens **only on explicit user action** (Settings button, context-menu toggle, `--register`, `--install`) — never on a normal launch.
- **S-REG-04 MUST** Registry reads are limited to: theme/accessibility settings, `App Paths`, editor install keys, association queries.

## C.10 Parser robustness / DoS (S-PARSE)
- **S-PARSE-01 MUST** No panic, hang, or unbounded allocation on any input; all recursion has explicit depth limits (visual nesting of quotes/lists capped at 64 levels — deeper content is laid out at level 64).
- **S-PARSE-02 MUST** Limits: file 256 MB hard / 20 MB soft prompt; a single paragraph > 1 MB uses a plain-text fast path; tables ≤ 1 000 columns and ≤ 100 000 cells (else shown as text); TOC ≤ 10 000 headings; ≤ 10 000 footnotes; ≤ 100 000 link reference definitions.
- **S-PARSE-03 MUST** Time budgets: any single block whose layout exceeds 200 ms is replaced by a "Content too complex" placeholder with [Render anyway]; the UI thread never blocks > 2 s.
- **S-PARSE-04 MUST** Only linear-time algorithms on user-controlled input; no backtracking regular expressions.
- **S-PARSE-05 MUST** Math: ≤ 10 000 chars per formula, depth ≤ 64, macro expansion ≤ 10 000 steps and ≤ 100 000 output tokens per formula, ≤ 5 000 rendered formulas per document (the rest shown as source).
- **S-PARSE-06 MUST** Mermaid: source ≤ 100 KB per diagram; soft limit 500 nodes / 1 500 edges (placeholder + [Render anyway]); hard limit 2 000 nodes / 6 000 edges; bounded layout iterations; ≤ 100 rendered diagrams per document.
- **S-PARSE-07 MUST** Checked or saturating arithmetic for sizes/offsets from input; `overflow-checks` stay on in release.
- **S-PARSE-08 MUST** Invalid encodings → U+FFFD for viewing; editing disabled when decoding was lossy (R-EDIT-12).

## C.11 Privacy (S-PRIV)
- **S-PRIV-01 MUST** No telemetry, analytics, usage counters, device IDs or crash upload.
- **S-PRIV-02 MUST** Logging off by default; when enabled: local only, rotated (≤ 1 MB × 3 files), never containing document text; file paths only when "diagnostic log" is enabled.
- **S-PRIV-03 MUST** Crash notes contain no document content.
- **S-PRIV-04 MUST** Settings offer "Clear history", "Clear favorites" and "Forget reading positions".
- **S-PRIV-05 MUST** The clipboard is read only on an explicit paste.
- **S-PRIV-06 MUST** Exports contain no personal metadata (PDF `Producer` = "YAMDV <version>", `Title` = first H1 or file name; no author/user name).

## C.12 Supply chain (S-SUP)
- **S-SUP-01 MUST** `Cargo.lock` committed; `cargo deny check` (advisories, bans, licenses, sources) in CI; builds use `--locked`.
- **S-SUP-02 MUST** Each dependency has an ADR (H5); reports list dependency changes with `cargo tree -e normal` deltas.
- **S-SUP-03 MUST** Banned crates (in `deny.toml`): `reqwest hyper hyper-util h2 h3 ureq isahc curl curl-sys surf attohttpc minreq tokio async-std smol mio socket2 native-tls openssl openssl-sys rustls webpki webpki-roots quinn tungstenite tokio-tungstenite websocket lettre trust-dns-resolver hickory-resolver wry tao tauri webview2 webview2-com winit egui eframe iced slint druid gtk glib fltk wgpu glutin sdl2 skia-safe rquickjs quick-js boa_engine v8 deno_core mlua rlua pyo3 libloading`.
- **S-SUP-04 MUST** License allowlist: MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, Unicode-DFS-2016, BSL-1.0, CC0-1.0. Everything else denied unless an ADR approves it.
- **S-SUP-05 MUST** No network access in build scripts; resource embedding (manifest, icons, version info) via a pure-Rust `.res` writer or a crate approved by ADR.
- **S-SUP-06 MUST** GitHub Actions pinned by full commit SHA, `permissions: contents: read`, no secrets, only well-known actions (`actions/checkout`, `actions/cache`, `actions/upload-artifact`, `EmbarkStudios/cargo-deny-action`, a toolchain installer).

## C.13 Export safety (S-EXP)
- **S-EXP-01 MUST** Exports are written atomically, only to the user-chosen path.
- **S-EXP-02 MUST** HTML export: `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'sha256-<hash of our inline script>'; base-uri 'none'; form-action 'none'">`; all text escaped; images only as `data:` URIs of decoded images; external links get `rel="noopener noreferrer"`; no external resource of any kind.
- **S-EXP-03 MUST** PDF: no JavaScript, launch actions, embedded files or forms; only URI annotations for `http`/`https`/`mailto` and internal GoTo links.
- **S-EXP-04 MUST** CSV export guards against formula injection: cells starting with `= + - @` or a tab/CR are prefixed with `'` (setting `export.csv_formula_guard`, default on).

## C.14 UI-level safety (S-UI)
- **S-UI-01 MUST** The real destination of every link is visible before it is followed (status tooltip + Ask popover).
- **S-UI-02 MUST** Dangerous confirmations default to Cancel; nothing is auto-accepted.
- **S-UI-03 MUST** Banners for: Mark-of-the-Web documents, elevated process, decoding fallback.

## C.15 Security verification (S-TEST)
- **S-TEST-01 MUST** Fuzz targets (`cargo-fuzz`): Markdown → block tree → layout (mock shaper), inline extensions, HTML sanitizer, math parser + layout, each Mermaid parser + layout, INI parser, encoding detection, IPC payload decoder. Each target runs ≥ 10 minutes in the phase that introduces or changes it (locally or in a manual CI workflow); small seed corpora are committed.
- **S-TEST-02 MUST** Security unit-test matrices: link policy (scheme × target type × MotW × setting), path validator (UNC variants, WebDAV forms, device names, ADS, traversal, reparse points), assets sandbox, HTML sanitizer (a list of ≥ 50 known XSS vectors must yield no executable output in exported HTML), CSV guard, argument quoting round-trip, IPC validation.
- **S-TEST-03 MUST** CI binary checks: forbidden imports (S-NET-03), PE `DllCharacteristics` contain `DYNAMIC_BASE`, `HIGH_ENTROPY_VA`, `NX_COMPAT`, `GUARD_CF`; CET compatibility flag present; no section is both writable and executable.
- **S-TEST-04 MUST** `docs/THREAT_MODEL.md` (STRIDE per entry point: document content, dropped files, clipboard, CLI, IPC, config files) is created as a skeleton in Phase 001 and completed in Phase 012.
