#!/usr/bin/env python3
"""pw1 (decision 0186): A CALCULATION (計算), not a measurement. Does it help to release the plug-ins at different phases of the period (and not to couple them in one lock step)?
Same demands as simulate_node.py (node CPU time per block; each instance with its own random phase in its 650-block series), c ideal cores with processor sharing, event driven.
Variants (per block b, a block is late when ANY plug-in answers after its own deadline = its release schedule + one period):
  coupled   : the engine of engine-worker.js: all plug-ins released together, the next block waits for all answers (lock step)
  decoupled : every plug-in has its own closed loop (own ring), all scheduled at b*P
  staggered : decoupled, plug-in k scheduled at b*P + k*P/K
usage: simulate_stagger.py DEMAND.json s0,s1,s3,s4,s5,s3 ott:12   (prints late blocks of 2000, mean of 5 phase draws, for c = 3..8)"""
import json, random, sys
P = 4096 / 44100 * 1000
D = json.load(open(sys.argv[1])); D = {k: v[10:] for k, v in D.items()}
names = []
for tok in sys.argv[2:]:
    for t in tok.split(','):
        if t.startswith('ott:'): names += ['ott'] * int(t[4:])
        elif t: names.append(t)
import os
N = int(os.environ.get('PW1_N', 2000)); R = int(os.environ.get('PW1_R', 5)); rng = random.Random(7)

def run(dem, c, mode):
    K = len(dem)
    off = [k * P / K if mode == 'staggered' else 0.0 for k in range(K)]
    nb = [0] * K; ready = list(off); active = {}   # k -> [b, remaining, release]
    t = 0.0; late_blocks = set(); finished = [0] * K
    if mode == 'coupled':
        # lock step: block b of all plug-ins is released at max(b*P, completion of block b-1 of all)
        prev = 0.0; late = 0
        for b in range(N):
            rel = max(b * P, prev)
            rem = sorted(dem[k][b] for k in range(K)); n = K; tt = 0.0; done = 0.0
            for i, d in enumerate(rem):
                act = n - i; rate = min(1.0, c / act); dt = (d - done) / rate; tt += dt; done = d
            prev = rel + tt
            late += prev > (b + 1) * P
        return late
    while True:
        # release
        for k in range(K):
            if k not in active and nb[k] < N and ready[k] <= t + 1e-12:
                active[k] = [nb[k], dem[k][nb[k]], t]
        if not active:
            nxt = [ready[k] for k in range(K) if nb[k] < N]
            if not nxt: break
            t = min(nxt); continue
        n = len(active); rate = min(1.0, c / n)
        t_done = t + min(v[1] for v in active.values()) / rate
        t_rel = min([max(ready[k], t) for k in range(K) if k not in active and nb[k] < N] + [1e18])
        t2 = min(t_done, t_rel)
        dt = t2 - t
        for v in active.values(): v[1] -= dt * rate
        t = t2
        for k in [k for k, v in active.items() if v[1] <= 1e-9]:
            b = active[k][0]
            if t > b * P + off[k] + P: late_blocks.add(b)
            del active[k]; nb[k] = b + 1; ready[k] = max((b + 1) * P + off[k], t if False else 0.0)
    return len(late_blocks)

demand = lambda: [(lambda s, ph: [s[(ph + i) % len(s)] for i in range(N)])(D[n], rng.randrange(len(D[n]))) for n in names]
print('# plug-ins:', ' '.join(names), '  mean demand %.2f cores' % (sum(sum(D[n]) / len(D[n]) for n in names) / P))
print('cores_c\tcoupled(lock step)\tdecoupled\tstaggered   (late blocks of %d, mean of %d draws)' % (N, R))
for c in (3, 3.5, 4, 5, 6, 8):
    res = {m: 0.0 for m in ('coupled', 'decoupled', 'staggered')}
    for _ in range(R):
        dem = demand()
        for m in res: res[m] += run(dem, c, m) / R
    print('%s\t%.1f\t%.1f\t%.1f' % (c, res['coupled'], res['decoupled'], res['staggered']))
