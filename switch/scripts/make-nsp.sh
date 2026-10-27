#!/usr/bin/env bash
# Packs switch/build/horizon.elf into an installable NSP with a home-screen icon and the full
# application memory: switch/build/Horizon.nsp.
# Usage: switch/scripts/make-nsp.sh PROD_KEYS [HACBREWPACK]
#   PROD_KEYS    your console's prod.keys (dumped with Lockpick_RCM); never commit it.
#   HACBREWPACK  the hacbrewpack binary (https://github.com/rlaphoenix/hacBrewPack); default: on PATH.
set -euo pipefail
. "$(dirname "$0")/env.sh"
KEYS=${1:?usage: make-nsp.sh PROD_KEYS [HACBREWPACK]}
HBP=${2:-hacbrewpack}
# A path, not a name on PATH: absolute, since the packing runs in the work folder.
case $HBP in */*) HBP=$(realpath "$HBP") ;; esac
KEYS=$(realpath "$KEYS")
TID=0100485952450000
VERSION=${VERSION:-0.1.0}
W=$SWITCH_BUILD/nsp
rm -rf "$W" && mkdir -p "$W/exefs" "$W/control"
elf2nso "$SWITCH_BUILD/horizon.elf" "$W/exefs/main"
cp "$SWITCH_DIR/npdm.json" "$W/npdm.json"
npdmtool "$W/npdm.json" "$W/exefs/main.npdm"
nacptool --create "Horizon" "Horizon (unofficial Switch port)" "$VERSION" "$W/control/control.nacp" --titleid=$TID
cp "$SWITCH_DIR/icon.jpg" "$W/control/icon_AmericanEnglish.dat"
(cd "$W" && "$HBP" -k "$KEYS" --titleid $TID --noromfs --nologo \
    --exefsdir exefs --controldir control --nspdir out --tempdir tmp --ncadir nca --backupdir bak)
cp "$W/out/$TID.nsp" "$SWITCH_BUILD/Horizon.nsp"
echo "built $SWITCH_BUILD/Horizon.nsp"
