#!/bin/bash
# The 6-module configuration of decisions 0113 and 0126 behind ONE entry, so that nothing outside the repository's harness is needed to reproduce it (decision 0127).
# `a4-multi.sh` does the work; this script only gives it the module list and the environment that decision 0126 measured with (it replaces out-fruity11/tools/run6.sh + measure.sh,
# which were not in the repository).
#
# usage: OUT=/abs/new/dir bash lift/harness/a4-fruity.sh on|on-0129|on-seh|off
#   on    FRUITY-on of decisions 0126/0127 (unchanged by decision 0133): FLE_CALL_FRUITY=1, REG_SEED (nsis + app), AUTH_CLAIMS, VFS_SEED (fonts + GM CCs.txt), FLE_SEEDS (8 thunks),
#         DSP_SEEDS (the 1,784 jump-table seeds), Blend2D seeds964 + the no-JIT annotation of decision 0120, FreeType seeds c2, FLE_FRUITY_MAX_CALLS=400000000, FLE_FRUITY_SECONDS=600
#         (the 20,000,000 default entries of the harness stop the run long before the idle stop (b) of decision 0126, which is at 134,880,081 entries from CreateFruityInstance).
#         That 134,880,081 is the value RECORDED in decisions 0126/0127. On HEAD 3ce8e7f `on` measured 134,882,055 entries (verify3, HEAD dcf64cb; 5,515 new functions, 10 threads, a TMsgForm
#         "Error" dialog, SIGABRT `MessageLoop: idle`): the +1,974 entries are not explained. `on` still ends at the idle stop; it does NOT reach the wall of decision 0129.
#   on-0129  the final configuration of decision 0129 (idle1), on top of `on`: VFS_SEED = vfs-seed-idle1.tsv (the fonts + the installed files that the guest asked for: 218 host files, 19 directory rows),
#         DSP_SEEDS = dsp-ippv2-2000000-seeds-idle1.txt (16,133 seeds), FLE_FRUITY_NO_IDLE_STOP=1, FLE_FRUITY_SECONDS=900 (CreateFruityInstance took 208-270 s to the wall on the measuring machine).
#         Expected (decision 0133: two independent runs, HEAD 3ce8e7f): wall 16 `kernel32!RaiseException(0x0EEDFADE, flags 1, 8 args)` (SIGABRT, "no structured exception handling is modelled"),
#         141,732,210 entries from CreateFruityInstance, 5,800 new functions, 10 threads, no dialog (TMsgForm 0), main window visible (G2) at 40,344,626 entries from the program start,
#         first non-background pixel at 141,387,834. The host files of the VFS seed are not in git (218 files, 5.6 MB): they are looked for under VFS_SEED_ROOT, and a missing one ends the run with exit 2
#         before anything is built (the message says which files and how to make them).
#   on-seh   decision 0138 (PROVISIONAL): exactly on-0129 plus FLE_SEH_HANDLERS=0x112c0 (decision 0132: fe-lift --seh-handler 0x112c0, the guest's own x64 structured exception handling runs; FLEngine's 2,816 functions that have
#         an except landing get the entry wrapper). on-0129 is kept as the baseline that stops at wall 16. Expected (decision 0138, PROVISIONAL; native, HEAD ea95cd9, JOBS=2, one run + the build of the same sources by an independent gen): the generated FLEngine C is 225,542,231 bytes, SHA-256 7573b620100afc0adafaf8028411b7bb76e45e50017e3ce5c39f9579e91c50d3
#         (without the flag 215,809,294 bytes); wall 16 is passed; no TMsgForm; first non-background pixel at 44,323,963 entries from the program start (lid 13); 17 threads; the run ends with `limit: wall time 900 s after CreateFruityInstance began`
#         (NOT a wall: 82,632,685 entries from CreateFruityInstance and 6,626 new functions in the measured run; the entry count depends on the machine's speed, decision 0132 recorded 82,763,135 and 83,167,054), two-native SHA-256
#         9f1a704533469f0b1d611178d592a2c9416a8fb2c504f0675b16ccab759268ad (the same as decision 0132's ofinA / ofinB). A deterministic stop instead of the time limit: run the built two-native again with FLE_FRUITY_NO_IDLE_STOP unset: it stops at
#         `MessageLoop: idle` (thread 0 in WaitMessage with an empty queue) after 53,441,474 entries from CreateFruityInstance (f_86a660 is entered once, at entry 42,424,519 of the call) with the face of lid 13 = 721b792e1c7729a21e8dcfaa0f72bd31488d78b5b52bd0a7eced194694f24f45
#         (lids 22 and 48 as well: 3 surfaces). The frames go on when the clock is allowed to move (idle stop off); the face then settles at 7ea690830b7e78cd744276ff51ecbbc3ada60b8c1a8bd1b92fbd8db6b208b9dd after about 225 frames (docs/decisions/0138-steady-state-and-fast-path.md).
#         NOTE (decision 0142, window visible regions): from a tree with decision 0142 the two faces above change; the idle face of lid 13 then begins 0133698f and the settled face is
#         eb4443d24f0f96377049588de9523a8ec5baf107fa7966c5ff62bc6ce2fb350a (the mixer is below the playlist, as the guest asks). 721b792e... / 7ea69083... are the pre-0142 values.
#         NOTE (decision 0150): "lid 13", "lids 22 and 48" and the like in the lines above are the numbers those builds gave the windows. A lid is the number of a window's creation (lift/runtime/fle_win.c next_lid++), so
#         it changes with the order of the guest's window creations and when the guest re-creates a window (docs/investigations/detach-window.md R5): it is not a name for the main window. The main window is the one
#         of class TFruityLoopsMainForm (the engine's window list, fle_av_poll); the app and the measurement tools look it up by that (app/src/engine/main-window.ts, docs/decisions/0150-main-window-by-properties.md).
#   off   the FRUITY-off baseline of decision 0113: FLE_CALL_FRUITY=0, no FLE_SEEDS, no DSP_SEEDS, Blend2D seeds58, FreeType c1, no annotation.
# env:  OUT=<new absolute dir>   required (a4-multi.sh refuses an OUT made from other sources or seeds: use a new name for every configuration and every tree).
#       FLE=<FLEngine_x64.dll>   default <repository root>/binaries/win64/FLEngine_x64.dll, the repository root being the parent of INPUT_LIFT
#       INPUT_LIFT=<the lift/ directory that holds the input files out-ext, out-blend2d, out-qfc, out-wallQ, out-inventory, out-seed58, out-three, out-fruity11/inputs>   default: this tree's lift/.
#                 In a `git archive` tree these directories do not exist (they are ignored by git): set INPUT_LIFT to the checkout that has them.
#       VFS_SEED_ROOT=<dir>   what the relative host paths of the VFS seed list are relative to; default the parent of INPUT_LIFT.
#       VFS_SEED_OVERRIDE=<abs file>, DSP_SEEDS_OVERRIDE=<abs file>   replace the VFS seed list / the dsp_ippv2 SEEDS file that the mode would use (a4-multi.sh's own VFS_SEED and DSP_SEEDS in the
#                 caller's environment are ignored, as before). DSP_SEEDS_OVERRIDE only with on / on-0129. With VFS_SEED_OVERRIDE, on-0129 does not run its list of representative host files
#                 (vfs_seed.h checks every host file and its SHA-256 and ends the run with exit 2 when one is wrong or missing).
#       FLE_FRUITY_MAX_CALLS, FLE_FRUITY_SECONDS   on / on-0129 only; defaults 400000000 and 600 (on) or 900 (on-0129).
#       FEBIN=<dir with fe-lift and fe-pe>   passed through to a4-multi.sh (default there: built into $OUT/target).   JOBS, TAG and the rest of a4-multi.sh's variables pass through.
# The inputs (SHA-256 in lift/out-git/disk-inventory/inputs-sha256.tsv; out-fruity11/inputs/GM_CCs.txt = af33bd90...eee5e, from the installer by tools/extract_nsis2.py) are checked to exist here.
# Exit status and stdout are a4-multi.sh's.
set -u -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
LIFT=$(cd "$HERE/.." && pwd)
die() { echo "a4-fruity: $*" >&2; exit 2; }
MODE=${1:-}
case "$MODE" in on|on-0129|on-seh|off) ;; *) die "usage: OUT=/abs/new/dir bash $0 on|on-0129|on-seh|off" ;; esac
IDLE1=0
[ "$MODE" = on-0129 ] || [ "$MODE" = on-seh ] && IDLE1=1
[ -n "${OUT:-}" ] || die "OUT is not set (a new absolute path)"
IN=$(cd "${INPUT_LIFT:-$LIFT}" && pwd) || die "INPUT_LIFT is not a directory"
export FLE=${FLE:-$IN/../binaries/win64/FLEngine_x64.dll}
export JOBS=${JOBS:-8}
export EXPECT_FLE_SHA=skip
export FLE_HOOKS=$HERE/hooks/flengine-hook-points.txt
VFS_SEED_MODE=$HERE/seeds/vfs-seed-fonts.tsv
[ "$IDLE1" = 1 ] && VFS_SEED_MODE=$HERE/seeds/vfs-seed-idle1.tsv
export VFS_SEED=${VFS_SEED_OVERRIDE:-$VFS_SEED_MODE}
export VFS_SEED_ROOT=${VFS_SEED_ROOT:-$(cd "$IN/.." && pwd)}

DSP=$IN/out-ext/dsp_ippv2_x64.dll
B2D=$IN/out-blend2d/Blend2D.dll
QFC=$IN/out-qfc/quickfontcache_x64.dll
FT=$IN/out-wallQ/freetype/freetype_x64.dll
ILT=$IN/out-inventory/extracted/ILTools_x64.dll
DSP_BASE_SEEDS=$IN/out-three/three/dll/seeds-idx0-plus-initterm2.txt   # the seeds of decision 0113 (452 VAs); the SEEDS field of the dsp module in both modes
B2D_SEEDS_OFF=$IN/out-seed58/seeds/seeds58-4700000.txt
if [ "$IDLE1" = 1 ] && [ -z "${VFS_SEED_OVERRIDE:-}" ]; then
    missing=""
    for r in lift/out-idle1/inputs/install/Data/System/Empty.flp lift/out-idle1/inputs/install/Data/System/Metronome2.wav \
             lift/out-idle1/inputs/install/Shared/Artwork/Fonts/OpenSans-Regular.ttf "lift/out-idle1/inputs/install/Data/Patches/Mixer presets/Mono tape simulation.fst" \
             lift/out-idle1/inputs/install/Data/Patches/Speech/AllYourBase.speech "lift/out-idle1/inputs/install/System/Config/Typing to piano/Other Whole Tone.scr" \
             lift/out-inventory/extracted/fonts/ILGlyphsEx.ilfont lift/out-inventory/extracted/fonts/ILCursors.ilfont lift/out-fruity11/inputs/GM_CCs.txt; do
        [ -f "$VFS_SEED_ROOT/$r" ] && [ -r "$VFS_SEED_ROOT/$r" ] || missing="$missing
    $VFS_SEED_ROOT/$r"
    done
    if [ -n "$missing" ]; then
        echo "a4-fruity: $MODE: host files of the VFS seed (vfs-seed-idle1.tsv) are missing under VFS_SEED_ROOT=$VFS_SEED_ROOT (VFS seed のホストファイルが無い):$missing" >&2
        echo "a4-fruity: they are not in git (218 files, 5.6 MB). Make them from the FL Studio installer flstudio_win64_26.1.7.5653.exe (SHA-256 04393e7946c7369aaa5cd9ce1c755803e922146289e1ae3263629a44f4e5fc02) (インストーラから作る):" >&2
        echo "    VFS_SEED_ROOT=<root> python3 lift/harness/seeds/gen_vfs_seed_installed.py <installer.exe> docs/investigations/idle1/logs/files-touched.tsv <root>/lift/out-idle1/inputs/install <out.tsv> lift/harness/seeds/vfs-seed-fonts.tsv" >&2
        echo "    (Data/..., Shared/..., System/... under lift/out-idle1/inputs/install/; the fonts under lift/out-inventory/extracted/fonts/ and GM_CCs.txt under lift/out-fruity11/inputs/ come from tools/extract_nsis2.py)," >&2
        echo "    or set VFS_SEED_ROOT / INPUT_LIFT to the checkout that has them. A different list: VFS_SEED_OVERRIDE=<abs file> (その場合 vfs_seed.h が全ファイルを確認し、誤りは終了 2)." >&2
        exit 2
    fi
fi
for f in "$FLE" "$DSP" "$B2D" "$QFC" "$FT" "$ILT" "$DSP_BASE_SEEDS" "$B2D_SEEDS_OFF" "$HERE/seeds/ft-seeds-4b00000-c1.txt" "$HERE/seeds/iltools-6000000-seeds400.txt" \
         "$HERE/hooks/qfc-hook-points.txt" "$HERE/hooks/iltools-hook-points.txt" "$FLE_HOOKS" "$VFS_SEED" "$VFS_SEED_ROOT/lift/out-inventory/extracted/fonts/ILGlyphsEx.ilfont" \
         "$VFS_SEED_ROOT/lift/out-fruity11/inputs/GM_CCs.txt"; do
    [ -r "$f" ] || die "cannot read $f"
done

unset FLE_SEEDS DSP_SEEDS REG_SEED AUTH_CLAIMS FLE_CALL_FRUITY
B2D_SEEDS=$B2D_SEEDS_OFF
FT_SEEDS=$HERE/seeds/ft-seeds-4b00000-c1.txt
SITES=""
if [ "$MODE" = on ] || [ "$IDLE1" = 1 ]; then
    FRUITY_SECONDS=600
    DSP_SEEDS_MODE=$HERE/seeds/dsp-ippv2-2000000-seeds-jt.txt
    if [ "$IDLE1" = 1 ]; then
        FRUITY_SECONDS=900
        DSP_SEEDS_MODE=$HERE/seeds/dsp-ippv2-2000000-seeds-idle1.txt
        export FLE_FRUITY_NO_IDLE_STOP=1
        [ "$MODE" = on-seh ] && export FLE_SEH_HANDLERS=0x112c0
    fi
    export FLE_CALL_FRUITY=1
    export REG_SEED=$HERE/seeds/reg-seed-nsis.tsv:$HERE/seeds/reg-seed-app.tsv
    export AUTH_CLAIMS=$HERE/seeds/authenticode-claims.tsv
    export FLE_SEEDS=$HERE/seeds/flengine-seeds-thunks.txt
    export DSP_SEEDS=${DSP_SEEDS_OVERRIDE:-$DSP_SEEDS_MODE}
    export FLE_FRUITY_MAX_CALLS=${FLE_FRUITY_MAX_CALLS:-400000000}
    export FLE_FRUITY_SECONDS=${FLE_FRUITY_SECONDS:-$FRUITY_SECONDS}
    B2D_SEEDS=$HERE/seeds/blend2d-4700000-seeds964.txt
    FT_SEEDS=$HERE/seeds/ft-seeds-4b00000-c2.txt
    SITES=$HERE/hooks/blend2d-no-jit-sites.txt
    for f in "$FLE_SEEDS" "$DSP_SEEDS" "$B2D_SEEDS" "$FT_SEEDS" "$SITES" "$HERE/seeds/reg-seed-nsis.tsv" "$HERE/seeds/reg-seed-app.tsv" "$AUTH_CLAIMS"; do
        [ -r "$f" ] || die "cannot read $f"
    done
else
    [ -z "${DSP_SEEDS_OVERRIDE:-}" ] || die "DSP_SEEDS_OVERRIDE is only for on / on-0129 / on-seh"
    export FLE_CALL_FRUITY=0
fi

exec bash "$HERE/a4-multi.sh" \
    "dsp_ippv2_x64.dll:$DSP:0x2000000:$DSP_BASE_SEEDS" \
    "blend2d.dll:$B2D:0x4700000:$B2D_SEEDS:pdata${SITES:+::$SITES}" \
    "quickfontcache_x64.dll:$QFC:0x1d00000::pdata:$HERE/hooks/qfc-hook-points.txt" \
    "freetype_x64.dll:$FT:0x4b00000:$FT_SEEDS:pdata" \
    "iltools_x64.dll:$ILT:0x6000000:$HERE/seeds/iltools-6000000-seeds400.txt:pdata:$HERE/hooks/iltools-hook-points.txt"
