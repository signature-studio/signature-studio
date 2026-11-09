#!/bin/bash
# Decision 0215 (sweep1, opt-in test tool). usage: build.sh native|wine|diff [WORKDIR]
#   native: builds sweep1_cases.c with the runtime objects (clang) and runs it: WORKDIR/native.out
#   wine:   builds it with mingw (-mwindows) and runs it under Wine 10.0 with LC_ALL=en_US.UTF-8: WORKDIR/wine.out (CR removed)
#   diff:   native + wine + the comparison with expected/known-divergence.txt applied
set -e
HERE=$(cd "$(dirname "$0")" && pwd); RT=$HERE/../../runtime; W=$(realpath -m "${2:-${TMPDIR:-/tmp}/sweep1-work}"); mkdir -p "$W"
native() {
  LIST="fle_sweep1_kbd fle_api2_call fle_kbd fle_input fle_locale fle_time fle_glob fle_imports fle_env fle_vmap fle_proc fle_sys fle_vfs"
  for i in 1 2 3 4 5 6 7 8; do
    O=""
    for f in $LIST; do clang -O0 -std=gnu11 -w -fPIE -c -I"$RT" "$RT/$f.c" -o "$W/$f.o"; O="$O $W/$f.o"; done
    out=$(clang -O0 -std=gnu11 -w -fPIE -pthread -I"$RT" "$HERE/sweep1_cases.c" $O -o "$W/sweep1_native" 2>&1 | grep "undefined reference" | sed "s/.*undefined reference to .\(.*\)'/\1/" | sort -u)
    [ -z "$out" ] && break
    for s in $out; do f=$(grep -l "^[a-z].* $s(" "$RT"/*.c 2>/dev/null | head -1); LIST="$LIST $(basename "${f:-none}" .c)"; done
    LIST=$(echo $LIST | tr ' ' '\n' | sort -u | tr '\n' ' ')
  done
  "$W/sweep1_native" > "$W/native.out" 2> "$W/native.err"
}
wine_run() {
  x86_64-w64-mingw32-gcc -O1 -mwindows -DSWEEP1_WINE -static-libgcc -o "$W/sweep1_wine.exe" "$HERE/sweep1_cases.c" -luser32 -lkernel32
  WINEPREFIX=${WINEPREFIX:-$W/wineprefix} WINEDEBUG=-all LC_ALL=en_US.UTF-8 wine "$W/sweep1_wine.exe" 2> "$W/wine.err" | tr -d '\r' > "$W/wine.out"
}
case "${1:-diff}" in
  native) native ;;
  wine) wine_run ;;
  diff)
    native; wine_run
    if [ -s "$HERE/expected/known-divergence.txt" ]; then grep -v '^#' "$HERE/expected/known-divergence.txt" | cut -f1 > "$W/skip.txt"; else : > "$W/skip.txt"; fi
    grep -v -F -f "$W/skip.txt" "$W/wine.out" > "$W/wine.cmp" || true; grep -v -F -f "$W/skip.txt" "$W/native.out" > "$W/native.cmp" || true
    diff "$W/wine.cmp" "$W/native.cmp" && echo "IDENTICAL (known divergences skipped: $(grep -vc '^#' "$HERE/expected/known-divergence.txt" 2>/dev/null))" ;;
esac
