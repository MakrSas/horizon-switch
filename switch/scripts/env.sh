# Environment for the Switch build. Source it from the repository root: `. switch/scripts/env.sh`.
# DEVKITPRO defaults to /opt/devkitpro (devkitA64, libnx, switch-mesa, switch-tools).
export DEVKITPRO=${DEVKITPRO:-/opt/devkitpro}
SWITCH_DIR=$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")/.." && pwd)
export SWITCH_DIR
export SWITCH_BUILD=${SWITCH_BUILD:-$SWITCH_DIR/build}
export PATH=$DEVKITPRO/devkitA64/bin:$DEVKITPRO/tools/bin:$PATH
# build-std builds this patched copy of the toolchain's std (scripts/prepare-std.sh).
export __CARGO_TESTS_ONLY_SRC_ROOT=$SWITCH_BUILD/rust-src/library
# C code in crates (cc): devkitA64 with libnx headers.
export CC_aarch64_nintendo_switch_newlib=$DEVKITPRO/devkitA64/bin/aarch64-none-elf-gcc
export AR_aarch64_nintendo_switch_newlib=$DEVKITPRO/devkitA64/bin/aarch64-none-elf-ar
export CFLAGS_aarch64_nintendo_switch_newlib="-O2 -march=armv8-a+crc+crypto -mtp=soft -fPIE -D__SWITCH__ -I$DEVKITPRO/libnx/include -I$DEVKITPRO/portlibs/switch/include"
