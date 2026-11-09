"""usage: statsdiff.py A.err B.err NBLOCKS_DIFF > table : per-block counts of the MXCSR-exact float operations during the process phase = (B - A) / NBLOCKS_DIFF, from two runs with -DFLE_MX_STATS that differ in --blocks. Lines 'fle-mx-vstats: op vec N guard_miss N window_miss N' and 'fle-mx-stats: op fast N slow N'."""
import re, sys
def rd(p):
    v = {}; s = {}
    for l in open(p):
        m = re.match(r'fle-mx-vstats: (\S+)\s+vec (\d+) guard_miss (\d+) window_miss (\d+)', l)
        if m: v[m.group(1)] = tuple(int(x) for x in m.groups()[1:])
        m = re.match(r'fle-mx-stats: (\S+)\s+fast (\d+) slow (\d+)', l)
        if m: s[m.group(1)] = (int(m.group(2)), int(m.group(3)))
    return v, s
a, b, n = sys.argv[1], sys.argv[2], int(sys.argv[3])
va, sa = rd(a); vb, sb = rd(b)
print('op\tvector_path_per_block\tguard_miss_per_block\twindow_miss_per_block\t(fle_mx_stat) fast_per_block\tslow_per_block')
tv = tg = tw = 0
for k in sorted(set(vb) | set(sb)):
    x = [(vb.get(k, (0, 0, 0))[i] - va.get(k, (0, 0, 0))[i]) / n for i in range(3)]
    y = [(sb.get(k, (0, 0))[i] - sa.get(k, (0, 0))[i]) / n for i in range(2)]
    if any(x) or any(y):
        print('%s\t%.0f\t%.0f\t%.0f\t%.0f\t%.0f' % (k, x[0], x[1], x[2], y[0], y[1]))
        tv += x[0]; tg += x[1]; tw += x[2]
print('TOTAL\t%.0f\t%.0f\t%.0f' % (tv, tg, tw))
