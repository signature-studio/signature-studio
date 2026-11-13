#!/bin/bash
# Decision 0323 (ci1, test tool). usage: build.sh native|wine|both TREE WORKDIR [SCENARIO]
#   native: contract_0323.c with the runtime of TREE (clang -O0, nice 19, one at a time): WORKDIR/native.out
#   wine:   the same file with mingw, run under Wine 10.0 headless in a prefix of its own: WORKDIR/wine.out
set -eu -o pipefail
MODE=$1; T=$(readlink -f "$2"); mkdir -p "$3"; W=$(readlink -f "$3"); SC=${4:-}
RT=$T/lift/runtime; HERE=$T/lift/harness/c0323
WL="fle_api2_call fle_appview fle_callback fle_class fle_comctl fle_cursor fle_env fle_event fle_gdc fle_gdi fle_host fle_imports fle_input fle_layered fle_locale fle_mxcsr fle_proc fle_raster fle_sys fle_thread fle_time fle_token fle_vfs fle_visrgn fle_vmap fle_vol fle_wall_ia2 fle_wall_ia2w fle_win fle_x87 fle_x87ctl"
native() {
  O=""
  for f in $WL; do
    if [ ! -f "$W/$f.o" ] || [ "$RT/$f.c" -nt "$W/$f.o" ] || [ "$RT/fle_win.h" -nt "$W/$f.o" ] || [ "$RT/fle_input.h" -nt "$W/$f.o" ] || [ "$RT/fle_class.h" -nt "$W/$f.o" ] || [ "$RT/fle_gdc.h" -nt "$W/$f.o" ]; then
      nice -n 19 clang -O0 -std=gnu11 -w -fPIE -c -I"$RT" "$RT/$f.c" -o "$W/$f.o"
    fi
    O="$O $W/$f.o"
  done
  nice -n 19 clang -O0 -std=gnu11 -w -fPIE -pthread -I"$RT" "$HERE/contract_0323.c" $O -o "$W/contract_native" -lm
  "$W/contract_native" $SC > "$W/native.out" 2> "$W/native.err" || echo "native rc=$?"
}
wine_run() {
  x86_64-w64-mingw32-gcc -O1 -mwindows -DC0323_WINE -static-libgcc -o "$W/contract_wine.exe" "$HERE/contract_0323.c" -lgdi32 -luser32
  env -u DISPLAY WINEPREFIX=${WINEPREFIX:-$W/wineprefix} WINEDEBUG=-all LC_ALL=en_US.UTF-8 wine "$W/contract_wine.exe" $SC 2> "$W/wine.err" | tr -d '\r' > "$W/wine.out" || echo "wine rc=$?"
}
case "$MODE" in
  native) native ;;
  wine) wine_run ;;
  both) native; wine_run; diff "$W/wine.out" "$W/native.out" > "$W/wine-vs-native.diff" || true; wc -l "$W/wine-vs-native.diff" ;;
  *) echo "usage: build.sh native|wine|both TREE WORKDIR [E|P|T|H]"; exit 2 ;;
esac
