#!/usr/bin/env python3
"""Decision 0177 (srm1): turns the output of run_states.sh (any number of files; label = the file name without extension) into TSV rows:
label  run  load_before  load_after  blocks  cpu_ms_block0  cpu_ms_median  cpu_ms_min  cpu_ms_max  out_hash  setState_cpu_ms  icount_per_block
usage: parse_runs.py OUT.tsv FILE..."""
import re, sys, os
rows = []
for fn in sys.argv[2:]:
    label = os.path.splitext(os.path.basename(fn))[0]
    cur = None
    for line in open(fn, errors='replace'):
        m = re.match(r'== (\S+)\s+load_before=(\S+) load_after=(\S+)', line)
        if m:
            cur = {'label': label, 'run': m.group(1), 'lb': m.group(2), 'la': m.group(3), 'ss': ''}
            rows.append(cur)
            continue
        if cur is None:
            continue
        m = re.search(r'RESULT blocks=(\d+) .*?cpu_ms_block0=([\d.]+) cpu_ms_median=([\d.]+) cpu_ms_min=([\d.]+) cpu_ms_max=([\d.]+).*?icount_per_block_mean=(\d+)', line)
        if m:
            cur.update(blocks=m.group(1), b0=m.group(2), med=m.group(3), mn=m.group(4), mx=m.group(5), ic=m.group(6))
        m = re.search(r'RESULT out_hash=(\w+)', line)
        if m:
            cur['hash'] = m.group(1)
        m = re.search(r'setState.*?cpu ([\d.]+) ms', line)
        if m and not cur['ss']:
            cur['ss'] = m.group(1)
with open(sys.argv[1], 'w') as f:
    f.write('label\trun\tload_before\tload_after\tblocks\tcpu_ms_block0\tcpu_ms_median\tcpu_ms_min\tcpu_ms_max\tout_hash\tsetState_cpu_ms\ticount_per_block\n')
    for r in rows:
        f.write('\t'.join(str(r.get(k, '')) for k in ('label', 'run', 'lb', 'la', 'blocks', 'b0', 'med', 'mn', 'mx', 'hash', 'ss', 'ic')) + '\n')
print(open(sys.argv[1]).read())
