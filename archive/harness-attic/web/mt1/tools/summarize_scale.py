#!/usr/bin/env python3
# mt1: tables from results/<tag>-<mod>-summary.json (run-scale.mjs). usage: summarize_scale.py RESULTS_DIR TAG OUT_TSV
import json, sys, statistics as st, glob, os
d, tag, out = sys.argv[1:4]
rows = []
for f in sorted(glob.glob(os.path.join(d, f'{tag}-*-summary.json'))):
    rows += json.load(open(f))
by = {}
for r in rows:
    by.setdefault((r['mod'], r['n']), []).append(r)
hdr = ['mod','n','reps','load_range','block_ms_median_of_reps','block_ms_min_of_reps','block_ms_max_of_reps','best_worker_p50_ms','p99_worst_ms','max_block_ms','cpu_ms_per_worker_median','rss_delta_MB_median','hashes_equal']
lines = ['\t'.join(hdr)]
base = {}
for (mod, n), rs in sorted(by.items()):
    p50 = [r['perBlockMedianOfMedians'] for r in rs]
    cpu = [sum(r['workerCpuMs']) / len(r['workerCpuMs']) for r in rs if r['workerCpuMs']]
    loads = [float(r['load_start']) for r in rs] + [float(r['load_end']) for r in rs]
    rss = [r['rssMB_after'] - r['rssMB_before'] for r in rs]
    lines.append('\t'.join(map(str, [mod, n, len(rs), f'{min(loads):.0f}-{max(loads):.0f}', f'{st.median(p50):.2f}', f'{min(p50):.2f}', f'{max(p50):.2f}', f"{min(r['perBlockP50min'] for r in rs):.2f}", f"{max(r['perBlockP99worst'] for r in rs):.1f}", f"{max(r['perBlockMax'] for r in rs):.1f}", f'{st.median(cpu):.0f}' if cpu else '', f'{st.median(rss):.0f}', all(r['hashesEqual'] for r in rs)])))
open(out, 'w').write('\n'.join(lines) + '\n')
print('\n'.join(lines))
