#!/usr/bin/env python3
"""parity1: wasm-only per-op table (no native binary): op, virtual_ms, wasm_main_cpu_ms, wasm_cpu_per_vs, progress. usage: wasm-table.py WASM_STDOUT   (TSV on stdout, same first columns as summarize.py's)"""
import re, sys
prev = cur = None
print('\t'.join(['op', 'virtual_ms', 'wasm_main_cpu_ms', 'wasm_cpu_per_vs', 'wasm_progress']))
for ln in open(sys.argv[1], errors='replace'):
    m = re.match(r'g3-snap: #\d+ (\S+) clock_ms (\d+)', ln)
    if m: cur = {'n': m.group(1), 'clock': int(m.group(2))}; continue
    m = re.match(r'parity1-maincpu: ([\d.]+) s', ln)
    if m and cur: cur['cpu'] = float(m.group(1)) * 1000; continue
    m = re.match(r'g3-progress: #\d+ (\S+) progress (\d+)', ln)
    if m and cur:
        cur['prog'] = int(m.group(2))
        if prev:
            v = cur['clock'] - prev['clock']; c = cur['cpu'] - prev['cpu']
            print('\t'.join([cur['n'], str(v), '%.0f' % c, ('%.3f' % (c / v)) if v else '-', str(cur['prog'] - prev['prog'])]))
        prev = cur
