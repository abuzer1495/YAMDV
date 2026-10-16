//! Startup process hardening, called first thing in `main` (S-PROC-01…04).
//! Each call's failure is logged and never fatal.
#![allow(unsafe_code)]

use windows::Win32::System::LibraryLoader::{
    LOAD_LIBRARY_SEARCH_SYSTEM32, SetDefaultDllDirectories, SetDllDirectoryW,
};
use windows::Win32::System::Memory::{HeapEnableTerminationOnCorruption, HeapSetInformation};
use windows::Win32::System::SystemServices::{
    PROCESS_MITIGATION_IMAGE_LOAD_POLICY, PROCESS_MITIGATION_IMAGE_LOAD_POLICY_0,
    PROCESS_MITIGATION_STRICT_HANDLE_CHECK_POLICY, PROCESS_MITIGATION_STRICT_HANDLE_CHECK_POLICY_0,
};
use windows::Win32::System::Threading::{
    ProcessImageLoadPolicy, ProcessStrictHandleCheckPolicy, SetProcessMitigationPolicy,
};
use windows::Win32::System::WindowsProgramming::{
    BASE_SEARCH_PATH_ENABLE_SAFE_SEARCHMODE, BASE_SEARCH_PATH_PERMANENT,
};
use windows::core::w;

/// Outcome of one hardening step.
#[derive(Debug, Clone)]
pub struct Step {
    /// Requirement-oriented name of the step.
    pub name: &'static str,
    /// `Ok` or the Win32 error text.
    pub result: Result<(), String>,
}

// ProcessImageLoadPolicy bits: NoRemoteImages | NoLowMandatoryLabelImages | PreferSystem32Images.
const IMAGE_LOAD_FLAGS: u32 = 0b111;
// ProcessStrictHandleCheckPolicy bits: RaiseExceptionOnInvalidHandleReference |
// HandleExceptionsPermanentlyEnabled.
const STRICT_HANDLE_FLAGS: u32 = 0b11;

/// Apply every hardening step and return the outcomes (failures are also logged).
pub fn apply() -> Vec<Step> {
    let mut steps = Vec::with_capacity(6);
    let mut push = |name: &'static str, r: windows::core::Result<()>| {
        let result = r.map_err(|e| e.to_string());
        if let Err(e) = &result {
            crate::log::warn("hardening", &format!("{name} failed: {e}"));
        }
        steps.push(Step { name, result });
    };

    // SAFETY: plain Win32 calls with constant arguments; no pointers are retained.
    push("SetDefaultDllDirectories(SYSTEM32)", unsafe {
        SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)
    });
    // SAFETY: w!("") is a valid NUL-terminated static wide string.
    push("SetDllDirectoryW(\"\")", unsafe {
        SetDllDirectoryW(w!(""))
    });
    // SAFETY: constant flags.
    push("SetSearchPathMode(SAFE|PERMANENT)", unsafe {
        windows::Win32::Storage::FileSystem::SetSearchPathMode(
            BASE_SEARCH_PATH_ENABLE_SAFE_SEARCHMODE | BASE_SEARCH_PATH_PERMANENT,
        )
    });
    // SAFETY: HeapEnableTerminationOnCorruption takes no information buffer.
    push("HeapSetInformation(TerminationOnCorruption)", unsafe {
        HeapSetInformation(None, HeapEnableTerminationOnCorruption, None, 0)
    });
    let image = PROCESS_MITIGATION_IMAGE_LOAD_POLICY {
        Anonymous: PROCESS_MITIGATION_IMAGE_LOAD_POLICY_0 {
            Flags: IMAGE_LOAD_FLAGS,
        },
    };
    // SAFETY: pointer and size describe a valid, initialized policy struct on the stack.
    push("SetProcessMitigationPolicy(ImageLoad)", unsafe {
        SetProcessMitigationPolicy(
            ProcessImageLoadPolicy,
            (&raw const image).cast(),
            size_of::<PROCESS_MITIGATION_IMAGE_LOAD_POLICY>(),
        )
    });
    let strict = PROCESS_MITIGATION_STRICT_HANDLE_CHECK_POLICY {
        Anonymous: PROCESS_MITIGATION_STRICT_HANDLE_CHECK_POLICY_0 {
            Flags: STRICT_HANDLE_FLAGS,
        },
    };
    // SAFETY: as above.
    push("SetProcessMitigationPolicy(StrictHandleCheck)", unsafe {
        SetProcessMitigationPolicy(
            ProcessStrictHandleCheckPolicy,
            (&raw const strict).cast(),
            size_of::<PROCESS_MITIGATION_STRICT_HANDLE_CHECK_POLICY>(),
        )
    });
    // PHASE-002: ProcessDynamicCodePolicy (SHOULD, S-PROC-04) is evaluated once DirectWrite, WIC,
    // IMEs and dialogs exist; it needs an ADR with test results.
    steps
}
