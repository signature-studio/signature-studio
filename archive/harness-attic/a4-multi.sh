#!/bin/bash
# A4 on a list of real modules, in one command: rebase, lift, split, merge, build, run, and the a4 lines on stdout.
# FLEngine alone (0 modules, a4-survey.sh with the single-module build), FLEngine + 1 real module (2-module build), + 2 (3-module
# build), ... The harness it drives is lift/harness/two/ (gen_two.sh, build_two_native.sh, main_two.c, two-native-run.sh) and, with
# no module, lift/harness/wasm/build_native.sh; the run itself is lift/harness/a4-survey.sh, which is not changed.
#
# usage: OUT=/abs/dir FLE=/abs/FLEngine_x64.dll bash lift/harness/a4-multi.sh [NAME:DLL:BASE[:SEEDS[:OPTS[:HOOKS[:SITES]]]] ...]
#   NAME   the name the guest's LoadLibraryW gives after normalisation (lower case, e.g. blend2d.dll): the entry of the
#          fle_realmod_table. Nothing here checks it: lift/runtime/fle_realmod.c does, at the first lookup.
#   DLL    absolute path of the original (not rebased) file. It is rebased with `fe-pe rebase DLL BASE` into $OUT/v-$TAG/dll/.
#   BASE   0x-prefixed hex, given to `fe-pe rebase` and to the realmod table. There is NO default: the bases used in the
#          investigations (dsp_ippv2 0x2000000, Blend2D 0x4700000) are measurement values, not decisions.
#   SEEDS  optional file, one VA per line at BASE (`fe-lift lift --seeds`). Empty field = none.
#   OPTS   optional; only `pdata` (`fe-lift lift --pdata`). No SEEDS and no OPTS = the closure of the entry and the exports.
#   HOOKS  optional file of inline hook points of the module (`fe-lift lift --hook-points`, decision 0105; one VA or `rva:` + hex RVA per
#          line, e.g. lift/harness/hooks/qfc-hook-points.txt). Empty field = none. The module then registers its points with the runtime.
#   SITES  optional file of call-site memory overrides of the module (`fe-lift lift --site-overrides`, decision 0120; one `ADDR store32 REG[+DISP] EXPECT NEW` per
#          line, e.g. lift/harness/hooks/blend2d-no-jit-sites.txt). Empty field = none, and the generated C is then the one made without the field, byte for byte.
#   Paths and names must not hold ':' ';' '|' or white space (the field separators of this script and of gen_two.sh).
#   At most as many real modules as fle_realmod.c holds (RM_MAX, 8 since decision 0106): more is stopped by the runtime by name, not here. A base that
#   fle_realmod.c refuses is stopped by it, unless `fe-pe rebase` or the image loader of the harness refuses it first.
# env:   OUT=<absolute dir>   required. Never deleted or overwritten: shared objects and a TAG's build are reused when present, every
#                             invocation writes its own $OUT/runs/<UTC time>-<pid>/ . An existing directory that a4-multi.sh did
#                             not make, or one made from different sources/tools (stamp.txt), is refused and nothing is written.
#        FLE=<FLEngine_x64.dll>   (default $LIFT/../binaries/win64/FLEngine_x64.dll)
#        LIFT=<tree with crates/ runtime/ harness/>   (default: the lift/ above this script; a `git archive` tree works)
#        FEBIN=<dir with fe-lift and fe-pe>   (default $OUT/target/release, built with cargo --offline when missing)
#        JOBS=6   TAG=<name>   (default: a hash of the module list, so another list never reuses this one's build)
#        FLE_SEEDS=<abs file>   (decision 0126) extra seeds of FLEngine (fe-lift lift --seeds; one VA per line; lift/harness/seeds/flengine-seeds-thunks.txt). Without it FLEngine is lifted as before. The file is part of the stamp.
#        FLE_HOOKS=<abs file>   inline hook points of FLEngine (lift/harness/hooks/flengine-hook-points.txt; decision 0105). Without it
#                               FLEngine is lifted as before. The generated C then differs from the old one (EXPECT_FLE_SHA=skip, or the new
#                               value), and the file is part of the stamp, so an OUT made without it is refused.
#        DSP_SEEDS=<abs file>   (decision 0127) the seeds of the module named dsp_ippv2_x64.dll in the FRUITY build (decision 0126 wall 97: the case bodies of the IPP jump tables;
#                               lift/harness/seeds/dsp-ippv2-2000000-seeds-jt.txt). It REPLACES the SEEDS field of that module (so does `run6.sh` of the decision 0126 work tree, which gave
#                               ${DSP_SEEDS:-<the seeds of decision 0113>} as that field). Only with FLE_CALL_FRUITY=1 (otherwise refused); the same checks as FLE_SEEDS (absolute, readable, no
#                               quote, space, ; or |); a module named dsp_ippv2_x64.dll must be in the list. The file is part of the stamp (an OUT made without it is refused) and, as a seeds
#                               file of a module, of the TAG.
#        VFS_SEED=<abs file>   (decision 0110, provisional) a list of product files of the installer that the harness writes to the virtual
#                               volume before the guest runs (lift/harness/seeds/vfs-seed-fonts.tsv; format and rules: lift/harness/vfs_seed.h).
#                               Without it the volume is empty, as before (decision 0018), and the output of this script is unchanged. It is a
#                               run-time input: not part of the stamp or of the TAG (the build is shared with the run without it), copied to
#                               $RUN/vfs-seed.tsv, and reported ("a4-multi: VFS_SEED=..." and a "---- vfs seed" block). Needs at least one real module
#                               (the 0-module build does not read it: stopped here). The variables FLE_VFS_SEED and FLE_VFS_SEED_ROOT of the caller are
#                               ignored (unset here): only VFS_SEED / VFS_SEED_ROOT count.
#        VFS_SEED_ROOT=<abs dir>   what the relative host paths of the list are relative to (default: the parent of $LIFT, the repository root;
#                               the fonts of the default list are in lift/out-inventory/extracted/fonts/, written by tools/extract_nsis2.py).
#        FLE_SEH_HANDLERS=<hex RVA[,hex RVA...]>   (decision 0132, provisional) the RVAs of the guest's Delphi exception handler(s) of FLEngine (0x112c0): fe-lift lift --seh-handler. The except landings of the
#                               functions that name such a handler become function entries and those functions get the entry wrapper that RtlUnwindEx jumps back to (runtime/fle_seh.c). Without it FLEngine is lifted as
#                               before, byte for byte. The value is part of the stamp (an OUT made without it is refused).
#        FLE_CALL_FRUITY=1   (stage 1 of CreateFruityInstance, docs/investigations/create-fruity-instance-harness.md) builds the harness with
#                               the fake host object (build_two_native.sh FRUITY=1, gen_two.sh FRUITY_ROWS=lift/harness/fruity-rows.txt; its own TAG, so
#                               the other builds of the same OUT are untouched), runs it, and after the DllMain report the guest calls
#                               CreateFruityInstance(0xC0, 0xEDE95A, host, &instance); the "fruity:" lines of the native log are printed in a "---- fruity"
#                               block. FLE_FRUITY_MAX_CALLS, FLE_FRUITY_SECONDS, FLE_FRUITY_STOP_AT and FLE_FRUITY_TRAP (lift/harness/fruity.h) pass
#                               through the environment. Unset or 0: this script does what it did before; the other FLE_FRUITY_* alone are refused.
#                               Needs at least one real module. Native only.
#        FLE_BLEND2D_PROBE=1   (X1, docs/investigations/blend2d-jit-and-drawing-path.md) builds the harness with the Blend2D probe (build_two_native.sh
#                               B2D_PROBE=1; its own TAG, so the other builds of the same OUT are untouched), runs it, and after the DllMain report the
#                               guest's own Blend2D is called: AggCreateInstance, attach(64x64 32-bit buffer), then drawing steps; the "b2d:" lines of the native
#                               log are printed in a "---- b2d" block. FLE_B2D_MODE, FLE_B2D_OPS, FLE_B2D_TLS, FLE_B2D_MAX_CALLS, FLE_B2D_SECONDS and FLE_B2D_TRAP
#                               (lift/harness/b2d_probe.h) pass through the environment. Unset or 0: this script does what it did before; the other FLE_B2D_*
#                               alone are refused. Needs the real module blend2d.dll among the modules. Native only. Not combined with FLE_CALL_FRUITY=1.
#        FLE_BLEND2D_SCAN=1    (X2, docs/investigations/blend2d-nojit-slots.md) builds the harness with the Agg2D scan (build_two_native.sh B2D_SCAN=1; its own TAG) and runs it;
#                               FLE_B2D_SCAN=<test> (lift/harness/b2d_scan.h; `list` prints the tests) names the one test to run after the DllMain report; the "scan:" lines of
#                               the native log are printed in a "---- scan" block. Unset or 0: this script does what it did before; FLE_B2D_SCAN alone is refused. Needs the real
#                               module blend2d.dll. Not combined with FLE_CALL_FRUITY=1 or FLE_BLEND2D_PROBE=1.
#        REG_SEED=<abs file>[:<abs file>...]   (decision 0116, provisional) lists of values that the harness writes to the virtual registry before
#                               the guest runs (lift/harness/seeds/reg-seed-*.tsv; format and rules: lift/harness/reg_seed.h). Only with
#                               FLE_CALL_FRUITY=1 (the FRUITY build is the only one that contains the code; any other build is refused here). Without it the
#                               registry is empty, as before (decisions 0018, 0085), and the output of this script is unchanged. It is a run-time
#                               input: not part of the stamp or of the TAG, copied to $RUN/reg-seed-N.tsv, and reported ("a4-multi: REG_SEED=..." and a
#                               "---- reg seed" block). The variable FLE_REG_SEED of the caller is ignored (unset here): only REG_SEED counts.
#        AUTH_CLAIMS=<abs file>   (decision 0117, provisional) the table of Authenticode claims that the harness registers with the runtime before the guest runs
#                               (lift/harness/seeds/authenticode-claims.tsv, written by tools/authenticode_check.py; format and rules: lift/harness/authenticode_claims.h).
#                               Only with FLE_CALL_FRUITY=1 (the FRUITY build is the only one that contains the code; any other build is refused here). Without it the
#                               table is empty and wintrust!WinVerifyTrust stops by name. A run-time input: not part of the stamp or of the TAG, copied to
#                               $RUN/authenticode-claims.tsv, and reported ("a4-multi: AUTH_CLAIMS=..." and a "---- authenticode claims" block). The variable
#                               FLE_AUTH_CLAIMS of the caller is ignored (unset here): only AUTH_CLAIMS counts.
#        FLE_PER_PART=3700   REAL_PER_PART=800   EXPECT_FLE_SHA=<SHA-256 of the FLEngine generated C; `skip` = not checked>
# stdout: the a4-survey summary, the 6 `a4:` lines, the stop message (the runtime message, whole), functions/entries, the exit code
#         of the harness, the stage times. Progress goes to stderr, the logs to $OUT/runs/<run>/.
# exit:  0 the survey ran (the stop is the result), 2 something needed is missing or a stage failed (a4-survey.sh: 3 for WASM=1 only).
set -u -o pipefail
export PYTHONDONTWRITEBYTECODE=1
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=${LIFT:-$(cd "$HERE/.." && pwd)}
die() { echo "a4-multi: $*" >&2; exit 2; }

[ -n "${OUT:-}" ] || die "OUT is not set (an absolute path)"
case "$OUT" in /*) ;; *) die "OUT must be an absolute path: $OUT" ;; esac
FLE=${FLE:-$LIFT/../binaries/win64/FLEngine_x64.dll}
[ -r "$FLE" ] || die "FLEngine dll not found: $FLE (set FLE=)"
FLE=$(cd "$(dirname "$FLE")" && pwd)/$(basename "$FLE")
JOBS=${JOBS:-6}
FLE_PER_PART=${FLE_PER_PART:-3700}
REAL_PER_PART=${REAL_PER_PART:-800}
EXPECT_FLE_SHA=${EXPECT_FLE_SHA:-f6d3a9715ddb92d734577fc1e87a381142c722aa17562a22a72c565599cbb08c}

# ---- the module list: NAME:DLL:BASE[:SEEDS[:OPTS]] ----
N=$#
# decision 0171 (PROVISIONAL, opt-in): FLE_EXTRA_IMPORTS=<abs file> adds import rows (fe-lift lift --extra-imports; decision 0156) to FLEngine AND to every real module, because the response table
# (GetProcAddress, the delay-load helper) must be the same in every module (merge_tables_n.py checks it). Without it nothing below differs from the version without it.
FLE_EXTRA_IMPORTS=${FLE_EXTRA_IMPORTS:-}
FLE_EXTRA_ARGS=""
if [ -n "$FLE_EXTRA_IMPORTS" ]; then
    case "$FLE_EXTRA_IMPORTS" in /*) ;; *) die "FLE_EXTRA_IMPORTS must be an absolute path: $FLE_EXTRA_IMPORTS" ;; esac
    [ -r "$FLE_EXTRA_IMPORTS" ] || die "cannot read FLE_EXTRA_IMPORTS=$FLE_EXTRA_IMPORTS"
    case "$FLE_EXTRA_IMPORTS" in *[\'\ \"\;\|]*) die "FLE_EXTRA_IMPORTS holds a quote, a space, ; or |" ;; esac
    FLE_EXTRA_ARGS="--extra-imports $FLE_EXTRA_IMPORTS"
fi
export FLE_EXTRA_IMPORTS
names=(); files=(); bases=(); seeds=(); opts=(); hooks=(); sitesf=(); largs=()
for arg in "$@"; do
    IFS=':' read -r nm f b s o hk sf extra <<< "$arg"
    { [ -n "$nm" ] && [ -n "$f" ] && [ -n "$b" ] && [ -z "${extra:-}" ]; } || die "bad module '$arg' (NAME:DLL:BASE[:SEEDS[:OPTS[:HOOKS[:SITES]]]])"
    case "$nm$f${s:-}${hk:-}${sf:-}" in *[\;\|\ \"\\]*) die "module '$arg': a name, path, seed file, hook file or sites file holds one of ; | \" \\ or a space" ;; esac
    case "$nm" in *=*) die "module '$arg': the name holds '='" ;; esac
    case "$f" in /*) ;; *) die "module '$arg': DLL must be an absolute path" ;; esac
    [ -r "$f" ] || die "module '$arg': cannot read $f"
    if [ -n "${s:-}" ]; then
        case "$s" in /*) ;; *) die "module '$arg': SEEDS must be an absolute path" ;; esac
        [ -r "$s" ] || die "module '$arg': cannot read $s"
    fi
    if [ -n "${hk:-}" ]; then
        case "$hk" in /*) ;; *) die "module '$arg': HOOKS must be an absolute path" ;; esac
        [ -r "$hk" ] || die "module '$arg': cannot read $hk"
    fi
    if [ -n "${sf:-}" ]; then
        case "$sf" in /*) ;; *) die "module '$arg': SITES must be an absolute path" ;; esac
        [ -r "$sf" ] || die "module '$arg': cannot read $sf"
    fi
    case "${o:-}" in ""|pdata) ;; *) die "module '$arg': unknown option '$o' (only pdata)" ;; esac
    la=""
    [ "${o:-}" = pdata ] && la="--pdata"
    [ -n "${s:-}" ] && la="${la:+$la }--seeds $s"
    [ -n "${hk:-}" ] && la="${la:+$la }--hook-points $hk"
    [ -n "${sf:-}" ] && la="${la:+$la }--site-overrides $sf"
    [ -n "$FLE_EXTRA_ARGS" ] && la="${la:+$la }$FLE_EXTRA_ARGS"
    names+=("$nm"); files+=("$f"); bases+=("$b"); seeds+=("${s:-}"); opts+=("${o:-}"); hooks+=("${hk:-}"); sitesf+=("${sf:-}"); largs+=("$la")
done
mk_largs() {   # mk_largs K : the `fe-lift lift` arguments of module K from its fields (the same text the loop above builds)
    local k=$1 la=""
    [ "${opts[$k]}" = pdata ] && la="--pdata"
    [ -n "${seeds[$k]}" ] && la="${la:+$la }--seeds ${seeds[$k]}"
    [ -n "${hooks[$k]}" ] && la="${la:+$la }--hook-points ${hooks[$k]}"
    [ -n "${sitesf[$k]}" ] && la="${la:+$la }--site-overrides ${sitesf[$k]}"
    [ -n "$FLE_EXTRA_ARGS" ] && la="${la:+$la }$FLE_EXTRA_ARGS"
    printf '%s' "$la"
}

# ---- VFS_SEED (decision 0110): the caller's FLE_VFS_SEED / FLE_VFS_SEED_ROOT must not seed a run that did not ask for it ----
unset FLE_VFS_SEED FLE_VFS_SEED_ROOT
VFS_SEED=${VFS_SEED:-}
VFS_SEED_ROOT=${VFS_SEED_ROOT:-}
if [ -n "$VFS_SEED" ]; then
    case "$VFS_SEED" in /*) ;; *) die "VFS_SEED must be an absolute path: $VFS_SEED" ;; esac
    [ -r "$VFS_SEED" ] || die "cannot read VFS_SEED=$VFS_SEED"
    case "$VFS_SEED" in *[\'\ \"\;\|]*) die "VFS_SEED holds a quote, a space, ; or |" ;; esac
    [ "$N" -gt 0 ] || die "VFS_SEED is set, but with no real module the 0-module build runs (harness/wasm/build_native.sh), which does not read it"
    [ -n "$VFS_SEED_ROOT" ] || VFS_SEED_ROOT=$(cd "$LIFT/.." && pwd)
    case "$VFS_SEED_ROOT" in /*) ;; *) die "VFS_SEED_ROOT must be an absolute path: $VFS_SEED_ROOT" ;; esac
    [ -d "$VFS_SEED_ROOT" ] || die "VFS_SEED_ROOT is not a directory: $VFS_SEED_ROOT"
    export FLE_VFS_SEED="$VFS_SEED" FLE_VFS_SEED_ROOT="$VFS_SEED_ROOT"
elif [ -n "$VFS_SEED_ROOT" ]; then
    die "VFS_SEED_ROOT is set without VFS_SEED"
fi

# ---- FLE_CALL_FRUITY (stage 1 of CreateFruityInstance): selects the build (own TAG) and the report block; the run itself reads the variables ----
FRUITY_BUILD=0
case "${FLE_CALL_FRUITY:-}" in
    ""|0) ;;
    1) FRUITY_BUILD=1 ;;
    *) die "FLE_CALL_FRUITY must be unset, empty, 0 or 1 (got '$FLE_CALL_FRUITY')" ;;
esac
if [ "$FRUITY_BUILD" = 1 ]; then
    [ "$N" -gt 0 ] || die "FLE_CALL_FRUITY=1 needs at least one real module (the 0-module build does not contain the fruity harness)"
    [ -r "$LIFT/harness/fruity-rows.txt" ] || die "cannot read $LIFT/harness/fruity-rows.txt (this LIFT tree has no stage-1 harness)"
else
    for v in FLE_FRUITY_MAX_CALLS FLE_FRUITY_SECONDS FLE_FRUITY_STOP_AT FLE_FRUITY_TRAP; do
        [ -z "${!v:-}" ] || die "$v is set without FLE_CALL_FRUITY=1"
    done
fi

# ---- DSP_SEEDS (decision 0127): the seeds of the module named dsp_ippv2_x64.dll in the FRUITY build (decision 0126 wall 97: the case bodies of the IPP jump tables,
# lift/harness/seeds/dsp-ippv2-2000000-seeds-jt.txt). It REPLACES the SEEDS field of that module (what out-fruity11/tools/run6.sh did with ${DSP_SEEDS:-<default seeds>}), so it is
# applied before the TAG and the stamp are computed and both see it. Only with FLE_CALL_FRUITY=1; without it a set DSP_SEEDS is refused (the FRUITY-off build is the one of
# decision 0113 and keeps the SEEDS field it was given). The same checks as FLE_SEEDS.
DSP_SEEDS=${DSP_SEEDS:-}
if [ -n "$DSP_SEEDS" ]; then
    [ "$FRUITY_BUILD" = 1 ] || die "DSP_SEEDS is set without FLE_CALL_FRUITY=1 (only the FRUITY configuration uses the jump-table seeds of decision 0126)"
    case "$DSP_SEEDS" in /*) ;; *) die "DSP_SEEDS must be an absolute path: $DSP_SEEDS" ;; esac
    [ -r "$DSP_SEEDS" ] || die "cannot read DSP_SEEDS=$DSP_SEEDS"
    case "$DSP_SEEDS" in *[\'\ \"\;\|]*) die "DSP_SEEDS holds a quote, a space, ; or |" ;; esac
    dsp_k=-1
    for k in "${!names[@]}"; do [ "${names[$k]}" = dsp_ippv2_x64.dll ] && dsp_k=$k; done
    [ "$dsp_k" -ge 0 ] || die "DSP_SEEDS is set but no module is named dsp_ippv2_x64.dll"
    seeds[$dsp_k]="$DSP_SEEDS"
    largs[$dsp_k]=$(mk_largs "$dsp_k")
fi

# ---- FLE_BLEND2D_PROBE (X1): selects the build (own TAG) and the report block; the run itself reads the variables ----
B2D_BUILD=0
case "${FLE_BLEND2D_PROBE:-}" in
    ""|0) ;;
    1) B2D_BUILD=1 ;;
    *) die "FLE_BLEND2D_PROBE must be unset, empty, 0 or 1 (got '$FLE_BLEND2D_PROBE')" ;;
esac
if [ "$B2D_BUILD" = 1 ]; then
    [ "$N" -gt 0 ] || die "FLE_BLEND2D_PROBE=1 needs at least one real module (the 0-module build does not contain the probe)"
    [ "$FRUITY_BUILD" = 0 ] || die "FLE_BLEND2D_PROBE=1 and FLE_CALL_FRUITY=1 are not combined"
    [ -r "$LIFT/harness/b2d_probe.c" ] || die "cannot read $LIFT/harness/b2d_probe.c (this LIFT tree has no X1 probe)"
    case " ${names[*]} " in *" blend2d.dll "*) ;; *) die "FLE_BLEND2D_PROBE=1 needs a module named blend2d.dll" ;; esac
else
    for v in FLE_B2D_MODE FLE_B2D_OPS FLE_B2D_TLS FLE_B2D_MAX_CALLS FLE_B2D_SECONDS FLE_B2D_TRAP; do
        [ -z "${!v:-}" ] || die "$v is set without FLE_BLEND2D_PROBE=1"
    done
fi

# ---- FLE_BLEND2D_SCAN (X2): selects the build (own TAG) and the report block; the run itself reads FLE_B2D_SCAN ----
SCAN_BUILD=0
case "${FLE_BLEND2D_SCAN:-}" in
    ""|0) ;;
    1) SCAN_BUILD=1 ;;
    *) die "FLE_BLEND2D_SCAN must be unset, empty, 0 or 1 (got '$FLE_BLEND2D_SCAN')" ;;
esac
if [ "$SCAN_BUILD" = 1 ]; then
    [ "$N" -gt 0 ] || die "FLE_BLEND2D_SCAN=1 needs at least one real module"
    [ "$FRUITY_BUILD$B2D_BUILD" = 00 ] || die "FLE_BLEND2D_SCAN=1 is not combined with FLE_CALL_FRUITY=1 or FLE_BLEND2D_PROBE=1"
    [ -r "$LIFT/harness/b2d_scan.c" ] || die "cannot read $LIFT/harness/b2d_scan.c (this LIFT tree has no X2 scan)"
    case " ${names[*]} " in *" blend2d.dll "*) ;; *) die "FLE_BLEND2D_SCAN=1 needs a module named blend2d.dll" ;; esac
else
    [ -z "${FLE_B2D_SCAN:-}" ] || die "FLE_B2D_SCAN is set without FLE_BLEND2D_SCAN=1"
fi

# ---- REG_SEED (decision 0116): the caller's FLE_REG_SEED must not seed a run that did not ask for it; only the FRUITY build contains the code ----
unset FLE_REG_SEED
REG_SEED=${REG_SEED:-}
REG_SEED_FILES=()
if [ -n "$REG_SEED" ]; then
    [ "$FRUITY_BUILD" = 1 ] || die "REG_SEED is set without FLE_CALL_FRUITY=1 (only the FRUITY build contains the registry seeding)"
    case "$REG_SEED" in *[\'\ \"\;\|]*) die "REG_SEED holds a quote, a space, ; or |" ;; esac
    IFS=':' read -ra REG_SEED_FILES <<< "$REG_SEED"
    [ "${#REG_SEED_FILES[@]}" -gt 0 ] || die "REG_SEED holds no file"
    for f in "${REG_SEED_FILES[@]}"; do
        case "$f" in /*) ;; *) die "REG_SEED: every file must be an absolute path: '$f'" ;; esac
        [ -r "$f" ] || die "cannot read REG_SEED file $f"
    done
    export FLE_REG_SEED="$REG_SEED"
fi

# ---- AUTH_CLAIMS (decision 0117): the caller's FLE_AUTH_CLAIMS must not vouch for a run that did not ask for it; only the FRUITY build contains the code ----
unset FLE_AUTH_CLAIMS
AUTH_CLAIMS=${AUTH_CLAIMS:-}
if [ -n "$AUTH_CLAIMS" ]; then
    [ "$FRUITY_BUILD" = 1 ] || die "AUTH_CLAIMS is set without FLE_CALL_FRUITY=1 (only the FRUITY build contains the claims registration)"
    case "$AUTH_CLAIMS" in /*) ;; *) die "AUTH_CLAIMS must be an absolute path: $AUTH_CLAIMS" ;; esac
    case "$AUTH_CLAIMS" in *[\'\ \"\;\|:]*) die "AUTH_CLAIMS holds a quote, a space, ; | or :" ;; esac
    [ -r "$AUTH_CLAIMS" ] || die "cannot read AUTH_CLAIMS file $AUTH_CLAIMS"
    export FLE_AUTH_CLAIMS="$AUTH_CLAIMS"
fi

# ---- OUT: only a directory that is empty, or that this script made ----
if [ -e "$OUT" ] && [ -n "$(ls -A "$OUT")" ] && [ ! -e "$OUT/.a4-multi" ]; then
    die "$OUT exists and was not made by a4-multi.sh: nothing written"
fi
mkdir -p "$OUT"
[ -e "$OUT/.a4-multi" ] || echo "made by lift/harness/a4-multi.sh" > "$OUT/.a4-multi"
mkdir -p "$OUT/tmp" "$OUT/log" "$OUT/runs"
export TMPDIR="$OUT/tmp"

FLE_SEEDS=${FLE_SEEDS:-}
FLE_SEED_ARGS=""
if [ -n "$FLE_SEEDS" ]; then
    case "$FLE_SEEDS" in /*) ;; *) die "FLE_SEEDS must be an absolute path: $FLE_SEEDS" ;; esac
    [ -r "$FLE_SEEDS" ] || die "cannot read FLE_SEEDS=$FLE_SEEDS"
    case "$FLE_SEEDS" in *[\'\ \"\;\|]*) die "FLE_SEEDS holds a quote, a space, ; or |" ;; esac
    FLE_SEED_ARGS="--seeds $FLE_SEEDS"
fi
export FLE_SEEDS
FLE_HOOKS=${FLE_HOOKS:-}
FLE_HOOK_ARGS=""
if [ -n "$FLE_HOOKS" ]; then
    case "$FLE_HOOKS" in /*) ;; *) die "FLE_HOOKS must be an absolute path: $FLE_HOOKS" ;; esac
    [ -r "$FLE_HOOKS" ] || die "cannot read FLE_HOOKS=$FLE_HOOKS"
    case "$FLE_HOOKS" in *[\'\ \"\;\|]*) die "FLE_HOOKS holds a quote, a space, ; or |" ;; esac
    FLE_HOOK_ARGS="--hook-points $FLE_HOOKS"
fi
export FLE_HOOKS
FLE_SEH_HANDLERS=${FLE_SEH_HANDLERS:-}
FLE_SEH_ARGS=""
if [ -n "$FLE_SEH_HANDLERS" ]; then
    case "$FLE_SEH_HANDLERS" in *[!0-9a-fx,]*|,*|*,|*,,*) die "FLE_SEH_HANDLERS must be hex RVAs separated by commas: $FLE_SEH_HANDLERS" ;; esac
    IFS=',' read -ra _seh_h <<< "$FLE_SEH_HANDLERS"
    for _h in "${_seh_h[@]}"; do FLE_SEH_ARGS="${FLE_SEH_ARGS:+$FLE_SEH_ARGS }--seh-handler $_h"; done
fi
export FLE_SEH_HANDLERS

FEBIN=${FEBIN:-$OUT/target/release}
if [ ! -x "$FEBIN/fe-lift" ] || [ ! -x "$FEBIN/fe-pe" ]; then
    [ "$FEBIN" = "$OUT/target/release" ] || die "fe-lift / fe-pe not found in FEBIN=$FEBIN"
    echo "a4-multi: building fe-pe and fe-lift into $OUT/target" >&2
    (cd "$LIFT" && CARGO_TARGET_DIR="$OUT/target" cargo build --release --offline -p fe-pe -p fe-lift) > "$OUT/log/cargo-build.txt" 2>&1 \
        || die "cargo build of fe-pe and fe-lift failed (see $OUT/log/cargo-build.txt)"
fi

# ---- the stamp: what the shared objects of this OUT were made from (the build scripts do not detect a stale object) ----
stamp() {
    (cd "$LIFT" && {
        find runtime -type f | sort
        ls harness/*.c harness/*.h harness/wasm/trace_native.c harness/wasm/split_c.py harness/wasm/build_native.sh \
           harness/two/main_two.c harness/two/gen_two.sh harness/two/build_two_native.sh harness/two/merge_tables_n.py \
           harness/two/two-native-run.sh harness/fruity-rows.txt crates/fe-lift/tests/realmod_s1/tools/*.py
    } | xargs sha256sum)
    sha256sum "$FEBIN/fe-lift" "$FEBIN/fe-pe" "$FLE"
    echo "FLE_PER_PART=$FLE_PER_PART REAL_PER_PART=$REAL_PER_PART EXPECT_FLE_SHA=$EXPECT_FLE_SHA"
    echo "FLE_HOOKS=$([ -n "$FLE_HOOKS" ] && sha256sum "$FLE_HOOKS" | cut -d' ' -f1 || echo none)"
    echo "FLE_SEEDS=$([ -n "$FLE_SEEDS" ] && sha256sum "$FLE_SEEDS" | cut -d' ' -f1 || echo none)"
    [ -z "$FLE_EXTRA_IMPORTS" ] || echo "FLE_EXTRA_IMPORTS=$(sha256sum "$FLE_EXTRA_IMPORTS" | cut -d' ' -f1)"
    echo "DSP_SEEDS=$([ -n "$DSP_SEEDS" ] && sha256sum "$DSP_SEEDS" | cut -d' ' -f1 || echo none)"
    [ -z "$FLE_SEH_HANDLERS" ] || echo "FLE_SEH_HANDLERS=$FLE_SEH_HANDLERS"
}
STAMP_NOW=$(stamp | sed 's|  .*/|  |' | sha256sum | cut -d' ' -f1)
if [ -e "$OUT/stamp.txt" ]; then
    [ "$(cat "$OUT/stamp.txt")" = "$STAMP_NOW" ] || die "$OUT was made from other sources or tools (stamp.txt differs): use a new OUT"
else
    echo "$STAMP_NOW" > "$OUT/stamp.txt"
fi

TAG=${TAG:-}
if [ "$N" -gt 0 ] && [ -z "$TAG" ]; then
    key=$(for k in "${!names[@]}"; do
        echo "${names[$k]}|$(sha256sum "${files[$k]}" | cut -d' ' -f1)|${bases[$k]}|${opts[$k]}|$([ -n "${seeds[$k]}" ] && sha256sum "${seeds[$k]}" | cut -d' ' -f1 || echo -)|$([ -n "${hooks[$k]}" ] && sha256sum "${hooks[$k]}" | cut -d' ' -f1 || echo -)$([ -n "${sitesf[$k]}" ] && echo "|$(sha256sum "${sitesf[$k]}" | cut -d' ' -f1)")"
    done)
    [ "$FRUITY_BUILD" = 1 ] && key="$key"$'\n'"fruity-rows|$(sha256sum "$LIFT/harness/fruity-rows.txt" | cut -d' ' -f1)"
    [ "$B2D_BUILD" = 1 ] && key="$key"$'\n'"b2d-probe|$(sha256sum "$LIFT/harness/b2d_probe.c" | cut -d' ' -f1)"
    [ "$SCAN_BUILD" = 1 ] && key="$key"$'\n'"b2d-scan|$(cat "$LIFT/harness/b2d_scan.c" "$LIFT/harness/b2d_scan_tests.h" "$LIFT/harness/b2d_scan_tests2.h" "$LIFT/harness/b2d_scan_table.h" "$LIFT/harness/b2d_scan.h" | sha256sum | cut -d' ' -f1)"
    TAG="t$(echo "$key" | sha256sum | cut -c1-12)"
fi
RUN="$OUT/runs/$(date -u +%Y%m%dT%H%M%SZ)-$$"
mkdir "$RUN" || die "cannot create $RUN"
TIMES=()
run_stage() {   # run_stage NAME CMD... : the output of CMD goes to $RUN/NAME.txt; a failure stops the script
    local name=$1 t0=$SECONDS rc
    shift
    echo "a4-multi: stage $name" >&2
    "$@" > "$RUN/$name.txt" 2>&1
    rc=$?
    TIMES+=("$name $((SECONDS - t0)) s")
    if [ $rc -ne 0 ]; then
        tail -n 15 "$RUN/$name.txt" >&2
        die "stage $name failed (exit $rc), log $RUN/$name.txt"
    fi
}

echo "a4-multi: OUT=$OUT  LIFT=$LIFT  FLE=$FLE"
echo "a4-multi: FLEngine sha256 $(sha256sum "$FLE" | cut -d' ' -f1)"
echo "a4-multi: real modules: $N${TAG:+  TAG=$TAG}"
[ "$FRUITY_BUILD" = 1 ] && echo "a4-multi: FLE_CALL_FRUITY=1 (stage-1 harness of CreateFruityInstance: FRUITY=1 build, FRUITY_ROWS=$LIFT/harness/fruity-rows.txt)"
[ "$B2D_BUILD" = 1 ] && echo "a4-multi: FLE_BLEND2D_PROBE=1 (X1 probe of Blend2D: B2D_PROBE=1 build)"
[ "$SCAN_BUILD" = 1 ] && echo "a4-multi: FLE_BLEND2D_SCAN=1 (X2 scan of the Agg2D entries: B2D_SCAN=1 build)"
if [ -n "$VFS_SEED" ]; then
    echo "a4-multi: VFS_SEED=$VFS_SEED sha256=$(sha256sum "$VFS_SEED" | cut -d' ' -f1) VFS_SEED_ROOT=$VFS_SEED_ROOT"
    cp "$VFS_SEED" "$RUN/vfs-seed.tsv"
fi

if [ -n "$REG_SEED" ]; then
    k=0
    for f in "${REG_SEED_FILES[@]}"; do
        k=$((k + 1))
        echo "a4-multi: REG_SEED[$k]=$f sha256=$(sha256sum "$f" | cut -d' ' -f1)"
        cp "$f" "$RUN/reg-seed-$k.tsv"
    done
fi

if [ -n "$AUTH_CLAIMS" ]; then
    echo "a4-multi: AUTH_CLAIMS=$AUTH_CLAIMS sha256=$(sha256sum "$AUTH_CLAIMS" | cut -d' ' -f1)"
    cp "$AUTH_CLAIMS" "$RUN/authenticode-claims.tsv"
fi

if [ "$N" -eq 0 ]; then
    # ---- FLEngine alone: the single-module build (harness/wasm/build_native.sh) and the unchanged a4-survey.sh ----
    SD="$OUT/single"
    mkdir -p "$OUT/gen" "$SD/out"
    if [ ! -e "$OUT/gen/flengine.c" ]; then
        run_stage lift-fle sh -c "'$FEBIN/fe-lift' lift '$FLE' '$OUT/gen/flengine.c' --pdata $FLE_SEED_ARGS $FLE_HOOK_ARGS $FLE_SEH_ARGS $FLE_EXTRA_ARGS 2> '$OUT/log/gen-flengine.err'"
    fi
    got=$(sha256sum "$OUT/gen/flengine.c" | cut -d' ' -f1)
    if [ "$EXPECT_FLE_SHA" != skip ] && [ "$got" != "$EXPECT_FLE_SHA" ]; then
        die "FLEngine C sha256 $got differs from $EXPECT_FLE_SHA"
    fi
    echo "a4-multi: FLEngine generated C sha256 $got"
    [ -e "$SD/out/wasm-pdata-part-00.c" ] || run_stage split-fle python3 "$LIFT/harness/wasm/split_c.py" "$OUT/gen/flengine.c" "$SD/out/wasm-pdata-part" "$FLE_PER_PART"
    run_stage build env OUT="$SD" JOBS="$JOBS" bash "$LIFT/harness/wasm/build_native.sh"
    NB="$SD/native/flengine-native"
    OUT="$RUN" DLL="$FLE" NATIVE_BIN="$NB" FEPE="$FEBIN/fe-pe" bash "$HERE/a4-survey.sh" > "$RUN/survey.txt" 2> "$RUN/survey.err"
else
    # ---- FLEngine + N real modules: gen_two.sh, build_two_native.sh, two-native-run.sh ----
    mods=""; rmn=""; treal=""
    for k in "${!names[@]}"; do
        label="m$((k + 1))"
        mods="${mods:+$mods;}$label|${files[$k]}|${bases[$k]}|${largs[$k]}||$REAL_PER_PART"
        rmn="${rmn:+$rmn;}$label=${names[$k]}"
        treal="${treal:+$treal:}$OUT/v-$TAG/dll/$(basename "${files[$k]}" .dll).$(printf '%x' "${bases[$k]}").dll"
        echo "a4-multi: module $label name=${names[$k]} base=${bases[$k]} opts=${opts[$k]:--} seeds=${seeds[$k]:--} hooks=${hooks[$k]:--}${sitesf[$k]:+ sites=${sitesf[$k]}} file=${files[$k]} sha256=$(sha256sum "${files[$k]}" | cut -d' ' -f1)"
    done
    run_stage gen env OUT="$OUT" TAG="$TAG" FEBIN="$FEBIN" FLE="$FLE" LIFT="$LIFT" MODULES="$mods" DLLDIR="v-$TAG/dll" \
        FLE_PER_PART="$FLE_PER_PART" REAL_PER_PART="$REAL_PER_PART" EXPECT_FLE_SHA="$EXPECT_FLE_SHA" \
        FRUITY_ROWS="$([ "$FRUITY_BUILD" = 1 ] && echo "$LIFT/harness/fruity-rows.txt")" \
        bash "$LIFT/harness/two/gen_two.sh" rebase gen split merge
    run_stage build env OUT="$OUT" TAG="$TAG" LIFT="$LIFT" JOBS="$JOBS" RMNAMES="$rmn" FRUITY="$FRUITY_BUILD" B2D_PROBE="$B2D_BUILD" B2D_SCAN="$SCAN_BUILD" bash "$LIFT/harness/two/build_two_native.sh"
    TN="$OUT/v-$TAG/native/two-native"
    [ -x "$TN" ] || die "the build did not make $TN (see $RUN/build.txt)"
    OUT="$RUN" DLL="$FLE" NATIVE_BIN="$LIFT/harness/two/two-native-run.sh" TWO_NATIVE="$TN" TWO_REAL="$treal" FEPE="$FEBIN/fe-pe" \
        bash "$HERE/a4-survey.sh" > "$RUN/survey.txt" 2> "$RUN/survey.err"
fi
SRC=$?
[ -s "$RUN/survey.txt" ] || die "a4-survey.sh printed nothing (exit $SRC), see $RUN/survey.err"

# ---- the report ----
NAT=$(ls "$RUN"/a4/*-native.txt 2>/dev/null | head -1)
[ -n "$NAT" ] || { cat "$RUN/survey.txt"; cat "$RUN/survey.err" >&2; die "no native log in $RUN/a4 (a4-survey.sh exit $SRC)"; }
echo "---- a4-survey.sh (exit $SRC)"
cat "$RUN/survey.txt"
echo "---- a4 lines"
grep '^a4:' "$NAT"
if [ -n "$VFS_SEED" ]; then
    echo "---- vfs seed"
    grep '^vfs seed:' "$NAT" || echo "(no 'vfs seed:' line in the native log: the harness did not seed; see the *-native.err beside it)"
fi
if [ -n "$REG_SEED" ]; then
    echo "---- reg seed"
    grep '^reg seed:' "$NAT" || echo "(no 'reg seed:' line in the native log: the harness did not seed; see the *-native.err beside it)"
fi
if [ -n "$AUTH_CLAIMS" ]; then
    echo "---- authenticode claims"
    grep '^authenticode claims:' "$NAT" || echo "(no 'authenticode claims:' line in the native log: the harness did not register; see the *-native.err beside it)"
fi
if [ "$FRUITY_BUILD" = 1 ]; then
    echo "---- fruity (stage 1 of CreateFruityInstance; the lines of the native log that start with 'fruity: ')"
    grep '^fruity: ' "$NAT" || echo "(no 'fruity:' line in the native log: the DllMain run did not return, or the harness refused; see the *-native.err beside it)"
fi
if [ "$B2D_BUILD" = 1 ]; then
    echo "---- b2d (X1 probe of Blend2D; the lines of the native log that start with 'b2d: ')"
    grep '^b2d: ' "$NAT" || echo "(no 'b2d:' line in the native log: the DllMain run did not return, or the harness refused; see the *-native.err beside it)"
fi
if [ "$SCAN_BUILD" = 1 ]; then
    echo "---- scan (X2 scan of the Agg2D entries; the lines of the native log that start with 'scan: ')"
    grep '^scan: ' "$NAT" || echo "(no 'scan:' line in the native log: the DllMain run did not return, or the harness refused; see the *-native.err beside it)"
fi
echo "---- stop"
echo "runtime message: $(sed -n 's/^runtime message: //p' "$NAT" | head -1)"
grep -E '^(== stop: |abort kind: |dispatch target: |call depth at stop: )' "$NAT"
echo "---- counts"
grep -E '^(functions executed \(distinct\): |function entries \(calls\): )' "$NAT"
HRC=$(sed -n 's/^A4 survey .*harness exit: //p' "$RUN/survey.txt" | head -1)
echo "harness exit code: ${HRC:-?}"
echo "---- stage times: $(IFS=,; echo "${TIMES[*]:-none}")"
echo "---- logs: $RUN"
exit $SRC
