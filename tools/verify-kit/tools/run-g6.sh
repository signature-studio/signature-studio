#!/bin/bash
# run-g6.sh: Firefox 158 functional check for RT-D3 (functional; not the counted P3 gate).
set -u
K=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
page_for() { echo "page-rt1/app"; }
run() {
  local variant=$1 port ref
  case "$variant" in e13) port=5053; ref=d0447902;; rtd3) port=5119; ref=rt1;; rtf) port=5122; ref=rt1;; rte) port=5121; ref=rt1;; esac
  ABI=$(mktemp /tmp/opencode/o19c-abi.XXXXXX)
  {
    echo "$(date +%T) g6 pre-run note $variant"
    echo "cargo/rustc: $(ps -eo args | grep -E 'cargo|rustc' | grep -v grep | wc -l)"
    free -m | sed -n '2,3p'; vmstat 1 3 | tail -2; cut -d' ' -f1-3 /proc/loadavg; cat /proc/pressure/io
  } | tee -a "$K/out/steps.log"
  SCN=scn-g6-d3.json ONLYNAME=g6-$variant OUTTAG=g6 RND=1 bash "$K/tools/run-o19.sh" song shown "$variant"
}
run rtf
