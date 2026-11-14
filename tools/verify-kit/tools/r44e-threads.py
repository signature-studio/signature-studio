#!/usr/bin/env python3
# r44e-threads.py: reads audio-p3.json of the R4-4d runs (out/r44d-mix<N>-<M>, 150 s, 15 windows of 10 s) and of the R4-3 runs (out/p3-chrome-{1,2,3} = condition 1, out/p3-mixmove-chrome-{1,2,3} = condition 2, 300 s, 30 windows). pulses[].threads[i] = [a, b, c] is read as cumulative values: a = run time of thread i in ms (sl_total), b = the longest single run of thread i so far in ms (sl_max, a running maximum), c = number of runs of thread i (sl_n) (lift/runtime/fle_audio_transport.c:136-156). Prints per 10 s window from the play mark: engine gap frames (the same snaps interpolation as r44b-corr.py), the thread with the largest run time in the window, the thread with the largest run count, and whether b rose in the window (a new longest single run). Also prints per-run and per-condition summaries and Spearman rank correlations (per run, not pooled). Standard library only; reads only; writes nothing. No arguments, no environment variables.
import bisect
import collections
import glob
import json
import os
import re
import statistics

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, '..', 'out')
P3_RUNS = [('p3-chrome-1', 1), ('p3-chrome-2', 1), ('p3-chrome-3', 1),
           ('p3-mixmove-chrome-1', 2), ('p3-mixmove-chrome-2', 2), ('p3-mixmove-chrome-3', 2)]
# evidence/R4-4d.md §3.1: the 150 s totals of engine gap frames, as written there
R44D_GAP_DOC = {(0, 1): 0.0, (0, 2): 340.0, (5, 1): 724.0, (5, 2): 170.0,
                (10, 1): 30270.0, (10, 2): 36330.0, (20, 1): 141779.356, (20, 2): 169984.0}


def interp(xs, ys, t):
    i = bisect.bisect_left(xs, t)
    if i <= 0:
        return ys[0]
    if i >= len(xs):
        return ys[-1]
    w = (t - xs[i - 1]) / max(1e-9, xs[i] - xs[i - 1])
    return ys[i - 1] + w * (ys[i] - ys[i - 1])


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


def load(folder, nw):
    a = json.load(open(os.path.join(folder, 'audio-p3.json')))
    t0 = a['marks']['play']
    S = sorted(a['snaps'], key=lambda s: s['t'])
    ts = [s['t'] for s in S]
    P = sorted(a['pulses'], key=lambda p: p['epoch'])
    ep = [p['epoch'] for p in P]
    nth = min(len(p['threads']) for p in P)
    col = {(i, j): [p['threads'][i][j] for p in P] for i in range(nth) for j in range(3)}

    def snap(t, key):
        return interp(ts, [(s.get(key) or 0) for s in S], t)

    def th(i, j, t):
        return interp(ep, col[(i, j)], t)

    wins = []
    for k in range(nw):
        lo = t0 + k * 10000
        hi = lo + 10000
        ad = [th(i, 0, hi) - th(i, 0, lo) for i in range(nth)]
        cd = [th(i, 2, hi) - th(i, 2, lo) for i in range(nth)]
        rise = [th(i, 1, hi) - th(i, 1, lo) for i in range(nth)]
        upd = [i for i in range(nth) if rise[i] > 1e-9]
        wi = max(upd, key=lambda i: th(i, 1, hi)) if upd else None
        wins.append(dict(k=k,
                         gap=snap(hi, 'engineGapFrames') - snap(lo, 'engineGapFrames'),
                         ad=ad, cd=cd,
                         ia=max(range(nth), key=lambda i: ad[i]),
                         ic=max(range(nth), key=lambda i: cd[i]),
                         wmax=th(wi, 1, hi) if wi is not None else 0.0,
                         wmax_i=wi))
    end = t0 + nw * 10000
    fb = [col[(i, 1)][-1] for i in range(nth)]
    mi = max(range(nth), key=lambda i: fb[i])
    reach = next(p['epoch'] for p in P if p['threads'][mi][1] >= fb[mi] - 1e-9)
    return dict(wins=wins, nth=nth, pulses=len(P),
                last_pulse_s=(ep[-1] - t0) / 1000.0,
                final_a=[col[(i, 0)][-1] for i in range(nth)],
                final_b=fb, final_bi=mi, final_b_reach_s=(reach - t0) / 1000.0,
                gap_direct=snap(end, 'engineGapFrames') - snap(t0, 'engineGapFrames'))


def collect():
    runs = []
    for folder in sorted(glob.glob(os.path.join(OUT, 'r44d-mix*-*'))):
        m = re.fullmatch(r'r44d-mix(0|5|10|20)-(\d+)', os.path.basename(folder))
        if not m or not os.path.exists(os.path.join(folder, 'audio-p3.json')):
            continue
        runs.append(dict(group=('R4-4d', int(m.group(1))), name=os.path.basename(folder), nm=int(m.group(2)),
                         data=load(folder, 15)))
    for name, cond in P3_RUNS:
        runs.append(dict(group=('R4-3', cond), name=name, nm=None, data=load(os.path.join(OUT, name), 30)))
    return runs


def group_label(g):
    return f'R4-4d 頻度 {g[1]} 回/秒' if g[0] == 'R4-4d' else f'R4-3 条件 {g[1]}'


def top(counter, n=3):
    return ', '.join(f'{k}({v})' for k, v in counter.most_common(n)) or 'なし'


def main():
    runs = collect()
    print('# r44e-threads: pulses[].threads の窓ごとの並び(既存の生データの再計算。実エンジンの実行なし。測定のやり直しなし)')
    print('# threads[i] = [a, b, c] は累積値(lift/runtime/fle_audio_transport.c:136-156): a = スレッド i の実行 ms の累積(sl_total)、b = スレッド i の 1 回の実行 ms の最大の累積(sl_max。単調増加。起動からの最大)、c = スレッド i の実行回数の累積(sl_n)。')
    print('# 窓: 再生開始(marks[play])から 10 s。gap は r44b-corr.py と同じ(snaps の線形補間)。pulse の値は epoch に対する線形補間(pulse の間隔は 500 ms 以上。p3-chrome-1 の先頭 20 間隔は 500〜902 ms)。')
    print('# 「b 窓内で増えた」= その窓の中で、そのスレッドに新しい最長の 1 回が起きた(b が上がった)。増えない窓では、その窓の最長は前の最大以下で値は不明。')
    print()

    print('## 0. 検算(a の合計・pulse の時間範囲・gap の合計)')
    print('| 群 | 回 | pulse 数 | 最後の pulse(再生開始から s) | a の全スレッド合計(最後の pulse)ms | 窓の gap 合計 | 直接の gap |')
    print('|---|---|---|---|---|---|---|')
    for r in runs:
        d = r['data']
        print(f"| {group_label(r['group'])} | {r['name']} | {d['pulses']} | {d['last_pulse_s']:.1f} | {sum(d['final_a']):.0f} | {sum(w['gap'] for w in d['wins']):.3f} | {d['gap_direct']:.3f} |")
    print()

    print('## 1. 窓ごと(回ごと)')
    print('列: a 最大 = 窓の中の実行 ms が最も多いスレッド(スレッド番号: ms)。c 最大 = 窓の中の実行回数が最も多いスレッド(番号: 回)。b 新記録 = 窓の中で新しい最長の 1 回が起きたスレッド(番号: その ms。無ければ「なし」)。')
    for r in runs:
        d = r['data']
        print(f"### {group_label(r['group'])} / {r['name']}")
        print('| 窓(s) | gap | a 最大 | c 最大 | b 新記録 |')
        print('|---|---|---|---|---|')
        for w in d['wins']:
            bcell = 'なし' if w['wmax_i'] is None else f"{w['wmax_i']}: {w['wmax']:.2f}"
            print(f"| {w['k'] * 10}-{w['k'] * 10 + 10} | {w['gap']:.3f} | {w['ia']}: {w['ad'][w['ia']]:.2f} | {w['ic']}: {w['cd'][w['ic']]:.0f} | {bcell} |")
        print()

    print('## 2. 回ごとの要約')
    print('終端 b の最大は起動からの最大(再生開始より前に到達しうる)。到達時刻(再生開始から s)で、窓の中か前かを見る。')
    print('| 群 | 回 | 終端 b の最大(スレッド: ms) | その到達(再生開始から s) | b 新記録の窓の数 | b 新記録の最頻スレッド | gap と b 新記録の値(Spearman) | gap と a 最大の増分(Spearman) |')
    print('|---|---|---|---|---|---|---|---|')
    for r in runs:
        d = r['data']
        cnt = collections.Counter(w['wmax_i'] for w in d['wins'] if w['wmax_i'] is not None)
        gap = [w['gap'] for w in d['wins']]
        wmax = [w['wmax'] for w in d['wins']]
        amax = [max(w['ad']) for w in d['wins']]
        print(f"| {group_label(r['group'])} | {r['name']} | {d['final_bi']}: {d['final_b'][d['final_bi']]:.2f} | {d['final_b_reach_s']:.1f} | {sum(cnt.values())} | {top(cnt, 1)} | {fmt(spearman(gap, wmax))} | {fmt(spearman(gap, amax))} |")
    print()

    print('## 3. 条件ごと(回の値をまとめず、窓の数だけを数える)')
    groups = collections.OrderedDict()
    for r in runs:
        groups.setdefault(r['group'], []).append(r)
    order = [('R4-4d', 0), ('R4-4d', 5), ('R4-4d', 10), ('R4-4d', 20), ('R4-3', 1), ('R4-3', 2)]
    print('| 群 | 回数 | 窓の数 | a 最大スレッドの窓数(上位 3) | b 新記録の窓の数 | b 新記録のスレッド(上位 3) | 窓の b 新記録の値(中央値 ms。新記録なしは 0) | 窓の b 新記録の最大(ms) |')
    print('|---|---|---|---|---|---|---|---|')
    for g in order:
        if g not in groups:
            print(f'| {group_label(g)} | 0 | - | - | - | - | - | - |')
            continue
        rs = groups[g]
        wins = [w for r in rs for w in r['data']['wins']]
        ca = collections.Counter(w['ia'] for w in wins)
        cb = collections.Counter(w['wmax_i'] for w in wins if w['wmax_i'] is not None)
        wm = [w['wmax'] for w in wins]
        nb = sum(cb.values())
        print(f"| {group_label(g)} | {len(rs)} | {len(wins)} | {top(ca)} | {nb} | {top(cb)} | {statistics.median(wm):.2f} | {max(wm):.2f} |")
    print()

    print('## 4. 検算(gap の窓の合計と直接の差分。R4-4d は evidence/R4-4d.md §3.1 の値と比べる)')
    for r in runs:
        d = r['data']
        gap_sum = sum(w['gap'] for w in d['wins'])
        ok = 'un' if abs(gap_sum - d['gap_direct']) > 1e-6 else ''
        line = f"{group_label(r['group'])} {r['name']}: 窓の合計 {gap_sum:.3f} / 直接 {d['gap_direct']:.3f} / 窓の合計と直接 {'一致' if not ok else '不一致'}"
        if r['group'][0] == 'R4-4d':
            doc = R44D_GAP_DOC[(r['group'][1], r['nm'])]
            line += f" / R4-4d.md §3.1 {doc:.3f} / 直接と文書 {'一致' if abs(d['gap_direct'] - doc) < 1e-3 else '不一致'}"
        print(line)
    print()

    print('## 注')
    print('注: 相関は因果ではない。n = 窓の数(R4-4d は 15、R4-3 は 30)。回ごとに別々に出し、まとめていない。')
    print('注: a の窓の値は pulse の線形補間。窓の境界の誤差は最大 500 ms 程度。')
    print('注: スレッドの番号の意味は未確認(スレッドの役割を確かめていない)。')
    print('注: 移動の送信数・送信時刻は含まない(audio-p3.json に項目が無い)。')


if __name__ == '__main__':
    main()
