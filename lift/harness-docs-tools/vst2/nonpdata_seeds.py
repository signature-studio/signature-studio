"""Filter the output of `fe-decode reloc-seeds FILE --not-function` to the values that lie in NO .pdata RUNTIME_FUNCTION range
(class (ii) of docs/investigations/dll-nonpdata-reloc-seeds.md: leaf functions that the CRT/C++ tables reference). usage: nonpdata_seeds.py FILE reloc.txt OUT"""
import struct, sys
fn, rel, out = sys.argv[1:4]
d = open(fn, 'rb').read()
lfa = struct.unpack_from('<I', d, 0x3c)[0]
nsec = struct.unpack_from('<H', d, lfa + 6)[0]
osz = struct.unpack_from('<H', d, lfa + 20)[0]
opt = lfa + 24
base = struct.unpack_from('<Q', d, opt + 24)[0]
dd = opt + 112
ex_rva, ex_sz = struct.unpack_from('<II', d, dd + 3 * 8)
secs = []
for i in range(nsec):
    s = lfa + 24 + osz + 40 * i
    vs, va, rs, rp = struct.unpack_from('<IIII', d, s + 8)
    secs.append((va, vs, rs, rp))
def r2o(rva):
    for va, vs, rs, rp in secs:
        if va <= rva < va + max(vs, rs): return rp + rva - va
ranges = []
o = r2o(ex_rva)
for i in range(ex_sz // 12):
    b, e, u = struct.unpack_from('<III', d, o + 12 * i)
    ranges.append((b, e))
ranges.sort()
import bisect
starts = [b for b, e in ranges]
def inside(rva):
    k = bisect.bisect_right(starts, rva) - 1
    return k >= 0 and ranges[k][0] <= rva < ranges[k][1]
n = m = 0
with open(out, 'x') as f:
    for l in open(rel):
        l = l.strip()
        if not l: continue
        v = int(l, 16); n += 1
        if not inside(v - base):
            f.write('0x%x\n' % v); m += 1
print('reloc not-function values', n, 'outside every .pdata range', m)
