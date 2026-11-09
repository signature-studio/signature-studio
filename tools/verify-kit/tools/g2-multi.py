#!/usr/bin/env python3
"""g2-multi.py LABEL=RUNDIR ... (ci1, 2026-11-08, PM 06:33): the playback File click and key-down T (first answered frame - tPage) and the queue-insertion (tInject - tPage, tInject - tEvent) of any number of lat runs side by side,
the same record selection as g2-analyze.py (records created after the previous dump). The first LABEL is the reference: the table gives each other run minus the reference (median of T). Output markdown."""
import sys, json, os, statistics
ORDER = ['a-idle-menu', 'c-mixer-drag', 'd-space', 'b-play-menu']
def recs(path):
    out = {}; prev = -1
    for sec in ORDER:
        d = json.load(open(os.path.join(path, f'lat-{sec}.json'))); rr = d['records']
        out[sec] = [r for r in rr if (r.get('seq') or 0) > prev]
        if rr: prev = max(prev, max(r.get('seq') or 0 for r in rr))
    return out
def pct(v, p):
    v = sorted(v); return v[min(len(v) - 1, max(0, -(-p * len(v) // 100) - 1))] if v else None
def T(rr, kind):
    return [min(r['frames'].values()) - r['tPage'] for r in rr if r['kind'] == kind and r.get('frames') and not r.get('dropped')]
def Q(rr, kind):
    return [r['tInject'] - r['tPage'] for r in rr if r['kind'] == kind and r.get('tInject') is not None and not r.get('dropped')]
runs = [a.split('=', 1) for a in sys.argv[1:]]
R = {l: recs(p) for l, p in runs}
ref = runs[0][0]
print('| section / input | metric | ' + ' | '.join(l for l, _ in runs) + ' | ' + ' | '.join(f'{l} - {ref}' for l, _ in runs[1:]) + ' |')
print('|---|---|' + '---|' * (len(runs) + len(runs) - 1))
for sec, kind, nm in (('a-idle-menu', 2, 'idle File click'), ('a-idle-menu', 5, 'idle key down'), ('b-play-menu', 2, 'PLAYBACK File click'), ('b-play-menu', 5, 'PLAYBACK key down')):
    for met, f in (('T median (p90) n', T), ('page->inject median (p90)', Q)):
        cells = []; meds = []
        for l, _ in runs:
            v = f(R[l][sec], kind)
            cells.append(f'{statistics.median(v):.2f} ({pct(v, 90):.2f}) n={len(v)}' if v else 'n=0'); meds.append(statistics.median(v) if v else None)
        diffs = [f'{m - meds[0]:+.2f}' if (m is not None and meds[0] is not None) else '-' for m in meds[1:]]
        print(f'| {nm} | {met} | ' + ' | '.join(cells) + ' | ' + ' | '.join(diffs) + ' |')
