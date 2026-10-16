#!/usr/bin/env python3
"""Regenerates crates/yamdv-core/src/emoji_table.rs from a local gemoji db/emoji.json (ADR-0014).
Usage: python3 tools/gen_emoji.py path/to/emoji.json COMMIT"""
import json, sys
d = json.load(open(sys.argv[1], encoding="utf-8"))
commit = sys.argv[2]
pairs = {}
for e in d:
    for a in e["aliases"]:
        pairs.setdefault(a, e["emoji"])
items = sorted(pairs.items(), key=lambda kv: kv[0].encode())
out = ["//! GitHub emoji shortcodes (R-MD-13). GENERATED from the gemoji database",
       "//! (https://github.com/github/gemoji, `db/emoji.json` at commit",
       f"//! {commit}, MIT license, ADR-0014). Do not edit by hand.", "",
       "/// (shortcode without colons, emoji), sorted by shortcode bytes for binary search.",
       f"pub static EMOJI: [(&str, &str); {len(items)}] = ["]
for k, v in items:
    esc = "".join("\\u{%x}" % ord(c) for c in v if ord(c) >= 0x20)
    out.append(f'    ("{k}", "{esc}"),')
out.append("];")
open("crates/yamdv-core/src/emoji_table.rs", "w", encoding="utf-8").write("\n".join(out) + "\n")
