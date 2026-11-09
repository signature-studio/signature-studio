"""Measurement aid (read-only): which CPU-selector variable (decision 0079: four .data variables, file value 0xffffffff) does each export read?
For every export of the PE (file given, any base) the first 32 bytes at the export VA are disassembled and the first `movsxd r64, dword ptr [rip+X]`
is taken: X is the selector variable of that stub. An export that is not such a stub is reported as 'other'.
With --gpa LOG (a gdb_imports.py log made with GDBI_GPA=1) only the exports that the guest obtained with GetProcAddress (hModule = base)
are counted. usage: selector_users.py PE.dll [--gpa LOG] [--names FILE]"""
import re
import struct
import sys

import pefile
from capstone import CS_ARCH_X86, CS_MODE_64, Cs

dll = sys.argv[1]
gpa = sys.argv[sys.argv.index("--gpa") + 1] if "--gpa" in sys.argv else None
pe = pefile.PE(dll, fast_load=True)
pe.parse_data_directories(directories=[pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_EXPORT"]])
base = pe.OPTIONAL_HEADER.ImageBase
img = pe.get_memory_mapped_image()
md = Cs(CS_ARCH_X86, CS_MODE_64)
md.detail = False

exports = {}
for s in pe.DIRECTORY_ENTRY_EXPORT.symbols:
    if s.name:
        exports[s.name.decode()] = base + s.address


def selector_of(va):
    o = va - base
    # the export VA is the init call (16 bytes before the movsxd) or the movsxd itself: scan 32 bytes
    for x in md.disasm(img[o:o + 32], va):
        if x.mnemonic == "movsxd" and "rip +" in x.op_str and "dword ptr" in x.op_str:
            m = re.search(r"rip \+ (0x[0-9a-f]+)", x.op_str)
            return x.address + x.size + int(m.group(1), 16)
    return None


if gpa:
    vals = set()
    for l in open(gpa):
        m = re.match(r"\s+-> GetProcAddress #(\d+) returned R\[0\]=(0x[0-9a-f]+)", l)
        if m:
            vals.add(int(m.group(2), 16))
    sel = {n: v for n, v in exports.items() if v in vals}
    print("guest GetProcAddress results on the module: %d distinct VAs, %d of them are named exports" % (len(vals), len(sel)))
else:
    sel = exports
cnt = {}
ex = {}
for n, v in sorted(sel.items()):
    s = selector_of(v)
    key = "other" if s is None else "%#x" % s
    cnt[key] = cnt.get(key, 0) + 1
    ex.setdefault(key, []).append(n)
print("selector variable -> number of exports (%s)" % ("guest-requested" if gpa else "all exports"))
for k, v in sorted(cnt.items(), key=lambda kv: -kv[1]):
    print("  %-12s %4d   e.g. %s" % (k, v, ", ".join(ex[k][:4])))
if "--names" in sys.argv:
    for k in ex:
        if k != "other":
            print(k, ",".join(ex[k]))
