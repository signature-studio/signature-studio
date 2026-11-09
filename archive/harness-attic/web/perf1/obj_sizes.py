"""perf1 (decision 0130): the code-section bytes of every wasm object (.o) of a build, summed per group (FLEngine units fle-*, the five real modules m1..m5, common.c, the runtime fle_*.c, the harness, the o1 split units).
usage: obj_sizes.py OBJDIR [--json OUT]   (OBJDIR = build_nmod_wasm.sh OBJ: mod/ fle/ rt/ h-fruity/ o1/obj/). The object file's code section is the function bodies before linking (relocations are not counted)."""
import json, mmap, os, sys, glob
def uleb(d, p):
    r = s = 0
    while True:
        b = d[p]; p += 1; r |= (b & 0x7f) << s; s += 7
        if not b & 0x80: return r, p
def code_bytes(path):
    f = open(path, 'rb'); d = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
    p = 8; code = data = 0
    while p < len(d):
        sid = d[p]; p += 1; sz, p = uleb(d, p)
        if sid == 10: code = sz
        if sid == 11: data = sz
        p += sz
    return code, data, len(d)
obj = sys.argv[1]
groups = {}
def add(g, path):
    c, dt, n = code_bytes(path)
    e = groups.setdefault(g, {'objects': 0, 'code': 0, 'data': 0, 'file': 0}); e['objects'] += 1; e['code'] += c; e['data'] += dt; e['file'] += n
for p in sorted(glob.glob(obj + '/fle/*.o')): add('FLEngine (fle-*.c)', p)
for p in sorted(glob.glob(obj + '/mod/*.o') + glob.glob(obj + '/o1/obj/*.o')):
    b = os.path.basename(p)
    k = b[:2] if b[0] == 'm' and b[1].isdigit() else ('fn' if b.startswith('fn-') else b)
    if b.startswith('fn-') or b == 'common.o': add('common.o (merged dispatch table)' if b == 'common.o' else 'o1 split units (fn-*; the 63 functions over 50,000 locals)', p)
    else: add({'m1': 'm1 dsp_ippv2', 'm2': 'm2 Blend2D', 'm3': 'm3 QuickFontCache', 'm4': 'm4 freetype', 'm5': 'm5 ILTools'}.get(k, k), p)
for p in sorted(glob.glob(obj + '/rt/*.o')): add('runtime (lift/runtime/fle_*.c)', p)
for p in sorted(glob.glob(obj + '/h-fruity/*.o')): add('harness (h-fruity/)', p)
tot = sum(g['code'] for g in groups.values())
for k, g in groups.items(): print('%-62s %4d objects  code %12d B  %5.1f %%   data %9d B' % (k, g['objects'], g['code'], 100.0 * g['code'] / tot, g['data']))
print('total code in objects', tot)
if len(sys.argv) > 3 and sys.argv[2] == '--json': json.dump({'groups': groups, 'total_code': tot}, open(sys.argv[3], 'w'), indent=1)
