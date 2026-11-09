#!/usr/bin/env python3
"""lat1: per-input guest work from the native fork-probe run. usage: natan.py RUNDIR OUTDIR
cov.txt.<probe> = 'F va count' (entries since process start, the parent's included). Input work = probe - control (same 2000 virtual ms from the same state)."""
import sys, os, re, collections, glob
rundir, outdir = sys.argv[1], sys.argv[2]
os.makedirs(outdir, exist_ok=True)
SEEDS = '/home/ubuntu/work/signature-studio-develop/lift/harness/seeds/'
direct = {}
for mod in ('fle', 'dsp', 'b2d', 'qfc', 'ft', 'ilt'):
    for l in open(SEEDS + 'hyb-U-%s.txt' % mod):
        l = l.strip()
        if l.startswith('0x'): direct[(mod, int(l, 16))] = 1
direct_vas = set(v for (m, v) in direct)
def load(p):
    d = {}
    for l in open(p):
        t = l.split()
        if len(t) == 3 and t[0] == 'F':
            d[int(t[1], 16) if t[1].startswith('0x') else int(t[1], 16)] = int(t[2])
    return d
files = {os.path.basename(f)[len('cov.txt.'):]: f for f in glob.glob(rundir + '/cov.txt.*')}
cov = {k: load(f) for k, f in files.items()}
probes = sorted(cov)
roles = {}
rp = '/home/ubuntu/work/signature-studio-develop/lift/out-ui1/tables/node-top30-D-prime-roles.tsv'
if os.path.exists(rp):
    for l in open(rp):
        t = l.rstrip('\n').split('\t')
        if len(t) > 8 and t[1].startswith('f_'): roles[int(t[1][2:], 16)] = t[8][:140]
def mod(va):
    if va < 0x1d00000: return 'fle'
    if va < 0x2000000: return 'qfc'
    if va < 0x4700000: return 'dsp'
    if va < 0x4b00000: return 'b2d'
    if va < 0x6000000: return 'ft'
    return 'ilt'
rows = []
tops = {}
allpos = {}
for state in ('i_', 'p_'):
    ctl = cov.get(state + 'ctl')
    if ctl is None: continue
    for pn in probes:
        if not pn.startswith(state) or pn == state + 'ctl': continue
        c = cov[pn]
        delta = {}
        for va in set(c) | set(ctl):
            dv = c.get(va, 0) - ctl.get(va, 0)
            if dv: delta[va] = dv
        pos = {va: v for va, v in delta.items() if v > 0}
        tot = sum(pos.values()); neg = -sum(v for v in delta.values() if v < 0)
        tot_c = sum(c.values()) - sum(ctl.values())
        in_direct = sum(v for va, v in pos.items() if va in direct_vas)
        # cumulative top functions
        order = sorted(pos.items(), key=lambda kv: -kv[1])
        cum = 0; n50 = n80 = None
        for i, (va, v) in enumerate(order):
            cum += v
            if n50 is None and cum >= 0.5 * tot: n50 = i + 1
            if n80 is None and cum >= 0.8 * tot: n80 = i + 1
        rows.append((state, pn[len(state):], tot_c, tot, neg, len(pos), in_direct, n50, n80))
        tops[pn] = order[:12]
        allpos[pn] = pos
ctl_tot = {s: sum(cov[s + 'ctl'].values()) for s in ('i_', 'p_') if s + 'ctl' in cov}
with open(outdir + '/guest-entries-per-input.tsv', 'w') as f:
    f.write('state\tprobe\tentries probe-ctl (net)\textra entries (sum of positive deltas)\tfewer entries (sum of negative deltas)\tfunctions with more entries\tof the extra: in the direct-wasm set p1000r\tdirect share\tfunctions for 50% of the extra\tfor 80%\n')
    for (s, n, tc, t, neg, nf, ind, n50, n80) in rows:
        f.write('%s\t%s\t%d\t%d\t%d\t%d\t%d\t%.1f%%\t%s\t%s\n' % ('idle' if s == 'i_' else 'play', n, tc, t, neg, nf, ind, 100.0 * ind / t if t else 0, n50, n80))
with open(outdir + '/guest-top-functions.tsv', 'w') as f:
    f.write('state\tprobe\trank\tfunction VA\textra entries\tshare of the extra\tin direct set\trole (ui1 reading, if any)\n')
    for (s, n, tc, t, neg, nf, ind, n50, n80) in rows:
        for i, (va, v) in enumerate(tops[s + n]):
            f.write('%s\t%s\t%d\t0x%x\t%d\t%.1f%%\t%s\t%s\n' % ('idle' if s == 'i_' else 'play', n, i + 1, va, v, 100.0 * v / t if t else 0, 'yes' if va in direct_vas else 'no', roles.get(va, '')))
# candidates for the next hybrid list: functions NOT in the p1000r direct set, ranked by the extra entries they get over all the input probes (idle and play separately summed per probe kind)
cand = collections.defaultdict(lambda: collections.Counter())
for pn, pos in allpos.items():
    kind = pn[2:] + ('/play' if pn.startswith('p_') else '/idle')
    for va, v in pos.items():
        if va not in direct_vas: cand[va][kind] += v
with open(outdir + '/hybrid-candidates-input.tsv', 'w') as f:
    f.write('function VA\tmodule\ttotal extra entries over all input probes\tin which probes (extra entries)\trole (ui1 reading, if any)\n')
    for va, c in sorted(cand.items(), key=lambda kv: -sum(kv[1].values()))[:60]:
        f.write('0x%x\t%s\t%d\t%s\t%s\n' % (va, mod(va), sum(c.values()), '; '.join('%s %d' % (k, v) for k, v in c.most_common(6)), roles.get(va, '')))
with open(outdir + '/guest-control.tsv', 'w') as f:
    f.write('state\tcontrol entries since start (all)\n')
    for s, v in ctl_tot.items(): f.write('%s\t%d\n' % ('idle' if s == 'i_' else 'play', v))
print(open(outdir + '/guest-entries-per-input.tsv').read())
