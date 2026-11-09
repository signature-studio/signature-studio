"""gdb helper (read-only): run the two-module binary until a fle_imp_* breakpoint fires N times, then print guest memory.
env: GDBI_BIN GDBI_REAL GDBI_FLE as gdb_imports.py; MEM_BP=<fle_imp name>; MEM_SKIP=<hits to skip>; MEM_DUMP="addr:len,addr:len" (hex)
Also prints the C backtrace up to 6 frames. Nothing is changed."""
import os

import gdb

gdb.execute("set pagination off")
gdb.execute("set confirm off")
gdb.execute("file " + os.environ["GDBI_BIN"])
gdb.execute("set args %s %s > /dev/null 2>&1" % (os.environ["GDBI_FLE"], os.environ["GDBI_REAL"]))
gdb.execute("break " + os.environ["MEM_BP"])
gdb.execute("run")
for _ in range(int(os.environ.get("MEM_SKIP", "0"))):
    gdb.execute("continue")


def ld(a, n):
    t = {1: "unsigned char", 2: "unsigned short", 4: "unsigned int", 8: "unsigned long long"}[n]
    return int(gdb.parse_and_eval("*(%s*)%d" % (t, a)))


for spec in os.environ.get("MEM_DUMP", "").split(","):
    if not spec:
        continue
    a, n = spec.split(":")
    a, n = int(a, 16), int(n, 16)
    row = " ".join("%016x" % ld(a + k, 8) for k in range(0, n, 8))
    print("mem %#x+%#x: %s" % (a, n, row))
gdb.execute("bt 6")
gdb.execute("kill")
