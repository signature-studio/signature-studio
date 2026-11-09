#!/usr/bin/env python3
"""opt1 (0265): the alternating write-rate A/B. For each run directory runs/p3-<tag>-<i>/ (audio-b.json of sdriveg2's audioDump, report.json with the Worker CPU of cpuStart/cpuStop over the 60 s of playback, load.txt = 'start end' load averages):
 - the engine's write rate per 10 s window = frames the guest wrote with DirectSound Unlock / (10 s * 44100) from the play mark (the `unlock` column of p3-rate.py; the `pulse` column is saturated by the page's silence fill and is not used)
 - mean of the windows 10-60 s (the first 10 s are the burst of the pre-filled buffer) and of the whole 60 s
 - Worker CPU over the 60 s (s), and Worker CPU per audio second written, and per Unlock
usage: ab3-summary.py A_TAG B_TAG ROUNDS [MAXLOAD=12]   (A, B = tags of the runs, e.g. base e5; run dirs runs/p3-<tag>-<i>)"""
import json, sys, bisect, math, os
A, B, R = sys.argv[1], sys.argv[2], int(sys.argv[3]); MAXL = float(sys.argv[4]) if len(sys.argv) > 4 else 12.0
SR = 44100.0
def load_run(tag, i):
    d = f'runs/p3-{tag}-{i}'
    a = json.load(open(f'{d}/audio-b.json')); t0 = a['marks']['play']
    U = sorted(a['unlocks'], key=lambda u: u['t'])
    wins = []
    for k in range(6):
        lo = t0 + k * 10000; hi = lo + 10000
        wins.append(sum(u['n'] for u in U if lo <= u['t'] < hi) / (10 * SR))
    frames = sum(u['n'] for u in U if t0 <= u['t'] < t0 + 60000); nun = sum(1 for u in U if t0 <= u['t'] < t0 + 60000)
    cpu = None
    try:
        r = json.load(open(f'{d}/report.json'))[0]; p = (r.get('profs') or [None])[0]
        if p: cpu = p['cpuS']['worker']
    except Exception: pass
    ld = open(f'{d}/load.txt').read().split() if os.path.exists(f'{d}/load.txt') else ['nan', 'nan']
    return dict(tag=tag, i=i, wins=wins, m1060=sum(wins[1:]) / 5, m60=sum(wins) / 6, cpu=cpu, frames=frames, unlocks=nun, load0=float(ld[0]), load1=float(ld[1]),
                cpu_per_audio_s=(cpu / (frames / SR)) if cpu and frames else None, cpu_per_unlock_ms=(1000 * cpu / nun) if cpu and nun else None)
runs = {A: [load_run(A, i) for i in range(1, R + 1)], B: [load_run(B, i) for i in range(1, R + 1)]}
def ms(xs):
    xs = [x for x in xs if x is not None]
    if not xs: return (float('nan'), float('nan'), 0)
    m = sum(xs) / len(xs); se = math.sqrt(sum((x - m) ** 2 for x in xs) / (len(xs) - 1) / len(xs)) if len(xs) > 1 else float('nan')
    return (m, se, len(xs))
print('run table (write rate = unlock frames / real-time frames, windows of 10 s from the play mark)')
print(f'{"run":10s} {"load start/end":>15s}  ' + ' '.join(f'{k*10:>3d}-{k*10+10:<3d}' for k in range(6)) + '  mean10-60  mean60  WorkerCPU(s)  CPU/audio-s  CPU/Unlock(ms)  Unlocks')
for t in (A, B):
    for r in runs[t]:
        print(f'{t}-{r["i"]:<7d} {r["load0"]:6.1f}/{r["load1"]:<6.1f}  ' + ' '.join(f'{w:7.2f}' for w in r['wins']) + f'  {r["m1060"]:9.3f} {r["m60"]:7.3f}  ' + (f'{r["cpu"]:11.2f}  {r["cpu_per_audio_s"]:10.2f}  {r["cpu_per_unlock_ms"]:13.2f}' if r['cpu'] else '          -           -              -') + f'  {r["unlocks"]:6d}')
def summarize(sel, label):
    print(f'\n{label}')
    for t in (A, B):
        rs = [r for r in runs[t] if sel(r)]
        for key, nm in (('m1060', 'mean 10-60 s'), ('m60', 'mean 60 s'), ('cpu', 'Worker CPU s'), ('cpu_per_audio_s', 'CPU per audio s'), ('cpu_per_unlock_ms', 'CPU per Unlock ms')):
            m, se, n = ms([r[key] for r in rs]); print(f'  {t:6s} {nm:18s} {m:8.3f} +- {se:6.3f} (n={n})')
    d = []
    for i in range(R):
        a, b = runs[A][i], runs[B][i]
        if sel(a) and sel(b): d.append((b['m1060'] - a['m1060'], b['m60'] - a['m60']))
    if d:
        for j, nm in ((0, 'mean 10-60 s'), (1, 'mean 60 s')):
            m, se, n = ms([x[j] for x in d]); print(f'  paired {B}-{A} {nm:14s} {m:+8.3f} +- {se:6.3f} (n={n} pairs)')
    else: print('  no pair in this set')
summarize(lambda r: True, 'all runs')
summarize(lambda r: max(r['load0'], r['load1']) <= MAXL, f'runs with the load at start and end <= {MAXL:g}')
