#!/usr/bin/env python3
"""b2d2: compare the RESULT / CHECK / VALUE lines of scan tests between a wasm run directory and a native reference directory (both: <test>.out files of scan-run.sh / scan_all.sh).
usage: scan-compare.py WASMDIR NATIVEDIR [OUT.tsv]    tests = the .out files of WASMDIR. A test is IDENTICAL when its filtered lines are equal in number and content; the native rc must be equal too (rc file)."""
import sys, os, re, glob
w, n = sys.argv[1], sys.argv[2]
pat = re.compile(r'^scan: (RESULT|CHECK|VALUE)')
def lines(f):
    return [l.rstrip('\n') for l in open(f, errors='replace') if pat.match(l)]
rows = []; same = 0; diff = []; tot = 0; nl = 0
for f in sorted(glob.glob(os.path.join(w, '*.out'))):
    t = os.path.basename(f)[:-4]
    nf = os.path.join(n, t + '.out')
    if not os.path.exists(nf): rows.append((t, 'no native reference', 0, 0)); continue
    a, b = lines(nf), lines(f)
    rcw = open(f[:-4] + '.rc').read().strip() if os.path.exists(f[:-4] + '.rc') else '?'
    rcn = open(nf[:-4] + '.rc').read().strip() if os.path.exists(nf[:-4] + '.rc') else '?'
    tot += 1; nl += len(a)
    ok = (a == b)
    if ok: same += 1
    else:
        first = next((i for i in range(min(len(a), len(b))) if a[i] != b[i]), min(len(a), len(b)))
        diff.append((t, len(a), len(b), first, a[first] if first < len(a) else '-', b[first] if first < len(b) else '-'))
    rows.append((t, 'IDENTICAL' if ok else 'DIFFERENT', len(a), len(b), rcn, rcw))
print(f'tests compared {tot}: identical {same}, different {len(diff)}; native reference lines in the compared tests {nl}')
for d in diff[:20]: print('  DIFF', d[0], f'native {d[1]} lines, wasm {d[2]} lines, first difference at {d[3]}:', '\n     native:', d[4][:150], '\n     wasm  :', d[5][:150])
if len(sys.argv) > 3:
    with open(sys.argv[3], 'w') as o:
        o.write('test\tresult\tnative_lines\twasm_lines\tnative_rc\twasm_rc\n')
        for r in rows: o.write('\t'.join(map(str, r)) + '\n')
