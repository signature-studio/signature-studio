#!/bin/bash
# ci/lib/extract_vst3.sh INSTALLER OUTDIR - extracts the VST3 of one installer named in ci/extract-vst3.tsv and checks its SHA-256.
#   Serum 2 and OTT: NSIS installers, read with 7z (only the one member). Vital: Inno Setup 6.7 (7z and innoextract cannot open it): docs/investigations/vst3-hosting/tools/inno67_carve.py, member = the 2nd carved file.
set -u -o pipefail
CI_DIR=$(cd "$(dirname "$0")/.." && pwd); REPO=$(cd "$CI_DIR/.." && pwd)
inst=$1; out=$2; base=$(basename "$inst")
SEVENZ=$(command -v 7z || command -v 7zz || command -v 7za || true)
[ -n "$SEVENZ" ] || { echo "extract_vst3: no 7z / 7zz / 7za on PATH (apt-get install 7zip)" >&2; exit 2; }
tmp=$(mktemp -d "${TMPDIR:-/tmp}/vst3.XXXXXX"); trap 'rm -rf "$tmp"' EXIT
row=$(awk -F'\t' -v b="$base" '!/^#/ && $1==b' "$CI_DIR/extract-vst3.tsv"); [ -n "$row" ] || { echo "extract_vst3: $base is not in ci/extract-vst3.tsv" >&2; exit 2; }
IFS=$'\t' read -r _ member dest want <<< "$row"
case "$base" in
  VitalInstaller.exe) python3 "$REPO/docs/investigations/vst3-hosting/tools/inno67_carve.py" "$inst" "$tmp/" > "$tmp/carve.log" 2>&1 || { cat "$tmp/carve.log" >&2; exit 2; }; src=$tmp/$member ;;
  *) "$SEVENZ" x -y -bd -o"$tmp" "$inst" "$member" > "$tmp/7z.log" 2>&1 || { cat "$tmp/7z.log" >&2; exit 2; }; src=$tmp/$member ;;
esac
[ -f "$src" ] || { echo "extract_vst3: $member not found in $base" >&2; exit 2; }
got=$(sha256sum "$src" | cut -d' ' -f1)
[ "$got" = "$want" ] || { echo "extract_vst3: $dest has SHA-256 $got, expected $want" >&2; exit 2; }
mkdir -p "$out"; mv "$src" "$out/$dest"
echo "extract_vst3: $dest ($(stat -c %s "$out/$dest") bytes) from $base, SHA-256 verified"
