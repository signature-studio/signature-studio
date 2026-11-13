#!/usr/bin/env python3
"""r44-top.py (R4-4): the self-time top 10 of the play-w*.cpuprofile files of tools/verify-kit/out/r44-*/, one table per run,
the order of the top 3 across runs, and the share of (program), (idle) and (garbage collector).
No arguments, no environment variables. Reads only; writes to stdout.

Definitions:
  self time of a function = the sum of the time deltas (timeDeltas, us) of the samples whose top frame is a node of that function
  (nodes are grouped by callFrame functionName + url + lineNumber + columnNumber).
  share = self time / total time of the run (the sum of all time deltas of the run's profile), in %.
  wasm-function[N] is shown as it is: the wasm name section is checked by the caller (see evidence/R4-4.md).
"""
import collections, glob, json, os, re, sys

KIT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
OUT = os.path.join(KIT, 'out')


def runs():
    out = []
    for d in sorted(glob.glob(os.path.join(OUT, 'r44-*')), key=lambda p: int(re.search(r'r44-(\d+)$', p).group(1))):
        if not os.path.isdir(d):
            continue
        for f in sorted(glob.glob(os.path.join(d, 'play-w*.cpuprofile'))):
            out.append((os.path.basename(d), f))
    return out


def load_profile(path):
    p = json.load(open(path))
    nodes = {n['id']: n for n in p['nodes']}
    samples = p.get('samples', [])
    deltas = p.get('timeDeltas', [])
    return nodes, samples, deltas


def key_of(cf):
    return (cf.get('functionName') or '(anonymous)', cf.get('url') or '', cf.get('lineNumber', -1), cf.get('columnNumber', -1))


def analyze(path):
    nodes, samples, deltas = load_profile(path)
    total = sum(deltas)
    self_us = collections.Counter()
    for sid, dt in zip(samples, deltas):
        cf = nodes[sid]['callFrame']
        self_us[key_of(cf)] += dt
    special = {}
    for k in self_us:
        if k[0] in ('(program)', '(idle)', '(garbage collector)'):
            special[k[0]] = special.get(k[0], 0) + self_us[k]
    ranked = [(k, v) for k, v in self_us.most_common() if k[0] not in ('(program)', '(idle)', '(garbage collector)', '(root)')]
    return dict(total=total, samples=len(samples), ranked=ranked, special=special)


def fmt_name(k):
    fn, url, line, col = k
    loc = url.rsplit('/', 1)[-1] if url else ''
    if line is not None and line >= 0:
        loc = f'{loc}:{line + 1}' if loc else f'line {line + 1}'
    return fn, loc


def main():
    rs = runs()
    if not rs:
        print('no play-w*.cpuprofile under tools/verify-kit/out/r44-*/')
        return 1
    res = []
    print('# R4-4: self-time top 10 per run (play-w*.cpuprofile, 30 s of the reference project playing)')
    print('# share % = self time / total time of the run (sum of timeDeltas); total time = sum of all timeDeltas of that profile')
    print()
    for run, path in rs:
        a = analyze(path)
        res.append((run, os.path.basename(path), a))
        tot_ms = a['total'] / 1000
        print(f'## {run} {os.path.basename(path)}: samples {a["samples"]}, total {tot_ms:.1f} ms')
        print('| rank | function | file:line | self ms | share % |')
        print('|---|---|---|---|---|')
        for i, (k, v) in enumerate(a['ranked'][:10], 1):
            fn, loc = fmt_name(k)
            print(f'| {i} | {fn} | {loc} | {v / 1000:.1f} | {100 * v / a["total"]:.2f} |')
        sp = a['special']
        print(f'special: (program) {100 * sp.get("(program)", 0) / a["total"]:.2f} %, '
              f'(idle) {100 * sp.get("(idle)", 0) / a["total"]:.2f} %, '
              f'(garbage collector) {100 * sp.get("(garbage collector)", 0) / a["total"]:.2f} %')
        print()
    print('## order across runs (top 3 by self time)')
    print('| run | 1st | 2nd | 3rd |')
    print('|---|---|---|---|')
    tops = []
    for run, name, a in res:
        t3 = [fmt_name(k)[0] for k, _ in a['ranked'][:3]]
        tops.append(t3)
        print(f'| {run} | ' + ' | '.join(t3) + ' |')
    same = len(tops) > 1 and all(t == tops[0] for t in tops)
    print()
    print(f'top 3 identical in all runs (same names, same order): {"yes" if same else "no"}')
    sets = [set(t) for t in tops]
    print(f'top 3 same set of names in all runs: {"yes" if all(s == sets[0] for s in sets) else "no"}')
    return 0


sys.exit(main())
