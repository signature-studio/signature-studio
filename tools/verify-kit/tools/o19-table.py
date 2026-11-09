#!/usr/bin/env python3
"""o19-table.py DIR... (ci1, 2026-11-07): the table of the O19 runs out/o19-PROJ-COND-ROUND-VARIANT/ (scn-o19.json, 60 s window after the mark 'play'): from audio-p3.json + report.json only.
Per run: underrun quanta / episodes in the window (snap at 'play' -> the last snap w30), lag cuts, engine gap frames, present/s, nonSilentQuanta in the window is NOT available per window (the 'detail' totals are for the whole run), so the non-silent share is the whole run's (printed with its span), per-Worker CPU of the window (profs p3-play), the Unlock write rate of the window (the guest's write cursor), load at the start/end."""
import sys, os, json, bisect
SR = 44100.0
def one(d):
    a = json.load(open(d + '/audio-p3.json')); r = json.load(open(d + '/report.json'))[0]
    t0 = a['marks']['play']; S = sorted(a['snaps'], key=lambda s: s['t'])
    b = [s for s in S if s['t'] >= t0][0]; f = S[-1]; span = (f['t'] - b['t']) / 1000.0
    U = sorted(a['unlocks'], key=lambda u: u['t']); ts = [(u['t'] - t0) / 1000.0 for u in U]; cur = [u['first'] + u['n'] for u in U]
    def cursor(t):
        i = bisect.bisect_right(ts, t) - 1
        return cur[i] if i >= 0 else None
    c0, c1 = cursor(0.0), cursor(span)
    wr = (c1 - c0) / (span * SR) if c0 is not None and c1 is not None else None
    prof = (r.get('profs') or [{}])[0]; cpu = prof.get('cpuS', {})
    det = a.get('detail', {})
    return dict(dir=os.path.basename(d.rstrip('/')), ok=bool(r.get('ok')) and not r.get('stopped'), load=f"{r.get('loadStart')}/{r.get('loadEnd')}", span=span,
        ur=f['underrunQuanta'] - b['underrunQuanta'], ep=f['episodes'] - b['episodes'], longest=f.get('longestRunQuanta'), lag=f['lagCuts'] - b['lagCuts'], gap=f['engineGapFrames'] - b['engineGapFrames'],
        late=f.get('engineLateWrites', 0) - b.get('engineLateWrites', 0), qwp_w=f['quantaWhilePlaying'] - b['quantaWhilePlaying'], pres=(f['presents'] - b['presents']) / span, wr=wr, workers=cpu.get('workers'), cpu=cpu.get('worker'), wall=prof.get('wallS'),
        ns=det.get('nonSilentQuanta'), qwp=det.get('quantaWhilePlaying'), win=r.get('windowsEnd'))
rows = [one(d) for d in sys.argv[1:]]
print('| run | ok | load start/end | window s | underrun q / quanta played in the window (share) / episodes | lag cuts | engine gap frames | late writes | present/s | write rate (cursor) | N | Worker CPU s (cores) | non-silent share of the whole run |')
print('|' + '---|' * 13)
for x in rows:
    w = x['workers']; cores = (x['cpu'] or 0) / (x['wall'] or 1)
    print(f"| {x['dir']} | {x['ok']} | {x['load']} | {x['span']:.0f} | {x['ur']} / {x['qwp_w']} ({100.0 * x['ur'] / x['qwp_w']:.2f}%) / {x['ep']} | {x['lag']} | {x['gap']} | {x['late']} | {x['pres']:.1f} | {('%.3f' % x['wr']) if x['wr'] is not None else '-'} | {len(w) if w else '-'} | {x['cpu']} ({cores:.2f}) | {x['ns']}/{x['qwp']} = {x['ns'] / x['qwp']:.4f} |")
