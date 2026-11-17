#!/bin/bash
# usage: VARIANT=<H|C2> [HOOKS=0|1] [GEN_OPT=O0] [BIG_OPT=O1] [LINK_OPT=O0] [JOBS=2] [CAP_MB=5500] wasm-build.sh STAGE...   (stages: rt harness units o1 link)
# wasm-final0129 (decision 0134): build_nmod_wasm.sh of the HEAD tree with the FRUITY-on settings of decision 0128 (THREADS=1 JSPI=1 SJLJ=wasm VAR=force FRUITY=1) on the size-cut units of tools/resplit.sh (OUT = out-wasm0129/wsrc),
# HEAPPROBE=1. Every stage runs inside a cgroup with a hard memory cap (tools/memguard.sh), after a free -m / MemAvailable record. Per-stage wall time, the tree's peak memory (cgroup memory.peak) and the largest process RSS go to logs/build-$VARIANT/.
. /home/ubuntu/work/signature-studio-develop/lift/out-wasm0129/src/docs/investigations/wasm-final0129/tools/env.sh
VARIANT=${VARIANT:?VARIANT is not set}
export OUT=$W/wsrc
TAGDIR=$(ls -d $OUT/v-* | head -1)
export TAG=$(basename $TAGDIR | sed 's/^v-//')
export TOOLS=$S/../docs/investigations/a4-six-module-wasm/tools
export REALMOD_C=$W/oonat/v-$TAG/native/realmod_table_gen.c
export VAR=force SJLJ=wasm JSPI=1 THREADS=1 FRUITY=1 HEAPPROBE=1
export JOBS=${JOBS:-2} SERIAL_MODS=${SERIAL_MODS:-none} DELETE_SRC=0
export HOOKS=${HOOKS:-1} GEN_OPT=${GEN_OPT:-O0} BIG_OPT=${BIG_OPT:-O1} LINK_OPT=${LINK_OPT:-O0}
export EM_CACHE=${EM_CACHE:-$W/emcache}
export OBJ=$W/w-$VARIANT/obj
CAP=${CAP_MB:-5500}
LD=$W/logs/build-$VARIANT
mkdir -p $LD $OBJ
for st in "$@"; do
  t1=$(date +%s)
  echo "== $(date -u +%H:%M:%SZ) stage $st variant $VARIANT HOOKS=$HOOKS GEN_OPT=$GEN_OPT BIG_OPT=$BIG_OPT LINK_OPT=$LINK_OPT JOBS=$JOBS" | tee -a $LD/stages.log
  $T/memguard.sh $CAP /usr/bin/time -v -o $LD/stage-$st.time bash $H/two/build_nmod_wasm.sh $st > $LD/stage-$st.out 2> $LD/stage-$st.err; rc=$?
  grep -E "^memguard:" $LD/stage-$st.err | tee -a $LD/stages.log
  echo "stage '$st' rc=$rc wall=$(( $(date +%s) - t1 ))s maxrss_largest_process=$(grep 'Maximum res' $LD/stage-$st.time | awk '{print $NF}')KB disk=$(du -sm $W/w-$VARIANT | cut -f1)MB" | tee -a $LD/stages.log
  [ $rc -ne 0 ] && { tail -20 $LD/stage-$st.out $LD/stage-$st.err; exit 1; }
  if [ "$st" = link ] && [ ! -s $OBJ/bin/flengine6.wasm ]; then echo "link: no flengine6.wasm (build_nmod_wasm.sh stage_link returns the status of its last ls, not the link's)" | tee -a $LD/stages.log; tail -5 $OBJ/log/link.log; exit 1; fi
done
