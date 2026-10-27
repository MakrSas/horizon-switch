#!/usr/bin/env bash
# Packs prepared game resources into one RomFS image for the SD card. One image opens far faster
# on the Switch than thousands of loose files (each open is a file-system IPC round trip).
# Usage: switch/scripts/make-romfs.sh RESOURCES_DIR [OUT]
#   RESOURCES_DIR  what `horizon-prepare --prepare` made from your own copy of Minecraft
#   OUT            default: resources.romfs; copy it to sdmc:/switch/horizon/resources.romfs
set -euo pipefail
. "$(dirname "$0")/env.sh"
SRC=${1:?usage: make-romfs.sh RESOURCES_DIR [OUT]}
OUT=${2:-resources.romfs}
build_romfs "$SRC" "$OUT"
ls -la "$OUT"
