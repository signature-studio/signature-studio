#!/usr/bin/env bash
# 段階 0 の入口(Issue #1388): 製品のビルド(-Oz)、変種のビルド(O2・O3)、計測(18 run と変種の 12 run)、対照実験、集計を、この順に行う。
# 各段は、出力が在れば飛ばす(途中で止まっても、続きから行える)。段を指定すると、その段だけ行う: bash tools/engine-cost/run.sh baseline runs-oz variants runs-variants contrast aggregate
# --quick(Issue #1396): 製品のビルドの後、場面 i・ii・iii を DPR 1.00・n=1 で、窓を 30 秒にして 1 回ずつ測る(runs/Oz/quick-<場面>-dpr1.00-n1.json)。集計は行わない。
# 場面を一つだけ(Issue #1397): bash tools/engine-cost/run.sh --scene i|ii|iii。製品のビルドと、その場面の Oz の run(DPR 2 つ・n=3)だけを行い、集計は行わない(findings は書き換えない)。
# Issue #1397 の場面 (iii) の run: bash tools/engine-cost/run.sh --scene iii
# 製品の wasm は lift/out-app/ の下にしか作らない(git で無視される)。本番の fl-Oz を変種が上書きしない。
set -euo pipefail
cd "$(dirname "$0")/../.."
OUT=lift/out-app/engine-cost
RUNS=$OUT/runs
LOG=$OUT/run.log
# the second DPR: drawn at random from 1.01 to 3.00 (node Math.random, 2026-11-24); the first is 1.00
DPR_A=1.00
DPR_B=1.325
SCENES_RUN=(i ii iii)
BASE=lift/out-app/bundle/raw/fl-Oz
NAMES=lift/out-app/gen/wasm-func-names.tsv
mkdir -p "$OUT"
say() { echo "$(date -u +%H:%M:%S) $*" | tee -a "$LOG" >&2; }

if [ -z "${EMCC_DIR:-}" ]; then
  if command -v emcc > /dev/null 2>&1; then EMCC_DIR=$(dirname "$(command -v emcc)"); else EMCC_DIR=$(compgen -G '/home/runner/work/_temp/*/emsdk-main/upstream/emscripten' | head -1 || true); fi
fi
test -n "$EMCC_DIR" && test -x "$EMCC_DIR/emcc" || { say "emcc が見つからない(EMCC_DIR を設定する)"; exit 1; }
export PATH="$EMCC_DIR/../bin:$EMCC_DIR:$PATH"

stage_baseline() {
  if [ -f "$BASE/manifest.json" ] || [ -f lift/out-app/bundle/raw/fl-Oz/manifest.json ]; then say "baseline: exists, skipped"; return 0; fi
  say "baseline: the product's build (fe-build engine, -Oz)"
  cargo build --release --locked --manifest-path lift/Cargo.toml
  npm run build --prefix app
  lift/target/release/fe-build engine
  lift/target/release/fe-build bundle
  say "baseline: done"
}

# one run: the scene, the DPR, the bundle and its names table, the output (its name starts with prefix; window is the extra arguments of measure.mjs). A run with error null is kept; any other is run again (twice at most).
run_one() {
  local variant=$1 scene=$2 dpr=$3 n=$4 bundle=$5 names=$6 prefix=${7:-} window=${8:-}
  local out="$RUNS/$variant/$prefix$scene-dpr$dpr-n$n.json"
  mkdir -p "$RUNS/$variant"
  if [ -s "$out" ] && grep -q '"error":null' "$out"; then return 0; fi
  for attempt in 1 2; do
    say "run $variant scene $scene dpr $dpr n $n $window (attempt $attempt)"
    node tools/engine-cost/measure.mjs --bundle "$ROOT_DIR/$bundle" --names "$ROOT_DIR/$names" --scene "$scene" --dpr "$dpr" --out "$ROOT_DIR/$out" $window > /dev/null 2>> "$LOG" || true
    if [ -s "$out" ] && grep -q '"error":null' "$out"; then return 0; fi
  done
  say "run $variant scene $scene dpr $dpr n $n: failed twice (see $out)"
  return 0
}
ROOT_DIR=$PWD

stage_runs_oz() {
  for dpr in "$DPR_A" "$DPR_B"; do
    for n in 1 2 3; do
      for scene in "${SCENES_RUN[@]}"; do
        run_one Oz "$scene" "$dpr" "$n" "$BASE" "$NAMES"
      done
    done
  done
}

# the short check (Issue #1396): the three scenes at DPR 1.00, n=1, with a 30 s window
stage_quick() {
  for scene in i ii iii; do
    run_one Oz "$scene" "$DPR_A" 1 "$BASE" "$NAMES" "quick-" "--window-s 30"
  done
}

stage_variants() {
  for v in O2 O3; do
    if [ -f "lift/out-app/variants/$v/bundle/raw/fl-Oz/manifest.json" ]; then say "variant $v: exists, skipped"; continue; fi
    say "variant $v: build"
    node tools/engine-cost/build-variant.mjs "$v"
  done
}

# the variants are measured at DPR 1.00 (scenes i and iii, n=3), as the comparison in the Issue asks
stage_runs_variants() {
  for v in O2 O3; do
    for n in 1 2 3; do
      for scene in i iii; do
        run_one "$v" "$scene" "$DPR_A" "$n" "lift/out-app/variants/$v/bundle/raw/fl-Oz" "lift/out-app/variants/$v/gen/wasm-func-names.tsv"
      done
    done
  done
}

stage_contrast() {
  say "contrast: the direct import against the emscripten glue"
  node tools/engine-cost/contrast/contrast.mjs
}

stage_aggregate() {
  say "aggregate: the findings JSON"
  node tools/engine-cost/aggregate.mjs
}

if [ "${1:-}" = "--quick" ]; then
  [ $# -eq 1 ] || { say "--quick takes no other argument"; exit 2; }
  set -- baseline quick
fi
stages=("$@")
if [ "${1:-}" = "--scene" ]; then
  # Issue #1397: the runs of one scene only (the product's build and the 6 runs of Oz at the two DPRs); no aggregate, so the findings are not written
  case "${2:-}" in
    i|ii|iii) SCENES_RUN=("$2") ;;
    *) say "usage: run.sh --scene i|ii|iii"; exit 2 ;;
  esac
  shift 2
  stages=("$@")
  if [ ${#stages[@]} -eq 0 ]; then stages=(baseline runs-oz); fi
fi
if [ ${#stages[@]} -eq 0 ]; then stages=(baseline runs-oz variants runs-variants contrast aggregate); fi
for s in "${stages[@]}"; do
  case "$s" in
    baseline) stage_baseline ;;
    quick) stage_quick ;;
    runs-oz) stage_runs_oz ;;
    variants) stage_variants ;;
    runs-variants) stage_runs_variants ;;
    contrast) stage_contrast ;;
    aggregate) stage_aggregate ;;
    *) say "unknown stage $s"; exit 2 ;;
  esac
done
say "run.sh: done ($*)"
