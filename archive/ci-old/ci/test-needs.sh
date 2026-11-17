#!/bin/bash
# ci/test-needs.sh - checks ci/needs.sh over all 128 combinations of cache hits (7 stages): the invariants of the "restore or run" decision
set -u -o pipefail
here=$(cd "$(dirname "$0")" && pwd); fail=0; n=0
get() { grep "^$1=" "$2" | cut -d= -f2; }
for mask in $(seq 0 127); do
  stages=(BUNDLE LINK SEED LIFT EXTRACT FELIFT B2D); env=()
  for i in "${!stages[@]}"; do if [ $(( (mask >> i) & 1 )) = 1 ]; then env+=("H_${stages[$i]}=true"); else env+=("H_${stages[$i]}=false"); fi; done
  out=$(mktemp); env "${env[@]}" bash "$here/needs.sh" "$out"
  v() { get "$1" "$out"; }
  n=$((n + 1)); err=""
  xor() { [ "$(v "$1")" != "$(v "$2")" ] || [ "$(v "$1")" = false ]; }   # not both true
  if [[ " ${env[*]} " == *" H_BUNDLE=true "* ]]; then
    for k in need_build need_link need_seed need_lift_out run_lift restore_lift need_extract restore_extract run_extract need_felift run_felift restore_felift need_b2d restore_b2d run_b2d need_emsdk restore_link restore_seed; do [ "$(v $k)" = false ] || err="bundle hit but $k=true"; done
  else
    [ "$(v need_build)" = true ] || err="need_build"
    [ "$(v need_extract)" = true ] || err="$err extract is always needed for a build"
    [ "$(v restore_extract)" != "$(v run_extract)" ] || err="$err extract restore xor run"
    [ "$(v restore_link)" != "$(v need_link)" ] || err="$err link restore xor run"
    [ "$(v restore_seed)" != "$(v need_seed)" ] || err="$err seed restore xor run"
    if [ "$(v need_lift_out)" = true ]; then [ "$(v restore_lift)" != "$(v run_lift)" ] || err="$err lift restore xor run"; else [ "$(v run_lift)$(v restore_lift)" = falsefalse ] || err="$err lift not needed but used"; fi
    if [ "$(v need_felift)" = true ]; then [ "$(v restore_felift)" != "$(v run_felift)" ] || err="$err felift restore xor run"; else [ "$(v run_felift)$(v restore_felift)" = falsefalse ] || err="$err felift not needed but used"; fi
    if [ "$(v run_lift)" = true ] || [ "$(v need_link)" = true ]; then [ "$(v need_felift)" = true ] || err="$err the lifter binaries are needed (lift run or link: fe-wasmgen)"; else [ "$(v need_felift)" = false ] || err="$err felift needed without a user"; fi
    if [ "$(v need_b2d)" = true ]; then [ "$(v restore_b2d)" != "$(v run_b2d)" ] || err="$err b2d restore xor run"; else [ "$(v run_b2d)$(v restore_b2d)" = falsefalse ] || err="$err b2d not needed but used"; fi
    if [ "$(v need_emsdk)" = true ]; then [ "$(v need_link)" = true ] || [ "$(v run_b2d)" = true ] || err="$err emsdk without a user"; fi
    if [ "$(v need_link)" = true ] || [ "$(v run_b2d)" = true ]; then [ "$(v need_emsdk)" = true ] || err="$err emsdk missing"; fi
    xor need_link restore_link >/dev/null
  fi
  rm -f "$out"
  if [ -n "$err" ]; then echo "FAIL ${env[*]}: $err"; fail=1; fi
done
[ "$fail" = 0 ] && echo "test-needs: $n combinations, all invariants hold" || exit 1
