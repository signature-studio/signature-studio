#!/usr/bin/env python3
# r44d-sum.py: reads the R4-4d runs (out/r44d-mix<N>-<M>/audio-p3.json and report.json; N = mouse moves per second 0, 5, 10, 20) and prints one table: per run the 150 s totals of underrun quanta, engine gap frames, lagCuts, presents/s and idle/s, the 15 ten-second window values of engine gap frames, the Worker CPU per second, and the report's state, stopped, ok, loadStart and ioStart. Then the order of the conditions by engine gap frames, and the Spearman rank correlation between moves per second and engine gap frames over all runs. Window and interpolation formulas are those of r44b-corr.py and p3-recompute.py. Standard library only; reads only; writes nothing. No arguments, no environment variables.
import bisect
import glob
import json
import os
import re

NW = 15
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, '..', 'out')
SCN_DIR = HERE
CONDS = [0, 5, 10, 20]


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


def scheduled_moves(n):
    path = os.path.join(SCN_DIR, f'scn-p3-mix{n}-150.json')
    if not os.path.exists(path):
        return None
    ops = json.load(open(path, encoding='utf-8'))[0]['ops']
    return sum(1 for o in ops if o.get('op') == 'move')


def run(folder, n):
    a = json.load(open(os.path.join(folder, 'audio-p3.json')))
    rep = json.load(open(os.path.join(folder, 'report.json')))[0]
    t0 = a['marks']['play']
    S = sorted(a['snaps'], key=lambda s: s['t'])
    ts = [s['t'] for s in S]

    def snap(t, key):
        return interp(ts, [(s.get(key) or 0) for s in S], t)

    def delta(key, lo, hi):
        return snap(hi, key) - snap(lo, key)

    wins_gap = [delta('engineGapFrames', t0 + k * 10000, t0 + (k + 1) * 10000) for k in range(NW)]
    end = t0 + NW * 10000
    prof = next(p for p in rep['profs'] if p['name'] == 'p3-play')
    return dict(
        n=n,
        folder=os.path.basename(folder),
        ur=delta('underrunQuanta', t0, end),
        gap=delta('engineGapFrames', t0, end),
        lag=delta('lagCuts', t0, end),
        pre=delta('presents', t0, end) / (NW * 10.0),
        idl=delta('idleCount', t0, end) / (NW * 10.0),
        wins_gap=wins_gap,
        worker_s=prof['cpuS']['worker'],
        wall_s=prof['wallS'],
        ms_per_s=prof['cpuS']['worker'] / prof['wallS'] * 1000.0,
        state=rep.get('state'),
        stopped=rep.get('stopped'),
        ok=rep.get('ok'),
        loadStart=rep.get('loadStart'),
        ioStart=rep.get('ioStart'),
    )


def main():
    runs = []
    for folder in sorted(glob.glob(os.path.join(OUT, 'r44d-mix*-*'))):
        m = re.fullmatch(r'r44d-mix(0|5|10|20)-(\d+)', os.path.basename(folder))
        if not m or not os.path.exists(os.path.join(folder, 'audio-p3.json')):
            continue
        runs.append(run(folder, int(m.group(1))))
    runs.sort(key=lambda r: (r['n'], r['folder']))

    print('# r44d-sum: R4-4d(E13。同じ曲を 150 s 再生。マウス移動の頻度 0・5・10・20 回/秒)')
    print('# 窓: 再生開始(marks[play])から 10 s x 15 窓。式は r44b-corr.py・p3-recompute.py と同じ(snaps の線形補間)。')
    print('# pre・idl は 150 s の差分 ÷ 150(1 秒あたり)。ms/s = cpuS.worker ÷ wallS × 1000。')
    print('# 予定 move は scn-p3-mix<N>-150.json の move の件数(予定。実際に送れた数は未測定)。')
    print()
    print('## 1. 回ごとの値')
    print('| 頻度(回/s) | 回 | フォルダ | 予定 move | state | stopped | ok | loadStart | ioStart | ur(150 s) | gap(150 s) | lag | pre | idl | Worker CPU s | wall s | ms/s |')
    print('|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|')
    for r in runs:
        mv = scheduled_moves(r['n'])
        print(f"| {r['n']} | {r['folder'].rsplit('-', 1)[1]} | {r['folder']} | {mv} | {r['state']} | {r['stopped']} | {r['ok']} | {r['loadStart']} | {r['ioStart']} | {r['ur']:.3f} | {r['gap']:.3f} | {r['lag']:.3f} | {r['pre']:.3f} | {r['idl']:.3f} | {r['worker_s']:.2f} | {r['wall_s']:.2f} | {r['ms_per_s']:.2f} |")
    print()

    print('## 1b. 窓ごとの engine gap frames(10 s x 15 窓。回ごと)')
    print('| 頻度(回/s) | 回 | ' + ' | '.join(f'{k * 10}-{k * 10 + 10}' for k in range(NW)) + ' |')
    print('|---|---|' + '---|' * NW)
    for r in runs:
        print(f"| {r['n']} | {r['folder'].rsplit('-', 1)[1]} | " + ' | '.join(f'{v:.0f}' for v in r['wins_gap']) + ' |')
    print()

    print('## 2. 条件ごとの範囲(回の値の最小 〜 最大)')
    print('| 頻度(回/s) | 回数 | gap 最小 〜 最大 | ur 最小 〜 最大 | ms/s 最小 〜 最大 |')
    print('|---|---|---|---|---|')
    groups = {c: [r for r in runs if r['n'] == c] for c in CONDS}
    for c in CONDS:
        g = groups[c]
        if not g:
            print(f'| {c} | 0 | - | - | - |')
            continue
        gv = [r['gap'] for r in g]
        uv = [r['ur'] for r in g]
        mv = [r['ms_per_s'] for r in g]
        print(f'| {c} | {len(g)} | {min(gv):.3f} 〜 {max(gv):.3f} | {min(uv):.3f} 〜 {max(uv):.3f} | {min(mv):.2f} 〜 {max(mv):.2f} |')
    print()

    print('## 3. 頻度の順(engine gap frames。隣り合う条件で、低い条件の最大 < 高い条件の最小 か)')
    broken = []
    for lo, hi in zip(CONDS, CONDS[1:]):
        gl = [r['gap'] for r in groups[lo]]
        gh = [r['gap'] for r in groups[hi]]
        if not gl or not gh:
            print(f'{lo} と {hi}: 比較できず(どちらかの回が 0)')
            broken.append((lo, hi))
            continue
        sep = max(gl) < min(gh)
        print(f'{lo} と {hi}: {lo} の最大 {max(gl):.3f} < {hi} の最小 {min(gh):.3f} → {sep}')
        if not sep:
            broken.append((lo, hi))
    print('頻度の順に並ぶ' if not broken else f'頻度の順に並ばない(崩れた隣接対: {broken})')
    print()

    print('## 4. 頻度と engine gap frames の Spearman(全回。参考)')
    x = [float(r['n']) for r in runs]
    y = [r['gap'] for r in runs]
    rho = spearman(x, y)
    print(f"n = {len(runs)}: {'定義できず(分散 0)' if rho is None else f'{rho:.3f}'}")
    print()

    print('## 注')
    print('注: 頻度は 4 条件の値(0・5・10・20)。同順位は平均順位。n は回の数で少ない。')
    print('注: 実際に送れた move の数は、この出力では測っていない(audio-p3.json に項目が無い)。')


if __name__ == '__main__':
    main()
