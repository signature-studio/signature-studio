#!/bin/bash
# alpha1: build and run lift/harness/alpha1/glass_seq.c on the runtime of $SRC (a tree with lift/). usage: build-glass.sh [SRC] [OUTDIR] [extra clang flags]
set -eu
REPO="$(readlink -f "$(dirname "${BASH_SOURCE[0]}")/../../..")"; A="$REPO/lift/out-alpha1"
SRC=${1:-$A/src}; OUT=${2:-$A/work/glass}; shift 2 2>/dev/null || true
RT=$SRC/lift/runtime; H=$SRC/lift/harness/alpha1
mkdir -p $OUT
OBJS="fle_api2_call fle_appview fle_callback fle_class fle_comctl fle_cursor fle_env fle_event fle_gdc fle_gdi fle_host fle_imports fle_input fle_layered fle_locale fle_mxcsr fle_proc fle_raster fle_sys fle_thread fle_time fle_token fle_vfs fle_visrgn fle_vmap fle_vol fle_win fle_x87 fle_x87ctl fle_wall_ia2 fle_wall_ia2w fle_desk fle_dwm fle_monitor"
objs=""
for n in $OBJS; do
  [ -e $RT/$n.c ] || { echo "no $n.c"; continue; }
  clang -O1 -std=gnu11 -w -fPIE -DFLE_DESKTOP "$@" -c -I$RT $RT/$n.c -o $OUT/$n.o &
  objs="$objs $OUT/$n.o"
  while [ "$(jobs -r | wc -l)" -ge 4 ]; do wait -n; done
done
wait
clang -O1 -std=gnu11 -w -fPIE -pthread -DFLE_DESKTOP "$@" -I$RT $H/glass_seq.c $objs -lm -o $OUT/glass_seq 2>$OUT/link.log || { head -30 $OUT/link.log; exit 1; }
$OUT/glass_seq
