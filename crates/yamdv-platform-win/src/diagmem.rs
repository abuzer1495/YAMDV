//! TEMPORARY V2-G diagnostic (export-progress PWS): counting allocator and a memory report
//! for the driver (`ui mem-report <label>`). dev-tools builds only; removed before merge.
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering::Relaxed};

/// Live Rust heap bytes.
pub static LIVE: AtomicIsize = AtomicIsize::new(0);
/// Peak live bytes since start or `mem-peak-reset`.
pub static PEAK: AtomicIsize = AtomicIsize::new(0);
/// Live blocks.
pub static BLOCKS: AtomicIsize = AtomicIsize::new(0);
/// Total allocations.
pub static ALLOCS: AtomicUsize = AtomicUsize::new(0);

struct Counting;

// SAFETY: forwards to `System`; only counts.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        // SAFETY: same contract as the caller's.
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            note(l.size() as isize, 1);
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        // SAFETY: same contract as the caller's.
        let p = unsafe { System.alloc_zeroed(l) };
        if !p.is_null() {
            note(l.size() as isize, 1);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(p, l) };
        note(-(l.size() as isize), -1);
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        // SAFETY: same contract as the caller's.
        let q = unsafe { System.realloc(p, l, n) };
        if !q.is_null() {
            note(n as isize - l.size() as isize, 0);
        }
        q
    }
}

fn note(d: isize, b: isize) {
    let live = LIVE.fetch_add(d, Relaxed) + d;
    BLOCKS.fetch_add(b, Relaxed);
    if b > 0 {
        ALLOCS.fetch_add(1, Relaxed);
    }
    PEAK.fetch_max(live, Relaxed);
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// Process heap committed / allocated bytes (`HeapSummary`).
pub fn heap_summary() -> (usize, usize) {
    use windows::Win32::System::Memory::{GetProcessHeap, HEAP_SUMMARY, HeapSummary};
    if std::env::var_os("WINEPREFIX").is_some() {
        return (0, 0); // Wine aborts on the unimplemented HeapSummary (local runs only).
    }
    let mut s = HEAP_SUMMARY {
        cb: u32::try_from(size_of::<HEAP_SUMMARY>()).unwrap_or(0),
        ..Default::default()
    };
    // SAFETY: valid heap handle and a sized out-structure.
    let ok = unsafe { GetProcessHeap().is_ok_and(|h| HeapSummary(h, 0, &raw mut s).as_bool()) };
    if ok {
        (s.cbCommitted, s.cbAllocated)
    } else {
        (0, 0)
    }
}
