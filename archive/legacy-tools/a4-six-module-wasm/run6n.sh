#!/bin/bash
# usage: run6n.sh OUTDIR   6-module configuration of decision 0113 with the COMMITTED a4-multi.sh of the PRISTINE archive (/tmp/opencode/wasm6/src).
R=/home/ubuntu/work/signature-studio-develop/lift
S=/tmp/opencode/wasm6/src/lift
H=$S/harness
export PATH=${WRAPDIR:-/tmp/opencode/wasm6/tools/wrap}:$PATH
export OUT=$1
export FEBIN=/tmp/opencode/wasm6/febin
export FLE=/home/ubuntu/work/signature-studio-develop/binaries/win64/FLEngine_x64.dll
export FLE_HOOKS=$H/hooks/flengine-hook-points.txt
export EXPECT_FLE_SHA=e047beeb44f16aa406a6ec254cc6603be192044206195c62ea2d459ec88bfdd2
export JOBS=${JOBS:-2}
export VFS_SEED=$H/seeds/vfs-seed-fonts.tsv
export VFS_SEED_ROOT=/home/ubuntu/work/signature-studio-develop
exec bash $H/a4-multi.sh \
  "dsp_ippv2_x64.dll:$R/out-ext/dsp_ippv2_x64.dll:0x2000000:$R/out-three/three/dll/seeds-idx0-plus-initterm2.txt" \
  "blend2d.dll:$R/out-blend2d/Blend2D.dll:0x4700000:$R/out-seed58/seeds/seeds58-4700000.txt:pdata" \
  "quickfontcache_x64.dll:$R/out-qfc/quickfontcache_x64.dll:0x1d00000::pdata:$H/hooks/qfc-hook-points.txt" \
  "freetype_x64.dll:$R/out-wallQ/freetype/freetype_x64.dll:0x4b00000:$H/seeds/ft-seeds-4b00000-c1.txt:pdata" \
  "iltools_x64.dll:$R/out-inventory/extracted/ILTools_x64.dll:0x6000000:$H/seeds/iltools-6000000-seeds400.txt:pdata:$H/hooks/iltools-hook-points.txt"
