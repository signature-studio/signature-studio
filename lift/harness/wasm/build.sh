#!/bin/bash
# wasm build of the fe-lift output: runtime (all fle_*.c) + all-pdata generated C (16 parts) + wasm harness -> flengine.wasm
# (docs/decisions/0045-wasm-port.md, run record docs/investigations/wasm-port-run.md). Target: C -> emcc -> wasm, run in node.
#
# usage: bash lift/harness/wasm/build.sh [stage...]       stages: gen split rt gen-c harness link   (default: all, in that order)
# env:   OUT=lift/out-wasm2   VARIANT=base   JOBS=2 (concurrent emcc; keep <= 2: one generated-C TU peaks near 1.8 GB RSS)
#        EXTRA_CFLAGS=""     (added to every compile)
#        INSTRUMENT=1        (-finstrument-functions on the generated C, which trace.c needs for its report)
#        MXCSR_OVERRIDE=auto (auto | 1 | 0, see below)
#        INITMEM, MAXMEM, GLOBAL_BASE (linker memory layout)
# Everything is written under $OUT (obj-$VARIANT/, bin-$VARIANT/, out/, tmp/, emcache/). Nothing is deleted or overwritten:
# a stage whose output exists is skipped; to redo it, move the output away.
#
# INTERIM (MXCSR): until runtime/fle_mxcsr_flags.h is the portable C version (another work item replaces the inline-asm
# host-SSE version), the build puts harness/wasm/interim/fle_mxcsr_flags.h (the probe's software version: no MXCSR status
# bits, RC/FTZ/DAZ ignored; docs/investigations/wasm-build-probe.md section 3.2 case A) in its place. A plain -I does not
# do that: fle_sse.h includes "fle_mxcsr_flags.h" with quotes and the compiler looks in fle_sse.h's own directory first.
# So the replacement is a clang virtual-file-system overlay (-ivfsoverlay), generated here, that maps
# runtime/fle_mxcsr_flags.h to the interim file. With MXCSR_OVERRIDE=auto (default) the overlay is used only when
# runtime/fle_mxcsr_flags.h still contains inline asm; once the runtime header is portable the overlay is not applied
# and this note can go (switch: MXCSR_OVERRIDE=0, or delete harness/wasm/interim).
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/../.." && pwd)
RT="$LIFT/runtime"
OUT=${OUT:-$LIFT/out-wasm2}
VARIANT=${VARIANT:-base}
JOBS=${JOBS:-2}
EXTRA_CFLAGS=${EXTRA_CFLAGS:-}
INSTRUMENT=${INSTRUMENT:-1}
MXCSR_OVERRIDE=${MXCSR_OVERRIDE:-auto}
DLL=${DLL:-$LIFT/../binaries/win64/FLEngine_x64.dll}
INITMEM=${INITMEM:-$((0x28000000))}      # 640 MiB; the arena (512 MiB) grows the memory on first use (ALLOW_MEMORY_GROWTH)
MAXMEM=${MAXMEM:-$((0x80000000))}        # 2 GiB (wasm32 limit 4 GiB)
GLOBAL_BASE=${GLOBAL_BASE:-268435456}    # 256 MiB: the image (0x400000..0x1cc0000) lies below
mkdir -p "$OUT/tmp" "$OUT/emcache" "$OUT/out" "$OUT/gen" "$OUT/obj-$VARIANT/rt" "$OUT/obj-$VARIANT/gen" "$OUT/obj-$VARIANT/h" \
    "$OUT/log-$VARIANT" "$OUT/bin-$VARIANT"
export PATH=/opt/emsdk:/opt/emsdk/upstream/emscripten:$PATH
export TMPDIR="$OUT/tmp" EM_CACHE="$OUT/emcache"
cd "$OUT"

SIMD=""     # no -msse2 -msimd128: fle_sse.h no longer includes <emmintrin.h> (docs/decisions/0049-wasm-portability-fixes.md)
CFLAGS_RT="-std=gnu11 -O0 -ffp-contract=off -fno-fast-math -Wall -Wextra"
CFLAGS_GEN="-std=gnu11 -O0 -w -fno-builtin-fprintf -fno-builtin-fputs -ffp-contract=off -fno-fast-math -ferror-limit=0"
[ "$INSTRUMENT" = 1 ] && CFLAGS_GEN="$CFLAGS_GEN -finstrument-functions"

OVERLAY=""
if [ "$MXCSR_OVERRIDE" = 1 ] || { [ "$MXCSR_OVERRIDE" = auto ] && grep -q '__asm__\|asm volatile' "$RT/fle_mxcsr_flags.h"; }; then
    cat > "$OUT/mxcsr-overlay.yaml.tmp" <<EOF
{ 'version': 0, 'case-sensitive': 'true', 'use-external-names': false,
  'roots': [ { 'name': '$RT', 'type': 'directory',
    'contents': [ { 'name': 'fle_mxcsr_flags.h', 'type': 'file', 'external-contents': '$HERE/interim/fle_mxcsr_flags.h' } ] } ] }
EOF
    mv -f "$OUT/mxcsr-overlay.yaml.tmp" "$OUT/mxcsr-overlay.yaml"
    OVERLAY="-ivfsoverlay $OUT/mxcsr-overlay.yaml"
    echo "build: MXCSR interim override ON (runtime/fle_mxcsr_flags.h -> harness/wasm/interim/fle_mxcsr_flags.h)"
else
    echo "build: MXCSR interim override OFF (runtime/fle_mxcsr_flags.h is used as is)"
fi
CC_COMMON="$SIMD $OVERLAY -I$RT $EXTRA_CFLAGS"

stage_gen() {   # fe-lift (built into $OUT/target so lift/target is untouched) -> out/wasm-pdata.c
    [ -e out/wasm-pdata.c ] && { echo "gen: exists"; return; }
    [ -x target/release/fe-lift ] || (cd "$LIFT" && CARGO_TARGET_DIR="$OUT/target" cargo build --release --offline -p fe-lift) || return 1
    target/release/fe-lift lift "$DLL" out/wasm-pdata.c --pdata > gen/gen-pdata.txt 2> gen/gen-pdata.err
}
stage_split() {
    [ -e out/wasm-pdata-part-00.c ] && { echo "split: exists"; return; }
    python3 "$HERE/split_c.py" out/wasm-pdata.c out/wasm-pdata-part 3700
}
compile_one() {   # $1 = source, $2 = object, $3 = log, $4 = flags
    [ -e "$2" ] && return
    /usr/bin/time -f "%e s %M KB" -o "$3.time" emcc -c $4 "$1" -o "$2" > "$3" 2>&1
    echo "$(basename "$1") exit=$? errors=$(grep -c 'error:' "$3") warnings=$(grep -c 'warning:' "$3") $(tail -1 "$3.time")"
}
export -f compile_one
stage_rt() {   # every runtime fle_*.c
    ls "$RT"/fle_*.c | xargs -P "$JOBS" -I{} bash -c 'b=$(basename {} .c); compile_one {} obj-'"$VARIANT"'/rt/$b.o log-'"$VARIANT"'/rt-$b.log "'"$CFLAGS_RT $CC_COMMON"'"' | tee -a "log-$VARIANT/rt-summary.txt"
}
stage_gen_c() {   # the 16 generated parts; at most JOBS emcc at once
    ls out/wasm-pdata-part-*.c | xargs -P "$JOBS" -I{} bash -c 'b=$(basename {} .c); compile_one {} obj-'"$VARIANT"'/gen/$b.o log-'"$VARIANT"'/gen-$b.log "'"$CFLAGS_GEN $CC_COMMON"'"' | tee -a "log-$VARIANT/gen-summary.txt"
}
stage_harness() {   # main, trace, and the native harness image.c (which has an __EMSCRIPTEN__ branch)
    compile_one "$HERE/main.c" obj-$VARIANT/h/main.o log-$VARIANT/h-main.log "-std=gnu11 -O1 -Wall -Wextra $CC_COMMON -I$LIFT/harness"
    compile_one "$HERE/trace.c" obj-$VARIANT/h/trace.o log-$VARIANT/h-trace.log "-std=gnu11 -O1 -Wall -Wextra $CC_COMMON -I$LIFT/harness"
    compile_one "$LIFT/harness/image.c" obj-$VARIANT/h/image.o log-$VARIANT/h-image.log "-std=gnu11 -O1 -Wall -Wextra $CC_COMMON -I$LIFT/harness"
    compile_one "$LIFT/harness/a4.c" obj-$VARIANT/h/a4.o log-$VARIANT/h-a4.log "-std=gnu11 -O1 -Wall -Wextra $CC_COMMON -I$LIFT/harness"
}
stage_link() {
    [ -e "bin-$VARIANT/flengine.wasm" ] && { echo "link: exists"; return; }
    /usr/bin/time -f "link %e s %M KB maxrss" -o "log-$VARIANT/link.time" emcc -O0 \
        -sALLOW_MEMORY_GROWTH=1 -sSTACK_SIZE=8MB -sGLOBAL_BASE="$GLOBAL_BASE" -sENVIRONMENT=node -sMODULARIZE=1 -sEXIT_RUNTIME=1 -sNODERAWFS=1 \
        -Wl,--initial-memory="$INITMEM" -Wl,--max-memory="$MAXMEM" \
        -Wl,--wrap=fprintf -Wl,--wrap=fputs -Wl,--wrap=abort \
        obj-$VARIANT/h/*.o obj-$VARIANT/gen/*.o obj-$VARIANT/rt/*.o -lm \
        -o "bin-$VARIANT/flengine.js" > "log-$VARIANT/link.log" 2>&1
    echo "link exit=$?"; tail -1 "log-$VARIANT/link.time"; ls -la "bin-$VARIANT"
    [ -e "bin-$VARIANT/run_node.cjs" ] || cp "$HERE/run_node.cjs" "bin-$VARIANT/run_node.cjs"
}

STAGES=${*:-gen split rt gen-c harness link}
for s in $STAGES; do echo "== stage $s (variant $VARIANT)"; "stage_${s//-/_}" || { echo "stage $s FAILED"; exit 1; }; done
