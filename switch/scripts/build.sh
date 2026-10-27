#!/usr/bin/env bash
# Builds the Switch executable: switch/build/horizon.elf.
# Usage (from the repository root): switch/scripts/build.sh
# Needs devkitPro (devkitA64, libnx, switch-mesa, switch-tools), a nightly Rust with rust-src, and
# NVK at switch/build/nvk/lib/libvulkan_bh.a (switch/README.md, "Building").
set -euo pipefail
. "$(dirname "$0")/env.sh"
cd "$SWITCH_DIR/.."

if [ ! -f "$SWITCH_BUILD/nvk/lib/libvulkan_bh.a" ]; then
    echo "error: $SWITCH_BUILD/nvk/lib/libvulkan_bh.a is missing (see switch/README.md, NVK)" >&2
    exit 1
fi
"$SWITCH_DIR/scripts/prepare-std.sh"
make -s -C "$SWITCH_DIR/nxshim" OUT="$SWITCH_BUILD/nxshim"

# The Cargo config, with this checkout's paths.
mkdir -p "$SWITCH_BUILD"
sed -e "s|@SWITCH_DIR@|$SWITCH_DIR|g" -e "s|@SWITCH_BUILD@|$SWITCH_BUILD|g" \
    -e "s|@DEVKITPRO@|$DEVKITPRO|g" \
    "$SWITCH_DIR/cargo-config.toml" > "$SWITCH_BUILD/cargo-config.toml"

NX_BUILD=$(date +%H%M) cargo +"${RUST_TOOLCHAIN:-nightly}" build --release \
    --config "$SWITCH_BUILD/cargo-config.toml" \
    -p horizon-client --no-default-features --bin horizon-client
cp target/aarch64-nintendo-switch-newlib/release/horizon-client.elf "$SWITCH_BUILD/horizon.elf"
echo "built $SWITCH_BUILD/horizon.elf"
