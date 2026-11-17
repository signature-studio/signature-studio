#!/bin/bash
# mk-rtd2-assets.sh BUNDLE_DIR [NAME=rtd2-g9f]   (ci1, 2026-11-07, PM 13:0x): assets for the RT-D2 bundle (alpha1's third Filter-call diagnostic). The manifest is a COPY with FLE_DIAG_G9F=1 only (the bundle is not written), the project = alpha1's variant ins1-lp under the name serina1-serum2.flp.
# Makes assets-NAME (port 5118: bash serve.sh 5118 NAME rt1) and prints the command of the Export: PAGE=rt1 ALLOW_OTHER_CHROME=1 SCN=scn-export-g9c.json bash tools/run-cand1-export.sh NAME 5118 OUTDIR
set -eu
. "$(dirname "${BASH_SOURCE[0]}")/../env.sh"
[ -n "${1:-}" ] || { echo "usage: mk-rtd2-assets.sh BUNDLE_DIR [NAME]"; exit 2; }
B=$(readlink -f "$1"); N=${2:-rtd2-g9f}
[ -f "$B/manifest.json" ] || { echo "no manifest in $B"; exit 2; }
bash $K/tools/mk-bundle-assets.sh "$B" "$N" FLE_DIAG_G9F=1
P=$K/proj-$N-ins1lp; rm -rf "$P"; mkdir -p "$P"
for f in "$K"/proj/*; do b=$(basename -- "$f"); if [ "$b" != serina1-serum2.flp ] && [ "$b" != serina1-serum2.zip ]; then ln -s -- "$(readlink -f -- "$f")" "$P/$b"; fi; done
ln -s ~/work/alpha22w/variant-flp/ins1-lp.flp "$P/serina1-serum2.flp"; rm -f "$K/assets-$N/proj"; ln -s "$P" "$K/assets-$N/proj"
echo "files in proj minus the zip: $(( $(ls "$K/proj" | wc -l) - 1 )); in $P: $(ls "$P" | wc -l); broken links: $(find "$P" -xtype l | wc -l)"
echo "flp: $(sha256sum "$K/assets-$N/proj/serina1-serum2.flp" | cut -c1-64)"
python3 -c "import json;print('env', sorted(json.load(open('$K/assets-$N/manifest.json'))['engine']['env']))"
echo "server:  cd $K && bash serve.sh 5118 $N rt1"
echo "export:  cd $K && PAGE=rt1 ALLOW_OTHER_CHROME=1 SCN=scn-export-g9c.json bash tools/run-cand1-export.sh $N 5118 $K/out/rtd2-export-ins1lp"
