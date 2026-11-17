#!/bin/bash
# mk-rtd-assets.sh [BUNDLE_DIR]  (ci1, 2026-11-07, PM 12:4x): assets for the RT-D bundle (RT-B + the Filter call diagnostic). The manifest is a COPY with FLE_DIAG_G9F=1 (and FLE_DIAG_WIN=1, FLE_DIAG_CB=1 as the default diagnostics of the RT-B tests; the bundle is not written) and the variant ins1-lp under the name serina1-serum2.flp.
# assets-rtd (original FLP + diagnostic env, port 5115) and assets-rtd-ins1lp (ins1-lp + diagnostic env, port 5116) are made; servers: bash serve.sh 5115 rtd rt1 ; bash serve.sh 5116 rtd-ins1lp rt1
set -eu
. "$(dirname "${BASH_SOURCE[0]}")/../env.sh"
B=$(readlink -f "${1:-$HOME/work/mt3w-s1/ci-rtd/engine/bundle/fl-engine-test-rtd-b9eacd3c}")
[ -f "$B/manifest.json" ] || { echo "no manifest in $B (the RT-D bundle is not there yet)"; exit 2; }
(cd "$B" && sha256sum -c SHA256SUMS | grep -vc ': OK$' | grep -qx 0) || { echo "SHA256SUMS of $B has a non-OK line: stop"; exit 3; }
bash $K/tools/mk-bundle-assets.sh "$B" rtd FLE_DIAG_G9F=1 FLE_DIAG_WIN=1 FLE_DIAG_CB=1
bash $K/tools/mk-bundle-assets.sh "$B" rtd-ins1lp FLE_DIAG_G9F=1 FLE_DIAG_WIN=1 FLE_DIAG_CB=1
P=$K/proj-rtd-ins1lp; rm -rf "$P"; mkdir -p "$P"
for f in "$K"/proj/*; do b=$(basename "$f"); [ "$b" = serina1-serum2.flp ] || [ "$b" = serina1-serum2.zip ] || ln -s "$(readlink -f "$f")" "$P/$b"; done
ln -s ~/work/alpha22w/variant-flp/ins1-lp.flp "$P/serina1-serum2.flp"; rm -f "$K/assets-rtd-ins1lp/proj"; ln -s "$P" "$K/assets-rtd-ins1lp/proj"
echo "flp in assets-rtd-ins1lp: $(sha256sum "$K/assets-rtd-ins1lp/proj/serina1-serum2.flp" | cut -c1-64)"
python3 -c "import json;print('env', sorted(json.load(open('$K/assets-rtd-ins1lp/manifest.json'))['engine']['env']))"
