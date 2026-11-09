# Static call-graph queries over the index made by fle_index.py (FLEngine_x64.dll). Read-only.
# usage: PYTHONDONTWRITEBYTECODE=1 python3 fle_paths.py INDEX.pkl TABLE.rs CMD [args]
#   callers API [depth]       : who calls the import (through its thunk), up to depth, direct edges only (tier T1)
#   reach ROOT_VA [tier]      : imports reachable from ROOT_VA. tier 1 = direct call/tail jmp only; tier 2 = + lea (address taken)
#   path ROOT_VA API          : shortest direct-call path from ROOT_VA to a thunk of API
#   fn VA                     : name the function that contains VA
# Tier meanings (same as K1): direct = call/jmp rel32; lea = function address taken by lea (callback/vtable candidates).
import sys, re, pickle, bisect, collections

idx = pickle.load(open(sys.argv[1], "rb"))
table_path = sys.argv[2]
cmd = sys.argv[3]
args = sys.argv[4:]
base = idx["base"]
funcs = idx["funcs"]
starts = [f[0] for f in funcs]
thunks = idx["thunks"]
calls, iatcalls, leas = idx["calls"], idx["iatcalls"], idx["leas"]

rows = set()
for ln in open(table_path, encoding="utf-8"):
    m = re.match(r'\s*\("([^"]+)",\s*"([^"]+)",\s*"fle_imp_', ln)
    if m:
        rows.add(m.group(1).lower() + "!" + m.group(2))


def func_of(va):
    k = bisect.bisect_right(starts, va) - 1
    if k >= 0 and funcs[k][0] <= va < funcs[k][1]:
        return funcs[k][0]
    return None


def norm(name):
    return name.replace("(delay)", "")


def status(name):
    return "row" if norm(name) in rows else "none"


# Edge tables: caller_func -> callee (func begin or thunk va)
def callee_nodes(f):
    out = set()
    for t in calls.get(f, ()):
        if t in thunks:
            out.add(t)
        else:
            g = func_of(t)
            if g is not None:
                out.add(g)
    return out


def imports_of(f):
    s = set(iatcalls.get(f, ()))
    return s


def parse_va(s):
    return int(s, 16)


if cmd == "fn":
    va = parse_va(args[0])
    f = func_of(va)
    print("%#x in function %s" % (va, ("%#x" % f) if f else "none (thunk? %s)" % thunks.get(va)))
elif cmd == "callers":
    api = args[0].lower()
    depth = int(args[1]) if len(args) > 1 else 1
    tvas = [t for t, n in thunks.items() if norm(n).lower().endswith("!" + api) or norm(n).lower() == api]
    direct = set(f for f, s in iatcalls.items() if any(norm(n).lower().endswith("!" + api) for n in s))
    # reverse edges
    rev = collections.defaultdict(set)
    for f in funcs:
        for g in callee_nodes(f[0]):
            rev[g].add(f[0])
    frontier = set()
    for t in tvas:
        frontier |= rev.get(t, set())
    frontier |= direct
    seen = {f: 1 for f in frontier}
    cur = frontier
    for d in range(2, depth + 1):
        nxt = set()
        for f in cur:
            for c in rev.get(f, ()):
                if c not in seen:
                    seen[c] = d
                    nxt.add(c)
        cur = nxt
    print("thunks:", [hex(t) for t in tvas])
    for f, d in sorted(seen.items(), key=lambda x: (x[1], x[0])):
        print("depth %d  f_%x" % (d, f))
elif cmd == "reach":
    root = parse_va(args[0])
    tier = int(args[1]) if len(args) > 1 else 1
    r = func_of(root) or root
    seen = {r}
    dq = collections.deque([r])
    imps = collections.Counter()
    while dq:
        f = dq.popleft()
        for n in imports_of(f):
            imps[n] += 1
        nxts = set()
        for g in callee_nodes(f):
            if g in thunks:
                imps[thunks[g]] += 1
            elif g not in seen:
                nxts.add(g)
        if tier >= 2:
            for t in leas.get(f, ()):
                g = func_of(t)
                if g is not None and g not in seen:
                    nxts.add(g)
                elif t in thunks:
                    imps[thunks[t]] += 1
        for g in nxts:
            if g not in seen:
                seen.add(g)
                dq.append(g)
    print("functions reached:", len(seen))
    byst = collections.Counter(status(n) for n in imps)
    print("distinct imports reached:", len(imps), dict(byst))
    for n, c in sorted(imps.items()):
        print("%s\t%s\t%d" % (status(n), n, c))
elif cmd == "path":
    root = parse_va(args[0])
    api = args[1].lower()
    r = func_of(root) or root
    prev = {r: None}
    dq = collections.deque([r])
    hit = None
    while dq and hit is None:
        f = dq.popleft()
        if any(norm(n).lower().endswith("!" + api) for n in imports_of(f)):
            hit = f
            break
        for g in callee_nodes(f):
            if g in thunks:
                if norm(thunks[g]).lower().endswith("!" + api):
                    prev[g] = f
                    hit = g
                    break
            elif g not in prev:
                prev[g] = f
                dq.append(g)
    if hit is None:
        print("no direct path")
    else:
        p = []
        x = hit
        while x is not None:
            p.append(x)
            x = prev.get(x)
        print(" <- ".join(("f_%x" % x) if x not in thunks else ("thunk_%x(%s)" % (x, thunks[x])) for x in p))
