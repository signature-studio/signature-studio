#!/bin/bash
# mt2 micro-prototype 1: builds exp/mp1-tls.wasm and exp/mp1-glob.wasm (negative control) with emscripten's clang. usage: build.sh OUTDIR
set -eu
export PATH=$PATH:/opt/emsdk/upstream/bin
HERE=$(cd "$(dirname "$0")" && pwd); OUT=${1:?OUTDIR}
mkdir -p "$OUT"
for v in tls glob; do
  D=""; [ $v = glob ] && D=-DGLOBAL_R
  clang --target=wasm32 -O2 -matomics -mbulk-memory -nostdlib $D -Wl,--no-entry -Wl,--shared-memory -Wl,--import-memory -Wl,--initial-memory=1048576 -Wl,--max-memory=67108864 \
    -Wl,--export-all -Wl,--export=__wasm_init_tls -Wl,--export=__tls_size -Wl,--export=__tls_align -Wl,--export=__stack_pointer "$HERE/mp1.c" -o "$OUT/mp1-$v.wasm"
done
ls -l "$OUT"
