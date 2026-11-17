#!/bin/bash
# lift/harness/parity/make-test-tree.sh PRODUCT_TREE PRODUCT_WORK TEST_TREE TEST_WORK
# The wasm TEST build (decision 0243): the product wasm reads no diagnostic environment variable (FLE_TEST_GETENV of lift/runtime/fle_test_env.h is a null constant there) and has no file system, so the node rows (the 16 start-up snapshots
# with FLE_G3_SCRIPT, the reference project with FLE_FILEDLG_PICK, lift/harness/parity/node-bundle-run.mjs) run on a build made in a COPY of the tree: that copy has lift/runtime/fle_test_env.h replaced by lift/harness/parity/fle_test_env_wasm.h
# (getenv), the glue has a file system, and the objects of the units that include the header are compiled again (the others are the product's objects, hard-linked; the link and the hybrid merge are redone). There is no -D and no flag in
# the product tree: the difference is a file in a copy. Then:
#   WORK=TEST_WORK HOST_ROOT=... B2D_UPSTREAM_LIB=... B2D_UPSTREAM_SRC=... bash TEST_TREE/app/scripts/build-engine-bundle.sh wasm weblink wasmopt hybrid seed bundle
# The product build never runs this and has no switch for it.
set -eu
PT=${1:?product tree}; PW=${2:?product work dir}; TT=${3:?test tree}; TW=${4:?test work dir}
[ ! -e "$TT" ] && [ ! -e "$TW" ] || { echo "make-test-tree: $TT or $TW exists" >&2; exit 2; }
cp -a "$PT" "$TT"
python3 - "$TT" <<'PY'
import sys
T = sys.argv[1]
def rep(n, old, new):
    p = T + '/' + n
    s = open(p).read()
    assert s.count(old) == 1, (n, old)
    open(p, 'w').write(s.replace(old, new))
rep('lift/harness/web/fruity/link-web.sh', '-sEXPORT_ES6=1 -sFILESYSTEM=0', '-sEXPORT_ES6=1 -sFORCE_FILESYSTEM=1')
rep('lift/harness/web/fruity/link-web.sh', '-sEXPORTED_RUNTIME_METHODS=ENV,wasmExports,stackRestore', '-sEXPORTED_RUNTIME_METHODS=FS,ENV,wasmExports,stackRestore')
PY
cp "$TT/lift/harness/parity/fle_test_env_wasm.h" "$TT/lift/runtime/fle_test_env.h"
mkdir -p "$TW"
for d in "$PW"/*; do
  b=$(basename "$d")
  case $b in hyb|bundle|in) cp -a "$d" "$TW/$b" ;; *) cp -al "$d" "$TW/$b" ;; esac
done
O=$TW/w-Oz
rm -rf "$O/web" "$TW/bundle" "$TW/in/seedpack.bin" "$TW/in/dll"
rm -rf "$TW/hyb/dir" "$TW/hyb/stripped.test"
# the units that include fle_test_env.h are compiled again (the wasm stage keeps an object that exists)
for f in $(grep -rl 'fle_test_env.h' "$PT/lift/runtime" "$PT/lift/harness" --include=*.c --include=*.cpp); do
  b=$(basename "$f"); b=${b%.*}
  rm -f "$O/obj/rt/$b.o" "$O/obj/h-fruity/$b.o" "$O/obj/b2d/$b.o"
done
echo "make-test-tree: $TT and $TW are ready"
