#!/usr/bin/env python3
"""pw1 (decision 0186): rows.jsonl files of run-pw1.mjs -> a TSV with one line per condition (hc x serum list x OTT x warm), aggregated over the repetitions.
usage: summarize.py OUT.tsv rows1.jsonl [rows2.jsonl ...]"""
import json, sys, collections

out = sys.argv[1]
# out_hash of the node runs of the same wasm, 650 blocks (lift/out-pw1/logs/node-650.txt); silent = all-zero output (same hash for every state)
EXP = {'0': '2f3b204e92461575', '1': 'a2ca9a834995e6de', '3': 'be9664de2a390723', '4': 'eb3390cc8731cf4a', '5': 'a118fe01c9e05450', '3c': 'e94d95275ab66e69'}
def hash_ok(r):
    ok = n = 0
    for p in r['per']:
        if p.get('kind') not in ('serum', 'ott'): continue
        if p.get('kind') == 'ott': want = 'ace9b275b03d330b'   # node-check.mjs of mt1 on ott-c.wasm, 650 blocks (logs/node-ott-650.txt)
        else:
            e = p['label'].split(':')[1]
            want = '8d120c2125062325' if e.endswith('s') else EXP.get(e)
        got = (p.get('hash') or '').replace('RESULT out_hash=', '')[:16]
        n += 1; ok += (got == want and r['blocks'] == 650)
    return ok, n
groups = collections.OrderedDict()
for f in sys.argv[2:]:
    for l in open(f):
        r = json.loads(l)
        if r.get('mode') == 'ring': continue
        if 'error' in r and 'serum' not in r:
            continue
        key = (r['hc'], r.get('cpus') or ('0-%d' % (r['hc'] - 1) if r['hc'] < 16 else 'all'), r['serum'], r['ott'], r['ottmod'] if r['ott'] else '', ('warm%dms' % r['warmms']) if r.get('warmms') else ('warm%db' % r['warmblocks'] if r.get('warmblocks') else 'warm150b') if r['warm'] else 'cold', r['mod'])
        groups.setdefault(key, []).append(r)

def rng(v, nd=2):
    v = [x for x in v if x is not None]
    if not v: return ''
    return ('%.*f' % (nd, min(v))) if min(v) == max(v) else ('%.*f-%.*f' % (nd, min(v), nd, max(v)))

hdr = ['hc', 'cpus', 'serum', 'ott', 'ottmod', 'warm_protocol', 'mod', 'reps', 'blocks_per_run', 'late_A1_per_rep', 'late_A1_total', 'late_A1_first5_blocks', 'late_A1_gt5ms', 'late_A1_gt20ms', 'late_A2_total', 'events_A1_per_rep', 'late_max_A1_ms', 'late_max_A2_ms',
       'first_blocks_late_A1_ms(rep0)', 'serum_proc_p50_ms(worst)', 'serum_proc_p99_ms(worst)', 'serum_proc_max_ms(worst)', 'ott_proc_p50_ms', 'ott_proc_max_ms', 'harness_overhead_ms', 'wake_mean_ms',
       'cores_total', 'cores_workers', 'cores_other', 'cores_mask_busy_all_procs', 'others_busy_cores_on_mask', 'foreign_busy_cores_mean/p90/max(per second, per rep)', 'worker_runqueue_wait_s_max_per_rep', 'sum_proc_mean_over_P(cores)', 'load_in', 'load_out', 'pss_MB', 'hc_reported', 'hash_match_of_650_block_node(serum+ott)']
rows = [hdr]
for key, rs in groups.items():
    hc, cpus, serum, ott, ottmod, warm, mod = key
    B = rs[0]['blocks']
    def worst(field, kind='serum'):
        v = []
        for r in rs:
            xs = [p[field] for p in r['per'] if p.get('kind') == kind and p.get(field) is not None]
            if xs: v.append(max(xs))
        return v
    sumproc = []
    for r in rs:
        sumproc.append(sum(p['procMean'] for p in r['per'] if p.get('procMean') is not None) / r['periodMs'])
    ho = [p['harnessOverheadMean'] for r in rs for p in r['per'] if p.get('kind') == 'serum' and p.get('harnessOverheadMean') is not None]
    wk = [p['wakeMean'] for r in rs for p in r['per'] if p.get('wakeMean') is not None]
    hashes = sorted({h for r in rs for h in r['hashes']})
    rows.append([hc, cpus, serum, ott, ottmod, warm, mod, len(rs), B,
                 '/'.join(str(r['lateA1']) for r in rs), sum(r['lateA1'] for r in rs), sum(r['lateA1_first5'] for r in rs), sum(r['lateA1_gt5ms'] for r in rs), sum(r['lateA1_gt20ms'] for r in rs), sum(r['lateA2'] for r in rs),
                 '/'.join(str(r['eventsA1']) for r in rs), '%.1f' % max(r['lateMaxA1'] for r in rs), '%.1f' % max(r['lateMaxA2'] for r in rs),
                 ' '.join('%.0f' % x for x in rs[0]['firstBlocksLateness']),
                 rng(worst('procP50')), rng(worst('procP99')), rng(worst('procMax')), rng(worst('procP50', 'ott')), rng(worst('procMax', 'ott')),
                 '%.2f' % (sum(ho) / len(ho)) if ho else '', '%.2f' % (sum(wk) / len(wk)) if wk else '',
                 rng([r['coresTotal'] for r in rs]), rng([r['coresWorkers'] for r in rs]), rng([r['coresOther'] for r in rs]), rng([r['coresMachineBusyOnMask'] for r in rs]), rng([r['coresMachineBusyOnMask'] - r['coresTotal'] for r in rs]), ' | '.join('%.1f/%.1f/%.1f' % (r['foreignMean'], r['foreignP90'], r['foreignMax']) if 'foreignMean' in r else '-' for r in rs), '/'.join('%.1f' % (max(r.get('workerRunDelayMs') or [0]) / 1000) for r in rs), rng(sumproc),
                 rng([float(r['load_in'].split()[0]) for r in rs], 1), rng([float(r['load_out'].split()[0]) for r in rs], 1), rng([r['pssMB'] for r in rs], 0), rs[0]['hcReported'], '%d/%d' % (sum(hash_ok(r)[0] for r in rs), sum(hash_ok(r)[1] for r in rs))])
import os
if os.environ.get('PER_REP'):
    h2 = ['name', 'hc', 'cpus', 'serum', 'ott', 'warm', 'load_in', 'load_out', 'late_A1', 'late_A1_first5', 'late_A1_gt5ms', 'late_A2', 'events_A1', 'late_max_A1_ms', 'others_busy_cores_on_mask', 'foreign_mean/p90/max', 'worker_rq_wait_s_max', 'cores_total', 'cores_workers', 'serum_proc_p50_ms(list)', 'serum_proc_max_ms(list)', 'ott_proc_p50_ms(max)', 'hash_ok']
    allr = [r for rs in groups.values() for r in rs]
    with open(os.environ['PER_REP'], 'w') as f:
        f.write('\t'.join(h2) + '\n')
        for r in allr:
            sp = [p for p in r['per'] if p.get('kind') == 'serum']; op = [p for p in r['per'] if p.get('kind') == 'ott']
            f.write('\t'.join(str(x) for x in [r['name'], r['hc'], r.get('cpus', ''), r['serum'], r['ott'], r['warm'], r['load_in'].split()[0], r['load_out'].split()[0], r['lateA1'], r['lateA1_first5'], r['lateA1_gt5ms'], r['lateA2'], r['eventsA1'], r['lateMaxA1'],
                '%.2f' % (r['coresMachineBusyOnMask'] - r['coresTotal']), ('%.1f/%.1f/%.1f' % (r['foreignMean'], r['foreignP90'], r['foreignMax'])) if 'foreignMean' in r else '-', '%.1f' % (max(r.get('workerRunDelayMs') or [0]) / 1000), r['coresTotal'], r['coresWorkers'], ' '.join('%.1f' % p['procP50'] for p in sp), ' '.join('%.0f' % p['procMax'] for p in sp),
                ('%.1f(%.0f)' % (sum(p['procP50'] for p in op) / len(op), max(p['procMax'] for p in op))) if op else '', '%d/%d' % hash_ok(r)]) + '\n')
with open(out, 'w') as f:
    for r in rows:
        f.write('\t'.join(str(x) for x in r) + '\n')
print(open(out).read())
