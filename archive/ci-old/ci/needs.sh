#!/bin/bash
# ci/needs.sh OUTFILE - from the cache lookups (env H_<STAGE>=true|false) to what the build job has to restore and to run. The Dockerfile rule: the deepest stage that is cached decides how much of the
# chain below it is needed at all. Pure function of the environment (try it: H_BUNDLE=false H_LINK=true H_SEED=true ci/needs.sh).
#   bundle hit            -> no build job
#   link / seed miss      -> the lift output is needed (restored from its cache, or made); a lift miss needs the extraction, the lifter and (for the C -> wasm stages) the upstream Blend2D
#   seed hit and link hit -> only the bundle step runs (it still needs the extraction: the script checks the engine DLL first)
# (0/1 arithmetic below: x * y = and, x + y > 0 = or, 1 - x = not)
# shellcheck disable=SC2034  # the variables are read through ${!v} at the end
set -u -o pipefail
OUT=${1:-/dev/stdout}
h() { [ "${1:-}" = true ] && echo 1 || echo 0; }
HB=$(h "${H_BUNDLE:-}"); HL=$(h "${H_LINK:-}"); HS=$(h "${H_SEED:-}"); HI=$(h "${H_LIFT:-}"); HX=$(h "${H_EXTRACT:-}"); HF=$(h "${H_FELIFT:-}"); HD=$(h "${H_B2D:-}")
or() { [ $(( $1 + $2 )) -gt 0 ] && echo 1 || echo 0; }
need_build=$((1 - HB))
need_link=$((need_build * (1 - HL)))
need_seed=$((need_build * (1 - HS)))
need_lift_out=$(or "$need_link" "$need_seed")
run_lift=$((need_lift_out * (1 - HI)))
restore_lift=$((need_lift_out * HI))
need_extract=$need_build   # app/scripts/build-engine-bundle.sh checks binaries/win64/FLEngine_x64.dll (and its SHA-256) before ANY stage, so every build needs the extraction (74 MB)
restore_extract=$((need_extract * HX))
run_extract=$((need_extract * (1 - HX)))
# the lifter binaries are needed to lift (run_lift) and by the link stage: its hybn0 and hybrid steps run fe-wasmgen (run 41986214303: the lift output was restored, the link had to run,
# no fe-wasmgen was there and the offline cargo build in the runner failed)
need_felift=$(or "$run_lift" "$need_link")
run_felift=$((need_felift * (1 - HF)))
restore_felift=$((need_felift * HF))
need_b2d=$(or "$need_link" "$run_lift")
restore_b2d=$((need_b2d * HD))
run_b2d=$((need_b2d * (1 - HD)))
need_emsdk=$(or "$run_b2d" "$need_link")
restore_link=$((need_build * HL))
restore_seed=$((need_build * HS))
{
  for v in need_build need_link need_seed need_lift_out run_lift restore_lift need_extract restore_extract run_extract need_felift run_felift restore_felift need_b2d restore_b2d run_b2d need_emsdk restore_link restore_seed; do
    [ "${!v}" = 1 ] && echo "$v=true" || echo "$v=false"
  done
} > "$OUT"
