# Which A2b audit P/X imports lie in the direct-call closure of a root in FLEngine_x64.dll. Read-only.
# usage: PYTHONDONTWRITEBYTECODE=1 python3 a2b_px_vs_closure.py INDEX.pkl AUDIT_DIR OUT.tsv [ROOT_VA ...]
#   INDEX.pkl  made by fle_index.py;  AUDIT_DIR = docs/investigations/a2b-audit (audit-GU.tsv, audit-K1.tsv, audit-K2.tsv)
#   default roots: 0x1375990 (CreateFruityInstance), 0x11b32a0, 0x13751e0, 0x86a660 (Run)
# A row is P/X when its class column contains P or X (so the three mixed rows P;X, N;P, X;P are included: 77 pure + 3 mixed = 80).
# The (DLL, name) match is by API name only; the closure is direct call / tail jmp edges (no vtable, no lea).
import sys, csv, pickle, bisect, os

idxp, auditdir, out = sys.argv[1:4]
roots = [int(x, 16) for x in sys.argv[4:]] or [0x1375990, 0x11b32a0, 0x13751e0, 0x86a660]
idx = pickle.load(open(idxp, "rb"))
calls, funcs, thunks, iatc = idx["calls"], idx["funcs"], idx["thunks"], idx["iatcalls"]
st = [f[0] for f in funcs]


def fo(v):
    k = bisect.bisect_right(st, v) - 1
    return funcs[k][0] if k >= 0 and funcs[k][0] <= v < funcs[k][1] else None


def clos(root):
    S = {root}
    dq = [root]
    while dq:
        f = dq.pop()
        for t in calls.get(f, ()):
            g = fo(t)
            if g is not None and g not in S:
                S.add(g)
                dq.append(g)
    return S


def norm(n):
    d, a = n.replace("(delay)", "").split("!", 1)
    return (d.replace(".dll", ""), a)


def imps(S):
    o = set()
    for f in S:
        for n in iatc.get(f, ()):
            o.add(norm(n))
        for t in calls.get(f, ()):
            if t in thunks:
                o.add(norm(thunks[t]))
    return o


rows = []
for f in ("GU", "K1", "K2"):
    for r in csv.DictReader(open(os.path.join(auditdir, "audit-%s.tsv" % f)), delimiter="\t"):
        rows.append(r)
px = sorted({(r["dll"].lower().replace(".dll", ""), r["api"]) for r in rows if ("P" in r["class"] or "X" in r["class"])})
res = {}
for rt in roots:
    S = clos(rt)
    res[rt] = (len(S), imps(S))
    print("root %#x closure %d functions: P/X reached %d of %d" % (rt, len(S), len([a for a in px if a in res[rt][1]]), len(px)))
with open(out, "w", encoding="utf-8") as fo_:
    fo_.write("api\t" + "\t".join("%#x" % r for r in roots) + "\n")
    for a in px:
        fo_.write("%s!%s\t%s\n" % (a[0], a[1], "\t".join("in" if a in res[r][1] else "-" for r in roots)))
