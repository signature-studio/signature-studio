#!/bin/bash
# lat1: builds lat1.diff (new files only: lift/harness/web/lat1/*) against a git archive of HEAD, with the work-tree paths of the tools replaced by LAT1_* environment variables. usage: package.sh OUTDIR
set -eu
W=/home/ubuntu/work/signature-studio-develop/lift/out-lat1; OUT=$1
rm -rf $OUT/pkgtmp; mkdir -p $OUT/pkgtmp/a $OUT/pkgtmp/b
git -C /home/ubuntu/work/signature-studio-develop archive HEAD lift/harness/web | tar -x -C $OUT/pkgtmp/a
git -C /home/ubuntu/work/signature-studio-develop archive HEAD lift/harness/web | tar -x -C $OUT/pkgtmp/b
D=$OUT/pkgtmp/b/lift/harness/web/lat1; mkdir -p $D
for f in hooks.mjs lib.mjs run.mjs analyze.mjs playwall.mjs trace.mjs rawupdate.mjs explore.mjs explore2.mjs probe-wheel.mjs probe-wheel2.mjs series.sh chain.mjs mk-nat-script.py run-nat.sh relink-lat1.sh covwatch.mjs natan.py package.sh strip.mjs prof-f8.mjs firstuse.py mk-owner.py mk-trace-table.py waitfile.mjs; do cp $W/tools/$f $D/$f; done
cp $W/doc/lat1-README.md $D/README.md
python3 - $D <<'PY'
import sys, re, os
d = sys.argv[1]
def sub(f, pairs):
    p = os.path.join(d, f); s = open(p).read()
    for a, b in pairs:
        if a not in s: print('MISSING in', f, ':', a[:70])
        s = s.replace(a, b)
    open(p, 'w').write(s)
sub('lib.mjs', [
 ("export const ROOT = '/home/ubuntu/work/signature-studio-develop';\nexport const SRC = `${ROOT}/lift/out-lat1/src/app`;\nexport const ASSETS = `${ROOT}/lift/out-websrv/assets`;",
  "import { fileURLToPath } from 'node:url';\n// LAT1_ROOT: the repository (default: four levels above this file); LAT1_APP: the app tree (default <root>/app); LAT1_DIST: its vite build (default <app>/dist); LAT1_ASSETS: the bundles (read only)\nexport const ROOT = process.env.LAT1_ROOT ?? fileURLToPath(new URL('../../../../', import.meta.url)).replace(/\\/$/, '');\nexport const SRC = process.env.LAT1_APP ?? `${ROOT}/app`;\nexport const ASSETS = process.env.LAT1_ASSETS ?? `${ROOT}/lift/out-websrv/assets`;\nexport const DIST = process.env.LAT1_DIST ?? `${SRC}/dist`;"),
 ("'--outDir', '../../dist-head'", "'--outDir', DIST"),
])
for f in ['run.mjs', 'rawupdate.mjs']:
    p = os.path.join(d, f); s = open(p).read()
    s = s.replace("${ROOT}/lift/out-lat1/src/app/test/e2e/", "${SRC}/test/e2e/")
    s = re.sub(r"import \{ ([^}]*)\} from './lib.mjs';", lambda m: "import { " + m.group(1) + (", SRC " if 'SRC' not in m.group(1) else "") + "} from './lib.mjs';", s, count=1)
    open(p, 'w').write(s)
PY
(cd $OUT/pkgtmp && diff -ruN a b > $OUT/lat1.diff) || true
for f in $D/*.mjs; do node --check $f || echo "SYNTAX $f"; done
python3 -c "import ast,sys;[ast.parse(open('$D/'+f).read()) for f in ('mk-nat-script.py','natan.py')]"
echo "lat1.diff: $(grep -c '^+++' $OUT/lat1.diff) files, $(wc -c < $OUT/lat1.diff) bytes"
