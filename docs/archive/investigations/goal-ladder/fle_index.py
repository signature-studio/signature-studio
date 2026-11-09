# Build a per-function index of a PE (default FLEngine_x64.dll) with capstone. Read-only on the PE.
# usage: PYTHONDONTWRITEBYTECODE=1 python3 fle_index.py PE.dll OUT.pkl
# Index (pickle): dict with
#   base, funcs: sorted list of (begin_va, end_va) from .pdata,
#   thunks: {thunk_va: "dll!name"}  (jmp [rip+IAT] stubs found by byte scan of executable sections),
#   calls:  {func_begin: set(target_va)}      direct call/jmp rel32 targets (any section)
#   iatcalls: {func_begin: set("dll!name")}   call/jmp [rip+IAT] direct to an import slot
#   leas:   {func_begin: set(target_va)}      lea r,[rip+X] targets
#   rips:   {func_begin: set(target_va)}      every rip-relative operand target (incl. lea/mov)
# Nothing is executed. Needs capstone and pefile.
import sys, struct, re, pickle, bisect
import pefile, capstone

pe_path, out = sys.argv[1], sys.argv[2]
pe = pefile.PE(pe_path)
base = pe.OPTIONAL_HEADER.ImageBase
img = pe.get_memory_mapped_image()

iat = {}
for kind, ents in (("iat", getattr(pe, "DIRECTORY_ENTRY_IMPORT", [])),):
    for d in ents:
        dll = d.dll.decode().lower()
        for i in d.imports:
            iat[i.address] = dll + "!" + (i.name.decode() if i.name else "ord%d" % i.ordinal)
delay = {}
for d in getattr(pe, "DIRECTORY_ENTRY_DELAY_IMPORT", []):
    dll = d.dll.decode().lower()
    for i in d.imports:
        delay[i.address] = dll + "!" + (i.name.decode() if i.name else "ord%d" % i.ordinal) + "(delay)"

ex = pe.OPTIONAL_HEADER.DATA_DIRECTORY[3]
funcs = sorted((base + b, base + e) for b, e, u in struct.iter_unpack("<III", img[ex.VirtualAddress:ex.VirtualAddress + ex.Size]) if b)
execs = [(base + s.VirtualAddress, base + s.VirtualAddress + s.Misc_VirtualSize) for s in pe.sections if s.Characteristics & 0x20000000]

thunks = {}
for lo, hi in execs:
    lo_o, hi_o = lo - base, hi - base
    pos = lo_o
    blob = img[lo_o:hi_o]
    p = blob.find(b"\xff\x25")
    while p != -1:
        if p + 6 <= len(blob):
            t = base + lo_o + p + 6 + struct.unpack_from("<i", blob, p + 2)[0]
            if t in iat:
                thunks[base + lo_o + p] = iat[t]
            elif t in delay:
                thunks[base + lo_o + p] = delay[t]
        p = blob.find(b"\xff\x25", p + 1)

md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
md.skipdata = True
rx_rip = re.compile(r"\[rip ([+-]) (0x[0-9a-f]+)\]")
calls, iatcalls, leas, rips = {}, {}, {}, {}
n = 0
for (b, e) in funcs:
    code = img[b - base:e - base]
    c, ic, lz, rp = set(), set(), set(), set()
    for addr, size, mn, ops in md.disasm_lite(code, b):
        n += 1
        if mn in ("call", "jmp"):
            if ops.startswith("0x"):
                try:
                    c.add(int(ops, 16))
                except ValueError:
                    pass
                continue
        if "rip" in ops:
            m = rx_rip.search(ops)
            if m:
                t = addr + size + (int(m.group(2), 16) if m.group(1) == "+" else -int(m.group(2), 16))
                rp.add(t)
                if mn == "lea":
                    lz.add(t)
                if mn in ("call", "jmp"):
                    nm = iat.get(t) or delay.get(t)
                    if nm:
                        ic.add(nm)
    calls[b], iatcalls[b], leas[b], rips[b] = c, ic, lz, rp
pickle.dump(dict(base=base, funcs=funcs, thunks=thunks, calls=calls, iatcalls=iatcalls, leas=leas, rips=rips), open(out, "wb"), protocol=4)
print("functions", len(funcs), "thunks", len(thunks), "instructions", n)
