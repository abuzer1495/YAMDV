//! `cargo xtask size-map <map> [--top N]` (prompt 016 F-30): the largest exe-size contributors by
//! crate and module, from an `lld-link /MAP` (link.exe-format) map. Prints a Markdown table.
//!
//! Each public or static symbol of the code (`.text`) and read-only data (`.rdata`) sections is
//! sized as the distance to the next symbol of the same section (the last one to the section end)
//! and attributed by its Rust v0 / legacy mangled path; non-Rust objects are grouped by library.

use std::collections::BTreeMap;

/// One map entry: section, offset, name, library/object.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Sym {
    sec: u32,
    off: u64,
    name: String,
    lib: String,
}

fn hex(s: &str) -> Option<u64> {
    u64::from_str_radix(s.trim_end_matches('H'), 16).ok()
}

fn sec_off(s: &str) -> Option<(u32, u64)> {
    let (a, b) = s.split_once(':')?;
    Some((u32::from_str_radix(a, 16).ok()?, hex(b)?))
}

/// Parses section lengths and symbols of sections 1 (.text) and 2 (.rdata).
fn parse(map: &str) -> (BTreeMap<u32, u64>, Vec<Sym>) {
    let mut lens: BTreeMap<u32, u64> = BTreeMap::new();
    let mut syms = Vec::new();
    for line in map.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        let Some((sec, off)) = f.first().and_then(|s| sec_off(s)) else {
            continue;
        };
        if f.len() == 4 && f.get(1).is_some_and(|l| l.ends_with('H')) {
            // Section contribution line: start, length, name, class.
            if let Some(len) = f.get(1).and_then(|l| hex(l)) {
                let e = lens.entry(sec).or_insert(0);
                *e = (*e).max(off + len);
            }
            continue;
        }
        if !(1..=2).contains(&sec) || f.len() < 4 {
            continue;
        }
        let (Some(name), Some(lib)) = (f.get(1), f.last()) else {
            continue;
        };
        syms.push(Sym {
            sec,
            off,
            name: (*name).to_string(),
            lib: (*lib).to_string(),
        });
    }
    syms.sort_by_key(|s| (s.sec, s.off));
    syms.dedup_by_key(|s| (s.sec, s.off));
    (lens, syms)
}

/// Identifiers of a mangled Rust path (`<len><ident>` runs after the crate root).
fn idents(mut s: &str) -> Vec<String> {
    let mut out = Vec::new();
    while !s.is_empty() {
        let digits = s.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            s = s.get(1..).unwrap_or("");
            continue;
        }
        let n: usize = s.get(..digits).and_then(|d| d.parse().ok()).unwrap_or(0);
        let rest = s.get(digits..).unwrap_or("");
        let rest = rest.strip_prefix('_').unwrap_or(rest);
        match rest.get(..n) {
            Some(id) if n > 0 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') => {
                out.push(id.to_string());
                s = rest.get(n..).unwrap_or("");
            }
            _ => s = rest,
        }
    }
    out
}

/// The mangled text from the first v0 crate root (`C[s<base62>_]<digit>…`).
fn crate_root(r: &str) -> Option<&str> {
    let b = r.as_bytes();
    (0..b.len()).find_map(|i| {
        if b.get(i) != Some(&b'C') {
            return None;
        }
        let t = r.get(i + 1..)?;
        let t = match t.strip_prefix('s') {
            Some(u) => {
                let k = u.find('_')?;
                u.get(..k)?.bytes().all(|c| c.is_ascii_alphanumeric()).then_some(())?;
                u.get(k + 1..)?
            }
            None => t,
        };
        t.bytes().next().filter(u8::is_ascii_digit).map(|_| t)
    })
}

/// `crate::module` of a symbol (or `[lib]` for non-Rust objects).
fn owner(sym: &Sym) -> String {
    let n = &sym.name;
    let path = if let Some(r) = n.strip_prefix("_R") {
        // v0: the crate root is `C` + optional `s<disambiguator>_` + `<len><name>`.
        crate_root(r).map(idents)
    } else {
        n.strip_prefix("_ZN").map(idents)
    };
    match path {
        Some(p) if !p.is_empty() => {
            let c = p.first().cloned().unwrap_or_default();
            p.get(1).map_or(c.clone(), |m| format!("{c}::{m}"))
        }
        _ if n.starts_with("anon.") => "[anonymous constants]".to_string(),
        _ if n.starts_with("__real@") || n.starts_with("__xmm@") => "[float constants]".to_string(),
        _ if sym.lib.ends_with(".rcgu.o") => "[other Rust]".to_string(),
        _ => {
            let lib = sym.lib.split(':').next().unwrap_or("").to_string();
            format!("[{lib}]")
        }
    }
}

/// Bytes per `crate::module`, largest first.
pub fn contributors(map: &str) -> Vec<(String, u64)> {
    let (lens, syms) = parse(map);
    let mut by: BTreeMap<String, u64> = BTreeMap::new();
    for (i, s) in syms.iter().enumerate() {
        let next = syms
            .get(i + 1)
            .filter(|n| n.sec == s.sec)
            .map(|n| n.off)
            .or_else(|| lens.get(&s.sec).copied())
            .unwrap_or(s.off);
        *by.entry(owner(s)).or_insert(0) += next.saturating_sub(s.off);
    }
    let mut v: Vec<(String, u64)> = by.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v
}

/// The Markdown table of the top `n` contributors.
pub fn table(map: &str, n: usize) -> String {
    let all = contributors(map);
    let total: u64 = all.iter().map(|x| x.1).sum::<u64>().max(1);
    let mut s = String::from("| # | Crate::module | Bytes | Share |\n|---|---|---:|---:|\n");
    for (i, (k, b)) in all.iter().take(n).enumerate() {
        let pct = b.saturating_mul(1000) / total;
        s.push_str(&format!(
            "| {} | `{k}` | {b} | {}.{}% |\n",
            i + 1,
            pct / 10,
            pct % 10
        ));
    }
    s.push_str(&format!("| | total (.text + .rdata symbols) | {total} | |\n"));
    s
}

/// Entry point.
pub fn run(args: &[String]) -> Result<(), String> {
    let path = args.first().ok_or("usage: size-map <map> [--top N]")?;
    let n = match (args.get(1).map(String::as_str), args.get(2)) {
        (Some("--top"), Some(v)) => v.parse().map_err(|_| "bad --top")?,
        _ => 15,
    };
    let map = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    print!("{}", table(&map, n));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: &str = " Start         Length     Name                   Class
 0001:00000000 00000100H .text                   CODE
 0002:00000000 00000040H .rdata                  DATA

  Address         Publics by Value              Rva+Base               Lib:Object

 0001:00000000       _RNvNtNtCs4TPeffA9bLD_10yamdv_core3doc3toc5close 0000000140001000     yamdv.o
 0001:00000030       _RNvMNtCs7KOhBc1n3Zm_5ropey4ropeNtB2_4Rope6insert 0000000140001030     yamdv.o
 0001:00000050       memcpy                     0000000140001050     libvcruntime:memcpy.obj
 0002:00000000       _ZN10yamdv_core3doc4KEYS17h0123456789abcdefE 0000000140002000     yamdv.o
";

    #[test]
    fn attributes_by_crate_and_module() {
        let v = contributors(MAP);
        assert_eq!(
            v,
            vec![
                ("[libvcruntime]".to_string(), 0xb0),
                ("yamdv_core::doc".to_string(), 0x30 + 0x40),
                ("ropey::rope".to_string(), 0x20),
            ]
        );
        assert!(table(MAP, 2).contains("| 1 | `[libvcruntime]` | 176 |"));
    }
}
