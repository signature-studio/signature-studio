#!/usr/bin/env python3
"""parity1: symbolise a PARITY_PROF sample file of the native build. usage: profsym.py SAMPLES BINARY [--from MS --to MS] [--top N] [--cat]
--from/--to select samples by virtual clock ms (g3-snap clock_ms of the operation's two snapshots)."""
import struct, subprocess, sys, bisect, collections
a = sys.argv[1:]
d = open(a[0], 'rb').read(); binary = a[1]
lo = int(a[a.index('--from') + 1]) if '--from' in a else 0; hi = int(a[a.index('--to') + 1]) if '--to' in a else 1 << 62
top = int(a[a.index('--top') + 1]) if '--top' in a else 30
recs = [struct.unpack('<QQ', d[i:i + 16]) for i in range(0, len(d) - 15, 16)]
anchor = recs[0][0]; recs = recs[1:]
syms = []
for ln in subprocess.run(['nm', '-n', '--defined-only', binary], capture_output=True, text=True).stdout.splitlines():
    t = ln.split()
    if len(t) == 3 and t[1] in 'tTwW': syms.append((int(t[0], 16), t[2]))
addrs = [s[0] for s in syms]
base = anchor - next(s[0] for s in syms if s[1] == 'parity_prof_anchor')
cnt = collections.Counter(); tot = 0
def cat(n):
    if n.startswith('f_') or n.startswith('fb_'): return 'guest (lifted FLEngine/modules)'
    if n.startswith('fle_imp_') or n.startswith('fle_'): return 'runtime fle_*'
    if n.startswith('bl') or '_ZN2bl' in n: return 'Blend2D upstream'
    return 'libc/other'
for pc, ms in recs:
    if not (lo <= ms <= hi): continue
    k = bisect.bisect_right(addrs, pc - base) - 1
    cnt[syms[k][1] if k >= 0 else '?'] += 1; tot += 1
print('samples', tot)
bycat = collections.Counter()
for n, c in cnt.items(): bycat[cat(n)] += c
for n, c in bycat.most_common(): print('%6d %5.1f%%  %s' % (c, 100.0 * c / tot, n))
print('-- top')
for n, c in cnt.most_common(top): print('%6d %5.1f%%  %s' % (c, 100.0 * c / tot, n))
