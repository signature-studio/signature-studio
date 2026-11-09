"""One-module counterpart of lift/harness/two/merge_tables_n.py (which needs at least two tables): writes common.c, the one translation unit that
defines R, FL, X, the dispatch table and the response table of a single lifted PE (decision 0156).
usage: mk_common.py OUT_common.c IN_gen.c [--extra-rows FILE ...]     (OUT must not exist)
IN_gen.c is the whole file fe-lift wrote; the table section is the text between `const fle_fn_entry fle_dispatch_table[]` and the first function.
--extra-rows FILE: lines "<0x VA> <C function name>" (host objects of the harness, same rules as merge_tables_n.py: VA >= 0x7FF900000000)."""
import re, sys
RESP_BASE = 0x7FF900000000
args = sys.argv[1:]
extra_files = []
while '--extra-rows' in args:
    i = args.index('--extra-rows')
    extra_files.append(args[i + 1])
    del args[i:i + 2]
out_path, in_path = args
lines = open(in_path).read().split('\n')
fd = re.compile(r'^void f_[0-9a-f]+\(void\)$')
first_def = next(i for i, l in enumerate(lines) if fd.match(l) and lines[i + 1] == '{')
table = next(i for i, l in enumerate(lines) if l.startswith('const fle_fn_entry fle_dispatch_table[]'))
txt = lines[table:first_def]
pat = re.compile(r'^    \{ UINT64_C\((0x[0-9a-f]+)\), ([A-Za-z_0-9]+) \},$')
rows = {}
intab = False
for l in txt:
    if l.startswith('const fle_fn_entry fle_dispatch_table[]'): intab = True; continue
    if intab:
        m = pat.match(l)
        if m: rows[int(m.group(1), 16)] = m.group(2)
        elif l.startswith('};'): intab = False
i = txt.index('const fle_resp_entry fle_resp_table[] = {')
j = next(k for k in range(i, len(txt)) if txt[k].startswith('const uint64_t fle_resp_count'))
resp = '\n'.join(txt[i:j + 1])
extra = []
for p in extra_files:
    for no, l in enumerate(open(p).read().split('\n'), 1):
        t = l.split('#', 1)[0].strip()
        if not t: continue
        parts = t.split()
        assert len(parts) == 2, '%s:%d' % (p, no)
        va = int(parts[0], 16)
        assert va >= RESP_BASE and va not in rows and re.fullmatch(r'[A-Za-z_][A-Za-z_0-9]*', parts[1]) and not re.fullmatch(r'f_[0-9a-f]+', parts[1]), (p, no)
        rows[va] = parts[1]
        extra.append((va, parts[1]))
print('rows', len(rows), 'extra', len(extra))
with open(out_path, 'x') as f:
    f.write('/* decision 0156: single-module dispatch table */\n#include "fle_runtime.h"\n#include "fle_sse.h"\n#include "fle_dispatch.h"\n#include "fle_modules.h"\n\n')
    f.write('uint64_t R[16];\nfle_eflags FL = { FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF };\nfle_xmm X[16];\n\n')
    for va in sorted(rows): f.write('void %s(void);\n' % rows[va])
    f.write('\nconst fle_fn_entry fle_dispatch_table[] = {\n')
    for va in sorted(rows): f.write('    { UINT64_C(%#x), %s },\n' % (va, rows[va]))
    f.write('};\nconst uint64_t fle_dispatch_count = %d;\n\n' % len(rows))
    f.write(resp + '\n')
