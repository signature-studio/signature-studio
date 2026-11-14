#!/usr/bin/env python3
# R4-4g: スレッド番号 0・18 などの時系列(初めて動いた pulse・Δa・Δc・1 回の平均 ms・1 秒あたりの回数)。
# 標準ライブラリだけ。引数・環境変数なし。読み取りだけ(標準出力に表)。
import json
import os

BASE = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'out')
RUNS = ['r44d-mix0-1', 'r44d-mix0-2', 'r44d-mix20-1', 'r44d-mix20-2']
THREADS = [0, 18, 8, 6, 1]
WIN_S = 150.0


def interp(xs, ys, x):
    if x <= xs[0]:
        return ys[0]
    if x >= xs[-1]:
        return ys[-1]
    lo, hi = 0, len(xs) - 1
    while hi - lo > 1:
        mid = (lo + hi) // 2
        if xs[mid] <= x:
            lo = mid
        else:
            hi = mid
    f = (x - xs[lo]) / (xs[hi] - xs[lo])
    return ys[lo] + f * (ys[hi] - ys[lo])


def main():
    for run in RUNS:
        path = os.path.join(BASE, run, 'audio-p3.json')
        with open(path) as fh:
            d = json.load(fh)
        play = d['marks']['play']
        pulses = d['pulses']
        epochs = [p['epoch'] for p in pulses]
        print('== %s  pulses=%d  epoch 範囲(再生から s)=%.2f..%.2f' % (
            run, len(pulses), (epochs[0] - play) / 1000.0, (epochs[-1] - play) / 1000.0))
        # 切り替え: pulse 間の mixer 値の変化(再生から s, 値)
        changes = []
        prev = None
        for p in pulses:
            if p['mixer'] != prev:
                changes.append(((p['epoch'] - play) / 1000.0, p['mixer']))
                prev = p['mixer']
        print('  mixer の変化(再生から s, 値): ' + ', '.join('%.1f:%d' % c for c in changes))
        t0 = play
        t1 = play + WIN_S * 1000.0
        for th in THREADS:
            a = [p['threads'][th][0] for p in pulses]
            c = [p['threads'][th][2] for p in pulses]
            first = None
            for p, av in zip(pulses, a):
                if av > 0:
                    first = (p['epoch'] - play) / 1000.0
                    break
            da = interp(epochs, a, t1) - interp(epochs, a, t0)
            dc = interp(epochs, c, t1) - interp(epochs, c, t0)
            ms_per = da / dc if dc > 0 else float('nan')
            per_s = dc / WIN_S
            ftxt = 'なし' if first is None else '%.1f' % first
            print('  thr %2d: 初めて a>0 の pulse(再生から s)=%s  Δa(0-150 s)=%.1f ms  Δc=%.1f 回  1 回平均=%.3f ms  1 秒あたり=%.1f 回' % (
                th, ftxt, da, dc, ms_per, per_s))
        print()


if __name__ == '__main__':
    main()
