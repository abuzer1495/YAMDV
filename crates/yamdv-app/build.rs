//! Build script: embeds manifest, icons and version resource into `yamdv.exe` on MSVC targets via
//! a pure-Rust `.res` writer (S-SUP-05). Set `YAMDV_SEGMENT_HEAP=1` to build the SegmentHeap
//! manifest variant used for the §4.7 heap measurement.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::indexing_slicing
)]

#[path = "build_support/res.rs"]
mod res;

fn main() {
    println!("cargo:rerun-if-changed=build_support/res.rs");
    println!("cargo:rerun-if-changed=../../assets/icons/yamdv.ico");
    println!("cargo:rerun-if-changed=../../assets/icons/document.ico");
    println!("cargo:rerun-if-env-changed=YAMDV_SEGMENT_HEAP");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if os != "windows" || env != "msvc" {
        return;
    }
    let version = env!("CARGO_PKG_VERSION");
    let parts: Vec<u16> = version.split('.').map(|p| p.parse().unwrap_or(0)).collect();
    let v = (parts[0], parts[1], parts[2]);
    let segment_heap = std::env::var("YAMDV_SEGMENT_HEAP").is_ok_and(|v| v == "1");
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let icons = manifest_dir.join("../../assets/icons");

    let mut r = res::Res::new();
    r.manifest(&res::manifest_xml("YAMDV.yamdv", version, segment_heap));
    r.icon(
        1,
        &std::fs::read(icons.join("yamdv.ico"))
            .expect("assets/icons/yamdv.ico (run cargo xtask gen-icons)"),
    )
    .expect("app icon");
    r.icon(
        2,
        &std::fs::read(icons.join("document.ico")).expect("assets/icons/document.ico"),
    )
    .expect("document icon");
    let ver = format!("{}.{}.{}.0", v.0, v.1, v.2);
    r.version(
        v,
        &[
            ("FileDescription", "YAMDV - Markdown viewer"),
            ("FileVersion", &ver),
            ("InternalName", "yamdv"),
            ("LegalCopyright", "Copyright (c) 2026 snipeTR"),
            ("OriginalFilename", "yamdv.exe"),
            ("ProductName", "YAMDV"),
            ("ProductVersion", &ver),
        ],
    );
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("yamdv.res");
    std::fs::write(&out, r.finish()).unwrap();
    println!("cargo:rustc-link-arg-bins={}", out.display());
}
