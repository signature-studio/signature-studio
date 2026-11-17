#!/bin/bash
# R5-6 first census run (shared hook file for all five; the four failures are the record). rebase (fe-pe) and lift (fe-lift, one C file, --pdata, as gen-all does per module) for the Balance/Filter
# calibration pair and the five candidate modules. Output in /tmp/opencode/r56/out. Nothing is written under the repository.
FE=$HOME/.cache/signature-studio/cargo-r56/release
IN=$HOME/.cache/signature-studio/r56/in
OUT=/tmp/opencode/r56/out
REPO=$HOME/work/signature-studio
HK=lift/harness/hooks/fruity-delphi-effect-hook-points.txt
EX=lift/harness/seeds/ship6-imports.txt
cd "$REPO" || exit 1

one() {
  # usage: one NAME INPUT BASE VARIANT(hook|nohook)
  local name=$1 in=$2 base=$3 var=$4
  local r=$OUT/$name.$var
  $FE/fe-pe rebase "$in" "$base" "$r.dll" > "$r.rebase.txt" 2>&1
  local rc1=$?
  local hk=""
  if [ "$var" = hook ]; then hk="--hook-points $HK"; fi
  if [ $rc1 -ne 0 ]; then echo "$name $var REBASE_RC=$rc1" >> $OUT/summary.txt; return; fi
  /usr/bin/time -v $FE/fe-lift lift "$r.dll" "$r.c" --pdata $hk --extra-imports $EX > "$r.stdout" 2> "$r.stderr"
  local rc=$?
  echo "$name $var base=$base rc=$rc" >> $OUT/summary.txt
}

rm -f $OUT/summary.txt
one balance "$IN/fruity_balance_x64.dll" 0x6800000 hook
one filter "$IN/fruity_filter_x64.dll" 0x7000000 hook
one delay2 "$IN/fruity_delay2_x64.dll" 0x7800000 hook
one freqshifter "$IN/frequency_shifter_x64.dll" 0x8000000 hook
one soundgoodizer "$IN/soundgoodizer_x64.dll" 0x8800000 hook
one peq2 "$IN/fruity_parametric_eq2_x64.dll" 0x9000000 hook
one flex "$IN/flex_x64.dll" 0x9800000 hook
one delay2 "$IN/fruity_delay2_x64.dll" 0x7800000 nohook
one freqshifter "$IN/frequency_shifter_x64.dll" 0x8000000 nohook
one soundgoodizer "$IN/soundgoodizer_x64.dll" 0x8800000 nohook
one peq2 "$IN/fruity_parametric_eq2_x64.dll" 0x9000000 nohook
one flex "$IN/flex_x64.dll" 0x9800000 nohook
echo done >> $OUT/summary.txt
