#!/usr/bin/env python3
# p3-recompute.py: re-reads the six P3 runs of R4-3 (out/p3-chrome-{1,2,3} = condition 1, out/p3-mixmove-chrome-{1,2,3} = condition 2) from audio-p3.json and prints their values over 0-300 s from the play mark. Same formulas as p3-win.py (write rate = Unlock frames / (10 x 44100) per 10 s window, snapshots interpolated linearly) and s3a-summary.py (a window with no record counts as 0 in its mean). Reads only; writes nothing.
import bisect, json, os

SR = 44100.0
NW = 30
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'out')
RUNS = [('p3-chrome-1', 1), ('p3-chrome-2', 1), ('p3-chrome-3', 1),
        ('p3-mixmove-chrome-1', 2), ('p3-mixmove-chrome-2', 2), ('p3-mixmove-chrome-3', 2)]


def interp(xs, ys, t):
    i = bisect.bisect_left(xs, t)
    if i <= 0:
        return ys[0]
    if i >= len(xs):
        return ys[-1]
    w = (t - xs[i - 1]) / max(1e-9, xs[i] - xs[i - 1])
    return ys[i - 1] + w * (ys[i] - ys[i - 1])


def spans(ks):
    out = []
    for k in ks:
        if out and out[-1][1] == k - 1:
            out[-1][1] = k
        else:
            out.append([k, k])
    return ', '.join(f'{a * 10}-{b * 10 + 10} s' for a, b in out) or 'なし'


def compute(name):
    a = json.load(open(os.path.join(OUT, name, 'audio-p3.json')))
    t0 = a['marks']['play']
    S = sorted(a['snaps'], key=lambda s: s['t'])
    ts = [s['t'] for s in S]
    U = sorted(a['unlocks'], key=lambda u: u['t'])
    first = U[0]['t']

    def snap(t, key):
        return interp(ts, [(s.get(key) or 0) for s in S], t)

    wins = []
    for k in range(NW):
        lo = t0 + k * 10000
        hi = lo + 10000
        wins.append(dict(full=lo >= first, part=lo < first < hi,
                         wr=sum(u['n'] for u in U if lo <= u['t'] < hi) / (10 * SR),
                         ur=snap(hi, 'underrunQuanta') - snap(lo, 'underrunQuanta')))
    end = t0 + NW * 10000
    data = [k for k in range(NW) if wins[k]['full']]
    empty = [k for k in range(NW) if not wins[k]['full'] and not wins[k]['part']]
    part = [k for k in range(NW) if wins[k]['part']]
    low = [k for k in data if wins[k]['wr'] < 0.95]
    kmin = min(data, key=lambda k: wins[k]['wr'])
    z = min(range(NW), key=lambda k: wins[k]['wr'])

    def cnt(key):
        return snap(end, key) - snap(t0, key)

    P = sorted(a['pulses'], key=lambda p: p['epoch'])
    ep = [p['epoch'] for p in P]
    c0 = interp(ep, [p['consumed'] for p in P], t0)
    last = P[-1]['epoch']
    music = (interp(ep, [p['consumed'] for p in P], last) - c0) / SR
    music_snap = (interp(ep, [p['consumed'] for p in P], min(S[-1]['t'], last)) - c0) / SR
    longest = S[-1]['longestRunQuanta']
    at_play = snap(t0, 'longestRunQuanta')
    at_max = next(s['t'] for s in S if (s.get('longestRunQuanta') or 0) == longest)
    return dict(
        music=music, music_snap=music_snap, wall=(last - t0) / 1000.0, longest=longest, at_play=at_play, grew=longest > at_play, longest_at=(at_max - t0) / 1000.0,
        empty=empty, part=part, partv=[wins[k]['wr'] for k in part], sum=sum(wins[k]['wr'] for k in data), ndata=len(data), low=low,
        mean=sum(wins[k]['wr'] for k in data) / len(data), kmin=kmin, wmin=wins[kmin]['wr'],
        mean_s3a=sum(w['wr'] for w in wins) / NW, z=z, zmin=wins[z]['wr'],
        w0=wins[0], w9=wins[9],
        ur=cnt('underrunQuanta'), ur_all=S[-1]['underrunQuanta'] - S[0]['underrunQuanta'], last_s=(S[-1]['t'] - t0) / 1000.0,
        gap=cnt('engineGapFrames'), gap_all=S[-1]['engineGapFrames'] - S[0]['engineGapFrames'],
        lag=cnt('lagCuts'), idle=cnt('idleCount') / (NW * 10.0), pre=cnt('presents') / (NW * 10.0))


def show(name, cond, r):
    print(f'## {name} (条件 {cond})')
    print(f"窓 0-300 s(10 s x 30): 記録なし = {spans(r['empty'])} / 記録の途中で始まる窓(部分値・除外) = {spans(r['part'])} {', '.join(f'wr {v:.3f}' for v in r['partv']) or ''} / 記録が全部ある窓 {r['ndata']}")
    print(f"記録が全部ある窓の wr の合計 {r['sum']:.3f}(÷ 30 = {r['sum'] / NW:.3f})")
    print(f"書き込み率(記録が全部ある窓): 平均 {r['mean']:.3f} / 最小 {r['wmin']:.3f}({r['kmin'] * 10}-{r['kmin'] * 10 + 10} s) / 0.95 未満 {len(r['low'])} 窓: 開始 {', '.join(str(k * 10) for k in r['low']) or 'なし'} s")
    print(f"書き込み率(s3a-summary.py の式: 記録のない窓は 0、部分窓はその値のまま、30 窓で平均): 平均 {r['mean_s3a']:.3f} / 最小 {r['zmin']:.3f}({r['z'] * 10}-{r['z'] * 10 + 10} s)")
    print(f"窓 0-10 s: 書き込み率 {r['w0']['wr']:.3f} / underrun {r['w0']['ur']:.0f} ・ 窓 90-100 s: 書き込み率 {r['w9']['wr']:.3f} / underrun {r['w9']['ur']:.0f}")
    print(f"underrun quanta 0-300 s(補間): {r['ur']:.0f} / snaps の最初〜最後({r['last_s']:.1f} s): {r['ur_all']:.0f}")
    print(f"engine gap frames 0-300 s(補間): {r['gap']:.0f} / snaps の最初〜最後: {r['gap_all']:.0f}")
    print(f"lagCuts 0-300 s: {r['lag']:.0f} / UI idles/s: {r['idle']:.1f} / presents/s: {r['pre']:.2f}")
    print(f"音楽の秒(consumed/44100。最後の pulse まで): {r['music']:.1f} / 同(最後の snap まで): {r['music_snap']:.1f} / 壁時計の秒(最後の pulse まで): {r['wall']:.1f}")
    where = '再生前' if r['longest_at'] <= 0 else f"{r['longest_at']:.1f} s"
    print(f"longestRunQuanta: 再生開始時 {r['at_play']:.0f} / 最後 {r['longest']:.0f}(初めて到達: {where})/ 再生後に伸びた: {'はい' if r['grew'] else 'いいえ'}")


def main():
    print('# p3-recompute: R4-3 の 6 run(再生 0〜300 s。audio-p3.json を読むだけ)')
    for name, cond in RUNS:
        show(name, cond, compute(name))


if __name__ == '__main__':
    main()
