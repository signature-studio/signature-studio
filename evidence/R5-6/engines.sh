#!/bin/bash
# R5-6: the engine DLLs that Frequency Shifter and FLEX load (census of the engines, one run each). Input: the three DLLs in ~/.cache/signature-studio/r56/in
FE=$HOME/.cache/signature-studio/cargo-r56/release
I=$HOME/.cache/signature-studio/r56/in
O=/tmp/opencode/r56/eng
REPO=$HOME/work/signature-studio
cd "$REPO" || exit 1
mkdir -p $O
for spec in "fs_engine frequency_shifter_engine_x64.dll 0x9e00000" "flex_engine flex_engine_x64.dll 0xa000000" "flex_enginev2 flex_enginev2_x64.dll 0xa800000"; do
  set -- $spec
  $FE/fe-pe rebase $I/$2 $3 $O/$1.dll > $O/$1.rebase.txt
  /usr/bin/time -v $FE/fe-lift lift $O/$1.dll /dev/null --pdata --extra-imports lift/harness/seeds/ship6-imports.txt > $O/$1.stdout 2> $O/$1.stderr
  echo "$1 rc=$?"
done
