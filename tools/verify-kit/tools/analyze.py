#!/usr/bin/env python3
"""analyze.py OUTAB: the A/B runs of ab-run.sh -> one table (medians and all runs) for the drag and picker measures"""
import json, glob, os, statistics as st, collections, sys
d = sys.argv[1]
rows = collections.defaultdict(list)
for f in sorted(glob.glob(d + '/*/report.json')):
    n = os.path.basename(os.path.dirname(f)); var, rest = n.split('-', 1); sc, i = rest.rsplit('-', 1)
    r = json.load(open(f))[0]
    for m in r.get('measures') or []:
        rows[(sc, m['name'], var)].append((m.get('workerMsPerMove'), m.get('presentsPerS', m.get('presentsPerUnit')), m.get('movesPerS'), m.get('workerMsPerUnit', None), r.get('loadStart'), m.get('wholeFrames'), m.get('workerBusyFrac')))
print('scenario        measure         variant n  workerMs/move  presents/s  moves/s  workerMs/unit  load0  runs(workerMs/move or ms/unit)')
for (sc, name, var), v in sorted(rows.items()):
    def med(k): xs = [x[k] for x in v if x[k] is not None]; return round(st.median(xs), 2) if xs else '-'
    runs = [x[0] if x[0] is not None else x[3] for x in v]
    print(f'{sc:15s} {name:15s} {var:7s} {len(v):2d} {str(med(0)):>13} {str(med(1)):>11} {str(med(2)):>8} {str(med(3)):>14} {str(med(4)):>6}  {runs}')
