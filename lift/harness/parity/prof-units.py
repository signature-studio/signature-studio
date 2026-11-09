#!/usr/bin/env python3
"""parity1: which translation units spend time in non-inlined runtime helpers (fle_ld, fle_st, fle_op_*, fle_nf_*, fle_cc, fle_i2_*, fle_dispatch ...) called from them, and their own self time. usage: prof-units.py INDEX.txt PROFILE.cpuprofile ..."""
import json, sys, re, collections
idx = dict(l.split() for l in open(sys.argv[1]) if len(l.split()) == 2)
helper = re.compile(r'^fle_(ld|st|ldx|stx|ldxa|stxa|op_\w+|nf_\w+|cc|i2_\w+|dispatch|mx_\w+|satu8|packuswb|pshuf\w*|padd\w*|psub\w*|pm\w+|punpck\w*)$')
unit_help = collections.Counter(); unit_self = collections.Counter(); tot = 0
for f in sys.argv[2:]:
    p = json.load(open(f)); by = {n['id']: n for n in p['nodes']}; par = {}
    for n in p['nodes']:
        for c in n.get('children', []): par[c] = n['id']
    self_us = collections.Counter()
    for i, s in enumerate(p['samples']): self_us[s] += p['timeDeltas'][i] if i < len(p['timeDeltas']) else 0
    for nid, us in self_us.items():
        name = by[nid]['callFrame']['functionName']
        if name == '(idle)': continue
        tot += us
        if name in idx: unit_self[idx[name]] += us
        elif helper.match(name):
            q = by.get(par.get(nid)); pn = q['callFrame']['functionName'] if q else ''
            if pn in idx: unit_help[idx[pn]] += us
            else: unit_help['(caller not a guest fn: %s)' % pn[:20]] += us
print('total non-idle %.2f s' % (tot / 1e6))
print('%-12s %10s %10s %10s' % ('unit', 'self ms', 'helper ms', 'sum %'))
for u in sorted(set(unit_self) | set(unit_help), key=lambda u: -(unit_self[u] + unit_help[u]))[:16]:
    print('%-12s %10.0f %10.0f %9.1f%%' % (u, unit_self[u] / 1000, unit_help[u] / 1000, 100.0 * (unit_self[u] + unit_help[u]) / tot))
