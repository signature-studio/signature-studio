#!/usr/bin/env bash
# R5-5b: P1 first idle, arm N (R5-5 bundle, HEAD 9135826) and arm C (control, HEAD 9610708), alternating N C N C ..., n=5 each, serial.
set -u
REPO=$HOME/work/signature-studio
OUT=$REPO/evidence/R5-5b
COND=$OUT/conditions.tsv
N=$HOME/.cache/signature-studio/work-r55b/bundle/fl-Oz
C=$HOME/.cache/signature-studio/work-r53k/bundle/fl-Oz
mkdir -p "$OUT"
printf 'arm\tn\ttry\tstart_utc\tloadavg1_before\tloadavg_line\tother_chrome_procs\tavail_mb_before\tvmstat_si_so_sample2\tvmstat_si_so_sample3\tcounted\trc\n' > "$COND"

run_one() {
  local arm=$1 n=$2 dir=$3 try=0 l1 counted la vm avail chr rc
  while :; do
    try=$((try + 1))
    la=$(cat /proc/loadavg)
    l1=${la%% *}
    vm=$(vmstat 1 3 | tail -n 2 | awk '{printf "%s/%s ", $7, $8}')
    avail=$(free -m | awk '/Mem:/{print $7}')
    chr=$(ps -eo comm | grep -ci chrom)
    local start
    start=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    if awk -v x="$l1" 'BEGIN{exit !(x <= 6)}'; then counted=yes; break; fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$arm" "$n" "$try" "$start" "$l1" "$la" "$chr" "$avail" "$(echo $vm | cut -d' ' -f1)" "$(echo $vm | cut -d' ' -f2)" "retry-high-load" "-" >> "$COND"
    if [ "$try" -ge 3 ]; then counted=no; break; fi
  done
  node "$REPO/tools/boot-check/boot-bundle.mjs" "$dir" > "$OUT/run-$arm-$n.json" 2> "$OUT/err-$arm-$n.txt"
  rc=$?
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$arm" "$n" "$try" "$start" "$l1" "$la" "$chr" "$avail" "$(echo $vm | cut -d' ' -f1)" "$(echo $vm | cut -d' ' -f2)" "$counted" "$rc" >> "$COND"
}

for n in 1 2 3 4 5; do
  run_one N "$n" "$N"
  run_one C "$n" "$C"
done
echo done
