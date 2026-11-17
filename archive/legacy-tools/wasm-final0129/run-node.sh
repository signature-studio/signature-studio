#!/bin/bash
# usage: VARIANT=<H|C2> run-node.sh NAME [node flags...] : run w-$VARIANT/obj/bin/flengine6.{js,wasm} in node (the harness of decision 0128: run_node_fruity.cjs, NODERAWFS: the host files are read from the repository checkout)
# with the environment of decision 0129's final configuration (tools/fruity-config.sh: vfs-seed-idle1.tsv, root = the repository, REG_SEED, AUTH_CLAIMS, FLE_FRUITY_NO_IDLE_STOP=1). Inside a cgroup with a hard memory cap (CAP_MB, default 5500).
# FOREGROUND. Logs: logs/node-$VARIANT-NAME/ .
. /home/ubuntu/work/signature-studio-develop/lift/out-wasm0129/src/docs/investigations/wasm-final0129/tools/env.sh
VARIANT=${VARIANT:?}
NAME=$1; shift
TAGDIR=$(ls -d $W/oonat/v-* | head -1)
DLLD=$TAGDIR/dll
REAL=$DLLD/dsp_ippv2_x64.2000000.dll:$DLLD/Blend2D.4700000.dll:$DLLD/quickfontcache_x64.1d00000.dll:$DLLD/freetype_x64.4b00000.dll:$DLLD/ILTools_x64.6000000.dll
LD=$W/logs/node-$VARIANT-$NAME
mkdir -p $LD
export FLE=$R/binaries/win64/FLEngine_x64.dll
export FLE_VFS_SEED=$H/seeds/vfs-seed-idle1.tsv FLE_VFS_SEED_ROOT=$R
export FLE_REG_SEED=$H/seeds/reg-seed-nsis.tsv:$H/seeds/reg-seed-app.tsv
export FLE_AUTH_CLAIMS=$H/seeds/authenticode-claims.tsv
export FLE_CALL_FRUITY=1 FLE_FRUITY_NO_IDLE_STOP=1 FLE_FRUITY_MAX_CALLS=4000000000 FLE_FRUITY_SECONDS=${WASM_SECONDS:-3000}
export FLE_WASM_DIR=${FLE_WASM_DIR:-$W/w-$VARIANT/obj/bin}
export WF1_OUT=$LD/surfaces WF1_JSON=$LD/summary.json
date -u +%Y-%m-%dT%H:%M:%SZ > $LD/start.txt
$T/memguard.sh ${CAP_MB:-5500} /usr/bin/time -v node --experimental-wasm-jspi "$@" $S/harness/two/run_node_fruity.cjs $FLE $REAL > $LD/stdout.txt 2> $LD/stderr.txt
echo "node rc=$?" | tee $LD/rc.txt
date -u +%Y-%m-%dT%H:%M:%SZ > $LD/end.txt
grep -E "^(fruity: (== stop|runtime message|CreateFruityInstance at|RETURNED)|wf1-surface:|a4: init_count|a4: all_units|functions executed|function entries|threads:|windows:)" $LD/stdout.txt | cut -c1-330
grep -E "Elapsed|Maximum resident|memguard" $LD/stderr.txt
