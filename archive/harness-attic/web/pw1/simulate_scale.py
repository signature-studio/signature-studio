#!/usr/bin/env python3
"""pw1 (decision 0186): A CALCULATION (計算). simulate.py's replay with the process time of the plug-ins whose label contains LABEL multiplied by each scale (what a faster state-5 Serum would do).
usage: simulate_scale.py LABEL RUN.json [RUN2.json ...]   e.g.  simulate_scale.py ':5' results/s6-hc16-r1.json"""
import json, sys
src = open(__file__.replace('simulate_scale.py', 'simulate.py')).read().split('for f in sys.argv[1:]:')[0]
ns = {}; exec(src, ns)
label = sys.argv[1]
print('run\tload\tscale\t' + '\t'.join('c=%s(A1/A2)' % c for c in (2, 3, 4, 6, 16)))
for f in sys.argv[2:]:
    d = json.load(open(f)); res = d['result']; row = d['row']
    dem = ns['demands'](res)
    idx = [i for i, p in enumerate(res['plugins']) if label in p['label']]
    for sc in (1.0, 0.9, 0.8, 0.7, 0.6):
        dd = [([v * sc for v in x] if i in idx else x) for i, x in enumerate(dem)]
        print('%s\t%s\t%.1f\t' % (row['name'], row['load_in'].split()[0], sc) + '\t'.join('%d/%d' % ns['simulate'](dd, c)[:2] for c in (2, 3, 4, 6, 16)))
