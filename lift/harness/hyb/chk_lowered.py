#!/usr/bin/env python3
"""hyb1: the functions that the strip step takes out of the C units (lowered-<m>.txt) must be the ones fe-wasmgen really emits (status ok in the report of the same options). usage: chk_lowered.py LIST REPORT.tsv"""
import sys
want = set(int(l.split()[0], 16) for l in open(sys.argv[1]) if l.strip())
ok = set(); why = {}
for l in open(sys.argv[2]):
    p = l.rstrip('\n').split('\t')
    if p[0] == 'va': continue
    if p[7] == 'ok': ok.add(int(p[0], 16))
    else: why[int(p[0], 16)] = p[7]
missing = sorted(want - ok)
print('listed %d, emitted ok %d, listed but not emitted %d, emitted but not listed %d' % (len(want), len(want & ok), len(missing), len(ok - want)))
if missing:
    with open(sys.argv[2] + '.missing', 'w') as f:
        for v in missing: f.write('0x%x\t%s\n' % (v, why.get(v, 'not in the report of fe-wasmgen')))
    for v in missing[:8]: print('  not emitted: 0x%x: %s' % (v, why.get(v, 'not in the report of fe-wasmgen')))
    if any('atomic read-modify-write' in why.get(v, '') for v in missing):
        print('  an "atomic read-modify-write" status: the function has a LOCK-prefixed or implicitly locked instruction and must not be in the hybrid list (decisions 0246, 0243): add its VA to seeds/mt4-atomic-functions*.tsv (lift/harness/hyb/list-atomic-functions.py) and run lift/harness/hyb/filter-hybrid-list.py')
    sys.exit(1)
