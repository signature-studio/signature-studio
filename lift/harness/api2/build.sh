#!/bin/bash
# Decision 0210 (api2, opt-in test tool). usage: build.sh native|wine|diff [WORKDIR]
#   native: builds api2_cases.c with the runtime objects (clang) and runs it: WORKDIR/native.out (stdout), native.err (stderr)
#   wine:   builds it with mingw (-mwindows: no console) and runs it under Wine 10.0 with TZ=UTC LC_ALL=en_US.UTF-8: WORKDIR/wine.out (CR removed)
#   diff:   native + wine + the comparison with expected/known-divergence.txt applied
set -e
HERE=$(cd "$(dirname "$0")" && pwd); RT=$HERE/../../runtime; W=$(realpath -m "${2:-${TMPDIR:-/tmp}/api2-work}"); mkdir -p "$W"
native() {
  LIST="fle_api2_call fle_api2_str fle_api2_sys fle_api2_shell fle_locale fle_time fle_glob fle_imports fle_env fle_vmap fle_proc fle_sys fle_vfs"
  for i in 1 2 3 4 5 6 7 8; do
    O=""
    for f in $LIST; do clang -O0 -std=gnu11 -w -fPIE -c -I"$RT" "$RT/$f.c" -o "$W/$f.o"; O="$O $W/$f.o"; done
    out=$(clang -O0 -std=gnu11 -w -fPIE -pthread -I"$RT" "$HERE/api2_cases.c" $O -o "$W/api2_native" 2>&1 | grep "undefined reference" | sed "s/.*undefined reference to .\(.*\)'/\1/" | sort -u)
    [ -z "$out" ] && break
    for s in $out; do f=$(grep -l "^[a-z].* $s(" "$RT"/*.c 2>/dev/null | head -1); LIST="$LIST $(basename "${f:-none}" .c)"; done
    LIST=$(echo $LIST | tr ' ' '\n' | sort -u | tr '\n' ' ')
  done
  "$W/api2_native" > "$W/native.out" 2> "$W/native.err"
}
wine_run() {
  x86_64-w64-mingw32-gcc -O1 -mwindows -DAPI2_WINE -static-libgcc -o "$W/api2_wine.exe" "$HERE/api2_cases.c" -lshell32 -luser32 -lkernel32
  WINEPREFIX=${WINEPREFIX:-$W/wineprefix} WINEDEBUG=-all TZ=UTC LC_ALL=en_US.UTF-8 wine "$W/api2_wine.exe" 2> "$W/wine.err" | tr -d '\r' > "$W/wine.out"
}
win() {
  # the update-region cases of win_upd.c: runtime window layer (native) against Wine 10.0 (headless); the two known GAPs are listed in expected/win-known-divergence.txt
  WL="fle_api2_call fle_appview fle_callback fle_class fle_comctl fle_cursor fle_env fle_event fle_gdc fle_gdi fle_host fle_imports fle_input fle_layered fle_locale fle_mxcsr fle_proc fle_raster fle_sys fle_thread fle_time fle_token fle_vfs fle_visrgn fle_vmap fle_vol fle_wall_ia2 fle_wall_ia2w fle_win fle_x87 fle_x87ctl"
  O=""; for f in $WL; do clang -O0 -std=gnu11 -w -fPIE -c -I"$RT" "$RT/$f.c" -o "$W/w_$f.o"; O="$O $W/w_$f.o"; done
  clang -O0 -std=gnu11 -w -fPIE -pthread -I"$RT" "$HERE/win_upd.c" $O -o "$W/win_upd_native" -lm
  "$W/win_upd_native" > "$W/win_native.out" 2> "$W/win_native.err"
  x86_64-w64-mingw32-gcc -O1 -mwindows -DAPI2_WINE -static-libgcc -o "$W/win_upd_wine.exe" "$HERE/win_upd.c" -lgdi32 -luser32
  env -u DISPLAY WINEPREFIX=${WINEPREFIX:-$W/wineprefix} WINEDEBUG=-all LC_ALL=en_US.UTF-8 wine "$W/win_upd_wine.exe" 2> "$W/win_wine.err" | tr -d '\r' > "$W/win_wine.out"
  diff "$W/win_wine.out" "$W/win_native.out" || true
}
seq() {
  # the message sequences of win_seq.c: runtime against Wine 10.0 (a measurement, not a pass/fail test: the differences are listed in docs/0210-api-layer-survey.md)
  WL="fle_api2_call fle_appview fle_callback fle_class fle_comctl fle_cursor fle_env fle_event fle_gdc fle_gdi fle_host fle_imports fle_input fle_layered fle_locale fle_mxcsr fle_proc fle_raster fle_sys fle_thread fle_time fle_token fle_vfs fle_visrgn fle_vmap fle_vol fle_wall_ia2 fle_wall_ia2w fle_win fle_x87 fle_x87ctl"
  O=""; for f in $WL; do [ -f "$W/w_$f.o" ] || clang -O0 -std=gnu11 -w -fPIE -c -I"$RT" "$RT/$f.c" -o "$W/w_$f.o"; O="$O $W/w_$f.o"; done
  clang -O0 -std=gnu11 -w -fPIE -pthread -I"$RT" "$HERE/win_seq.c" $O -o "$W/win_seq_native" -lm
  "$W/win_seq_native" > "$W/seq_native.out" 2> "$W/seq_native.err"
  x86_64-w64-mingw32-gcc -O1 -mwindows -DAPI2_WINE -static-libgcc -o "$W/win_seq_wine.exe" "$HERE/win_seq.c" -lgdi32 -luser32
  env -u DISPLAY WINEPREFIX=${WINEPREFIX:-$W/wineprefix} WINEDEBUG=-all LC_ALL=en_US.UTF-8 wine "$W/win_seq_wine.exe" 2> "$W/seq_wine.err" | tr -d '\r' > "$W/seq_wine.out"
  diff "$W/seq_wine.out" "$W/seq_native.out" || true
}
drag() {
  # build.sh drag WORKDIR IMPL.c... : the HDROP cases against the given implementation of DragQueryFileW/DragQueryPoint/DragFinish (native) and against Wine 10.0, then the comparison.
  # Environment: DRAG_RT=<runtime dir that IMPL.c belongs to, searched before this one>, DRAG_REG=<header that declares fle_drop_register>, e.g. crash1: DRAG_RT=.../out-crash1/src/lift/runtime DRAG_REG='"fle_crash1_drag.h"'
  shift 2; IMPL="$@"; R2=${DRAG_RT:-$RT}
  DL="fle_api2_call fle_glob fle_imports fle_env fle_sys fle_vfs fle_vmap fle_proc fle_time fle_locale"
  O=""; for f in $DL; do clang -O0 -std=gnu11 -w -fPIE -c -I"$R2" -I"$RT" "$R2/$f.c" -o "$W/d_$f.o" 2>/dev/null || clang -O0 -std=gnu11 -w -fPIE -c -I"$RT" "$RT/$f.c" -o "$W/d_$f.o"; O="$O $W/d_$f.o"; done
  for c in $IMPL; do b=$(basename "$c" .c); clang -O0 -std=gnu11 -w -fPIE -c -I"$R2" -I"$RT" "$c" -o "$W/d_$b.o"; O="$O $W/d_$b.o"; done
  printf 'int fle_win_set_exstyle_bits(unsigned long long h, unsigned m, unsigned v) { (void)h; (void)m; (void)v; return 0; }\n' > "$W/d_stub.c"; clang -O0 -w -fPIE -c "$W/d_stub.c" -o "$W/d_stub.o"; O="$O $W/d_stub.o"
  REG=""; [ -n "${DRAG_REG:-}" ] && REG="-DAPI2_DRAG_REGISTER_HEADER=${DRAG_REG}"
  clang -O0 -std=gnu11 -w -fPIE -pthread -I"$R2" -I"$RT" $REG "$HERE/api2_drag.c" $O -o "$W/api2_drag_native" -lm
  "$W/api2_drag_native" > "$W/drag_native.out" 2> "$W/drag_native.err"; echo "native rc=$?"
  x86_64-w64-mingw32-gcc -O1 -mwindows -DAPI2_WINE -static-libgcc -o "$W/api2_drag_wine.exe" "$HERE/api2_drag.c" -lshell32 -luser32 -lkernel32
  env -u DISPLAY WINEPREFIX=${WINEPREFIX:-$W/wineprefix} WINEDEBUG=-all LC_ALL=en_US.UTF-8 wine "$W/api2_drag_wine.exe" 2> "$W/drag_wine.err" | tr -d '\r' > "$W/drag_wine.out"
  diff "$W/drag_wine.out" "$W/drag_native.out" && echo "DRAG: identical to Wine"
}
act() {
  # decision 0222: the z-order / activation / mouse-activation / close cases of win_act.c: runtime window layer (native) against Wine 10.0 (headless); the known differences are expected/win-act-known-divergence.txt
  WL="fle_api2_call fle_appview fle_callback fle_class fle_comctl fle_cursor fle_env fle_event fle_gdc fle_gdi fle_host fle_imports fle_input fle_layered fle_locale fle_mxcsr fle_proc fle_raster fle_sys fle_thread fle_time fle_token fle_vfs fle_visrgn fle_vmap fle_vol fle_wall_ia2 fle_wall_ia2w fle_win fle_x87 fle_x87ctl"
  O=""; for f in $WL; do [ -f "$W/w_$f.o" ] && [ "$W/w_$f.o" -nt "$RT/$f.c" ] || clang -O0 -std=gnu11 -w -fPIE -c -I"$RT" "$RT/$f.c" -o "$W/w_$f.o"; O="$O $W/w_$f.o"; done
  clang -O0 -std=gnu11 -w -fPIE -pthread -I"$RT" "$HERE/win_act.c" $O -o "$W/win_act_native" -lm
  "$W/win_act_native" $SCEN > "$W/act_native.out" 2> "$W/act_native.err"
  x86_64-w64-mingw32-gcc -O1 -mwindows -DAPI2_WINE -static-libgcc -o "$W/win_act_wine.exe" "$HERE/win_act.c" -lgdi32 -luser32
  env -u DISPLAY WINEPREFIX=${WINEPREFIX:-$W/wineprefix} WINEDEBUG=-all LC_ALL=en_US.UTF-8 wine "$W/win_act_wine.exe" $SCEN 2> "$W/act_wine.err" | tr -d '\r' > "$W/act_wine.out"
  diff "$W/act_wine.out" "$W/act_native.out" || true
}
case "$1" in
  act) act ;;
  drag) drag "$@" ;;
  seq) seq ;;
  win) win ;;
  native) native ;;
  wine) wine_run ;;
  diff)
    native; wine_run
    grep -v '^NATIVE ' "$W/native.out" > "$W/native.cmp"
    # lines whose label is listed in expected/known-divergence.txt are compared with the hand-written native line instead of Wine's
    python3 - "$W/wine.out" "$W/native.cmp" "$HERE/expected/known-divergence.txt" <<'PY'
import sys
wine = open(sys.argv[1]).read().split('\n'); nat = open(sys.argv[2]).read().split('\n')
div = {}
for ln in open(sys.argv[3]):
    if ln.startswith('#') or not ln.strip(): continue
    wl, nl, _ = ln.rstrip('\n').split('\t', 2)
    div[wl] = nl
bad = 0
for i, (w, n) in enumerate(zip(wine, nat)):
    exp = div.get(w, w)
    if exp != n: print('DIFF line %d: wine=%r native=%r expected=%r' % (i + 1, w, n, exp)); bad += 1
if len(wine) != len(nat): print('LINE COUNT', len(wine), len(nat)); bad += 1
print('compared %d lines, %d differences, %d known divergences' % (len(wine), bad, len(div)))
sys.exit(1 if bad else 0)
PY
    ;;
  *) echo "usage: build.sh native|wine|diff|win|seq [WORKDIR] | drag WORKDIR IMPL.c..."; exit 2 ;;
esac
