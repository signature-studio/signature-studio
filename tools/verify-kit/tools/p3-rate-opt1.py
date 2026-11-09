#!/usr/bin/env python3
"""gpu3 (0251, P3): the engine's write rate = frames the guest's mixer wrote into the audio ring per frame of real time (mt3's definition: unlock_bytes / t_ms / 176,400 B/s = 1.0), per WIN seconds from the 'play' mark, from audio-b.json (audioDump of sdriveg2: the page's 2 Hz engine pulses and the Unlock times). Two readings: (pulse) delta of `written` (ring write_frames) over real time between the pulses nearest the window edges; (unlock) sum of the frames of the Unlocks whose time is in the window. usage: p3-rate.py audio-b.json [WIN=10] [LEN=45]"""
import json, sys, bisect
f = sys.argv[1]; WIN = float(sys.argv[2]) if len(sys.argv) > 2 else 10.0; LEN = float(sys.argv[3]) if len(sys.argv) > 3 else 45.0
d = json.load(open(f)); t0 = d['marks']['play']; R = 44100.0
P = sorted(d['pulses'], key=lambda p: p['epoch']); ep = [p['epoch'] for p in P]
def at(t, key):
    i = bisect.bisect_left(ep, t)
    if i <= 0: return P[0][key] if P else 0
    if i >= len(P): return P[-1][key]
    a, b = P[i - 1], P[i]; w = (t - a['epoch']) / max(1e-9, b['epoch'] - a['epoch'])
    return a[key] + w * (b[key] - a[key])
U = sorted(d['unlocks'], key=lambda u: u['t'])
rows = []; k = 0
while (k + 1) * WIN * 1000 <= LEN * 1000 + 1:
    a = t0 + k * WIN * 1000; b = a + WIN * 1000
    wr = (at(b, 'written') - at(a, 'written')) / (WIN * R)
    cr = (at(b, 'consumed') - at(a, 'consumed')) / (WIN * R)
    un = sum(u['n'] for u in U if a <= u['t'] < b) / (WIN * R)
    rows.append((k, wr, un, cr)); k += 1
print(f'{f}: window {WIN:.0f} s from the play mark; pulses {len(P)}, unlocks {len(U)}; detail {d["detail"]}')
print('  win    write-rate(pulse written)  write-rate(unlocks)  worklet-read-rate')
for k, wr, un, cr in rows: print(f'  {k * WIN:3.0f}-{(k + 1) * WIN:3.0f}s   {wr:5.2f}   {un:5.2f}   {cr:5.2f}')
tot = (at(t0 + LEN * 1000, 'written') - at(t0, 'written')) / (LEN * R)
print(f'  whole {LEN:.0f} s: write rate {tot:.2f} (pulse); mean of windows {sum(r[1] for r in rows) / max(1, len(rows)):.2f}')
