import sys, struct, bisect, pefile
# usage: c1leaall.py DLL : candidates = targets of `lea r64,[rip+d32]` found by a RAW byte scan of every executable section
# (so the leaf functions without .pdata are covered too) that lie in an executable section outside every .pdata function,
# are 16-byte aligned and have an int3 (0xCC) immediately before them (function padding). Prints "VA nsources first-source".
pe = pefile.PE(sys.argv[1], fast_load=True)
base = pe.OPTIONAL_HEADER.ImageBase
img = pe.get_memory_mapped_image()
text = [(s.VirtualAddress, s.VirtualAddress + s.Misc_VirtualSize) for s in pe.sections if s.Characteristics & 0x20000000]
def in_text(r): return any(a <= r < b for a, b in text)
funcs = []
for s in pe.sections:
    if s.Name.rstrip(b'\0') == b'.pdata':
        for o in range(s.VirtualAddress, s.VirtualAddress + s.Misc_VirtualSize - 11, 12):
            b, e, u = struct.unpack_from('<III', img, o)
            if b: funcs.append((b, e))
funcs.sort(); starts = [f[0] for f in funcs]
def in_func(r):
    k = bisect.bisect_right(starts, r) - 1
    return k >= 0 and funcs[k][0] <= r < funcs[k][1]
found = {}
for a, z in text:
    for o in range(a, z - 7):
        if img[o] in (0x48, 0x4c) and img[o+1] == 0x8d and (img[o+2] & 0xC7) == 0x05:
            d = struct.unpack_from('<i', img, o+3)[0]
            t = o + 7 + d
            if in_text(t) and not in_func(t) and t % 16 == 0 and img[t-1] == 0xCC:
                found.setdefault(t, []).append(o)
for t in sorted(found):
    print("%#x %d %#x" % (base + t, len(found[t]), base + found[t][0]))
