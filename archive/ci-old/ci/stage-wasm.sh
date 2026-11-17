#!/bin/bash
# ci/stage-wasm.sh b2d|objects|link - the stage "wasm build" (C -> wasm objects, final link, wasm-opt).
#   b2d:     upstream Blend2D for wasm (lift/harness/b2d1/build_upstream.sh: fetched at a pinned commit and tarball SHA-256, patched with $B2D_PATCH, built at $B2D_OPT with atomics and bulk memory like every other object)
#   objects: app/scripts/build-engine-bundle.sh `wasm`: one object per translation unit; a unit whose object exists (restored from the cache) is skipped by build_nmod_wasm.sh
#   link:    `hybn0 weblink wasmopt hybrid` (fe-wasmgen over the lists with dummy addresses, the emcc link of all objects into flengine6.{js,wasm}, wasm-opt -Oz of the C part, the direct wasm of fe-wasmgen merged into it -> flengine6.opt.wasm)
set -u -o pipefail
. "$(dirname "$0")/lib/common.sh"
case "${1:-}" in
  b2d)
    t0=$SECONDS
    patch="$REPO/$B2D_PATCH"
    rm -rf "$B2D_DIR"
    EMCC_CFLAGS="$B2D_CFLAGS" PATH=$EMSDK_DIR:$EMSDK_DIR/upstream/emscripten:$PATH OUTDIR=$B2D_DIR TARGET=wasm OPT="$B2D_OPT" JOBS=${CI_JOBS:-$(jobs_default)} PATCH="$patch" bash "$REPO/lift/harness/b2d1/build_upstream.sh" || die "upstream Blend2D failed"
    log "b2d done in $((SECONDS - t0)) s"
    ;;
  objects)
    [ -f "$B2D_DIR/.placeholder" ] && die "the upstream Blend2D is a placeholder: run ci/stage-wasm.sh b2d first"
    t0=$SECONDS
    run_bb wasm || die "wasm stage failed (see $ENGINE_WORK/logs/wasm-$VARIANT-*.err)"
    log "objects done in $((SECONDS - t0)) s"
    ;;
  link)
    t0=$SECONDS
    run_bb hybn0 weblink wasmopt hybrid || die "hybn0 weblink wasmopt hybrid failed"
    web=$ENGINE_WORK/w-$VARIANT/web
    if [ -s "$web/flengine6.opt.wasm" ]; then
      # the bundle packs flengine6.opt.wasm and only checks that flengine6.wasm is not empty: keep a note instead of the unoptimised 300 MB (it is not worth a cache entry)
      printf 'unoptimised wasm dropped by ci/stage-wasm.sh; the bundle packs flengine6.opt.wasm\n' > "$web/flengine6.wasm"
    fi
    ls -l "$ENGINE_WORK/w-$VARIANT/web/"
    log "link done in $((SECONDS - t0)) s"
    ;;
  *) die "usage: stage-wasm.sh b2d|objects|link" ;;
esac
