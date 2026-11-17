#!/bin/bash
# Two-module build, shared part: rebase the real module (decision 0080 D2), lift both files, split both generated C files into
# translation units that share R/FL/X, and merge the two dispatch tables into one common.c (docs/investigations/two-module-design.md
# section 4; the tools are the ones of lift/crates/fe-lift/tests/realmod_s1/tools, used unchanged).
# The generator is not changed: the FLEngine C is the one `fe-lift lift FLEngine_x64.dll OUT.c --pdata` writes, and this script
# checks its SHA-256 against EXPECT_FLE_SHA.
#
# The FLEngine side ($OUT/gen, $OUT/src/fle-*) does not depend on the real module and is shared by every TAG. Everything that
# depends on the real module (its generated C, units, merged table) lives in $OUT/v-$TAG/, so a different seed set is a new TAG and
# nothing is overwritten.
#
# N real modules: MODULES="label|file|base|lift-args|expect-sha256-of-file|funcs-per-part;label|file|base|..."   one entry per real
#   module, ';' between entries, '|' between fields (lift-args may hold spaces, e.g. "--seeds X" or "--pdata --seeds X"; expect-sha
#   and funcs-per-part may be empty). The label "dll" keeps the file names of the one-module script (gen/real.c, src/dll-NN.c,
#   seeds.txt); any other label L gives gen/L.c, src/L-NN.c, seeds-L.txt. Without MODULES the one entry
#   "dll|$REAL|$BASE|[--seeds $SEEDS]|$EXPECT_REAL_SHA|$REAL_PER_PART" is used, which is what the one-module script did.
#   The merged table is made by merge_tables_n.py (N tables; the output for two tables is the one of the earlier merge_tables.py) and
#   the list of modules is written to $V/modules.txt for build_two_native.sh. The bases are measurement values given by the caller;
#   nothing here checks them (fle_realmod.c does, when the guest first looks a module up).
#
# usage: bash lift/harness/two/gen_two.sh [stage...]      stages: rebase gen split merge   (default: all, in that order)
# env:   OUT=lift/out-stage2/two   TAG=red   FEBIN=<dir with fe-lift and fe-pe>   (default $LIFT/out-stage2/target/release)
#        LIFT=<tree with crates/ and runtime/>   (default: the lift/ above this script's directory)
#        FLE=binaries/win64/FLEngine_x64.dll      REAL=lift/out-ext/dsp_ippv2_x64.dll      BASE=0x2000000
#        SEEDS=<file>   extra seeds of the real module, one VA per line, already at BASE (default: none = the closure of the entry
#                       and the 446 exports)
#        FLE_SEEDS=<file>   extra seeds of FLEngine (fe-lift --seeds, decision 0126; lift/harness/seeds/flengine-seeds-thunks.txt: import thunks that the closure and .pdata do not reach). Without it the FLEngine C is the old one.
#        FLE_HOOKS=<file>   inline hook points of FLEngine (fe-lift --hook-points, decision 0105; lift/harness/hooks/flengine-hook-points.txt).
#                       The FLEngine C is then not the old one (EXPECT_FLE_SHA=skip, or the new value); gen/flengine.c is reused when
#                       present, so use a new OUT. The hook points of a real module go into its lift-args (--hook-points FILE).
#        FLE_SEH_HANDLERS=<hex RVA[,hex RVA...]>   the Delphi exception handler(s) of FLEngine (fe-lift --seh-handler, decision 0132; 0x112c0). Without it the FLEngine C is the old one.
#        FLE_EXTRA_IMPORTS=<file>   extra import rows of FLEngine (fe-lift --extra-imports, decisions 0156 and 0171; lift/harness/seeds/ia1-imports.txt: the user32/comctl32/... APIs that the main window's menus, clipboard and toolbar reach). Without it the FLEngine C is the old one.
#        FLE_PER_PART=3700   REAL_PER_PART=800   EXPECT_FLE_SHA=feb5b908... (the value at HEAD dc27f4d; it changes with the response
#                       table; EXPECT_FLE_SHA=skip prints the SHA-256 and does not check it)
#        DLLDIR=dll   directory (under OUT) of the rebased files; a4-multi.sh sets it per TAG so that two module lists cannot share a
#                     rebased file of the same name and base
#        FRUITY_ROWS=<file>   extra rows of the MERGED dispatch table (merge_tables_n.py --extra-rows; lift/harness/fruity-rows.txt): the three stubs
#                     of the fake host object of CreateFruityInstance (docs/investigations/create-fruity-instance-harness.md). Default: none, and
#                     then common.c is the one of the version of this script without the variable. a4-multi.sh sets it for FLE_CALL_FRUITY=1 (and
#                     uses another TAG for it, so the two common.c never share a directory).
# Nothing is deleted or overwritten: a stage whose output exists is skipped (to redo it, move the output away).
set -u -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=${LIFT:-$(cd "$HERE/../.." && pwd)}
TOOLS="$LIFT/crates/fe-lift/tests/realmod_s1/tools"
OUT=${OUT:-$LIFT/out-stage2/two}
TAG=${TAG:-red}
FEBIN=${FEBIN:-$LIFT/out-stage2/target/release}
FLE=${FLE:-$LIFT/../binaries/win64/FLEngine_x64.dll}
REAL=${REAL:-$LIFT/out-ext/dsp_ippv2_x64.dll}
BASE=${BASE:-0x2000000}
SEEDS=${SEEDS:-}
FLE_PER_PART=${FLE_PER_PART:-3700}
REAL_PER_PART=${REAL_PER_PART:-800}
EXPECT_FLE_SHA=${EXPECT_FLE_SHA:-feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f}
EXPECT_REAL_SHA=${EXPECT_REAL_SHA:-497dd4f77e859df9e86d999b3811be3c72526b41f86c38b0c3c6e395f691c352}
DLLDIR=${DLLDIR:-dll}
SEHARGS=()
if [ -n "${FLE_SEH_HANDLERS:-}" ]; then
    IFS=',' read -ra _sh <<< "$FLE_SEH_HANDLERS"
    for _h in "${_sh[@]}"; do SEHARGS+=(--seh-handler "$_h"); done
fi
FRUITY_ROWS=${FRUITY_ROWS:-}
V="$OUT/v-$TAG"
mkdir -p "$OUT/tmp" "$OUT/gen" "$OUT/dll" "$OUT/src" "$OUT/log" "$V/gen" "$V/src" "$V/log" "$OUT/$DLLDIR"
export TMPDIR="$OUT/tmp"
cd "$OUT"

if [ -z "${MODULES:-}" ]; then
    LARGS=""
    [ -n "$SEEDS" ] && LARGS="--seeds $SEEDS"
    MODULES="dll|$REAL|$BASE|$LARGS|$EXPECT_REAL_SHA|$REAL_PER_PART"
fi
SPECS=()
IFS=';' read -ra SPECS <<< "$MODULES"

# sets M_label M_real M_base M_largs M_sha M_per M_sfx M_stem M_rebased M_gen M_pre for one entry of MODULES
parse_mod() {
    IFS='|' read -r M_label M_real M_base M_largs M_sha M_per <<< "$1"
    M_per=${M_per:-$REAL_PER_PART}
    if [ "$M_label" = dll ]; then M_sfx=""; M_stem=real; else M_sfx="-$M_label"; M_stem="$M_label"; fi
    M_rebased="$DLLDIR/$(basename "$M_real" .dll).$(printf '%x' "$M_base").dll"
    M_gen="$V/gen/$M_stem.c"
    M_pre="$V/src/$M_label"
}

stage_rebase() {
    local spec rlog
    for spec in "${SPECS[@]}"; do
        parse_mod "$spec"
        [ -e "$M_rebased" ] && { echo "rebase[$M_label]: exists"; continue; }
        if [ -n "$M_sha" ]; then
            [ "$(sha256sum "$M_real" | cut -d' ' -f1)" = "$M_sha" ] || { echo "rebase[$M_label]: $M_real is not the expected file (sha256)"; return 1; }
        fi
        rlog="log/rebase$M_sfx.txt"
        [ "$DLLDIR" = dll ] || rlog="$V/log/rebase$M_sfx.txt"
        "$FEBIN/fe-pe" rebase "$M_real" "$M_base" "$M_rebased" | tee "$rlog"
    done
}
stage_gen() {
    local spec s
    if [ ! -e gen/flengine.c ]; then
        "$FEBIN/fe-lift" lift "$FLE" gen/flengine.c --pdata ${FLE_SEEDS:+--seeds "$FLE_SEEDS"} ${FLE_HOOKS:+--hook-points "$FLE_HOOKS"} ${FLE_EXTRA_IMPORTS:+--extra-imports "$FLE_EXTRA_IMPORTS"} ${SEHARGS[@]+"${SEHARGS[@]}"} > log/gen-flengine.txt 2> log/gen-flengine.err || return 1
    else echo "gen: flengine.c exists"; fi
    got=$(sha256sum gen/flengine.c | cut -d' ' -f1)
    if [ "$EXPECT_FLE_SHA" = skip ]; then
        echo "gen: FLEngine C sha256 $got (EXPECT_FLE_SHA=skip: not checked)"
    else
        [ "$got" = "$EXPECT_FLE_SHA" ] || { echo "gen: FLEngine C sha256 $got differs from $EXPECT_FLE_SHA"; return 1; }
        echo "gen: FLEngine C sha256 $got (expected)"
    fi
    [ -e "$V/modules.txt" ] || printf '%s\n' "${MODULES//;/$'\n'}" > "$V/modules.txt"
    for spec in "${SPECS[@]}"; do
        parse_mod "$spec"
        if [ ! -e "$M_gen" ]; then
            [ -e "$M_rebased" ] || { echo "gen: run the rebase stage first"; return 1; }
            "$FEBIN/fe-lift" lift "$M_rebased" "$M_gen" $M_largs > "$V/log/gen-$M_stem.txt" 2> "$V/log/gen-$M_stem.err" || return 1
            if [[ "$M_largs" == *--seeds* ]]; then
                s=${M_largs#*--seeds }; s=${s%% *}
                cp -n "$s" "$V/seeds$M_sfx.txt"
            fi
        else echo "gen: v-$TAG/$M_stem.c exists"; fi
        echo "gen[$M_label]: $M_gen sha256 $(sha256sum "$M_gen" | cut -d' ' -f1)"
        sed -n 1,9p "$V/log/gen-$M_stem.txt"
    done
}
stage_split() {
    local spec
    if [ -e src/fle-00.c ]; then echo "split: fle exists"; else
        python3 "$TOOLS/split_mod.py" gen/flengine.c src/fle "$FLE_PER_PART" || return 1
    fi
    for spec in "${SPECS[@]}"; do
        parse_mod "$spec"
        if [ -e "${M_pre}-00.c" ]; then echo "split: v-$TAG $M_label exists"; else
            python3 "$TOOLS/split_mod.py" "$M_gen" "$M_pre" "$M_per" || return 1
        fi
    done
}
stage_merge() {
    local spec tabs=(src/fle-tables.txt) xr=()
    [ -e "$V/src/common.c" ] && { echo "merge: v-$TAG exists"; return; }
    for spec in "${SPECS[@]}"; do
        parse_mod "$spec"
        tabs+=("${M_pre}-tables.txt")
    done
    if [ -n "$FRUITY_ROWS" ]; then
        [ -r "$FRUITY_ROWS" ] || { echo "merge: cannot read FRUITY_ROWS=$FRUITY_ROWS"; return 1; }
        xr=(--extra-rows "$FRUITY_ROWS")
    fi
    python3 "$HERE/merge_tables_n.py" "$V/src/common.c" "${tabs[@]}" ${xr[@]+"${xr[@]}"} | tee "$V/log/merge.txt"
}

STAGES=${*:-rebase gen split merge}
for s in $STAGES; do echo "== stage $s"; "stage_$s" || { echo "stage $s FAILED"; exit 1; }; done
