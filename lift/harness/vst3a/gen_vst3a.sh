#!/bin/bash
# Generation stage for Serum2.vst3 (decision 0165): rows + stubs, then lift + split + common.c with the vst harness's gen_vst.sh (unchanged).
# usage: OUT=lift/out-vst3a TAG=s2a FILE=<Serum2.vst3> FEBIN=<dir with fe-lift> SEEDS=<seeds file> bash lift/harness/vst3a/gen_vst3a.sh
set -u -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=${LIFT:-$(cd "$HERE/../.." && pwd)}
OUT=${OUT:?}; TAG=${TAG:?}; FILE=${FILE:?}; FEBIN=${FEBIN:?}; SEEDS=${SEEDS:?}
mkdir -p "$OUT/rows" "$OUT/log" "$OUT/gen"
python3 "$HERE/mk_stubs.py" "$FILE" "$LIFT/crates/fe-lift/src/imports_table.rs" "$LIFT/harness/vst/imports-vst.txt" "$OUT/rows/stub-rows-$TAG.txt" "$OUT/rows/vst3a_stubs.c" "$OUT/rows/ord-rows-$TAG.txt" || exit 1
cat "$LIFT/harness/vst/imports-vst.txt" "$OUT/rows/stub-rows-$TAG.txt" > "$OUT/rows/imports-$TAG.txt"
bash "$LIFT/harness/vst/mkrows.sh" "$OUT/rows/host-rows.txt"
cat "$OUT/rows/ord-rows-$TAG.txt" >> "$OUT/rows/host-rows.txt"
printf '0x7ffc30000010 vst3a_thread_entry\n' >> "$OUT/rows/host-rows.txt"
k=0; for n in msg_qi msg_addref msg_release msg_getMessageID msg_setMessageID msg_getAttributes attr_qi attr_addref attr_release attr_setInt attr_getInt attr_setFloat attr_getFloat attr_setString attr_getString attr_setBinary attr_getBinary; do printf '0x%x vst3a_%s\n' $((0x7FFC40000000 + 0x10 * (k + 1))) $n >> "$OUT/rows/host-rows.txt"; k=$((k + 1)); done
k=0; for n in qi addref release read write seek tell; do printf '0x%x vst3a_stream_%s\n' $((0x7FFC20000000 + 0x10 * (k + 1))) $n >> "$OUT/rows/host-rows.txt"; k=$((k + 1)); done
mkdir -p "$OUT/gen"
if [ ! -e "$OUT/gen/$TAG.c" ]; then
    "$FEBIN/fe-lift" lift "$FILE" "$OUT/gen/$TAG.c" --pdata --seeds "$SEEDS" --extra-imports "$OUT/rows/imports-$TAG.txt" > "$OUT/log/gen-$TAG.txt" 2> "$OUT/log/gen-$TAG.err" || exit 1
    python3 "$HERE/patch_ordinals.py" "$FILE" "$OUT/gen/$TAG.c" "$OUT/rows/ord-rows-$TAG.txt.slots" | tee -a "$OUT/log/gen-$TAG.txt"
    python3 "$HERE/patch_shd.py" "$FILE" "$OUT/gen/$TAG.c" | tee -a "$OUT/log/gen-$TAG.txt"
    python3 "$HERE/patch_bmi.py" "$FILE" "$OUT/gen/$TAG.c" "$FEBIN" "$OUT/gen" "$TAG" --pdata --seeds "$SEEDS" --extra-imports "$OUT/rows/imports-$TAG.txt" | tee -a "$OUT/log/gen-$TAG.txt"
fi
OUT="$OUT" TAG="$TAG" FILE="$FILE" FEBIN="$FEBIN" LIFT_ARGS="--pdata --seeds $SEEDS --extra-imports $OUT/rows/imports-$TAG.txt" ROWS="$OUT/rows/host-rows.txt" PER=${PER:-4000000} \
    bash "$LIFT/harness/vst/gen_vst.sh"
