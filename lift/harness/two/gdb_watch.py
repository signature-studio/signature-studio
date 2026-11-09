"""gdb helper (read-only apart from the TEMPORARY patches of GDBI_PATCHES): log every write to guest memory words while DllMain of the real
module runs, using hardware watchpoints. Used to see which function writes the four CPU-selector variables (decision 0079).
env: GDBI_BIN GDBI_REAL GDBI_FLE GDBI_PATCHES CHAIN_MAX as gdb_imports.py; WATCH_ADDRS="4541000,455d050,45410a0,454f270" (hex, 4-byte words);
     WATCH_STOP_AT=<fle_imp name>  (stop the measurement when this API is first called after the writes, default LoadLibraryW of the guest)."""
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
state = {"in": False, "skip": False, "done": False}


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


def guest_frames():
    out = []
    f = gdb.newest_frame()
    while f is not None and len(out) < 8:
        n = f.name()
        if n and n.startswith("f_"):
            out.append(n)
        f = f.older()
    return out


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


class W(gdb.Breakpoint):
    def __init__(self, addr):
        super().__init__("*(unsigned int*)%#x" % addr, gdb.BP_WATCHPOINT, gdb.WP_WRITE)
        self.addr = addr

    def stop(self):
        if state["in"]:
            print("WRITE %#x = %#x   from %s" % (self.addr, G.ld(self.addr, 4), " <- ".join(guest_frames())))
        return False


txt = gdb.execute("info functions ^fle_imp_", to_string=True)
for n in sorted(set(re.findall(r"\b(fle_imp_[A-Za-z0-9_]+)\(", txt))):
    Imp(n)
Entry("f_37caca8")
gdb.execute("break main")
gdb.execute("run")
for a in os.environ["WATCH_ADDRS"].split(","):
    W(int(a, 16))
while True:
    gdb.execute("continue")
    try:
        alive = gdb.selected_inferior().pid != 0
    except gdb.error:
        alive = False
    if alive and state.get("skip"):
        state["skip"] = False
        gdb.execute("return")
        continue
    break
if alive:
    gdb.execute("kill")
