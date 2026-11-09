"""Decision 0165 (vst3a): turns the per-function profile of a COUNT run (VST3A_PROFILE_PROCESS / VST3A_PROFILE_INIT: rows fid calls n scalar_sse packed_sse x87 avx) and fnlist.tsv (fid -> VA, from count_fn.py)
into a table of the top functions with the .pdata size of each function and the cumulative share. usage: fprof_report.py FILE.pe fnlist.tsv profile.tsv [TOP] > report.tsv"""
import struct, sys, bisect
pe, fnl, prof = sys.argv[1:4]
top = int(sys.argv[4]) if len(sys.argv) > 4 else 30
d = open(pe, 'rb').read()
lfa = struct.unpack_from('<I', d, 0x3c)[0]
nsec = struct.unpack_from('<H', d, lfa + 6)[0]; osz = struct.unpack_from('<H', d, lfa + 20)[0]
base = struct.unpack_from('<Q', d, lfa + 24 + 24)[0]
secs = []
for i in range(nsec):
    s = lfa + 24 + osz + 40 * i
    vs, va, rs, rp = struct.unpack_from('<IIII', d, s + 8)
    secs.append((va, vs, rs, rp, d[s:s + 8].rstrip(b'\0')))
pd = next(s for s in secs if s[4] == b'.pdata')
fs = []
for k in range(pd[1] // 12):
    b, e, u = struct.unpack_from('<III', d, pd[3] + 12 * k)
    if b: fs.append((b, e))
fs.sort(); starts = [f[0] for f in fs]
def size_of(va):
    r = va - base
    k = bisect.bisect_right(starts, r) - 1
    if k >= 0 and fs[k][0] <= r < fs[k][1]: return fs[k][1] - fs[k][0], (r - fs[k][0])
    return None, None
va_of = {}
for l in list(open(fnl))[1:]:
    f, v, u = l.split('\t'); va_of[int(f)] = int(v, 16)
rows = []
for l in list(open(prof))[1:]:
    t = l.split('\t'); rows.append((int(t[0]), *[int(x) for x in t[1:]]))
tot = sum(r[2] for r in rows)
print('rank\tfid\tva\tcalls\tinstructions\tshare%\tcum%\tscalar_sse\tpacked_sse\tx87\tavx\tpdata_size_B\toffset_in_pdata_func')
cum = 0
for k, r in enumerate(rows[:top], 1):
    cum += r[2]
    sz, off = size_of(va_of[r[0]])
    print('%d\t%d\t%#x\t%d\t%d\t%.3f\t%.3f\t%d\t%d\t%d\t%d\t%s\t%s' % (k, r[0], va_of[r[0]], r[1], r[2], 100.0 * r[2] / tot, 100.0 * cum / tot, r[3], r[4], r[5], r[6], sz if sz is not None else '-', off if off is not None else '-'))
print('# functions %d, instructions %d' % (len(rows), tot))
