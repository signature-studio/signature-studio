#!/usr/bin/env python3
"""pick1: per-operation API-call table from an api.log (apiw_core.c or gdb-api.py: "#dump N", "A NAME COUNT", "P ...", "E ...") and the stdout of the same run (g3-cpu lines give the operation names in order).
usage: mk-api-table.py RUNDIR [--cumulative] [--top N] [--names a,b,c] > table.tsv
--cumulative: the A counts are running totals (gdb-api.py); without it they are the calls since the previous dump (apiw_core.c)."""
import sys, re, collections
d = sys.argv[1]; cum = '--cumulative' in sys.argv
top = int(sys.argv[sys.argv.index('--top') + 1]) if '--top' in sys.argv else 12
want = sys.argv[sys.argv.index('--names') + 1].split(',') if '--names' in sys.argv else None
ops = []
for ln in open(d + '/stdout.txt', errors='replace'):
    m = re.match(r'g3-cpu: (\S+) user_ms (\d+) sys_ms (\d+) clock_ms (\d+)', ln)
    if m: ops.append((m.group(1), int(m.group(2)) + int(m.group(3)), int(m.group(4))))
blocks = []; cur = None
for ln in open(d + '/api.log', errors='replace'):
    if ln.startswith('#dump'): cur = collections.Counter(); blocks.append(cur)
    elif ln.startswith('A ') and cur is not None:
        t = ln.split(); cur[t[1]] = int(t[2])
prev = collections.Counter(); rows = []
for k, b in enumerate(blocks):
    if cum:
        delta = collections.Counter({n: v - prev.get(n, 0) for n, v in b.items() if v != prev.get(n, 0)})
        for n, v in b.items(): prev[n] = v
    else:
        delta = b
    rows.append(delta)
print('op\tcpu_ms_cum\tclock_ms\tapi_calls\tdistinct\ttop')
for k, (nm, cpu, clk) in enumerate(ops):
    if k >= len(rows): break
    r = rows[k]; tot = sum(r.values())
    if want: tops = ' '.join('%s=%d' % (n, r.get(n, 0)) for n in want)
    else: tops = ' '.join('%s=%d' % (n, v) for n, v in r.most_common(top))
    print('%s\t%d\t%d\t%d\t%d\t%s' % (nm, cpu, clk, tot, len(r), tops))
