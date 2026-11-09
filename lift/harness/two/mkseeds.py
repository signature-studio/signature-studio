"""Measurement aid: make a --seeds file (one hex VA per line) for the real module from (a) the nonzero code pointers of a table of
the file (--table BEGIN END, read from the rebased PE image) and/or (b) VAs given on the command line (--va 0x...).
A table entry is kept when it lies in an executable section. Nothing is decided here: the output is an INPUT of fe-lift for a
measurement build, not a statement that these VAs are function entries beyond what the run proved (see the investigation record).
usage: mkseeds.py PE.dll OUT.txt [--table 0xBEGIN 0xEND]... [--va 0xVA]... [--from FILE]...
OUT must not exist. Existing seed files given with --from are merged (sorted, distinct)."""
import struct
import sys

import pefile

if len(sys.argv) < 3:
    sys.exit(__doc__)
dll, out = sys.argv[1], sys.argv[2]
pe = pefile.PE(dll, fast_load=True)
base = pe.OPTIONAL_HEADER.ImageBase
img = pe.get_memory_mapped_image()
execs = [(base + s.VirtualAddress, base + s.VirtualAddress + s.Misc_VirtualSize)
         for s in pe.sections if s.Characteristics & 0x20000000]


def in_code(v):
    return any(lo <= v < hi for lo, hi in execs)


vals = set()
a = sys.argv[3:]
i = 0
while i < len(a):
    if a[i] == "--table":
        b, e = int(a[i + 1], 16), int(a[i + 2], 16)
        for va in range(b, e, 8):
            v = struct.unpack_from("<Q", img, va - base)[0]
            if v and in_code(v):
                vals.add(v)
        i += 3
    elif a[i] == "--va":
        vals.add(int(a[i + 1], 16))
        i += 2
    elif a[i] == "--from":
        for l in open(a[i + 1]).read().split():
            vals.add(int(l, 16))
        i += 2
    else:
        sys.exit("bad argument " + a[i])
with open(out, "x") as f:
    for v in sorted(vals):
        f.write("0x%x\n" % v)
print(len(vals), "seeds ->", out)
