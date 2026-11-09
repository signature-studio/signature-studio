#!/usr/bin/env python3
"""pw1 (decision 0186): which plug-in answered last in the late blocks (A1)? usage: attribute.py RUN.json [...]  (raw files results/<name>.json)"""
import json, sys, collections
P = 4096 / 44100 * 1000
for f in sys.argv[1:]:
    d = json.load(open(f)); r = d['result']; E = r['engine']; B = len(E['allDone']); K = len(r['plugins'])
    c = collections.Counter(); n = 0
    for b in range(B):
        if E['allDone'][b] - (b + 1) * P > 0:
            n += 1
            ends = [r['plugins'][k]['T'][3][b] for k in range(K)]
            c[r['plugins'][k := ends.index(max(ends))]['label']] += 1
    print('%s: late %d; last to answer: %s' % (d['row']['name'], n, ', '.join('%s %d' % kv for kv in c.most_common(5))))
