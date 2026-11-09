#!/usr/bin/env python3
# summ-ab.py DIR: the Firefox A/B (old df0512b0 / new ff5ee429) -> medians and all runs of the Mixer drag (Worker ms per move), the picker (steady ms/s, fades) and the F9 band, with the load at the start of every run
import json, glob, os, re, statistics as st, collections, subprocess, sys
d = sys.argv[1]
rows = collections.defaultdict(list)
for f in sorted(glob.glob(d + '/*/report.json')):
    n = os.path.basename(os.path.dirname(f)); var, rest = n.split('-', 1); sc, i = rest.rsplit('-', 1)
    r = json.load(open(f))[0]
    for m in r.get('measures') or []:
        v = m.get('workerMsPerMove') if m.get('workerMsPerMove') is not None else m.get('workerMsPerUnit')
        if v is None: continue
        rows[(sc, m['name'], var)].append((v, r.get('loadStart')))
print('| scenario | measure | bundle | n | median | runs (load at start) |\n|---|---|---|---|---|---|')
for (sc, name, var), v in sorted(rows.items()):
    print(f"| {sc} | {name} | {var} | {len(v)} | {st.median(x[0] for x in v):.2f} | " + ', '.join(f"{x[0]:.2f} ({x[1]})" for x in v) + ' |')
print()
for var in ('old', 'new'):
    for f in sorted(glob.glob(f'{d}/{var}-f9-band-*/report.json')):
        dd = os.path.dirname(f)
        o = subprocess.run(['python3', '../c5/tools3/band.py', dd], capture_output=True, text=True).stdout.splitlines()
        print(var, os.path.basename(dd), o[1][:140] if len(o) > 1 else 'no pictures')
