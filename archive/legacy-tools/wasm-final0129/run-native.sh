#!/bin/bash
# usage: run-native.sh NAME : an a4-multi.sh run (FRUITY on, 6 modules, decision 0129's final configuration) of the tree $SRCTREE; OUT = out-wasm0129/o<NAME>; logs in out-wasm0129/logs/<NAME>/ .
# FLE_FRUITY_NO_IDLE_STOP=1 is set here (decision 0129's wall 16 needs no idle build: verification.md 5.1). Extra FLE_FRUITY_* come from the caller. Does not touch a4-multi.sh / a4-fruity.sh.
. /home/ubuntu/work/signature-studio-develop/lift/out-wasm0129/src/docs/investigations/wasm-final0129/tools/fruity-config.sh
NAME=$1
export FLE_FRUITY_NO_IDLE_STOP=1
export OUT=$W/o$NAME FEBIN=${FEBIN:-$W/cargo-target/release} JOBS=${JOBS:-4}
export PATH=$T/wrap:$PATH
mkdir -p $W/logs/$NAME
date -u +%Y-%m-%dT%H:%M:%SZ > $W/logs/$NAME/start.txt
/usr/bin/time -v bash $H/a4-multi.sh "${FRUITY_MODS[@]}" > $W/logs/$NAME/a4-multi.stdout 2> $W/logs/$NAME/a4-multi.stderr
echo "a4-multi rc=$?" | tee $W/logs/$NAME/rc.txt
date -u +%Y-%m-%dT%H:%M:%SZ > $W/logs/$NAME/end.txt
grep -E "^fruity: (== stop|runtime message|abort kind|CreateFruityInstance at|RETURNED)|^(a4: |functions executed|function entries|harness exit code)" $W/logs/$NAME/a4-multi.stdout | cut -c1-400
find $OUT -name two-native | xargs sha256sum
grep -E "Elapsed|Maximum resident" $W/logs/$NAME/a4-multi.stderr
