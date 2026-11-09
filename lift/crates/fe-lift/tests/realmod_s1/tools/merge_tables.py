"""design aid (option S): merge the dispatch tables of two modules into one TU 'common.c' that also defines R, FL, X.
usage: merge_tables.py A-tables.txt B-tables.txt OUT_common.c   (OUT must not exist)
Rows with the same VA must name the same function (the pseudo-VA rows of the response table are in both)."""
import re, sys
rows = {}
resp = None
pat = re.compile(r'^    \{ UINT64_C\((0x[0-9a-f]+)\), ([A-Za-z_0-9]+) \},$')
srcs = {}
for p in sys.argv[1:3]:
    txt = open(p).read().split('\n')
    cur = []
    intab = False
    for l in txt:
        if l.startswith('const fle_fn_entry fle_dispatch_table[]'): intab = True; continue
        if intab:
            m = pat.match(l)
            if m:
                va = int(m.group(1), 16)
                if va in rows:
                    assert rows[va] == m.group(2), (hex(va), rows[va], m.group(2))
                    srcs[va] = srcs.get(va, 0) + 1
                else:
                    rows[va] = m.group(2); srcs[va] = 1
            elif l.startswith('};'):
                intab = False
    i = txt.index('const fle_resp_entry fle_resp_table[] = {')
    j = next(k for k in range(i, len(txt)) if txt[k].startswith('const uint64_t fle_resp_count'))
    r = '\n'.join(txt[i:j + 1])
    if resp is None: resp = r
    else: assert resp == r, 'response tables differ'
dup = sum(1 for v in srcs.values() if v > 1)
with open(sys.argv[3], 'x') as f:
    f.write('/* design aid: merged dispatch table of two modules */\n#include "fle_runtime.h"\n#include "fle_sse.h"\n#include "fle_dispatch.h"\n#include "fle_modules.h"\n\n')
    f.write('uint64_t R[16];\nfle_eflags FL = { FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF };\nfle_xmm X[16];\n\n')
    for va in sorted(rows):
        f.write('void %s(void);\n' % rows[va])
    f.write('\nconst fle_fn_entry fle_dispatch_table[] = {\n')
    for va in sorted(rows):
        f.write('    { UINT64_C(%#x), %s },\n' % (va, rows[va]))
    f.write('};\nconst uint64_t fle_dispatch_count = %d;\n\n' % len(rows))
    f.write(resp + '\n')
print('merged rows', len(rows), 'rows present in both', dup)
