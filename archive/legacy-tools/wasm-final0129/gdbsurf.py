# wasm-final0129 (decision 0134): measurement only. gdb -x gdbsurf.py <two-native>. A copy of idle1's tools/gdbsurf.py that dumps the BGRX bytes of the drawing surface of top-level window lid 13 at EVERY call of describe_view
# (runtime/fle_raster.c: called inside fle_raster_wrote right after the write that makes the first non-background pixel, with the surface in that state, and by fle_raster_report at the stop), tagged hit1 = first pixel, hit2 = stop,  The layout it reads (wsurf of runtime/fle_raster.c: lid u32 @0,
# live u8 @4, w u32 @8, h u32 @12, bits pointer @16) is checked: lid must be 13, live non-zero, w x h = 1930 x 1092, or nothing is written.
# environment: GDBSURF_BIN (the same binary, for nm), GDBSURF_OUT (directory), GDBSURF_FLE, GDBSURF_REAL, GDBSURF_STDOUT, GDBSURF_STDERR.
import gdb
import os
import re
import subprocess

BIN = os.environ["GDBSURF_BIN"]
OUT = os.environ["GDBSURF_OUT"]
os.makedirs(OUT, exist_ok=True)
LOG = open(os.path.join(OUT, "gdbsurf.txt"), "w")


def log(s):
    LOG.write(s + "\n")
    LOG.flush()


def static(sym):
    for line in subprocess.run(["nm", BIN], capture_output=True, text=True).stdout.splitlines():
        f = line.split()
        if len(f) == 3 and f[2] == sym:
            return int(f[0], 16)
    raise gdb.error("nm: no symbol " + sym)


def rd(addr, n):
    return bytes(gdb.selected_inferior().read_memory(addr, n))


def u(addr, n):
    return int.from_bytes(rd(addr, n), "little")


base = [None]


def rt(sym):
    if base[0] is None:
        m = re.search(r"is at (0x[0-9a-fA-F]+)", gdb.execute("info address main", to_string=True))
        base[0] = int(m.group(1), 16) - static("main")
    return base[0] + static(sym)


def dump(tag):
    wsp, wn = rt("ws"), rt("ws_n")
    n = u(wn, 4)
    arr = u(wsp, 8)
    log("%s: ws_n=%d ws=%#x" % (tag, n, arr))
    for k in range(n):
        s = u(arr + 8 * k, 8)
        lid, live, w, h, bits = u(s, 4), u(s + 4, 1), u(s + 8, 4), u(s + 12, 4), u(s + 16, 8)
        log("  surface #%d lid=%d live=%d %dx%d bits=%#x" % (k, lid, live, w, h, bits))
        if lid == 13 and live and w == 1930 and h == 1092:
            data = rd(bits, w * h * 4)
            path = os.path.join(OUT, tag + ".bgrx")
            open(path, "wb").write(data)
            nz = sum(1 for i in range(0, len(data), 4) if data[i:i + 3] != b"\0\0\0")
            log("  WROTE %s %d bytes, non-zero pixels %d" % (path, len(data), nz))
            return True
    log("  NOT WRITTEN: no live lid-13 surface of 1930x1092 found (the layout assumption failed or the surface is gone)")
    return False


class WinBp(gdb.Breakpoint):
    n = 0

    def stop(self):
        try:
            WinBp.n += 1
            dump("hit%d" % WinBp.n)
        except Exception as e:
            log("ERROR %s" % e)
        return False


for sig in ("SIGALRM", "SIGUSR1", "SIGUSR2", "SIGPIPE", "SIGABRT"):
    gdb.execute("handle %s nostop noprint pass" % sig, to_string=True)
gdb.execute("set pagination off")
gdb.execute("set confirm off")
bp = WinBp("describe_view")
gdb.execute('set args %s "%s" > %s 2> %s' % (os.environ["GDBSURF_FLE"], os.environ["GDBSURF_REAL"], os.environ["GDBSURF_STDOUT"], os.environ["GDBSURF_STDERR"]))
gdb.execute("run")
log("hits of describe_view: %d" % WinBp.n)
try:
    gdb.execute("kill")
except gdb.error:
    pass
