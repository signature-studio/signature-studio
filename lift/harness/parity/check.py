#!/usr/bin/env python3
"""parity gate checker (decision 0226). usage: check.py OUTDIR THRESHOLDS.tsv [--native]
Reads OUTDIR/matrix-<session>.tsv (summarize.py columns when --native, else wasm-table.py columns) and expected/thresholds.tsv:
  session  op  max_ratio  max_cpu_per_vs  max_cpu_ms  note
  (`*` matches any session/op; the most specific row wins; max_ratio = wasm CPU / native CPU; max_cpu_per_vs = wasm CPU seconds per virtual second; max_cpu_ms = wasm CPU of a discrete op)
FAIL: a pixel difference between native and wasm (surfaces_equal != yes), a ratio / per-vs / ms above its threshold. WARN (not a failure): above the targets of decision 0226 (ratio 1.2, 1.0 CPU s per virtual s for continuous ops).
Rows of the preludes and of the calibration (cal_snap, wiz0, e03, opened, end) are listed but not checked against the targets."""
import csv, glob, os, sys
out, thr_path = sys.argv[1], sys.argv[2]; native = '--native' in sys.argv
thr = {}
for ln in open(thr_path, encoding='utf-8'):
    if ln.startswith('#') or not ln.strip(): continue
    f = ln.rstrip('\n').split('\t') + [''] * 6
    thr[(f[0], f[1])] = (f[2], f[3], f[4])
def lookup(s, op):
    for k in ((s, op), (s, '*'), ('*', op), ('*', '*')):
        if k in thr: return thr[k]
    return ('', '', '')
def num(x):
    try: return float(x)
    except Exception: return None
fails = warns = checked = 0
for p in sorted(glob.glob(os.path.join(out, 'matrix-*.tsv'))):
    s = os.path.basename(p)[len('matrix-'):-4]
    for r in csv.DictReader(open(p), delimiter='\t'):
        op = r['op']
        if op in ('wiz0', 'e03', 'opened', 'end', 'cal_snap'): continue
        checked += 1
        mr, mv, mm = (num(x) for x in lookup(s, op))
        kind = r.get('kind', '')
        cpu = num(r.get('wasm_main_cpu_ms')); vms = num(r.get('virtual_ms')); ratio = num(r.get('ratio_main'))
        pv = (cpu / vms) if cpu is not None and vms else None
        if native and r.get('surfaces_equal') not in ('yes',):
            print('FAIL %s %s: pixels differ between native and wasm' % (s, op)); fails += 1
        if native and ratio is not None and mr is not None and ratio > mr:
            print('FAIL %s %s: wasm/native CPU %.2f > %.2f' % (s, op, ratio, mr)); fails += 1
        if pv is not None and mv is not None and pv > mv and kind == 'C':
            print('FAIL %s %s: %.3f CPU s per virtual s > %.3f' % (s, op, pv, mv)); fails += 1
        if mm is not None and cpu is not None and kind == 'D' and cpu > mm:
            print('FAIL %s %s: %.0f ms > %.0f ms' % (s, op, cpu, mm)); fails += 1
        if native and ratio is not None and ratio > 1.2: warns += 1
        if pv is not None and kind == 'C' and pv > 1.0: print('WARN %s %s: %.2f CPU s per virtual s (target <= 1.0)' % (s, op, pv))
print('parity check: %d rows, %d FAIL, %d rows above the 1.2x target' % (checked, fails, warns))
sys.exit(1 if fails else 0)
