#!/bin/bash
# usage: VARIANT=<H|C2> [DIST=<dir>] run-chrome.sh NAME [key=value ...] : one isolated headless Chrome 156 run of the page of src/lift/harness/web/final0129 (run-chrome-final.mjs) for the dist dir of the variant.
# Inside a cgroup with a hard memory cap (CAP_MB, default 5500). FOREGROUND. Output: logs/chrome-$VARIANT-NAME/ ; the net-log check is part of the runner (TCP non-loopback 0 and UDP bytes sent 0).
. /home/ubuntu/work/signature-studio-develop/lift/out-wasm0129/src/docs/investigations/wasm-final0129/tools/env.sh
VARIANT=${VARIANT:?}
NAME=$1; shift
DIST=${DIST:-$W/dist-$VARIANT/web}
LD=$W/logs/chrome-$VARIANT-$NAME
mkdir -p $LD /tmp/opencode/wasm0129/chrome
export PRESENT_WORK=${PRESENT_WORK:-/tmp/opencode/wasm0129/chrome} NETLOG_TOLERATE_IPV6_PROBE=1
$T/memguard.sh ${CAP_MB:-5500} /usr/bin/time -v -o $LD/time.txt node $S/harness/web/final0129/run-chrome-final.mjs name=$VARIANT-$NAME dist=$DIST out=$LD expect=$T/expected-0129.json timeout=${TIMEOUT:-3000} stall=${STALL:-900} "$@" > $LD/runner.stdout 2> $LD/runner.stderr
echo "chrome runner rc=$?"
tail -32 $LD/runner.stdout
grep -E "memguard|Elapsed" $LD/runner.stderr $LD/time.txt
