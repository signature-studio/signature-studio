#!/usr/bin/env python3
"""srm1 (decision 0177): the JSON rows printed by run-serum.mjs (one per line, or the *-summary.json files) -> a TSV.
usage: summarize.py OUT.tsv FILE...   (a FILE is a text file of JSON lines or a summary .json list)"""
import json, sys
rows = []
for fn in sys.argv[2:]:
    t = open(fn).read().strip()
    if t.startswith('['):
        rows += json.loads(t)
    else:
        for l in t.splitlines():
            l = l.strip()
            if l.startswith('{'):
                rows.append(json.loads(l))
cols = ['name', 'mod', 'n', 'state', 'blocks', 'load_start', 'load_end', 'perBlockMin', 'perBlockP50', 'perBlockMax', 'block0', 'hash0', 'hashesEqual', 'compileMs', 'instMs', 'runMs', 'dllmain_ms', 'dllmain_to_setActive_ms',
        'memMiB_end', 'rssMB_before_run', 'rssMB_after_run', 'pssMB_before_run', 'pssMB_after_run', 'workerCpuMs']
def stage(r, pat, idx=0):
    import re
    try:
        m = re.search(pat, ' '.join(r['stage'][idx]))
        return m.group(1) if m else ''
    except Exception:
        return ''
with open(sys.argv[1], 'w') as f:
    f.write('\t'.join(cols) + '\n')
    for r in rows:
        r = dict(r)
        r['hash0'] = (r.get('hash') or [''])[0].replace('RESULT out_hash=', '').split(' ')[0]
        r['dllmain_ms'] = stage(r, r'DllMain ran inside\)\s+cpu ([\d.]+) ms')
        r['dllmain_to_setActive_ms'] = stage(r, r'DllMain\.\.setActive cpu ([\d.]+) ms')
        r['memMiB_end'] = ','.join(str(round(x / 1048576)) for x in r.get('memBytesEnd', []))
        f.write('\t'.join(json.dumps(r.get(c, '')) if isinstance(r.get(c, ''), (list, dict)) else str(r.get(c, '')) for c in cols) + '\n')
print(open(sys.argv[1]).read())
