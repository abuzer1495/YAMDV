# ADR-0018: Printing runs in a worker process with reduced hardening

- Status: accepted (Phase V1-D, prompt 008 §3; owner addenda A-5/A-6)

Context: on CI run 39043473274 `yamdv --export-pdf` on "Microsoft Print to PDF" died with 0xC000041D (exception inside a kernel→user callback). Printer drivers run inside the printing process; under `RaiseExceptionOnInvalidHandleReference` (S-PROC strict handle check) or ACG a driver bug or code generation becomes a process crash.

Decision: `--export-pdf` and the internal `--print-job <file>` apply `hardening::apply_for_print()` = every step except StrictHandleCheck and ACG (DLL search, image-load policy and heap termination stay). The GUI keeps the full hardening and never loads a printer driver: Print… and Export PDF… write a job file (`%TEMP%\yamdv-print-<pid>-<n>.job`: settings, printer, DEVMODE hex, range, output, then the source) and wait for `yamdv --print-job`, which deletes nothing but prints; the GUI deletes the file afterwards.

Alternatives: printing in the dialog host (mixes roles); dropping the mitigations for the GUI (weakens the main process).

Consequences: the job file briefly holds the document text in the user's temp folder. Root cause unverified on Windows (A-6: no CI until 2026-11-01); owner QA covers it.
