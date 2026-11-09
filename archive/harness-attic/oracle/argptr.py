#!/usr/bin/env python3
"""A5 oracle v2: static detection of pointer arguments (decision 0036).

For every pure-compute function (select_pure.py output) decide which incoming arguments are
dereferenced as pointers: RCX/RDX/R8/R9 (positions 0-3) and the first four stack arguments
(positions 4-7, [rsp+0x28...] at entry). One forward linear sweep over the function body with
  - register tags: ("arg", k) = still holds argument k (or a pointer derived from it by mov/lea/add/sub),
                   ("ind", k) = was loaded from memory addressed through argument k (double indirection)
  - rsp-relative stack slots (entry-relative offsets), so `mov [rsp+8], rcx` / `mov rax, [rsp+8]` keeps the tag
  - forward branches: the state at a branch is merged (union) into the state at its target
  - a memory operand whose base register carries ("arg", k) marks k as a pointer ("ind", k) marks k as
    pointing to a structure that holds pointers
  - a direct call/jmp out of the function passes the tags of RCX/RDX/R8/R9 to the callee; the callee's own
    pointer positions are propagated back (fixpoint over the call graph)
Linear sweep with union merge is an approximation (no loop back edges, no rbp-relative slots).

usage: argptr.py PE PURE.json OUT.json
"""
import json
import multiprocessing as mp
import sys
from collections import defaultdict

from capstone import Cs, CS_ARCH_X86, CS_MODE_64, CS_OP_IMM, CS_OP_MEM, CS_OP_REG, CS_GRP_CALL, CS_GRP_JUMP, CS_GRP_RET

sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))
from select_pure import load_pe  # noqa: E402

CANON = {}
for _n in ("ax", "cx", "dx", "bx", "sp", "bp", "si", "di"):
    CANON["r" + _n] = "r" + _n
    CANON["e" + _n] = "r" + _n
    CANON[_n] = "r" + _n
for _n, _p in (("al", "rax"), ("cl", "rcx"), ("dl", "rdx"), ("bl", "rbx"), ("ah", "rax"), ("ch", "rcx"),
               ("dh", "rdx"), ("bh", "rbx"), ("spl", "rsp"), ("bpl", "rbp"), ("sil", "rsi"), ("dil", "rdi")):
    CANON[_n] = _p
for _i in range(8, 16):
    for _s in ("", "d", "w", "b"):
        CANON["r%d%s" % (_i, _s)] = "r%d" % _i
ARGREG = ["rcx", "rdx", "r8", "r9"]
VOLATILE = ("rax", "rcx", "rdx", "r8", "r9", "r10", "r11")
NO_DEREF = {"lea", "nop", "prefetcht0", "prefetcht1", "prefetcht2", "prefetchnta", "prefetchw", "prefetch"}
KEEP_TAG = {"add", "sub", "inc", "dec", "and", "or"}
G = {}


def init(pe):
    d, base, dd, secs = load_pe(pe)
    md = Cs(CS_ARCH_X86, CS_MODE_64)
    md.detail = True
    img = bytearray(max(va + max(vs, rs) for _n, va, vs, rs, _rp, _c in secs))
    for _n, va, vs, rs, rp, _c in secs:
        img[va: va + rs] = d[rp: rp + rs]
    G["md"] = md
    G["base"] = base
    G["code"] = bytes(img)


def canon(md, reg):
    return CANON.get(md.reg_name(reg)) if reg else None


def merge(states):
    regs = defaultdict(set)
    slots = defaultdict(set)
    deltas = set()
    for r, s, dl in states:
        for k, v in r.items():
            regs[k] |= v
        for k, v in s.items():
            slots[k] |= v
        deltas.add(dl)
    return dict(regs), dict(slots), (deltas.pop() if len(deltas) == 1 else None)


def analyze(fn):
    lo, hi = fn
    md = G["md"]
    base = G["base"]
    buf = G["code"][lo - base: hi - base]
    ptr, dbl = set(), set()
    idx, ptrf, idxf = set(), set(), set()
    calls = []
    regs = {"rcx": {("arg", 0, None)}, "rdx": {("arg", 1, None)}, "r8": {("arg", 2, None)}, "r9": {("arg", 3, None)}}
    slots = {}
    delta = 0
    bdelta = [None]
    alive = True
    snaps = defaultdict(list)
    ninsn = 0

    def mark(tags):
        for t, k, d in tags:
            if t == "arg":
                ptr.add(k)
            elif t in ("ind", "ind2"):
                ptr.add(k)
                dbl.add(k)
                if t == "ind" and d is not None and 0 <= d < 0x4000:
                    ptrf.add((k, d))

    def mark_index(tags):
        for t, k, d in tags:
            if t in ("arg", "val"):
                idx.add(k)
            elif t == "ind" and d is not None and 0 <= d < 0x4000:
                idxf.add((k, d))

    def conv_val(tags):
        return {("val", k, None) for t, k, d in tags if t in ("arg", "val")}

    def mem_tags(op):
        m = op.mem
        b = canon(md, m.base)
        return b, (regs.get(b, set()) if b else set())

    def slot_off(op):
        m = op.mem
        if m.index:
            return None
        cb = canon(md, m.base)
        if cb == "rsp" and delta is not None:
            return delta + m.disp
        if cb == "rbp" and bdelta[0] is not None:
            return bdelta[0] + m.disp
        return None

    for ins in md.disasm(buf, lo):
        ninsn += 1
        a = ins.address
        if a in snaps:
            states = snaps.pop(a)
            if alive:
                states.append((regs, slots, delta))
            regs, slots, delta = merge(states)
            regs = {k: set(v) for k, v in regs.items()}
            slots = {k: set(v) for k, v in slots.items()}
            alive = True
        if not alive:
            continue
        mn = ins.mnemonic
        ops = ins.operands
        # 1. dereferences through pointer tags
        if mn not in NO_DEREF:
            for op in ops:
                if op.type == CS_OP_MEM:
                    b, tg = mem_tags(op)
                    if b and b != "rsp":
                        mark(tg)
                    ic = canon(md, op.mem.index)
                    if ic and regs.get(ic):
                        mark_index(regs[ic])
        if any(s in mn for s in ("movs", "stos", "cmps", "scas", "lods")) and not mn.startswith(("movz", "movq", "movd", "movap", "movup", "movdq", "movs" + "x", "movss", "movsd", "movsl", "movsh")):
            if mn.startswith("rep") or mn in ("movsb", "movsw", "movsq", "stosb", "stosw", "stosd", "stosq", "cmpsb", "scasb", "lodsb"):
                mark(regs.get("rsi", set()))
                mark(regs.get("rdi", set()))
        # 2. control flow
        is_call = ins.group(CS_GRP_CALL)
        is_jmp = ins.group(CS_GRP_JUMP)
        if is_call or is_jmp or ins.group(CS_GRP_RET):
            tgt = ops[0].imm if (ops and ops[0].type == CS_OP_IMM) else None
            if ins.group(CS_GRP_RET):
                alive = False
                continue
            inside = tgt is not None and lo <= tgt < hi
            if is_call or (is_jmp and tgt is not None and not inside):
                if tgt is not None:
                    passed = {j: sorted(regs.get(ARGREG[j], ())) for j in range(4) if regs.get(ARGREG[j])}
                    if passed:
                        calls.append((tgt, passed))
            if is_call:
                for r in VOLATILE:
                    regs.pop(r, None)
                continue
            if is_jmp:
                if inside and tgt > a:
                    snaps[tgt].append(({k: set(v) for k, v in regs.items()}, {k: set(v) for k, v in slots.items()}, delta))
                if mn == "jmp":
                    alive = False
                continue
        # 3. data movement
        wr = set()
        try:
            _rd, wl = ins.regs_access()
            for r in wl:
                c = canon(md, r)
                if c:
                    wr.add(c)
        except Exception:
            pass
        new = {}
        dst = ops[0] if ops else None
        d64 = dst is not None and dst.type == CS_OP_REG and dst.size == 8 and canon(md, dst.reg)
        dnarrow = dst is not None and dst.type == CS_OP_REG and dst.size in (1, 2, 4) and canon(md, dst.reg)
        if dnarrow and dnarrow not in ("rsp",) and len(ops) == 2 and ops[1].type == CS_OP_REG and mn in ("mov", "movzx", "movsxd", "movsx", "add", "sub", "and", "or", "xor", "inc", "dec", "lea"):
            sc = canon(md, ops[1].reg)
            t = conv_val(regs.get(sc, set()))
            if mn in ("add", "sub", "and", "or"):
                t |= conv_val(regs.get(dnarrow, set()))
            if t:
                new[dnarrow] = t
        elif dnarrow and mn in ("add", "sub", "and", "or", "inc", "dec", "neg", "not", "shl", "shr", "sar") and regs.get(dnarrow):
            new[dnarrow] = conv_val(regs[dnarrow])
        if mn == "mov" and len(ops) == 2 and d64:
            s = ops[1]
            if s.type == CS_OP_REG and s.size == 8:
                sc = canon(md, s.reg)
                if regs.get(sc):
                    new[d64] = set(regs[sc])
            elif s.type == CS_OP_MEM:
                so = slot_off(s)
                if so is not None and slots.get(so):
                    new[d64] = set(slots[so])
                else:
                    b, tg = mem_tags(s)
                    ind = {("ind", k, s.mem.disp) for t, k, d in tg if t == "arg"} | \
                          {("ind2", k, None) for t, k, d in tg if t in ("ind", "ind2")}
                    if b and b != "rsp" and ind:
                        new[d64] = ind
                if so is not None and 0x28 <= so < 0x28 + 8 * 4 and (so - 0x28) % 8 == 0 and d64 not in new:
                    new[d64] = {("arg", 4 + (so - 0x28) // 8, None)}
        elif mn == "lea" and len(ops) == 2 and d64 and ops[1].type == CS_OP_MEM:
            b, tg = mem_tags(ops[1])
            if tg:
                new[d64] = set(tg)
            elif b == "rsp":
                pass
        elif mn == "cmovcc" or (mn.startswith("cmov") and d64 and len(ops) == 2 and ops[1].type == CS_OP_REG and ops[1].size == 8):
            sc = canon(md, ops[1].reg)
            t = set(regs.get(d64, set())) | set(regs.get(sc, set()))
            if t:
                new[d64] = t
        elif mn in KEEP_TAG and d64 and regs.get(d64):
            new[d64] = set(regs[d64])
            if len(ops) == 2 and ops[1].type == CS_OP_REG and mn in ("add", "sub") and regs.get(canon(md, ops[1].reg)):
                for t, k, d in regs[canon(md, ops[1].reg)]:
                    if t in ("arg", "val"):
                        new[d64].add(("val", k, None))
        # stores / push / pop / rsp tracking
        if mn == "mov" and len(ops) == 2 and d64 == "rbp" and ops[1].type == CS_OP_REG and canon(md, ops[1].reg) == "rsp":
            bdelta[0] = delta
        elif mn == "lea" and d64 == "rbp" and ops[1].type == CS_OP_MEM and canon(md, ops[1].mem.base) == "rsp" \
                and not ops[1].mem.index and delta is not None:
            bdelta[0] = delta + ops[1].mem.disp
        elif d64 == "rbp" and mn not in ("pop",):
            bdelta[0] = None
        if mn == "mov" and len(ops) == 2 and ops[0].type == CS_OP_MEM:
            so = slot_off(ops[0])
            if so is not None:
                if ops[1].type == CS_OP_REG and ops[1].size == 8 and regs.get(canon(md, ops[1].reg)):
                    slots[so] = set(regs[canon(md, ops[1].reg)])
                else:
                    slots.pop(so, None)
        if mn == "push" and delta is not None:
            delta -= 8
            if ops and ops[0].type == CS_OP_REG and regs.get(canon(md, ops[0].reg)):
                slots[delta] = set(regs[canon(md, ops[0].reg)])
            else:
                slots.pop(delta, None)
        elif mn == "pop" and delta is not None:
            if d64 and slots.get(delta):
                new[d64] = set(slots[delta])
            delta += 8
        elif "rsp" in wr:
            if mn == "sub" and len(ops) == 2 and ops[1].type == CS_OP_IMM and delta is not None:
                delta -= ops[1].imm
            elif mn == "add" and len(ops) == 2 and ops[1].type == CS_OP_IMM and delta is not None:
                delta += ops[1].imm
            elif mn == "lea" and ops[1].type == CS_OP_MEM and canon(md, ops[1].mem.base) == "rsp" and not ops[1].mem.index and delta is not None:
                delta += ops[1].mem.disp
            elif mn == "lea" and ops[1].type == CS_OP_MEM and canon(md, ops[1].mem.base) == "rbp" and not ops[1].mem.index and bdelta[0] is not None:
                delta = bdelta[0] + ops[1].mem.disp
            elif mn == "mov" and len(ops) == 2 and ops[1].type == CS_OP_REG and canon(md, ops[1].reg) == "rbp" and bdelta[0] is not None:
                delta = bdelta[0]
            elif mn in ("call", "ret"):
                pass
            else:
                delta = None
        for r in wr:
            if r != "rsp":
                regs.pop(r, None)
        for r, t in new.items():
            regs[r] = t
    return (lo, sorted(ptr), sorted(dbl), [(t, {str(j): v for j, v in p.items()}) for t, p in calls], ninsn,
            sorted(idx - ptr), sorted(ptrf), sorted(idxf))


def main():
    pe, pure_path, out_path = sys.argv[1:4]
    pure = json.load(open(pure_path))
    fns = [(int(f["va"], 16), int(f["end"], 16)) for f in pure["functions"]]
    with mp.Pool(16, initializer=init, initargs=(pe,)) as pool:
        rows = pool.map(analyze, fns, chunksize=64)
    info = {r[0]: {"ptr": set(r[1]), "dbl": set(r[2]), "calls": r[3], "n": r[4], "idx": set(r[5]),
                   "ptrf": set(map(tuple, r[6])), "idxf": set(map(tuple, r[7]))} for r in rows}
    changed = True
    rounds = 0
    while changed:
        changed = False
        rounds += 1
        for va, inf in info.items():
            for tgt, passed in inf["calls"]:
                callee = info.get(tgt)
                if not callee:
                    continue
                for j, tags in passed.items():
                    j = int(j)
                    if j in callee["idx"]:
                        for t, k, d in tags:
                            if t in ("arg", "val") and k not in inf["idx"] and k not in inf["ptr"]:
                                inf["idx"].add(k)
                                changed = True
                    if j in callee["ptr"] or j in callee["dbl"]:
                        for t, k, d in tags:
                            if t not in ("arg", "ind", "ind2"):
                                continue
                            if k not in inf["ptr"]:
                                inf["ptr"].add(k)
                                inf["idx"].discard(k)
                                changed = True
                            if t == "ind" and d is not None and 0 <= d < 0x4000 and (k, d) not in inf["ptrf"]:
                                inf["ptrf"].add((k, d))
                                changed = True
                            if (j in callee["dbl"] and t == "arg") or t in ("ind", "ind2"):
                                if k not in inf["dbl"]:
                                    inf["dbl"].add(k)
                                    changed = True
    out = {"pe": pe, "rounds": rounds,
           "functions": {hex(va): {"ptr": sorted(i["ptr"]), "dbl": sorted(i["dbl"]), "insns": i["n"],
                                   "idx": sorted(i["idx"] - i["ptr"]), "ptrf": sorted(i["ptrf"]),
                                   "idxf": sorted(i["idxf"] - i["ptrf"])} for va, i in info.items()}}
    with open(out_path, "w") as fh:
        json.dump(out, fh)
    n_ptr = sum(1 for i in info.values() if i["ptr"])
    per = defaultdict(int)
    for i in info.values():
        for k in i["ptr"]:
            per[k] += 1
    print(json.dumps({"functions": len(info), "with_ptr_args": n_ptr, "per_position": dict(sorted(per.items())),
                      "with_double_indirection": sum(1 for i in info.values() if i["dbl"]), "rounds": rounds}),
          file=sys.stderr)


if __name__ == "__main__":
    main()
