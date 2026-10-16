# ADR-0013: Out-of-process dialog host

- Status: accepted (Phase 003, F-09; P-10, P-11, P-14, P-15, P-20)

## Context
In Phase 002, `IFileOpenDialog` ran inside the main process. The shell loaded about 40 modules plus shell-extension state that never went away again. The M6 budget was only met with a working-set trim right after the dialog, and that trim hid the real cost (reviewer item 4).

## Decision
- **Launch:** `Ctrl+O` starts a second instance of our own executable. The path comes from `GetModuleFileNameW(NULL)` and is passed as `lpApplicationName` to `CreateProcessW`, with no handle inheritance and the command line `--dialog-host open <owner-hwnd> <nonce>`. The nonce is 64 random bits from `BCryptGenRandom` (system-preferred RNG).
- **Host:**
  - The host runs the same startup hardening as the main process (S-PROC-01…04), *including* ACG. Phase 002 verified on CI that `IFileOpenDialog` works under ACG (M6). If a future Windows build or shell extension breaks it, the host may drop ACG: it parses no documents and lives for one dialog only.
  - It refuses with exit code 2 unless the owner window has class `YAMDV.Window` and belongs to a process of the same user (token SIDs compared with `EqualSid`).
  - It shows one `IFileOpenDialog` owned by that window (filters per R-OPEN-03, multi-select). It sends the result with `SendMessageTimeoutW(WM_COPYDATA, SMTO_ABORTIFHUNG, 10 s)` from a message-only window, then exits.
  - Exit codes: 0 for OK or cancel, 1 when the dialog could not be shown.
- **Payload** (`yamdv_core::ipc`):
  - Fields: magic `YMDH` (also in `dwData`), version 1, kind, nonce, status (cancel / paths / error), then length-prefixed UTF-16 paths.
  - Limits: 4 MiB, 1000 paths, 32,767 units per path.
  - The decoder is strict (exact length, valid UTF-16) and has unit tests.
- **Main window:**
  - It calls `EnableWindow(FALSE)` while it waits.
  - It accepts `WM_COPYDATA` only while it waits for a host it started (one at a time per UI thread), and only when all of these hold:
    - the message was sent to the owner window;
    - the sender window belongs to the host's PID (`GetWindowThreadProcessId`);
    - `dwData` is the magic and the payload decodes;
    - the nonce matches.
  - Returned paths go through the same intake as user-opened paths.
  - The message loop waits on the host's process handle (`MsgWaitForMultipleObjectsEx`, `MWMO_INPUTAVAILABLE`). If the host exits without a valid answer, the window is re-enabled and a toast says "The file dialog could not be opened". The same toast appears when the launch fails or the host reports status error.
- **Removed:** the in-process dialog path and `sys::trim_working_set`. No trim is performed anywhere in Phase 003.
- **Future reuse:** save dialogs (Phase 011) and the print dialog will reuse the host with new `kind` values. The print dialog returns only serializable data (printer name, DEVMODE bytes, page range, copies), never a handle; the main process creates its printer DC itself.

## Alternatives
- Keep the in-process dialog and trim afterwards: rejected by the reviewer, because it fakes the metric.
- A persistent broker process: it would cost memory all the time, so it was rejected.
- Named-pipe IPC: more code and a larger attack surface than `WM_COPYDATA` between two windows of the same user.

## Consequences
- The main process module count is unchanged after the dialog. memprobe records it (`modules_before_dialog` vs `module_count`) and the M6 gate fails if it grows.
- T-DLG is now process start plus the dialog, measured by memprobe from posting the open command to the host dialog becoming visible (soft budget 400 ms). See `docs/reports/003_REPORT.md`.
- `WM_COPYDATA` can be sent by any process of the same desktop. That is why the PID and nonce checks exist, and why returned paths get no extra trust.
