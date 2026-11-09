#!/bin/bash
# Decision 0165 (vst3a): reproduction of the Serum 2 run with real FLP states, from the repository root at HEAD 071c53f + vst3a.diff.
#   bash repro.sh gen     # seeds, lift, generated C, units (about 1.5 min, 3 GB of disk)
#   bash repro.sh build   # native -O2 build (about 6 min with JOBS=2 at load 12) and a -O0 one (50 s); BUILD_STATS=1 also builds the -DFLE_MX_STATS=1 -O0 variant
#   bash repro.sh data    # the five states from the FLP, the part of the content that they reference, the VFS seed list
#   bash repro.sh run     # the measured scenarios (one held note C4, states 0,1,3,4,5, 40 blocks) -> $OUT/repro-run.txt
# Environment: REPO (default: the current directory = repository root with the vst3a files), OUT (work directory, default $REPO/lift/out-vst3a-repro), SERUM (Serum2.vst3, default $REPO/lift/out-vst1/bin/Serum2.vst3,
# SHA-256 501e7bb3dd9cafe416b3412df3d4e084c01b7468201e5690b9009d7ecd4e5283), INSTALLER (Install_Xfer_Serum2_2.1.5.exe), FLP (serina1-serum2.flp from serina1-serum2.zip), JOBS (default 2).
set -u -o pipefail
REPO=${REPO:-$PWD}; OUT=${OUT:-$REPO/lift/out-vst3a-repro}; SERUM=${SERUM:-$REPO/lift/out-vst1/bin/Serum2.vst3}; JOBS=${JOBS:-2}; TAG=${TAG:-s2g}
H=$REPO/lift/harness/vst3a; T=$H/tools; V2=$REPO/docs/investigations/vst2/tools
mkdir -p "$OUT"
case ${1:-} in
gen)
    (cd "$REPO/lift" && CARGO_TARGET_DIR=$OUT/target nice -n 15 cargo build --offline --release -p fe-lift -p fe-decode -p fe-pe) || exit 1
    mkdir -p "$OUT/seeds"
    "$OUT/target/release/fe-decode" reloc-seeds "$SERUM" --not-function > "$OUT/seeds/reloc.txt"
    python3 "$V2/nonpdata_seeds.py" "$SERUM" "$OUT/seeds/reloc.txt" "$OUT/seeds/nonpdata.txt"
    python3 "$REPO/lift/harness/seeds/lea_leaf_candidates.py" "$SERUM" > "$OUT/seeds/lea.txt"
    { cat "$OUT/seeds/nonpdata.txt"; awk '{print $1}' "$OUT/seeds/lea.txt"; } | sort -u > "$OUT/seeds/base.txt"
    python3 "$H/rel_table_seeds.py" "$SERUM" "$OUT/seeds/rel.txt" "$OUT/seeds/rel-sites.tsv"
    cat "$OUT/seeds/base.txt" "$OUT/seeds/rel.txt" | sort -u > "$OUT/seeds/base-$TAG.txt"
    cp "$T/seed_fixpoint.sh" "$T/rejected_seeds.py" "$OUT/seeds/"
    FE=$OUT/target/release/fe-lift bash "$OUT/seeds/seed_fixpoint.sh" "$SERUM" "$OUT/seeds/base-$TAG.txt" "$OUT/seeds/fix-$TAG.txt"
    # gen_vst.sh (vst) writes the units to $OUT/src, so $OUT must not be a source tree
    OUT=$OUT TAG=$TAG FILE=$SERUM FEBIN=$OUT/target/release SEEDS=$OUT/seeds/fix-$TAG.txt bash "$H/gen_vst3a.sh"
    ;;
build)
    OUT=$OUT TAG=$TAG FILE=$SERUM OPT=O2 BUILTIN=1 COUNT=0 JOBS=$JOBS nice -n 15 bash "$H/build_vst3a_native.sh"          # $OUT/native/$TAG-O2-b1-c0/vst-native (timing)
    OUT=$OUT TAG=$TAG FILE=$SERUM OPT=O2 BUILTIN=1 COUNT=1 JOBS=$JOBS nice -n 15 bash "$H/build_vst3a_native.sh"          # ...-c1 (instruction counts per function)
    OUT=$OUT TAG=$TAG FILE=$SERUM OPT=O0 BUILTIN=1 COUNT=0 JOBS=$JOBS nice -n 15 bash "$H/build_vst3a_native.sh"          # ...-O0-b1-c0 (debugging)
    [ "${BUILD_STATS:-0}" = 1 ] && XCF=-DFLE_MX_STATS=1 OUT=$OUT TAG=$TAG FILE=$SERUM OPT=O0 BUILTIN=1 COUNT=0 JOBS=$JOBS nice -n 15 bash "$H/build_vst3a_native.sh"   # ...-O0-b1-c0-x (fast/slow counts)
    ;;
data)
    INSTALLER=${INSTALLER:?} FLP=${FLP:?}
    python3 "$T/extract_states.py" "$FLP" "$OUT/state"
    bash "$T/extract_content.sh" "$INSTALLER" "$OUT/content"
    python3 "$T/mk_seed.py" "$OUT/content" "$OUT/seed-content.tsv"
    cat "$REPO/lift/harness/vst/vfs-seed-vst.tsv" "$OUT/seed-content.tsv" > "$OUT/seed-with-content.tsv"
    ;;
run)
    B=$OUT/native/$TAG-O2-b1-c0/vst-native
    export FLE_VFS_SEED=$OUT/seed-with-content.tsv
    : > "$OUT/repro-run.txt"
    for s in 0 1 3 4 5; do
        extra=""; [ $s = 5 ] && extra="--state-proc-only"    # state 5: the controller's setComponentState dereferences a NULL (decision 0165, section 4.3)
        echo "== state $s, one held note C4 (velocity 100), 40 blocks of 4096 frames" >> "$OUT/repro-run.txt"
        nice -n 15 "$B" "$SERUM" --blocks 40 --note --state $s --state-dir "$OUT/state" $extra --presets-dir 'C:\SerumData\Presets' 2>&1 | grep -a "^RESULT blocks\|^RESULT out_hash\|setState" >> "$OUT/repro-run.txt"
    done
    echo "== six instances in one process (states 0,1,3,4,-1 [initial patch],5), one note each"  >> "$OUT/repro-run.txt"
    nice -n 15 "$B" "$SERUM" --blocks 12 --note --multi 0,1,3,4,-1,5 --state-dir "$OUT/state" --presets-dir 'C:\SerumData\Presets' 2>&1 | grep -a "RESULT multi" >> "$OUT/repro-run.txt"
    echo "== diagnostics (output NOT the plugin's): RC bits of the guest MXCSR cleared after every ldmxcsr, state 3"  >> "$OUT/repro-run.txt"
    VST3A_DIAG_GUEST_RC0=1 nice -n 15 "$B" "$SERUM" --blocks 20 --note --state 3 --state-dir "$OUT/state" --presets-dir 'C:\SerumData\Presets' 2>&1 | grep -a "^RESULT blocks" >> "$OUT/repro-run.txt"
    cat "$OUT/repro-run.txt"
    ;;
*) sed -n 2,9p "$0"; exit 64;;
esac
