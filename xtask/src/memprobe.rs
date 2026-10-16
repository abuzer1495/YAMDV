//! `cargo xtask memprobe`: launch an app, wait for its idle main window, measure memory per
//! Appendix D §D.3, close it with `WM_CLOSE`, verify exit. The measurement itself is Windows-only;
//! argument parsing, statistics and the JSON schema are platform-independent and unit-tested.

use serde_json::{Value, json};

/// JSON schema identifier written into every memprobe output.
pub const SCHEMA: &str = "yamdv.memprobe/2";

/// Parsed command-line options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Executable to launch.
    pub exe: String,
    /// Optional document passed as the first argument.
    pub doc: Option<String>,
    /// Extra arguments passed through (`--arg <x>`, repeatable).
    pub args: Vec<String>,
    /// Settle time after the window is idle.
    pub settle_ms: u32,
    /// Number of runs (median is reported).
    pub runs: u32,
    /// JSON output path.
    pub json: Option<String>,
    /// Wait until the window title contains this text before settling (used by spikes).
    pub wait_title: Option<String>,
    /// Save a PNG screenshot of the window (first run only).
    pub screenshot: Option<String>,
    /// Label stored in the JSON.
    pub label: Option<String>,
    /// Set the client area to `W×H` px before settling (F-08).
    pub size: Option<(i32, i32)>,
    /// Open and cancel the Open dialog (`WM_COMMAND` 0x101) before settling (M6).
    pub open_dialog: bool,
    /// Measure CPU time consumed during this many ms after settling (T7).
    pub cpu_ms: u32,
    /// Budgets (CI gate): private working set (MiB), commit ÷ PWS ratio, first paint (ms), idle CPU (ms).
    pub budget: Budget,
}

/// Soft budgets checked by [`check_budget`]; `None` = not gated.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Budget {
    /// Max median private working set, MiB.
    pub pws_mib: Option<f64>,
    /// Max median commit ÷ PWS (M7).
    pub commit_ratio: Option<f64>,
    /// Max median first-paint time, ms.
    pub first_paint_ms: Option<u64>,
    /// Max median idle CPU, ms.
    pub cpu_ms: Option<f64>,
    /// Commit (private usage) budget in MiB (M6, P-10).
    pub commit_mib: Option<f64>,
}

impl Eq for Budget {}

/// Parse memprobe arguments (everything after `memprobe`).
pub fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut it = args.iter();
    let mut o = Options {
        exe: String::new(),
        doc: None,
        args: Vec::new(),
        settle_ms: 3000,
        runs: 3,
        json: None,
        wait_title: None,
        screenshot: None,
        label: None,
        size: None,
        open_dialog: false,
        cpu_ms: 0,
        budget: Budget::default(),
    };
    let num = |v: String, n: &str| v.parse::<f64>().map_err(|_| format!("bad {n}"));
    let next = |it: &mut std::slice::Iter<'_, String>, name: &str| {
        it.next()
            .cloned()
            .ok_or_else(|| format!("{name} needs a value"))
    };
    while let Some(a) = it.next() {
        match a.as_str() {
            "--doc" => o.doc = Some(next(&mut it, a)?),
            "--arg" => o.args.push(next(&mut it, a)?),
            "--settle-ms" => {
                o.settle_ms = next(&mut it, a)?
                    .parse()
                    .map_err(|_| "bad --settle-ms".to_string())?;
            }
            "--runs" => {
                o.runs = next(&mut it, a)?
                    .parse()
                    .map_err(|_| "bad --runs".to_string())?;
                if o.runs == 0 {
                    return Err("--runs must be ≥ 1".into());
                }
            }
            "--json" => o.json = Some(next(&mut it, a)?),
            "--wait-title" => o.wait_title = Some(next(&mut it, a)?),
            "--screenshot" => o.screenshot = Some(next(&mut it, a)?),
            "--label" => o.label = Some(next(&mut it, a)?),
            "--size" => {
                let v = next(&mut it, a)?;
                let (w, h) = v.split_once('x').ok_or("bad --size (WxH)")?;
                let w = w.parse::<i32>().map_err(|_| "bad --size")?;
                let h = h.parse::<i32>().map_err(|_| "bad --size")?;
                if !(200..=8000).contains(&w) || !(200..=8000).contains(&h) {
                    return Err("bad --size".into());
                }
                o.size = Some((w, h));
            }
            "--open-dialog" => o.open_dialog = true,
            "--cpu-ms" => o.cpu_ms = next(&mut it, a)?.parse().map_err(|_| "bad --cpu-ms")?,
            "--budget-pws-mib" => o.budget.pws_mib = Some(num(next(&mut it, a)?, a)?),
            "--budget-commit-ratio" => o.budget.commit_ratio = Some(num(next(&mut it, a)?, a)?),
            "--budget-first-paint-ms" => {
                o.budget.first_paint_ms = Some(
                    next(&mut it, a)?
                        .parse()
                        .map_err(|_| "bad --budget-first-paint-ms")?,
                );
            }
            "--budget-cpu-ms" => o.budget.cpu_ms = Some(num(next(&mut it, a)?, a)?),
            "--budget-commit-mib" => o.budget.commit_mib = Some(num(next(&mut it, a)?, a)?),
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            s if o.exe.is_empty() => o.exe = s.to_string(),
            s => return Err(format!("unexpected argument {s}")),
        }
    }
    if o.exe.is_empty() {
        return Err("usage: memprobe <exe> [--doc <file>] [--arg <x>]… [--settle-ms 3000] [--runs 3] [--json out.json] [--wait-title <text>] [--screenshot out.png] [--label <name>] [--size WxH] [--open-dialog] [--cpu-ms N] [--budget-pws-mib X] [--budget-commit-ratio X] [--budget-first-paint-ms N] [--budget-cpu-ms X] [--budget-commit-mib X]".into());
    }
    Ok(o)
}

/// One measurement of a running process.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sample {
    /// Private working set computed from `QueryWorkingSet` (pages with Shared bit 0 × page size).
    pub private_ws: u64,
    /// `PROCESS_MEMORY_COUNTERS_EX2::PrivateWorkingSetSize` (cross-check), if available.
    pub private_ws_counter: Option<u64>,
    /// `PrivateUsage` (commit).
    pub private_usage: u64,
    /// `WorkingSetSize`.
    pub working_set: u64,
    /// `PeakWorkingSetSize`.
    pub peak_working_set: u64,
    /// Handle count.
    pub handles: u32,
    /// GDI objects.
    pub gdi_objects: u32,
    /// USER objects.
    pub user_objects: u32,
    /// Loaded module base names.
    pub modules: Vec<String>,
    /// Process exited cleanly (exit code 0) after `WM_CLOSE`.
    pub exit_ok: bool,
    /// First paint in ms since process creation (window property `YAMDV.FirstPaintMs`).
    pub first_paint_ms: Option<u64>,
    /// Actual client size (px) at measurement time.
    pub client: (i32, i32),
    /// Primary monitor resolution (px).
    pub monitor: (i32, i32),
    /// CPU time (kernel + user) during the `--cpu-ms` window, in µs.
    pub cpu_us: Option<u64>,
    /// Band buffer bytes reported by the app (window property `YAMDV.BandBytes`).
    pub band_bytes: Option<u64>,
    /// `Ctrl+O` → host dialog visible, ms (T-DLG).
    pub dlg_ms: Option<u64>,
    /// Main-process module count right before `Ctrl+O` (F-09: unchanged after the dialog).
    pub modules_before_dialog: Option<usize>,
}

/// Median of a list (lower median for even counts). Returns 0 for an empty list.
pub fn median(values: &mut [u64]) -> u64 {
    values.sort_unstable();
    if values.is_empty() {
        return 0;
    }
    values.get((values.len() - 1) / 2).copied().unwrap_or(0)
}

#[allow(clippy::cast_precision_loss)]
fn mib(v: u64) -> f64 {
    v as f64 / (1024.0 * 1024.0)
}

/// Median sample over all numeric fields (modules taken from the first run).
pub fn median_sample(samples: &[Sample]) -> Sample {
    let pick = |f: &dyn Fn(&Sample) -> u64| median(&mut samples.iter().map(f).collect::<Vec<_>>());
    let counter: Vec<u64> = samples
        .iter()
        .filter_map(|s| s.private_ws_counter)
        .collect();
    Sample {
        private_ws: pick(&|s| s.private_ws),
        private_ws_counter: if counter.is_empty() {
            None
        } else {
            Some(median(&mut counter.clone()))
        },
        private_usage: pick(&|s| s.private_usage),
        working_set: pick(&|s| s.working_set),
        peak_working_set: pick(&|s| s.peak_working_set),
        handles: u32::try_from(pick(&|s| u64::from(s.handles))).unwrap_or(u32::MAX),
        gdi_objects: u32::try_from(pick(&|s| u64::from(s.gdi_objects))).unwrap_or(u32::MAX),
        user_objects: u32::try_from(pick(&|s| u64::from(s.user_objects))).unwrap_or(u32::MAX),
        modules: samples
            .first()
            .map(|s| s.modules.clone())
            .unwrap_or_default(),
        exit_ok: samples.iter().all(|s| s.exit_ok),
        first_paint_ms: {
            let mut v: Vec<u64> = samples.iter().filter_map(|s| s.first_paint_ms).collect();
            if v.is_empty() {
                None
            } else {
                Some(median(&mut v))
            }
        },
        client: samples.first().map(|s| s.client).unwrap_or_default(),
        monitor: samples.first().map(|s| s.monitor).unwrap_or_default(),
        band_bytes: samples.first().and_then(|s| s.band_bytes),
        dlg_ms: {
            let mut v: Vec<u64> = samples.iter().filter_map(|s| s.dlg_ms).collect();
            if v.is_empty() {
                None
            } else {
                Some(median(&mut v))
            }
        },
        modules_before_dialog: samples.first().and_then(|s| s.modules_before_dialog),
        cpu_us: {
            let mut v: Vec<u64> = samples.iter().filter_map(|s| s.cpu_us).collect();
            if v.is_empty() {
                None
            } else {
                Some(median(&mut v))
            }
        },
    }
}

/// Rounds MiB to two decimals (P-05).
fn mib2(v: u64) -> f64 {
    (mib(v) * 100.0).round() / 100.0
}

/// Checks the median against the budgets; returns the violations.
pub fn check_budget(b: &Budget, samples: &[Sample]) -> Vec<String> {
    let m = median_sample(samples);
    let mut out = Vec::new();
    if let Some(x) = b.pws_mib
        && mib(m.private_ws) > x
    {
        out.push(format!("PWS {:.2} MiB > {x} MiB", mib(m.private_ws)));
    }
    #[allow(clippy::cast_precision_loss)]
    if let Some(x) = b.commit_ratio
        && m.private_ws > 0
        && m.private_usage as f64 / m.private_ws as f64 > x
    {
        out.push(format!(
            "commit/PWS {:.2} > {x}",
            m.private_usage as f64 / m.private_ws as f64
        ));
    }
    if let Some(x) = b.first_paint_ms {
        match m.first_paint_ms {
            Some(v) if v <= x => {}
            Some(v) => out.push(format!("first paint {v} ms > {x} ms")),
            None => out.push("first paint not reported".into()),
        }
    }
    #[allow(clippy::cast_precision_loss)]
    if let Some(x) = b.cpu_ms {
        match m.cpu_us {
            Some(v) if v as f64 / 1000.0 <= x => {}
            Some(v) => out.push(format!("idle CPU {:.3} ms > {x} ms", v as f64 / 1000.0)),
            None => out.push("idle CPU not measured".into()),
        }
    }
    if let Some(x) = b.commit_mib
        && mib(m.private_usage) > x
    {
        out.push(format!("commit {:.2} MiB > {x} MiB", mib(m.private_usage)));
    }
    if let Some(before) = m.modules_before_dialog
        && m.modules.len() > before
    {
        out.push(format!(
            "module count grew after the dialog: {before} -> {}",
            m.modules.len()
        ));
    }
    if !m.exit_ok {
        out.push("process did not exit cleanly".into());
    }
    out
}

fn sample_json(s: &Sample) -> Value {
    json!({
        "private_ws_bytes": s.private_ws,
        "private_ws_mib": mib2(s.private_ws),
        "private_usage_mib": mib2(s.private_usage),
        "working_set_mib": mib2(s.working_set),
        "first_paint_ms": s.first_paint_ms,
        "client_size": [s.client.0, s.client.1],
        "monitor_resolution": [s.monitor.0, s.monitor.1],
        "idle_cpu_us": s.cpu_us,
        "band_bytes": s.band_bytes,
        "t_dlg_ms": s.dlg_ms,
        "modules_before_dialog": s.modules_before_dialog,
        "module_count": s.modules.len(),
        "private_ws_counter_bytes": s.private_ws_counter,
        "private_usage_bytes": s.private_usage,
        "working_set_bytes": s.working_set,
        "peak_working_set_bytes": s.peak_working_set,
        "handles": s.handles,
        "gdi_objects": s.gdi_objects,
        "user_objects": s.user_objects,
        "module_count": s.modules.len(),
        "exit_ok": s.exit_ok,
    })
}

/// Build the JSON document for a set of runs.
pub fn to_json(opts: &Options, samples: &[Sample]) -> Value {
    let m = median_sample(samples);
    json!({
        "schema": SCHEMA,
        "label": opts.label,
        "exe": opts.exe,
        "doc": opts.doc,
        "args": opts.args,
        "settle_ms": opts.settle_ms,
        "requested_size": opts.size.map(|(w, h)| [w, h]),
        "open_dialog": opts.open_dialog,
        "cpu_window_ms": opts.cpu_ms,
        "units": "bytes + MiB (2^20), P-05",
        "runs": samples.len(),
        "method": "QueryWorkingSet shared-bit counting (Appendix D §D.3); median of runs",
        "samples": samples.iter().map(sample_json).collect::<Vec<_>>(),
        "median": sample_json(&m),
        "modules": m.modules,
    })
}

/// Validate that a JSON value follows the memprobe schema.
pub fn validate(v: &Value) -> Result<(), String> {
    if v.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        return Err("schema id missing or wrong".into());
    }
    for key in [
        "exe",
        "settle_ms",
        "runs",
        "samples",
        "median",
        "modules",
        "method",
    ] {
        if v.get(key).is_none() {
            return Err(format!("missing key {key}"));
        }
    }
    let check_sample = |s: &Value| -> Result<(), String> {
        for key in [
            "private_ws_bytes",
            "private_usage_bytes",
            "working_set_bytes",
            "peak_working_set_bytes",
            "handles",
            "gdi_objects",
            "user_objects",
            "module_count",
        ] {
            if s.get(key).and_then(Value::as_u64).is_none() {
                return Err(format!("sample key {key} missing or not an integer"));
            }
        }
        for key in ["private_ws_mib", "client_size", "monitor_resolution"] {
            if s.get(key).is_none() {
                return Err(format!("sample key {key} missing"));
            }
        }
        if s.get("exit_ok").and_then(Value::as_bool).is_none() {
            return Err("sample key exit_ok missing".into());
        }
        Ok(())
    };
    check_sample(v.get("median").unwrap_or(&Value::Null))?;
    let samples = v
        .get("samples")
        .and_then(Value::as_array)
        .ok_or("samples not an array")?;
    if samples.is_empty() {
        return Err("no samples".into());
    }
    for s in samples {
        check_sample(s)?;
    }
    Ok(())
}

/// Human-readable summary.
pub fn summary(opts: &Options, samples: &[Sample]) -> String {
    let m = median_sample(samples);
    let mut out = format!(
        "memprobe {} ({} runs, settle {} ms){}\n",
        opts.exe,
        samples.len(),
        opts.settle_ms,
        opts.label
            .as_deref()
            .map(|l| format!(" [{l}]"))
            .unwrap_or_default()
    );
    for (i, s) in samples.iter().enumerate() {
        out.push_str(&format!(
            "  run {}: PWS {:.2} MiB, commit {:.2} MiB, WS {:.2} MiB, peak {:.2} MiB, handles {}, GDI {}, USER {}, modules {}, exit_ok {}\n",
            i + 1,
            mib(s.private_ws),
            mib(s.private_usage),
            mib(s.working_set),
            mib(s.peak_working_set),
            s.handles,
            s.gdi_objects,
            s.user_objects,
            s.modules.len(),
            s.exit_ok
        ));
    }
    out.push_str(&format!(
        "  MEDIAN: PWS {:.2} MiB ({} bytes; counter {}), commit {:.2} MiB ({} bytes), WS {:.2} MiB, peak {:.2} MiB, handles {}, GDI {}, USER {}\n  first paint {:?} ms, idle CPU {:?} µs, client {:?}, monitor {:?}\n  modules ({}): {}\n",
        mib(m.private_ws),
        m.private_ws,
        m.private_ws_counter.map(|c| c.to_string()).unwrap_or_else(|| "n/a".into()),
        mib(m.private_usage),
        m.private_usage,
        mib(m.working_set),
        mib(m.peak_working_set),
        m.handles,
        m.gdi_objects,
        m.user_objects,
        m.first_paint_ms,
        m.cpu_us,
        m.client,
        m.monitor,
        m.modules.len(),
        m.modules.join(", ")
    ));
    out
}

#[cfg(windows)]
pub use win::run;

#[cfg(not(windows))]
/// memprobe needs Windows; on other hosts it fails with an explanatory error.
pub fn run(_opts: &Options) -> Result<Vec<Sample>, String> {
    Err("memprobe requires Windows (run it on CI windows-latest)".into())
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod win {
    //! Win32 measurement code. Every `unsafe` block calls documented Win32 APIs with valid
    //! handles we own and buffers sized as the API requires.
    use super::{Options, Sample};
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, HMODULE, HWND, LPARAM, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
        DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    };
    use windows::Win32::Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow};
    use windows::Win32::System::ProcessStatus::{
        EnumProcessModulesEx, GetModuleBaseNameW, GetProcessMemoryInfo, LIST_MODULES_ALL,
        PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX, PROCESS_MEMORY_COUNTERS_EX2,
        QueryWorkingSet,
    };
    use windows::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
    use windows::Win32::System::Threading::GetProcessTimes;
    use windows::Win32::System::Threading::{
        CreateProcessW, GR_GDIOBJECTS, GR_USEROBJECTS, GetExitCodeProcess, GetGuiResources,
        GetProcessHandleCount, PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOW,
        TerminateProcess, WaitForInputIdle, WaitForSingleObject,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GW_OWNER, GetClassNameW, GetClientRect, GetPropW, GetSystemMetrics, GetWindow,
        GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsWindow, IsWindowVisible,
        PostMessageW, SM_CXSCREEN, SM_CYSCREEN, SWP_NOMOVE, SWP_NOZORDER, SetWindowPos, WM_CLOSE,
        WM_COMMAND,
    };
    use windows::core::{BOOL, PCWSTR, PWSTR};

    fn quote(arg: &str) -> String {
        if !arg.is_empty() && !arg.contains([' ', '\t', '"']) {
            return arg.to_string();
        }
        let mut out = String::from('"');
        let mut backslashes = 0usize;
        for c in arg.chars() {
            match c {
                '\\' => backslashes += 1,
                '"' => {
                    out.push_str(&"\\".repeat(backslashes * 2 + 1));
                    out.push('"');
                    backslashes = 0;
                }
                _ => {
                    out.push_str(&"\\".repeat(backslashes));
                    out.push(c);
                    backslashes = 0;
                }
            }
        }
        out.push_str(&"\\".repeat(backslashes * 2));
        out.push('"');
        out
    }

    struct Proc {
        info: PROCESS_INFORMATION,
    }

    impl Drop for Proc {
        fn drop(&mut self) {
            // SAFETY: handles come from CreateProcessW and are closed exactly once here.
            unsafe {
                let _ = CloseHandle(self.info.hThread);
                let _ = CloseHandle(self.info.hProcess);
            }
        }
    }

    fn class(hwnd: HWND) -> String {
        let mut buf = [0u16; 256];
        // SAFETY: valid buffer.
        let n = unsafe { GetClassNameW(hwnd, &mut buf) };
        String::from_utf16_lossy(buf.get(..usize::try_from(n).unwrap_or(0)).unwrap_or(&[]))
    }

    fn find_window(pid: u32) -> Option<HWND> {
        find_window_class(pid, None)
    }

    fn find_window_class(pid: u32, want: Option<&str>) -> Option<HWND> {
        struct Search<'a> {
            pid: u32,
            want: Option<&'a str>,
            found: Option<HWND>,
        }
        unsafe extern "system" fn cb(hwnd: HWND, lp: LPARAM) -> BOOL {
            // SAFETY: lp is the address of the `Search` on the caller's stack, alive for the
            // duration of EnumWindows.
            let s = unsafe { &mut *(lp.0 as *mut Search) };
            let mut pid = 0u32;
            // SAFETY: hwnd is a valid window handle supplied by EnumWindows.
            unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
            // SAFETY: as above.
            // SAFETY: as above.
            let visible = unsafe { IsWindowVisible(hwnd) }.as_bool();
            if pid == s.pid
                && visible
                && s.want.is_none_or(|w| class(hwnd) == w)
                && (s.want.is_some() || class(hwnd) != "#32770")
            {
                s.found = Some(hwnd);
                return BOOL(0);
            }
            BOOL(1)
        }
        let mut s = Search {
            pid,
            want,
            found: None,
        };
        // SAFETY: the callback only dereferences `s`, which outlives the call.
        let _ = unsafe { EnumWindows(Some(cb), LPARAM(&mut s as *mut Search as isize)) };
        s.found
    }

    fn title(hwnd: HWND) -> String {
        let mut buf = [0u16; 512];
        // SAFETY: buffer is valid and its length is passed implicitly by the slice.
        let n = unsafe { GetWindowTextW(hwnd, &mut buf) };
        String::from_utf16_lossy(buf.get(..usize::try_from(n).unwrap_or(0)).unwrap_or(&[]))
    }

    fn page_size() -> u64 {
        let mut si = SYSTEM_INFO::default();
        // SAFETY: si is a valid out-pointer.
        unsafe { GetSystemInfo(&mut si) };
        u64::from(si.dwPageSize)
    }

    fn private_ws(h: HANDLE) -> Result<u64, String> {
        // PSAPI_WORKING_SET_INFORMATION: NumberOfEntries (usize) followed by usize blocks.
        let mut buf: Vec<usize> = vec![0; 1 + 16 * 1024];
        for _ in 0..8 {
            let bytes = u32::try_from(buf.len() * size_of::<usize>()).map_err(|e| e.to_string())?;
            // SAFETY: buf is a writable buffer of `bytes` bytes.
            let r = unsafe { QueryWorkingSet(h, buf.as_mut_ptr().cast(), bytes) };
            let n = buf.first().copied().unwrap_or(0);
            if r.is_ok() && n < buf.len() {
                let private = buf
                    .get(1..=n)
                    .unwrap_or(&[])
                    .iter()
                    .filter(|&&e| (e >> 8) & 1 == 0)
                    .count() as u64;
                return Ok(private * page_size());
            }
            buf.resize(n.max(buf.len()) * 2 + 1024, 0);
        }
        Err("QueryWorkingSet: buffer never large enough".into())
    }

    fn modules(h: HANDLE) -> Vec<String> {
        let mut mods = vec![HMODULE::default(); 1024];
        let mut needed = 0u32;
        let cb = u32::try_from(mods.len() * size_of::<HMODULE>()).unwrap_or(0);
        // SAFETY: mods is a writable array of `cb` bytes.
        if unsafe { EnumProcessModulesEx(h, mods.as_mut_ptr(), cb, &mut needed, LIST_MODULES_ALL) }
            .is_err()
        {
            return Vec::new();
        }
        let n = (needed as usize / size_of::<HMODULE>()).min(mods.len());
        let mut names: Vec<String> = mods
            .iter()
            .take(n)
            .map(|&m| {
                let mut buf = [0u16; 260];
                // SAFETY: valid process handle, module handle from EnumProcessModulesEx, buffer slice.
                let len = unsafe { GetModuleBaseNameW(h, Some(m), &mut buf) } as usize;
                String::from_utf16_lossy(buf.get(..len).unwrap_or(&[]))
            })
            .collect();
        names.sort_by_key(|s| s.to_ascii_lowercase());
        names
    }

    fn measure(h: HANDLE) -> Result<Sample, String> {
        let mut s = Sample {
            private_ws: private_ws(h)?,
            ..Sample::default()
        };
        let mut ex2 = PROCESS_MEMORY_COUNTERS_EX2 {
            cb: u32::try_from(size_of::<PROCESS_MEMORY_COUNTERS_EX2>()).unwrap_or(0),
            ..Default::default()
        };
        // SAFETY: ex2 is a correctly sized PROCESS_MEMORY_COUNTERS_EX2 with cb set.
        let r = unsafe {
            GetProcessMemoryInfo(
                h,
                (&mut ex2 as *mut PROCESS_MEMORY_COUNTERS_EX2).cast::<PROCESS_MEMORY_COUNTERS>(),
                ex2.cb,
            )
        };
        if r.is_ok() {
            s.private_ws_counter = Some(ex2.PrivateWorkingSetSize as u64);
            s.private_usage = ex2.PrivateUsage as u64;
            s.working_set = ex2.WorkingSetSize as u64;
            s.peak_working_set = ex2.PeakWorkingSetSize as u64;
        } else {
            let mut ex = PROCESS_MEMORY_COUNTERS_EX {
                cb: u32::try_from(size_of::<PROCESS_MEMORY_COUNTERS_EX>()).unwrap_or(0),
                ..Default::default()
            };
            // SAFETY: as above with the EX structure.
            unsafe {
                GetProcessMemoryInfo(
                    h,
                    (&mut ex as *mut PROCESS_MEMORY_COUNTERS_EX).cast::<PROCESS_MEMORY_COUNTERS>(),
                    ex.cb,
                )
            }
            .map_err(|e| e.to_string())?;
            s.private_usage = ex.PrivateUsage as u64;
            s.working_set = ex.WorkingSetSize as u64;
            s.peak_working_set = ex.PeakWorkingSetSize as u64;
        }
        // SAFETY: valid process handle and out-pointer.
        let _ = unsafe { GetProcessHandleCount(h, &mut s.handles) };
        // SAFETY: valid process handle.
        s.gdi_objects = unsafe { GetGuiResources(h, GR_GDIOBJECTS) };
        // SAFETY: valid process handle.
        s.user_objects = unsafe { GetGuiResources(h, GR_USEROBJECTS) };
        s.modules = modules(h);
        Ok(s)
    }

    fn screenshot(hwnd: HWND, path: &str) -> Result<(), String> {
        let mut rc = Default::default();
        // SAFETY: valid hwnd and out-pointer.
        unsafe { GetClientRect(hwnd, &mut rc) }.map_err(|e| e.to_string())?;
        let (w, h) = (rc.right - rc.left, rc.bottom - rc.top);
        if w <= 0 || h <= 0 {
            return Err("empty client area".into());
        }
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: u32::try_from(size_of::<BITMAPINFOHEADER>()).unwrap_or(0),
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        // SAFETY: standard DIB section + PrintWindow sequence; every object is released below.
        unsafe {
            let screen = GetDC(None);
            let dc = CreateCompatibleDC(Some(screen));
            let bmp = CreateDIBSection(Some(dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
                .map_err(|e| e.to_string())?;
            let old = SelectObject(dc, bmp.into());
            // PW_CLIENTONLY (1) | PW_RENDERFULLCONTENT (2)
            let ok = PrintWindow(hwnd, dc, PRINT_WINDOW_FLAGS(3)).as_bool();
            let len = w.unsigned_abs() as usize * h.unsigned_abs() as usize * 4;
            let mut data = std::slice::from_raw_parts(bits.cast::<u8>(), len).to_vec();
            SelectObject(dc, old);
            let _ = DeleteObject(bmp.into());
            let _ = DeleteDC(dc);
            ReleaseDC(None, screen);
            if !ok {
                return Err("PrintWindow failed".into());
            }
            for px in data.as_chunks_mut::<4>().0 {
                px.swap(0, 2);
                if let Some(a) = px.get_mut(3) {
                    *a = 255;
                }
            }
            crate::icons::write_png(
                path,
                u32::try_from(w).unwrap_or(0),
                u32::try_from(h).unwrap_or(0),
                data,
            )
        }
    }

    fn cpu_100ns(h: HANDLE) -> u64 {
        let (mut c, mut e, mut k, mut u) = (
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
        );
        // SAFETY: valid process handle and out-pointers.
        let _ = unsafe { GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u) };
        let f = |f: FILETIME| (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime);
        f(k) + f(u)
    }

    fn client(hwnd: HWND) -> (i32, i32) {
        let mut rc = Default::default();
        // SAFETY: valid hwnd and out-pointer.
        let _ = unsafe { GetClientRect(hwnd, &mut rc) };
        (rc.right - rc.left, rc.bottom - rc.top)
    }

    fn set_client(hwnd: HWND, w: i32, h: i32) {
        let mut wr = Default::default();
        // SAFETY: valid hwnd and out-pointer.
        let _ = unsafe { GetWindowRect(hwnd, &mut wr) };
        let (cw, ch) = client(hwnd);
        let (fw, fh) = ((wr.right - wr.left) - cw, (wr.bottom - wr.top) - ch);
        // SAFETY: valid hwnd (owned by the child; SetWindowPos works cross-process).
        let _ =
            unsafe { SetWindowPos(hwnd, None, 0, 0, w + fw, h + fh, SWP_NOMOVE | SWP_NOZORDER) };
    }

    fn first_paint(hwnd: HWND) -> Option<u64> {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(20) {
            // SAFETY: valid hwnd; static NUL-terminated property name.
            let v = unsafe { GetPropW(hwnd, windows::core::w!("YAMDV.FirstPaintMs")) };
            if !v.0.is_null() {
                return Some((v.0 as usize as u64).saturating_sub(1));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        None
    }

    /// A visible `#32770` window owned by `owner`, in any process (the dialog host, F-09).
    fn find_owned_dialog(owner: HWND) -> Option<HWND> {
        struct Search {
            owner: HWND,
            found: Option<HWND>,
        }
        unsafe extern "system" fn cb(hwnd: HWND, lp: LPARAM) -> BOOL {
            // SAFETY: lp is the address of the `Search` on the caller's stack, alive for the
            // duration of EnumWindows.
            let s = unsafe { &mut *(lp.0 as *mut Search) };
            // SAFETY: hwnd is a valid window handle supplied by EnumWindows.
            let visible = unsafe { IsWindowVisible(hwnd) }.as_bool();
            // SAFETY: as above.
            let own = unsafe { GetWindow(hwnd, GW_OWNER) }.ok();
            if visible && own == Some(s.owner) && class(hwnd) == "#32770" {
                s.found = Some(hwnd);
                return BOOL(0);
            }
            BOOL(1)
        }
        let mut s = Search { owner, found: None };
        // SAFETY: the callback only dereferences `s`, which outlives the call.
        let _ = unsafe { EnumWindows(Some(cb), LPARAM(&mut s as *mut Search as isize)) };
        s.found
    }

    /// Ctrl+O (via the menu command id), wait for the host's dialog, cancel it, wait until the
    /// main window is enabled again. Returns T-DLG in ms.
    fn open_and_cancel_dialog(hwnd: HWND) -> Result<u64, String> {
        let t0 = Instant::now();
        // SAFETY: valid window handle owned by the child process.
        let _ = unsafe { PostMessageW(Some(hwnd), WM_COMMAND, WPARAM(0x101), LPARAM(0)) };
        let dlg = loop {
            if let Some(d) = find_owned_dialog(hwnd) {
                break d;
            }
            if t0.elapsed() > Duration::from_secs(20) {
                return Err("Open dialog did not appear".into());
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        let ms = u64::try_from(t0.elapsed().as_millis()).unwrap_or(u64::MAX);
        std::thread::sleep(Duration::from_millis(1500));
        // SAFETY: valid dialog handle (WM_CLOSE on a common dialog = Cancel).
        let _ = unsafe { PostMessageW(Some(dlg), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        let start = Instant::now();
        let busy = || {
            // SAFETY: IsWindow / IsWindowEnabled only test the handles.
            unsafe { IsWindow(Some(dlg)).as_bool() || !IsWindowEnabled(hwnd).as_bool() }
        };
        while busy() {
            if start.elapsed() > Duration::from_secs(10) {
                return Err("Open dialog did not close / main window not re-enabled".into());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(ms)
    }

    /// Launch, measure and close the target `runs` times.
    pub fn run(opts: &Options) -> Result<Vec<Sample>, String> {
        let mut samples = Vec::new();
        for run in 0..opts.runs {
            let mut cmd = quote(&opts.exe);
            for a in opts.doc.iter().chain(opts.args.iter()) {
                cmd.push(' ');
                cmd.push_str(&quote(a));
            }
            let mut cmdw: Vec<u16> = cmd.encode_utf16().chain(Some(0)).collect();
            let si = STARTUPINFOW {
                cb: u32::try_from(size_of::<STARTUPINFOW>()).unwrap_or(0),
                ..Default::default()
            };
            let mut info = PROCESS_INFORMATION::default();
            // SAFETY: command line buffer is mutable and NUL-terminated; out-pointers are valid.
            unsafe {
                CreateProcessW(
                    PCWSTR::null(),
                    Some(PWSTR(cmdw.as_mut_ptr())),
                    None,
                    None,
                    false,
                    PROCESS_CREATION_FLAGS(0),
                    None,
                    PCWSTR::null(),
                    &si,
                    &mut info,
                )
            }
            .map_err(|e| format!("CreateProcessW({cmd}): {e}"))?;
            let p = Proc { info };
            // SAFETY: valid process handle.
            unsafe { WaitForInputIdle(p.info.hProcess, 15_000) };
            let start = Instant::now();
            let hwnd = loop {
                if let Some(h) = find_window(p.info.dwProcessId) {
                    break h;
                }
                if start.elapsed() > Duration::from_secs(15) {
                    // SAFETY: valid process handle.
                    let _ = unsafe { TerminateProcess(p.info.hProcess, 1) };
                    return Err("main window did not appear within 15 s".into());
                }
                std::thread::sleep(Duration::from_millis(20));
            };
            let fp = first_paint(hwnd);
            let (mut dlg_ms, mut modules_before) = (None, None);
            if let Some((w, h)) = opts.size {
                set_client(hwnd, w, h);
                std::thread::sleep(Duration::from_millis(300));
            }
            if opts.open_dialog {
                // SAFETY: valid process handle.
                unsafe { WaitForInputIdle(p.info.hProcess, 15_000) };
                std::thread::sleep(Duration::from_millis(1000));
                modules_before = Some(modules(p.info.hProcess).len());
                match open_and_cancel_dialog(hwnd) {
                    Ok(ms) => dlg_ms = Some(ms),
                    Err(e) => {
                        // SAFETY: valid process handle.
                        let _ = unsafe { TerminateProcess(p.info.hProcess, 1) };
                        return Err(e);
                    }
                }
            }
            if let Some(t) = &opts.wait_title {
                let start = Instant::now();
                while !title(hwnd).contains(t.as_str()) {
                    if start.elapsed() > Duration::from_secs(180) {
                        // SAFETY: valid process handle.
                        let _ = unsafe { TerminateProcess(p.info.hProcess, 1) };
                        return Err(format!("window title never contained {t:?}"));
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
            // SAFETY: valid process handle.
            unsafe { WaitForInputIdle(p.info.hProcess, 15_000) };
            std::thread::sleep(Duration::from_millis(u64::from(opts.settle_ms)));
            let mut sample = measure(p.info.hProcess)?;
            sample.first_paint_ms = fp;
            sample.dlg_ms = dlg_ms;
            sample.modules_before_dialog = modules_before;
            sample.client = client(hwnd);
            // SAFETY: valid hwnd; static property name.
            let bb = unsafe { GetPropW(hwnd, windows::core::w!("YAMDV.BandBytes")) };
            sample.band_bytes = (!bb.0.is_null()).then(|| (bb.0 as usize as u64).saturating_sub(1));
            // SAFETY: plain metric queries.
            sample.monitor =
                unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
            if opts.cpu_ms > 0 {
                let a = cpu_100ns(p.info.hProcess);
                std::thread::sleep(Duration::from_millis(u64::from(opts.cpu_ms)));
                sample.cpu_us = Some((cpu_100ns(p.info.hProcess) - a) / 10);
            }
            if run == 0
                && let Some(path) = &opts.screenshot
                && let Err(e) = screenshot(hwnd, path)
            {
                eprintln!("screenshot failed: {e}");
            }
            // SAFETY: valid window handle owned by the child process.
            let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
            // SAFETY: valid process handle.
            let waited = unsafe { WaitForSingleObject(p.info.hProcess, 10_000) };
            let mut code = 1u32;
            // SAFETY: valid process handle and out-pointer.
            let _ = unsafe { GetExitCodeProcess(p.info.hProcess, &mut code) };
            sample.exit_ok = waited.0 == 0 && code == 0;
            if waited.0 != 0 {
                // SAFETY: valid process handle.
                let _ = unsafe { TerminateProcess(p.info.hProcess, 1) };
            }
            samples.push(sample);
        }
        Ok(samples)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(pws: u64) -> Sample {
        Sample {
            private_ws: pws,
            private_ws_counter: Some(pws),
            private_usage: pws * 2,
            working_set: pws * 3,
            peak_working_set: pws * 4,
            handles: 10,
            gdi_objects: 5,
            user_objects: 3,
            modules: vec!["yamdv.exe".into(), "ntdll.dll".into()],
            exit_ok: true,
            first_paint_ms: Some(pws / 10),
            client: (1000, 700),
            monitor: (1024, 768),
            cpu_us: Some(0),
            band_bytes: Some(512_000),
            dlg_ms: None,
            modules_before_dialog: None,
        }
    }

    #[test]
    fn budgets_and_size() {
        let o = parse_args(
            &[
                "a.exe",
                "--size",
                "1000x700",
                "--budget-pws-mib",
                "4.8",
                "--open-dialog",
            ]
            .map(String::from),
        )
        .unwrap();
        assert_eq!(o.size, Some((1000, 700)));
        assert!(o.open_dialog);
        let small = s(1024 * 1024);
        assert!(check_budget(&o.budget, std::slice::from_ref(&small)).is_empty());
        let big = s(5 * 1024 * 1024);
        assert_eq!(check_budget(&o.budget, &[big]).len(), 1);
        let b = Budget {
            commit_ratio: Some(1.5),
            ..Budget::default()
        };
        assert_eq!(check_budget(&b, &[small]).len(), 1);
        assert!(parse_args(&["a.exe", "--size", "10x10"].map(String::from)).is_err());
        let v = to_json(&o, &[s(1024 * 1024)]);
        assert_eq!(v["median"]["private_ws_mib"], 1.0);
        assert_eq!(v["median"]["client_size"][0], 1000);
    }

    #[test]
    fn median_odd_and_even() {
        assert_eq!(median(&mut [3, 1, 2]), 2);
        assert_eq!(median(&mut [4, 1, 3, 2]), 2);
        assert_eq!(median(&mut []), 0);
    }

    #[test]
    fn json_output_matches_schema() {
        let opts = parse_args(&["app.exe".into(), "--runs".into(), "3".into()]).unwrap();
        let v = to_json(&opts, &[s(300), s(100), s(200)]);
        validate(&v).unwrap();
        assert_eq!(v["median"]["private_ws_bytes"], 200);
        assert_eq!(v["runs"], 3);
        assert_eq!(v["modules"][0], "yamdv.exe");
        // Round-trip through text.
        let text = serde_json::to_string_pretty(&v).unwrap();
        validate(&serde_json::from_str(&text).unwrap()).unwrap();
    }

    #[test]
    fn schema_validation_rejects_broken_documents() {
        let opts = parse_args(&["app.exe".into()]).unwrap();
        let mut v = to_json(&opts, &[s(1)]);
        v["schema"] = json!("other");
        assert!(validate(&v).is_err());
        let mut v = to_json(&opts, &[s(1)]);
        v["median"]["handles"] = json!("x");
        assert!(validate(&v).is_err());
        let mut v = to_json(&opts, &[s(1)]);
        v["samples"] = json!([]);
        assert!(validate(&v).is_err());
    }

    #[test]
    fn args_parse() {
        let o = parse_args(
            &[
                "a.exe",
                "--doc",
                "d.md",
                "--settle-ms",
                "500",
                "--runs",
                "1",
                "--arg",
                "--x",
                "--json",
                "o.json",
            ]
            .map(String::from),
        )
        .unwrap();
        assert_eq!(o.doc.as_deref(), Some("d.md"));
        assert_eq!(o.settle_ms, 500);
        assert_eq!(o.args, vec!["--x"]);
        assert!(parse_args(&["--runs".into(), "0".into()]).is_err());
        assert!(parse_args(&[]).is_err());
        assert!(parse_args(&["a".into(), "--bogus".into()]).is_err());
    }
}
