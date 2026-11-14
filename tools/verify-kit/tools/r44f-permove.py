#!/usr/bin/env python3
# r44f-permove.py: per-thread run time (a) and run count (c) from pulses[].threads of audio-p3.json, per scheduled move, and as difference from 0 moves/s.
# Reads out/r44d-mix<N>-<M> (150 s, window 0-150 s from marks.play) and out/p3-chrome-{1,2,3} (condition 1) and out/p3-mixmove-chrome-{1,2,3} (condition 2) (300 s, split 0-150 and 150-300 s).
# threads[i] = [a, b, c]: a = run time of thread i in ms (sl_total), c = run count of thread i (sl_n), both cumulative (lift/runtime/fle_audio_transport.c:136-156). Values are interpolated linearly on epoch, the same formula as r44e-threads.py.
# Scheduled moves are 150 s x frequency (evidence/R4-4d.md §1). Actual sends are not in audio-p3.json.
# Standard library only. Reads only; writes nothing. No arguments, no environment variables.
import bisect
import json
import os
import statistics

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, '..', 'out')
MOVES = {0: 0, 5: 750, 10: 1500, 20: 3000}
R44D_RUNS = [(mm, m) for mm in (0, 5, 10, 20) for m in (1, 2)]
R43_RUNS = [('p3-chrome-1', 1), ('p3-chrome-2', 1), ('p3-chrome-3', 1),
            ('p3-mixmove-chrome-1', 2), ('p3-mixmove-chrome-2', 2), ('p3-mixmove-chrome-3', 2)]
KEEP = (0, 18)
N_TOP = 5


def interp(xs, ys, t):
    i = bisect.bisect_left(xs, t)
    if i <= 0:
        return ys[0]
    if i >= len(xs):
        return ys[-1]
    w = (t - xs[i - 1]) / max(1e-9, xs[i] - xs[i - 1])
    return ys[i - 1] + w * (ys[i] - ys[i - 1])


class Run:
    def __init__(self, folder):
        a = json.load(open(os.path.join(folder, 'audio-p3.json')))
        self.t0 = a['marks']['play']
        pulses = sorted(a['pulses'], key=lambda p: p['epoch'])
        self.ep = [p['epoch'] for p in pulses]
        self.nth = min(len(p['threads']) for p in pulses)
        self.col = {(i, j): [p['threads'][i][j] for p in pulses] for i in range(self.nth) for j in range(3)}
        self.last_s = (self.ep[-1] - self.t0) / 1000.0

    def val(self, i, j, t_s):
        return interp(self.ep, self.col[(i, j)], self.t0 + t_s * 1000.0)

    def delta(self, lo_s, hi_s):
        da = [self.val(i, 0, hi_s) - self.val(i, 0, lo_s) for i in range(self.nth)]
        dc = [self.val(i, 2, hi_s) - self.val(i, 2, lo_s) for i in range(self.nth)]
        return da, dc


def top(vals, n=N_TOP):
    return sorted(range(len(vals)), key=lambda i: vals[i], reverse=True)[:n]


def f1(x):
    return f'{x:,.1f}'


def row(cells):
    print('| ' + ' | '.join(cells) + ' |')


def main():
    r44d = {}
    for mm, m in R44D_RUNS:
        run = Run(os.path.join(OUT, f'r44d-mix{mm}-{m}'))
        da, dc = run.delta(0, 150)
        r44d[(mm, m)] = dict(run=run, da=da, dc=dc)
    ctrl_a = [(r44d[(0, 1)]['da'][i] + r44d[(0, 2)]['da'][i]) / 2 for i in range(32)]
    ctrl_c = [(r44d[(0, 1)]['dc'][i] + r44d[(0, 2)]['dc'][i]) / 2 for i in range(32)]
    sel = set(KEEP) | set(top(ctrl_a))
    for mm, m in R44D_RUNS:
        sel |= set(top(r44d[(mm, m)]['da']))
    sel = sorted(sel)
    names = [f'{mm}-{m}' for mm, m in R44D_RUNS]

    print('# r44f-permove: スレッドごとの実行 ms(a)・実行回数(c)の、台本の移動 1 回あたり・0 回/秒との差(既存の生データの再計算。測定なし)')
    print('# 区間: 再生開始(marks.play)から 0〜150 s(R4-4d の 8 回)。R4-3 の 6 回は 0〜150 s と 150〜300 s の 2 区間。値は pulse の epoch に対する線形補間(r44e-threads.py と同じ式)。')
    print('# 列名 "頻度-回": 例 "5-1" = 5 回/秒の 1 回目。対照 = 0 回/秒の 2 回の平均。差 = その run − 対照。')
    print('# 移動の件数: 台本の予定(0・750・1,500・3,000。150 s × 頻度。evidence/R4-4d.md §1)。移動の実際の送信数は未測定。スレッド番号の役割は未確認。')
    print()

    print('## 1. R4-4d 0〜150 s の生の値(ms・回)')
    print('選んだスレッド: 0・18・0 回/秒の平均の上位 5・各 run の上位 5 の和集合。')
    print()
    print('### 1.1 Δa(実行 ms)')
    row(['スレッド', '対照(0 回/秒 平均)'] + names)
    row(['---'] * (2 + len(names)))
    for t in sel:
        row([str(t), f1(ctrl_a[t])] + [f1(r44d[k]['da'][t]) for k in R44D_RUNS])
    print()
    print('### 1.2 Δc(実行回数)')
    row(['スレッド', '対照(0 回/秒 平均)'] + names)
    row(['---'] * (2 + len(names)))
    for t in sel:
        row([str(t), f1(ctrl_c[t])] + [f1(r44d[k]['dc'][t]) for k in R44D_RUNS])
    print()

    print('## 2. R4-4d 対照との差(Δ − 対照)')
    print()
    moving = [(mm, m) for mm, m in R44D_RUNS if mm != 0]
    mv_names = [f'{mm}-{m}' for mm, m in moving]
    print('### 2.1 Δa 差 ÷ 台本の move 回数(ms/回)')
    row(['スレッド'] + mv_names)
    row(['---'] * (1 + len(mv_names)))
    for t in sel:
        row([str(t)] + [f1((r44d[(mm, m)]['da'][t] - ctrl_a[t]) / MOVES[mm]) for mm, m in moving])
    print()
    print('### 2.2 Δa 差(ms・150 s あたり)')
    row(['スレッド'] + names)
    row(['---'] * (1 + len(names)))
    for t in sel:
        row([str(t)] + [f1(r44d[k]['da'][t] - ctrl_a[t]) for k in R44D_RUNS])
    print()
    print('### 2.3 Δc 差 ÷ 台本の move 回数(回/回)')
    row(['スレッド'] + mv_names)
    row(['---'] * (1 + len(mv_names)))
    for t in sel:
        row([str(t)] + [f1((r44d[(mm, m)]['dc'][t] - ctrl_c[t]) / MOVES[mm]) for mm, m in moving])
    print()
    print('### 2.4 Δc 差(回・150 s あたり)')
    row(['スレッド'] + names)
    row(['---'] * (1 + len(names)))
    for t in sel:
        row([str(t)] + [f1(r44d[k]['dc'][t] - ctrl_c[t]) for k in R44D_RUNS])
    print()

    print('### 2.5 全スレッドの合計(150 s)')
    row(['回', '台本の move', 'Δa 全スレッド合計(ms)', 'Δa 合計 ÷ 150,000', 'Δa 合計 − 150,000(ms)', 'Δc 全スレッド合計(回)'])
    row(['---'] * 6)
    for mm, m in R44D_RUNS:
        da_sum = sum(r44d[(mm, m)]['da'])
        dc_sum = sum(r44d[(mm, m)]['dc'])
        row([f'{mm} 回/秒 {m}', f'{MOVES[mm]:,}', f1(da_sum), f'{da_sum / 150000:.4f}', f1(da_sum - 150000), f1(dc_sum)])
    print()

    print('## 3. R4-3 の 6 回(0〜150 s と 150〜300 s。条件ごとに 3 回の中央値)')
    print()
    r43 = []
    for name, cond in R43_RUNS:
        run = Run(os.path.join(OUT, name))
        r43.append(dict(cond=cond, name=name, w1=run.delta(0, 150), w2=run.delta(150, 300), last_s=run.last_s))
    med = {}
    for cond in (1, 2):
        rs = [x for x in r43 if x['cond'] == cond]
        for w in ('w1', 'w2'):
            med[(cond, w)] = [statistics.median([x[w][0][i] for x in rs]) for i in range(len(rs[0][w][0]))]
    sel3 = set(KEEP)
    for key in med:
        sel3 |= set(top(med[key]))
    sel3 = sorted(sel3)
    print('選んだスレッド: 0・18・各区間の中央値 Δa の上位 5 の和集合。')
    print()
    row(['スレッド', '条件 1 0〜150 s', '条件 1 150〜300 s', '条件 2 0〜150 s', '条件 2 150〜300 s'])
    row(['---'] * 5)
    for t in sel3:
        row([str(t), f1(med[(1, 'w1')][t]), f1(med[(1, 'w2')][t]), f1(med[(2, 'w1')][t]), f1(med[(2, 'w2')][t])])
    row(['全スレッド合計 ÷ 150,000',
         f'{sum(med[(1, "w1")]) / 150000:.4f}', f'{sum(med[(1, "w2")]) / 150000:.4f}',
         f'{sum(med[(2, "w1")]) / 150000:.4f}', f'{sum(med[(2, "w2")]) / 150000:.4f}'])
    print()
    print('R4-3 の 6 回(区間の Δa 全スレッド合計 ÷ 150,000。回ごと):')
    for x in r43:
        print(f"- {x['name']}: 0〜150 s {sum(x['w1'][0]) / 150000:.4f} / 150〜300 s {sum(x['w2'][0]) / 150000:.4f}(最後の pulse {x['last_s']:.1f} s)")
    print()

    print('## 4. 検算')
    print('(a) 10 s 窓の a の最大(スレッド: ms)。r44e-threads.py §1 の「a 最大」列と同じ値になるはず。')
    for mm, m in R44D_RUNS:
        run = r44d[(mm, m)]['run']
        cells = []
        for k in range(15):
            da, _ = run.delta(10 * k, 10 * k + 10)
            i = max(range(run.nth), key=lambda j: da[j])
            cells.append(f'{k * 10}-{k * 10 + 10}: {i}: {da[i]:.2f}')
        print(f'- r44d-mix{mm}-{m}: ' + ' | '.join(cells))
    print()
    print('(b) 10 s 窓の Δa の合計と 0〜150 s の Δa の差(最大の絶対誤差 ms。0 に近ければ窓の和が一致)。')
    for mm, m in R44D_RUNS:
        run = r44d[(mm, m)]['run']
        acc = [0.0] * run.nth
        for k in range(15):
            da, _ = run.delta(10 * k, 10 * k + 10)
            acc = [acc[i] + da[i] for i in range(run.nth)]
        err = max(abs(acc[i] - r44d[(mm, m)]['da'][i]) for i in range(run.nth))
        print(f'- r44d-mix{mm}-{m}: {err:.9f}')
    print()
    print('(c) 移動の件数(台本の予定): ' + ', '.join(f'{mm} 回/秒 = {MOVES[mm]:,}' for mm in (5, 10, 20)) + '(evidence/R4-4d.md §1 と一致)')


if __name__ == '__main__':
    main()
