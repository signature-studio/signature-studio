#!/usr/bin/env python3
"""parity1: the master table. usage: mk-master.py OUT.tsv LABEL=RUNDIR_PATTERN ...  where RUNDIR_PATTERN has {s} for the session (e.g. N1=runs/n-{s}-2 V1=runs/w-{s}-v1); a label starting with N is a native run, any other a wasm-in-node run.
Output columns per op: session, op, category, kind, virtual_ms, then for every label cpu_ms, cpu_per_vs, and surfaces digest."""
import sys, os, re
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parity_lib import parse, deltas
out = sys.argv[1]; specs = [a.split('=', 1) for a in sys.argv[2:]]
ops = {}; order = []
for ln in open(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'ops.tsv'), encoding='utf-8'):
    if ln.startswith('#') or not ln.strip(): continue
    f = ln.rstrip('\n').split('\t'); ops[(f[0], f[1])] = (f[2], f[5]); order.append((f[0], f[1]))
sessions = sorted({s for s, _ in order}, key=lambda s: ['boot', 'ui-pl', 'ui-cr', 'ui-mx', 'ui-mxd', 'ui-br', 'ui-menu', 'ui-f8', 'ui-pr', 'ui-win', 'ui-hang1'].index(s) if s in ['boot', 'ui-pl', 'ui-cr', 'ui-mx', 'ui-mxd', 'ui-br', 'ui-menu', 'ui-f8', 'ui-pr', 'ui-win', 'ui-hang1'] else 99)
data = {}
for lab, pat in specs:
    for s in sessions:
        for alt in pat.split('|'):
            p = alt.replace('{s}', s) + '/stdout.txt'
            if os.path.exists(p):
                d = deltas(parse(p, not lab.startswith('N')))
                if 'end' in d:   # the run reached its end (a run cut by the CPU limit has no `end` snapshot and is only used when nothing better exists)
                    data[(lab, s)] = d; break
                data.setdefault((lab, s), d)
hdr = ['session', 'op', 'category', 'kind', 'virtual_ms'] + [x for lab, _ in specs for x in (lab + '_cpu_ms', lab + '_per_vs', lab + '_surf')]
rows = []
pre = [('boot', 'wiz0'), ('ui', 'e03'), ('ui', 'opened')]
for s in sessions:
    names = ['wiz0', 'e03', 'opened'] if s != 'boot' else ['wiz0']
    seq = [(n, ('prelude', 'D')) for n in names] + [(op, ops[(ss, op)]) for ss, op in order if ss == s]
    for n, (cat, kind) in seq:
        if s != 'boot' and n in ('wiz0',) : continue
        row = [s, n, cat, kind, '']
        vm = None
        for lab, _ in specs:
            d = data.get((lab, s), {}).get(n)
            if d is None: row += ['-', '-', '-']; continue
            vm = d['virtual_ms'] if vm is None else vm
            row += [str(d['cpu_ms']), ('%.3f' % (d['cpu_ms'] / d['virtual_ms'])) if d['virtual_ms'] else '-', d['surf']]
        row[4] = str(vm) if vm is not None else '-'
        rows.append(row)
open(out, 'w').write('\t'.join(hdr) + '\n' + '\n'.join('\t'.join(r) for r in rows) + '\n')
print(len(rows), 'rows ->', out)
