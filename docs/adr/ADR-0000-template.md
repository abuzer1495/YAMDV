# ADR-0000: ADR template and index

- Status: accepted (Phase 001)

Every architectural decision and every dependency (H5, S-SUP-02) gets an ADR, `ADR-NNNN-<slug>.md`, with:
Status · Context · Decision · Alternatives considered · Consequences (size/memory/security) · Revisit when.

| ADR | Title | Status |
|---|---|---|
| 0001 | Toolchain 1.99.0 and build hardening flags | accepted |
| 0002 | Rendering backend | accepted (see measurements) |
| 0003 | Dependency: `windows` / `windows-core` / `windows-numerics` | accepted |
| 0004 | Dependency (xtask only): `object` | accepted |
| 0005 | Dependency (xtask only): `serde_json` | accepted |
| 0006 | Dependency: `tiny-skia` | accepted (xtask + spike A; product use decided by ADR-0002) |
| 0019 | Dependency: `regex-lite` (find & replace regex) | accepted |
