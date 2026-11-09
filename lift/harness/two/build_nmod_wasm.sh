#!/bin/bash
# The wasm build of the lifted engine, shared part (decision 0243: one configuration, no switch): the units made by gen_two.sh for FLEngine + the real modules (the list of modules.txt) + the runtime + the realmod table of that list +
# the FRUITY harness objects (the stage-1 harness of CreateFruityInstance in wasm, decision 0128), compiled with the flags below.
#   - the guest's setjmp landing uses the emscripten setjmp/longjmp as Wasm exception handling instructions (-sSUPPORT_LONGJMP=wasm, no JS frame), the cooperative guest threads switch by JSPI (runtime/fle_thread_wasm.c);
#   - the generated C is compiled at -Oz without entry hooks and with the compiler's builtins (decision 0151; the two calls that the harness intercepts at link time, fprintf and fputs, stay real calls), the runtime at -O2 (decision 0226),
#     the harness objects at -O1;
#   - AI_UNITS: the units named there (fle-00 m1-36 ...) are compiled with -DFLE_ALWAYS_INLINE (lift/runtime/fle_ai.h: the small static inline helpers of the runtime headers become always_inline; decision 0161);
#   - every object is compiled with the EMCC_CFLAGS of the caller (app/scripts/build-engine-bundle.sh wasm_env: WebAssembly SIMD, atomics and bulk memory).
# One emcc per unit under `ulimit -v $EMCC_VMEM_KB`, JOBS at a time.
#
# usage: OUT=<gen_two OUT> TAG=<tag> REALMOD_C=<realmod_table_gen.c> OBJ=<object directory> AI_UNITS="..." bash lift/harness/two/build_nmod_wasm.sh [stage...]       stages: rt harness units   (default: all, in that order)
# Output: $OBJ/{rt,h-fruity,fle,mod}/*.o; logs $OBJ/log. Nothing is overwritten: an existing object is skipped.
set -u -o pipefail
export PYTHONDONTWRITEBYTECODE=1
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/../.." && pwd)
RT="$LIFT/runtime"
OUT=${OUT:?OUT is not set}
TAG=${TAG:?TAG is not set}
REALMOD_C=${REALMOD_C:?REALMOD_C is not set}
JOBS=${JOBS:-2}
EMCC_VMEM_KB=12582912
V="$OUT/v-$TAG"
OBJ=${OBJ:-$OUT/wasm-obj}
mkdir -p "$OBJ"/{rt,h-fruity,fle,mod,log,bin} "$OUT/tmp"
export PATH=/opt/emsdk:/opt/emsdk/upstream/emscripten:$PATH
export TMPDIR="$OUT/tmp" EM_CACHE="${EM_CACHE:-$OUT/emcache}"
SJ="-sSUPPORT_LONGJMP=wasm"
CFLAGS_RT="-std=gnu11 -O2 -ffp-contract=off -fno-fast-math -Wall -Wextra -I$RT $SJ"
CFLAGS_GEN="-std=gnu11 -Oz -w -fno-builtin-fprintf -fno-builtin-fputs -ffp-contract=off -fno-fast-math -ferror-limit=0 -I$RT $SJ"
CFLAGS_H="-std=gnu11 -O1 -Wall -Wextra -I$RT -I$LIFT/harness $SJ"
export OBJ EMCC_VMEM_KB CFLAGS_GEN AI_UNITS=${AI_UNITS:-}

compile_one() {   # $1 source, $2 object, $3 log, $4 flags
    [ -e "$2" ] && return 0
    ( ulimit -v "$EMCC_VMEM_KB"; /usr/bin/time -f "%e s %M KB" -o "$3.time" emcc -c $4 "$1" -o "$2" > "$3" 2>&1 )
    local rc=$?
    echo "$(basename "$1") exit=$rc errors=$(grep -c 'error:' "$3") $(tail -1 "$3.time" 2>/dev/null)"
    return $rc
}
export -f compile_one

unit() {   # $1 source, $2 destination directory of the object
    local src=$1 dest=$2 b fl="$CFLAGS_GEN"
    b=$(basename "$src" .c)
    case " ${AI_UNITS:-} " in *" $b "*) fl="$fl -DFLE_ALWAYS_INLINE" ;; esac
    compile_one "$src" "$dest/$b.o" "$OBJ/log/$b.log" "$fl"
}
export -f unit

stage_rt() {
    local f b
    for f in "$RT"/fle_*.c; do
        b=$(basename "$f" .c)
        compile_one "$f" "$OBJ/rt/$b.o" "$OBJ/log/rt-$b.log" "$CFLAGS_RT" || return 1
    done
}
stage_harness() {
    local hf="$OBJ/h-fruity" HL="$OBJ/log/h-fruity"
    compile_one "$HERE/main_two.c" "$hf/main_two.o" "$HL-main_two.log" "$CFLAGS_H -DFLE_FRUITY=1" || return 1
    compile_one "$LIFT/harness/fruity.c" "$hf/fruity.o" "$HL-fruity.log" "$CFLAGS_H" || return 1
    compile_one "$LIFT/harness/fruity_surface.c" "$hf/fruity_surface.o" "$HL-fruity_surface.log" "$CFLAGS_H" || return 1
    compile_one "$LIFT/harness/fruity_idle.c" "$hf/fruity_idle.o" "$HL-fruity_idle.log" "$CFLAGS_H" || return 1
    compile_one "$LIFT/harness/fruity_g3.c" "$hf/fruity_g3.o" "$HL-fruity_g3.log" "$CFLAGS_H" || return 1
    compile_one "$LIFT/harness/wasm/trace_fruity.c" "$hf/trace.o" "$HL-trace.log" "$CFLAGS_H" || return 1
    compile_one "$LIFT/harness/web/jspi/tools/ra0.c" "$hf/ra0.o" "$HL-ra0.log" "$CFLAGS_H" || return 1
    compile_one "$LIFT/harness/image.c" "$hf/image.o" "$HL-image.log" "$CFLAGS_H" || return 1
    compile_one "$LIFT/harness/a4.c" "$hf/a4.o" "$HL-a4.log" "$CFLAGS_H" || return 1
    compile_one "$REALMOD_C" "$hf/realmod_table.o" "$HL-realmod_table.log" "$CFLAGS_H" || return 1
    compile_one "$LIFT/harness/hyb1/hybrid_glue.c" "$hf/hybrid_glue.o" "$HL-hybrid_glue.log" "$CFLAGS_H" || return 1   # the glue that the direct wasm of fe-wasmgen needs (fle_hyb_dispatch ...)
}
stage_units() {
    local f
    compile_one "$V/src/common.c" "$OBJ/mod/common.o" "$OBJ/log/common.log" "$CFLAGS_GEN" || return 1
    {
        for f in "$OUT"/src/fle-[0-9][0-9].c; do [ -e "$f" ] && echo "$f $OBJ/fle"; done
        for f in "$V"/src/m[0-9]*-[0-9][0-9].c; do [ -e "$f" ] && echo "$f $OBJ/mod"; done
    } | xargs -P "$JOBS" -L 1 bash -c 'unit "$0" "$1"' || return 1
}
STAGES=${*:-rt harness units}
for s in $STAGES; do echo "== stage $s (tag $TAG)"; "stage_$s" || { echo "stage $s FAILED"; exit 1; }; done
