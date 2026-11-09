"""usage: unanswered.py FILE imports_table.rs [extra-imports.txt]   imports of the PE that have no row in TABLE (nor in the extra file). Read only."""
import sys, re, pefile
rows = set()
for l in open(sys.argv[2]):
    m = re.match(r'\s*\("([^"]+)",\s*"([^"]+)",\s*"fle_imp_', l)
    if m: rows.add((m.group(1).lower(), m.group(2)))
if len(sys.argv) > 3:
    for l in open(sys.argv[3]):
        t = l.split('#', 1)[0].split()
        if len(t) >= 2: rows.add((t[0].lower(), t[1]))
pe = pefile.PE(sys.argv[1], fast_load=True); pe.parse_data_directories(directories=[pefile.DIRECTORY_ENTRY['IMAGE_DIRECTORY_ENTRY_IMPORT']])
tot = 0; miss = {}
for e in pe.DIRECTORY_ENTRY_IMPORT:
    d = e.dll.decode().lower()
    for imp in e.imports:
        tot += 1
        n = imp.name.decode() if imp.name else 'ord%d' % imp.ordinal
        if (d, n) not in rows: miss.setdefault(d, []).append(n)
print('imports', tot, 'unanswered', sum(len(v) for v in miss.values()))
for d, v in sorted(miss.items()):
    print(d, len(v), ' '.join(v))
