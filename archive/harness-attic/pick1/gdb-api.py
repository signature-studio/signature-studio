# pick1 (decision 0242): count the runtime import calls of a NATIVE N1 binary with gdb (no rebuild). Used until the recorder build (apiw_core.c) exists.
#   gdb -q -batch -x gdb-api.py   (env GDBAPI_OUT = output file, GDBAPI_NAMES = file with the fle_imp_ names, one per line)
# Every import gets a breakpoint with a huge ignore count (gdb counts the hits); the probe / event APIs are logged with their arguments (Python stop(), returns False = the guest keeps running).
# `getrusage` (the G3 `cpu LABEL` line) dumps the cumulative counts. THE INPUT IS A HARNESS SIMULATION (G3 script).
import gdb, os, re, struct
OUT = open(os.environ['GDBAPI_OUT'], 'w')
names = [l.strip() for l in open(os.environ['GDBAPI_NAMES']) if l.strip()]
inf = gdb.selected_inferior()
RB = [0]
R_STATIC = int(gdb.parse_and_eval('(unsigned long)&R')); MAIN_STATIC = int(gdb.parse_and_eval('(unsigned long)&main'))
def reg(i):
    if not RB[0]: RB[0] = R_STATIC + int(gdb.parse_and_eval('(unsigned long)&main')) - MAIN_STATIC
    return struct.unpack('<Q', bytes(inf.read_memory(RB[0] + 8 * i, 8)))[0]
def rd(a, n):
    try: return bytes(inf.read_memory(a, n))
    except Exception: return None
def wstr(a, cap=60):
    if a < 0x10000: return '#atom%d' % a
    o = ''
    for i in range(cap):
        b = rd(a + 2 * i, 2)
        if b is None: return o or '(unreadable)'
        c = struct.unpack('<H', b)[0]
        if c == 0: break
        o += chr(c) if 0x20 <= c < 0x7f else '?'
    return o
def astr(a, cap=60):
    if a < 0x10000: return '#ord%d' % a
    o = ''
    for i in range(cap):
        b = rd(a + i, 1)
        if b is None: return o or '(unreadable)'
        if b[0] == 0: break
        o += chr(b[0]) if 0x20 <= b[0] < 0x7f else '?'
    return o
def guid(a):
    b = rd(a, 16)
    if b is None: return '(unreadable)'
    return '{%08X-%04X-%04X-%s-%s}' % (struct.unpack('<I', b[:4])[0], struct.unpack('<H', b[4:6])[0], struct.unpack('<H', b[6:8])[0], b[8:10].hex().upper(), b[10:].hex().upper())
def arg(n):
    if n <= 4: return reg([1, 2, 8, 9][n - 1])
    b = rd(reg(4) + 0x28 + 8 * (n - 5), 8)
    return struct.unpack('<Q', b)[0] if b else 0
PROBE = {
 'LoadLibraryW': lambda: '"%s"' % wstr(arg(1)), 'LoadLibraryA': lambda: '"%s"' % astr(arg(1)), 'LoadLibraryExW': lambda: '"%s"' % wstr(arg(1)), 'LoadLibraryExA': lambda: '"%s"' % astr(arg(1)),
 'GetModuleHandleW': lambda: '"%s"' % wstr(arg(1)), 'GetModuleHandleA': lambda: '"%s"' % astr(arg(1)), 'GetModuleHandleExW': lambda: 'flags=%#x "%s"' % (arg(1), wstr(arg(3))),
 'GetProcAddress': lambda: '"%s"' % astr(arg(2)), 'CoCreateInstance': lambda: 'clsid=%s riid=%s ctx=%#x' % (guid(arg(1)), guid(arg(4)), arg(3)),
 'RegOpenKeyExW': lambda: '"%s"' % wstr(arg(2)), 'RegQueryValueExW': lambda: '"%s"' % wstr(arg(2)), 'RegCreateKeyExW': lambda: '"%s"' % wstr(arg(2)), 'RegGetValueW': lambda: '"%s" "%s"' % (wstr(arg(2)), wstr(arg(3))),
 'SystemParametersInfoW': lambda: 'action=%#x ui=%d' % (arg(1) & 0xffffffff, arg(2) & 0xffffffff), 'SystemParametersInfoA': lambda: 'action=%#x ui=%d' % (arg(1) & 0xffffffff, arg(2) & 0xffffffff),
 'GetSystemMetrics': lambda: 'idx=%d' % (arg(1) & 0xffffffff), 'GetSystemMetricsForDpi': lambda: 'idx=%d' % (arg(1) & 0xffffffff), 'GetDeviceCaps': lambda: 'idx=%d' % (arg(2) & 0xffffffff),
 'IsProcessorFeaturePresent': lambda: 'idx=%d' % (arg(1) & 0xffffffff), 'GetSysColor': lambda: 'idx=%d' % (arg(1) & 0xffffffff), 'GetStockObject': lambda: 'idx=%d' % (arg(1) & 0xffffffff),
 'GetEnvironmentVariableW': lambda: '"%s"' % wstr(arg(1)), 'OpenThemeData': lambda: '"%s"' % wstr(arg(2)), 'CLSIDFromProgID': lambda: '"%s"' % wstr(arg(1)),
 'DwmIsCompositionEnabled': lambda: 'p', 'DwmExtendFrameIntoClientArea': lambda: 'h=%#x' % arg(1), 'DwmEnableBlurBehindWindow': lambda: 'h=%#x' % arg(1), 'DwmGetColorizationColor': lambda: '', 'DwmGetWindowAttribute': lambda: 'attr=%d' % (arg(2) & 0xffffffff), 'DwmSetWindowAttribute': lambda: 'attr=%d' % (arg(2) & 0xffffffff),
 'IsThemeActive': lambda: '', 'IsAppThemed': lambda: '', 'BufferedPaintInit': lambda: '', 'BufferedPaintUnInit': lambda: '', 'BeginBufferedPaint': lambda: '', 'EndBufferedPaint': lambda: '',
 'IsProcessDPIAware': lambda: '', 'SetProcessDPIAware': lambda: '', 'GetDpiForMonitor': lambda: 'type=%d' % (arg(2) & 0xffffffff), 'GetProcessDpiAwareness': lambda: '', 'SetThreadDpiAwarenessContext': lambda: '',
 'GetVersionExW': lambda: '', 'GetVersionExA': lambda: '', 'RtlGetVersion': lambda: '', 'VerifyVersionInfoW': lambda: '', 'GetMonitorInfoW': lambda: '', 'EnumDisplayMonitors': lambda: '', 'MonitorFromWindow': lambda: '', 'MonitorFromPoint': lambda: '',
 'GetKeyboardLayout': lambda: '', 'GetACP': lambda: '', 'IsDebuggerPresent': lambda: '', 'GetSystemInfo': lambda: '', 'GetNativeSystemInfo': lambda: '', 'IsWow64Process': lambda: '', 'WICConvertBitmapSource': lambda: '',
 'CreateWindowExW': lambda: 'ex=%#x style=%#x class="%s" title="%s" x=%d y=%d w=%d h=%d parent=%#x' % (arg(1) & 0xffffffff, arg(4) & 0xffffffff, wstr(arg(2)), wstr(arg(3), 30), ctypes32(arg(5)), ctypes32(arg(6)), ctypes32(arg(7)), ctypes32(arg(8)), arg(9)),
 'DestroyWindow': lambda: 'h=%#x' % arg(1), 'ShowWindow': lambda: 'h=%#x cmd=%d' % (arg(1), arg(2) & 0xffffffff),
 'SetWindowPos': lambda: 'h=%#x after=%#x x=%d y=%d cx=%d cy=%d flags=%#x' % (arg(1), arg(2), ctypes32(arg(3)), ctypes32(arg(4)), ctypes32(arg(5)), ctypes32(arg(6)), arg(7) & 0xffffffff),
 'UpdateLayeredWindow': lambda: 'h=%#x hdcSrc=%#x' % (arg(1), arg(5)), 'UpdateLayeredWindowIndirect': lambda: 'h=%#x' % arg(1), 'SetLayeredWindowAttributes': lambda: 'h=%#x key=%#x alpha=%d flags=%#x' % (arg(1), arg(2) & 0xffffffff, arg(3) & 0xff, arg(4) & 0xffffffff),
 'SetParent': lambda: 'h=%#x parent=%#x' % (arg(1), arg(2)), 'EnableWindow': lambda: 'h=%#x en=%d' % (arg(1), arg(2) & 0xffffffff), 'SetForegroundWindow': lambda: 'h=%#x' % arg(1), 'AlphaBlend': lambda: '', 'GradientFill': lambda: '',
 'SetWindowRgn': lambda: 'h=%#x' % arg(1), 'AnimateWindow': lambda: 'h=%#x' % arg(1),
}
CLK=[None]
def clock():
    return 0
def ctypes32(v):
    v &= 0xffffffff
    return v - (1 << 32) if v >= (1 << 31) else v
EVENT = {'CreateWindowExW', 'DestroyWindow', 'ShowWindow', 'SetWindowPos', 'UpdateLayeredWindow', 'UpdateLayeredWindowIndirect', 'SetLayeredWindowAttributes', 'SetParent', 'EnableWindow', 'AlphaBlend', 'GradientFill', 'SetWindowRgn', 'AnimateWindow', 'DwmEnableBlurBehindWindow', 'DwmExtendFrameIntoClientArea'}
for _n in ('SetWindowLongW', 'SetWindowLongA', 'SetWindowLongPtrW', 'SetWindowLongPtrA', 'SetClassLongPtrW', 'SetClassLongW'):
    PROBE[_n] = lambda: 'h=%#x idx=%d val=%#x' % (arg(1), ctypes32(arg(2)), arg(3))
EVENT.update({'SetWindowLongW', 'SetWindowLongA', 'SetWindowLongPtrW', 'SetWindowLongPtrA', 'SetClassLongPtrW', 'SetClassLongW'})
class Fin(gdb.FinishBreakpoint):
    def __init__(self, frame, owner, txt, ra):
        super().__init__(frame, internal=True); self.owner = owner; self.txt = txt; self.ra = ra
    def stop(self):
        try: ret = reg(0)
        except Exception: ret = -1
        self.owner.done(self.txt, self.ra, ret)
        return False
    def out_of_scope(self):
        self.owner.done(self.txt, self.ra, None)
class Log(gdb.Breakpoint):
    def __init__(self, short, fn):
        super().__init__(fn, internal=True); self.short = short; self.n = 0; self.rows = {}
    def done(self, txt, ra, ret):
        key = '%s ra=%#x ret=%s' % (txt, ra, ('%#x' % ret) if ret is not None else '?')
        self.rows[key] = self.rows.get(key, 0) + 1
        OUT.write('%s %d %s %s\n' % ('E' if self.short in EVENT else 'R', clock(), self.short, key))
    def stop(self):
        self.n += 1
        try:
            txt = PROBE[self.short]()
            ra = struct.unpack('<Q', rd(reg(4), 8) or b'\0' * 8)[0]
            Fin(gdb.newest_frame(), self, txt, ra)
        except Exception as e:
            OUT.write('# error %s %s\n' % (self.short, e))
        return False
class Cnt(gdb.Breakpoint):
    def __init__(self, short, fn):
        super().__init__(fn, internal=True); self.short = short; self.n = 0; self.ignore_count = 1 << 30
class Dump(gdb.Breakpoint):
    def __init__(self):
        super().__init__('getrusage', internal=True); self.k = 0
    def stop(self):
        self.k += 1
        OUT.write('#dump %d\n' % self.k)
        for b in bps.values():
            if isinstance(b, Cnt):
                h = b.hit_count
                if h != last.get(b.short, 0): OUT.write('A %s %d\n' % (b.short, h)); last[b.short] = h
            else:
                if b.n != last.get(b.short, 0): OUT.write('A %s %d\n' % (b.short, b.n)); last[b.short] = b.n
                for k, v in b.rows.items():
                    if last.get((b.short, k), 0) != v: OUT.write('P %s %d %s\n' % (b.short, v, k)); last[(b.short, k)] = v
        OUT.flush()
        return False
bps = {}; last = {}
gdb.execute('set disable-randomization on'); gdb.execute('set pagination off'); gdb.execute('set confirm off'); gdb.execute('handle SIGPROF nostop noprint pass')
gdb.execute('set breakpoint pending on')
for fn in names:
    short = fn[len('fle_imp_'):]
    if short in PROBE: bps[short] = Log(short, fn)
    elif os.environ.get('GDBAPI_COUNT_ALL'): bps[short] = Cnt(short, fn)
dumper = Dump()
try:
    gdb.execute('run')
except Exception as e:
    OUT.write('# run ended: %s\n' % e)
dumper.stop()
OUT.write('# end\n'); OUT.close()
gdb.execute('kill') if False else None
