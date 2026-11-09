#!/usr/bin/env python3
"""Audit of `noreturn` declarations against the implementations that can return (decision 0204; crates/fe-lift/tests/noreturn_audit.rs runs it).

Why: a header that declares a function `__attribute__((noreturn))` ("stops by name", decision 0078) lets clang drop every instruction after a call to it. If an implementation
that is linked in (a harness `__wrap_<name>`, or a strong definition) does return, the caller's code after the call is missing from the object: decision 0190 (wall W12) was
`fle_imp_GetModuleHandleExW`, which cost the guest stack 8 bytes per call.

What it does (no build of the product is needed, only clang):
 1. Declarations. Every runtime header (`<runtime>/*.h`) is parsed alone with `clang -fsyntax-only -Xclang -ast-dump=json`, once as it is and once with -DFLE_GMHEX_RETURNS (the
    opt-in macro of decision 0190), and each function whose type carries `noreturn` is recorded with its header and line. The headers that a generated unit includes are found
    from the `#include "fle_*.h"` string literals of crates/fe-lift/src/*.rs and the `#include` closure of those headers ("gen-visible").
 2. Implementations. Every C file of <runtime> and <harness> that mentions one of those names (or builds a name with `##`) is compiled with
    `clang -O0 -S -emit-llvm -gline-tables-only` after a prelude that declares `__real_<name>` noreturn too (a wrapper that only forwards to the real function returns exactly when the
    real one does, so it cannot contradict the declaration). A definition of `<name>` or `__wrap_<name>` whose IR has a `ret` instruction is an implementation that can return.
    clang itself already warns (-Winvalid-noreturn) for a definition in the same translation unit as its declaration; the IR check also covers the wrappers, which are different
    symbols that the linker substitutes (`-Wl,--wrap=`).
 3. The reverse direction: a strong definition of an `fle_imp_*` function with no `ret` in its IR whose declarations are not noreturn ("stops, but is declared as returning").
    Harmless for correctness (the code after the call is only dead), reported as a table.
 4. Unanalysed files: a file that mentions one of the names but cannot be compiled natively (wasm-only, emscripten.h, windows.h, a missing -D) is listed; the test fails unless the file
    is on the allow list with a reason, because a definition in it could not be seen.

The verdict compares the findings with an allow list (TSV, one row per accepted finding with a reason). Not covered (the table says so): implementations in JavaScript or assembly,
wrappers that the build scripts of out-of-tree directories generate (the three gen_*wraps.py generators are run on the names here: --gen-wraps), a caller in a harness that continues after
a call to a noreturn name (clang drops that code silently), and a `ret` that is reachable in the IR but never at run time (the check is structural).

usage: noreturn_audit.py --lift DIR [--runtime DIR] [--harness DIR] [--allow FILE] [--extra-c FILE ...] [--table DIR] [--work DIR] [--gen-c FILE ...] [--quiet]
exit status: 0 = no finding outside the allow list and nothing stale; 1 = findings (printed); 2 = the tool itself failed.
"""
import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor

CLANG = os.environ.get("CLANG", "clang")
OPT_MACROS = ["FLE_GMHEX_RETURNS"]
JOBS = 4


def run(cmd, **kw):
    return subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, errors="replace", **kw)


def rel(p, base):
    try:
        return os.path.relpath(p, base)
    except ValueError:
        return p


# ---------------------------------------------------------------- declarations

def walk_files(node, cur, found):
    """Document-order walk that tracks the current file the way clang's JSON writer does (a `file` key appears only when it changes)."""
    for key in ("loc", "range"):
        v = node.get(key)
        if not v:
            continue
        locs = [v] if key == "loc" else [v.get("begin", {}), v.get("end", {})]
        for l in locs:
            if "file" in l:
                cur[0] = l["file"]
            sp = l.get("spellingLoc")
            if sp and "file" in sp:
                cur[0] = sp["file"]
            ex = l.get("expansionLoc")
            if ex and "file" in ex:
                cur[0] = ex["file"]
    if node.get("kind") == "FunctionDecl" and node.get("name"):
        q = node.get("type", {}).get("qualType", "")
        attrs = [a.get("kind", "") for a in node.get("inner", []) if isinstance(a, dict)]
        if "noreturn" in q or any("NoReturn" in a for a in attrs):
            l = node.get("loc", {})
            sp = l.get("spellingLoc", l)
            found.append((node["name"], cur[0], sp.get("line", 0)))
    for ch in node.get("inner", []):
        if isinstance(ch, dict):
            walk_files(ch, cur, found)


def header_decls(runtime, hdr, defines, work):
    src = os.path.join(work, "decl_" + hdr.replace(".", "_") + ("_" + "_".join(defines) if defines else "") + ".c")
    with open(src, "w") as f:
        f.write('#include "%s"\n' % hdr)
    cmd = [CLANG, "-fsyntax-only", "-std=gnu11", "-w", "-Xclang", "-ast-dump=json", "-I", runtime] + ["-D" + d for d in defines] + [src]
    r = run(cmd)
    if r.returncode != 0:
        return None, r.stderr[:400]
    try:
        j = json.loads(r.stdout)
    except ValueError as e:
        return None, str(e)
    found = []
    walk_files(j, [None], found)
    out = []
    seen = set()
    for name, f, line in found:
        if f and os.path.basename(f) == hdr and (name, line) not in seen:
            seen.add((name, line))
            out.append((name, line))
    return out, ""


def include_closure(runtime, hdrs):
    inc = re.compile(r'^\s*#\s*include\s*"([^"]+)"', re.M)
    out = set()
    todo = list(hdrs)
    while todo:
        h = todo.pop()
        if h in out:
            continue
        p = os.path.join(runtime, h)
        if not os.path.isfile(p):
            continue
        out.add(h)
        todo += inc.findall(open(p, errors="replace").read())
    return out


def gen_headers(lift):
    """The runtime headers that the generator writes into a unit: the string literals `#include \"x.h\"` in crates/fe-lift/src/*.rs."""
    src = os.path.join(lift, "crates", "fe-lift", "src")
    hs = set()
    if os.path.isdir(src):
        for fn in sorted(os.listdir(src)):
            if fn.endswith(".rs"):
                t = open(os.path.join(src, fn), errors="replace").read()
                hs.update(re.findall(r'#include \\"(fle_[A-Za-z0-9_]+\.h)\\"', t))
                hs.update(re.findall(r'#include "(fle_[A-Za-z0-9_]+\.h)"', t))
    return hs


# ---------------------------------------------------------------- implementations

def find_c(root):
    out = []
    for d, ds, fs in os.walk(root):
        ds[:] = sorted(x for x in ds if not x.startswith("out-") and x not in ("target", "node_modules", ".git"))
        for f in sorted(fs):
            if f.endswith(".c"):
                out.append(os.path.join(d, f))
    return out


def all_dirs(root):
    out = []
    for d, ds, fs in os.walk(root):
        ds[:] = sorted(x for x in ds if not x.startswith("out-") and x not in ("target", "node_modules", ".git"))
        if any(f.endswith(".h") for f in fs):
            out.append(d)
    return out


def literal_mentions(path, names):
    """The noreturn names (full, or without the `fle_imp_` prefix: the macro arguments of V1API(GetFileType)) that a file spells out; used for a file that cannot be compiled."""
    t = open(path, errors="replace").read()
    words = set(re.findall(r"[A-Za-z_]\w*", t))
    hit = set()
    for n in names:
        short = n[len("fle_imp_"):] if n.startswith("fle_imp_") else None
        if n in words or "__wrap_" + n in words or (short and len(short) >= 6 and short in words):
            hit.add(n)
    return hit


def mentions(path, names, wrapnames):
    t = open(path, errors="replace").read()
    if re.search(r"fle_imp_\w*##|__wrap_\w*##|__real_\w*##", t):
        return True
    for m in re.finditer(r"[A-Za-z_]\w*", t):
        w = m.group(0)
        if w in names or w in wrapnames:
            return True
    return False


DEF_RE = re.compile(r"^define\b[^\n]*@([A-Za-z0-9_.$]+)\(", re.M)


def ir_functions(text):
    """name -> dict(ret, line, forward) for every function defined in an IR text; the line comes from the DISubprogram of -gline-tables-only.
    forward: the body is one basic block with one call whose arguments carry the address of `__real_<n>` (diag_stub_call(i, __real_X), v1_call("X", __real_X)): the wrapper
    hands the real function to a harness helper that calls it, so it returns exactly when the real one does."""
    subs = {}
    files = {}
    for m in re.finditer(r'^!(\d+) = !DIFile\(filename: "([^"]*)"', text, re.M):
        files[m.group(1)] = os.path.basename(m.group(2))
    sublist = []   # (name, line, basename of its file), in the order of the IR
    for m in re.finditer(r'!DISubprogram\(name: "([^"]+)"[^\n]*?file: !(\d+), line: (\d+)', text):
        subs.setdefault(m.group(1), int(m.group(3)))
        sublist.append((m.group(1), int(m.group(3)), files.get(m.group(2), "")))
    res = {}
    for m in DEF_RE.finditer(text):
        name = m.group(1)
        end = text.find("\n}\n", m.end())
        body = text[m.end():end if end >= 0 else len(text)]
        has_ret = re.search(r"^\s+ret\b", body, re.M) is not None
        calls = re.findall(r"^\s+(?:%\S+ = )?(?:tail )?call\b[^\n]*", body, re.M)
        one_block = re.search(r"^\s+(?:br|switch|select|indirectbr)\b", body, re.M) is None and "phi " not in body
        forward = has_ret and one_block and len(calls) == 1 and ("@__real_" + name[7:]) in calls[0] if name.startswith("__wrap_") else False
        callees = set(re.findall(r"\bcall\b[^\n]*?@([A-Za-z0-9_.$]+)\(", body))
        res[name] = {"ret": has_ret, "line": subs.get(name, 0), "forward": forward, "callees": callees}
    return res, sublist


def compile_ir(path, incs, extra, prelude, work, tag):
    ll = os.path.join(work, tag + ".ll")
    cmd = [CLANG, "-O0", "-std=gnu11", "-S", "-emit-llvm", "-gline-tables-only", "-Wno-everything", "-Winvalid-noreturn", "-Wno-error", "-ferror-limit=5", "-include", prelude]
    for i in incs:
        cmd += ["-I", i]
    cmd += extra + [path, "-o", ll]
    r = run(cmd)
    if r.returncode != 0:
        return None, r.stderr
    return open(ll).read(), r.stderr


def tag_of(path, base):
    return re.sub(r"[^A-Za-z0-9]+", "_", rel(path, base))


def analyse_file(path, lift, runtime, harness, prelude, work):
    tag = tag_of(path, lift)
    inc1 = [runtime, os.path.dirname(path), harness]
    text, err = compile_ir(path, inc1, [], prelude, work, tag)
    how = "plain"
    if text is None:
        inc2 = inc1 + all_dirs(harness)
        text2, err2 = compile_ir(path, inc2, [], prelude, work, tag + "_b")
        if text2 is not None:
            text, err, how = text2, err2, "all-harness-include-dirs"
        else:
            stub = os.path.join(work, "stubs")
            os.makedirs(stub, exist_ok=True)
            for h in ("emscripten.h",):
                open(os.path.join(stub, h), "a").close()
            text3, err3 = compile_ir(path, inc2 + [stub], ["-Wno-implicit-function-declaration", "-Wno-int-conversion"], prelude, work, tag + "_c")
            if text3 is not None:
                text, err, how = text3, err3, "empty emscripten.h stub"
            else:
                m = re.search(r"error: ([^\n]*)", err)
                return None, (m.group(1) if m else (err.strip().splitlines()[0] if err.strip() else "compile failed")), how
    warn = [l for l in err.splitlines() if "-Winvalid-noreturn" in l]
    return (ir_functions(text), warn), "", how


def run_gen_wraps(lift, names, work):
    """Runs the generators of pass-through wrappers on the names and returns the C files they wrote (they are analysed like any other file)."""
    outs = []
    syms = os.path.join(work, "gen_syms.txt")
    with open(syms, "w") as f:
        for n in sorted(names):
            if n.startswith("fle_imp_"):
                f.write(n + "\n")
    for rel_py, outname in (("harness/idle/gen_idle_wraps.py", "idle_wraps.c"), ("harness/web/perf2/gen_p2_wraps.py", "p2_wraps.c")):
        py = os.path.join(lift, rel_py)
        if not os.path.isfile(py):
            continue
        d = os.path.join(work, "genwraps_" + os.path.basename(py)[:-3])
        os.makedirs(d, exist_ok=True)
        r = run([sys.executable, py, syms, d])
        c = os.path.join(d, outname)
        if r.returncode == 0 and os.path.isfile(c):
            outs.append((c, rel_py))
    return outs


# ---------------------------------------------------------------- main

def load_allow(path):
    rows = []
    if path and os.path.isfile(path):
        for ln, l in enumerate(open(path, encoding="utf-8"), 1):
            l = l.rstrip("\n")
            if not l.strip() or l.startswith("#"):
                continue
            f = l.split("\t")
            if len(f) < 4:
                raise SystemExit("allow list %s:%d: need kind<TAB>name<TAB>where<TAB>reason" % (path, ln))
            rows.append((f[0], f[1], f[2], "\t".join(f[3:]), ln))
    return rows


def count_gen_calls(gen_files, names):
    """Per generated file: the direct calls `fle_imp_<name>();` that the generator writes into a lifted block, and every other mention (the declaration, a table row).
    Also whether the file includes fle_modules.h before its last call (the noreturn declarations of that header are then in force for the calls)."""
    pat = re.compile(r"\b(fle_imp_[A-Za-z0-9_]+)\b(\(\))?")
    out = {}
    for gf in gen_files:
        calls = {n: 0 for n in names}
        other = {n: 0 for n in names}
        with open(gf, "r", errors="replace") as f:
            for line in f:
                if "fle_imp_" not in line:
                    continue
                for m in pat.finditer(line):
                    n = m.group(1)
                    if n in calls:
                        if m.group(2) and not line.startswith("void "):
                            calls[n] += 1
                        else:
                            other[n] += 1
        out[os.path.basename(os.path.dirname(gf)) + "/" + os.path.basename(gf)] = (calls, other)
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--lift", required=True)
    ap.add_argument("--runtime")
    ap.add_argument("--harness")
    ap.add_argument("--allow")
    ap.add_argument("--extra-c", action="append", default=[])
    ap.add_argument("--table")
    ap.add_argument("--work")
    ap.add_argument("--gen-c", action="append", default=[])
    ap.add_argument("--no-gen-wraps", action="store_true")
    ap.add_argument("--quiet", action="store_true")
    ap.add_argument("--fast", action="store_true", help="parse only the headers that spell `noreturn` and skip the runtime files that name no noreturn function (no reverse-direction table); for the mutation tests")
    a = ap.parse_args()
    lift = os.path.abspath(a.lift)
    runtime = os.path.abspath(a.runtime or os.path.join(lift, "runtime"))
    harness = os.path.abspath(a.harness or os.path.join(lift, "harness"))
    work = a.work or tempfile.mkdtemp(prefix="noret-")
    os.makedirs(work, exist_ok=True)

    # 1. declarations
    hdrs = sorted(f for f in os.listdir(runtime) if f.endswith(".h"))
    decl = {}      # name -> list of dict(header, line, states)
    errors = []
    fragments = set()   # headers that are not self-contained (included in the middle of a .c) and never say 'noreturn'
    if a.fast:
        hdrs = [h for h in hdrs if "noreturn" in open(os.path.join(runtime, h), errors="replace").read().lower()]
    jobs = [(h, state, defs) for h in hdrs for state, defs in (("default", []), ("opt-in", OPT_MACROS))]
    with ThreadPoolExecutor(max_workers=JOBS) as ex:
        results = list(ex.map(lambda j: header_decls(runtime, j[0], j[2], work), jobs))
    by_h = {}
    for (h, state, defs), (got, err) in zip(jobs, results):
        by_h.setdefault(h, {})[state] = (got, err)
    for h in hdrs:
        per_state = {}
        for state in ("default", "opt-in"):
            got, err = by_h[h][state]
            if got is None and "noreturn" not in open(os.path.join(runtime, h), errors="replace").read().lower():
                fragments.add(h)
                continue
            if got is None:
                errors.append("header %s (%s) does not parse alone: %s" % (h, state, err.strip().splitlines()[0] if err.strip() else ""))
                continue
            per_state[state] = got
        names = {}
        for state, lst in per_state.items():
            for n, line in lst:
                names.setdefault((n, line), set()).add(state)
        allstates = set(per_state)
        for (n, line), st in sorted(names.items(), key=lambda x: (x[0][1], x[0][0])):
            decl.setdefault(n, []).append({"header": h, "line": line, "always": st == allstates, "states": sorted(st)})
    ghdr = include_closure(runtime, gen_headers(lift))
    nr_names = set(decl)
    wrapnames = {"__wrap_" + n for n in nr_names}

    # 2. implementations
    files = find_c(runtime) + find_c(harness) + [os.path.abspath(p) for p in a.extra_c]
    cand = [p for p in files if mentions(p, nr_names, wrapnames)]
    prelude = os.path.join(work, "noret_prelude.h")
    with open(prelude, "w") as f:
        for n in sorted(x for x in nr_names if x.startswith("fle_imp_")):
            f.write("void __real_%s(void) __attribute__((noreturn));\n" % n)
    gen_files = []
    if not a.no_gen_wraps:
        gen_files = run_gen_wraps(lift, nr_names, work)
    impls = []          # (kind, name, file, line, returns)
    unanalysed = []     # (file, reason)
    unanalysed_rt = []  # runtime files that mention no noreturn name and do not compile: only the reverse direction loses them
    noret_warn = []     # (file, text)
    callers = []        # (noreturn fle_imp_ name, calling function, file, line): a C call that expects a return
    forwards = []       # (name, file, line)  wrappers that hand the real function to a helper
    reverse = []        # (name, file, line)
    wrap_noret = []     # (name, file, line)   __wrap_ of a non-noreturn function that never returns
    n_analysed = 0
    cand_set = set(cand)
    gen_set = {g: py for g, py in gen_files}
    # every runtime file is analysed for the reverse direction, whether or not it mentions a noreturn name
    todo = list(cand) + [g for g, _ in gen_files] + ([] if a.fast else [p for p in find_c(runtime) if p not in cand_set])
    with ThreadPoolExecutor(max_workers=JOBS) as ex:
        analysed = list(ex.map(lambda p: analyse_file(p, lift, runtime, harness, prelude, work), todo))
    for p, (res, why, how) in zip(todo, analysed):
        shown = ("generated by " + gen_set[p]) if p in gen_set else (rel(p, lift) if p.startswith(lift) else p)
        in_runtime = p.startswith(runtime + os.sep)
        if res is None:
            if p in cand_set or p in gen_set:
                lit = literal_mentions(p, nr_names)
                if lit:
                    unanalysed.append((shown, why + "; spells out: " + ",".join(sorted(lit))[:200]))
                else:
                    unanalysed_rt.append((shown, "spells out no noreturn name (a ## paste only; wasm-only source): " + why))
            else:
                unanalysed_rt.append((shown, "a runtime file that names no noreturn function; only the reverse-direction table misses it: " + why))
            continue
        n_analysed += 1
        (fns, sublist), warn = res
        for w in warn:
            # "file:line:col: warning: function declared 'noreturn' should not return [-Winvalid-noreturn]" (the message has no name; the position is the closing brace): in the TU that
            # sees the declaration clang ends such a function with `unreachable`, so its IR has no ret and this warning is the evidence. The function is the last one of the same file
            # that starts at or before the warning's line.
            m = re.match(r"(.*?):(\d+):\d+: warning:", w.strip())
            owner = None
            if m:
                base, wl = os.path.basename(m.group(1)), int(m.group(2))
                cands = [x for x in sublist if x[2] == base and x[1] <= wl]
                if cands:
                    owner = max(cands, key=lambda x: x[1])[0]
            if owner in nr_names:
                impls.append(("def", owner, shown, wl, True))
            else:
                noret_warn.append((shown, w.strip()))
        for fname, info in fns.items():
            has_ret, line = info["ret"], info["line"]
            if not (fname in nr_names or (fname.startswith("__wrap_") and fname[7:] in nr_names)):
                for c in sorted(info["callees"] & nr_names):
                    if c.startswith("fle_imp_"):
                        callers.append((c, fname, shown, line))
            if fname in nr_names:
                impls.append(("def", fname, shown, line, has_ret))
            elif fname.startswith("__wrap_") and fname[7:] in nr_names:
                if info["forward"]:
                    forwards.append((fname[7:], shown, line))
                    impls.append(("wrap", fname[7:], shown, line, False))
                else:
                    impls.append(("wrap", fname[7:], shown, line, has_ret))
            elif in_runtime and fname.startswith("fle_imp_") and not has_ret:
                reverse.append((fname, shown, line))
            elif (not in_runtime) and fname.startswith("__wrap_fle_imp_") and not has_ret:
                wrap_noret.append((fname[7:], shown, line))

    # 3. findings
    returning = [i for i in impls if i[4]]
    findings = []   # (kind, name, where, text)
    for (k, n, f, line, r) in returning:
        d = decl[n]
        vis = any(x["header"] in ghdr for x in d)
        cond = all(not x["always"] for x in d)
        hd = ",".join(sorted({"%s:%d%s" % (x["header"], x["line"], "" if x["always"] else "(only without the opt-in macro)") for x in d}))
        findings.append(("returns", n, "%s:%d" % (f, line), "%s %s can return; declared noreturn at %s; gen-visible=%s" % (k, n, hd, "yes" if vis else "no"), {"vis": vis, "cond": cond}))
    for (f, w) in noret_warn:
        findings.append(("invalid-noreturn", w.split(":")[0] if ":" in w else f, f, w, {}))
    for (f, why) in unanalysed:
        findings.append(("unanalysed", f, f, why, {}))
    for e in errors:
        findings.append(("tool-error", e, "-", e, {}))

    allow = load_allow(a.allow)
    used = set()
    bad = []
    for kind, name, where, text, meta in findings:
        hit = None
        why_not = ""
        for i, (ak, an, aw, reason, ln) in enumerate(allow):
            if ak == kind and an == name and (aw == "*" or where.split(":")[0] == aw or where == aw):
                tag = reason.split(":", 1)[0].strip()
                if tag == "not-gen-visible" and meta.get("vis"):
                    why_not = "allow row line %d says not-gen-visible, but a header that declares it noreturn is in the include closure of the generated units now" % ln
                    used.add(i)
                    continue
                if tag == "opt-in-macro" and not meta.get("cond"):
                    why_not = "allow row line %d says opt-in-macro, but the declaration is no longer conditional on the macro" % ln
                    used.add(i)
                    continue
                hit = i
                break
        if hit is None:
            bad.append((kind, name, where, text + ((" | " + why_not) if why_not else "")))
        else:
            used.add(hit)
    stale = [r for i, r in enumerate(allow) if i not in used]

    # gen counts (optional)
    gcounts = None
    if a.gen_c:
        gcounts = count_gen_calls(a.gen_c, nr_names)

    # tables
    if a.table:
        os.makedirs(a.table, exist_ok=True)
        with open(os.path.join(a.table, "declared-noreturn.tsv"), "w") as f:
            f.write("name\theader:line\tin generated include closure\tdefinitions (def+wrap)\tof which can return\tforwarding wrappers" + "".join("\tdirect calls in %s" % k for k in (gcounts or {})) + "\n")
            for n in sorted(nr_names):
                d = decl[n]
                hs = ";".join("%s:%d%s" % (x["header"], x["line"], "" if x["always"] else "[macro-conditional]") for x in d)
                vis = "yes" if any(x["header"] in ghdr for x in d) else "no"
                mine = [i for i in impls if i[1] == n]
                row = [n, hs, vis, str(len(mine)), str(sum(1 for i in mine if i[4])), str(sum(1 for x in forwards if x[0] == n))]
                for k, (calls, other) in (gcounts or {}).items():
                    row.append(str(calls.get(n, 0)))
                f.write("\t".join(row) + "\n")
        with open(os.path.join(a.table, "definitions.tsv"), "w") as f:
            f.write("kind\tname\tfile:line\tcan return\n")
            for (k, n, fl, ln, r) in sorted(impls, key=lambda x: (x[1], x[2], x[3])):
                f.write("%s\t%s\t%s:%d\t%s\n" % (k, n, fl, ln, "RETURNS" if r else "no"))
        with open(os.path.join(a.table, "direct-callers.tsv"), "w") as f:
            f.write("noreturn fle_imp_ function\tcalled by\tfile:line\n")
            for (c, fn, fl, ln) in sorted(set(callers)):
                f.write("%s\t%s\t%s:%d\n" % (c, fn, fl, ln))
        with open(os.path.join(a.table, "returning-implementations.tsv"), "w") as f:
            f.write("kind\tname\tfile:line\tdeclared in\tgen-visible\n")
            for (k, n, fl, ln, r) in sorted(returning, key=lambda x: (x[1], x[2])):
                d = decl[n]
                f.write("\t".join([k, n, "%s:%d" % (fl, ln), ";".join("%s:%d%s" % (x["header"], x["line"], "" if x["always"] else "[macro-conditional]") for x in d), "yes" if any(x["header"] in ghdr for x in d) else "no"]) + "\n")
        with open(os.path.join(a.table, "reverse-direction.tsv"), "w") as f:
            f.write("function (strong definition, no ret in the IR, not declared noreturn)\tfile:line\n")
            for (n, fl, ln) in sorted(set(reverse)):
                f.write("%s\t%s:%d\n" % (n, fl, ln))
        with open(os.path.join(a.table, "wrap-noreturn-of-returning.tsv"), "w") as f:
            f.write("wrapped function (declared as returning)\t__wrap_ definition that has no ret\n")
            for (n, fl, ln) in sorted(set(wrap_noret)):
                f.write("%s\t%s:%d\n" % (n, fl, ln))
        with open(os.path.join(a.table, "forwarding-wrappers.tsv"), "w") as f:
            f.write("function (declared noreturn)\t__wrap_ that forwards the real function to a helper (not a finding)\n")
            for (n, fl, ln) in sorted(set(forwards)):
                f.write("%s\t%s:%d\n" % (n, fl, ln))
        with open(os.path.join(a.table, "unanalysed.tsv"), "w") as f:
            f.write("file that mentions a noreturn name or pastes a name with ##\treason\n")
            for (fl, why) in sorted(unanalysed):
                f.write("%s\t%s\n" % (fl, why))
            for (fl, why) in sorted(unanalysed_rt):
                f.write("%s\t(not a finding) %s\n" % (fl, why))
        with open(os.path.join(a.table, "summary.txt"), "w") as f:
            f.write("headers parsed: %d (fragments, not self-contained and without 'noreturn': %s)\nnames declared noreturn: %d (%d in a header of the generated include closure)\n" % (len(hdrs), " ".join(sorted(fragments)) or "none", len(nr_names), sum(1 for n in nr_names if any(x["header"] in ghdr for x in decl[n]))))
            f.write("generated include closure: %s\n" % " ".join(sorted(ghdr)))
            f.write("C files analysed (IR): %d (runtime %d, harness %d, written by the %d wrapper generators %d); unanalysed with a noreturn name: %d, without one: %d\n" % (
                n_analysed, sum(1 for p in todo if p.startswith(runtime + os.sep)), sum(1 for p in todo if p in cand_set and not p.startswith(runtime + os.sep)), 3, len(gen_files), len(unanalysed), len(unanalysed_rt)))
            f.write("definitions of noreturn names found: %d, of which can return: %d\n" % (len(impls), len(returning)))
            f.write("reverse direction (no ret, not declared noreturn): %d fle_imp_ definitions; wraps with no ret of a returning API: %d\n" % (len(set(reverse)), len(set(wrap_noret))))
            f.write("findings: %d, outside the allow list: %d, stale allow rows: %d\n" % (len(findings), len(bad), len(stale)))

    ok = not bad and not stale
    if not a.quiet or not ok:
        print("noreturn_audit: %d noreturn names (%d gen-visible), %d C files analysed, %d unanalysed, %d definitions, %d can return, %d findings, %d outside the allow list, %d stale allow rows"
              % (len(nr_names), sum(1 for n in nr_names if any(x["header"] in ghdr for x in decl[n])), n_analysed, len(unanalysed), len(impls), len(returning), len(findings), len(bad), len(stale)))
    for (k, n, w, t) in bad:
        print("NEW FINDING [%s] %s @ %s: %s" % (k, n, w, t))
    for (k, n, w, reason, ln) in stale:
        print("STALE ALLOW ROW (line %d) [%s] %s @ %s: nothing matches it any more (%s)" % (ln, k, n, w, reason[:80]))
    return 0 if ok else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except SystemExit:
        raise
    except Exception as e:  # the tool itself failed
        import traceback
        traceback.print_exc()
        sys.exit(2)
