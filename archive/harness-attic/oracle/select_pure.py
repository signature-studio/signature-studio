#!/usr/bin/env python3
"""A5 oracle: select pure-compute functions of FLEngine_x64.dll from the disassembly.

A function (a .pdata RUNTIME_FUNCTION that is not a chained child, and is a lifted entry in
the all-pdata dispatch table) is "pure" when neither it nor any function reachable through its
direct calls/jumps contains one of:
  - an operand that references the import directories (.idata/.didata), i.e. an IAT call/jump
    or a read of an IAT slot
  - an indirect call or jump (register or memory target; this includes jump tables)
  - an x87 instruction
  - an fs:/gs: memory operand (LEA excepted: it only computes the offset)
  - cpuid, rdtsc(p), xgetbv, syscall, sysenter, int*, hlt, ud2, in/out, rdrand/rdseed
and every direct target is the start of a lifted function.

usage: select_pure.py PE DISPATCH_C OUT.json
"""
import json
import multiprocessing as mp
import re
import struct
import sys

from capstone import Cs, CS_ARCH_X86, CS_MODE_64, CS_OP_IMM, CS_OP_MEM, CS_GRP_JUMP, CS_GRP_CALL, CS_GRP_RET
from capstone.x86 import X86_GRP_FPU, X86_REG_RIP, X86_REG_GS, X86_REG_FS, X86_REG_INVALID

BAD_MNEM = {
    "cpuid", "rdtsc", "rdtscp", "xgetbv", "xsetbv", "syscall", "sysenter", "sysret", "int", "int1", "int3",
    "into", "hlt", "ud2", "in", "out", "insb", "insw", "insd", "outsb", "outsw", "outsd", "rdrand", "rdseed",
    "rdpmc", "rdmsr", "wrmsr", "lgdt", "lidt", "iretq", "iret", "wait", "fwait",
}


def load_pe(path):
    d = open(path, "rb").read()
    lf = struct.unpack_from("<I", d, 0x3C)[0]
    nsec = struct.unpack_from("<H", d, lf + 6)[0]
    osz = struct.unpack_from("<H", d, lf + 20)[0]
    opt = lf + 24
    base = struct.unpack_from("<Q", d, opt + 24)[0]
    dd = [struct.unpack_from("<II", d, opt + 112 + 8 * i) for i in range(16)]
    secs = []
    for i in range(nsec):
        s = d[opt + osz + 40 * i: opt + osz + 40 * i + 40]
        name = s[:8].rstrip(b"\0").decode()
        vs, va, rs, rp = struct.unpack_from("<IIII", s, 8)
        ch = struct.unpack_from("<I", s, 36)[0]
        secs.append((name, va, vs, rs, rp, ch))
    return d, base, dd, secs


def rva_off(secs, rva):
    for _n, va, vs, rs, rp, _c in secs:
        if va <= rva < va + max(vs, rs):
            return rp + rva - va if rva - va < rs else None
    return None


def pdata_functions(d, base, dd, secs):
    prva, psz = dd[3]
    po = rva_off(secs, prva)
    ents = [struct.unpack_from("<III", d, po + 12 * i) for i in range(psz // 12)]
    chained = set()
    for b, e, u in ents:
        uo = rva_off(secs, u & ~1)
        if (d[uo] >> 3) & 4:
            chained.add(b)
    return [(base + b, base + e) for b, e, u in ents if b not in chained]


def lifted_set(dispatch_c):
    pat = re.compile(rb"\{ UINT64_C\(0x([0-9a-f]+)\), f_[0-9a-f]+ \}")
    out = set()
    with open(dispatch_c, "rb") as fh:
        for line in fh:
            m = pat.search(line)
            if m:
                out.add(int(m.group(1), 16))
    return out


G = {}


def analyze(fn):
    lo, hi = fn
    md = G["md"]
    code = G["code"]
    base = G["base"]
    imp_lo, imp_hi = G["imp"]
    off = lo - base
    buf = code[off: hi - base]
    bad = None
    targets = set()
    n = 0
    for ins in md.disasm(buf, lo):
        n += 1
        mn = ins.mnemonic
        if mn in BAD_MNEM:
            bad = bad or mn
            continue
        if ins.group(X86_GRP_FPU):
            bad = bad or "x87:" + mn
            continue
        is_call = ins.group(CS_GRP_CALL)
        is_jmp = ins.group(CS_GRP_JUMP)
        for op in ins.operands:
            if op.type == CS_OP_MEM:
                m = op.mem
                if m.base == X86_REG_RIP:
                    t = ins.address + ins.size + m.disp
                    if imp_lo <= t < imp_hi:
                        bad = bad or "iat"
                elif m.base == X86_REG_INVALID and m.index == X86_REG_INVALID and imp_lo <= m.disp < imp_hi:
                    bad = bad or "iat"
                if op.mem.segment in (X86_REG_GS, X86_REG_FS) and mn != "lea":
                    bad = bad or "segment"
            if (is_call or is_jmp) and op.type != CS_OP_IMM:
                bad = bad or ("indirect_call" if is_call else "indirect_jmp")
            if (is_call or is_jmp) and op.type == CS_OP_IMM:
                t = op.imm
                if not (lo <= t < hi) or is_call:
                    targets.add(t)
    if n == 0:
        bad = bad or "undecodable"
    return (lo, hi, n, bad, sorted(targets))


def init(pe):
    d, base, dd, secs = load_pe(pe)
    md = Cs(CS_ARCH_X86, CS_MODE_64)
    md.detail = True
    md.skipdata = False
    imp = [(base + va, base + va + max(vs, rs)) for n, va, vs, rs, rp, c in secs if n in (".idata", ".didata")]
    if not imp and dd[12][0]:
        # no .idata/.didata (dsp_ippv2_x64.dll: the IAT sits inside .rdata): the IAT data directory (index 12)
        imp = [(base + dd[12][0], base + dd[12][0] + dd[12][1])]
    G["md"] = md
    G["base"] = base
    G["imp"] = (min(a for a, b in imp), max(b for a, b in imp))
    # code[] indexed by RVA so a function body is code[lo-base : hi-base]
    img = bytearray(max(va + max(vs, rs) for _n, va, vs, rs, _rp, _c in secs))
    for _n, va, vs, rs, rp, _c in secs:
        img[va: va + rs] = d[rp: rp + rs]
    G["code"] = bytes(img)


def main():
    pe, dispatch_c, out_path = sys.argv[1:4]
    d, base, dd, secs = load_pe(pe)
    funcs = pdata_functions(d, base, dd, secs)
    lifted = lifted_set(dispatch_c)
    cand = [f for f in funcs if f[0] in lifted]
    print("pdata starts %d, lifted %d, candidates %d" % (len(funcs), len(lifted), len(cand)), file=sys.stderr)
    with mp.Pool(16, initializer=init, initargs=(pe,)) as pool:
        rows = pool.map(analyze, cand, chunksize=64)
    info = {r[0]: r for r in rows}
    bad = {r[0]: r[3] for r in rows if r[3]}
    # a direct target that is not a lifted entry (or not a pdata start) taints the caller
    for lo, hi, n, b, tg in rows:
        if not b and any(t not in info for t in tg):
            bad[lo] = "target_not_function"
    rev = {}
    for lo, hi, n, b, tg in rows:
        for t in tg:
            rev.setdefault(t, []).append(lo)
    work = list(bad)
    while work:
        f = work.pop()
        for c in rev.get(f, []):
            if c not in bad:
                bad[c] = "callee:%x" % f
                work.append(c)
    reasons = {}
    for v in bad.values():
        k = v.split(":")[0] if v.startswith(("callee", "x87")) else v
        reasons[k] = reasons.get(k, 0) + 1
    pure = [r for r in rows if r[0] not in bad]
    out = {
        "pe": pe,
        "counts": {"candidates": len(cand), "pure": len(pure), "excluded_by_reason": reasons},
        "functions": [{"va": "0x%x" % r[0], "end": "0x%x" % r[1], "insns": r[2], "calls": ["0x%x" % t for t in r[4]]}
                      for r in pure],
    }
    with open(out_path, "w") as fh:
        json.dump(out, fh, indent=1)
    print(json.dumps(out["counts"]), file=sys.stderr)


if __name__ == "__main__":
    main()
