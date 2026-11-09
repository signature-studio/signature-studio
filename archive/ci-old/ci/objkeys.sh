#!/bin/bash
# ci/objkeys.sh [OUTFILE] - the cache keys of the wasm OBJECT groups, from the CONTENT of the generated C they are compiled from (so they need the lift output: the workflow runs this after the lift output
# is in place). A group's key = hash(its translation units' bytes, the build inputs of the wasm stage). When only the dsp module's C changes, the other groups keep their keys and their objects.
#   groups: fle (FLEngine: wsrc/src/fle-NN.c), m1 dsp_ippv2, m2 Blend2D, m3 QuickFontCache, m4 FreeType, m5 ILTools, oggio and Elastique (m5 holds m5-m7: wsrc/v-<tag>/src/mK-NN.c), rt (runtime fle_*.c, the harness objects, common.c, realmod table)
# env: the values of ci/plan.sh: PLAN_H_WASM, PLAN_TOOLS, PLAN_SP_WASM, PLAN_K_B2D (the workflow passes plan.sh's outputs). The rt group holds the Blend2D adapter object, which is compiled against the upstream headers: it carries PLAN_K_B2D.
set -u -o pipefail
. "$(dirname "$0")/lib/common.sh"
OUT=${1:-/dev/stdout}
: "${PLAN_H_WASM:?}" "${PLAN_TOOLS:?}"
W=$ENGINE_WORK/wsrc
tagdir=$(ls -d "$W"/v-* 2>/dev/null | head -1); [ -n "$tagdir" ] || die "no $W/v-*: the lift output is not in place"
grp() { # NAME FILES...
  local n=$1; shift
  echo "ko_$n=fl-obj-$n-$(sha_str "|$n|$PLAN_H_WASM|$PLAN_TOOLS|${PLAN_SP_WASM:-}|$(cat "$@" | sha256sum | cut -d' ' -f1)" | cut -c1-40)"
}
{
  # shellcheck disable=SC2046
  grp fle $(ls "$W"/src/fle-[0-9][0-9].c)
  for k in 1 2 3 4 5; do
    # shellcheck disable=SC2046
    grp "m$k" $(ls "$tagdir"/src/m$k-[0-9][0-9].c $( [ "$k" = 5 ] && ls "$tagdir"/src/m[6-9]-[0-9][0-9].c 2>/dev/null ))
  done
  PLAN_SP_WASM="${PLAN_SP_WASM:-}|b2d=${PLAN_K_B2D:-}" grp rt "$tagdir/src/common.c" "$(ls -d "$ENGINE_WORK"/onat/v-*/native)/realmod_table_gen.c" "$tagdir/modules.txt"
} > "$OUT"
