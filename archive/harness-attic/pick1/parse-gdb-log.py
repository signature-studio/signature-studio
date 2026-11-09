#!/usr/bin/env python3
"""pick1: parse a gdb-api.py log ("R|E CLOCK NAME key" rows, "#dump N" markers) into phases named by the g3-cpu lines of the run's stdout.
usage: parse-gdb-log.py RUNDIR   -> prints a list of (phase, kind, name, text, count) as TSV on stdout"""
import sys, re, collections
d = sys.argv[1]
ops = [m.group(1) for ln in open(d + '/stdout.txt', errors='replace') for m in [re.match(r'g3-cpu: (\S+) ', ln)] if m]
phase = 0; rows = collections.OrderedDict()
for ln in open(d + '/api.log', errors='replace'):
    if ln.startswith('#dump'):
        phase += 1; continue
    m = re.match(r'([RE]) (\d+) (\S+) (.*)$', ln.rstrip('\n'))
    if not m: continue
    kind, clk, name, txt = m.groups()
    ph = ops[phase] if phase < len(ops) else 'after-end'
    rows[(ph, kind, name, txt)] = rows.get((ph, kind, name, txt), 0) + 1
print('phase\tkind\tname\tcount\ttext')
for (ph, kind, name, txt), c in rows.items():
    print('%s\t%s\t%s\t%d\t%s' % (ph, kind, name, c, txt))
