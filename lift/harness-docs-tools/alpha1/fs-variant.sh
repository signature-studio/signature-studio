#!/bin/bash
# alpha1: the objects of ciW linked WITH the Emscripten file system (NO_FS=0), uncompressed bundle, for lift/harness/parity/node-bundle-run.mjs style runs in node (MEMFS). After lift/out-ref3/tools/fs-variant.sh (decision 0233); the preset stays F2xH7 (the hybrid included).
# usage: fs-variant.sh CINAME TREE   -> CINAME-fs/engine/bundle/<prefix>
set -eu
N=$1; TREE=$2; R=/home/ubuntu/work/signature-studio-develop/lift/out-alpha1; C=$R/$N; F=$R/$N-fs
[ ! -e "$F" ] || { echo "$F exists"; exit 1; }
mkdir -p "$F/engine/w-Oz" "$F/engine/logs"; ln -s "$C/host" "$F/host"; cp -al "$C/b2d" "$F/b2d"
for d in cargo-target onat wsrc emcache hyb febin-prune febin-size-opts in tmp; do [ -e "$C/engine/$d" ] && cp -al "$C/engine/$d" "$F/engine/$d"; done
cp -al "$C/engine/w-Oz/obj" "$F/engine/w-Oz/obj"
export NO_FS=0 COMPRESS=none
bash $R/tools/run-ci-build.sh "$TREE" "$F" F2xH7 link seed bundle > "$R/logs/fs-$N.log" 2>&1
tail -2 "$R/logs/fs-$N.log"; ls "$F"/engine/bundle/*/ | head -8
