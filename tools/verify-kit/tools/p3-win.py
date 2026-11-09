#!/usr/bin/env python3
# p3-win.py BROWSER [MAXLOAD=8]: the P3 decider (decision 0277 item 3). Runs: out/p3-BROWSER-{prev,new}-N/ (audio-p3.json of audioDump with the audioSnap counters every 10 s, report.json with load / I/O pressure at the start and the end and the per-thread CPU).
# Windows of 10 s from the 'play' mark (opt1's definition, p3-rate.py / ab3-summary.py): write rate = frames the guest wrote with DirectSound Unlock / (10 s x 44100) ('unlock' column), consumed rate = frames the worklet read (pulses), and the page's audio counters (underrun quanta, episodes, the engine's own gap frames) as differences between the 10 s boundaries (linear interpolation between the snapshots).
# A run is COUNTED only if the load average at its start is <= MAXLOAD and the I/O pressure avg10 at its start is 0.
import json, sys, glob, os, bisect, itertools, statistics as st
B = sys.argv[1]; MAXL = float(sys.argv[2]) if len(sys.argv) > 2 else 8.0
NW = int(os.environ.get('P3WINS', '6')); PFX = os.environ.get('P3PREFIX', 'p3')
K = os.path.dirname(os.path.dirname(os.path.abspath(__file__))); O = K + '/out'; SR = 44100.0
def load(d):
    a = json.load(open(d + '/audio-p3.json')); r = json.load(open(d + '/report.json'))[0]
    t0 = a['marks']['play']; S = sorted(a.get('snaps') or [], key=lambda x: x['t'])
    P = sorted(a['pulses'], key=lambda p: p['epoch']); ep = [p['epoch'] for p in P]; U = sorted(a['unlocks'], key=lambda u: u['t'])
    def at(t, key):
        i = bisect.bisect_left(ep, t)
        if i <= 0: return P[0][key]
        if i >= len(P): return P[-1][key]
        x, y = P[i - 1], P[i]; w = (t - x['epoch']) / max(1e-9, y['epoch'] - x['epoch']); return x[key] + w * (y[key] - x[key])
    def snap(t, key):
        ts = [s['t'] for s in S]; i = bisect.bisect_left(ts, t)
        if i <= 0: return S[0][key] or 0
        if i >= len(S): return S[-1][key] or 0
        x, y = S[i - 1], S[i]; w = (t - x['t']) / max(1e-9, y['t'] - x['t']); return (x[key] or 0) + w * ((y[key] or 0) - (x[key] or 0))
    has = lambda k: bool(S) and all(s.get(k) is not None for s in S)
    wins = []
    for k in range(NW):
        lo = t0 + k * 10000; hi = lo + 10000
        wins.append(dict(
            ur=snap(hi, 'underrunQuanta') - snap(lo, 'underrunQuanta'), ep=snap(hi, 'episodes') - snap(lo, 'episodes'), gap=snap(hi, 'engineGapFrames') - snap(lo, 'engineGapFrames'),
            wr=sum(u['n'] for u in U if lo <= u['t'] < hi) / (10 * SR), cr=(at(hi, 'consumed') - at(lo, 'consumed')) / (10 * SR),
            idl=(snap(hi, 'idleCount') - snap(lo, 'idleCount')) / 10.0 if has('idleCount') else None, lag=(snap(hi, 'lagCuts') - snap(lo, 'lagCuts')) / 10.0 if has('lagCuts') else None,
            pre=(snap(hi, 'presents') - snap(lo, 'presents')) / 10.0 if has('presents') else None, vr=(at(hi, 'virt') - at(lo, 'virt')) / 10000.0 / max(1e-9, at(hi, 'real') - at(lo, 'real'))))
    # music-matched bins (gpu1's matched-music.py definition): the music position = the virtual clock since the play mark, in seconds; for every complete 10 s bin of music the wall interval it took, the distinct frames the guest wrote (Unlock coverage) per wall second, the Unlocks per wall second, and the page's counters over that wall interval
    V = [p_['virt'] for p_ in P]
    def vat(tt):
        i = bisect.bisect_left(ep, tt)
        if i <= 0: return V[0]
        if i >= len(P): return V[-1]
        w_ = (tt - ep[i - 1]) / max(1e-9, ep[i] - ep[i - 1]); return V[i - 1] + w_ * (V[i] - V[i - 1])
    v0 = vat(t0); grid = [t0 + k * 250 for k in range(0, NW * 40 + 1)]; mus = [(vat(g) - v0) / 1e7 for g in grid]
    def wall_at(m):
        i = bisect.bisect_left(mus, m)
        if i <= 0: return grid[0]
        if i >= len(mus): return None
        w_ = (m - mus[i - 1]) / max(1e-9, mus[i] - mus[i - 1]); return grid[i - 1] + w_ * (grid[i] - grid[i - 1])
    R_ = sorted(a.get('reports') or [], key=lambda x: x['at'])
    def gap_at(tt):
        if not R_: return None
        xs = [x['at'] for x in R_]; i = bisect.bisect_left(xs, tt)
        if i <= 0: return R_[0]['gapFrames']
        if i >= len(R_): return R_[-1]['gapFrames']
        w_ = (tt - xs[i - 1]) / max(1e-9, xs[i] - xs[i - 1]); return R_[i - 1]['gapFrames'] + w_ * (R_[i]['gapFrames'] - R_[i - 1]['gapFrames'])
    mbins = []
    for lo in range(0, 10 * NW * 2, 10):
        ta = wall_at(lo); tb = wall_at(lo + 10)
        if ta is None or tb is None: break
        wall = (tb - ta) / 1000.0
        Us = [u for u in U if ta <= u['t'] < tb]; iv = sorted((u['first'], u['first'] + u['n']) for u in Us); cov = 0; end_ = -1
        for l_, h_ in iv:
            if l_ > end_: cov += h_ - l_; end_ = h_
            elif h_ > end_: cov += h_ - end_; end_ = h_
        mbins.append(dict(lo=lo, wall=wall, frames=cov / SR / wall, unl=len(Us) / wall, ur=snap(tb, 'underrunQuanta') - snap(ta, 'underrunQuanta'), ep=snap(tb, 'episodes') - snap(ta, 'episodes'),
                          gap=(gap_at(tb) - gap_at(ta)) if R_ else None))
    prof = (r.get('profs') or [{}])[0].get('cpuS') or {}
    wt = ''
    try: wt = open(d + '/wait.txt').read().strip()
    except Exception: pass
    import re as _re
    mm = _re.search(r'after (\d+) s', wt)
    return dict(mbins=mbins, mus_end=mus[-1], d=d, wait=int(mm.group(1)) if mm else None, wins=wins, load0=r.get('loadStart'), load1=r.get('loadEnd'), io0=r.get('ioStart'), io1=r.get('ioEnd'), worker=prof.get('worker'), main=prof.get('main'), top=prof.get('top'),
                ok=bool(r.get('ok')), stopped=bool(r.get('stopped')), webgl=None)
CC = []
try:
    for l_ in open(O + '/chrome-count.tsv'):
        c_ = l_.split('\t')
        if len(c_) >= 4: CC.append((float(c_[0]), int(c_[1])))
except FileNotFoundError: pass
MAN = {}
try: MAN = json.load(open(O + '/chrome-count-manual.json'))
except Exception: pass
def others(name, t0):
    # the number of other Chrome browsers (not ci1's) at the play mark, at play + 60 s and the maximum between; manual entries (before the observer ran) win
    if name in MAN: return MAN[name]
    seg = [(t, n) for t, n in CC if t0 - 3000 <= t <= t0 + 63000]
    if not seg: return None
    a = min(seg, key=lambda x: abs(x[0] - t0))[1]; b = min(seg, key=lambda x: abs(x[0] - (t0 + 60000)))[1]
    return dict(start=a, end=b, max=max(n for _, n in seg))
REFR = {}
try: REFR = json.load(open(O + '/p3-reference.json'))
except Exception: pass
runs = {}
for var in ('prev', 'new'):
    runs[var] = []
    for d in sorted(glob.glob(f'{O}/{PFX}-{B}-{var}-*/'), key=lambda x: int(x.rstrip('/').rsplit('-', 1)[1])):
        d = d.rstrip('/')
        try: x = load(d); x['i'] = int(d.rsplit('-', 1)[1]); x['oc'] = others(os.path.basename(d), json.load(open(d + '/audio-p3.json'))['marks']['play']); x['overlap'] = bool(x['oc']) and x['oc']['max'] >= 2; x['quiet'] = x['load0'] is not None and x['load0'] <= MAXL and (x['io0'] or 0) == 0; x['ref'] = REFR.get(os.path.basename(d)); x['counted'] = x['quiet'] and not x['overlap'] and not x['ref']; runs[var].append(x)
        except Exception as e: print(f'(run {d} unusable: {e})')
def f(v, n=0): return '-' if v is None else f'{v:.{n}f}'
def m5(x, key, n=1):
    v = [w_[key] for w_ in x['wins'][1:] if w_.get(key) is not None]
    return f'{sum(v) / len(v):.{n}f}' if v else '-'
def med_(c, k, key, n=1):
    v = [x['wins'][k][key] for x in c if x['wins'][k].get(key) is not None]
    return f'{st.median(v):.{n}f}' if v else '-'
def tot(x, key, lo, hi): return sum(w[key] for w in x['wins'][lo:hi])
L = []
def w(s=''): L.append(s)
w(f'### P3 decider, {B}: old (previous) vs new, windows of 10 s from the play mark; counted = start load ≤{MAXL:g} and I/O pressure avg10 = 0')
w()
w('| run | bundle | counted | other Chrome browsers at the play mark / at +60 s / max | waited for the quiet window (s) | load start → end | I/O avg10 start → end | Worker CPU s (60 s) = CPU/wall x 60 | main CPU s | underrun quanta 0-60 s | episodes 0-60 s | engine gap frames 0-60 s | **underrun 30-60 s** | write rate mean 10-60 s | consumed rate mean 10-60 s | UI idles/s (10-60 s) | presents/s | lagCuts/s | virtual s per real s | stopped |')
w('|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|')
for var in ('prev', 'new'):
    for x in runs[var]:
        w(f"| {x['i']} | {var} | {'yes' if x['counted'] else ('reference (' + x['ref'] + ')' if x['ref'] else 'reference (overlap: ' + str(x['oc']['max']) + ' other Chrome)' if x['overlap'] else 'NO (not counted: load / I/O)')} | {(str(x['oc']['start']) + ' / ' + str(x['oc']['end']) + ' / ' + str(x['oc']['max'])) if x['oc'] else 'not observed'} | {f(x['wait'])} | {f(x['load0'],1)} → {f(x['load1'],1)} | {f(x['io0'],2)} → {f(x['io1'],2)} | {f(x['worker'],1)} | {f(x['main'],1)} | {tot(x,'ur',0,6):.0f} | {tot(x,'ep',0,6):.0f} | {tot(x,'gap',0,6):.0f} | **{tot(x,'ur',3,6):.0f}** | {sum(v['wr'] for v in x['wins'][1:]) / (NW - 1):.3f} | {sum(v['cr'] for v in x['wins'][1:]) / (NW - 1):.3f} | {m5(x,'idl')} | {m5(x,'pre')} | {m5(x,'lag')} | {m5(x,'vr',3)} | {x['stopped']} |")
w()
w('Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):')
w()
w('| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |')
w('|---|---|---|---|---|---|---|---|---|---|---|---|')
for k in range(NW):
    for var in ('prev', 'new'):
        c = [x for x in runs[var] if x['counted']]
        if not c: continue
        w(f"| {k*10}-{k*10+10} s | {var} | {len(c)} | {st.median(x['wins'][k]['ur'] for x in c):.0f} | {st.median(x['wins'][k]['ep'] for x in c):.0f} | {st.median(x['wins'][k]['gap'] for x in c):.0f} | {st.median(x['wins'][k]['wr'] for x in c):.2f} | {st.median(x['wins'][k]['cr'] for x in c):.2f} | {med_(c, k, 'idl')} | {med_(c, k, 'pre')} | {med_(c, k, 'lag')} | {med_(c, k, 'vr', 2)} |")
w()
def mw(a, b):  # exact two-sided permutation test of the difference of the medians' rank sum (Mann-Whitney U), small n
    if len(a) < 2 or len(b) < 2: return None
    allv = a + b; n = len(a); obs = abs(st.mean(a) - st.mean(b)); cnt = tot_ = 0
    for idx in itertools.combinations(range(len(allv)), n):
        s = set(idx); aa = [allv[i] for i in idx]; bb = [allv[i] for i in range(len(allv)) if i not in s]; tot_ += 1
        if abs(st.mean(aa) - st.mean(bb)) >= obs - 1e-12: cnt += 1
    return cnt / tot_
w('Music-matched windows (the same music in both bundles): the music position is the virtual clock since the play mark (gpu1 matched-music.py); the bins are complete 10 s bins of music that each run reached within its 60 s of wall time. wall = the wall time the bin took; distinct frames = Unlock coverage per wall second (1.0 = real time); Unlocks / wall s; underrun quanta and episodes of the page and the engine gap frames over the wall interval of the bin (the 10 s snapshots and the 1 Hz engine reports, linearly interpolated). Median over the counted runs.')
w()
w('| music bin | bundle | n | wall time of the bin (s) | distinct frames per wall s | Unlocks per wall s | underrun quanta | episodes | engine gap frames |')
w('|---|---|---|---|---|---|---|---|---|')
maxbin = {var: max([len(x['mbins']) for x in runs[var] if x['counted']] or [0]) for var in runs}
for k in range(max(maxbin.values()) if maxbin else 0):
    for var in ('prev', 'new'):
        c = [x for x in runs[var] if x['counted'] and len(x['mbins']) > k]
        if not c: continue
        md = lambda key: st.median(x['mbins'][k][key] for x in c if x['mbins'][k][key] is not None) if any(x['mbins'][k][key] is not None for x in c) else None
        w(f"| {k*10}-{k*10+10} s | {var} | {len(c)} | {md('wall'):.1f} | {md('frames'):.3f} | {md('unl'):.2f} | {md('ur'):.0f} | {md('ep'):.0f} | {f(md('gap'))} |")
w()
w('Old vs new at the same music position (counted runs; exact permutation test; only bins both bundles reached):')
w()
w('| music bin | measure | old median (n) | new median (n) | new − old | p |')
w('|---|---|---|---|---|---|')
for k in range(min(maxbin.values()) if maxbin else 0):
    for name, key in (('wall time of the bin (s)', 'wall'), ('distinct frames per wall s', 'frames'), ('underrun quanta', 'ur'), ('episodes', 'ep'), ('engine gap frames', 'gap')):
        a_ = [x['mbins'][k][key] for x in runs['prev'] if x['counted'] and len(x['mbins']) > k and x['mbins'][k][key] is not None]; b_ = [x['mbins'][k][key] for x in runs['new'] if x['counted'] and len(x['mbins']) > k and x['mbins'][k][key] is not None]
        if len(a_) < 2 or len(b_) < 2: continue
        w(f"| {k*10}-{k*10+10} s | {name} | {st.median(a_):.3f} ({len(a_)}) | {st.median(b_):.3f} ({len(b_)}) | {st.median(b_) - st.median(a_):+.3f} | {mw(a_, b_):.3f} |")
w()
w('Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):')
w()
w('| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |')
w('|---|---|---|---|---|---|---|---|---|')
for name, fn in (('underrun quanta 0-60 s', lambda x: tot(x, 'ur', 0, NW)), ('underrun quanta 30-60 s (heavy interval)', lambda x: tot(x, 'ur', NW // 2, NW)), ('episodes 0-60 s', lambda x: tot(x, 'ep', 0, NW)), ('engine gap frames 0-60 s', lambda x: tot(x, 'gap', 0, NW)),
                 ('write rate mean 10-60 s', lambda x: sum(v['wr'] for v in x['wins'][1:]) / (NW - 1)), ('Worker CPU s (60 s)', lambda x: x['worker']),
                 ('UI idles/s (10-60 s)', lambda x: (sum(v['idl'] for v in x['wins'][1:]) / (NW - 1)) if x['wins'][1].get('idl') is not None else None), ('presents/s (10-60 s)', lambda x: (sum(v['pre'] for v in x['wins'][1:]) / (NW - 1)) if x['wins'][1].get('pre') is not None else None),
                 ('lagCuts/s (10-60 s)', lambda x: (sum(v['lag'] for v in x['wins'][1:]) / (NW - 1)) if x['wins'][1].get('lag') is not None else None), ('virtual s per real s (10-60 s)', lambda x: sum(v['vr'] for v in x['wins'][1:]) / (NW - 1))):
    cells = []
    for only in (True, False):
        a = [fn(x) for x in runs['prev'] if (x['counted'] or not only) and fn(x) is not None]; b = [fn(x) for x in runs['new'] if (x['counted'] or not only) and fn(x) is not None]
        if not a or not b: cells += ['-', '-', '-', '-']; continue
        p = mw(a, b); cells += [f'{st.median(a):.3f} ({len(a)})', f'{st.median(b):.3f} ({len(b)})', f'{st.median(b) - st.median(a):+.3f}', f(p, 3)]
    w(f"| {name} | " + ' | '.join(cells) + ' |')
w()
open(f'{O}/{PFX.upper()}-{B}.md', 'w').write('\n'.join(L) + '\n'); print('\n'.join(L))
