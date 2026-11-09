"""design aid (option S), N-way version of merge_tables.py: merge the dispatch tables of N modules into one TU 'common.c' that also
defines R, FL, X.
usage: merge_tables_n.py OUT_common.c A-tables.txt B-tables.txt [C-tables.txt ...] [--extra-rows FILE ...]   (OUT must not exist; at least two tables)
--extra-rows FILE (optional, may be repeated; only the stage-1 harness of CreateFruityInstance uses it, lift/harness/fruity-rows.txt): one row per
line, "<0x VA> <C function name>", '#' starts a comment. The rows are added to the merged dispatch table; the function is declared like the others
and must be defined by the harness. A VA must lie in the pseudo-VA region (>= 0x7FF900000000), must not be a VA that a table already has, and a
name must be a plain C identifier that is not an f_<hex> name. Without the option nothing below changes: the output and the printed lines are the
ones of the version without it.
Rows with the same VA must name the same function (the pseudo-VA rows of the response table are in every table). Besides that, a VA
below the response-table pseudo-VA region that appears in more than one table is a collision of f_<VA> between modules: it is counted,
printed, and the merge fails (the link would fail with a multiple definition). For two tables the output is byte-identical to the one
of merge_tables.py."""
import re, sys
RESP_BASE = 0x7FF900000000
args = sys.argv[1:]
extra_files = []
while '--extra-rows' in args:
    i = args.index('--extra-rows')
    assert i + 1 < len(args), '--extra-rows needs a file'
    extra_files.append(args[i + 1])
    del args[i:i + 2]
out_path = args[0]
tabs = args[1:]
assert len(tabs) >= 2, 'need at least two tables'
rows = {}
resp = None
pat = re.compile(r'^    \{ UINT64_C\((0x[0-9a-f]+)\), ([A-Za-z_0-9]+) \},$')
srcs = {}
per_table = []
for p in tabs:
    txt = open(p).read().split('\n')
    intab = False
    n_rows = 0
    for l in txt:
        if l.startswith('const fle_fn_entry fle_dispatch_table[]'): intab = True; continue
        if intab:
            m = pat.match(l)
            if m:
                va = int(m.group(1), 16)
                n_rows += 1
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
    per_table.append((p, n_rows))
dup = sum(1 for v in srcs.values() if v > 1)
coll = sorted(va for va, v in srcs.items() if v > 1 and va < RESP_BASE)
extra_rows = []
for p in extra_files:
    for no, l in enumerate(open(p).read().split('\n'), 1):
        t = l.split('#', 1)[0].strip()
        if not t:
            continue
        parts = t.split()
        assert len(parts) == 2, '%s:%d: want "<0x VA> <name>"' % (p, no)
        assert re.fullmatch(r'0x[0-9a-fA-F]+', parts[0]), '%s:%d: bad VA %r' % (p, no, parts[0])
        va = int(parts[0], 16)
        name = parts[1]
        assert re.fullmatch(r'[A-Za-z_][A-Za-z_0-9]*', name), '%s:%d: bad function name %r' % (p, no, name)
        assert not re.fullmatch(r'f_[0-9a-f]+', name), '%s:%d: %r looks like a lifted function name' % (p, no, name)
        assert va >= RESP_BASE, '%s:%d: VA %#x is below the pseudo-VA region %#x' % (p, no, va, RESP_BASE)
        assert va not in rows, '%s:%d: VA %#x is already a row (%s)' % (p, no, va, rows[va])
        assert name not in rows.values(), '%s:%d: name %s is already a row' % (p, no, name)
        rows[va] = name
        extra_rows.append((va, name))
word = {2: 'two', 3: 'three'}.get(len(tabs), str(len(tabs)))
for p, n in per_table:
    print('table rows', n, p)
print('merged rows', len(rows), 'rows present in more than one table', dup)
if extra_files:
    print('extra rows (--extra-rows, harness stubs):', len(extra_rows), ' '.join('%#x=%s' % r for r in extra_rows))
print('f_<VA> collisions (VA below the pseudo-VA region present in more than one table):', len(coll))
if coll:
    for va in coll[:20]: print('  collision', hex(va), rows[va])
    sys.exit('merge_tables_n: f_<VA> collision between modules')
with open(out_path, 'x') as f:
    f.write('/* design aid: merged dispatch table of %s modules */\n#include "fle_runtime.h"\n#include "fle_sse.h"\n#include "fle_dispatch.h"\n#include "fle_modules.h"\n\n' % word)
    f.write('uint64_t R[16];\nfle_eflags FL = { FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF };\nfle_xmm X[16];\n\n')
    for va in sorted(rows):
        f.write('void %s(void);\n' % rows[va])
    f.write('\nconst fle_fn_entry fle_dispatch_table[] = {\n')
    for va in sorted(rows):
        f.write('    { UINT64_C(%#x), %s },\n' % (va, rows[va]))
    f.write('};\nconst uint64_t fle_dispatch_count = %d;\n\n' % len(rows))
    f.write(resp + '\n')
