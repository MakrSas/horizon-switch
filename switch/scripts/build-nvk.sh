#!/usr/bin/env bash
# Builds NVK, Mesa's Vulkan driver, for the Switch with bevy-horizon's container scripts, and
# stages the linkable archive at switch/build/nvk/lib/libvulkan_bh.a.
# Usage: switch/scripts/build-nvk.sh   (needs Docker or Podman and about 20 GB of disk)
#   mesa-switch: https://github.com/danfromtico/mesa-switch (danfromtico, NaGaa95)
#   bevy-horizon: https://github.com/JohnUzoka/bevy-horizon (its NVK patches and build scripts)
set -euo pipefail
. "$(dirname "$0")/env.sh"
MESA_COMMIT=606441c90e459028a50fca352daebfc20211f890
BEVY_HORIZON_COMMIT=3eac779b232ee47cd0eb957676c64386ac7b0f1d
W=$SWITCH_BUILD/nvk-src
mkdir -p "$W"
cd "$W"
[ -d bevy-horizon ] || git clone https://github.com/JohnUzoka/bevy-horizon
[ -d mesa-switch ] || git clone https://github.com/danfromtico/mesa-switch
git -C bevy-horizon checkout -q $BEVY_HORIZON_COMMIT
git -C mesa-switch checkout -q $MESA_COMMIT
ENGINE="${CONTAINER_ENGINE:-$(command -v docker >/dev/null && echo docker || echo podman)}"
"$ENGINE" image inspect bevy-horizon-toolchain:local >/dev/null 2>&1 ||
    "$ENGINE" build -t bevy-horizon-toolchain:local -f bevy-horizon/Containerfile bevy-horizon
bevy-horizon/scripts/build_nvk.sh "$W/mesa-switch"
bevy-horizon/scripts/localize_nvk.sh
mkdir -p "$SWITCH_BUILD/nvk/lib"
cp bevy-horizon/nvk/lib/libvulkan_bh.a "$SWITCH_BUILD/nvk/lib/"
echo "staged $SWITCH_BUILD/nvk/lib/libvulkan_bh.a"
