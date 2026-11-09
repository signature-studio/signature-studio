"""gdb helper (measurement): run to the point where DllMain of the real module has returned success (fle_realmod.c: `m->state = ST_LOADED`,
with the TEMPORARY patches of GDBI_PATCHES), then call ONE lifted function of the real module from gdb with a clean guest stack and report
what happens: it returns, it stops by name (runtime message on stderr), or a breakpoint at PROBE_BP is reached.
env: GDBI_BIN GDBI_REAL GDBI_FLE GDBI_PATCHES CHAIN_MAX GDBI_STDERR as gdb_imports.py; PROBE_FN=f_27436d0; PROBE_BP=f_27470e0 (optional);
     RET_BP=fle_realmod.c:144; PROBE_MEM="4541000:4,..." (guest words printed first)
The arguments of the call are whatever the guest registers hold (rcx.. are not set): this probes the CPU-selector stub (decision 0079), not the
IPP function behind it. Nothing in the program is changed apart from the call."""
import contextlib
import io
import os
import re

import gdb

gdb.execute("set pagination off")
gdb.execute("set confirm off")
gdb.execute("file " + os.environ["GDBI_BIN"])
se = os.environ.get("GDBI_STDERR", "/dev/null")
gdb.execute("set args %s %s > /dev/null 2> %s" % (os.environ["GDBI_FLE"], os.environ["GDBI_REAL"], se))
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


class Imp(gdb.Breakpoint):
    def __init__(self, n):
        super().__init__(n)
        self.nm = n

    def stop(self):
        if not state["in"]:
            return False
        p = PATCHES.get(self.nm)
        if p:
            with contextlib.redirect_stdout(io.StringIO()):
                r = p(G)
            if r == "skip":
                state["skip"] = True
                return True
        return False


class Entry(gdb.Breakpoint):
    def stop(self):
        state["in"] = True
        return False


class Done(gdb.Breakpoint):
    pass


txt = gdb.execute("info functions ^fle_imp_", to_string=True)
for n in sorted(set(re.findall(r"\b(fle_imp_[A-Za-z0-9_]+)\(", txt))):
    Imp(n)
Entry("f_37caca8")
done = Done(os.environ.get("RET_BP", "fle_realmod.c:144"))
gdb.execute("run")
while True:
    hit = [b for b in gdb.breakpoints() if b is done and b.hit_count > 0]
    if hit:
        break
    if state.get("skip"):
        state["skip"] = False
        gdb.execute("return")
        gdb.execute("continue")
        continue
    print("PROBE: the run stopped before DllMain returned")
    raise SystemExit(1)

for spec in os.environ.get("PROBE_MEM", "").split(","):
    if spec:
        a, n = spec.split(":")
        print("PROBE mem %#x = %#x" % (int(a, 16), G.ld(int(a, 16), int(n))))
fn = os.environ["PROBE_FN"]
sp = (reg(4) - 0x400) & ~0xF
gdb.execute("set var R[4] = %d" % (sp - 8))
G.st(sp - 8, 8, 0xdead1234)
if os.environ.get("PROBE_BP"):
    gdb.Breakpoint(os.environ["PROBE_BP"])
try:
    for one in fn.split(","):
        gdb.execute("set var R[4] = %d" % (sp - 8))
        G.st(sp - 8, 8, 0xdead1234)
        gdb.execute("call %s()" % one)
        print("PROBE %s: returned normally (R[4]=%#x = sp-8 = %#x: the lifted ret leaves R[4] at the return-address slot)" % (one, reg(4), sp - 8))
        for spec in os.environ.get("PROBE_MEM", "").split(","):
            if spec:
                a, n = spec.split(":")
                print("PROBE   mem %#x = %#x" % (int(a, 16), G.ld(int(a, 16), int(n))))
except gdb.error as e:
    print("PROBE %s: %s" % (fn, str(e).splitlines()[0]))
    try:
        print("PROBE frame: %s" % gdb.execute("bt 3", to_string=True).splitlines()[0:3])
    except gdb.error:
        pass
gdb.execute("kill")
