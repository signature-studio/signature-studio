"""gdb helper (measurement only): trace the fle_imp_* calls made while the real module's DllMain runs, report the stop, and (optionally)
apply TEMPORARY values at chosen APIs so that the run can go on to the next stop.
usage: gdb -q -batch -x gdb_imports.py
env:   GDBI_BIN=<two-native>  GDBI_REAL=<rebased dll>  GDBI_FLE=<FLEngine dll>  GDBI_OUT=<log file, must not exist>
       GDBI_STDOUT=<file for the program's stdout (stop report)>  GDBI_STDERR=<file for its stderr (the "fle: ..." line)>
       GDBI_ENTRY=f_37caca8   (DllMain of the real module: logging starts when it is entered)
       GDBI_PATCHES=<python file> defining PATCHES = { "fle_imp_X": function(gdbi) -> None | "skip" }  (optional)
A patch runs when fle_imp_X is entered (after DllMain entry). It may write guest memory or registers with the helpers of `gdbi`
(g.reg(i), g.setreg(i, v), g.st(addr, n, v), g.ld(addr, n)); returning "skip" returns from fle_imp_X without running it (R[0] is whatever
the patch set). Every patch hit is logged as PATCH. A value put in by a patch is a measurement aid, not a decision and not the runtime's
behaviour. Nothing in the program's source is changed."""
import os
import re

import gdb

binp = os.environ["GDBI_BIN"]
real = os.environ["GDBI_REAL"]
outp = os.environ.get("GDBI_OUT")
entry_sym = os.environ.get("GDBI_ENTRY", "f_37caca8")
out = open(outp, "x") if outp else None


def log(s):
    if out:
        out.write(s + "\n")
        out.flush()
    else:
        gdb.write(s + "\n")


class G:
    @staticmethod
    def reg(i):
        return int(gdb.parse_and_eval("R[%d]" % i))

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

    @staticmethod
    def wstr(a, maxn=80):
        s = ""
        for k in range(maxn):
            c = G.ld(a + 2 * k, 2)
            if c == 0:
                break
            s += chr(c) if 32 <= c < 127 else "\\u%04x" % c
        return s

    @staticmethod
    def astr(a, maxn=80):
        s = ""
        for k in range(maxn):
            c = G.ld(a + k, 1)
            if c == 0:
                break
            s += chr(c) if 32 <= c < 127 else "\\x%02x" % c
        return s


patches = {}
if os.environ.get("GDBI_PATCHES"):
    ns = {}
    exec(open(os.environ["GDBI_PATCHES"]).read(), ns)
    patches = ns["PATCHES"]

gdb.execute("set pagination off")
gdb.execute("set confirm off")
gdb.execute("set print thread-events off")
gdb.execute("file " + binp)
so = os.environ.get("GDBI_STDOUT", "/dev/null")
se = os.environ.get("GDBI_STDERR", "/dev/null")
gdb.execute("set args %s %s > %s 2> %s" % (os.environ.get("GDBI_FLE", "../binaries/win64/FLEngine_x64.dll"), real, so, se))
state = {"in": False, "n": 0}


class Entry(gdb.Breakpoint):
    def stop(self):
        state["in"] = True
        log("DllMain entered (%s) rcx=%#x rdx=%d r8=%d" % (entry_sym, G.reg(1), G.reg(2), G.reg(8)))
        return False


class GpaDone(gdb.FinishBreakpoint):
    """Logs the value that fle_imp_GetProcAddress returned (R[0]) for a call on the real module (hModule = its base)."""

    def __init__(self, k, name_ptr):
        super().__init__(gdb.newest_frame(), internal=True)
        self.k, self.name_ptr = k, name_ptr

    def stop(self):
        log("     -> GetProcAddress #%d returned R[0]=%#x" % (self.k, G.reg(0)))
        return False

    def out_of_scope(self):
        pass


class Imp(gdb.Breakpoint):
    def __init__(self, name):
        super().__init__(name, internal=False)
        self.nm = name

    def stop(self):
        if not state["in"]:
            return False
        state["n"] += 1
        try:
            ret = G.ld(G.reg(4), 8)
        except gdb.error:
            ret = -1
        log("%4d %s rcx=%#x rdx=%#x r8=%#x r9=%#x ret=%#x" % (state["n"], self.nm, G.reg(1), G.reg(2), G.reg(8), G.reg(9), ret))
        if self.nm == "fle_imp_GetProcAddress" and os.environ.get("GDBI_GPA") == "1":
            GpaDone(state["n"], G.reg(2))
        p = patches.get(self.nm)
        if p:
            r = p(G)
            log("     HOOK %s ran%s   (the patch prints its own '(stage N)' note when its guard let it act)" % (self.nm, "; skip: fle_imp returns at once" if r == "skip" else ""))
            if r == "skip":
                state["skip"] = True
                return True    # stop here; the main loop pops the frame and continues
        return False


txt = gdb.execute("info functions ^fle_imp_", to_string=True)
names = sorted(set(re.findall(r"\b(fle_imp_[A-Za-z0-9_]+)\(", txt)))
log("breakpoints on %d fle_imp_* functions" % len(names))
Entry(entry_sym)
for n in names:
    Imp(n)
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
sig = alive
if sig:
    log("process stopped (signal). guest/C backtrace:")
    log(gdb.execute("bt 14", to_string=True))
    log("guest registers at the stop: rcx=%#x rdx=%#x r8=%#x r9=%#x rsp=%#x rax=%#x" % (G.reg(1), G.reg(2), G.reg(8), G.reg(9), G.reg(4), G.reg(0)))
    if os.environ.get("GDBI_DELIVER", "1") == "1":
        # deliver the SIGABRT to the program so that its own handler (trace.c) prints the stop report to GDBI_STDOUT
        gdb.execute("signal SIGABRT")
    else:
        gdb.execute("kill")
log("end: %d fle_imp_* calls after DllMain entry" % state["n"])
