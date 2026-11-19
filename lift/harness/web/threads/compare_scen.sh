#!/bin/bash
# usage: compare_scen.sh NATIVE_BIN WASM_DIR OUTDIR [scenario ...]
# Runs every scenario (default: all of `NATIVE_BIN list`) with the native program and with the wasm program under node (--experimental-wasm-jspi), stores
# OUTDIR/<name>.native.{out,err,rc} and .wasm.{out,err,rc}, and reports per scenario whether stdout, stderr and the exit status are byte-identical.
# A scenario that stops by name (abort) exits 134 natively and traps in wasm: for these only "nonzero on both" is compared for the status, and stdout plus the stderr lines
# without the JS/abort trace and with the host address of `rsp` normalised (see the code) must be identical. For a scenario that ends normally the JS scheduler's counters must also
# show starts == ends (no JSPI stack left suspended). The last line is `IDENTICAL n of m`.
NATIVE=$1; WASM=$2; OUT=$3; shift 3
HERE=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$OUT"
names=("$@")
[ ${#names[@]} -gt 0 ] || mapfile -t names < <("$NATIVE" list | awk '{print $1}')
same=0; total=0
for s in "${names[@]}"; do
    total=$((total + 1))
    "$NATIVE" "$s" > "$OUT/$s.native.out" 2> "$OUT/$s.native.err"; echo $? > "$OUT/$s.native.rc"
    FLE_SCEN_STATS="$OUT/$s.wasm.stats" node --experimental-wasm-jspi "$HERE/run_node_scen.cjs" "$WASM" "$s" > "$OUT/$s.wasm.out" 2> "$OUT/$s.wasm.err"; echo $? > "$OUT/$s.wasm.rc"
    nrc=$(cat "$OUT/$s.native.rc"); wrc=$(cat "$OUT/$s.wasm.rc")
    if [ "$nrc" = 0 ]; then rcok=$([ "$wrc" = 0 ] && echo yes || echo no); else rcok=$([ "$wrc" != 0 ] && echo yes || echo no); fi
    outok=$(cmp -s "$OUT/$s.native.out" "$OUT/$s.wasm.out" && echo yes || echo no)
    if [ "$nrc" = 0 ]; then
        errok=$(cmp -s "$OUT/$s.native.err" "$OUT/$s.wasm.err" && echo yes || echo no)
    else
        # a stop: the runtime's "fle: ..." line and the thread report that follows it must match, except (1) the value after `rsp` (a HOST address of the guest stack: ASLR makes it differ
        # between two native runs of the same program, wasm's is a constant), which is replaced by `rsp X`, and (2) the lines of wasm's abort report (`Aborted(...)`, `[js] ...`, `at ...`)
        norm() { sed -E 's/rsp 0x[0-9a-f]+/rsp X/' "$1" | grep -v '^\s*at \|^Aborted\|^\[js\] fle thread [0-9]* failed\|^$'; }
        errok=$(cmp -s <(norm "$OUT/$s.native.err") <(norm "$OUT/$s.wasm.err") && echo yes || echo no)
    fi
    # leak check (wasm only, scenarios that end normally): every JSPI stack that was started must have ended, i.e. starts == ends in the JS scheduler's own counters
    leakok=yes
    if [ "$nrc" = 0 ]; then
        st=$(python3 -c "import json,sys;d=json.load(open('$OUT/$s.wasm.stats'));print(d['starts'],d['ends'])" 2>/dev/null)
        [ -n "$st" ] && [ "${st% *}" = "${st#* }" ] || leakok="no(starts/ends ${st:-none})"
    fi
    ev=$(grep -c . "$OUT/$s.native.err")
    if [ "$outok$errok$rcok$leakok" = yesyesyesyes ]; then same=$((same + 1)); r=IDENTICAL; else r=DIFFERENT; fi
    echo "$r $s (stdout $outok, stderr $errok [$ev lines], exit native $nrc wasm $wrc$([ "$leakok" = yes ] || echo ", LEAK $leakok"))"
done
echo "IDENTICAL $same of $total"
[ "$same" = "$total" ]
