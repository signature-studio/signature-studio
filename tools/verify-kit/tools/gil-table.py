#!/usr/bin/env python3
"""gil-table.py ENGINE_LOG (ci1, 2026-11-08): the FLE_DIAG_GIL lines of an engine log -> per Worker (W0..) the cumulative wait / hold histograms of the dump with the highest acquisition count, the totals, and the APIs of the top-held entries.
The log has no time stamps and the dumps are periodic, so the numbers are cumulative since the engine started: they are NOT restricted to a window (the phase C of the File clicks)."""
import re, sys, collections
L = [l.rstrip('\n') for l in open(sys.argv[1], errors='replace') if 'fl-diag: gil' in l]
sm = re.compile(r'gil W(\d+) acq (\d+) \(held sampled (\d+)\) wait n\[<0.1 <1 <5 <20 <50 >=50\]=(\d+),(\d+),(\d+),(\d+),(\d+),(\d+) max ([\d.]+) ms; held n=(\d+),(\d+),(\d+),(\d+),(\d+),(\d+) max ([\d.]+) ms mean ([\d.]+) ms')
th = re.compile(r'gil W(\d+) top-held #(\d+) ([\d.]+) ms by (.*)')
best = {}; pos = {}; ndump = collections.Counter()
for i, l in enumerate(L):
    m = sm.search(l)
    if m:
        w = int(m.group(1)); v = [float(x) if '.' in x else int(x) for x in m.groups()[1:]]; ndump[w] += 1
        if w not in best or v[0] >= best[w][0]: best[w] = v; pos[w] = i
print(f'GIL lines {len(L)}; dumps per Worker: ' + ', '.join(f'W{w} {n}' for w, n in sorted(ndump.items())))
print('\n| Worker | acquisitions | wait <0.1 | <1 | <5 | <20 | <50 | >=50 ms | longest wait ms | held (sampled) n | held <0.1 | <1 | <5 | <20 | <50 | >=50 ms | longest hold ms | mean hold ms |')
print('|' + '---|' * 18)
for w in sorted(best):
    v = best[w]
    print(f'| W{w} | {v[0]} | ' + ' | '.join(str(x) for x in v[2:8]) + f' | {v[8]} | {v[1]} | ' + ' | '.join(str(x) for x in v[9:15]) + f' | {v[15]} | {v[16]} |')
tot = sum(best[w][0] for w in best); wt = [sum(best[w][2 + k] for w in best) for k in range(6)]
print(f'\nall Workers: acquisitions {tot}; wait histogram {wt}; waits >= 0.1 ms: {sum(wt[1:])} = {100 * sum(wt[1:]) / max(1, sum(wt)):.3f}% of the acquisitions')
w0 = best.get(0)
if w0: print(f'W0: waits >= 0.1 ms {sum(w0[3:8])} of {w0[0]} acquisitions ({100 * sum(w0[3:8]) / w0[0]:.3f}%), longest single wait {w0[8]} ms')
print('\ntop-held APIs (entries over all dumps: count / longest ms), W0 and W1..W7:')
for name, sel in (('W0', lambda w: w == 0), ('W1..W7', lambda w: w > 0)):
    c = collections.Counter(); mx = collections.defaultdict(float)
    for l in L:
        m = th.search(l)
        if m and sel(int(m.group(1))): k = m.group(4); c[k] += 1; mx[k] = max(mx[k], float(m.group(3)))
    print(f'- {name}: ' + '; '.join(f'{k} {n} / {mx[k]:.3f}' for k, n in c.most_common(10)))
