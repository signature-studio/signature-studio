#!/usr/bin/env python3
"""pw1 (decision 0186): adds ownOverrun / ownMax (blocks whose own release->answer time exceeded the period, independent of the backlog) to the rows of older runs from their raw JSON.
usage: backfill.py RESULTS_DIR rows.jsonl [...]   (rewrites the rows files in place)"""
import json, sys, os
P = 4096 / 44100 * 1000
rd = sys.argv[1]
for f in sys.argv[2:]:
    out = []
    for l in open(f):
        r = json.loads(l)
        if 'ownOverrun' not in r and 'serum' in r:
            raw = os.path.join(rd, r['name'] + '.json')
            if os.path.exists(raw):
                E = json.load(open(raw))['result']['engine']
                own = [E['allDone'][b] - E['rel'][b] for b in range(len(E['rel'])) if E['allDone'][b] is not None and E['rel'][b] is not None]
                r['ownOverrun'] = sum(1 for x in own if x > P); r['ownMax'] = round(max(own), 3)
        out.append(json.dumps(r))
    open(f, 'w').write('\n'.join(out) + '\n')
