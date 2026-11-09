#!/usr/bin/env python3
# mt1: table from the JSON lines of run-pipe.mjs. usage: summarize_pipe.py LOGFILE OUT_TSV
import json, sys, statistics as st
rows = [json.loads(l) for l in open(sys.argv[1]) if l.startswith('{')]
by = {}
for r in rows: by.setdefault((r['topo'], r['k'], r['paced']), []).append(r)
out = ['\t'.join(['topology', 'K', 'paced(92.88ms)', 'reps', 'load_range', 'block_ms_p50(min-max of reps)', 'p90', 'p99', 'max', 'first_block_ms(median)', 'late_blocks(>92.88ms,after warm-up)/blocks', 'late_blocks_incl_first', 'cpu_ms_per_worker(150 blocks+init)'])]
for key, rs in by.items():
    f = lambda k: [r[k] for r in rs]
    loads = [float(x) for r in rs for x in (r['load_start'], r['load_end'])]
    out.append('\t'.join(map(str, [key[0], key[1], key[2], len(rs), f'{min(loads):.0f}-{max(loads):.0f}', f"{st.median(f('lat_p50')):.1f} ({min(f('lat_p50')):.1f}-{max(f('lat_p50')):.1f})", f"{st.median(f('lat_p90')):.1f}", f"{st.median(f('lat_p99')):.1f}", f"{max(f('lat_max')):.1f}", f"{st.median([r['first'][0] for r in rs]):.0f}", f"{sum(f('missesSteady'))}/{sum(r['blocks'] - 5 for r in rs)}", sum(f('missesAll')), f"{st.median([sum(r['workerCpuMs'][:r['k']]) / r['k'] for r in rs]):.0f}"])))
open(sys.argv[2], 'w').write('\n'.join(out) + '\n'); print('\n'.join(out))
