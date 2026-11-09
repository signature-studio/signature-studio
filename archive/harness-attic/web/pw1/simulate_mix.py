#!/usr/bin/env python3
"""pw1 (decision 0186): A CALCULATION (計算), not a measurement. simulate.py's replay on c ideal cores, with the demands composed from several measured runs: SPEC = RUN.json[:label-substring[:count]] ...
e.g.  simulate_mix.py results/s6-hc16-r1.json results/ott14-hc16-r0.json::12
The per-block process times of the selected plug-ins of each run (every plug-in whose label contains the substring, at most `count`) are taken as independent CPU demands (block b of one run is used with
block b of the others, so the sequences are not aligned in time, only in the statistics). Columns: ideal cores, late A1 / A2 of 650 blocks, worst lateness. Lower bound for c dedicated cores (no SMT slow-down, foreign load, wake-up cost, engine thread)."""
import json, sys
src = open(__file__.replace('simulate_mix.py', 'simulate.py')).read().split('for f in sys.argv[1:]:')[0]
ns = {}; exec(src, ns)
dem = []; names = []
for spec in sys.argv[1:]:
    parts = spec.split(':')
    f = parts[0]; sub = parts[1] if len(parts) > 1 else ''; cnt = int(parts[2]) if len(parts) > 2 and parts[2] else 10 ** 6
    d = json.load(open(f)); res = d['result']; dd = ns['demands'](res)
    k = 0
    for p, x in zip(res['plugins'], dd):
        if sub in p['label'] and k < cnt:
            dem.append(x[:650]); names.append(p['label']); k += 1
B = min(len(x) for x in dem)
dem = [x[:B] for x in dem]
print('# plug-ins:', len(dem), ' mean total demand %.2f cores' % (sum(sum(x) / B for x in dem) / ns['P']))
print('cores_c\tlateA1\tlateA2\tmax_lateness_ms')
for c in (2, 2.5, 3, 3.5, 4, 5, 6, 8, 16):
    l1, l2, mx = ns['simulate'](dem, c)
    print('%s\t%d\t%d\t%.1f' % (c, l1, l2, mx))
