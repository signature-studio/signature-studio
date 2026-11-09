#!/usr/bin/env python3
"""Decision 0177 (srm1): ab_measure.sh output -> TSV rows (build case rep load_before load_after cpu_ms_block0 median min max hash init_cpu_ms) + a summary per (build, case):
n, min of the per-run minima, median of the per-run minima, median of the per-run medians, load range, hash set.   usage: parse_ab.py AB.txt RAW.tsv SUMMARY.tsv"""
import re, sys, statistics as st, collections
raw = []; cur = None
for line in open(sys.argv[1], errors='replace'):
    m = re.match(r'RUN (\S+) case=(\S+) rep=(\d+) load_before=(\S+) load_after=(\S+)', line)
    if m:
        cur = dict(build=m.group(1), case=m.group(2), rep=m.group(3), lb=float(m.group(4)), la=float(m.group(5)), hash='', b0='', med='', mn='', mx='', init='')
        raw.append(cur); continue
    if cur is None: continue
    m = re.search(r'RESULT blocks=\d+ .*?cpu_ms_block0=([\d.]+) cpu_ms_median=([\d.]+) cpu_ms_min=([\d.]+) cpu_ms_max=([\d.]+)', line)
    if m: cur.update(b0=m.group(1), med=m.group(2), mn=m.group(3), mx=m.group(4))
    m = re.search(r'RESULT out_hash=(\w+)', line)
    if m: cur['hash'] = m.group(1)
    m = re.search(r'RESULT init_cpu_ms=([\d.]+)', line)
    if m: cur['init'] = m.group(1)
    if re.search(r'Aborted|misaligned|failed', line): cur['hash'] = 'ERROR'
with open(sys.argv[2], 'w') as f:
    f.write('build\tcase\trep\tload_before\tload_after\tcpu_ms_block0\tcpu_ms_median\tcpu_ms_min\tcpu_ms_max\tout_hash\tinit_cpu_ms\n')
    for r in raw: f.write('\t'.join(str(r[k]) for k in ('build', 'case', 'rep', 'lb', 'la', 'b0', 'med', 'mn', 'mx', 'hash', 'init')) + '\n')
g = collections.defaultdict(list)
for r in raw:
    if r['mn'] != '': g[(r['build'], r['case'])].append(r)
with open(sys.argv[3], 'w') as f:
    f.write('build\tcase\tn\tmin_of_min_ms\tmedian_of_min_ms\tmedian_of_median_ms\tload_min\tload_max\thashes\n')
    for (b, c), rs in sorted(g.items(), key=lambda kv: (kv[0][1], kv[0][0])):
        mins = [float(r['mn']) for r in rs]; meds = [float(r['med']) for r in rs]; loads = [x for r in rs for x in (r['lb'], r['la'])]
        f.write(f"{b}\t{c}\t{len(rs)}\t{min(mins):.1f}\t{st.median(mins):.1f}\t{st.median(meds):.1f}\t{min(loads):.1f}\t{max(loads):.1f}\t{','.join(sorted({r['hash'] for r in rs}))}\n")
print(open(sys.argv[3]).read())
