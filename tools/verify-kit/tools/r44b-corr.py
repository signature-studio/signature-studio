#!/usr/bin/env python3
# r44b-corr.py: reads the six P3 runs of R4-3 (out/p3-chrome-{1,2,3} = condition 1, out/p3-mixmove-chrome-{1,2,3} = condition 2) and prints, for each 10 s window over 0-300 s from the play mark, the differences of the page counters (underrun quanta, engine gap frames, lagCuts, presents/s, idleCount/s) and the number of mouse moves scheduled in scn-p3-mixmove-300.json. Window and interpolation formulas are those of p3-recompute.py. Prints Spearman rank correlations (n = 30 windows) of the underrun quanta against the others. Standard library only; reads only; writes nothing.
import bisect
import json
import os
import statistics

NW = 30
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, '..', 'out')
SCN = os.path.join(HERE, 'scn-p3-mixmove-300.json')
RUNS = [('p3-chrome-1', 1), ('p3-chrome-2', 1), ('p3-chrome-3', 1),
        ('p3-mixmove-chrome-1', 2), ('p3-mixmove-chrome-2', 2), ('p3-mixmove-chrome-3', 2)]
IND = ['gap', 'lag', 'pre', 'idl']
IND_NAME = {'gap': 'engine gap frames', 'lag': 'lagCuts', 'pre': 'presents/s', 'idl': 'idle/s'}


def interp(xs, ys, t):
    i = bisect.bisect_left(xs, t)
    if i <= 0:
        return ys[0]
    if i >= len(xs):
        return ys[-1]
    w = (t - xs[i - 1]) / max(1e-9, xs[i] - xs[i - 1])
    return ys[i - 1] + w * (ys[i] - ys[i - 1])


def schedule():
    with open(SCN) as f:
        ops = json.load(f)[0]['ops']
    start = next(i for i, o in enumerate(ops) if o.get('op') == 'audioMark' and o.get('name') == 'play')
    t = 0.0
    moves = []
    snaps = []
    for o in ops[start + 1:]:
        if o['op'] == 'cpuStop':
            break
        if o['op'] == 'wait':
            t += o['ms']
        elif o['op'] == 'move':
            moves.append(t)
        elif o['op'] == 'audioSnap':
            snaps.append((o['name'], t))
    return moves, snaps


def load(name):
    with open(os.path.join(OUT, name, 'audio-p3.json')) as f:
        a = json.load(f)
    t0 = a['marks']['play']
    S = sorted(a['snaps'], key=lambda s: s['t'])
    ts = [s['t'] for s in S]

    def snap(t, key):
        return interp(ts, [(s.get(key) or 0) for s in S], t)

    rows = []
    for k in range(NW):
        lo = t0 + k * 10000
        hi = lo + 10000
        rows.append(dict(
            k=k,
            ur=snap(hi, 'underrunQuanta') - snap(lo, 'underrunQuanta'),
            gap=snap(hi, 'engineGapFrames') - snap(lo, 'engineGapFrames'),
            lag=snap(hi, 'lagCuts') - snap(lo, 'lagCuts'),
            pre=(snap(hi, 'presents') - snap(lo, 'presents')) / 10.0,
            idl=(snap(hi, 'idleCount') - snap(lo, 'idleCount')) / 10.0))
    end = t0 + NW * 10000
    total = dict(ur=snap(end, 'underrunQuanta') - snap(t0, 'underrunQuanta'),
                 gap=snap(end, 'engineGapFrames') - snap(t0, 'engineGapFrames'),
                 lag=snap(end, 'lagCuts') - snap(t0, 'lagCuts'))
    w150 = next((s for s in S if s.get('name') == 'w150'), None)
    keys = sorted({k for s in S for k in s})
    return dict(rows=rows, total=total, keys=keys,
                elapsed150=None if w150 is None else (w150['t'] - t0) / 1000.0)


def ranks(v):
    order = sorted(range(len(v)), key=lambda i: v[i])
    r = [0.0] * len(v)
    i = 0
    while i < len(order):
        j = i
        while j + 1 < len(order) and v[order[j + 1]] == v[order[i]]:
            j += 1
        avg = (i + j) / 2.0 + 1.0
        for m in range(i, j + 1):
            r[order[m]] = avg
        i = j + 1
    return r


def pearson(x, y):
    n = len(x)
    mx = sum(x) / n
    my = sum(y) / n
    sxy = sum((a - mx) * (b - my) for a, b in zip(x, y))
    sxx = sum((a - mx) ** 2 for a in x)
    syy = sum((b - my) ** 2 for b in y)
    if sxx == 0 or syy == 0:
        return None
    return sxy / (sxx * syy) ** 0.5


def spearman(x, y):
    return pearson(ranks(x), ranks(y))


def fmt(r):
    return '定義できず(分散 0)' if r is None else f'{r:.3f}'


def main():
    moves, snaps = schedule()
    mv = [sum(1 for t in moves if int(t // 10000) == k) for k in range(NW)]
    snap_pos = [(n, t) for n, t in snaps if n[1:].isdigit() and int(n[1:]) >= 1]

    data = {}
    for name, cond in RUNS:
        data[name] = dict(cond=cond, **load(name))

    print('# r44b-corr: R4-3 の既存の生データの再計算(実エンジンの実行なし。測定のやり直しなし)')
    print('# 窓: 再生開始(marks[play])から 10 s x 30 窓。式は p3-recompute.py と同じ(snaps の線形補間)。')
    print('# pre・idl は窓の差分 ÷ 10(1 秒あたり)。mv は台本の予定数(予定。実際に送れた数ではない)。')
    print()
    print('## 0. 台本の予定(scn-p3-mixmove-300.json、play 印の後の wait の累積)')
    print(f'予定 move 総数: {len(moves)}')
    print(f'予定 move の窓ごとの件数(k=0..29): min {min(mv)} / max {max(mv)}')
    print(f'最後の予定 move: {moves[-1] / 1000:.1f} s')
    print(f'予定の snap 時刻(wN): {len(snap_pos)} 件。w1 {snap_pos[0][1] / 1000:.1f} s / w150 {snap_pos[-1][1] / 1000:.1f} s(台本の wait の累積)')
    any_moves = any('moves' in data[n]['keys'] for n, _ in RUNS)
    print(f"audio-p3.json の snaps に moves の項目: {'有' if any_moves else '無'}(6 run の項目の union: {sorted(set(k for n, _ in RUNS for k in data[n]['keys']))})")
    print()

    print('## 1. 窓ごとの値(6 run x 30 窓)')
    for name, cond in RUNS:
        d = data[name]
        print(f'### {name} (条件 {cond})')
        print(f"snap w150 の実測時刻 - play 印 = {d['elapsed150']:.3f} s(台本の予定 300.0 s)")
        print('| 窓(s) | underrun quanta | engine gap frames | lagCuts | presents/s | idle/s | move 予定(件) |')
        print('|---|---|---|---|---|---|---|')
        for r in d['rows']:
            k = r['k']
            print(f"| {k * 10}-{k * 10 + 10} | {r['ur']:.3f} | {r['gap']:.3f} | {r['lag']:.3f} | {r['pre']:.3f} | {r['idl']:.3f} | {mv[k]} |")
        print()

    print('## 2. underrun quanta と他の指標の Spearman 順位相関(窓 n = 30、run ごと)')
    for name, cond in RUNS:
        rows = data[name]['rows']
        x = [r['ur'] for r in rows]
        parts = [f"{IND_NAME[i]} {fmt(spearman(x, [r[i] for r in rows]))}" for i in IND]
        parts.append(f"move 予定 {fmt(spearman(x, [float(m) for m in mv]))}")
        print(f'{name} (条件 {cond}): ' + ' / '.join(parts))
    print()

    print('## 3. 条件 2 の run ごとの underrun quanta 上位 5 窓と、同じ窓の条件 1 の中央値(3 run)')
    c1 = [n for n, c in RUNS if c == 1]
    med1 = {k: {i: statistics.median([data[n]['rows'][k][i] for n in c1]) for i in IND} for k in range(NW)}
    for name, cond in RUNS:
        if cond != 2:
            continue
        rows = data[name]['rows']
        top = sorted(rows, key=lambda r: (-r['ur'], r['k']))[:5]
        print(f'### {name} (条件 2)')
        print('| 順位 | 窓(s) | underrun quanta | engine gap frames | lagCuts | presents/s | idle/s | 条件1中央値 gap | 条件1中央値 lagCuts | 条件1中央値 presents/s | 条件1中央値 idle/s |')
        print('|---|---|---|---|---|---|---|---|---|---|---|')
        for n, r in enumerate(top, 1):
            k = r['k']
            m = med1[k]
            print(f"| {n} | {k * 10}-{k * 10 + 10} | {r['ur']:.3f} | {r['gap']:.3f} | {r['lag']:.3f} | {r['pre']:.3f} | {r['idl']:.3f} | {m['gap']:.3f} | {m['lag']:.3f} | {m['pre']:.3f} | {m['idl']:.3f} |")
        print()

    print('## 4. 条件 1(放置)の 3 run の窓ごとの中央値')
    print('| 窓(s) | underrun quanta | engine gap frames | lagCuts | presents/s | idle/s |')
    print('|---|---|---|---|---|---|')
    for k in range(NW):
        m = med1[k]
        ur = statistics.median([data[n]['rows'][k]['ur'] for n in c1])
        print(f"| {k * 10}-{k * 10 + 10} | {ur:.3f} | {m['gap']:.3f} | {m['lag']:.3f} | {m['pre']:.3f} | {m['idl']:.3f} |")
    print()

    print('## 5. 検算(窓の合計と、0-300 s の直接の差分)')
    for name, cond in RUNS:
        d = data[name]
        sums = {i: sum(r[i] for r in d['rows']) for i in ['ur', 'gap', 'lag']}
        print(f"{name} (条件 {cond}): ur 窓合計 {sums['ur']:.3f} / 直接 {d['total']['ur']:.3f} ・ gap 窓合計 {sums['gap']:.3f} / 直接 {d['total']['gap']:.3f} ・ lag 窓合計 {sums['lag']:.3f} / 直接 {d['total']['lag']:.3f}")
    print()

    print('## 注')
    print('注: 相関は因果ではない。n = 30 は各 run の中の窓。係数は run ごとに別で、3 run をまとめていない。')
    print('注: 条件 1 の underrun quanta はほぼ 0。分散 0 の指標は「定義できず」と出す。')
    print('注: move 予定は台本の予定で全窓同数のため、分散 0 で相関は定義できない。実際に送れた数は未測定。')
    print('注: 順位は値の小さい順に 1 から付け、同順位は平均順位。')


if __name__ == '__main__':
    main()
