#!/usr/bin/env python3
"""A5 oracle for dsp_ippv2_x64.dll: the pure-function selection with the resolved indirect jmps exempted.

select_pure.py excludes every function with an indirect jmp (and every caller of one). The DLL's CPU-dispatch stubs
(decision 0079, 434 `jmp [r10+rax*8]`) and MSVC switches (decision 0089/0094, 5 `jmp reg`) are indirect jmps that the
lifter resolves to a `switch`. This script repeats select_pure.analyze with ONE difference: an indirect jmp whose address
is in the exempt set (the `switch (t)` sites of the generated C, dsp_stubs.py) is not a reason. Everything else is the
same as select_pure.py (IAT, indirect call, other indirect jmp, x87, FS/GS, cpuid etc., direct targets that are not lifted
functions, and the transitive closure over direct calls/jumps). A stub's own targets are NOT direct targets, so the stub
function itself is listed here; whether a run through it is a comparison of pure code depends on the selector value (the
table entry must itself be in the plain pure set), which dsp_run_stubs.py checks per (stub, selector value).
usage: dsp_select_ext.py PE GEN.c STUBS.json OUT.json
"""
import json
import multiprocessing as mp
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import select_pure as sp  # noqa: E402
from capstone import CS_OP_IMM, CS_OP_MEM, CS_GRP_JUMP, CS_GRP_CALL  # noqa: E402
from capstone.x86 import X86_GRP_FPU, X86_REG_RIP, X86_REG_GS, X86_REG_FS, X86_REG_INVALID  # noqa: E402

EXEMPT = set()


def init(pe, exempt):
    sp.init(pe)
    EXEMPT.update(exempt)


def analyze(fn):
    lo, hi = fn
    md, code, base = sp.G["md"], sp.G["code"], sp.G["base"]
    imp_lo, imp_hi = sp.G["imp"]
    buf = code[lo - base: hi - base]
    bad = None
    targets = set()
    n = 0
    exempted = 0
    for ins in md.disasm(buf, lo):
        n += 1
        mn = ins.mnemonic
        if mn in sp.BAD_MNEM:
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
                if is_jmp and ins.address in EXEMPT:
                    exempted += 1
                else:
                    bad = bad or ("indirect_call" if is_call else "indirect_jmp")
            if (is_call or is_jmp) and op.type == CS_OP_IMM:
                t = op.imm
                if not (lo <= t < hi) or is_call:
                    targets.add(t)
    if n == 0:
        bad = bad or "undecodable"
    return (lo, hi, n, bad, sorted(targets), exempted)


def main():
    pe, gen, stubs, out_path = sys.argv[1:5]
    st = json.load(open(stubs))
    exempt = {s["site"] for s in st["stubs"]} | {m["site"] for m in st["msvc"]}
    d, base, dd, secs = sp.load_pe(pe)
    funcs = sp.pdata_functions(d, base, dd, secs)
    lifted = sp.lifted_set(gen)
    cand = [f for f in funcs if f[0] in lifted]
    with mp.Pool(4, initializer=init, initargs=(pe, exempt)) as pool:
        rows = pool.map(analyze, cand, chunksize=64)
    info = {r[0]: r for r in rows}
    bad = {r[0]: r[3] for r in rows if r[3]}
    for lo, hi, n, b, tg, ex in rows:
        if not b and any(t not in info for t in tg):
            bad[lo] = "target_not_function"
    rev = {}
    for lo, hi, n, b, tg, ex in rows:
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
    stub_fns = {s["fn"] for s in st["stubs"]}
    msvc_fns = {m["fn"] for m in st["msvc"]}
    res = {
        "pe": pe,
        "counts": {"candidates": len(cand), "pure": len(pure), "excluded_by_reason": reasons,
                   "exempt_sites": len(exempt), "stub_functions": len(stub_fns), "stub_functions_pure_ext": len(stub_fns & {r[0] for r in pure}),
                   "msvc_functions": len(msvc_fns), "msvc_functions_pure_ext": len(msvc_fns & {r[0] for r in pure}),
                   "msvc_functions_not_pure_ext": {hex(f): bad.get(f) for f in sorted(msvc_fns) if f in bad},
                   "callers_of_exempt_functions_pure_ext": None},
        "functions": [{"va": "0x%x" % r[0], "end": "0x%x" % r[1], "insns": r[2], "calls": ["0x%x" % t for t in r[4]]}
                      for r in pure],
    }
    with open(out_path, "x") as fh:
        json.dump(res, fh, indent=1)
    print(json.dumps(res["counts"], default=str))


if __name__ == "__main__":
    main()
