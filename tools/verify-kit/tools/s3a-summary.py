#!/usr/bin/env python3
"""s3a-summary.py [OUTDIR=out] [--dirs D1 D2 ...]  (S3a sweep, ci1 0326): the table of the runs out/s3a-<round>-<condition>-<scenario>/ (audio-p3.json + report.json of tools/sdriveg2-lat.mjs with scn-s3a-*.json).
A run is COUNTED only if the load average at its start was <= 8, the I/O pressure avg10 at its start was 0 and the engine did not stop (report.json stopped / error). Every value comes from the run's own files; nothing is estimated.
Per run: write rate = frames the guest wrote with DirectSound Unlock per 10 s of wall time after the 'play' mark / 441000 (windows of 10 s, up to 30); consumed rate = the same for what the page's AudioWorklet consumed; underrun quanta (one quantum = 128 frames = 2.90 ms: the Web Audio render quantum, an ASSUMPTION);
episodes and the longest run of underrun quanta (the zero run of the PCM the listener hears); lag cuts of the virtual clock; the music position = frames consumed / 44100 at the end of the run and the wall time per 30 s of music (aligned by the song position, not by the wall clock);
Worker CPU = the CPU seconds of the DedicatedWorker threads while the 'p3-play' span ran, cores = that / the span's wall seconds, busy workers = the threads in the span's top list with CPU >= 10 % of the wall."""
import sys, os, json, bisect, glob, statistics as st
SR = 44100.0; Q = 128 / SR * 1000.0
args = sys.argv[1:]; dirs = []
if '--dirs' in args:
    i = args.index('--dirs'); dirs = args[i + 1:]; args = args[:i]
K = os.path.dirname(os.path.dirname(os.path.abspath(__file__))); O = args[0] if args else K + '/out'
if not dirs: dirs = sorted(glob.glob(O + '/s3a-r*-*-*'))
def one(d):
    try: a = json.load(open(d + '/audio-p3.json')); r = json.load(open(d + '/report.json'))[0]
    except Exception as e: return {'dir': os.path.basename(d), 'error': f'no data ({e.__class__.__name__})'}
    t0 = a['marks'].get('play'); S = sorted(a.get('snaps') or [], key=lambda x: x['t']); P = sorted(a['pulses'], key=lambda p: p['epoch']); U = sorted(a['unlocks'], key=lambda u: u['t'])
    if t0 is None or not S or not P: return {'dir': os.path.basename(d), 'error': 'no play mark / snaps / pulses'}
    ep = [p['epoch'] for p in P]; ts = [s['t'] for s in S]
    def interp(xs, ys, t):
        i = bisect.bisect_left(xs, t)
        if i <= 0: return ys[0]
        if i >= len(xs): return ys[-1]
        w = (t - xs[i - 1]) / max(1e-9, xs[i] - xs[i - 1]); return ys[i - 1] + w * (ys[i] - ys[i - 1])
    def pul(t, k): return interp(ep, [p[k] for p in P], t)
    def snap(t, k): return interp(ts, [(s.get(k) or 0) for s in S], t)
    span = min(300.0, (min(ts[-1], ep[-1]) - t0) / 1000.0); nw = max(0, int(span // 10))
    W = []
    for k in range(nw):
        lo = t0 + k * 10000; hi = lo + 10000
        W.append(dict(wr=sum(u['n'] for u in U if lo <= u['t'] < hi) / (10 * SR), cr=(pul(hi, 'consumed') - pul(lo, 'consumed')) / (10 * SR), ur=snap(hi, 'underrunQuanta') - snap(lo, 'underrunQuanta')))
    # the wall time at which the music reached 30, 60, ... s (by the frames consumed), and the real-time factor of each 30 s of music
    cons = [p['consumed'] for p in P]; c0 = pul(t0, 'consumed'); marks = []
    for m in range(1, 11):
        target = c0 + m * 30 * SR
        if cons[-1] < target: break
        i = bisect.bisect_left(cons, target); x, y = P[i - 1] if i > 0 else P[0], P[i]
        w = (target - x['consumed']) / max(1e-9, y['consumed'] - x['consumed']); marks.append((x['epoch'] + w * (y['epoch'] - x['epoch']) - t0) / 1000.0)
    rtf = [30.0 / (marks[i] - (marks[i - 1] if i else 0.0)) for i in range(len(marks))] if marks else []
    prof = next((p for p in (r.get('profs') or []) if p.get('name') == 'p3-play'), None); cpu = (prof or {}).get('cpuS') or {}; wall = (prof or {}).get('wallS') or 0
    wks = cpu.get('workers'); busy = sum(1 for v in wks if v >= 0.1 * wall) if (wks is not None and wall) else None
    end = ts[-1]
    ut = [u['t'] for u in U if t0 <= u['t'] <= t0 + span * 1000]; gaps = sorted(b - a for a, b in zip(ut, ut[1:]))
    gp99 = gaps[min(len(gaps) - 1, int(len(gaps) * 0.99))] if gaps else None; gmax = gaps[-1] if gaps else None
    out = dict(dir=os.path.basename(d), span=round(span, 1), load=r.get('loadStart'), io=r.get('ioStart'), stopped=r.get('stopped'), state=r.get('state'), error=r.get('error'), opErrors=r.get('opErrors'),
               wr_mean=st.mean(w['wr'] for w in W) if W else None, wr_min=min(w['wr'] for w in W) if W else None, wr_lt95=sum(1 for w in W if w['wr'] < 0.95), nw=nw,
               cr_mean=st.mean(w['cr'] for w in W) if W else None, ur=sum(w['ur'] for w in W), ep=snap(end, 'episodes') - snap(t0, 'episodes'), longest_q=(lambda fin, at0: fin if fin > at0 else None)(max((s.get('longestRunQuanta') or 0) for s in S), snap(t0, 'longestRunQuanta')), lag_cuts=snap(end, 'lagCuts') - snap(t0, 'lagCuts'),
               music_s=(pul(min(end, ep[-1]), 'consumed') - c0) / SR, music_per_wall=((pul(min(end, ep[-1]), 'consumed') - c0) / SR) / span if span else None, rtf=rtf, wall_per30=[round(marks[i] - (marks[i - 1] if i else 0.0), 2) for i in range(len(marks))],
               gap_p99=gp99, gap_max=gmax, worker_cpu=cpu.get('worker'), cpu_wall=wall, cores=(cpu.get('worker') / wall) if wall and cpu.get('worker') is not None else None, busy_workers=busy)
    out['counted'] = bool(out['load'] is not None and out['load'] <= 8 and out['io'] == 0 and not out['stopped'] and not out['error'])
    return out
rows = [one(d) for d in dirs]
f = lambda v, n=3: '-' if v is None else (f'{v:.{n}f}' if isinstance(v, float) else str(v))
print('| run | counted | span s | load/io at start | write rate mean (min) | windows < 0.95 | consumed rate | music s / wall s | underrun quanta | episodes | longest run ms (after play) | Unlock gap p99 / max ms | lag cuts | Worker CPU s | cores | Workers >= 10 % | stop / error |')
print('|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|')
for x in rows:
    if 'error' in x and 'span' not in x: print(f"| {x['dir']} | no | - | - | - | - | - | - | - | - | - | - | - | - | - | - | {x['error']} |"); continue
    print(f"| {x['dir']} | {'yes' if x['counted'] else 'no'} | {x['span']} | {f(x['load'],2)}/{f(x['io'],2)} | {f(x['wr_mean'])} ({f(x['wr_min'])}) | {x['wr_lt95']}/{x['nw']} | {f(x['cr_mean'])} | {f(x['music_s'],1)} / {x['span']} = {f(x['music_per_wall'])} | {f(x['ur'],0)} | {f(x['ep'],0)} | {f(None if x['longest_q'] is None else x['longest_q']*Q,0)} | {f(x['gap_p99'],0)} / {f(x['gap_max'],0)} | {f(x['lag_cuts'],0)} | {f(x['worker_cpu'],1)} | {f(x['cores'],2)} | {x['busy_workers']} | {x['stopped'] or x['error'] or x['state'] or ''} |")
# per condition and scenario: the counted runs
by = {}
for x in rows:
    if x.get('counted'):
        p = x['dir'].split('-'); by.setdefault((p[2], p[3]), []).append(x)
if by:
    print('\n| condition | scenario | counted runs | write rate (mean of means) | music s / wall s | underrun quanta (sum) | longest run ms (max) | cores (mean) |')
    print('|---|---|---|---|---|---|---|---|')
    for (c, s), xs in sorted(by.items()):
        print(f"| {c} | {s} | {len(xs)} | {f(st.mean(x['wr_mean'] for x in xs))} | {f(st.mean(x['music_per_wall'] for x in xs))} | {f(sum(x['ur'] for x in xs),0)} | {f(max((x['longest_q'] for x in xs if x['longest_q'] is not None), default=None) and max(x['longest_q'] for x in xs if x['longest_q'] is not None)*Q,0)} | {f(st.mean(x['cores'] for x in xs if x['cores'] is not None),2) if any(x['cores'] is not None for x in xs) else '-'} |")
