#!/usr/bin/env python3
# lat-ab.py BROWSER: the input latency T (page sent -> answer presented) of runs out/lat-BROWSER-{prev,new}-N/ (op latDump of sdriveg2-lat.mjs, scenario scn-lat-ff.json, gpu7's LatencyTracker records): per run and per bundle for the File click and Esc DURING PLAYBACK (section b-play-menu) and the File click at idle (a-idle-menu)
import sys, json, os, glob, collections, statistics as st
B = sys.argv[1]; K = os.path.dirname(os.path.dirname(os.path.abspath(__file__))); O = K + '/out'
ORDER = ['a-idle-menu', 'c-mixer-drag', 'd-space', 'b-play-menu']
def pct(v, p):
    if not v: return None
    v = sorted(v); i = min(len(v) - 1, max(0, int(-(-p * len(v) // 100)) - 1)); return v[i]
def fm(x): return '-' if x is None else f'{x:.1f}'
def load(d):
    last = {}; prev = -1
    for x in ORDER:
        p = f'{d}/lat-{x}.json'
        if os.path.exists(p):
            last[x] = prev; prev = max([r['seq'] for r in json.load(open(p))['records']] or [prev])
    out = {}
    for sec in ('a-idle-menu', 'b-play-menu'):
        p = f'{d}/lat-{sec}.json'
        if not os.path.exists(p): continue
        for r in json.load(open(p))['records']:
            if r.get('dropped') or r['seq'] <= last.get(sec, -1) or r['kind'] == 1: continue
            k = {2: 'File click (button down)', 5: 'Esc (key down)'}.get(r['kind'])
            if not k: continue
            fr = r['frames']
            out.setdefault((sec, k), [None, 0])
            out[(sec, k)][1] += 1
            if fr:
                lid = sorted(fr)[0]; out[(sec, k)][0] = (out[(sec, k)][0] or []) + [fr[lid] - r['tPage']]
    return out
import re
def waited(d):
    try: m = re.search(r'after (\d+) s', open(d + '/wait.txt').read()); return m.group(1) if m else '-'
    except Exception: return '-'
CC = []
try:
    for l_ in open(O + '/chrome-count.tsv'):
        c_ = l_.split('\t')
        if len(c_) >= 4: CC.append((float(c_[0]), int(c_[1])))
except FileNotFoundError: pass
def others(d, rep):
    # the other Chrome browsers (not ci1's) during the run: at the start, at the end and the maximum
    end = os.path.getmtime(d + '/report.json') * 1000; start = end - 1000 * float(rep.get('seconds') or 0)
    seg = [(t, n) for t, n in CC if start <= t <= end]
    if not seg: return None
    return dict(start=seg[0][1], end=seg[-1][1], max=max(n for _, n in seg))
runs = []
for var in ('prev', 'new'):
    for d in sorted(glob.glob(f'{O}/lat-{B}-{var}-*/'), key=lambda x: int(x.rstrip('/').rsplit('-', 1)[1])):
        d = d.rstrip('/'); i = int(d.rsplit('-', 1)[1])
        try: rep = json.load(open(d + '/report.json'))[0]; runs.append((var, i, d, rep, load(d)))
        except Exception as e: print(f'(run {d} unusable: {e})')
L = []
def w(s=''): L.append(s)
w(f'### Input latency T (ms, page sent -> answer presented), {B}; counted = start load <=8 and I/O pressure avg10 = 0'); w()
w('| run | bundle | counted | other Chrome at the start / end / max during the run | waited (s) | load start -> end | I/O avg10 start -> end | section | input | n sent | answered | T median / p90 / p99 / max |'); w('|---|---|---|---|---|---|---|---|---|---|---|---|')
agg = collections.defaultdict(list)
for var, i, d, rep, o in runs:
    oc = others(d, rep); ovl = bool(oc) and oc['max'] >= 2
    cnt = rep.get('loadStart') is not None and rep['loadStart'] <= 8 and (rep.get('ioStart') or 0) == 0 and not ovl
    for (sec, k), (v, n) in sorted(o.items()):
        v = v or []
        w(f"| {i} | {var} | {'yes' if cnt else ('reference (overlap: ' + str(oc['max']) + ' other Chrome)' if ovl else 'NO (load / I/O)')} | {(str(oc['start']) + ' / ' + str(oc['end']) + ' / ' + str(oc['max'])) if oc else 'not observed'} | {waited(d)} | {rep.get('loadStart')} -> {rep.get('loadEnd')} | {rep.get('ioStart')} -> {rep.get('ioEnd')} | {sec} | {k} | {n} | {len(v)} | {fm(pct(v, 50))} / {fm(pct(v, 90))} / {fm(pct(v, 99))} / {fm(max(v) if v else None)} |")
        if cnt: agg[(var, sec, k)] += v
w(); w('Per bundle (all answered records of the counted runs pooled):'); w()
w('| section | input | bundle | runs | answered | T median / p90 / p99 / max |'); w('|---|---|---|---|---|---|')
for sec, k in sorted({(s, k) for (_, s, k) in agg}):
    for var in ('prev', 'new'):
        v = agg.get((var, sec, k), []); nr = sum(1 for r in runs if r[0] == var and r[3].get('loadStart') is not None and r[3]['loadStart'] <= 8 and (r[3].get('ioStart') or 0) == 0 and not ((others(r[2], r[3]) or {'max': 0})['max'] >= 2))
        w(f"| {sec} | {k} | {var} | {nr} | {len(v)} | {fm(pct(v, 50))} / {fm(pct(v, 90))} / {fm(pct(v, 99))} / {fm(max(v) if v else None)} |")
open(f'{O}/LAT-{B}.md', 'w').write('\n'.join(L) + '\n'); print('\n'.join(L))
