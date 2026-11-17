#!/bin/bash
# ci/test-hybrid.sh - checks of the hybrid stage's data and cache keys (decisions 0230, 0239, 0243); nothing is built:
#   1. ship5-imports.txt = ship4-imports.txt (unchanged, first) + ref1-imports.txt (last), no (dll, API) pair twice; ship6-imports.txt (decision 0302) = ship5 (unchanged, first) + dwmti-imports.txt (last), no pair twice
#   2. the strip of the C units on a synthetic unit, and the lists of the hybrid (decision 0239): hyb-U-<module>.txt of the 8 modules (3,813 functions: ship6 H9opt 3,879 minus the 65 + 1 that contain a LOCK-prefixed instruction, decisions 0246 and 0243), the extraction rows of oggio / elastique, the seeds of elastique
#   3b. ci/plan.sh: a change of the seed's data (the seed list, os-fonts/) changes the keys of extract, seed, bundle and not those of lift, rt, link
#   3. ci/plan.sh: a change of one line of a list changes the keys of the stages that depend on it (lift, link, seed, bundle) and not the keys of the stages that do not (extract, felift, vst3, app, emsdk); the lifter key names fe-wasmgen
#   4. the address guard of the hybrid stage (hyb_guard)
set -u -o pipefail
here=$(cd "$(dirname "$0")" && pwd); repo=$(cd "$here/.." && pwd); fail=0
ok() { echo "ok   $*"; }
bad() { echo "FAIL $*"; fail=1; }
t=$(mktemp -d); trap 'rm -rf "$t"' EXIT
get() { awk -F= -v k="$1" '$1==k {sub(/^[^=]*=/,""); print}'; }

# ---- 1. the rows
s=$repo/lift/harness/seeds
rows() { grep -v '^[[:space:]]*#' "$1" | awk 'NF'; }
n5=$(rows "$s/ship5-imports.txt" | wc -l); n4=$(rows "$s/ship4-imports.txt" | wc -l); nr=$(rows "$s/ref1-imports.txt" | wc -l)
[ "$n5" = $((n4 + nr)) ] && ok "ship5-imports.txt has $n5 rows = $n4 (ship4) + $nr (ref1)" || bad "ship5-imports.txt has $n5 rows, ship4 $n4 + ref1 $nr"
[ "$(rows "$s/ship5-imports.txt" | head -n "$n4")" = "$(rows "$s/ship4-imports.txt")" ] && ok "the first $n4 rows are the rows of ship4-imports.txt, in order" || bad "the first rows of ship5-imports.txt are not ship4-imports.txt"
[ "$(rows "$s/ship5-imports.txt" | tail -n "$nr")" = "$(rows "$s/ref1-imports.txt")" ] && ok "the last $nr rows are the rows of ref1-imports.txt, in order" || bad "the last rows of ship5-imports.txt are not ref1-imports.txt"
d=$(rows "$s/ship5-imports.txt" | awk '{print tolower($1) " " tolower($2)}' | LC_ALL=C sort | uniq -d | wc -l)
[ "$d" = 0 ] && ok "no (dll, API) pair twice" || bad "$d (dll, API) pair(s) twice"
n6=$(rows "$s/ship6-imports.txt" | wc -l); nd=$(rows "$s/dwmti-imports.txt" | wc -l)
[ "$n6" = $((n5 + nd)) ] && ok "ship6-imports.txt has $n6 rows = $n5 (ship5) + $nd (dwmti)" || bad "ship6-imports.txt has $n6 rows, ship5 $n5 + dwmti $nd"
[ "$(rows "$s/ship6-imports.txt" | head -n "$n5")" = "$(rows "$s/ship5-imports.txt")" ] && ok "the first $n5 rows are the rows of ship5-imports.txt, in order" || bad "the first rows of ship6-imports.txt are not the rows of ship5-imports.txt"
[ "$(rows "$s/ship6-imports.txt" | tail -n "$nd")" = "$(rows "$s/dwmti-imports.txt")" ] && ok "the last $nd row(s) are the rows of dwmti-imports.txt" || bad "the last rows of ship6-imports.txt are not the rows of dwmti-imports.txt"
d=$(rows "$s/ship6-imports.txt" | awk '{print tolower($1) " " tolower($2)}' | LC_ALL=C sort | uniq -d | wc -l)
[ "$d" = 0 ] && ok "ship6: no (dll, API) pair twice" || bad "ship6: $d (dll, API) pair(s) twice"

fn() { awk 'NF && !/^#/ {print $1}' "$1"; }   # the function lines of a list file
u=$t/unit.c; printf 'void f_1000(void)\n{\n  r[0] = 1;\n}\nvoid f_2000(void)\n{\n  r[0] = 2;\n}\nvoid f_3000(void)\n{\n  r[0] = 3;\n}\n' > "$u"; printf '0x1000\n0x3000\n' > "$t/l.txt"
sl=$(python3 "$repo/lift/harness/hyb/strip_units.py" "$u" "$t/l.txt" "$t/unit.out.c" 2> /dev/null) || sl=""
{ [ -s "$t/unit.out.c" ] && [ "$(grep -c 'r\[0\] = 2' "$t/unit.out.c")" = 1 ] && [ "$(grep -c 'r\[0\] = [13]' "$t/unit.out.c")" = 0 ]; } && ok "strip_units.py: a listed function's body goes, the other stays ($sl)" || bad "strip_units.py on a synthetic unit: '$sl'"

# ---- 2. the lists of the hybrid (decision 0239): 8 modules, a function once in a module; the extraction rows of oggio / elastique; the cost limit is a constant of fe-wasmgen
nu=0; du=0
for m in fle dsp b2d qfc ft ilt oggio elastique; do
  f=$s/hyb-U-$m.txt
  nu=$((nu + $(fn "$f" | wc -l))); du=$((du + $(fn "$f" | sort | uniq -d | wc -l)))
done
[ "$nu" = 3875 ] && [ "$du" = 0 ] && ok "the hyb-U lists hold 3,875 functions in 8 modules, no function twice in a module (the strip drops that many)" || bad "the hyb-U lists hold $nu functions, $du twice"
[ "$(fn "$s/hyb-U-elastique.txt" | wc -l)" = 165 ] && [ "$(fn "$s/hyb-U-oggio.txt" | wc -l)" = 46 ] && ok "hyb-U: elastique 165, oggio 46" || bad "hyb-U: elastique / oggio counts"
grep -q 'pub const MAX_FN_COST: u64 = 75_000_000;' "$repo/lift/crates/fe-wasmgen/src/lib.rs" && ok "fe-wasmgen's cost limit is 75,000,000 (decision 0264: the locals of the flag operands are shared, so the 2 large Elastique functions and the 62 MB Blend2D function fit)" || bad "fe-wasmgen: no cost limit of 75,000,000"
[ "$(ls "$s"/hyb-*.txt | wc -l)" = 8 ] && ok "only the 8 lists of the hybrid are there (one data file per module, decision 0243)" || bad "more or fewer than 8 hyb-*.txt files"
awk -F'\t' '$2 ~ /out-sampledec/ {n++} END {exit n == 2 ? 0 : 1}' "$repo/ci/extract-dlls.tsv" && ok "ci/extract-dlls.tsv has the rows of oggio_x64.dll and elastique_x64.dll" || bad "ci/extract-dlls.tsv: the oggio/elastique rows are missing"
[ -s "$s/elastique-seeds-all3.txt" ] && ok "the seeds of elastique_x64.dll are in lift/harness/seeds" || bad "elastique-seeds-all3.txt is missing"


# ---- 3. the keys of ci/plan.sh, in a copy of the files it reads (so that a list can be changed); a tree without .git is hashed by content
w3=$t/tree; mkdir -p "$w3"
( cd "$repo" && tar -cf - ci app/scripts lift/Cargo.toml lift/Cargo.lock lift/crates lift/harness lift/runtime docs/investigations/a4-six-module-wasm/tools docs/investigations/wasm-final0129/tools docs/investigations/prune1/tables tools 2>/dev/null ) | tar -xf - -C "$w3"
mkdir -p "$w3/app" "$w3/lift/harness"; [ -e "$w3/app/package.json" ] || cp "$repo/app/package.json" "$w3/app/" 2>/dev/null
keys() { ( cd "$w3" && GIT_CEILING_DIRECTORIES="$t" bash ci/plan.sh /dev/stdout 2> /dev/null ); }   # the copy is hashed by content even when the temporary directory lies inside a git work tree
k() { echo "$1" | get "$2"; }
p1=$(keys)
sed -i '1s/.*/0x1/' "$w3/lift/harness/seeds/hyb-U-dsp.txt"
p2=$(keys)
for name in k_lift k_link k_bundle k_seed; do
  [ -n "$(k "$p1" $name)" ] && [ "$(k "$p1" $name)" != "$(k "$p2" $name)" ] && ok "$name differs when one line of a hybrid list changes" || bad "$name does not depend on the hybrid lists"
done
for name in k_extract k_felift k_b2d k_emsdk k_vst3 k_app; do
  [ -n "$(k "$p1" $name)" ] && [ "$(k "$p1" $name)" = "$(k "$p2" $name)" ] && ok "$name does not depend on the hybrid lists" || bad "$name depends on the hybrid lists"
done
# ---- 3b. (seed data) a change of the seed's own data (a row of the seed list, a font under os-fonts/) changes the keys of extract, seed and bundle and NOT the keys of lift, rt (wasm objects) and link: a push that only
# changes the seed must skip the lift and the C -> wasm stages. The copy is a git repository of its own: hashpaths uses `git ls-files` and its :(exclude) pathspecs there (a tree without .git is hashed by content and does not honour them)
w4=$t/tree4; mkdir -p "$w4"
( cd "$repo" && tar -cf - ci app/scripts lift/Cargo.toml lift/Cargo.lock lift/crates lift/harness lift/runtime docs/investigations/a4-six-module-wasm/tools docs/investigations/wasm-final0129/tools docs/investigations/prune1/tables tools 2>/dev/null ) | tar -xf - -C "$w4"
mkdir -p "$w4/app" "$w4/lift/harness"; [ -e "$w4/app/package.json" ] || cp "$repo/app/package.json" "$w4/app/" 2>/dev/null
g4() { ( cd "$w4" && git -c user.name=t -c user.email=t@t "$@" >/dev/null 2>&1 ); }
keys4() { ( cd "$w4" && GIT_CEILING_DIRECTORIES="$t" bash ci/plan.sh /dev/stdout 2> /dev/null ); }
g4 init -q .; g4 add -A; g4 commit -qm base
q1=$(keys4)
printf 'C:\\Windows\\Fonts\\seedtest.ttf\tlift/harness/seeds/os-fonts/seedtest.ttf\t%s\n' "$(printf x | sha256sum | cut -d' ' -f1)" >> "$w4/lift/harness/seeds/vfs-seed-idle1.tsv"; echo x > "$w4/lift/harness/seeds/os-fonts/seedtest.ttf"
g4 add -A; g4 commit -qm seed
q2=$(keys4)
for name in k_extract k_seed k_bundle prefix; do
  [ -n "$(k "$q1" $name)" ] && [ "$(k "$q1" $name)" != "$(k "$q2" $name)" ] && ok "$name differs when the seed list and a font under os-fonts/ change" || bad "$name does not depend on the seed data"
done
for name in k_lift k_rt k_link k_felift k_b2d k_emsdk k_vst3 k_app; do
  [ -n "$(k "$q1" $name)" ] && [ "$(k "$q1" $name)" = "$(k "$q2" $name)" ] && ok "$name does not depend on the seed data (a seed-only push skips it)" || bad "$name depends on the seed data"
done
case "$(k "$p1" prefix)" in fl-engine-ci-????????) ok "the bundle prefix is fl-engine-ci- and the end of the bundle key" ;; *) bad "prefix=$(k "$p1" prefix)" ;; esac
grep -q 'bins=fe-pe,fe-lift,fe-wasmgen' "$repo/ci/plan.sh" && ok "the lifter key names the fe-wasmgen build" || bad "ci/plan.sh: the lifter key does not name fe-wasmgen"
grep -q 'fe-wasmgen' "$repo/.github/workflows/deploy.yml" && ok "deploy.yml caches the fe-wasmgen binary with the lifter" || bad "deploy.yml: P_FELIFT lacks fe-wasmgen"

# ---- 4. (decision 0238b) the address guard of the hybrid stage (the function hyb_guard of build-engine-bundle.sh, hashed into SP_WASM; not a file of lift/harness, which would change the key of the lift stage): the direct wasm gets fle_teb / fle_mxcsr / fle_hyb_buf / fle_progress_count from the link map, so the link map has to be the one of the C part
bb=$repo/app/scripts/build-engine-bundle.sh; w=$t/guard; mkdir -p "$w"
eval "$(awk '/^hyb_guard\(\) \{/ {p=1} p {print} p && /^}/ {p=0}' "$bb")"   # the function itself, out of the script that ci/plan.sh hashes
# `i32.const <address>` = 0x41 + sleb128 (written out here, not computed by the guard): layout A (a bundle that works) and layout B (0x40 further, the one of a sibling build)
python3 - "$w" <<'PY'
import sys
d = sys.argv[1]
A = {'teb': 'd0f4c68401', 'mx': 'f4a9818101', 'hb': 'e0b4c78401', 'pg': '8883c28101'}
B = {'teb': '90f5c68401', 'mx': 'b4aa818101', 'hb': 'a0b5c78401', 'pg': 'c883c28101'}
def mod(keys, extra=()):
    return bytes.fromhex('0061736d01000000') + b''.join(b'\x41' + bytes.fromhex(k) + b'\x1a' for k in keys) + b''.join(b'\x41' + bytes.fromhex(k) + b'\x28\x02\x00' for k in extra)
open(d + '/c-a.wasm', 'wb').write(mod([A['teb'], A['mx'], A['hb'], A['pg']]))
open(d + '/c-b.wasm', 'wb').write(mod([B['teb'], B['mx'], B['hb'], B['pg']]))
open(d + '/c-a-nopg.wasm', 'wb').write(mod([A['teb'], A['mx'], A['hb']]))
# decision 0285: the three data of the typed indirect call (fle_progress_limit, fle_landing_top, fle_dispatch_hook), as `i32.const` in the optimised C part (7, 23 and 5 times in the cold build of E10 + 0285; written out here)
def sleb(v):
    o = bytearray()
    while True:
        b = v & 0x7f; v >>= 7
        if (v == 0 and not (b & 0x40)) or (v == -1 and (b & 0x40)): o.append(b); return bytes(o).hex()
        o.append(b | 0x80)
T3 = [sleb(0x10209708), sleb(0x1030cc54), sleb(0x1030cc40)]
open(d + '/c-a7.wasm', 'wb').write(mod([A['teb'], A['mx'], A['hb'], A['pg']] + T3))
open(d + '/c-a7-nolanding.wasm', 'wb').write(mod([A['teb'], A['mx'], A['hb'], A['pg'], T3[0], T3[2]]))
open(d + '/m-a.wasm', 'wb').write(mod([A['teb'], A['mx'], A['hb'], A['pg']], [A['mx'], A['mx'], A['hb']]))
open(d + '/m-b.wasm', 'wb').write(mod([A['teb'], A['mx'], A['hb'], A['pg']], [B['mx'], B['mx'], B['hb']]))
PY
ga=(0x1091ba50 0x102054f4 0x1091da60 0x10308188); gb=(0x1091ba90 0x10205534 0x1091daa0 0x103081c8)
hyb_guard cpart "$w/c-a.wasm" "${ga[@]}" > "$w/o1" 2>&1 && ok "hyb_guard cpart: a C part that uses all four addresses passes" || bad "hyb_guard cpart refused a C part that uses the four addresses: $(cat "$w/o1")"
hyb_guard cpart "$w/c-a.wasm" "${gb[@]}" > "$w/o2" 2>&1 && bad "hyb_guard cpart passed a C part that uses other addresses (the sibling layout, +0x40)" || ok "hyb_guard cpart: a C part of another layout than the addresses given fails"
hyb_guard cpart "$w/c-b.wasm" "${ga[@]}" > "$w/o3" 2>&1 && bad "hyb_guard cpart passed the reverse mismatch" || ok "hyb_guard cpart: the reverse mismatch fails too"
hyb_guard cpart "$w/c-a-nopg.wasm" "${ga[@]}" > "$w/o4" 2>&1 && bad "hyb_guard cpart passed a C part without fle_progress_count" || ok "hyb_guard cpart: one address that the C part does not use is enough to fail"
g7=(0x10209708 0x1030cc54 0x1030cc40)
hyb_guard cpart "$w/c-a7.wasm" "${ga[@]}" "${g7[@]}" > "$w/o5a" 2>&1 && ok "hyb_guard cpart: a C part that uses all seven addresses (decision 0285) passes" || bad "hyb_guard cpart refused a C part that uses the seven addresses: $(cat "$w/o5a")"
hyb_guard cpart "$w/c-a7-nolanding.wasm" "${ga[@]}" "${g7[@]}" > "$w/o5b" 2>&1 && bad "hyb_guard cpart passed a C part without fle_landing_top" || ok "hyb_guard cpart: the three data of the typed indirect call are checked too (one that the C part does not use fails)"
hyb_guard cpart "$w/c-a.wasm" "${ga[@]}" "${g7[@]}" > "$w/o5c" 2>&1 && bad "hyb_guard cpart passed a C part that uses none of the three new addresses" || ok "hyb_guard cpart: seven addresses against a C part with only the four fail"
hyb_guard merged "$w/c-a.wasm" "$w/m-a.wasm" 0x102054f4 0x1091da60 > "$w/o5" 2>&1 && ok "hyb_guard merged: direct code that uses fle_mxcsr and fle_hyb_buf of the link passes" || bad "hyb_guard merged refused a good merged module: $(cat "$w/o5")"
hyb_guard merged "$w/c-a.wasm" "$w/m-b.wasm" 0x102054f4 0x1091da60 > "$w/o6" 2>&1 && bad "hyb_guard merged passed direct code made with another layout" || ok "hyb_guard merged: direct code made with the addresses of another layout fails"
printf 'link map A\n' > "$w/link.map"; hyb_guard record "$w/link.map" "$w/link.map.sha256"
hyb_guard map "$w/link.map" "$w/link.map.sha256" > "$w/o7" 2>&1 && ok "hyb_guard map: the link map that was recorded passes" || bad "hyb_guard map refused the recorded link map: $(cat "$w/o7")"
printf 'link map B\n' > "$w/link.map"
hyb_guard map "$w/link.map" "$w/link.map.sha256" > "$w/o8" 2>&1 && bad "hyb_guard map passed a link map that was overwritten after the record" || ok "hyb_guard map: a link map that another build overwrote fails"
hyb_guard map "$w/link.map" "$w/no-such-record" > "$w/o9" 2>&1 && grep -q 'no record' "$w/o9" && ok "hyb_guard map: no record = a note and no failure (the check of the C part stays)" || bad "hyb_guard map: no record: $(cat "$w/o9")"
{ grep -q 'hyb_guard record' "$bb" && grep -q 'hyb_guard map' "$bb" && grep -q 'hyb_guard cpart' "$bb" && grep -q 'hyb_guard merged' "$bb"; } && ok "build-engine-bundle.sh calls the four checks (record in weblink; map, cpart, map, merged in the hybrid stage)" || bad "build-engine-bundle.sh does not call all four checks of hyb_guard.py"
hg=$(sed -n '/^stage_hybrid()/,/^}/p' "$bb" | grep -c 'hyb_guard '); wg=$(sed -n '/^stage_weblink()/,/^}/p' "$bb" | grep -c 'hyb_guard ')
[ "$hg" = 4 ] && [ "$wg" = 1 ] && ok "stage_hybrid holds 4 calls and stage_weblink 1; both are in SP_WASM of ci/plan.sh" || bad "guard calls: stage_hybrid $hg (want 4), stage_weblink $wg (want 1)"
grep -q 'stage_hybrid hyb_guard;' "$repo/ci/plan.sh" && ok "ci/plan.sh hashes the function hyb_guard into SP_WASM (with stage_weblink and stage_hybrid): a change of the guard changes the keys of rt, link and bundle, and not the key of lift" || bad "ci/plan.sh does not hash hyb_guard"
[ -z "$(git -C "$repo" ls-files -- lift/harness/hyb/hyb_guard.py 2>/dev/null)" ] && [ ! -e "$repo/lift/harness/hyb/hyb_guard.py" ] && ok "no new file under lift/harness (H_GEN, the key of the lift stage, is unchanged by the guard)" || bad "a hyb_guard.py under lift/harness changes the key of the lift stage"

# ---- 5. (decision 0285) the typed indirect call: the stage that finds the C functions the direct wasm calls (hybn0) and the stage that makes it (hybrid) both pass --hybrid-dispatch, and the three data it reads (fle_progress_limit, fle_landing_top, fle_dispatch_hook) come from the link map like fle_progress_count
hn=$(sed -n '/^stage_hybn0()/,/^}/p' "$bb" | grep -c -- '--hybrid-dispatch'); hh=$(sed -n '/^stage_hybrid()/,/^}/p' "$bb" | grep -c -- '--hybrid-dispatch')
[ "$hn" = 1 ] && [ "$hh" = 1 ] && ok "hybn0 and hybrid both pass --hybrid-dispatch to fe-wasmgen (the module that imports c.fle_hyb_slice is the same in both, so the export list of the C part has it)" || bad "--hybrid-dispatch: hybn0 $hn (want 1), hybrid $hh (want 1)"
{ grep -q 'fle_progress_limit\$' "$bb" && grep -q 'fle_landing_top\$' "$bb" && grep -q 'fle_dispatch_hook\$' "$bb"; } && ok "the link map gives fle_progress_limit, fle_landing_top and fle_dispatch_hook (a missing one stops the stage)" || bad "build-engine-bundle.sh does not read the three data of the typed indirect call from the link map"

[ "$fail" = 0 ] && echo "test-hybrid: all checks hold" || { echo "test-hybrid: FAILED"; exit 1; }
