#!/bin/bash
# ship1 (decision 0189): the hybrid (C path + direct wasm of fe-wasmgen) on top of the F1xH bundle. Derived from hyb4's h1/hyb-build.sh + tools/mk-prod.sh (decisions 0169, 0179, 0185), with the paths of THIS tree:
# the C part is the F1xH build in lift/out-ship1/$BASE (units, objects), the module arguments of fe-wasmgen are the ones fe-lift got for F1x (Blend2D: wrapper seeds, no --pdata).
# usage: hyb.sh STAGE...    stages: prep n0 [bb: wasm weblink] nodelink opt gen merge
# env:   HYB (work dir name under lift/out-ship1, default H1), BASE (default A), LISTS (default lift/out-ship1/lists/shiph: lowered-<m>.txt per module), FEW_EXTRA (default "--static-calls --sync-saved")
set -u -o pipefail
R=/home/ubuntu/work/signature-studio-develop; W=$R/lift/out-ship1
HYB=${HYB:-H1}; BASE=${BASE:-A}; LISTS=${LISTS:-$W/lists/shiph}
A=$W/$BASE; WORK=$W/$HYB; SRC=$W/src; L=$SRC/lift; HS=$L/harness
FEW=${FEW:-$W/cargo-target/release/fe-wasmgen}
CIMP=$W/lists/c-imports-engine.txt
export PATH=$W/tools/shim:/opt/emsdk:/opt/emsdk/upstream/emscripten:/opt/emsdk/upstream/bin:$HOME/.local/bin:$PATH PYTHONDONTWRITEBYTECODE=1
export EM_CACHE=$W/A/emcache
TAGDIR=$(ls -d $A/onat/v-* | head -1); TAG=$(basename $TAGDIR | sed 's/^v-//'); DLLD=$TAGDIR/dll
OBJ=$WORK/w-Oz/obj; AOBJ=$A/w-Oz/obj; WEB=$WORK/w-Oz/web
MODS="fle dsp b2d qfc ft ilt"
FEW_FLAGS="--mx-window --trap-vex --no-elide --lazy-ext --hybrid-merge --c-imports $CIMP --mem-import c.memory ${FEW_EXTRA:---static-calls --sync-saved}"
FEAT_OPT="--enable-bulk-memory --enable-exception-handling --enable-sign-ext --enable-mutable-globals --enable-nontrapping-float-to-int --enable-multivalue --enable-reference-types --enable-simd --enable-threads --enable-tail-call --enable-extended-const"
modargs() {   # the arguments fe-lift got for F1x (gen-direct.sh with B2D_SWAP=1 HLE_DSP=1): Blend2D = wrapper seeds, no --pdata
  PDATA="--pdata"
  case $1 in
   fle) PE=$R/binaries/win64/FLEngine_x64.dll; SEEDS="--seeds $HS/seeds/flengine-seeds-thunks.txt" ;;
   dsp) PE=$DLLD/dsp_ippv2_x64.2000000.dll; SEEDS="--seeds $HS/seeds/dsp-ippv2-2000000-seeds-idle1.txt"; PDATA="" ;;
   b2d) PE=$DLLD/Blend2D.4700000.dll; SEEDS="--seeds $HS/b2d1/blend2d-4700000-seeds-wrapper.txt"; PDATA="" ;;
   qfc) PE=$DLLD/quickfontcache_x64.1d00000.dll; SEEDS="" ;;
   ft)  PE=$DLLD/freetype_x64.4b00000.dll; SEEDS="--seeds $HS/seeds/ft-seeds-4b00000-c2.txt" ;;
   ilt) PE=$DLLD/ILTools_x64.6000000.dll; SEEDS="--seeds $HS/seeds/iltools-6000000-seeds400.txt" ;;
  esac
}
lowered() { [ -s $LISTS/lowered-$1.txt ]; }
stage_prep() {
  mkdir -p $WORK/{logs,tmp,in,wsrc/src,wsrc/v-$TAG/src} $OBJ/{rt,h-fruity,fle,mod,log,bin,h,o1/obj,o1/src,b2d} $WEB
  [ -e $WORK/onat ] || ln -s $A/onat $WORK/onat
  [ -e $WORK/cargo-target ] || ln -s $W/cargo-target $WORK/cargo-target
  : > $WORK/logs/strip.txt
  local f u out sub
  for f in $A/wsrc/src/fle-[0-9][0-9].c $A/wsrc/v-$TAG/src/m[0-9]-[0-9][0-9].c; do
    u=$(basename $f .c); out=$WORK/wsrc/src/$u.c; sub=fle; case $u in m*) out=$WORK/wsrc/v-$TAG/src/$u.c; sub=mod ;; esac
    rm -f $out
    python3 $W/tools/strip_units2.py $f $LISTS/lowered-all.txt $out >> $WORK/logs/strip.txt
    if [ ! -e $out ]; then ln -f $f $out; ln -f $AOBJ/$sub/$u.o $OBJ/$sub/$u.o; fi   # nothing dropped: the unit and its object are the base's (hard links)
  done
  cp $A/wsrc/v-$TAG/src/common.c $WORK/wsrc/v-$TAG/src/common.c; cp $A/wsrc/v-$TAG/modules.txt $WORK/wsrc/v-$TAG/modules.txt
  ln -f $AOBJ/mod/common.o $OBJ/mod/common.o
  for d in rt h-fruity; do for o in $AOBJ/$d/*.o; do ln -f $o $OBJ/$d/$(basename $o); done; done
  ln -f $AOBJ/b2d/b2d_swap.o $OBJ/b2d/b2d_swap.o
  awk '{d+=$3; k+=$5} END {print "units: dropped " d " functions, kept " k}' $WORK/logs/strip.txt
  echo "units changed: $(awk '$3>0' $WORK/logs/strip.txt | wc -l) of $(wc -l < $WORK/logs/strip.txt)"
  # hybrid_glue.c: fle_hyb_dispatch, fle_hyb_gs_ld8, fle_hyb_mx ... into the harness objects (the flags of the harness stage: -O1, FLE_LANDING_FORCE, SUPPORT_LONGJMP=wasm, the low register file)
  if [ ! -e $OBJ/h-fruity/hybrid_glue.o ]; then
    ( export TMPDIR=$WORK/tmp; EMCC_CFLAGS="-DFLE_LOWREGS -DFLE_LOWREGS_X" emcc -c -std=gnu11 -O1 -Wall -Wextra -I$L/runtime -I$L/harness -DFLE_LANDING_FORCE -sSUPPORT_LONGJMP=wasm $HS/hyb1/hybrid_glue.c -o $OBJ/h-fruity/hybrid_glue.o ) 2>&1 | tail -3
  fi
  ls -l $OBJ/h-fruity/hybrid_glue.o
}
stage_n0() {   # fe-wasmgen with dummy addresses: the C functions the direct code imports -> exports.rsp
  local m
  : > $WORK/c-imports-used.txt
  for m in $MODS; do
    lowered $m || continue
    modargs $m
    nice -n 15 $FEW $PE $WORK/n0-$m.wasm $PDATA $SEEDS --teb 0x1000 --hybrid-r 0x2000 --hybrid-x 0x2200 --hybrid-mx 0x3000 0x3100 --hybrid-progress 0x3300 $FEW_FLAGS --hybrid-only $LISTS/lowered-$m.txt --report $WORK/n0-$m.tsv > $WORK/logs/n0-$m.txt 2> $WORK/logs/n0-$m.err || { echo "n0 $m failed"; tail -3 $WORK/logs/n0-$m.err; return 1; }
    wasm-objdump -x $WORK/n0-$m.wasm 2>/dev/null | grep -E "^ - func\[[0-9]+\] sig=[0-9]+ <c\." | sed 's/.*<c\.\(.*\)>.*/\1/' >> $WORK/c-imports-used.txt
    python3 $W/tools/chk_lowered.py $LISTS/lowered-$m.txt $WORK/n0-$m.tsv | sed "s/^/n0 $m: /"
  done
  sort -u $WORK/c-imports-used.txt > $WORK/c-imports-used.u.txt
  sed 's/^/-Wl,--export-if-defined=/' $WORK/c-imports-used.u.txt > $WORK/exports.rsp
  wc -l $WORK/exports.rsp
}
LINKX() { echo "-Wl,--allow-undefined -sERROR_ON_UNDEFINED_SYMBOLS=0 -sWARN_ON_UNDEFINED_SYMBOLS=0 @$WORK/exports.rsp -Wl,-Map,$1"; }
stage_weblink() {   # the units that changed + the web link of the C part (flengine6.{js,wasm} + link.map)
  [ -s $WORK/exports.rsp ] || { echo "no exports.rsp: run n0"; return 1; }
  NAME=$HYB WEBLINK_EXTRA="$(LINKX $WORK/link.map)" bash $W/tools/bb.sh wasm weblink
}
stage_nodelink() {   # the node link of the same objects (glue for node + the same wasm if byte-identical)
  LINKX="$(LINKX $WORK/link-node.map)" bash $W/tools/nodelink.sh $HYB link
}
stage_opt() {
  local IN=$WORK/node/engine/obj/bin/flengine6.wasm
  [ -s $IN ] || { echo "no node-linked wasm"; return 1; }
  [ -s $WORK/c.opt.wasm ] && { echo "opt exists"; return 0; }
  free -m | awk 'NR==2{print "available MB " $7}'
  /usr/bin/time -f "wasm-opt %e s %M KB" -o $WORK/logs/c-opt.time nice -n 15 wasm-opt -Oz $FEAT_OPT -o $WORK/c.opt.wasm.part $IN && mv $WORK/c.opt.wasm.part $WORK/c.opt.wasm
  cat $WORK/logs/c-opt.time; ls -l $IN $WORK/c.opt.wasm
}
g() { grep -E "$1" $WORK/link.map | awk '{print $1}' | head -1; }
stage_gen() {
  local m T MXA BUFA PRG
  [ -s $WORK/link.map ] || { echo "no link.map"; return 1; }
  T=$(g "^\s*[0-9a-f]+\s+[0-9a-f]+\s+[0-9a-f]+\s+fle_teb$"); MXA=$(g "^\s*[0-9a-f]+\s+[0-9a-f]+\s+4\s+fle_mxcsr$"); BUFA=$(g "^\s*[0-9a-f]+\s+0\s+40\s+fle_hyb_buf$"); PRG=$(g "^\s*[0-9a-f]+\s+[0-9a-f]+\s+8\s+fle_progress_count$")
  echo "addresses: fle_teb 0x$T fle_mxcsr 0x$MXA fle_hyb_buf 0x$BUFA fle_progress_count 0x$PRG" | tee $WORK/logs/addresses.txt
  [ -n "$T" ] && [ -n "$MXA" ] && [ -n "$BUFA" ] && [ -n "$PRG" ] || { echo "address missing"; return 1; }
  mkdir -p $WORK/dir
  for m in $MODS; do
    lowered $m || continue
    modargs $m
    nice -n 15 $FEW $PE $WORK/dir/n-$m.wasm $PDATA $SEEDS --teb 0x$T --hybrid-r 0x2000 --hybrid-x 0x2200 --hybrid-mx 0x$MXA 0x$BUFA --hybrid-progress 0x$PRG $FEW_FLAGS --hybrid-only $LISTS/lowered-$m.txt --report $WORK/dir/n-$m.tsv > $WORK/logs/gen-$m.txt 2> $WORK/logs/gen-$m.err || { echo "gen $m failed"; tail -3 $WORK/logs/gen-$m.err; return 1; }
    wasm-validate --enable-all $WORK/dir/n-$m.wasm 2> $WORK/logs/val-$m.txt || { echo "validate $m failed: $(head -2 $WORK/logs/val-$m.txt)"; return 1; }
    python3 $W/tools/chk_lowered.py $LISTS/lowered-$m.txt $WORK/dir/n-$m.tsv | sed "s/^/gen $m: /" || return 1
    if [ ! -s $WORK/dir/n-$m.oz.wasm ]; then nice -n 15 wasm-opt -Oz $FEAT_OPT -o $WORK/dir/n-$m.oz.wasm.part $WORK/dir/n-$m.wasm && mv $WORK/dir/n-$m.oz.wasm.part $WORK/dir/n-$m.oz.wasm || return 1; fi
    echo "gen $m: $(stat -c %s $WORK/dir/n-$m.wasm) -> -Oz $(stat -c %s $WORK/dir/n-$m.oz.wasm) bytes"
  done
}
stage_merge() {   # the direct modules among themselves, then into the C part under the name `env`... as hyb3/hyb4 (--all-features)
  local m ins="" M=/opt/emsdk/upstream/bin/wasm-merge FEAT; FEAT=$(cat $W/tools/wasm-features.txt)
  for m in $MODS; do lowered $m && ins="$ins $WORK/dir/n-$m.oz.wasm d$m"; done
  $M $FEAT --rename-export-conflicts $ins -o $WORK/direct.wasm > $WORK/logs/merge1.txt 2>&1 || { echo "merge1 failed"; tail -3 $WORK/logs/merge1.txt; return 1; }
  ls -l $WORK/direct.wasm
  $M $FEAT --rename-export-conflicts $WORK/direct.wasm env $WORK/c.opt.wasm c -o $WORK/merged.wasm > $WORK/logs/merge2.txt 2>&1 || { echo "merge2 failed"; tail -3 $WORK/logs/merge2.txt; return 1; }
  mkdir -p $WORK/bin-node $WORK/bin-web
  cp -f $WORK/merged.wasm $WORK/bin-node/flengine6.wasm
  python3 $W/tools/strip_glue_stubs.py $WORK/node/engine/obj/bin/flengine6.js $WORK/bin-node/flengine6.js
  ls -l $WORK/merged.wasm $WORK/bin-node
  echo "unresolved c.* imports of the merged module: $(wasm-objdump -j Import -x $WORK/merged.wasm 2>/dev/null | grep -cE ' <c\.')"
}
stage_webprep() {   # the web bundle's inputs: the stripped web glue (the stubs of the lowered functions removed) and the merged wasm as flengine6.opt.wasm (build-engine-bundle.sh `bundle` picks them with WASM_OPT=1)
  cmp -s $WORK/node/engine/obj/bin/flengine6.wasm $WEB/flengine6.wasm || { echo "node link wasm != web link wasm: the merged wasm was made from the node link"; return 1; }
  [ -e $WEB/flengine6.orig.js ] || cp $WEB/flengine6.js $WEB/flengine6.orig.js
  python3 $W/tools/strip_glue_stubs.py $WEB/flengine6.orig.js $WEB/flengine6.js
  cp -f $WORK/merged.wasm $WEB/flengine6.opt.wasm
  ls -l $WEB
}
for st in "$@"; do
  echo "== $HYB $st $(date +%T) load=$(cut -d' ' -f1 /proc/loadavg) avail=$(awk '/MemAvailable/{print int($2/1024)}' /proc/meminfo)MB"
  "stage_$st" || { echo "stage $st FAILED"; exit 1; }
done
