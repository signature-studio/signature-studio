#!/usr/bin/env python3
"""ci/test-real-deploy.py - tests the condition that decides whether the workflow deploys for real. It reads the `if:` expressions of .github/workflows/deploy.yml that mention
github.ref, evaluates them with the semantics of GitHub's expression language for every event/ref combination, and fails if the table is not as expected: a run deploys for real exactly when it runs on main
(a push or a dispatch; the dispatch has no inputs). usage: python3 ci/test-real-deploy.py [workflow.yml]"""
import re, sys

path = sys.argv[1] if len(sys.argv) > 1 else ".github/workflows/deploy.yml"
text = open(path, encoding="utf-8").read()

TOK = re.compile(r"\s*(\(|\)|&&|\|\||==|!=|'[^']*'|[A-Za-z_][A-Za-z0-9_.]*)")
def tokenize(s):
    pos, out = 0, []
    while pos < len(s):
        m = TOK.match(s, pos)
        if not m:
            raise SystemExit("cannot tokenize %r at %d" % (s, pos))
        out.append(m.group(1)); pos = m.end()
    return out

def num(v):
    if v is None or v is False: return 0.0
    if v is True: return 1.0
    if isinstance(v, str):
        try: return float(v) if v.strip() else 0.0
        except ValueError: return float("nan")
    return float(v)
def eq(a, b):
    if type(a) is type(b) or (a is None and b is None):
        return (a.lower() == b.lower()) if isinstance(a, str) else a == b   # strings compare case-insensitively
    return num(a) == num(b)
def truthy(v): return not (v is None or v is False or v == 0 or v == "")

def evaluate(expr, ctx):
    toks = tokenize(expr); i = [0]
    def peek(): return toks[i[0]] if i[0] < len(toks) else None
    def take(): i[0] += 1; return toks[i[0] - 1]
    def atom():
        t = take()
        if t == "(":
            v = oror(); assert take() == ")"; return v
        if t.startswith("'"): return t[1:-1]
        if t == "true": return True
        if t == "false": return False
        if t == "null": return None
        if t in ctx: return ctx[t]
        raise SystemExit("unknown name %r in %r" % (t, expr))
    def cmp_():
        v = atom()
        while peek() in ("==", "!="):
            op = take(); r = atom(); v = eq(v, r) if op == "==" else not eq(v, r)
        return v
    def andand():
        v = cmp_()
        while peek() == "&&":
            take(); r = cmp_(); v = r if truthy(v) else v
        return v
    def oror():
        v = andand()
        while peek() == "||":
            take(); r = andand(); v = v if truthy(v) else r
        return v
    v = oror()
    if i[0] != len(toks): raise SystemExit("trailing tokens in %r" % expr)
    return truthy(v)

# every `if:` that mentions the ref
exprs = {}
for m in re.finditer(r"^\s*if:\s*\$\{\{\s*(.*?)\s*\}\}\s*$", text, re.M):
    e = m.group(1)
    if "github.ref" in e:
        exprs[e] = exprs.get(e, 0) + 1
assert len(exprs) == 1, "expected one real-deploy condition, found %r" % list(exprs)
REAL = list(exprs)[0]
assert exprs[REAL] == 5, "the real-deploy condition must gate exactly 5 steps (upload of the files, the lifecycle of the beacons, the Worker version, the switch of the default manifest, the Access probe), found %d" % exprs[REAL]
assert "inputs." not in text.split("permissions:")[0] and "inputs." not in text, "the workflow has no inputs (decision 0243)"
assert re.search(r"^  workflow_dispatch:\s*$", text, re.M), "workflow_dispatch must be there and have no inputs"

MAIN, BRANCH = "refs/heads/main", "refs/heads/ci/cloudflare-deploy"
# (event, ref) -> real deploy
CASES = [
    ("push", MAIN, True),
    ("push", BRANCH, False),
    ("push", "refs/heads/feature", False),
    ("push", "refs/tags/v1", False),
    ("workflow_dispatch", MAIN, True),
    ("workflow_dispatch", BRANCH, False),
    ("workflow_dispatch", "refs/heads/feature", False),
]
bad = 0
for ev, ref, want_real in CASES:
    ctx = {"github.event_name": ev, "github.ref": ref}
    got_real = evaluate(REAL, ctx)
    ok = got_real == want_real
    bad += not ok
    print("%s %-18s %-34s -> real deploy %-5s" % ("ok  " if ok else "FAIL", ev, ref, got_real))
sys.exit(1 if bad else 0)
