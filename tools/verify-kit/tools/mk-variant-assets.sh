#!/bin/bash
# mk-variant-assets.sh BUNDLE_DIR NAME [VARIANT.flp]   (ci1, 2026-11-07; no Chrome, no build)
# Makes $K/assets-NAME (for serve.sh PORT NAME PAGEREF): the manifest of BUNDLE_DIR's own assets dir if there is one, else BUNDLE_DIR/manifest.json is used by the page as the bundle; the bundle dir; and proj -> proj-NAME,
# a directory with every file of $K/proj (symlinks) except serina1-serum2.flp, which is VARIANT.flp under that name (the sample paths of the project name it; alpha1's variants are used under that name, PM 2026-11-07).
# Without VARIANT.flp, proj -> $K/proj (the original). Prints what it made and the sha256 of the .flp the page will open.
set -eu
. /home/ubuntu/work/signature-studio-develop/lift/out-ci1/kit/env.sh
B=$(readlink -f "$1"); N=$2; V=${3:-}
A=$K/assets-$N
[ -f "$B/manifest.json" ] || { echo "no manifest.json in $B"; exit 2; }
rm -rf "$A"; mkdir -p "$A"
ln -s "$B" "$A/$(basename "$B")"
cp "$B/manifest.json" "$A/manifest.json"
if [ -n "$V" ]; then
  P=$K/proj-$N; rm -rf "$P"; mkdir -p "$P"
  for f in "$K"/proj/*; do b=$(basename "$f"); [ "$b" = serina1-serum2.flp ] || [ "$b" = serina1-serum2.zip ] || ln -s "$(readlink -f "$f")" "$P/$b"; done
  ln -s "$(readlink -f "$V")" "$P/serina1-serum2.flp"
  ln -s "$P" "$A/proj"
else
  ln -s "$K/proj" "$A/proj"
fi
echo "assets: $A"; ls -la "$A" | tail -n +4
echo "flp the page opens: $(sha256sum "$A/proj/serina1-serum2.flp" | cut -c1-64)"
