"""design aid (read-only): scan the code section of a wasm file; print the number of functions, the maximum local count,
how many functions have more than LIMIT locals (default 50000) and the top few (module-wide function index, locals, body bytes)."""
import sys, mmap
limit = int(sys.argv[2]) if len(sys.argv) > 2 else 50000
f = open(sys.argv[1], 'rb'); d = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
def uleb(p):
    r = 0; s = 0
    while True:
        b = d[p]; p += 1; r |= (b & 0x7f) << s; s += 7
        if not b & 0x80: return r, p
p = 8; secs = {}
while p < len(d):
    sid = d[p]; p += 1; sz, p = uleb(p)
    if sid in (2, 10): secs[sid] = (p, sz)
    p += sz
ip, _ = secs[2]; n, q = uleb(ip); nimp = 0
for i in range(n):
    l, q = uleb(q); q += l; l, q = uleb(q); q += l; k = d[q]; q += 1
    if k == 0: _, q = uleb(q); nimp += 1
    elif k == 1: q += 1; fl = d[q]; q += 1; _, q = uleb(q);  (fl & 1) and uleb(q)
    elif k == 2:
        fl = d[q]; q += 1; _, q = uleb(q)
        if fl & 1: _, q = uleb(q)
    elif k == 3: q += 2
cp, _ = secs[10]; n, q = uleb(cp); res = []
for i in range(n):
    l, q2 = uleb(q); nl, r = uleb(q2); tot = 0
    for g in range(nl):
        c, r = uleb(r); r += 1; tot += c
    res.append((tot, nimp + i, l)); q = q2 + l
res.sort(reverse=True)
print('functions', n, 'max locals', res[0][0], 'over %d:' % limit, sum(1 for r in res if r[0] > limit))
for r in res[:8]: print('  func #%d locals %d body %d bytes' % (r[1], r[0], r[2]))
