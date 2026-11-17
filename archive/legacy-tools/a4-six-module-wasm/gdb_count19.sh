#!/bin/bash
export FLE_VFS_SEED=/tmp/opencode/wasm6/src/lift/harness/seeds/vfs-seed-fonts.tsv FLE_VFS_SEED_ROOT=/home/ubuntu/work/signature-studio-develop
V=/tmp/opencode/wasm6/oc2/v-t2331d00e8ebb
D=$V/dll
LIST=$(cat /tmp/opencode/wasm6/results/modlist.txt | sed 's|/on/|/oc2/|g')
exec gdb -q -batch -x /tmp/opencode/wasm6/tools/gdb_count19.py --args $V/native/two-native /home/ubuntu/work/signature-studio-develop/binaries/win64/FLEngine_x64.dll "$LIST"
