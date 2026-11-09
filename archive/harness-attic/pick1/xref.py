#!/usr/bin/env python3
"""pick1: static cross references in FLEngine_x64.dll (image base 0x400000, VA = what the lifted f_<VA> names use).
usage: xref.py FILE.dll lea  VA...        : code locations with `lea reg,[rip+d]` whose target is VA (rex.W lea, 48/4c 8d modrm=05..3d)
       xref.py FILE.dll call VA...        : `call rel32` / `jmp rel32` locations whose target is VA
       xref.py FILE.dll str  TEXT...      : VAs of ASCII and UTF-16LE occurrences of TEXT
       xref.py FILE.dll func VA           : start of the function that contains VA (previous run of int3/nop padding, then the first instruction)
"""
import sys, re, struct, pefile
pe = pefile.PE(sys.argv[1], fast_load=True); base = pe.OPTIONAL_HEADER.ImageBase
d = open(sys.argv[1], 'rb').read()
def va_of_off(o):
    for s in pe.sections:
        if s.PointerToRawData <= o < s.PointerToRawData + s.SizeOfRawData: return base + s.VirtualAddress + o - s.PointerToRawData
def off_of_va(v):
    for s in pe.sections:
        a = base + s.VirtualAddress
        if a <= v < a + max(s.Misc_VirtualSize, s.SizeOfRawData): return s.PointerToRawData + v - a
text = [s for s in pe.sections if s.Name.startswith(b'.text')][0]
t0, t1 = text.PointerToRawData, text.PointerToRawData + text.SizeOfRawData
mode = sys.argv[2]; args = sys.argv[3:]
if mode == 'lea':
    want = {int(x, 0) for x in args}
    for m in re.finditer(rb'[\x48\x4c]\x8d[\x05\x0d\x15\x1d\x25\x2d\x35\x3d]', d[t0:t1], re.S):
        o = t0 + m.start(); disp = struct.unpack_from('<i', d, o + 3)[0]; va = va_of_off(o)
        if va + 7 + disp in want: print('lea  %#x -> %#x' % (va, va + 7 + disp))
elif mode == 'call':
    want = {int(x, 0) for x in args}
    for m in re.finditer(rb'[\xe8\xe9]', d[t0:t1], re.S):
        o = t0 + m.start(); disp = struct.unpack_from('<i', d, o + 1)[0]; va = va_of_off(o)
        if va + 5 + disp in want: print('%s %#x -> %#x' % ('call' if d[o] == 0xe8 else 'jmp ', va, va + 5 + disp))
elif mode == 'str':
    for a in args:
        for enc in ('ascii', 'utf-16le'):
            for m in re.finditer(re.escape(a.encode(enc)), d):
                print('%-9s %#x  %s' % (enc, va_of_off(m.start()) or 0, a))
elif mode == 'func':
    v = int(args[0], 0); o = off_of_va(v)
    while o > t0 and not (d[o - 1] in (0xcc, 0x90, 0x00) and d[o] not in (0xcc, 0x90, 0x00)): o -= 1
    print('function start %#x' % va_of_off(o))
