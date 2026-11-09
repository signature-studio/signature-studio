#!/usr/bin/env python3
"""o19c-table.py DIR... (ci1, 2026-11-07, PM 14:1x C): the song played to its end (up to 300 s) once; per 10 s window after the mark 'play': underrun quanta, lag cuts, engine gap frames, present/s, non-silent quanta (snaps carry nonSilentQuanta only in runs made with sdriveg2-lat-A.mjs), and from the worklet's 50 ms posts ('worklet' in audio-p3.json, A): the longest run of silent posts and the number of silent runs >= 100 ms.
Per run summary: windows with underrun > 0 / windows, the number of silent runs >= 100 ms (A). A post is 'silent' when its nonSilent count is 0 and the output is playing (state 2) after the first sound.
Output: a table with one row per 10 s window and one column per run (underrun quanta), then the summary rows."""
import sys, os, json
SR = 44100.0
def load(d):
    a = json.load(open(d + '/audio-p3.json')); r = json.load(open(d + '/report.json'))[0]
    t0 = a['marks']['play']; S = sorted(a['snaps'], key=lambda s: s['t']); return a, r, t0, S
def at(S, t, key):
    # the value of a snap counter at time t: the last snap at or before t
    v = None
    for s in S:
        if s['t'] <= t: v = s.get(key)
        else: break
    return v
def one(d):
    a, r, t0, S = load(d); end = S[-1]['t']; n = int((end - t0) // 10000)
    rows = []
    for k in range(n):
        lo, hi = t0 + 10000 * k, t0 + 10000 * (k + 1)
        g = lambda key: (at(S, hi, key) or 0) - (at(S, lo, key) or 0)
        rows.append(dict(w=k, ur=g('underrunQuanta'), lag=g('lagCuts'), gap=g('engineGapFrames'), pres=g('presents') / 10.0, ns=(g('nonSilentQuanta') if 'nonSilentQuanta' in S[0] else None), qwp=g('quantaWhilePlaying')))
    sil = None
    W = a.get('worklet')
    if W:
        ts = W['t']; ns = W['nonSilent']; st = W['state']
        runs = []; cur = 0; started = False; start_t = None
        for i in range(len(ts)):
            if ns[i] > 0: started = True
            silent = started and ns[i] == 0 and st[i] == 2 and (ts[i] + W['epoch0']) >= t0
            if silent:
                if cur == 0: start_t = ts[i]
                cur += 1
            else:
                if cur: runs.append((cur, start_t)); cur = 0
        if cur: runs.append((cur, start_t))
        sil = dict(longest_posts=max([c for c, _ in runs], default=0), runs_ge100ms=sum(1 for c, _ in runs if c * 50 >= 100), n_runs=len(runs), posts=len(ts))
    prof = (r.get('profs') or [{}])[0]; cpu = prof.get('cpuS', {})
    return dict(dir=os.path.basename(d.rstrip('/')), load=f"{r.get('loadStart')}/{r.get('loadEnd')}", rows=rows, sil=sil, cpu=cpu.get('worker'), wall=prof.get('wallS'), n=len(cpu.get('workers') or []), ok=bool(r.get('ok')) and not r.get('stopped'))
R = [one(d) for d in sys.argv[1:]]
print('| window (s after play) | ' + ' | '.join(x['dir'] for x in R) + ' |'); print('|' + '---|' * (len(R) + 1))
m = max(len(x['rows']) for x in R)
for k in range(m): print(f'| {10 * k}-{10 * k + 10} | ' + ' | '.join(str(x['rows'][k]['ur']) if k < len(x['rows']) else '-' for x in R) + ' |')
print('\n| summary | ' + ' | '.join(x['dir'] for x in R) + ' |'); print('|' + '---|' * (len(R) + 1))
print('| load start/end | ' + ' | '.join(x['load'] for x in R) + ' |')
print('| windows with underrun > 0 / windows | ' + ' | '.join(f"{sum(1 for w in x['rows'] if w['ur'] > 0)}/{len(x['rows'])}" for x in R) + ' |')
print('| underrun quanta total | ' + ' | '.join(str(sum(w['ur'] for w in x['rows'])) for x in R) + ' |')
print('| lag cuts total | ' + ' | '.join(str(sum(w['lag'] for w in x['rows'])) for x in R) + ' |')
print('| engine gap frames total | ' + ' | '.join(str(sum(w['gap'] for w in x['rows'])) for x in R) + ' |')
print('| present/s (mean of windows) | ' + ' | '.join(f"{sum(w['pres'] for w in x['rows']) / max(1, len(x['rows'])):.1f}" for x in R) + ' |')
print('| silent runs >= 100 ms (A) / longest (posts of 50 ms) | ' + ' | '.join((f"{x['sil']['runs_ge100ms']} / {x['sil']['longest_posts']}" if x['sil'] else 'n/a (run without A)') for x in R) + ' |')
print('| Worker CPU s (cores) / N | ' + ' | '.join(f"{x['cpu']} ({(x['cpu'] or 0) / (x['wall'] or 1):.2f}) / {x['n']}" for x in R) + ' |')
