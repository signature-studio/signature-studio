#!/bin/bash
# draw1: print the disassembly of the given function indices of a wasm (wasm-objdump -d streamed, stops after the highest index). usage: extract-fn.sh WASM OUTDIR IDX...
W=$1; O=$2; shift 2; mkdir -p $O
MAX=$(printf '%s\n' "$@" | sort -n | tail -1)
nice -n 15 wasm-objdump -d "$W" 2>/dev/null | awk -v want="$*" -v max=$MAX -v out=$O '
BEGIN { n = split(want, a, " "); for (i = 1; i <= n; i++) w[a[i]] = 1 }
/^[0-9a-f]+ func\[[0-9]+\]/ { match($0, /func\[[0-9]+\]/); id = substr($0, RSTART + 5, RLENGTH - 6); if (cur != "") close(cur); cur = ""; if (id + 0 > max) exit; if (id in w) { cur = out "/f" id ".txt" } }
cur != "" { print > cur }'
