#!/bin/bash
# wasm-final0129 (decision 0134): the FRUITY-on configuration of decision 0129's final state (idle1's tools/fruity-config.sh with the paths of this work dir): 6 modules, dsp seeds idle1 (16,133 lines),
# VFS seed vfs-seed-idle1.tsv (host files under lift/out-idle1/inputs, read only), FLE_FRUITY_NO_IDLE_STOP=1 (set by the caller of the native run).  Same list as verification.md section 10.
. /home/ubuntu/work/signature-studio-develop/lift/out-wasm0129/src/docs/investigations/wasm-final0129/tools/env.sh
export FLE=$R/binaries/win64/FLEngine_x64.dll
export FLE_HOOKS=$H/hooks/flengine-hook-points.txt
export EXPECT_FLE_SHA=skip
export VFS_SEED=$H/seeds/vfs-seed-idle1.tsv
export VFS_SEED_ROOT=$R
export FLE_CALL_FRUITY=1
export REG_SEED=$H/seeds/reg-seed-nsis.tsv:$H/seeds/reg-seed-app.tsv
export AUTH_CLAIMS=$H/seeds/authenticode-claims.tsv
export FLE_SEEDS=$H/seeds/flengine-seeds-thunks.txt
export FLE_FRUITY_MAX_CALLS=${FLE_FRUITY_MAX_CALLS:-4000000000}
export FLE_FRUITY_SECONDS=${FLE_FRUITY_SECONDS:-900}
DSPSEEDS=$H/seeds/dsp-ippv2-2000000-seeds-idle1.txt
FRUITY_MODS=(
  "dsp_ippv2_x64.dll:$L/out-ext/dsp_ippv2_x64.dll:0x2000000:$DSPSEEDS"
  "blend2d.dll:$L/out-blend2d/Blend2D.dll:0x4700000:$H/seeds/blend2d-4700000-seeds964.txt:pdata::$H/hooks/blend2d-no-jit-sites.txt"
  "quickfontcache_x64.dll:$L/out-qfc/quickfontcache_x64.dll:0x1d00000::pdata:$H/hooks/qfc-hook-points.txt"
  "freetype_x64.dll:$L/out-wallQ/freetype/freetype_x64.dll:0x4b00000:$H/seeds/ft-seeds-4b00000-c2.txt:pdata"
  "iltools_x64.dll:$L/out-inventory/extracted/ILTools_x64.dll:0x6000000:$H/seeds/iltools-6000000-seeds400.txt:pdata:$H/hooks/iltools-hook-points.txt"
)
