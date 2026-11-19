#!/bin/bash
# Decision 0177 (srm1): seeds + lift + units of the REBASED Serum2.vst3 (ImageBase 0x2000000, below the wasm GLOBAL_BASE). The same steps as tools/repro.sh gen (decision 0165) with another file.
# usage (repository root, release binaries of fe-lift/fe-decode/fe-pe built into $FE):
#   $FE/fe-pe rebase lift/out-vst1/bin/Serum2.vst3 0x2000000 $OUT/../rebased/Serum2.2000000.vst3
#   OUT=<dir> FE=<dir with fe-lift> bash lift/harness/vst3a/tools/gen_wasm.sh        -> $OUT/gen/s2w.c (SHA-256 c90fd52c..., 43,918 functions), $OUT/src/s2w-NN.c, common-s2w.c, $OUT/rows/
# With SERUM=<the original PE> TAG=s2g the same script reproduces decision 0165's generated C (SHA-256 5a88d6cb...).
set -u -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
S=$(cd "$HERE/../../../.." && pwd)            # repository root (the tree with these files)
H=$S/lift/harness/vst3a; T=$H/tools; V2=$S/docs/investigations/vst2/tools
OUT=${OUT:?OUT (work directory, must not be a source tree) is not set}; TAG=${TAG:-s2w}; FE=${FE:?FE (directory with fe-lift, fe-decode) is not set}
SERUM=${SERUM:-$OUT/../rebased/Serum2.2000000.vst3}
mkdir -p "$OUT/seeds"
$FE/fe-decode reloc-seeds "$SERUM" --not-function > "$OUT/seeds/reloc.txt"
python3 "$V2/nonpdata_seeds.py" "$SERUM" "$OUT/seeds/reloc.txt" "$OUT/seeds/nonpdata.txt"
python3 "$S/lift/harness/seeds/lea_leaf_candidates.py" "$SERUM" > "$OUT/seeds/lea.txt"
{ cat "$OUT/seeds/nonpdata.txt"; awk '{print $1}' "$OUT/seeds/lea.txt"; } | sort -u > "$OUT/seeds/base.txt"
python3 "$H/rel_table_seeds.py" "$SERUM" "$OUT/seeds/rel.txt" "$OUT/seeds/rel-sites.tsv"
cat "$OUT/seeds/base.txt" "$OUT/seeds/rel.txt" | sort -u > "$OUT/seeds/base-$TAG.txt"
cp "$T/seed_fixpoint.sh" "$T/rejected_seeds.py" "$OUT/seeds/"
FE=$FE/fe-lift bash "$OUT/seeds/seed_fixpoint.sh" "$SERUM" "$OUT/seeds/base-$TAG.txt" "$OUT/seeds/fix-$TAG.txt"
OUT=$OUT TAG=$TAG FILE=$SERUM FEBIN=$FE SEEDS=$OUT/seeds/fix-$TAG.txt LIFT=$S/lift bash "$H/gen_vst3a.sh"
