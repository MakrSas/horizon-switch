#!/usr/bin/env bash
# Copies the toolchain's std sources to switch/build/rust-src and applies switch/std/std.patch:
# std's own workspace then builds against the patched libc (switch/patches/libc-0.2.191), and
# available_parallelism asks the kernel (nxshim). build-std ignores the workspace's [patch], so
# std gets the patch in its own Cargo.toml.
set -euo pipefail
. "$(dirname "$0")/env.sh"
SYSROOT=$(rustc +"${RUST_TOOLCHAIN:-nightly}" --print sysroot)
SRC=$SYSROOT/lib/rustlib/src/rust/library
DEST=$SWITCH_BUILD/rust-src
STAMP=$DEST/.prepared
WANT="$(rustc +"${RUST_TOOLCHAIN:-nightly}" -V) $(sha256sum "$SWITCH_DIR/std/std.patch" | cut -c1-16)"
if [ -f "$STAMP" ] && [ "$(cat "$STAMP")" = "$WANT" ]; then
    exit 0
fi
[ -d "$SRC" ] || { echo "error: no rust-src in $SYSROOT (rustup component add rust-src)" >&2; exit 1; }
rm -rf "$DEST"
mkdir -p "$DEST"
cp -r "$SRC" "$DEST/library"
patch -s -p1 -d "$DEST" < "$SWITCH_DIR/std/std.patch"
sed -i "s|@SWITCH_DIR@|$SWITCH_DIR|g" "$DEST/library/Cargo.toml"
# std's own lock pins crates.io libc: the libc entry loses its source and checksum, so cargo
# takes the patched copy at the path in std's Cargo.toml, or std builds the 3DS layouts.
python3 - "$DEST/library/Cargo.lock" <<'PY'
import re, sys
p = sys.argv[1]
s = open(p).read()
s = re.sub(r'(\[\[package\]\]\nname = "libc"\nversion = "[^"]+"\n)source = "[^"]+"\n(checksum = "[^"]+"\n)?', r"\1", s)
open(p, "w").write(s)
PY
if grep -A2 '^name = "libc"' "$DEST/library/Cargo.lock" | grep -q '^source'; then
    echo "error: std would build against crates.io libc, not switch/patches/libc-0.2.191" >&2
    exit 1
fi
echo "$WANT" > "$STAMP"
echo "prepared std in $DEST"
