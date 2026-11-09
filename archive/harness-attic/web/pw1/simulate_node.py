#!/usr/bin/env python3
"""pw1 (decision 0186): A CALCULATION (計算), not a measurement. The per-block CPU times that node measured for each plug-in (tools/node_demand.py: thread CPU clock, no contention, 650 blocks) are replayed
on c ideal cores (processor sharing) with the closed-loop engine of engine-worker.js. Each instance gets its own random phase in its 650-block series (the instances of the reference project are
not aligned in time). SPEC: s0,s1,s3,s4,s5,s3,... for Serum (s3c chord, s3s silent) and ott:M for M OTT. Output: late A1 / A2 of the blocks, worst lateness, for c = 2..16 (mean over 20 phase draws).
usage: simulate_node.py DEMAND.json s0,s1,s3,s4,s5,s3 ott:12"""
import json, random, sys
src = open(__file__.replace('simulate_node.py', 'simulate.py')).read().split('for f in sys.argv[1:]:')[0]
ns = {}; exec(src, ns)
D = json.load(open(sys.argv[1]))
D = {k: v[10:] for k, v in D.items()}   # the first 10 blocks of every node run are first-touch / JIT (block 0: 280-680 ms), not steady state
names = []
for tok in sys.argv[2:]:
    for t in tok.split(','):
        if t.startswith('ott:'): names += ['ott'] * int(t[4:])
        elif t: names.append(t)   # name or name*scale (e.g. s5*0.8: that plug-in's demand scaled)
for i, n in enumerate(names):
    if '*' in n:
        base, sc = n.split('*'); key = n
        D[key] = [v * float(sc) for v in D[base]]
B = 650; N = 2000   # simulate 2000 blocks by cycling the series
rng = random.Random(1)
print('# plug-ins:', ' '.join(names), '  mean demand %.2f cores' % (sum(sum(D[n]) / len(D[n]) for n in names) / ns['P']))
print('cores_c\tlateA1_of_%d\tlateA2\tmax_lateness_ms(max over draws)' % N)
for c in (2, 2.5, 3, 3.5, 4, 5, 6, 8, 16):
    a1 = a2 = 0; mx = -1e9; R = 20
    for _ in range(R):
        dem = []
        for n in names:
            s = D[n]; ph = rng.randrange(len(s)); dem.append([s[(ph + i) % len(s)] for i in range(N)])
        l1, l2, m = ns['simulate'](dem, c); a1 += l1; a2 += l2; mx = max(mx, m)
    print('%s\t%.1f\t%.1f\t%.1f' % (c, a1 / R, a2 / R, mx))
