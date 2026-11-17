#!/bin/bash
# R5-6 census (final): the variants chosen in evidence/R5-6.md (n=3 each for the time; the C goes to /dev/null, the stats are on stdout).
FE=$HOME/.cache/signature-studio/cargo-r56/release
IN=$HOME/.cache/signature-studio/r56/in
OUT=/tmp/opencode/r56/final
H=/tmp/opencode/r56/hooks
HK=lift/harness/hooks/fruity-delphi-effect-hook-points.txt
EX=lift/harness/seeds/ship6-imports.txt
REPO=$HOME/work/signature-studio
cd "$REPO" || exit 1
mkdir -p $OUT
rm -f $OUT/summary.txt
run() {
  # usage: run NAME DLL BASE HOOKFILE(or -) REP [noextra]
  local name=$1 in=$2 base=$3 hk=$4 rep=$5 mode=$6
  local r=$OUT/$name.r$rep
  $FE/fe-pe rebase "$in" "$base" "$r.dll" > /dev/null 2>&1 || { echo "$name r$rep REBASE_FAIL" >> $OUT/summary.txt; return; }
  local args=(--pdata)
  if [ "$hk" != "-" ]; then args+=(--hook-points "$hk"); fi
  if [ "$mode" != noextra ]; then args+=(--extra-imports "$EX"); fi
  /usr/bin/time -v $FE/fe-lift lift "$r.dll" /dev/null "${args[@]}" > "$r.stdout" 2> "$r.stderr"
  echo "$name r$rep rc=$?" >> $OUT/summary.txt
  rm -f "$r.dll"
}
for rep in 1 2 3; do
  run balance "$IN/fruity_balance_x64.dll" 0x6800000 "$HK" $rep
  run filter "$IN/fruity_filter_x64.dll" 0x7000000 "$HK" $rep
  run delay2 "$IN/fruity_delay2_x64.dll" 0x7800000 "$HK" $rep
  run freqshifter "$IN/frequency_shifter_x64.dll" 0x8000000 "$H/freqshifter-own.txt" $rep
  run soundgoodizer "$IN/soundgoodizer_x64.dll" 0x8800000 "$H/soundgoodizer-own.txt" $rep
  run peq2 "$IN/fruity_parametric_eq2_x64.dll" 0x9000000 "$H/peq2-own.txt" $rep
  run flex "$IN/flex_x64.dll" 0x9800000 - $rep
done
run balance_bare "$IN/fruity_balance_x64.dll" 0x6800000 - 1 noextra
run delay2_bare "$IN/fruity_delay2_x64.dll" 0x7800000 - 1 noextra
echo done >> $OUT/summary.txt
