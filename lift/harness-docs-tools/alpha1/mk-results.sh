#!/bin/bash
# alpha1: assemble lift/out-git/alpha1-results/ and alpha1.diff from the work tree (src/). usage: mk-results.sh
set -eu
R=/home/ubuntu/work/signature-studio-develop; A=$R/lift/out-alpha1; O=${O:-$R/lift/out-git/alpha1-results}
[ ! -e "$O" ] || { echo "$O exists (new names only)"; exit 1; }
cd $A; rm -rf work/head2 work/new2; mkdir -p work/head2 work/new2
(cd $R && git archive HEAD) | tar -x -C work/head2
cp -a work/head2/. work/new2/
for f in lift/runtime/fle_desk.c lift/runtime/fle_desk.h lift/runtime/fle_dwm.c lift/runtime/fle_gdi.c lift/runtime/fle_win.c lift/runtime/fle_win.h lift/crates/fe-lift/tests/wall_ia1.rs lift/crates/fe-lift/tests/alpha1_glass.rs; do cp src/$f work/new2/$f; done
mkdir -p work/new2/lift/harness/alpha1/g3; cp src/lift/harness/alpha1/{glass_seq.c,check-fade.py,run-fade.sh,README.md} work/new2/lift/harness/alpha1/; cp src/lift/harness/alpha1/g3/* work/new2/lift/harness/alpha1/g3/
cp docsrc/0244-translucent-windows.md work/new2/docs/decisions/0244-translucent-windows.md
D=work/new2/docs/investigations/alpha1; mkdir -p $D/tools $D/tables $D/learn $D/scripts
cp docsrc/MERGE-NOTES.md $D/MERGE-NOTES.md
for t in alpha1_rec.c alpha1_rec.h gen-wrap.py build-n1.sh run-n1.sh build-glass.sh mutate-glass.py mk-expect.py mk-tables.py fade-any.py gl-parity.c rgba2png.py sdrive-a1.mjs node-desk-run.mjs scn-alpha1.json series-report.py mk-results.sh mk-wasm.sh fs-variant.sh run-all.sh html2txt.py; do [ -e tools/$t ] && cp tools/$t $D/tools/ || cp learn/$t $D/tools/ 2>/dev/null || true; done
cp tables/* $D/tables/; cp learn/*.txt $D/learn/; cp scripts/*.g3 scripts/gen-g3.py $D/scripts/ 2>/dev/null || true
chmod +x work/new2/lift/harness/alpha1/run-fade.sh
(cd work && diff -ruN head2 new2 > alpha1.diff.raw) || true
mkdir -p $O/docs $O/tables $O/tools $O/logs $O/shots
sed 's#^--- head2/#--- a/#; s#^+++ new2/#+++ b/#' work/alpha1.diff.raw > $O/alpha1.diff
cp docsrc/0244-translucent-windows.md $O/docs/; cp docsrc/README-0244-line.txt $O/docs/; cp docsrc/MERGE-NOTES.md $O/MERGE-NOTES.md
cp -r $D/tables/* $O/tables/; cp $D/tools/* $O/tools/
cp work/alpha1-dirty.diff $O/alpha1-dirty.diff
for l in mutate-glass.txt felift-default.sha256 felift-default.out; do [ -e logs/$l ] && cp logs/$l $O/logs/; done
tail -60 logs/a4-off.log > $O/logs/a4-off-tail.log; tail -14 logs/test-full.log > $O/logs/test-full-1-tail.log; tail -9 logs/test-full-2.log > $O/logs/test-full-2-tail.log; tail -9 logs/test-full-3.log > $O/logs/test-full-3-tail.log
grep -h . logs/chrome2.log logs/firefox2.log logs/chrome-base.log > $O/logs/browser-runs.log 2>/dev/null || true
cp shots/*.png $O/shots/ 2>/dev/null || true
cp runs/node-pull1/desk.txt $O/logs/node-pull1-desk.txt 2>/dev/null; cp runs/node-f8/desk.txt $O/logs/node-f8-desk.txt 2>/dev/null; cp runs/node-cmp-fix/g3-lines.txt $O/logs/node-cmp-short-84.txt 2>/dev/null
cp runs/cmp-fix3/g3-lines.txt $O/logs/native-cmp-short-fix3-68.txt 2>/dev/null; cp runs/cmp-base/g3-lines.txt $O/logs/native-cmp-short-base-68.txt 2>/dev/null
echo "diff lines: $(wc -l < $O/alpha1.diff); minus lines: $(grep -c '^-[^-]' $O/alpha1.diff)"
