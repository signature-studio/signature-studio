#!/usr/bin/env python3
# p3-recompute-dirs.py RUN... : the values of p3-recompute.py (same formulas: 10 s windows from the 'play' mark over 0-300 s, snapshots interpolated linearly, a window's write rate counted only when the Unlock record covers the whole window) for any runs under out/ (a folder name or a path). Prints one row per run and the median / min / max per label (the label is the folder name before "-chrome-"). Reads audio-p3.json and report.json only; writes nothing. Exit 2 with no argument.
import bisect, json, os, re, statistics, sys

SR = 44100.0
NW = 30
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'out')


def interp(xs, ys, t):
    i = bisect.bisect_left(xs, t)
    if i <= 0:
        return ys[0]
    if i >= len(xs):
        return ys[-1]
    w = (t - xs[i - 1]) / max(1e-9, xs[i] - xs[i - 1])
    return ys[i - 1] + w * (ys[i] - ys[i - 1])


def label_of(name):
    m = re.match(r'^(.+)-chrome-\d+$', name)
    return m.group(1) if m else name


def run(name):
    d = os.path.join(OUT, name)
    a = json.load(open(os.path.join(d, 'audio-p3.json')))
    rep = json.load(open(os.path.join(d, 'report.json')))[0]
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
        wins.append(dict(full=lo >= first,
                         wr=sum(u['n'] for u in U if lo <= u['t'] < hi) / (10 * SR),
                         ur=snap(hi, 'underrunQuanta') - snap(lo, 'underrunQuanta')))
    end = t0 + NW * 10000
    data = [k for k in range(NW) if wins[k]['full']]
    low = [k for k in data if wins[k]['wr'] < 0.95]
    kmin = min(data, key=lambda k: wins[k]['wr'])

    def cnt(key):
        return snap(end, key) - snap(t0, key)

    prof = (rep.get('profs') or [{}])[0]
    cpu = (prof.get('cpuS') or {}).get('worker')
    wall = prof.get('wallS')
    return dict(
        name=name, label=label_of(name),
        load=rep.get('loadStart'), io=rep.get('ioStart'),
        first_s=(first - t0) / 1000.0, ndata=len(data),
        ur=cnt('underrunQuanta'), wmin=wins[kmin]['wr'], kmin=kmin * 10,
        low=len(low), pre=cnt('presents') / (NW * 10.0), lag=cnt('lagCuts'), gap=cnt('engineGapFrames'),
        cpu=cpu, wall=wall, cores=(cpu / wall) if cpu is not None and wall else None,
        snap_end=(S[-1]['t'] - t0) / 1000.0,
        bitblt=any('bitblt' in k.lower() for k in json.dumps(rep).split('"')))


def f(v, n=0):
    return '-' if v is None else f'{v:.{n}f}'


def main():
    names = [os.path.basename(os.path.normpath(x)) for x in sys.argv[1:]]
    if not names:
        sys.stderr.write('usage: python3 tools/verify-kit/tools/p3-recompute-dirs.py <run folder name or path> ...\n')
        sys.exit(2)
    rows = [run(n) for n in names]
    print('# p3-recompute-dirs: 0-300 s from the play mark (audio-p3.json and report.json only; the formulas of p3-recompute.py)')
    print('| run | label | loadStart | ioStart | 最初の書き込み(s) | 記録が全部ある窓 | underrun 0-300 s | 書き込み率 最小(窓の開始 s) | 0.95 未満の窓 | presents/s | lagCuts | engine gap frames | Worker CPU s | wallS | cores | snaps の最後(s) |')
    print('|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|')
    for r in rows:
        print(f"| {r['name']} | {r['label']} | {f(r['load'], 2)} | {f(r['io'], 2)} | {f(r['first_s'], 1)} | {r['ndata']} | {r['ur']:.0f} | {r['wmin']:.3f} ({r['kmin']}) | {r['low']} | {r['pre']:.2f} | {r['lag']:.0f} | {r['gap']:.0f} | {f(r['cpu'], 2)} | {f(r['wall'], 2)} | {f(r['cores'], 3)} | {r['snap_end']:.1f} |")
    short = [r['name'] for r in rows if r['snap_end'] < 300.0]
    print(f"snaps が 300 s に届かない run: {', '.join(short) or 'なし'}")
    print(f"BitBlt CPU: {'report.json に BitBlt の項目あり(検索結果を読むこと)' if any(r['bitblt'] for r in rows) else '取れない(未測定。report.json に BitBlt の項目が無い)'}")
    print()
    print('## label ごと(median (min - max))')
    print('| label | n | underrun 0-300 s | 書き込み率 最小 | 0.95 未満の窓 | presents/s | lagCuts | engine gap frames | Worker CPU s |')
    print('|---|---|---|---|---|---|---|---|---|')
    labels = []
    for r in rows:
        if r['label'] not in labels:
            labels.append(r['label'])
    for lb in labels:
        g = [r for r in rows if r['label'] == lb]

        def ms(key, n=0, scale=1.0):
            v = [r[key] * scale for r in g if r[key] is not None]
            if not v:
                return '-'
            med = statistics.median(v)
            return f'{med:.{n}f} ({min(v):.{n}f} - {max(v):.{n}f})'
        print(f"| {lb} | {len(g)} | {ms('ur')} | {ms('wmin', 3)} | {ms('low')} | {ms('pre', 2)} | {ms('lag')} | {ms('gap')} | {ms('cpu', 1)} |")


if __name__ == '__main__':
    main()
