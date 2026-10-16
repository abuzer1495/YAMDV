//! YAMDV developer tasks: `cargo xtask <ci|check-imports|size|memprobe|gen-icons>`.
//! Dev-only; never linked into `yamdv.exe`.

mod icons;
mod memprobe;
mod pe;
mod sha256;
mod size;
mod spec;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

fn root() -> PathBuf {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    here.parent().map(Path::to_path_buf).unwrap_or(here)
}

/// Default location of the release `yamdv.exe` (native Windows build or cross build).
fn default_exe() -> PathBuf {
    let r = root().join("target");
    let native = r.join("release").join("yamdv.exe");
    let cross = r
        .join("x86_64-pc-windows-msvc")
        .join("release")
        .join("yamdv.exe");
    if cfg!(windows) && native.exists() {
        native
    } else if cross.exists() {
        cross
    } else {
        native
    }
}

fn exe_arg(args: &[String]) -> PathBuf {
    args.first().map(PathBuf::from).unwrap_or_else(default_exe)
}

fn check_imports(args: &[String]) -> Result<(), String> {
    let exe = exe_arg(args);
    let data = std::fs::read(&exe).map_err(|e| format!("{}: {e}", exe.display()))?;
    let report = pe::analyze(&data).map_err(|e| e.to_string())?;
    println!("check-imports: {}", exe.display());
    for imp in &report.imports {
        println!(
            "  {}{} ({} functions): {}",
            imp.dll,
            if imp.delay_load { " [delay-load]" } else { "" },
            imp.functions.len(),
            imp.functions.join(", ")
        );
    }
    println!(
        "  DllCharacteristics = {:#06x}, CET_COMPAT = {}, W+X sections = {:?}",
        report.dll_characteristics, report.cet_compat, report.wx_sections
    );
    let violations = pe::check(&report, pe::CheckOptions { hardening: true });
    if violations.is_empty() {
        println!(
            "check-imports: OK ({} DLLs, no forbidden imports, hardening flags present)",
            report.imports.len()
        );
        Ok(())
    } else {
        for v in &violations {
            println!("  VIOLATION: {v}");
        }
        Err(format!("check-imports: {} violation(s)", violations.len()))
    }
}

fn size_cmd(args: &[String]) -> Result<(), String> {
    let exe = exe_arg(args);
    let bytes = std::fs::metadata(&exe)
        .map_err(|e| format!("{}: {e}", exe.display()))?
        .len();
    let r = size::check(bytes);
    println!("{} — {}", exe.display(), r.summary());
    if r.ok() {
        Ok(())
    } else {
        Err("size over budget".into())
    }
}

fn memprobe_cmd(args: &[String]) -> Result<(), String> {
    let opts = memprobe::parse_args(args)?;
    let samples = memprobe::run(&opts)?;
    print!("{}", memprobe::summary(&opts, &samples));
    let v = memprobe::to_json(&opts, &samples);
    memprobe::validate(&v)?;
    if let Some(path) = &opts.json {
        let text = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?;
        std::fs::write(path, text).map_err(|e| format!("{path}: {e}"))?;
        println!("  json: {path}");
    }
    let bad = memprobe::check_budget(&opts.budget, &samples);
    if bad.is_empty() {
        println!("  budget: OK");
        Ok(())
    } else {
        Err(format!("BUDGET EXCEEDED: {}", bad.join("; ")))
    }
}

fn run(step: &str, program: &str, args: &[&str]) -> Result<(), String> {
    println!("==> [{step}] {program} {}", args.join(" "));
    let status = Command::new(program)
        .args(args)
        .current_dir(root())
        .status()
        .map_err(|e| format!("{program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("gate failed: {step}"))
    }
}

fn ci() -> Result<(), String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    run("fmt", &cargo, &["fmt", "--all", "--check"])?;
    run(
        "clippy",
        &cargo,
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run("test", &cargo, &["test", "--workspace", "--locked"])?;
    run("deny", &cargo, &["deny", "check"])?;
    if cfg!(windows) {
        run(
            "build",
            &cargo,
            &["build", "-p", "yamdv-app", "--release", "--locked"],
        )?;
    } else {
        // Non-Windows host (§4.1): cross-check Windows code and cross-build the product with cargo-xwin.
        let t = "x86_64-pc-windows-msvc";
        run(
            "clippy-windows",
            &cargo,
            &[
                "xwin",
                "clippy",
                "--target",
                t,
                "--workspace",
                "--all-targets",
                "--locked",
                "--",
                "-D",
                "warnings",
            ],
        )?;
        run(
            "build",
            &cargo,
            &[
                "xwin",
                "build",
                "--target",
                t,
                "-p",
                "yamdv-app",
                "--release",
                "--locked",
            ],
        )?;
    }
    println!("==> [check-imports]");
    check_imports(&[])?;
    println!("==> [size]");
    size_cmd(&[])?;
    println!("ci: all gates green");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cmd, rest) = match args.split_first() {
        Some((c, r)) => (c.as_str(), r),
        None => ("help", &[][..]),
    };
    let result = match cmd {
        "ci" => ci(),
        "check-imports" => check_imports(rest),
        "size" => size_cmd(rest),
        "memprobe" => memprobe_cmd(rest),
        "spec-check" => spec::check(&root()),
        "spec-patch" => spec::patch(&root()),
        "gen-icons" => icons::generate(&root()).map(|files| {
            for f in files {
                println!("wrote {f}");
            }
        }),
        _ => {
            println!(
                "usage: cargo xtask <ci | check-imports [<exe>] | size [<exe>] | memprobe <exe> [...] | gen-icons | spec-check | spec-patch>"
            );
            Ok(())
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
