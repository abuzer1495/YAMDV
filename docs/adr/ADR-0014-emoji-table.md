# ADR-0014 — Emoji shortcode table from gemoji (R-MD-13)

Status: accepted (Phase 003)

## Context
R-MD-13 asks for GitHub emoji shortcodes (`:smile:`) from a compile-time table, without
network access at build time.

## Decision
- Source: the gemoji database (`db/emoji.json`, https://github.com/github/gemoji), commit
  `fadaeaf1f1a9be82b321316a6c5502e43138b2f6`. License: **MIT** (Copyright (c) 2019 GitHub, Inc.),
  on the S-SUP-04 allowlist. Only data (shortcode → emoji) is used; no code.
- The table is generated once and committed as `crates/yamdv-core/src/emoji_table.rs`
  (1 913 aliases of 1 870 emoji, sorted by shortcode bytes, binary search). The generator is
  `tools/gen_emoji.py` (developer machine only; never run by the build).
- Matching (`doc::inline`): `:name:` with `name` in `[a-z0-9_+-]{1,64}`, only in text events
  (never inside code, link destinations or HTML); unknown names stay literal.

## Consequences
About 40 KB of static data in `yamdv.exe` (read-only section, shared pages; no PWS cost until
touched). Updating the table is a regenerate + review of the diff.
