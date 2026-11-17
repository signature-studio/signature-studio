#!/bin/bash
# pick1: copy the current tables / tools / docs / scripts into the results directory and rebuild the two diffs (pick1.diff = new files only; pick1-spi.diff = runtime + tests). Run from lift/out-pick1.
set -eu
R=../out-git/pick1-results
rm -rf stage && cp -al src-orig stage
mkdir -p stage/lift/harness/pick1 stage/docs/investigations/pick1/{tables,logs,scripts}
for f in README.md apiw.h apiw_core.c pix_wrap.c gen-apiwrap.py link-native.sh link-plain.sh run-native.sh ops-pick1.tsv mk-pick1-scripts.py mk-api-table.py mk-path-matrix.py mk-cap-table.py prof-modules.py parse-gdb-log.py gdb-api.py raw2png.py html2txt.py xref.py sync-results.sh; do cp tools/$f stage/lift/harness/pick1/$f; done
cp -r tables/. stage/docs/investigations/pick1/tables/
for f in logs/cargo-test-spi.log logs/cargo-test-full.log logs/a4-off.log logs/ci-lift.log logs/delay-imports.txt logs/b2d-build.log; do [ -e $f ] && cp $f stage/docs/investigations/pick1/logs/; done
cp scripts/pk.native.g3 scripts/pk.wasm.g3 scripts/explore1.g3 scripts/sel1.native.g3 scripts/dd1.native.g3 scripts/cr1.native.g3 scripts/pk2.native.g3 stage/docs/investigations/pick1/scripts/
diff -ruN src-orig stage > $R/pick1.diff || true
# explicit file list: a running cargo test creates lift/out-* and lift/target inside the working tree, which must never reach the diff
: > $R/pick1-spi.diff
for f in lift/runtime/fle_spi.c lift/runtime/fle_spi.h lift/crates/fe-lift/tests/spi.rs; do diff -u --label a/$f --label b/$f src-orig/$f src/$f >> $R/pick1-spi.diff || true; done
rm -rf $R/tables $R/tools; mkdir -p $R/tables $R/tools $R/logs $R/docs
cp -r tables/. $R/tables/; cp tools/* $R/tools/
for f in logs/cargo-test-spi.log logs/cargo-test-full.log logs/a4-off.log logs/ci-lift.log logs/delay-imports.txt; do [ -e $f ] && cp $f $R/logs/; done
cp docsrc/0242-fl-drawing-paths.md $R/docs/
echo synced; du -sh $R | cut -f1
