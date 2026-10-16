//! Embeds the same manifest as the product (PerMonitorV2, UTF-8, Common Controls v6) so spike
//! measurements match `yamdv.exe`. `YAMDV_SEGMENT_HEAP=1` builds the SegmentHeap variant.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "../crates/yamdv-app/build_support/res.rs"]
mod res;

fn main() {
    println!("cargo:rerun-if-changed=../crates/yamdv-app/build_support/res.rs");
    println!("cargo:rerun-if-env-changed=YAMDV_SEGMENT_HEAP");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if os != "windows" || env != "msvc" {
        return;
    }
    let segment_heap = std::env::var("YAMDV_SEGMENT_HEAP").is_ok_and(|v| v == "1");
    let mut r = res::Res::new();
    r.manifest(&res::manifest_xml("YAMDV.spike", "0.1.0", segment_heap));
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("spike.res");
    std::fs::write(&out, r.finish()).unwrap();
    println!("cargo:rustc-link-arg-bins={}", out.display());
}
