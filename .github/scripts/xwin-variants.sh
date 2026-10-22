#!/bin/bash
# V2-B fix diagnosis: build cargo-xwin variants (hardening flags on/off) of a hello-world and of
# yamdv.exe so the Windows smoke can tell which flag/toolchain combination crashes at startup.
set -euo pipefail
T=x86_64-pc-windows-msvc
out=$PWD/pack/variants; mkdir -p "$out"
base=(-C target-feature=+crt-static -C link-arg=/DYNAMICBASE -C link-arg=/HIGHENTROPYVA -C link-arg=/NXCOMPAT -C link-arg=/OPT:REF -C link-arg=/OPT:ICF)
cfg=(-C control-flow-guard); cet=(-C link-arg=/CETCOMPAT)
# cargo-xwin merges .cargo/config.toml rustflags with RUSTFLAGS, so park the config meanwhile.
mv .cargo/config.toml .cargo/config.toml.off; trap 'mv .cargo/config.toml.off .cargo/config.toml' EXIT
declare -A V=(
  [full]="${base[*]} ${cfg[*]} ${cet[*]}"
  [nocet]="${base[*]} ${cfg[*]}"
  [nocfg]="${base[*]} ${cet[*]}"
  [plain]="${base[*]}"
)
# Inside the repo so rust-toolchain.toml applies; [workspace] keeps it standalone.
h=$PWD/target/hello; rm -rf "$h"; mkdir -p "$h/src"
printf '[package]\nname = "hello"\nversion = "0.1.0"\nedition = "2024"\n\n[workspace]\n' >| "$h/Cargo.toml"
printf 'fn main() { println!("hello"); }\n' >| "$h/src/main.rs"
for k in "${!V[@]}"; do
  (cd "$h" && RUSTFLAGS="${V[$k]}" cargo xwin build -q --release --target $T --target-dir "$h/t-$k")
  cp "$h/t-$k/$T/release/hello.exe" "$out/hello-$k.exe"
done
for k in nocet nocfg; do
  RUSTFLAGS="${V[$k]}" cargo xwin build -q --release --locked --target $T -p yamdv-app --target-dir "target/v-$k"
  cp "target/v-$k/$T/release/yamdv.exe" "$out/yamdv-$k.exe"
done
ls -la "$out"
