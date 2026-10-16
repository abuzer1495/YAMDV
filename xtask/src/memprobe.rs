//! `cargo xtask memprobe`: launch an app, wait for its idle main window, measure memory per
//! Appendix D §D.3, close it with `WM_CLOSE`, verify exit. The measurement itself is Windows-only;
//! argument parsing, statistics and the JSON schema are platform-independent and unit-tested.

use serde_json::{Value, json};

/// JSON schema identifier written into every memprobe output.
pub const SCHEMA: &str = "yamdv.memprobe/1";

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
}

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
    };
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
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            s if o.exe.is_empty() => o.exe = s.to_string(),
            s => return Err(format!("unexpected argument {s}")),
        }
    }
    if o.exe.is_empty() {
        return Err("usage: memprobe <exe> [--doc <file>] [--arg <x>]… [--settle-ms 3000] [--runs 3] [--json out.json] [--wait-title <text>] [--screenshot out.png] [--label <name>]".into());
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
    }
}

fn sample_json(s: &Sample) -> Value {
    json!({
        "private_ws_bytes": s.private_ws,
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
        "  MEDIAN: PWS {:.2} MiB ({} bytes; counter {}), commit {:.2} MiB, WS {:.2} MiB, peak {:.2} MiB, handles {}, GDI {}, USER {}\n  modules ({}): {}\n",
        mib(m.private_ws),
        m.private_ws,
        m.private_ws_counter.map(|c| c.to_string()).unwrap_or_else(|| "n/a".into()),
        mib(m.private_usage),
        mib(m.working_set),
        mib(m.peak_working_set),
        m.handles,
        m.gdi_objects,
        m.user_objects,
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
    use windows::Win32::System::Threading::{
        CreateProcessW, GR_GDIOBJECTS, GR_USEROBJECTS, GetExitCodeProcess, GetGuiResources,
        GetProcessHandleCount, PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOW,
        TerminateProcess, WaitForInputIdle, WaitForSingleObject,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClientRect, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
        PostMessageW, WM_CLOSE,
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

    fn find_window(pid: u32) -> Option<HWND> {
        struct Search {
            pid: u32,
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
            if pid == s.pid && unsafe { IsWindowVisible(hwnd) }.as_bool() {
                s.found = Some(hwnd);
                return BOOL(0);
            }
            BOOL(1)
        }
        let mut s = Search { pid, found: None };
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
        }
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
