#!/usr/bin/env python3
"""rta-p3-table.py DIR... (ci1, 2026-11-07): the P3 table of the runs out/s3a-rN-COND-song|solo12 for the cooperative E13 and the real-thread RT-A, from audio-p3.json + report.json only (no estimate). The audio-p3.json 'unlocks' list is the LAST 2400 Unlock events of the run, so the write rate (the advance of the guest's write cursor per 10 s window) is computed only over the span it covers
(printed). 'pulses' is empty in the real-thread builds, so the page-side consumed rate and the music position of s3a-summary.py cannot be made for them; the counters of the snaps (underrun, lag cuts, engine gap, presents) exist for all."""
import sys, os, json, statistics as st
SR = 44100.0
def one(d):
    a = json.load(open(d + '/audio-p3.json')); r = json.load(open(d + '/report.json'))[0]
    t0 = a['marks']['play']; S = sorted(a['snaps'], key=lambda s: s['t']); U = sorted(a['unlocks'], key=lambda u: u['t'])
    sp = [s for s in S if s['t'] >= t0][0]; fin = S[-1]
    cov0 = (U[0]['t'] - t0) / 1000.0; cov1 = (U[-1]['t'] - t0) / 1000.0
    import bisect
    ts = [(u['t'] - t0) / 1000.0 for u in U]; cur = [u['first'] + u['n'] for u in U]
    def cursor(t):   # the guest's DirectSound write cursor (frames) after the last Unlock at or before t seconds after play
        i = bisect.bisect_right(ts, t) - 1
        return cur[i] if i >= 0 else None
    W = []; k = int(-(-cov0 // 10)) * 10 if cov0 > 0 else 0
    while k + 10 <= cov1:
        c0, c1 = cursor(k), cursor(k + 10)
        if c0 is not None and c1 is not None: W.append((c1 - c0) / (10 * SR))
        k += 10
    gaps = [U[i + 1]['t'] - U[i]['t'] for i in range(len(U) - 1)]
    span = (U[-1]['t'] - U[0]['t']) / 1000.0; slope = (U[-1]['first'] + U[-1]['n'] - U[0]['first']) / SR / span
    pb = [s for s in S if s['t'] >= t0]; pres = (pb[-1]['presents'] - pb[0]['presents']) / ((pb[-1]['t'] - pb[0]['t']) / 1000.0)
    prof = (r.get('profs') or [{}])[0]; cpu = prof.get('cpuS', {})
    p99 = sorted(gaps)[int(len(gaps) * 0.99)] if gaps else None
    return dict(dir=os.path.basename(d), ok=bool(r.get('ok')), stopped=bool(r.get('stopped')), load=f"{r.get('loadStart')}/{r.get('loadEnd')}", firstidle=round(r.get('firstIdleMs') or 0),
        pre_ur=sp['underrunQuanta'], pre_ep=sp['episodes'], post_ur=fin['underrunQuanta'] - sp['underrunQuanta'], post_ep=fin['episodes'] - sp['episodes'], longest=fin.get('longestRunQuanta'),
        lag=fin['lagCuts'] - sp['lagCuts'], gap=fin['engineGapFrames'] - sp['engineGapFrames'], late=fin.get('engineLateWrites', 0) - sp.get('engineLateWrites', 0),
        cov=f'{cov0:.0f}-{cov1:.0f}', wr_mean=st.mean(W) if W else None, wr_min=min(W) if W else None, wr_low=f'{sum(1 for w in W if w < 0.95)}/{len(W)}', slope=slope, gap_p99=p99, gap_max=max(gaps) if gaps else None,
        pres=pres, workers=cpu.get('workers'), cpu=cpu.get('worker'), cores=(cpu.get('worker') or 0) / (prof.get('wallS') or 1))
rows = [one(d.rstrip('/')) for d in sys.argv[1:]]
print('| run | ok | load start/end | first idle ms | N (Workers) | CPU s (cores) | underrun pre-play (q/ep) | underrun after play (q/ep) | longest q | lag cuts after play | engine gap frames after play | Unlock span s | write rate mean (min) / <0.95 | write slope | Unlock gap p99/max ms | present/s |')
print('|' + '---|' * 16)
for x in rows:
    w = x['workers']
    print(f"| {x['dir']} | {x['ok'] and not x['stopped']} | {x['load']} | {x['firstidle']} | {len(w) if w else '-'} | {x['cpu']} ({x['cores']:.2f}) | {x['pre_ur']}/{x['pre_ep']} | {x['post_ur']}/{x['post_ep']} | {x['longest']} | {x['lag']} | {x['gap']} | {x['cov']} | {x['wr_mean']:.3f} ({x['wr_min']:.3f}) / {x['wr_low']} | {x['slope']:.3f} | {x['gap_p99']:.0f}/{x['gap_max']:.0f} | {x['pres']:.1f} |")
for x in rows: print(x['dir'], 'workers CPU s', x['workers'])
