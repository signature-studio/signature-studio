#!/usr/bin/env python3
# mt1: tables from the JSON lines of run-rtt.mjs. usage: summarize_rtt.py LOGFILE OUT_TSV
import json, sys, statistics as st
rows = [json.loads(l) for l in open(sys.argv[1]) if l.startswith('{')]
by = {}
for r in rows:
    by.setdefault((r['prod'], r['mode'], r['pmode'], r['k'], r['workUs']), []).append(r)
hdr = ['producer','mode','plugin_mode','K','work_us','reps','load_range','p50_us(min-max)','p90_us','p99_us','p99.9_us','max_us','rounds_gt_1ms(sum)','rounds_total']
out = ['\t'.join(hdr)]
for key, rs in by.items():
    f = lambda k: [r[k] for r in rs]
    loads = [float(x) for r in rs for x in (r['load_start'], r['load_end'])]
    out.append('\t'.join(map(str, [*key[:3], key[3], key[4], len(rs), f'{min(loads):.0f}-{max(loads):.0f}', f"{st.median(f('p50')):g} ({min(f('p50')):g}-{max(f('p50')):g})", f"{st.median(f('p90')):g}", f"{st.median(f('p99')):g}", f"{st.median(f('p999')):g}", f"{max(f('max')):g}", sum(f('over1ms')), sum(r['rounds'] for r in rs)])))
open(sys.argv[2], 'w').write('\n'.join(out) + '\n'); print('\n'.join(out))
