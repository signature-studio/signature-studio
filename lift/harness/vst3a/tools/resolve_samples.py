"""usage: resolve_samples.py BINARY SAMPLES.txt [TOP] : maps the sample offsets of VST3A_SAMPLE (3 frames per sample) to the nearest preceding text symbol of BINARY (nm) and prints the share per top-frame symbol,
and the share of samples whose top 3 frames contain a lifted function f_<VA> (self = top frame only)."""
import subprocess, sys, bisect, collections
b, sf = sys.argv[1:3]
top = int(sys.argv[3]) if len(sys.argv) > 3 else 25
syms = []
for l in subprocess.check_output(['nm', '-n', '--defined-only', b]).decode().split('\n'):
    t = l.split()
    if len(t) == 3 and t[1] in 'tTwW': syms.append((int(t[0], 16), t[2]))
addrs = [s[0] for s in syms]
def name(o):
    if o == 0: return '-'
    k = bisect.bisect_right(addrs, o - 1) - 1   # a return address is after the call: o - 1 is inside the caller
    return syms[k][1] if k >= 0 else '?'
rows = [tuple(int(x, 16) for x in l.split()) for l in open(sf)]
c0 = collections.Counter(name(r[0]) for r in rows)
n = len(rows)
print('samples', n)
cum = 0
for s, c in c0.most_common(top):
    cum += c
    print('%6d %6.2f%% cum %6.2f%%  %s' % (c, 100.0 * c / n, 100.0 * cum / n, s))
lifted = sum(1 for r in rows if name(r[0]).startswith('f_'))
print('top frame is a lifted function f_*: %.1f%%; runtime/other: %.1f%%' % (100.0 * lifted / n, 100.0 - 100.0 * lifted / n))
cs = collections.Counter()
for r in rows:
    cs[name(r[0]) if name(r[0]).startswith('f_') else (next((name(x) for x in r[1:] if name(x).startswith('f_')), '(no lifted frame)'))] += 1
print('-- attributing runtime-helper samples to the nearest lifted caller:')
cum = 0
for s, c in cs.most_common(top):
    cum += c
    print('%6d %6.2f%% cum %6.2f%%  %s' % (c, 100.0 * c / n, 100.0 * cum / n, s))
