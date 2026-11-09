#!/usr/bin/env python3
"""A5 oracle v2: why functions are not in the pure-compute pool (decision 0036).

select_pure.py records the first reason found per function. This script records every own reason
of every candidate (a .pdata start that is a lifted entry) and the reasons reached through direct
calls/jumps, so the exclusion classes can be counted as overlapping sets and as a disjoint split.

own reasons: iat_call (direct call/jmp to a `jmp [rip+IAT]` thunk), iat_operand (other operand
into .idata/.didata), indirect_call, indirect_jmp, x87, segment (fs/gs operand), special (cpuid,
rdtsc, int, ...), other_target (direct target that is neither a lifted function nor an IAT thunk).
usage: exclude_stats.py PE DISPATCH_C OUT.json
"""
import json
import multiprocessing as mp
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import select_pure as sp  # noqa: E402
from capstone import CS_OP_IMM, CS_OP_MEM, CS_GRP_JUMP, CS_GRP_CALL  # noqa: E402
from capstone.x86 import X86_GRP_FPU, X86_REG_RIP, X86_REG_GS, X86_REG_FS, X86_REG_INVALID  # noqa: E402

REASONS = ["iat_call", "iat_operand", "indirect_call", "indirect_jmp", "x87", "segment", "special", "other_target"]
BIT = {r: 1 << i for i, r in enumerate(REASONS)}


def analyze(fn):
    lo, hi = fn
    md, code, base = sp.G["md"], sp.G["code"], sp.G["base"]
    imp_lo, imp_hi = sp.G["imp"]
    buf = code[lo - base: hi - base]
    mask = 0
    targets = set()
    for ins in md.disasm(buf, lo):
        mn = ins.mnemonic
        if mn in sp.BAD_MNEM:
            mask |= BIT["special"]
            continue
        if ins.group(X86_GRP_FPU):
            mask |= BIT["x87"]
            continue
        is_call, is_jmp = ins.group(CS_GRP_CALL), ins.group(CS_GRP_JUMP)
        for op in ins.operands:
            if op.type == CS_OP_MEM:
                m = op.mem
                if m.base == X86_REG_RIP:
                    t = ins.address + ins.size + m.disp
                    if imp_lo <= t < imp_hi:
                        mask |= BIT["iat_operand"]
                elif m.base == X86_REG_INVALID and m.index == X86_REG_INVALID and imp_lo <= m.disp < imp_hi:
                    mask |= BIT["iat_operand"]
                if m.segment in (X86_REG_GS, X86_REG_FS) and mn != "lea":
                    mask |= BIT["segment"]
            if (is_call or is_jmp) and op.type != CS_OP_IMM:
                mask |= BIT["indirect_call"] if is_call else BIT["indirect_jmp"]
            if (is_call or is_jmp) and op.type == CS_OP_IMM:
                t = op.imm
                if not (lo <= t < hi) or is_call:
                    targets.add(t)
    return lo, mask, sorted(targets)


def is_thunk(t):
    code, base = sp.G["code"], sp.G["base"]
    off = t - base
    if off < 0 or off + 6 > len(code) or code[off] != 0xFF or code[off + 1] != 0x25:
        return False
    disp = struct.unpack_from("<i", code, off + 2)[0]
    slot = t + 6 + disp
    return sp.G["imp"][0] <= slot < sp.G["imp"][1]


def main():
    pe, dispatch_c, out_path = sys.argv[1:4]
    d, base, dd, secs = sp.load_pe(pe)
    funcs = sp.pdata_functions(d, base, dd, secs)
    lifted = sp.lifted_set(dispatch_c)
    cand = [f for f in funcs if f[0] in lifted]
    with mp.Pool(16, initializer=sp.init, initargs=(pe,)) as pool:
        rows = pool.map(analyze, cand, chunksize=64)
    sp.init(pe)
    own = {}
    edges = {}
    for lo, mask, tg in rows:
        own[lo] = mask
        edges[lo] = tg
    cset = set(own)
    thunk_cache = {}
    for lo, tg in edges.items():
        keep = []
        for t in tg:
            if t in cset:
                keep.append(t)
                continue
            if t not in thunk_cache:
                thunk_cache[t] = is_thunk(t)
            own[lo] |= BIT["iat_call"] if thunk_cache[t] else BIT["other_target"]
        edges[lo] = keep
    closure = dict(own)
    rev = {}
    for lo, tg in edges.items():
        for t in tg:
            rev.setdefault(t, []).append(lo)
    work = [f for f, m in closure.items() if m]
    while work:
        f = work.pop()
        for c in rev.get(f, []):
            nm = closure[c] | closure[f]
            if nm != closure[c]:
                closure[c] = nm
                work.append(c)
    n = len(cset)
    res = {"candidates": n, "pure": sum(1 for m in closure.values() if m == 0),
           "own_overlapping": {r: sum(1 for m in own.values() if m & BIT[r]) for r in REASONS},
           "own_any": sum(1 for m in own.values() if m),
           "reach_overlapping": {r: sum(1 for m in closure.values() if m & BIT[r]) for r in REASONS},
           "indirect_call_or_jmp_reach": sum(1 for m in closure.values() if m & (BIT["indirect_call"] | BIT["indirect_jmp"])),
           "iat_reach": sum(1 for m in closure.values() if m & (BIT["iat_call"] | BIT["iat_operand"])),
           "x87_reach": sum(1 for m in closure.values() if m & BIT["x87"]),
           "gs_fs_reach": sum(1 for m in closure.values() if m & BIT["segment"])}
    disjoint = {}
    prio = ["iat_call", "iat_operand", "indirect_call", "indirect_jmp", "x87", "segment", "special", "other_target"]
    for lo, m in closure.items():
        if m == 0:
            k = "pure"
        else:
            k = None
            for r in prio:
                if own[lo] & BIT[r]:
                    k = "own:" + r
                    break
            if k is None:
                for r in prio:
                    if closure[lo] & BIT[r]:
                        k = "callee_only:" + r
                        break
        disjoint[k] = disjoint.get(k, 0) + 1
    res["disjoint_by_priority"] = dict(sorted(disjoint.items()))
    res["priority"] = prio
    with open(out_path, "w") as fh:
        json.dump(res, fh, indent=1)
    print(json.dumps(res, indent=1))


if __name__ == "__main__":
    main()
