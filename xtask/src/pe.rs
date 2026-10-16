//! PE import-table and hardening checks for `yamdv.exe` (S-NET-03, S-PROC-05, S-TEST-03).

use object::LittleEndian as LE;
use object::pe;
use object::read::pe::{ImageNtHeaders, ImageOptionalHeader, PeFile64};

/// DLLs that must never appear in the import or delay-import tables (S-NET-03).
pub const FORBIDDEN_DLLS: &[&str] = &[
    "ws2_32.dll",
    "wsock32.dll",
    "mswsock.dll",
    "winhttp.dll",
    "wininet.dll",
    "urlmon.dll",
    "dnsapi.dll",
    "iphlpapi.dll",
    "webio.dll",
    "httpapi.dll",
    "netapi32.dll",
    "mpr.dll",
    "rasapi32.dll",
    "wlanapi.dll",
    "winhttpcom.dll",
];

/// Forbidden function names: `(pattern, is_prefix)` (S-NET-03).
pub const FORBIDDEN_FUNCTIONS: &[(&str, bool)] = &[
    ("URLDownloadToFile", true),
    ("URLOpenStream", true),
    ("Internet", true),
    ("WinHttp", true),
    ("socket", false),
    ("WSA", true),
    ("connect", false),
    ("getaddrinfo", false),
    ("GetAddrInfo", true),
];

/// One imported DLL with its imported function names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DllImports {
    /// DLL name as stored in the PE file.
    pub dll: String,
    /// Function names (ordinals rendered as `#<n>`).
    pub functions: Vec<String>,
    /// `true` when the DLL comes from the delay-load import table.
    pub delay_load: bool,
}

/// Result of analysing one PE image.
#[derive(Debug, Clone, Default)]
pub struct PeReport {
    /// Imported DLLs (regular and delay-load).
    pub imports: Vec<DllImports>,
    /// `IMAGE_OPTIONAL_HEADER::DllCharacteristics`.
    pub dll_characteristics: u16,
    /// CET shadow-stack compatibility flag from the extended DLL characteristics debug entry.
    pub cet_compat: bool,
    /// Names of sections that are both writable and executable.
    pub wx_sections: Vec<String>,
}

/// A rule violation found by [`check`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// A forbidden DLL is imported.
    ForbiddenDll(String),
    /// A forbidden function is imported from the given DLL.
    ForbiddenFunction {
        /// DLL the function is imported from.
        dll: String,
        /// Function name.
        function: String,
    },
    /// A required hardening flag is missing.
    MissingHardening(&'static str),
    /// A section is both writable and executable.
    WritableExecutableSection(String),
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForbiddenDll(d) => write!(f, "forbidden DLL imported: {d}"),
            Self::ForbiddenFunction { dll, function } => {
                write!(f, "forbidden function imported: {dll}!{function}")
            }
            Self::MissingHardening(flag) => write!(f, "missing hardening flag: {flag}"),
            Self::WritableExecutableSection(s) => write!(f, "section is writable+executable: {s}"),
        }
    }
}

/// Error while parsing a PE file.
#[derive(Debug)]
pub struct PeError(pub String);

impl std::fmt::Display for PeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn err<E: std::fmt::Display>(e: E) -> PeError {
    PeError(e.to_string())
}

const IMAGE_DEBUG_TYPE_EX_DLLCHARACTERISTICS: u32 = 20;
const IMAGE_DLLCHARACTERISTICS_EX_CET_COMPAT: u32 = 0x1;

/// Parse a PE32+ image and collect imports and hardening information.
pub fn analyze(data: &[u8]) -> Result<PeReport, PeError> {
    let file = PeFile64::parse(data).map_err(err)?;
    let mut report = PeReport {
        dll_characteristics: file.nt_headers().optional_header().dll_characteristics().0,
        ..PeReport::default()
    };

    if let Some(table) = file.import_table().map_err(err)? {
        let mut descs = table.descriptors().map_err(err)?;
        while let Some(desc) = descs.next().map_err(err)? {
            let dll = String::from_utf8_lossy(table.name(desc.name.get(LE)).map_err(err)?).into();
            let first = desc.original_first_thunk.get(LE);
            let addr = if first != 0 {
                first
            } else {
                desc.first_thunk.get(LE)
            };
            let mut thunks = table.thunks(addr).map_err(err)?;
            let mut functions = Vec::new();
            while let Some(thunk) = thunks.next::<pe::ImageNtHeaders64>().map_err(err)? {
                functions.push(import_name(
                    table.import::<pe::ImageNtHeaders64>(thunk).map_err(err)?,
                ));
            }
            report.imports.push(DllImports {
                dll,
                functions,
                delay_load: false,
            });
        }
    }

    if let Some(table) = file.delay_load_import_table().map_err(err)? {
        let mut descs = table.descriptors().map_err(err)?;
        while let Some(desc) = descs.next().map_err(err)? {
            let dll =
                String::from_utf8_lossy(table.name(desc.dll_name_rva.get(LE)).map_err(err)?).into();
            let mut thunks = table
                .thunks(desc.import_name_table_rva.get(LE))
                .map_err(err)?;
            let mut functions = Vec::new();
            while let Some(thunk) = thunks.next::<pe::ImageNtHeaders64>().map_err(err)? {
                functions.push(import_name(
                    table.import::<pe::ImageNtHeaders64>(thunk).map_err(err)?,
                ));
            }
            report.imports.push(DllImports {
                dll,
                functions,
                delay_load: true,
            });
        }
    }

    for section in file.section_table().iter() {
        let c = section.characteristics.get(LE).0;
        if c & pe::IMAGE_SCN_MEM_WRITE.0 != 0 && c & pe::IMAGE_SCN_MEM_EXECUTE.0 != 0 {
            let name = String::from_utf8_lossy(section.raw_name())
                .trim_end_matches('\0')
                .to_string();
            report.wx_sections.push(name);
        }
    }

    report.cet_compat = cet_compat(&file, data);
    Ok(report)
}

fn import_name(import: object::read::pe::Import<'_>) -> String {
    match import {
        object::read::pe::Import::Name(_, name) => String::from_utf8_lossy(name).into(),
        object::read::pe::Import::Ordinal(n) => format!("#{n}"),
    }
}

fn cet_compat(file: &PeFile64<'_>, data: &[u8]) -> bool {
    let Some(dir) = file.data_directory(pe::IMAGE_DIRECTORY_ENTRY_DEBUG) else {
        return false;
    };
    let Ok(bytes) = dir.data(data, &file.section_table()) else {
        return false;
    };
    // IMAGE_DEBUG_DIRECTORY is 28 bytes: Type at offset 12, PointerToRawData at offset 24.
    for entry in bytes.as_chunks::<28>().0 {
        let rd = |o: usize| -> Option<u32> {
            let b: [u8; 4] = entry.get(o..o + 4)?.try_into().ok()?;
            Some(u32::from_le_bytes(b))
        };
        let (Some(kind), Some(size), Some(ptr)) = (rd(12), rd(16), rd(24)) else {
            continue;
        };
        if kind == IMAGE_DEBUG_TYPE_EX_DLLCHARACTERISTICS && size >= 4 {
            let p = ptr as usize;
            if let Some(b) = data.get(p..p + 4).and_then(|b| <[u8; 4]>::try_from(b).ok()) {
                let flags = u32::from_le_bytes(b);
                return flags & IMAGE_DLLCHARACTERISTICS_EX_CET_COMPAT != 0;
            }
        }
    }
    false
}

fn function_is_forbidden(name: &str) -> bool {
    FORBIDDEN_FUNCTIONS.iter().any(|(p, prefix)| {
        if *prefix {
            name.starts_with(p)
        } else {
            name == *p
        }
    })
}

/// Which checks to apply.
#[derive(Debug, Clone, Copy)]
pub struct CheckOptions {
    /// Also require the S-PROC-05 hardening flags.
    pub hardening: bool,
}

/// Apply the S-NET-03 (and optionally S-PROC-05) rules to a report.
pub fn check(report: &PeReport, opts: CheckOptions) -> Vec<Violation> {
    let mut v = Vec::new();
    for imp in &report.imports {
        let lower = imp.dll.to_ascii_lowercase();
        if FORBIDDEN_DLLS.contains(&lower.as_str()) {
            v.push(Violation::ForbiddenDll(imp.dll.clone()));
        }
        for f in &imp.functions {
            if function_is_forbidden(f) {
                v.push(Violation::ForbiddenFunction {
                    dll: imp.dll.clone(),
                    function: f.clone(),
                });
            }
        }
    }
    if opts.hardening {
        let dc = report.dll_characteristics;
        let flags: [(u16, &'static str); 4] = [
            (pe::IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE.0, "DYNAMIC_BASE"),
            (
                pe::IMAGE_DLLCHARACTERISTICS_HIGH_ENTROPY_VA.0,
                "HIGH_ENTROPY_VA",
            ),
            (pe::IMAGE_DLLCHARACTERISTICS_NX_COMPAT.0, "NX_COMPAT"),
            (pe::IMAGE_DLLCHARACTERISTICS_GUARD_CF.0, "GUARD_CF"),
        ];
        for (bit, name) in flags {
            if dc & bit == 0 {
                v.push(Violation::MissingHardening(name));
            }
        }
        if !report.cet_compat {
            v.push(Violation::MissingHardening("CET_COMPAT"));
        }
        for s in &report.wx_sections {
            v.push(Violation::WritableExecutableSection(s.clone()));
        }
    }
    v
}

#[cfg(test)]
#[allow(clippy::cast_possible_truncation, clippy::indexing_slicing)]
pub(crate) mod fixture {
    //! Builds minimal synthetic PE32+ images in memory (no binary fixtures committed).

    fn put16(b: &mut [u8], o: usize, v: u16) {
        b[o..o + 2].copy_from_slice(&v.to_le_bytes());
    }
    fn put32(b: &mut [u8], o: usize, v: u32) {
        b[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn put64(b: &mut [u8], o: usize, v: u64) {
        b[o..o + 8].copy_from_slice(&v.to_le_bytes());
    }

    /// Build a PE32+ image with the given imports and DLL characteristics.
    /// `delay` imports go into a delay-load table; `cet` adds the CET-compat debug entry.
    pub fn build(
        imports: &[(&str, &[&str])],
        delay: &[(&str, &[&str])],
        dll_characteristics: u16,
        cet: bool,
    ) -> Vec<u8> {
        const SEC_RVA: u32 = 0x1000;
        const SEC_RAW: u32 = 0x200;
        let mut sec: Vec<u8> = Vec::new();
        let n_imp = imports.len();
        let n_delay = delay.len();
        // Layout: import descriptors (20 each + null), delay descriptors (32 each + null),
        // debug directory (28), cet data (4), then thunk arrays and names.
        let imp_desc_off = 0usize;
        let delay_desc_off = imp_desc_off + 20 * (n_imp + 1);
        let debug_off = delay_desc_off + 32 * (n_delay + 1);
        let cet_off = debug_off + 28;
        let mut cursor = cet_off + 4;
        sec.resize(cursor, 0);
        let write_tables =
            |sec: &mut Vec<u8>, cursor: &mut usize, dll: &str, funcs: &[&str]| -> (u32, u32) {
                // thunk array
                let thunk_off = *cursor;
                *cursor += 8 * (funcs.len() + 1);
                sec.resize(*cursor, 0);
                let name_off = *cursor;
                sec.extend_from_slice(dll.as_bytes());
                sec.push(0);
                if !sec.len().is_multiple_of(2) {
                    sec.push(0);
                }
                *cursor = sec.len();
                for (i, f) in funcs.iter().enumerate() {
                    let hn = *cursor;
                    sec.extend_from_slice(&[0, 0]);
                    sec.extend_from_slice(f.as_bytes());
                    sec.push(0);
                    if !sec.len().is_multiple_of(2) {
                        sec.push(0);
                    }
                    *cursor = sec.len();
                    let rva = u64::from(SEC_RVA) + hn as u64;
                    sec[thunk_off + 8 * i..thunk_off + 8 * i + 8]
                        .copy_from_slice(&rva.to_le_bytes());
                }
                (SEC_RVA + thunk_off as u32, SEC_RVA + name_off as u32)
            };
        for (i, (dll, funcs)) in imports.iter().enumerate() {
            let (thunk, name) = write_tables(&mut sec, &mut cursor, dll, funcs);
            let d = imp_desc_off + 20 * i;
            put32(&mut sec, d, thunk); // OriginalFirstThunk
            put32(&mut sec, d + 12, name);
            put32(&mut sec, d + 16, thunk); // FirstThunk
        }
        for (i, (dll, funcs)) in delay.iter().enumerate() {
            let (thunk, name) = write_tables(&mut sec, &mut cursor, dll, funcs);
            let d = delay_desc_off + 32 * i;
            put32(&mut sec, d, 1); // attributes: RVA based
            put32(&mut sec, d + 4, name);
            put32(&mut sec, d + 12, thunk); // IAT
            put32(&mut sec, d + 16, thunk); // INT
        }
        let raw_size = sec.len().div_ceil(0x200) * 0x200;
        sec.resize(raw_size, 0);
        if cet {
            put32(&mut sec, debug_off + 12, 20); // Type
            put32(&mut sec, debug_off + 16, 4); // SizeOfData
            put32(&mut sec, debug_off + 20, SEC_RVA + cet_off as u32);
            put32(&mut sec, debug_off + 24, SEC_RAW + cet_off as u32);
            put32(&mut sec, cet_off, 1);
        }

        let mut img = vec![0u8; SEC_RAW as usize];
        img[0] = b'M';
        img[1] = b'Z';
        put32(&mut img, 0x3c, 0x40);
        img[0x40..0x44].copy_from_slice(b"PE\0\0");
        let coff = 0x44;
        put16(&mut img, coff, 0x8664);
        put16(&mut img, coff + 2, 1);
        put16(&mut img, coff + 16, 240);
        put16(&mut img, coff + 18, 0x22);
        let opt = coff + 20;
        put16(&mut img, opt, 0x20b);
        put64(&mut img, opt + 24, 0x1_4000_0000);
        put32(&mut img, opt + 32, 0x1000);
        put32(&mut img, opt + 36, 0x200);
        put16(&mut img, opt + 48, 6);
        let image_size = SEC_RVA + (raw_size as u32).div_ceil(0x1000) * 0x1000;
        put32(&mut img, opt + 56, image_size);
        put32(&mut img, opt + 60, SEC_RAW);
        put16(&mut img, opt + 68, 2);
        put16(&mut img, opt + 70, dll_characteristics);
        put32(&mut img, opt + 108, 16);
        let dd = opt + 112;
        put32(&mut img, dd + 8, SEC_RVA + imp_desc_off as u32);
        put32(&mut img, dd + 12, (20 * (n_imp + 1)) as u32);
        if cet {
            put32(&mut img, dd + 6 * 8, SEC_RVA + debug_off as u32);
            put32(&mut img, dd + 6 * 8 + 4, 28);
        }
        if n_delay > 0 {
            put32(&mut img, dd + 13 * 8, SEC_RVA + delay_desc_off as u32);
            put32(&mut img, dd + 13 * 8 + 4, (32 * (n_delay + 1)) as u32);
        }
        let sh = opt + 240;
        img[sh..sh + 6].copy_from_slice(b".idata");
        put32(&mut img, sh + 8, raw_size as u32);
        put32(&mut img, sh + 12, SEC_RVA);
        put32(&mut img, sh + 16, raw_size as u32);
        put32(&mut img, sh + 20, SEC_RAW);
        put32(&mut img, sh + 36, 0xC000_0040);
        img.extend_from_slice(&sec);
        img
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    const HARDENED: u16 = pe::IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE.0
        | pe::IMAGE_DLLCHARACTERISTICS_HIGH_ENTROPY_VA.0
        | pe::IMAGE_DLLCHARACTERISTICS_NX_COMPAT.0
        | pe::IMAGE_DLLCHARACTERISTICS_GUARD_CF.0;
    const ALL: CheckOptions = CheckOptions { hardening: true };

    #[test]
    fn s_net_03_clean_image_passes() {
        let img = fixture::build(
            &[
                ("KERNEL32.dll", &["CreateFileW", "ExitProcess"]),
                ("USER32.dll", &["CreateWindowExW"]),
            ],
            &[],
            HARDENED,
            true,
        );
        let r = analyze(&img).unwrap();
        assert_eq!(r.imports.len(), 2);
        assert_eq!(r.imports[0].functions, vec!["CreateFileW", "ExitProcess"]);
        assert!(r.cet_compat);
        assert_eq!(check(&r, ALL), vec![]);
    }

    #[test]
    fn s_net_03_forbidden_dll_and_functions_fail() {
        let img = fixture::build(
            &[
                ("KERNEL32.dll", &["ExitProcess"]),
                ("WS2_32.dll", &["socket", "WSAStartup", "connect"]),
            ],
            &[],
            HARDENED,
            true,
        );
        let v = check(&analyze(&img).unwrap(), ALL);
        assert!(v.contains(&Violation::ForbiddenDll("WS2_32.dll".into())));
        for f in ["socket", "WSAStartup", "connect"] {
            assert!(v.contains(&Violation::ForbiddenFunction {
                dll: "WS2_32.dll".into(),
                function: f.into()
            }));
        }
        assert_eq!(v.len(), 4);
    }

    #[test]
    fn s_net_03_delay_load_imports_are_checked() {
        let img = fixture::build(
            &[("KERNEL32.dll", &["ExitProcess"])],
            &[("wininet.dll", &["InternetOpenW"])],
            HARDENED,
            true,
        );
        let r = analyze(&img).unwrap();
        assert!(
            r.imports
                .iter()
                .any(|i| i.delay_load && i.dll == "wininet.dll")
        );
        let v = check(&r, ALL);
        assert!(v.contains(&Violation::ForbiddenDll("wininet.dll".into())));
        assert!(v.contains(&Violation::ForbiddenFunction {
            dll: "wininet.dll".into(),
            function: "InternetOpenW".into()
        }));
    }

    #[test]
    fn s_net_03_function_name_rules() {
        assert!(function_is_forbidden("URLDownloadToFileW"));
        assert!(function_is_forbidden("GetAddrInfoExW"));
        assert!(function_is_forbidden("WinHttpOpen"));
        assert!(function_is_forbidden("getaddrinfo"));
        assert!(!function_is_forbidden("connectx"));
        assert!(!function_is_forbidden("socketpair"));
        assert!(!function_is_forbidden("ConnectNamedPipe"));
    }

    #[test]
    fn s_test_03_missing_hardening_flags_fail() {
        let img = fixture::build(&[("KERNEL32.dll", &["ExitProcess"])], &[], 0, false);
        let v = check(&analyze(&img).unwrap(), ALL);
        for f in [
            "DYNAMIC_BASE",
            "HIGH_ENTROPY_VA",
            "NX_COMPAT",
            "GUARD_CF",
            "CET_COMPAT",
        ] {
            assert!(v.contains(&Violation::MissingHardening(f)), "{f}");
        }
        assert!(check(&analyze(&img).unwrap(), CheckOptions { hardening: false }).is_empty());
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert!(analyze(b"not a pe file").is_err());
        assert!(analyze(&[]).is_err());
    }
}
