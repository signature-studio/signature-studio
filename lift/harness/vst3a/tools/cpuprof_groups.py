#!/usr/bin/env python3
"""Decision 0177 (srm1): groups the self time of a V8 .cpuprofile of the wasm Serum 2 harness (linked with --profiling-funcs) over the process window: lifted functions (f_<va>), the exact-rounding
helpers of decision 0172 (fle_mxrc_*), the older MXCSR code (fle_mx_*), other runtime/libc, and (idle)/(program)/(garbage collector). Also prints the top N functions with their ORIGINAL va
(the profiled image is rebased: va_orig = va + 0x180000000 - 0x2000000).
usage: cpuprof_groups.py FILE.cpuprofile PROCESS_MS [TOPN=12]   (PROCESS_MS = blocks x mean block ms + about 350 ms: the window is the last PROCESS_MS of the profile)"""
import json, sys, collections
p = json.load(open(sys.argv[1])); win_us = float(sys.argv[2]) * 1000; topn = int(sys.argv[3]) if len(sys.argv) > 3 else 12
byid = {n['id']: n for n in p['nodes']}
t = p['startTime']; tot = sum(p['timeDeltas']); start = tot - win_us
self_ = collections.Counter(); acc = 0; total = 0
for s, dt in zip(p['samples'], p['timeDeltas']):
    acc += dt
    if acc < start: continue
    name = byid[s]['callFrame']['functionName'] or '(anonymous)'
    self_[name] += dt; total += dt
def group(n):
    if n.startswith('f_'): return 'lifted functions f_<va>'
    if n.startswith('fle_mxrc_'): return 'exact-rounding helpers fle_mxrc_* (decision 0172)'
    if n.startswith('fle_mx'): return 'other fle_mx* (decision 0143/0154)'
    if n.startswith('(') : return n
    if n.startswith('fle_'): return 'other runtime fle_*'
    return 'other (libc, harness, wasm-function[n])'
g = collections.Counter()
for n, v in self_.items(): g[group(n)] += v
print('window %.0f ms of %.0f ms; sampled %.0f ms' % (win_us / 1000, tot / 1000, total / 1000))
for k, v in g.most_common(): print('  %-55s %8.1f ms %5.1f %%' % (k, v / 1000, 100 * v / total))
print('top functions:')
for n, v in self_.most_common(topn):
    o = ''
    if n.startswith('f_'):
        try: o = '  (orig f_%x)' % (int(n[2:], 16) + 0x180000000 - 0x2000000)
        except ValueError: pass
    print('  %-28s %8.1f ms %5.1f %%%s' % (n, v / 1000, 100 * v / total, o))
