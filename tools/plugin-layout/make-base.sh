#!/bin/sh
# Issue #1379: the bundle of the placement before Issue #1379, for tools/delivery-split/measure.mjs to compare with the new one. Run it from the
# repository root, after `fe-build engine` (it reads the objects, exports.rsp, the DLL dir, the seed pack and the plugin tables of lift/out-app/).
# It relinks the same objects with the old link flags (GLOBAL_BASE 268435456 = 0x10000000, INITIAL_MEMORY 671088640: the values of link.rsp before
# Issue #1379; MAXIMUM_MEMORY is unchanged) and makes the same downstream steps as fe-build (wasm-opt, hybrid, glue, bundle) under lift/out-base/.
# The seed pack, the DLL dir and exports.rsp do not depend on the placement and are read from lift/out-app/. The host trees are symlinks.
# Nothing is written outside lift/out-base/. Exit code 0 on success, 1 on failure (the failed step is named on stderr).
fail() { echo "make-base: $*" >&2; exit 1; }
[ -f lift/Cargo.toml ] || fail "run from the repository root (lift/Cargo.toml is not here)"
[ -f lift/out-app/web/engine.wasm ] || fail "no engine of fe-build engine (lift/out-app/web/engine.wasm)"
[ -f lift/out-app/exports.rsp ] || fail "no lift/out-app/exports.rsp (fe-build engine)"
LC_ALL=C
export LC_ALL
emcc_dir=$(dirname "$(command -v emcc)") || fail "emcc is not on PATH"
PATH="$emcc_dir/../bin:$PATH"
export PATH

rm -rf lift/out-base/web lift/out-base/bundle lift/out-base/hyb lift/out-base/xmod
mkdir -p lift/out-base/web lift/out-base/xmod || fail "mkdir lift/out-base"
ln -sfn ../out-app/host lift/out-base/host || fail "host link"
ln -sfn ../out-app/host-extra lift/out-base/host-extra || fail "host-extra link"

# 1. the link: link.rsp with the old placement and the outputs under lift/out-base/
sed -e 's|^-sGLOBAL_BASE=.*|-sGLOBAL_BASE=268435456|' -e 's|^-sINITIAL_MEMORY=.*|-sINITIAL_MEMORY=671088640|' -e 's|lift/out-app/web/|lift/out-base/web/|g' \
  lift/harness/web/link.rsp > lift/out-base/link.rsp || fail "link.rsp"
grep -q '^-sGLOBAL_BASE=268435456$' lift/out-base/link.rsp || fail "link.rsp has no GLOBAL_BASE line"
# the objects in the order of fe-build's link stage: the harness, the modules, the fle units, the runtime; each group sorted
set --
for d in h mod fle rt; do
  for o in lift/out-app/obj/$d/*.o; do set -- "$@" "$o"; done
done
emcc "@lift/out-base/link.rsp" "@lift/out-app/exports.rsp" "$@" || fail "link"
[ -f lift/out-base/web/flengine6.wasm ] || fail "link: no lift/out-base/web/flengine6.wasm"

# 2. wasm-opt of the C part (the arguments of fe-build's wasm-opt stage, under lift/out-base/)
sed 's|lift/out-app/web/|lift/out-base/web/|g' lift/harness/web/wasm-opt-c.cmd > lift/out-base/wasm-opt-c.cmd || fail "wasm-opt-c.cmd"
set -- wasm-opt
while IFS= read -r line; do set -- "$@" "$line"; done < lift/out-base/wasm-opt-c.cmd
"$@" || fail "wasm-opt"

# 3. hybrid: the work directory starts as a copy of fe-build's (the hybrid-n0 results are in it; exports.rsp is from it)
cp -a lift/out-app/hyb lift/out-base/hyb || fail "copy of lift/out-app/hyb"
sed -e 's|^lift/out-app/web/|lift/out-base/web/|' -e 's|^lift/out-app/hyb$|lift/out-base/hyb|' -e 's|^lift/out-app/xmod$|lift/out-base/xmod|' \
  lift/out-app/plugins/hybrid.cmd > lift/out-base/hybrid.cmd || fail "hybrid.cmd"
set -- lift/target/release/fe-wasmgen
while IFS= read -r line; do set -- "$@" "$line"; done < lift/out-base/hybrid.cmd
"$@" || fail "hybrid"
[ -f lift/out-base/web/engine.wasm ] || fail "hybrid: no lift/out-base/web/engine.wasm"

# 4. glue: the two rules of glue.sed (ERE), as fe-build's glue step
sed -E -f lift/harness/web/glue.sed lift/out-base/web/flengine6.js > lift/out-base/web/engine-glue.js || fail "glue"

# 5. bundle: the pack.args of fe-build without the external plugins' lazy files (as keep_lazies), under lift/out-base/. The seed pack is
# lift/out-app/seed/seed.bin (it does not depend on the placement).
awk '$0 == "--lazy-file" { getline v; if (index(v, "lift/out-app/plugins/") == 1) next; print "--lazy-file"; print v; next } { print }' \
  lift/out-app/plugins/pack.args > lift/out-base/pack.args || fail "pack.args"
sed -e 's|lift/out-app/bundle|lift/out-base/bundle|' -e 's|lift/out-app/web/|lift/out-base/web/|g' -e 's|lift/out-app/xmod/|lift/out-base/xmod/|g' \
  lift/out-base/pack.args > lift/out-base/pack.args.new && mv lift/out-base/pack.args.new lift/out-base/pack.args || fail "pack.args paths"
ver=$(sha256sum lift/out-base/web/engine.wasm | cut -c1-12) || fail "sha256 of the engine"
set -- node app/scripts/pack-assets.mjs --version "$ver"
while IFS= read -r line; do set -- "$@" "$line"; done < lift/out-base/pack.args
"$@" > lift/out-base/bundle.log 2>&1 || fail "bundle (see lift/out-base/bundle.log)"
[ -f lift/out-base/bundle/raw/fl-Oz/manifest.json ] || fail "bundle: no manifest"
echo "make-base: lift/out-base/bundle/raw/fl-Oz (GLOBAL_BASE 268435456, INITIAL_MEMORY 671088640, engine.wasm sha256 $ver...)"
