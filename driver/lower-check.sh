#!/usr/bin/env bash
# driver/lower-check.sh : 関数を直接 wasm にした後の、ビルドと起動確認(型 lowerb の受け入れの後半。台帳 L-037)。
# queue/kinds/lower.md の accept の後半と同じ判定で、条件は 1 つも弱めない(error が null・state が engine-idle・standIn が false)。
# 段の名前は「lower-check: 段: 名前」で出す。落ちたときは、最後に出た段が落ちた段。通ったときは最後に「lower-check: 通った」を出す。
set -euo pipefail
: "${RUNNER_TEMP:?}"
step() { echo "lower-check: 段: $1"; }
step cargo
cargo build --release --locked --manifest-path lift/Cargo.toml
rm -rf lift/out-app/b2d-src lift/out-app/gen
step engine
lift/target/release/fe-build engine
step bundle
lift/target/release/fe-build bundle
step boot
rm -f "$RUNNER_TEMP/boot.json"
node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json"
node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" || { echo "lower-check: 起動確認の結果: $(head -c 300 "$RUNNER_TEMP/boot.json")"; exit 1; }
echo "lower-check: 通った"
