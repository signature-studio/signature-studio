#!/usr/bin/env bash
# The upstream Blend2D library is cached by the key of ci/plan.sh. Every input of its build has to be in that key, or a library built before the input changed is restored
# (run 41982180403: the compiler flags -matomics -mbulk-memory were added to the build but not to the key; the cached library without atomics was restored and the shared-memory link failed).
set -eu
cd "$(dirname "$0")/.."
fail=0
ok() { echo "ok   $*"; }
bad() { echo "FAIL $*"; fail=1; }
b2d_line=$(grep -n 'build_upstream.sh' ci/stage-wasm.sh | grep EMCC_CFLAGS | head -1 || true)
[ -n "$b2d_line" ] && ok "stage-wasm.sh builds Blend2D with EMCC_CFLAGS" || bad "no EMCC_CFLAGS on the build_upstream.sh line of ci/stage-wasm.sh"
case "$b2d_line" in *'EMCC_CFLAGS="$B2D_CFLAGS"'*) ok "the build takes its flags from \$B2D_CFLAGS (no literal copy)";; *) bad "the b2d build line does not use \$B2D_CFLAGS";; esac
grep -q '^KB2D=.*cflags=\$B2D_CFLAGS' ci/plan.sh && ok "the b2d key has the compiler flags" || bad "KB2D in ci/plan.sh does not contain cflags=\$B2D_CFLAGS"
grep -q '^B2D_CFLAGS=.*-matomics.*-mbulk-memory' ci/lib/common.sh && ok "B2D_CFLAGS is defined with atomics and bulk memory" || bad "B2D_CFLAGS is not defined in ci/lib/common.sh"
for v in B2D_OPT B2D_PATCH; do
  grep -q "^KB2D=.*\\\$$v" ci/plan.sh && ok "the b2d key has $v" || bad "the b2d key lacks $v"
done
[ "$fail" = 0 ] && echo "test-b2d-key: all checks hold" || { echo "test-b2d-key: FAILED"; exit 1; }
