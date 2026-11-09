"""gdb helper (measurement): count how many times each of the given lifted functions is entered in a chain run (the TEMPORARY patches of
GDBI_PATCHES apply exactly as in gdb_imports.py). Prints one line per function with its hit count and the C backtrace head at the stop.
env: GDBI_BIN GDBI_REAL GDBI_FLE GDBI_PATCHES CHAIN_MAX CHAIN_ABSENT as gdb_imports.py; HITS_FUNCS="f_3d04cf0,f_3d3a420,..." """
import contextlib
import io
import os
import re

import gdb

gdb.execute("set pagination off")
gdb.execute("set confirm off")
gdb.execute("file " + os.environ["GDBI_BIN"])
gdb.execute("set args %s %s > /dev/null 2> /dev/null" % (os.environ["GDBI_FLE"], os.environ["GDBI_REAL"]))
ns = {}
exec(open(os.environ["GDBI_PATCHES"]).read(), ns)
PATCHES = ns["PATCHES"]
state = {"in": False, "skip": False}


def reg(i):
    return int(gdb.parse_and_eval("R[%d]" % i))


class G:
    reg = staticmethod(reg)

    @staticmethod
    def setreg(i, v):
        gdb.execute("set var R[%d] = %d" % (i, v))

    @staticmethod
    def ld(a, n):
        t = {1: "unsigned char", 2: "unsigned short", 4: "unsigned int", 8: "unsigned long long"}[n]
        return int(gdb.parse_and_eval("*(%s*)%d" % (t, a)))

    @staticmethod
    def st(a, n, v):
        t = {1: "unsigned char", 2: "unsigned short", 4: "unsigned int", 8: "unsigned long long"}[n]
        gdb.execute("set var *(%s*)%d = %d" % (t, a, v))


notes = []


class Imp(gdb.Breakpoint):
    def __init__(self, n):
        super().__init__(n)
        self.nm = n

    def stop(self):
        if not state["in"]:
            return False
        p = PATCHES.get(self.nm)
        if p:
            buf = io.StringIO()
            with contextlib.redirect_stdout(buf):
                r = p(G)
            if buf.getvalue():
                notes.append(buf.getvalue().strip().split("(stage ")[1][0] if "(stage " in buf.getvalue() else "?")
            if r == "skip":
                state["skip"] = True
                return True
        return False


class Entry(gdb.Breakpoint):
    def stop(self):
        state["in"] = True
        return False


txt = gdb.execute("info functions ^fle_imp_", to_string=True)
for n in sorted(set(re.findall(r"\b(fle_imp_[A-Za-z0-9_]+)\(", txt))):
    Imp(n)
Entry("f_37caca8")
class Count(gdb.Breakpoint):
    """Counts entries of a lifted function and never stops the program (stop() returns False)."""

    def __init__(self, f):
        super().__init__(f)
        self.n = 0
        self.f = f

    def stop(self):
        if state["in"]:
            self.n += 1
        return False


fb = [Count(f) for f in os.environ.get("HITS_FUNCS", "").split(",") if f]
gdb.execute("run")
while True:
    try:
        alive = gdb.selected_inferior().pid != 0
    except gdb.error:
        alive = False
    if alive and state.get("skip"):
        state["skip"] = False
        gdb.execute("return")
        gdb.execute("continue")
        continue
    break
for b in fb:
    print("HITS %-14s %d" % (b.f, b.n))
import collections
print("PATCH-ACTED per stage (the guard let the patch act):", dict(sorted(collections.Counter(notes).items())))
if alive:
    gdb.execute("kill")
