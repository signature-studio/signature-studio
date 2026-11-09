#!/usr/bin/env python3
"""pw1 (decision 0186): A CALCULATION (計算), not a measurement. The per-block process times that K plug-in Workers needed in a measured run of run-pw1.mjs on the 16-vCPU machine (taken as CPU demand)
are replayed on c ideal, otherwise idle cores with processor sharing (n runnable threads on c cores each run at min(1, c/n)); the engine is closed loop exactly as in engine-worker.js (a block is released at
max(b*P, completion of block b-1)). Output: late counts A1 / A2 and the worst lateness for c cores. It shows what the scheduler alone does with the measured demands (no SMT slow-down, no foreign load, no
wake-up cost), so it is a lower bound of the lateness on c dedicated cores.
usage: simulate.py RUN.json [RUN2.json ...]   (the raw files results/<name>.json; every plug-in of the run, Serum and OTT, is replayed)"""
import json, sys

P = 4096 / 44100 * 1000

def demands(res):
    out = []
    for p in res['plugins']:
        T = p['T']
        out.append([T[3][b] - T[2][b] for b in range(len(T[3]))])
    return out

def block_done(rem, c):
    """processor sharing: time until all tasks of `rem` (remaining demands, ms) finish on c cores"""
    r = sorted(rem)
    n = len(r); t = 0.0; done = 0.0
    for i, d in enumerate(r):
        active = n - i
        rate = min(1.0, c / active)
        dt = (d - done) / rate
        t += dt; done = d
    return t

def simulate(dem, c):
    B = len(dem[0]); K = len(dem)
    prev = 0.0; late1 = late2 = 0; mx = -1e9; rel = []
    for b in range(B):
        R = max(b * P, prev)
        d = block_done([dem[k][b] for k in range(K)], c)
        prev = R + d
        l = prev - (b + 1) * P
        late1 += l > 0; late2 += prev - (b + 2) * P > 0; mx = max(mx, l)
    return late1, late2, mx

for f in sys.argv[1:]:
    d = json.load(open(f)); res = d['result']; row = d['row']
    dem = demands(res)
    tot = sum(sum(x) / len(x) for x in dem) / P
    print('# %s: serum=%s ott=%d hc=%s, measured late A1 %d, mean demand %.2f cores, load %s->%s' % (row['name'], row['serum'] or '-', row['ott'], row['hcReported'], row['lateA1'], tot, row['load_in'].split()[0], row['load_out'].split()[0]))
    print('cores_c\tlateA1\tlateA2\tmax_lateness_ms')
    for c in (1, 1.5, 2, 2.5, 3, 4, 6, 8, 16):
        l1, l2, mx = simulate(dem, c)
        print('%s\t%d\t%d\t%.1f' % (c, l1, l2, mx))
