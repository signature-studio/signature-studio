#!/bin/bash
# wasm-final0129 (copy of perf1: memguard.sh): run a command inside a systemd user scope with a hard memory cap (cgroup v2 MemoryMax, no swap): the cap, not the command, decides what is killed.
# usage: memguard.sh CAP_MB cmd...   Records free -m and MemAvailable before (refuses to start when MemAvailable < 4096 MB, the PM's rule of 2026-11-01 16:09) and the scope's memory.peak after
# (the peak of the whole process tree of the command: the sum, not the largest process that /usr/bin/time -v reports).
CAP=$1; shift
avail=$(awk '/MemAvailable/ {print int($2/1024)}' /proc/meminfo)
echo "memguard: $(date -u +%H:%M:%SZ) MemAvailable=${avail}MB cap=${CAP}MB cmd=$1" >&2
free -m | sed -n 1,3p >&2
[ "$avail" -ge 4096 ] || { echo "memguard: MemAvailable $avail MB < 4096 MB: not started" >&2; exit 99; }
exec systemd-run --user --scope --quiet -p MemoryMax=${CAP}M -p MemorySwapMax=0 bash -c '"$@"; rc=$?; cg=/sys/fs/cgroup$(cut -d: -f3 /proc/self/cgroup); echo "memguard: cgroup memory.peak=$(( $(cat $cg/memory.peak 2>/dev/null || echo 0) / 1048576 ))MB oom_kill=$(grep -s oom_kill $cg/memory.events | cut -d" " -f2) rc=$rc" >&2; exit $rc' _ "$@"
