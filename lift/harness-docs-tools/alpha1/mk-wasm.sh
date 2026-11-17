#!/bin/bash
# gpu1: an incremental build of F2xH7 from ciA (the cold build of HEAD): lift fresh by the CI stage, strip, the objects of ciA whose unit C is byte-identical hard-linked, the runtime objects (rt/ h-fruity/ common.o) compiled again, link, seed, bundle (COMPRESS=none unless set).
# usage: mk-inc.sh NAME TREE   -> lift/out-gpu1/ciNAME, logs/build-NAME-*.log   (like lift/out-ship5/tools/s5-mk-var2.sh)
set -eu
REPO="$(readlink -f "$(dirname "${BASH_SOURCE[0]}")/../../..")"; G="$REPO/lift/out-gpu1"; R="$REPO/lift/out-alpha1"; N=$1; A=${DONOR:-$G/ciA}; T=$R/$2; P=${PRESET:-F2xH7}; D=$R/ci$N
export PATH=$PATH:/opt/emsdk/node/24.19.0_64bit/bin
[ ! -e "$D" ] || { echo "$D exists"; exit 1; }
mkdir -p "$D/engine/w-Oz" "$D/engine/logs"; cp -a "$A/host" "$D/host"; cp -a "$A/b2d" "$D/b2d"
cp -a "$A/engine/cargo-target" "$D/engine/cargo-target"; cp -a "$A/engine/emcache" "$D/engine/emcache"
export CI_JOBS=${CI_JOBS:-4} COMPRESS=${COMPRESS:-none}
bash $R/tools/run-ci-build.sh "$T" "$D" "$P" extract lift > "$R/logs/build-$N-1.log" 2>&1 || { tail -n 5 "$R/logs/build-$N-1.log"; exit 1; }
( export CI_WORK=$D SHIP_PRESET=$P; cd "$T"; nice -n 15 bash -c '. ci/lib/common.sh; run_bb hybstrip' ) > "$R/logs/build-$N-strip.log" 2>&1 || { tail -n 5 "$R/logs/build-$N-strip.log"; exit 1; }
cp -a "$A/engine/w-Oz/obj" "$D/engine/w-Oz/obj"
rm -f "$D"/engine/w-Oz/obj/rt/*.o "$D"/engine/w-Oz/obj/h-fruity/*.o "$D"/engine/w-Oz/obj/mod/common.o
n=0; k=0
for f in $D/engine/wsrc/src/fle-[0-9][0-9].c $D/engine/wsrc/v-*/src/m[0-9]-[0-9][0-9].c $D/engine/wsrc/v-*/src/common.c; do
  u=$(basename "$f" .c); rel=${f#$D/engine/wsrc/}
  case $u in fle-*) o=fle/$u.o ;; *) o=mod/$u.o ;; esac
  if [ ! -e "$A/engine/wsrc/$rel" ] || ! cmp -s "$f" "$A/engine/wsrc/$rel"; then rm -f "$D/engine/w-Oz/obj/$o"; n=$((n + 1)); else k=$((k + 1)); fi
done
echo "ci$N: $n units without an object, $k units reused from ciA" | tee "$R/logs/build-$N-units.txt"
bash $R/tools/run-ci-build.sh "$T" "$D" "$P" objects link seed bundle > "$R/logs/build-$N-2.log" 2>&1
tail -n 3 "$R/logs/build-$N-2.log"
