#!/bin/bash
# Environment of the test suite (decision 0149). Source it (". harness/test-env.sh") or run it ("harness/test-env.sh") to see what it found.
#
# Several tests read product files that are not in git (the DLLs, the seeds). Each one has a default path under lift/ or binaries/, and an environment
# variable that overrides it; a test whose file is missing prints SKIPPED and passes (or, when the variable is set to a missing file, fails). This script
# sets the variables that need setting and reports every input it could not find, so that a SKIP is never a surprise.
#
# DSP_IPPV2_PE is deliberately left unset: fe-pe/tests/rebase.rs, jt_cpu.rs and gs_stacklimit.rs need the un-rebased dsp_ippv2_x64.dll (image base
# 0x180000000), and two tests of fe-decode/tests/msvc_switch.rs read the same variable but need the copy rebased to 0x2000000 (otherwise they print
# "SKIPPED: the image base is 0x180000000" and pass without checking anything). Unset, each test uses its own default path, which is the right file.
#
# FLE_INPUT_ROOT: a checkout that has the inputs (default: this checkout). In a worktree or a clean export, `FLE_INPUT_ROOT=/path/to/main/checkout
# harness/test-env.sh --link` makes symlinks to the inputs at their default places (they are all git-ignored paths).
_here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
_lift=$(cd "$_here/.." && pwd)
_repo=$(cd "$_lift/.." && pwd)
_root=${FLE_INPUT_ROOT:-$_repo}
_link=0; [ "${1:-}" = "--link" ] && _link=1
_missing=0
_find() { # name relative-path (from the repo root)
    local p="$_repo/$2"
    if [ ! -e "$p" ] && [ -e "$_root/$2" ]; then
        if [ $_link = 1 ]; then mkdir -p "$(dirname "$p")" && ln -s "$_root/$2" "$p"; else p="$_root/$2"; fi
    fi
    if [ -e "$p" ]; then printf '%s' "$p"; else printf ''; _missing=$((_missing + 1)); echo "test-env: MISSING input $2 (tests that need it will SKIP or fail)" >&2; fi
}
_set() { local v; v=$(_find "$1" "$2"); if [ -n "$v" ]; then export "$1=$v"; fi; }
_set FLENGINE_PE binaries/win64/FLEngine_x64.dll
export FLENGINE_DLL=${FLENGINE_PE:-}; [ -z "$FLENGINE_DLL" ] && unset FLENGINE_DLL
_set ILTOOLS_DLL lift/out-inventory/extracted/ILTools_x64.dll
_set BLEND2D_PE lift/out-seed58/s58/dll/Blend2D.4700000.dll
_set DSP_IPPV2_PE_2000000 lift/out-jtb2d90/three/dll/dsp_ippv2_x64.2000000.dll
_set FREETYPE_PE lift/out-wallQ/freetype/freetype_x64.4b00000.dll
_set FL64_EXE lift/out-fruity6/inputs/FL64.exe
unset DSP_IPPV2_PE
# inputs that the tests open by a fixed relative path (checked only; --link makes the symlink)
for rel in lift/out-ext/dsp_ippv2_x64.dll lift/out-blend2d/Blend2D.dll lift/out-qfc/quickfontcache_x64.dll lift/out-modules/seeds-896.txt lift/out-vfs/seeds-896.txt lift/out-pdata/seeds-pdata.txt lift/out-ctype/tables/in-units.txt; do
    _find X "$rel" > /dev/null
done
export PYTHONDONTWRITEBYTECODE=1
export CARGO_INCREMENTAL=${CARGO_INCREMENTAL:-0} CARGO_PROFILE_DEV_DEBUG=${CARGO_PROFILE_DEV_DEBUG:-0}
export FLE_TEST_MISSING_INPUTS=$_missing
if [ "${BASH_SOURCE[0]}" = "$0" ]; then
    echo "test-env: repo $_repo, inputs from $_root, $_missing input(s) missing"
    env | grep -E '^(FLENGINE_PE|FLENGINE_DLL|ILTOOLS_DLL|BLEND2D_PE|DSP_IPPV2_PE_2000000|FREETYPE_PE|FL64_EXE)=' | sort
fi
