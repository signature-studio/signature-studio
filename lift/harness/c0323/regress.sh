#!/bin/bash
# regression: the native outputs of the api2 window cases (win_upd, win_seq, win_act) with the runtime of TREE, using the objects of WORKDIR (built by c0323/build.sh)
set -eu -o pipefail
T=$(readlink -f "$1"); W=$(readlink -f "$2"); RT=$T/lift/runtime; H=$T/lift/harness/api2
O=""; for f in fle_api2_call fle_appview fle_callback fle_class fle_comctl fle_cursor fle_env fle_event fle_gdc fle_gdi fle_host fle_imports fle_input fle_layered fle_locale fle_mxcsr fle_proc fle_raster fle_sys fle_thread fle_time fle_token fle_vfs fle_visrgn fle_vmap fle_vol fle_wall_ia2 fle_wall_ia2w fle_win fle_x87 fle_x87ctl; do O="$O $W/$f.o"; done
for c in win_upd win_seq win_act; do
  nice -n 19 clang -O0 -std=gnu11 -w -fPIE -pthread -I"$RT" "$H/$c.c" $O -o "$W/${c}_native" -lm
  "$W/${c}_native" > "$W/${c}.out" 2> "$W/${c}.err" || echo "$c rc=$?"
done
wc -l "$W"/win_upd.out "$W"/win_seq.out "$W"/win_act.out | tail -4
