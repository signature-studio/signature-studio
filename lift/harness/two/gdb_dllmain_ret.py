"""gdb helper (read-only apart from the TEMPORARY patches given in GDBI_PATCHES, same file format as gdb_imports.py): print the value
that DllMain of the real module returns (R[0] when fle_call_guest returns, as seen by fle_realmod_load) and what fle_realmod_load returns.
env: GDBI_BIN GDBI_REAL GDBI_FLE GDBI_PATCHES CHAIN_MAX as gdb_imports.py / patches_chain.py."""
import contextlib
import io
import os
import re

import gdb

gdb.execute("set pagination off")
gdb.execute("set confirm off")
gdb.execute("file " + os.environ["GDBI_BIN"])
gdb.execute("set args %s %s > /dev/null 2>&1" % (os.environ["GDBI_FLE"], os.environ["GDBI_REAL"]))
ns = {}
exec(open(os.environ["GDBI_PATCHES"]).read(), ns)
PATCHES = ns["PATCHES"]
state = {"in": False}


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


class Ret(gdb.Breakpoint):
    """Breakpoint on the line of fle_realmod.c that follows the DllMain call (the `return` of call_dllmain)."""

    def stop(self):
        print("RESULT fle_call_guest returned: R[0]=%#x, DllMain eax=%#x (reason 1 = DLL_PROCESS_ATTACH)" % (reg(0), reg(0) & 0xffffffff))
        for spec in os.environ.get("RET_MEM", "").split(","):
            if spec:
                a, n = spec.split(":")
                a, n = int(a, 16), int(n)
                print("RESULT mem %#x (%d bytes) = %#x" % (a, n, G.ld(a, n)))
        return False


txt = gdb.execute("info functions ^fle_imp_", to_string=True)
for n in sorted(set(re.findall(r"\b(fle_imp_[A-Za-z0-9_]+)\(", txt))):
    Imp(n)
Entry("f_37caca8")
Ret(os.environ.get("RET_BP", "fle_realmod.c:117"))
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
if alive:
    print("stopped:", gdb.execute("bt 3", to_string=True).splitlines()[:3])
    gdb.execute("kill")
