#!/bin/bash
# run-e14-e2e.sh [--echo] [TARGET ...]   (ci1, 0321; started only on the PM's signal, in a quiet Chrome window; see E2E-INVENTORY.md)
# The e2e of the final page of E14 (P + V7 = page-e14-PV7) with the engine bundle of the production E13 (fl-engine-ci-0363b050), target by target (default: preview dev worker; worker is the heavy one and goes last).
#   --echo   prints what it WOULD run and checks that the inputs exist; starts nothing (no Chrome, no server, no test)
# Before each target: no other Chrome/Firefox/Wine/compiler/node-Export process, free -m and vmstat noted in out/steps.log (swap in/out must be 0), wait-quiet.mjs (load <= 8, I/O pressure 0, no swap traffic).
# Results: $APP_LOGS-<target>/e2e-results.json and the output of each target in $LOGDIR/e2e-<target>.txt; a line per target in out/steps.log; copied to lift/out-git/ci1-results/0277/e14/e2e/.
# A target whose load went over 8 while it ran is a reference run (the load at its end is in the summary line). `compositor-synth` needs emcc: without it the group is SKIPPED and still "passes" (desk2 MERGE-NOTES:37), so emcc is required here.
set -u
ECHO=0; if [ "${1:-}" = "--echo" ]; then ECHO=1; shift; fi
TARGETS=${*:-preview dev worker}
. "$(dirname "${BASH_SOURCE[0]}")/../env.sh"
PAGEREF=${PAGEREF:-e14-PV7}          # the page to test (page-$PAGEREF); the control page of the production E13 is d0447902
PAGE=$K/page-$PAGEREF/app
WQ_LOAD=${WQ_LOAD:-8}                  # the load limit of wait-quiet: 8 for a timed run; the PM allowed <= 12 for this functional (untimed) e2e on 2026-11-07 08:5x
ONLY=${ONLY:-}                         # a comma list of e2e groups (run.mjs --only), to repeat the groups that failed
TAG=${TAG:-pv7}
BUNDLE=$K/bundles/fl-engine-ci-0363b050
EMCC_DIR=/opt/emsdk/upstream/emscripten
BASE=$K/tmp/e2e-$TAG
LOGDIR=$BASE/logs
KEEP=$REPO/lift/out-git/ci1-results/0277/e14/e2e${TAG:+-$TAG}
STEPS=$K/out/steps.log
say() { echo "$*"; }
note() { echo "$(date +%T) $* load $(cut -d' ' -f1 /proc/loadavg) io $(io)" | tee -a "$STEPS"; }
need() { [ -e "$1" ] && say "ok: $1" || { say "MISSING: $1"; MISSING=1; }; }
MISSING=0
say "== inputs"
need "$PAGE/dist/index.html"; need "$PAGE/test/e2e/run.mjs"; need "$PAGE/node_modules"; need "$BUNDLE/manifest.json"; need "$EMCC_DIR/emcc"; need "$REPO/docs/investigations/wasm-final0129/tools/expected-0129.json"
[ "$MISSING" = 0 ] || { say "stop: an input is missing"; exit 2; }
others() { local n; n=$(ps -eo ni,args | awk '$1 < 10' | grep -E "firefox|geckodriver|FL64|wineserver|clang|rustc|wasm-opt|emcc|node-run|sdriveg2|sdrive-a1|vite build|test/e2e/run.mjs" | grep -v grep | grep -v "run-e14-e2e" | wc -l); local c; c=$(tail -1 "$K/out/chrome-count.tsv" | cut -f2); echo $((n + c)); }  # 2026-11-07 08:4x: nice >= 10 jobs (mt3's builds, PM allowed) are not counted; other Chrome = the last line of chrome-count.tsv (automated browsers, as in the pure/lat tables), not every process with "chrome" in its name
say "== other heavy processes now: $(others) (must be 0 to start)"
for t in $TARGETS; do
  case "$t" in preview|dev|worker) ;; *) say "unknown target $t"; exit 2 ;; esac
  CMD="cd $PAGE && env APP_REPO_ROOT=$REPO APP_WORK=$BASE/work-$t APP_LOGS=$BASE/logs-$t FL_ENGINE_BUNDLE=$BUNDLE PATH=$EMCC_DIR:\$PATH node test/e2e/run.mjs --target $t --no-build ${ONLY:+--only $ONLY} > $LOGDIR/e2e-$t.txt 2>&1"
  say "== target $t"
  if [ "$ECHO" = 1 ]; then
    say "[echo] mkdir -p $BASE/work-$t $BASE/logs-$t $LOGDIR"
    say "[echo] note: free -m | sed -n 2,3p; vmstat 1 3 | tail -2  >> $STEPS"
    say "[echo] node $K/tools/wait-quiet.mjs $WQ_LOAD 15"
    say "[echo] $CMD"
    say "[echo] then: grep '^e2e: ' $LOGDIR/e2e-$t.txt; grep -c SKIPPED $LOGDIR/e2e-$t.txt; cp the logs to $KEEP/"
    continue
  fi
  [ "$(others)" = 0 ] || { say "stop: another heavy process is running; not starting $t"; exit 3; }
  mkdir -p "$BASE/work-$t" "$BASE/logs-$t" "$LOGDIR" "$KEEP"
  { echo "$(date +%T) e2e $t pre-run note"; free -m | sed -n 2,3p; vmstat 1 3 | tail -2; } | tee -a "$STEPS"
  Q=$(node "$K/tools/wait-quiet.mjs" $WQ_LOAD 15) || { say "stop: not quiet: $Q"; exit 4; }
  note "e2e $t start page-$PAGEREF only=${ONLY:-all} wq-load<=$WQ_LOAD ($Q)"
  bash -c "$CMD"; RC=$?
  RES=$(grep '^e2e: ' "$LOGDIR/e2e-$t.txt" | tail -1)
  note "e2e $t end page-$PAGEREF only=${ONLY:-all} rc=$RC: ${RES:-no summary line}; SKIPPED lines: $(grep -c SKIPPED "$LOGDIR/e2e-$t.txt")"
  cp "$LOGDIR/e2e-$t.txt" "$KEEP/" 2>/dev/null; cp "$BASE/logs-$t/e2e-results.json" "$KEEP/e2e-results-$t.json" 2>/dev/null
done
say "== done"
