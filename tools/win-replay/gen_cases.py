#!/usr/bin/env python3
"""gen_cases.py - build cases.tsv for replay.exe from the recorded 6-module trace.

  python gen_cases.py [--trace TRACE.tsv.gz] [--apis apis.tsv] [--gpa getprocaddress.tsv] [--out cases.tsv]

Inputs (all in this repository, docs/investigations/a4-six-module-trace/):
  trace.tsv.gz                 13,572 calls: api, decoded arguments, return value, last error, decoded outputs
  tools/apis.tsv               argument types per API (letters, see replay.c)
  summary/getprocaddress.tsv   (module, function) pairs the guest asked GetProcAddress for

Every row of cases.tsv is one call that replay.exe makes. kind:
  trace  the argument shape was seen in the trace (pointer arguments are replaced by output buffers
         of the size the trace's own count argument gives); trace_* columns hold what the project's runtime answered
  scn    one step of a scenario modelled on a traced sequence (handles are $variables; the trace columns
         hold the answers of the same step shape with handle values folded to H)
  probe  not in the trace: it samples the real Windows behaviour that a 'trace' row depends on
The trace has no record of the contents of struct/buffer inputs ('ptr'); where a call needs one,
the generator builds it from the documented struct layout and says so in the note column.
"""
import argparse, collections, csv, gzip, json, os, re, sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, '..', '..'))
TRDIR = os.path.join(REPO, 'docs', 'investigations', 'a4-six-module-trace')
csv.field_size_limit(10 ** 9)

# ---------------------------------------------------------------- class of each API: how a real-Windows value is used
# M = the value is defined by Windows (a spec fact the runtime can be checked against; part of a call may still be environment)
# R = machine / user / time / screen / process-start dependent: recorded as reference only
# L = depends on the locale / ANSI code page of the VM: reference unless the VM is en-US / 1252
# K = not called (reason in KREASON)
CLS = {}
def setcls(c, names):
    for n in names.split():
        CLS[n] = c
setcls('M', '''WideCharToMultiByte MultiByteToWideChar CompareStringW LCMapStringW GetStringTypeW GetLocaleInfoW IsValidLocale GetACP GetCPInfo
 IsValidCodePage GetThreadPreferredUILanguages GetSystemDefaultLocaleName MulDiv GetVersion GetVersionExW GetVersionExA VerifyVersionInfoW VerSetConditionMask
 SetThreadLocale SetErrorMode SetThreadErrorMode SetHandleCount SetDllDirectoryW GetModuleHandleW GetModuleHandleA GetProcAddress LoadLibraryW LoadLibraryA LoadLibraryExW
 FreeLibrary HeapAlloc HeapReAlloc HeapSize HeapFree HeapCreate HeapSetInformation LocalAlloc VirtualAlloc VirtualProtect TlsAlloc TlsSetValue TlsGetValue FlsAlloc
 FlsSetValue FlsGetValue InitializeCriticalSection InitializeCriticalSectionAndSpinCount EnterCriticalSection LeaveCriticalSection DeleteCriticalSection
 AcquireSRWLockExclusive ReleaseSRWLockExclusive InitializeSListHead EncodePointer DecodePointer GetFileAttributesW GetFileAttributesExW FindFirstFileW
 GetFileVersionInfoSizeW RegOpenKeyExW RegOpenKeyExA RegCloseKey SHGetMalloc CharLowerBuffW CharUpperBuffW CharLowerW CharNextW LoadStringW LoadImageW LoadIconW
 LoadCursorW DestroyCursor CreateIconIndirect IntersectRect GetClassInfoW GetClipboardFormatNameW GetDC ReleaseDC GetStockObject CreateCompatibleDC DeleteDC DeleteObject
 SelectObject SelectPalette GetObjectW CreateBitmap CreateBrushIndirect CreatePenIndirect CreatePalette CreateFontIndirectW CreateDIBSection GdiFlush
 GetCurrentPositionEx MoveToEx GetDIBColorTable PatBlt StretchDIBits SetBkColor SetTextColor SetBkMode SetROP2 SetStretchBltMode UnrealizeObject SysAllocStringLen
 SysReAllocStringLen SysFreeString OleInitialize FreeEnvironmentStringsW GetCurrentProcess GetCurrentThread GetLastError SetLastError GetSystemMetrics
 SystemParametersInfoW GetDeviceCaps GetSystemInfo GetNativeSystemInfo GetEnvironmentVariableW GetModuleFileNameW GetModuleFileNameA GetTempPathW SHGetFolderPathW
 GetStdHandle GetEnvironmentStringsW GetCommandLineW GetCommandLineA GetStartupInfoW GetStartupInfoA GetKeyboardLayout acmMetrics GetProcessDpiAwareness''')
setcls('R', '''GetCurrentThreadId GetCurrentProcessId GetTickCount QueryPerformanceCounter QueryPerformanceFrequency GetSystemTimeAsFileTime GetLocalTime
 GetTimeZoneInformation GetThreadLocale GetUserDefaultUILanguage GetThreadUILanguage GetComputerNameExW GetLogicalProcessorInformation GetProcessHeap GetSysColor
 EnumDisplayMonitors EnumDisplaySettingsW EnumFontFamiliesExW GetMonitorInfoW GetClipCursor RegQueryValueExW RegQueryInfoKeyW''')
# machine / user / time / screen / process-start dependent values that sit in the M list above: recorded as reference
setcls('R', '''GetModuleHandleW GetModuleHandleA GetFileVersionInfoSizeW GetEnvironmentVariableW GetEnvironmentStringsW GetCommandLineW GetCommandLineA GetStartupInfoW GetStartupInfoA
 GetModuleFileNameW GetModuleFileNameA GetTempPathW SHGetFolderPathW GetKeyboardLayout GetStdHandle GetProcessDpiAwareness GetSystemMetrics SystemParametersInfoW GetDeviceCaps
 GetDC GetSystemDefaultLocaleName''')
# depends on the locale / ANSI code page of the VM (the runtime claims en-US / 1252, which is an environment definition)
setcls('L', 'GetACP GetThreadPreferredUILanguages')
KREASON = {
    'CreateEventW': 'kernel object; S/D in the audit, nothing to measure', 'SetEvent': 'S in the audit', 'WaitForSingleObject': 'could block forever; S in the audit',
    'CloseHandle': 'S in the audit; only valid on handles of the skipped file/event APIs', 'RegisterWindowMessageW': 'adds to the session-wide atom table (side effect)',
    'RegisterClipboardFormatW': 'adds to the session-wide atom table (side effect)', 'GlobalAddAtomW': 'adds to the global atom table (side effect)',
    'CreateDirectoryW': 'writes the file system', 'SetFilePointer': 'needs a file handle; file APIs are not called', 'ReadFile': 'file API, not called',
    'WriteFile': 'writes the file system', 'CreateFileW': 'file API; the traced paths are FL Studio files that do not exist here', 'CreateWindowExW': 'creates a window',
    'DefWindowProcW': 'needs a window', 'SetWindowLongPtrW': 'needs a window', 'RegisterClassW': 'window class registration', 'RegCreateKeyExW': 'writes the registry (RegOpenKeyExW on the same key is measured instead)',
    'RegSetValueExW': 'writes the registry',
}
for k in KREASON:
    CLS[k] = 'K'

# ---------------------------------------------------------------- trace loading / parsing
def load_apis(path):
    d = {}
    for ln in open(path, encoding='utf-8'):
        if ln.startswith('#') or not ln.strip():
            continue
        f = ln.rstrip('\n').split('\t')
        d[f[0]] = (f[1] if len(f) > 1 else '', f[2] if len(f) > 2 else 'q')
    return d

QRE = re.compile(r'"(.*?)"(…)?(?:;|$)', re.S)
def unesc(s):
    return re.sub(r'\\u([0-9a-f]{4})', lambda m: chr(int(m.group(1), 16)), s)

def split_args(s, n):
    """('str', text, truncated) / ('raw', text) tokens; None when the arity does not fit"""
    if n == 0:
        return []
    toks, pos = [], 0
    while pos <= len(s) and len(toks) < n:
        if s.startswith('"', pos):
            m = QRE.match(s, pos)
            if not m:
                return None
            toks.append(('str', unesc(m.group(1)), bool(m.group(2))))
            pos = m.end()
        else:
            j = s.find(';', pos)
            if j < 0:
                j = len(s)
            toks.append(('raw', s[pos:j]))
            pos = j + 1
    return toks if len(toks) == n and pos >= len(s) else None

def iv(t):
    """integer value of a raw token ('NULL' -> 0; 'ptr' -> None)"""
    if t[0] != 'raw':
        return None
    x = t[1]
    if x == 'NULL':
        return 0
    try:
        return int(x, 0)
    except ValueError:
        return None

HANDLE_ARGS = {'SelectObject': (0, 1), 'GetDeviceCaps': (0,), 'ReleaseDC': (1,), 'CreateCompatibleDC': (0,), 'DeleteDC': (0,), 'DeleteObject': (0,), 'SetBkColor': (0,),
               'SetTextColor': (0,), 'SetBkMode': (0,), 'SetROP2': (0,), 'SetStretchBltMode': (0,), 'MoveToEx': (0,), 'GetCurrentPositionEx': (0,), 'PatBlt': (0,),
               'StretchDIBits': (0,), 'GetDIBColorTable': (0,), 'SelectPalette': (0,), 'UnrealizeObject': (0,), 'GetObjectW': (0,), 'CreateDIBSection': (0,),
               'EnumFontFamiliesExW': (0,), 'GetMonitorInfoW': (0,), 'DestroyCursor': (0,), 'RegQueryValueExW': (0,), 'RegCloseKey': (0,),
               'RegQueryInfoKeyW': (0,)}
HANDLE_RET = {'CreateCompatibleDC', 'GetDC', 'SelectObject', 'CreateBitmap', 'CreateBrushIndirect', 'CreateDIBSection', 'CreateFontIndirectW', 'CreatePalette', 'CreatePenIndirect',
              'CreateIconIndirect', 'GetStockObject', 'SelectPalette', 'LoadImageW', 'LoadIconW', 'LoadCursorW', 'CreateEventW'}

def norm_ret(api, rtype, r):
    if rtype == 'v' or r in ('-', ''):
        return r
    try:
        v = int(r, 0)
    except ValueError:
        return r
    if rtype == 'q' and v >= 0x10000 and v < (1 << 64) - 2:
        return 'H' if api in HANDLE_RET else 'PTR'
    return r

def norm_le(s):
    a, _, b = s.partition('>')
    return ('same:' if a == b else 'set:') + b

class Agg:
    def __init__(self):
        self.n = 0; self.seq = None; self.rets = collections.Counter(); self.les = collections.Counter(); self.outs = collections.Counter()

def load_trace(path, apis):
    agg = collections.defaultdict(Agg)
    opener = gzip.open if path.endswith('.gz') else open
    rows = list(csv.reader(opener(path, 'rt', encoding='utf-8', newline=''), delimiter='\t', quoting=csv.QUOTE_NONE))
    hdr, rows = rows[0], rows[1:]
    bad = collections.Counter()
    calls = []
    for r in rows:
        seq, api, args, ret, le, out = r[0], r[6], r[9], r[10], r[11], r[12]
        sig, rt = apis.get(api, ('', 'q'))
        toks = split_args(args, len(sig))
        if toks is None:
            bad[api] += 1
            continue
        calls.append((int(seq), api, toks, args, ret, le, out))
        key = (api, tuple(norm_arg(api, i, t) for i, t in enumerate(toks)))
        a = agg[key]
        a.n += 1
        a.seq = a.seq or int(seq)
        a.rets[norm_ret(api, rt, ret)] += 1
        a.les[norm_le(le)] += 1
        if out:
            a.outs[out] += 1
    return agg, calls, bad

def norm_arg(api, i, t):
    if t[0] == 'str':
        return '"' + t[1] + '"' + ('…' if t[2] else '')
    if i in HANDLE_ARGS.get(api, ()):
        v = iv(t)
        if v is not None and v >= 0x10000:
            return 'H'
    return t[1]

# ---------------------------------------------------------------- token builders
def pct(b):
    out = []
    for c in b:
        if c in b'%|\t\n\r' or c < 0x20 or c > 0x7e:
            out.append('%%%02X' % c)
        else:
            out.append(chr(c))
    return ''.join(out)

def Wt(text):
    if all(0x20 <= ord(c) < 0x7f for c in text):
        return 'W:' + pct(text.encode('ascii'))
    return 'U:' + text.encode('utf-16-le', 'surrogatepass').hex()
def Ut(units):
    return 'U:' + b''.join(int(u).to_bytes(2, 'little') for u in units).hex()
def At(bs):
    if isinstance(bs, str):
        bs = bs.encode('latin-1')
    if all(0x20 <= c < 0x7f for c in bs):
        return 'A:' + pct(bs)
    return 'X:' + bs.hex()
def d(v):
    return 'd:%s' % (v if isinstance(v, str) else hex(v))
def i64(v):
    return 'i:%s' % (v if isinstance(v, str) else hex(v))
def Z(n, patches=None, name=None):
    s = 'Z:%d' % n
    if patches:
        s += ':' + ','.join('%d=%s' % (o, h) for o, h in patches)
    if name:
        s += (':' if patches else '::') + name
    return s
def O(n, name=None):
    return 'O:%d' % n + (':' + name if name else '')
def D(v=0, name=None):
    return 'D:%s' % hex(v) + (':' + name if name else '')
def le32(*vs):
    return ''.join((v & 0xffffffff).to_bytes(4, 'little').hex() for v in vs)
def u16h(v):
    return v.to_bytes(2, 'little').hex()
def wstr_hex(s):
    return s.encode('utf-16-le').hex()

def hx(v):
    return str(v) if v < 10 else hex(v)

def tok_of(t, kind):
    """a trace string token -> replay token"""
    if t[0] == 'raw':
        if t[1] == 'NULL':
            return 'NULL'
        v = iv(t)
        if v is not None:
            return i64(v)
        return None
    return Wt(t[1]) if kind == 'W' else At(t[1])

# ---------------------------------------------------------------- the generator
class Gen:
    def __init__(self, agg, calls, apis, gpa):
        self.agg, self.calls, self.apis, self.gpa = agg, calls, apis, gpa
        self.rows = []; self.stats = collections.Counter(); self.sec = None
        self.used_trace_keys = set()
        self.by_seq = {c[0]: c[2] for c in calls}
    def section(self, name):
        self.rows.append(('#', name))
    def add(self, api, args, capture='', dll='', tkey=None, note='', kind=None, agg=None):
        rt = self.apis.get(api, ('', 'q'))[1] if not api.startswith('@') else 'q'
        a = agg or (self.agg.get((api, tuple(tkey))) if tkey is not None else None)
        k = kind or (('scn' if tkey is not None and 'H' in tkey else 'trace') if a else 'probe')
        if a:
            self.used_trace_keys.add((api, tuple(tkey)))
        self.rows.append(('row', dict(kind=k, api=api, dll=dll, args='|'.join(args), capture=capture, rtype=rt, cls=CLS.get(api, 'H'),
                                      n=a.n if a else 0, seq=a.seq if a else '',
                                      ret=json.dumps(sorted(a.rets.items(), key=lambda x: -x[1])) if a else '',
                                      le=json.dumps(sorted(a.les.items(), key=lambda x: -x[1])) if a else '',
                                      out=json.dumps(sorted(a.outs.items(), key=lambda x: -x[1])[:3], ensure_ascii=False) if a and a.outs else '',
                                      note=note)))
    def shapes(self, api):
        return sorted(((k[1], a) for k, a in self.agg.items() if k[0] == api), key=lambda x: x[1].seq)

def build(gen):
    A = gen.add
    ag = gen.agg

    # =============== 1. text and locale ===============
    gen.section('GetLocaleInfoW: trace shapes (LCID x LCType x cch), then LCID/LCType sweeps')
    ltypes = []; lshapes = set()
    for key, a in gen.shapes('GetLocaleInfoW'):
        lcid, lct, cch = int(key[0], 0), int(key[1], 0), int(key[3], 0)
        ltypes.append(lct); lshapes.add((lcid, lct, cch))
        A('GetLocaleInfoW', [d(lcid), d(lct), O(cch * 2), d(cch)], tkey=list(key))
    ltypes = sorted(set(ltypes))
    for lcid in (0x409, 0x400, 0x800):
        for lct in ltypes:
            if (lcid, lct, 0x100) in lshapes:
                continue
            A('GetLocaleInfoW', [d(lcid), d(lct), O(0x200), d(0x100)], note='probe: LCID/LCType of a traced type')
    for lct in list(range(0, 0x100)) + list(range(0x1000, 0x1040)):
        if lct in ltypes:
            continue
        A('GetLocaleInfoW', [d(0x409), d(lct), O(0x200), d(0x100)], note='probe: LCType sweep at en-US')
    for lct in ltypes:
        A('GetLocaleInfoW', [d(0x409), d(lct), 'NULL', d(0)], note='probe: size query (cchData=0)')
    for lct in (0x59, 0x5c, 0x1f, 0x20):
        A('GetLocaleInfoW', [d(0x409), d(lct), O(4), d(1)], note='probe: buffer too small (cchData=1)')
    for lct in (0xd, 0x1009, 0x100b, 0x10, 0x11, 0x1a, 0x1c, 0x21, 0x22):
        A('GetLocaleInfoW', [d(0x409), d(lct | 0x20000000), O(16), d(2)], note='probe: LOCALE_RETURN_NUMBER')

    gen.section('IsValidLocale / code pages / default locale values')
    for key, a in gen.shapes('IsValidLocale'):
        A('IsValidLocale', [d(int(key[0], 0)), d(int(key[1], 0))], tkey=list(key))
    for lcid in (0, 0x400, 0x800, 0x409, 0x809, 0xc09, 0x407, 0x411, 0x412, 0x804, 0x1009, 0x416, 0x40c, 0x410, 0x419, 0x1000, 0x2000, 0x3000, 0x7f, 0xffff, 0x10000, 0x1234, 0x4009, 0x41f):
        for fl in (1, 2):
            A('IsValidLocale', [d(lcid), d(fl)], note='probe: LCID list')
    for api in ('GetACP', 'GetOEMCP', 'GetThreadLocale', 'GetUserDefaultUILanguage', 'GetThreadUILanguage', 'GetUserDefaultLCID', 'GetSystemDefaultLCID'):
        A(api, [], tkey=[])
    cpshapes = set()
    for key, a in gen.shapes('GetCPInfo'):
        cpshapes.add(int(key[0], 0))
        A('GetCPInfo', [d(int(key[0], 0)), O(20)], tkey=list(key))
    cps = [0, 1, 2, 3, 437, 850, 1250, 1251, 1252, 932, 936, 949, 950, 20127, 28591, 65000, 65001, 1200, 12000, 99999]
    for cp in cps:
        if cp not in cpshapes:
            A('GetCPInfo', [d(cp), O(20)], note='probe: code page list')
        A('IsValidCodePage', [d(cp)], tkey=[hx(cp)], note='' if cp == 1252 else 'probe: code page list')
    A('GetSystemDefaultLocaleName', [O(0x55 * 2), d(0x55)], tkey=['ptr', '0x55'])
    A('GetSystemDefaultLocaleName', [O(8), d(3)], note='probe: buffer too small')
    A('GetUserDefaultLocaleName', [O(0x55 * 2), d(0x55)], note='probe: helper')
    # preferred UI languages: size query, then the list
    A('GetThreadPreferredUILanguages', [d(0x38), D(0, 'pl_cnt'), 'NULL', D(0, 'pl_len')], tkey=['0x38', 'ptr', 'NULL', 'ptr'])
    A('GetThreadPreferredUILanguages', [d(0x38), '$pl_cnt', O(512), '$pl_len'], tkey=['0x38', 'ptr', 'ptr', 'ptr'])
    A('SetLastError', [d(0)], note='probe: clear')
    # SetThreadLocale (thread-local; restored)
    gen.section('SetThreadLocale / SetErrorMode / SetThreadErrorMode (restored)')
    A('GetThreadLocale', [], capture='tl0', note='scenario: remember')
    A('SetThreadLocale', [d(0x400)], tkey=['0x400'])
    A('GetThreadLocale', [], note='scenario: after set')
    A('SetThreadLocale', ['$tl0'], note='scenario: restore')
    A('GetThreadLocale', [], note='scenario: after restore')
    A('GetErrorMode', [], capture='em0', note='probe')
    A('SetErrorMode', [d(0x8000)], tkey=['0x8000'])
    A('SetErrorMode', [d(0)], tkey=['0'])
    A('SetErrorMode', ['$em0'], note='scenario: restore')
    A('SetThreadErrorMode', [d(1), O(4)], tkey=['1', 'ptr'])
    A('SetThreadErrorMode', [d(0), O(4)], tkey=['0', 'ptr'])
    A('SetHandleCount', [d(0x20)], tkey=['0x20'])
    A('SetLastError', [d(2)], tkey=['2'])
    A('SetLastError', [d(0x7f)], tkey=['0x7f'])
    A('GetLastError', [], tkey=[], note='the harness sets a sentinel last error just before each call; GetLastError must return it')

    # ---- conversions
    gen.section('MultiByteToWideChar / WideCharToMultiByte: traced shapes (de-duplicated) and code page tables')
    def conv_rows(api):
        seen = collections.Counter(); out = []
        for key, a in gen.shapes(api):
            toks = gen.by_seq.get(a.seq)
            if toks is None:
                continue
            if toks[2][0] != 'str' or toks[2][2]:
                continue
            cp, fl = iv(toks[0]), iv(toks[1])
            s = toks[2][1]
            ascii_ = all(0x20 <= ord(c) < 0x7f for c in s)
            cls = (cp, fl, ascii_, toks[4] == ('raw', 'NULL'), iv(toks[3]) == 0xffffffff, len(s) < 8)
            seen[cls] += 1
            if ascii_ and seen[cls] > 3:
                continue
            out.append((key, a, toks))
        return out
    for key, a, toks in conv_rows('MultiByteToWideChar'):
        cp, fl, s = iv(toks[0]), iv(toks[1]), toks[2][1]
        cb, cch = iv(toks[3]), iv(toks[5])
        A('MultiByteToWideChar', [d(cp), d(fl), At(s), d(cb), 'NULL' if toks[4] == ('raw', 'NULL') else O(max(cch, 1) * 2), d(cch)], tkey=list(key))
    for key, a, toks in conv_rows('WideCharToMultiByte'):
        cp, fl, s = iv(toks[0]), iv(toks[1]), toks[2][1]
        cch, cb = iv(toks[3]), iv(toks[5])
        A('WideCharToMultiByte', [d(cp), d(fl), Wt(s), d(cch), 'NULL' if toks[4] == ('raw', 'NULL') else O(max(cb, 1)), d(cb),
                                  'NULL', 'NULL' if toks[7] == ('raw', 'NULL') else O(4)], tkey=list(key))
    tab = bytes(range(256))
    for cp in (1252, 437, 850, 28591, 0, 1, 3, 1250, 65001):
        for fl in ((0, 1) if cp == 1252 else (0,)):
            A('MultiByteToWideChar', [d(cp), d(fl), 'X:' + tab.hex(), d(256), O(512 + 16), d(256 + 8)], note='probe: all 256 byte values, one call (SBCS tables; invalid UTF-8 for 65001)')
    for fl in (0, 8):
        for name, bs in (('C3A9', 'c3a9'), ('E282AC', 'e282ac'), ('F09F9880', 'f09f9880'), ('80', '80'), ('C080', 'c080'), ('E282', 'e282'), ('EDA080', 'eda080'), ('F4908080', 'f4908080'), ('41FF42', '41ff42')):
            b = bytes.fromhex(bs)
            A('MultiByteToWideChar', [d(65001), d(fl), 'X:' + bs, d(len(b)), O(32), d(8)], note='probe: UTF-8 sequence %s flags=%d' % (name, fl))
    A('MultiByteToWideChar', [d(1252), d(0), 'A:abc', d(-1), 'NULL', d(0)], note='probe: size query, cb=-1')
    A('MultiByteToWideChar', [d(1252), d(0), 'A:abc', d(-1), O(8), d(2)], note='probe: output too small')
    A('MultiByteToWideChar', [d(1252), d(0), 'A:abc', d(3), O(8), d(0)], note='probe: size query')
    A('MultiByteToWideChar', [d(99999), d(0), 'A:abc', d(3), O(8), d(4)], note='probe: invalid code page')
    wtab = list(range(256))
    sp1252 = [0x20ac, 0x201a, 0x192, 0x201e, 0x2026, 0x2020, 0x2021, 0x2c6, 0x2030, 0x160, 0x2039, 0x152, 0x17d, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014, 0x2dc, 0x2122, 0x161, 0x203a, 0x153, 0x17e, 0x178]
    for cp in (1252, 437, 850, 28591, 0, 1, 3, 65001):
        for fl in (0, 0x400):
            if cp == 65001 and fl:
                continue
            A('WideCharToMultiByte', [d(cp), d(fl), Ut(wtab), d(256), O(1024), d(1024), 'NULL', O(4)], note='probe: U+0000..U+00FF, cp=%d flags=0x%x' % (cp, fl))
        A('WideCharToMultiByte', [d(cp), d(0), Ut(sp1252), d(len(sp1252)), O(256), d(256), 'A:?', O(4)], note='probe: the 27 non-Latin-1 code points of Windows-1252, cp=%d' % cp)
        if cp != 65001:
            A('WideCharToMultiByte', [d(cp), d(0x400), Ut(sp1252), d(len(sp1252)), O(256), d(256), 'A:?', O(4)], note='probe: same, WC_NO_BEST_FIT_CHARS, cp=%d' % cp)
    for lo, hi, nm in ((0x100, 0x180, 'Latin Extended-A'), (0x370, 0x400, 'Greek'), (0x2000, 0x2100, 'General Punctuation'), (0xff00, 0xff80, 'Fullwidth')):
        for cp in (1252, 437):
            for fl in (0, 0x400):
                A('WideCharToMultiByte', [d(cp), d(fl), Ut(range(lo, hi)), d(hi - lo), O(512), d(512), 'NULL', O(4)], note='probe: %s best-fit, cp=%d flags=0x%x' % (nm, cp, fl))
    for name, units in (('surrogate pair', [0xd83d, 0xde00]), ('lone high surrogate', [0xd800, 0x41]), ('euro', [0x20ac]), ('e-acute', [0xe9])):
        for fl in (0, 0x80):
            A('WideCharToMultiByte', [d(65001), d(fl), Ut(units), d(len(units)), O(16), d(16), 'NULL', 'NULL'], note='probe: %s to UTF-8 flags=0x%x' % (name, fl))
    A('WideCharToMultiByte', [d(1252), d(0), 'W:abc', d(-1), 'NULL', d(0), 'NULL', 'NULL'], note='probe: size query, cch=-1')
    A('WideCharToMultiByte', [d(1252), d(0), 'W:abc', d(-1), O(2), d(2), 'NULL', 'NULL'], note='probe: output too small')
    A('WideCharToMultiByte', [d(65001), d(0), 'W:abc', d(-1), 'NULL', d(0), 'A:?', 'NULL'], note='probe: lpDefaultChar with UTF-8 (invalid parameter)')

    gen.section('CompareStringW / LCMapStringW / GetStringTypeW')
    for key, a in gen.shapes('CompareStringW'):
        toks = gen.by_seq[a.seq]
        A('CompareStringW', [d(iv(toks[0])), d(iv(toks[1])), Wt(toks[2][1]), d(iv(toks[3])), Wt(toks[4][1]), d(iv(toks[5]))], tkey=list(key))
    names = ['_x64.dll', 'ppv2.dll', 'ache.dll', 'type.dll']
    for x in names:
        for y in names:
            if x != y:
                for fl in (0, 1):
                    A('CompareStringW', [d(0x409), d(fl), Wt(x), d(8), Wt(y), d(8)], note='probe: word sort of the module-name tails')
    for x, y in (('default', 'Default'), ('Default', 'Segoe UI'), ('Default', 'Calibri'), ('Segoe UI', 'Segoe UI'), ('_', 'a'), ('a', '_'), ('-', 'a'), ("'", 'a'), ('a-b', 'ab'), ('co-op', 'coop'), ('a', 'A'), ('a', 'b')):
        for fl in (0, 1, 0x10000000):
            A('CompareStringW', [d(0x409), d(fl), Wt(x), d(-1), Wt(y), d(-1)], note='probe: punctuation / case ordering, flags=0x%x' % fl)
    for key, a in gen.shapes('LCMapStringW'):
        toks = gen.by_seq[a.seq]
        s = toks[2][1]; cch = iv(toks[3]); dest = iv(toks[5])
        A('LCMapStringW', [d(iv(toks[0])), d(iv(toks[1])), Wt(s), d(cch), 'NULL' if toks[4] == ('raw', 'NULL') else O(max(dest, 1) * 2), d(dest)], tkey=list(key))
    for fl in (0x100, 0x200, 0x1000100, 0x1000200):
        for lcid in (0, 0x409, 0x41f):
            A('LCMapStringW', [d(lcid), d(fl), Ut(wtab), d(256), O(520), d(256)], note='probe: Latin-1 table, lcid=0x%x flags=0x%x' % (lcid, fl))
        A('LCMapStringW', [d(0), d(fl), Ut(sp1252), d(len(sp1252)), O(80), d(len(sp1252))], note='probe: the 27 code points, flags=0x%x' % fl)
    A('LCMapStringW', [d(0x409), d(0x100), Ut([0x391, 0x3a3, 0x3c2, 0x130, 0x49, 0xdf]), d(6), O(32), d(6)], note='probe: Greek / dotted I / sharp s lower')
    A('LCMapStringW', [d(0x409), d(0x200), Ut([0x3b1, 0x3c2, 0x131, 0x69, 0xdf]), d(5), O(32), d(5)], note='probe: Greek / dotless i / sharp s upper')
    for key, a in gen.shapes('GetStringTypeW'):
        toks = gen.by_seq[a.seq]
        s = toks[1][1]
        A('GetStringTypeW', [d(iv(toks[0])), Wt(s) if all(0x20 <= ord(c) < 0x7f for c in s) else Ut([ord(c) for c in s]), d(iv(toks[2])), O(max(iv(toks[2]), 1) * 2 + 2)], tkey=list(key))
    for ct in (1, 2, 4):
        A('GetStringTypeW', [d(ct), Ut(wtab), d(256), O(520)], note='probe: Latin-1 table, ctype=%d' % ct)
        A('GetStringTypeW', [d(ct), Ut(sp1252), d(len(sp1252)), O(80)], note='probe: the 27 code points, ctype=%d' % ct)

    gen.section('Char* functions')
    for n in (3, 4, 5, 9, 0xa, 0xb):
        s = 'AbCdEfGhIjKlMnOpQ'[:n]
        A('CharLowerBuffW', [Wt(s), d(n)], tkey=['ptr', hex(n) if n > 9 else str(n)], note='contents are not in the trace; probe string of the traced length')
        A('CharUpperBuffW', [Wt(s), d(n)], tkey=['ptr', hex(n) if n > 9 else str(n)], note='contents are not in the trace; probe string of the traced length')
    A('CharLowerBuffW', [Ut(wtab), d(256)], note='probe: Latin-1 table')
    A('CharUpperBuffW', [Ut(wtab), d(256)], note='probe: Latin-1 table')
    A('CharLowerBuffW', [Ut(sp1252), d(len(sp1252))], note='probe: the 27 code points')
    A('CharUpperBuffW', [Ut(sp1252), d(len(sp1252))], note='probe: the 27 code points')
    A('CharLowerBuffW', [Ut([0x391, 0x3a3, 0x130, 0x49]), d(4)], note='probe: Greek / dotted I')
    A('CharLowerBuffW', [Wt('ABC'), d(0)], note='probe: cch=0')
    A('CharLowerW', [Wt('ABC')], tkey=['ptr'], note='probe: pointer form')
    A('CharLowerW', [i64(0x41)], note='probe: single character form (value below 0x10000)')
    A('CharLowerW', [i64(0xc9)], note='probe: single character form (E-acute)')
    A('CharNextW', [Wt('FLEngine_x64')], tkey=['"FLEngine_x64"'])
    A('CharNextW', [Wt('')], note='probe: empty string')
    A('CharNextW', [Ut([0xd83d, 0xde00, 0x41])], note='probe: surrogate pair')
    A('CharNextW', [Ut([0x41, 0x301, 0x42])], note='probe: combining mark')
    for a_, b_, c_ in ((8, 0x60, 0x48), (1, 1, 2), (-1, 1, 2), (1, -1, 2), (3, 1, 2), (5, 1, 2), (7, 1, 3), (100, 100, 0), (0x7fffffff, 2, 1), (-0x80000000, 1, 1), (8, 96, -72), (0, 5, 0), (10, 3, 4)):
        A('MulDiv', [d(a_ & 0xffffffff), d(b_ & 0xffffffff), d(c_ & 0xffffffff)], tkey=['8', '0x60', '0x48'] if (a_, b_, c_) == (8, 0x60, 0x48) else None, note='' if (a_, b_, c_) == (8, 0x60, 0x48) else 'probe: rounding / error cases')

    # =============== 2. versions, system, process ===============
    gen.section('Versions')
    A('GetVersion', [], tkey=[])
    A('GetVersionExW', [Z(284, [(0, le32(284))])], tkey=['ptr'], note='size 284 = OSVERSIONINFOEXW (the trace shows size=284); struct contents are written by the call')
    A('GetVersionExA', [Z(156, [(0, le32(148))])], tkey=['ptr'], note='size 148 = OSVERSIONINFOA (trace shows size=148); buffer is the EX size so the tool can show whether more is written')
    A('GetVersionExW', [Z(276, [(0, le32(276))])], note='probe: plain OSVERSIONINFOW')
    A('RtlGetVersion', [Z(284, [(0, le32(284))])], note='probe: helper, unaffected by manifest')
    # VerSetConditionMask chains seen in the trace
    A('VerSetConditionMask', [i64(0), d(2), d(3)], capture='m1', tkey=['0', '2', '0x1010103'], note='trace shape had a stale upper part in the 3rd argument; the call takes a BYTE')
    A('VerSetConditionMask', ['$m1', d(1), d(3)], capture='m2', tkey=['0x18', '1', '0x1010103'])
    A('VerSetConditionMask', ['$m2', d(0x20), d(3)], capture='m3', tkey=['0x1b', '0x20', '0x1010103'])
    A('VerSetConditionMask', [i64(0), d(0x80), d(1)], capture='m4', tkey=['0', '0x80', '0x1010101'])
    for t in (1, 2, 4, 8, 0x10, 0x20, 0x40, 0x80):
        for c in (1, 2, 3, 4, 5):
            A('VerSetConditionMask', [i64(0), d(t), d(c)], note='probe: single condition type=0x%x cond=%d' % (t, c))
    A('VerSetConditionMask', [i64(0), d(2), d(0)], note='probe: invalid condition 0')
    A('VerSetConditionMask', [i64(0), d(3), d(1)], note='probe: type with two bits')
    # VerifyVersionInfoW: struct is OSVERSIONINFOEXW (284 bytes)
    def osvi(maj, mnr, build=0, spmaj=0, spmin=0, suite=0, prod=0, size=284):
        p = [(0, le32(size, maj, mnr, build))]
        p.append((276, u16h(spmaj) + u16h(spmin) + u16h(suite) + '%02x' % prod))
        return Z(284, p)
    A('VerifyVersionInfoW', [osvi(6, 0), d(0x23), '$m3'], tkey=['ptr', '0x23', '0x1801b'], note='trace shape: MAJOR|MINOR|SPMAJOR with >=,>=,>= masks; the struct values were not recorded, probes below vary them')
    A('VerifyVersionInfoW', [osvi(0, 0, size=0, prod=1), d(0x80), '$m4'], tkey=['ptr', '0x80', '0x200000'], note='trace shape: size=0, wProductType=1, VER_PRODUCT_TYPE equal (decision 0040 retraction condition)')
    for (mj, mn, sp) in ((5, 1, 0), (6, 0, 0), (6, 1, 0), (6, 2, 0), (6, 3, 0), (10, 0, 0), (10, 0, 1), (10, 1, 0), (11, 0, 0), (4, 0, 0)):
        A('VerSetConditionMask', [i64(0), d(2), d(3)], capture='vm1')
        A('VerSetConditionMask', ['$vm1', d(1), d(3)], capture='vm2')
        A('VerSetConditionMask', ['$vm2', d(0x20), d(3)], capture='vm3')
        A('VerifyVersionInfoW', [osvi(mj, mn, spmaj=sp), d(0x23), '$vm3'], note='probe: is the OS >= %d.%d sp%d ?' % (mj, mn, sp))
    A('VerSetConditionMask', [i64(0), d(0x80), d(1)], capture='vp1')
    A('VerifyVersionInfoW', [osvi(0, 0, prod=1), d(0x80), '$vp1'], note='probe: VER_NT_WORKSTATION, size set correctly')
    A('VerifyVersionInfoW', [osvi(0, 0, prod=3), d(0x80), '$vp1'], note='probe: VER_NT_SERVER')

    gen.section('System info / process info (machine dependent: reference)')
    for key, a in gen.shapes('GetSystemInfo'):
        A('GetSystemInfo', [O(48)], tkey=list(key))
    A('GetNativeSystemInfo', [O(48)], tkey=['ptr'])
    A('GetLogicalProcessorInformation', ['NULL', D(0, 'lpi_len')], tkey=['NULL', 'ptr'], note='size query; expect FALSE + ERROR_INSUFFICIENT_BUFFER')
    A('GetLogicalProcessorInformation', [O(8192), '$lpi_len'], tkey=['ptr', 'ptr'], note='second call reuses the length written by the first')
    for form in range(0, 9):
        A('GetComputerNameExW', [d(form), O(1024), D(512)], tkey=['0', 'ptr', 'ptr'] if form == 0 else None, note='value is masked (length / character classes only)')
    for form in (0, 1, 3):
        A('GetComputerNameExW', [d(form), O(8), D(1)], note='probe: buffer too small (masked)')
    for nm in ('LANG', 'SystemRoot', 'windir', 'OS', 'PROCESSOR_ARCHITECTURE', 'NUMBER_OF_PROCESSORS', 'PATHEXT', 'ComSpec', 'REPLAY_NO_SUCH_VAR'):
        A('GetEnvironmentVariableW', [Wt(nm), O(1024), d(512)], tkey=['"LANG"', 'ptr', '1'] if nm == 'LANG' else None, note='allow-listed names only')
    A('GetEnvironmentVariableW', [Wt('LANG'), O(4), d(1)], tkey=['"LANG"', 'ptr', '1'])
    A('GetEnvironmentVariableW', [Wt('OS'), O(4), d(1)], note='probe: too small, expect required size')
    A('GetEnvironmentStringsW', [], capture='envp', tkey=[])
    A('FreeEnvironmentStringsW', ['$envp'], tkey=['ptr'])
    A('GetCommandLineW', [], tkey=[])
    A('GetCommandLineA', [], tkey=[])
    A('GetStartupInfoW', [Z(104, [(0, le32(0))])], tkey=['ptr'])
    A('GetStartupInfoA', [Z(104, [(0, le32(0))])], tkey=['ptr'])
    seen_sizes = set()
    for key, a in gen.shapes('GetModuleFileNameW'):
        mh, n = key[0], int(key[2], 0)
        if mh not in ('0', '0x400000') or n in seen_sizes:
            continue
        seen_sizes.add(n)
        A('GetModuleFileNameW', ['NULL', O(n * 2 + 2), d(n)], tkey=list(key))
    for n in (0x104, 1, 2, 5, 20, 100):
        A('GetModuleFileNameW', ['NULL', O(n * 2 + 2), d(n)], note='probe: truncation (hModule=NULL)')
    A('GetModuleFileNameW', ['M:self', O(528), d(0x104)], tkey=None, note='probe: hModule=own exe')
    for mod in ('kernel32', 'user32', 'avrt'):
        A('GetModuleFileNameW', ['M:' + mod, O(528), d(0x104)], note='probe: system module path form')
    A('GetModuleFileNameA', ['NULL', O(0x105), d(0x104)], tkey=['0', 'ptr', '0x104'])
    A('GetModuleFileNameA', ['NULL', O(8), d(5)], note='probe: truncation')
    A('GetTempPathW', [d(0x104), O(528)], tkey=['0x104', 'ptr'])
    A('GetTempPathW', [d(5), O(12)], note='probe: too small')
    A('GetTimeZoneInformation', [O(172)], tkey=['ptr'])
    A('GetLocalTime', [O(16)], tkey=['ptr'])
    A('GetSystemTimeAsFileTime', [O(8)], tkey=['ptr'])
    A('GetTickCount', [], tkey=[])
    A('QueryPerformanceCounter', [O(8)], tkey=['ptr'])
    A('QueryPerformanceFrequency', [O(8)], tkey=['ptr'])
    A('GetCurrentProcess', [], tkey=[])
    A('GetCurrentThread', [], tkey=[])
    A('GetCurrentProcessId', [], tkey=[])
    A('GetCurrentThreadId', [], tkey=[])
    for h in (0xfffffff6, 0xfffffff5, 0xfffffff4):
        A('GetStdHandle', [d(h)], tkey=[hex(h)])
    A('GetStdHandle', [d(0)], note='probe: invalid handle id')

    # =============== 3. modules ===============
    gen.section('GetModuleHandle* / LoadLibrary* / FreeLibrary / SetDllDirectoryW')
    sysok = lambda n: re.sub(r'(?i)\.dll$', '', n).lower() in ('kernel32', 'user32', 'advapi32', 'shell32', 'shfolder', 'shcore', 'ole32', 'oleaut32', 'comctl32', 'avrt', 'powrprof', 'dwmapi', 'imm32', 'msctf') or n.lower().startswith('api-ms-win-')
    for api, kind in (('GetModuleHandleW', 'W'), ('GetModuleHandleA', 'A')):
        for key, a in gen.shapes(api):
            nm = key[0].strip('"')
            if sysok(nm):
                A(api, [Wt(nm) if kind == 'W' else At(nm)], tkey=list(key))
        A(api, ['NULL'], note='probe: NULL = the exe')
        A(api, [Wt('NOSUCHMODULE_REPLAY') if kind == 'W' else At('NOSUCHMODULE_REPLAY')], note='probe: not a module (must be refused by the allow-list)')
    for nm in ('avrt', 'powrprof', 'ntdll', 'gdi32', 'version', 'msacm32', 'shcore.dll', 'KERNEL32.DLL', 'Kernel32'):
        A('GetModuleHandleW', [Wt(nm)], note='probe: name resolution rules (extension / case / loaded or not)')
    for api, kind in (('LoadLibraryW', 'W'), ('LoadLibraryA', 'A')):
        for key, a in gen.shapes(api):
            nm = key[0].strip('"')
            if sysok(nm):
                A(api, [Wt(nm) if kind == 'W' else At(nm)], tkey=list(key))
    for key, a in gen.shapes('LoadLibraryExW'):
        nm = key[0].strip('"')
        if sysok(nm):
            A('LoadLibraryExW', [Wt(nm), 'NULL', d(int(key[2], 0))], tkey=list(key))
    for nm in ('avrt', 'powrprof', 'dwmapi', 'imm32', 'msctf', 'version', 'shcore', 'api-ms-win-core-synch-l1-2-0', 'api-ms-win-core-fibers-l1-1-1',
               'api-ms-win-core-localization-l1-2-1', 'api-ms-win-core-synch-l1-2-1', 'api-ms-win-core-sysinfo-l1-2-0', 'api-ms-win-core-nonexistent-l1-1-0', 'shfolder'):
        A('LoadLibraryW', [Wt(nm)], capture='lib_' + re.sub(r'\W', '_', nm), note='probe: does this system module exist')
    for nm in ('avrt', 'shcore'):
        A('FreeLibrary', ['$lib_' + nm], note='scenario: free what this tool loaded')
    A('FreeLibrary', ['$lib_avrt'], note='scenario: second free of the same handle is refused by the policy (module not owned any more)')
    A('SetDllDirectoryW', [Wt('Shared\\')], tkey=['"Shared\\"'])
    A('SetDllDirectoryW', ['NULL'], note='scenario: restore')

    gen.section('GetProcAddress: (module, function) pairs the guest asked for (system modules only)')
    for (mod, fn, res, calls_, lastseq) in gen.gpa:
        if not sysok(mod):
            continue
        a = gen.agg_gpa.get((mod, fn))
        arg = i64(int(fn[1:])) if fn.startswith('#') else At(fn)
        A('GetProcAddress', ['M:' + mod, arg], note='trace result: %s (%s calls); module name as the guest passed it' % (res, calls_), kind='trace')
        gen.rows[-1][1].update(n=int(calls_), ret=json.dumps([['NULL' if res == 'NULL' else 'H', int(calls_)]]), le='[]', out='', seq=lastseq)

    # =============== 4. memory, TLS, sync ===============
    gen.section('Heap / LocalAlloc / VirtualAlloc / VirtualProtect (process-private; blocks are freed)')
    A('GetProcessHeap', [], capture='heap0', tkey=[])
    hshapes = sorted(gen.shapes('HeapAlloc'), key=lambda x: -x[1].n)
    blocks = []
    for key, a in hshapes:
        fl, sz = int(key[1], 0), int(key[2], 0)
        if (fl, sz) in [(b[0], b[1]) for b in blocks] or len(blocks) >= 8:
            continue
        nm = 'hp%d' % len(blocks)
        blocks.append((fl, sz, nm))
        A('HeapAlloc', ['$heap0', d(fl), i64(sz)], capture=nm, tkey=[key[0], key[1], key[2]], note='trace shape HeapAlloc(process heap, flags=%d, size=0x%x)' % (fl, sz))
    for fl, sz in ((0, 0), (8, 1), (8, 0x100)):
        nm = 'hp%d' % len(blocks)
        blocks.append((fl, sz, nm))
        A('HeapAlloc', ['$heap0', d(fl), i64(sz)], capture=nm, note='probe: HeapAlloc(flags=%d, size=0x%x)' % (fl, sz))
    for fl, sz, nm in blocks:
        A('HeapSize', ['$heap0', d(0), '$' + nm], note='size of block %s (requested 0x%x)' % (nm, sz))
        if fl == 8:
            A('@peek', ['$' + nm, d(min(sz, 32))], note='HEAP_ZERO_MEMORY block %s must be zero' % nm)
    b0 = blocks[0][2]; bz = [b[2] for b in blocks if b[1] == 0][0]
    A('HeapReAlloc', ['$heap0', d(0), '$' + b0, i64(0x10)], capture='hp_re1', note='probe: shrink')
    A('HeapReAlloc', ['$heap0', d(0x10), '$hp_re1', i64(0x4000)], capture='hp_re3', note='probe: HEAP_REALLOC_IN_PLACE_ONLY grow')
    A('HeapReAlloc', ['$heap0', d(0), '$' + bz, i64(0x20)], capture='hp_re4', note='probe: grow from zero length')
    A('HeapReAlloc', ['$heap0', d(8), '$hp_re4', i64(0x2000)], capture='hp_re5', note='probe: grow with HEAP_ZERO_MEMORY')
    A('@peek', ['$hp_re5', d(32)], note='the tail added by HeapReAlloc with HEAP_ZERO_MEMORY must be zero')
    A('HeapFree', ['$heap0', d(0), 'NULL'], note='probe: HeapFree(NULL) (decision-0102 gap: NULL case)')
    A('HeapFree', ['$heap0', d(0), '$hp_re5'], note='probe')
    A('HeapCreate', [d(0), i64(0x1000), i64(0)], capture='heap1', tkey=['0', '0x1000', '0'])
    A('HeapSetInformation', ['$heap1', d(0), D(2), i64(4)], note='scenario: HeapCompatibilityInformation = 2 (LFH); the trace shape')
    A('HeapAlloc', ['$heap1', d(0), i64(0x100)], capture='hp1a', note='scenario')
    A('HeapSize', ['$heap1', d(0), '$hp1a'], note='scenario: expect 0x100')
    A('HeapFree', ['$heap1', d(0), '$hp1a'], note='scenario')
    for fl, sz in ((0x40, 0x32a8), (0x40, 0x10), (0x40, 0x240), (0, 0x100)):
        A('LocalAlloc', [d(fl), i64(sz)], capture='lp_%x_%x' % (fl, sz), note='scenario: LPTR=0x40 is LMEM_FIXED|LMEM_ZEROINIT')
    A('@peek', ['$lp_40_240', d(32)], note='scenario: LPTR block is zero')
    A('LocalFree', ['$lp_40_240'], note='helper')
    seenv = set()
    for key, a in gen.shapes('VirtualAlloc'):
        if key[1:] in seenv:
            continue
        seenv.add(key[1:])
        prot = int(key[3], 0)
        if prot != 4:
            continue
        A('VirtualAlloc', [i64(0), i64(int(key[1], 0)), d(int(key[2], 0)), d(prot)], tkey=list(key), note='the block is released at once; only the alignment and the zero-fill are recorded')
    A('@selfcode', [], capture='code', note='address of a code page of this exe')
    A('VirtualProtect', ['$code', i64(5), d(0x40), O(4)], tkey=None, note='trace shape VirtualProtect(addr, 5, PAGE_EXECUTE_READWRITE, &old) on a .text page of this exe; restored at once')
    gen.section('TLS / FLS / critical section / SRW / SList / pointer encoding')
    A('TlsAlloc', [], capture='tls1', tkey=[])
    A('TlsGetValue', ['$tls1'], note='scenario: initial value is 0; does the call set last error to 0?')
    A('TlsSetValue', ['$tls1', Z(16, name='tlsval')], note='scenario')
    A('TlsGetValue', ['$tls1'], tkey=['0'], note='scenario: returns the stored pointer; success clears last error? (trace shape: slot 0 returned a pointer, last error set to 0)')
    A('TlsGetValue', [d(0x7fffff)], note='probe: invalid index')
    A('TlsSetValue', [d(0x7fffff), 'NULL'], note='probe: invalid index')
    A('TlsFree', ['$tls1'], note='helper')
    A('FlsAlloc', ['F:none'], capture='fls1', tkey=['ptr'])
    A('FlsGetValue', ['$fls1'], note='scenario: initial value 0; last error after success?')
    A('FlsSetValue', ['$fls1', '$tlsval'], note='scenario')
    A('FlsGetValue', ['$fls1'], tkey=['1'], note='scenario: stored pointer; last error after success? (trace shape: slot 1 returned a pointer 138 times)')
    A('FlsGetValue', [d(0x7fffff)], note='probe: invalid index')
    A('FlsFree', ['$fls1'], note='helper')
    A('InitializeCriticalSectionAndSpinCount', [Z(40, name='cs1'), d(0xfa0)], tkey=['ptr', '0xfa0'])
    A('InitializeCriticalSection', [Z(40, name='cs2')], tkey=['ptr'])
    A('EnterCriticalSection', ['$cs1'], tkey=['ptr'])
    A('LeaveCriticalSection', ['$cs1'], tkey=['ptr'])
    A('EnterCriticalSection', ['$cs2'], note='scenario')
    A('LeaveCriticalSection', ['$cs2'], note='scenario')
    A('DeleteCriticalSection', ['$cs1'], tkey=['ptr'])
    A('DeleteCriticalSection', ['$cs2'], note='scenario')
    A('AcquireSRWLockExclusive', [Z(8, name='srw1')], tkey=['ptr'], note='statically initialised lock (SRWLOCK_INIT = 0); the buffer after the call is recorded')
    A('ReleaseSRWLockExclusive', ['$srw1'], tkey=['ptr'], note='the buffer after the call is recorded')
    A('InitializeSListHead', [Z(16, [(0, 'ff' * 16)])], tkey=['ptr'], note='buffer pre-filled with ff to show what is written')
    A('EncodePointer', ['$tlsval'], capture='enc1', tkey=['ptr'])
    A('DecodePointer', ['$enc1'], capture='dec1', tkey=['ptr'])
    A('@eq', ['$dec1', '$tlsval'], note='round trip: 1 expected')
    A('@eq', ['$enc1', '$tlsval'], note='the encoded pointer differs from the pointer: 0 expected')
    A('EncodePointer', ['NULL'], capture='enc0', tkey=['NULL'])
    A('DecodePointer', ['$enc0'], capture='dec0', note='scenario')
    A('@eq', ['$enc0', i64(0)], note='encoded NULL is not NULL: 0 expected')
    A('@eq', ['$dec0', i64(0)], note='decode(encode(NULL)) is NULL: 1 expected')
    A('DecodePointer', ['NULL'], capture='dec00', tkey=None, note='probe: decode of NULL')

    # BSTR
    gen.section('BSTR functions')
    A('SysAllocStringLen', [Wt('False'), d(5)], capture='bs1', tkey=['"False"', '5'])
    A('@bstr', ['$bs1'], note='length prefix and characters (incl. terminator)')
    A('SysAllocStringLen', ['NULL', d(11)], capture='bs2', tkey=['NULL', '0xb'])
    A('@bstr', ['$bs2'], note='uninitialised content: only the prefix and the terminator are defined')
    A('SysAllocStringLen', [Wt('abc'), d(0)], capture='bs3', note='probe: length 0')
    A('@bstr', ['$bs3'])
    A('SysAllocStringLen', [Wt('abc'), d(2)], capture='bs4', note='probe: length shorter than the string')
    A('@bstr', ['$bs4'])
    A('@box', ['$bs1'], capture='bsbox', note='helper: 8-byte buffer holding the BSTR pointer')
    A('SysReAllocStringLen', ['$bsbox', Wt('en_US'), d(5)], tkey=['ptr', '"en_US"', '5'], note='realloc the BSTR through the box')
    A('@bstrp', ['$bsbox'], note='the BSTR after the call')
    A('SysFreeString', ['$bs2'], tkey=['ptr'])
    A('SysFreeString', ['$bs3'], note='scenario')
    A('SysFreeString', ['$bs4'], note='scenario')

    # =============== 5. file system, registry, shell ===============
    gen.section('GetFileAttributesW / GetFileAttributesExW / FindFirstFileW / GetFileVersionInfoSizeW (read-only path queries)')
    done = set()
    for key, a in gen.shapes('GetFileAttributesW'):
        if key[0] in done:
            continue
        done.add(key[0])
        A('GetFileAttributesW', [Wt(key[0].strip('"'))], tkey=list(key))
    for pth in ('C:\\Windows', 'C:\\Windows\\System32', 'C:\\Windows\\Fonts', 'C:\\Windows\\System32\\kernel32.dll', 'C:\\Program Files', 'C:\\Windows\\Fonts\\arial.ttf', 'C:\\Windows\\Fonts\\segoeui.ttf',
                'C:\\Windows\\NoSuchDirectory\\x.ttf', 'C:\\Windows\\NoSuchFile.ttf', 'C:\\Windows\\Fonts\\NoSuchFile.ttf', 'Q:\\no\\such', 'C:\\Windows\\System32\\kernel32.dll\\x', '', 'C:', 'C:\\', '.', '..',
                'Shared\\', 'c:\\program files\\image-line', 'C:\\Program Files\\Image-Line\\FL Studio 21', 'C:\\Windows\\Fonts\\'):
        A('GetFileAttributesW', [Wt(pth)], note='probe: which last error does Windows give for this path shape')
    for key, a in gen.shapes('GetFileAttributesExW'):
        A('GetFileAttributesExW', [Wt(key[0].strip('"')), d(0), O(36)], tkey=list(key))
    A('GetFileAttributesExW', [Wt('C:\\Windows\\System32\\kernel32.dll'), d(0), O(36)], note='probe: existing file')
    for key, a in gen.shapes('FindFirstFileW'):
        A('FindFirstFileW', [Wt(key[0].strip('"')), O(592)], tkey=list(key))
    A('FindFirstFileW', [Wt('C:\\Windows\\System32\\kernel32.dll'), O(592)], note='probe: existing file (handle is closed at once)')
    A('FindFirstFileW', [Wt('C:\\Windows\\NoSuchDirectory\\*'), O(592)], note='probe: missing directory')
    for key, a in gen.shapes('GetFileVersionInfoSizeW'):
        A('GetFileVersionInfoSizeW', [Wt(key[0].strip('"')), D(0)], tkey=list(key))
    A('GetFileVersionInfoSizeW', [Wt('C:\\Windows\\System32\\kernel32.dll'), D(0)], note='probe: existing file')

    gen.section('Registry: read-only opens (access forced to KEY_READ by the tool), values of the OS version key')
    opens = []
    seenk = set()
    for (seq, ap, toks, raw, ret, le, o) in gen.calls:
        if ap in ('RegOpenKeyExW', 'RegOpenKeyExA') and toks[1][0] == 'str':
            root, sub = iv(toks[0]), toks[1][1]
            if root not in (0x80000001, 0x80000002, 0xffffffff80000001, 0xffffffff80000002):
                continue
            k = (ap, root & 0xffffffff, sub)
            if k in seenk:
                continue
            seenk.add(k); opens.append((ap, root, sub, tuple(norm_arg(ap, i, t) for i, t in enumerate(toks)), toks))
    nreg = 0
    for ap, root, sub, tk, toks in opens:
        nreg += 1
        cap = 'rk%d' % nreg
        m = Agg()
        for (k2, a2) in gen.agg.items():
            if k2[0] == ap and k2[1][1] == '"' + sub + '"' and iv(('raw', k2[1][0])) is not None and (iv(('raw', k2[1][0])) & 0xffffffff) == (root & 0xffffffff):
                m.n += a2.n; m.seq = m.seq or a2.seq; m.rets.update(a2.rets); m.les.update(a2.les)
        A(ap, [i64(root), Wt(sub) if ap.endswith('W') else At(sub), d(0), d(iv(toks[3])), O(8)], capture=cap, agg=m if m.n else None, tkey=list(tk), note='all traced opens of this key/root merged; samDesired is replaced by KEY_READ')
        if sub.lower() == 'software\\microsoft\\windows nt\\currentversion':
            for vn in ('CurrentMajorVersionNumber', 'CurrentMinorVersionNumber', 'CurrentVersion', 'CurrentBuild', 'CurrentBuildNumber', 'DisplayVersion', 'ReleaseId', 'UBR', 'ProductName'):
                A('RegQueryValueExW', ['$' + cap, Wt(vn), 'NULL', 'NULL', 'NULL', D(0)], tkey=['H', '"%s"' % vn, 'NULL', 'NULL', 'NULL', 'ptr'] if vn in ('CurrentMajorVersionNumber', 'CurrentVersion', 'CurrentBuild', 'DisplayVersion', 'ReleaseId', 'UBR') else None, note='size query as in the trace')
                A('RegQueryValueExW', ['$' + cap, Wt(vn), 'NULL', O(4), O(512), D(512)], note='probe: type and data')
        if sub.lower() == 'software\\microsoft\\windows nt\\currentversion\\fonts':
            A('RegQueryInfoKeyW', ['$' + cap, 'NULL', 'NULL', 'NULL', D(0), D(0), 'NULL', D(0), D(0), D(0), 'NULL', O(8)], tkey=None, note='counts of subkeys / values (the key exists on Windows; RegCreateKeyExW would open it)')
        A('RegCloseKey', ['$' + cap], note='scenario: closes the handle if the open succeeded (skipped by the tool when the open failed)')

    A('RegOpenKeyExW', [i64(0xffffffff80000002), Wt('Software\\Microsoft\\Windows NT\\CurrentVersion\\Fonts'), d(0), d(0xf003f), O(8)], capture='rkfonts',
      note='the key the guest created with RegCreateKeyExW (decision 0085 question): does it exist? samDesired replaced by KEY_READ')
    A('RegQueryInfoKeyW', ['$rkfonts', 'NULL', 'NULL', 'NULL', D(0), D(0), 'NULL', D(0), D(0), D(0), 'NULL', O(8)], note='subkey / value counts of the Fonts key (the traced call: both 0 in the runtime)')
    A('RegCloseKey', ['$rkfonts'], note='scenario')

    gen.section('SHGetFolderPathW (shell32 and shfolder); CSIDL_FLAG_CREATE is refused by the tool')
    for key, a in gen.shapes('SHGetFolderPathW'):
        A('SHGetFolderPathW', ['NULL', d(int(key[1], 0)), 'NULL', d(0), O(520)], tkey=list(key))
        A('SHGetFolderPathW', ['NULL', d(int(key[1], 0)), 'NULL', d(0), O(520)], dll='shfolder', note='same call through shfolder.dll')
    for csidl in (0x00, 0x02, 0x05, 0x07, 0x0b, 0x10, 0x14, 0x1a, 0x1c, 0x1d, 0x1f, 0x23, 0x24, 0x25, 0x26, 0x28, 0x2a, 0x2b, 0x2c, 0x2e, 0x35, 0x36):
        if ('0', hex(csidl) if csidl > 9 else str(csidl), '0', '0', 'ptr') in [k for k, _ in gen.shapes('SHGetFolderPathW')]:
            continue
        A('SHGetFolderPathW', ['NULL', d(csidl), 'NULL', d(0), O(520)], note='probe: CSIDL sweep (paths are masked)')
    A('SHGetMalloc', [O(8)], tkey=['ptr'])
    A('OleInitialize', ['NULL'], tkey=['NULL'])
    A('OleInitialize', ['NULL'], tkey=['NULL'], note='second call on the same thread')
    A('OleUninitialize', [], note='helper')
    A('OleUninitialize', [], note='helper')

    # =============== 6. user32 / gdi32 ===============
    gen.section('GetSystemMetrics / GetSysColor / SystemParametersInfoW (get-only) (screen dependent: reference)')
    sm_seen = {int(k[0], 0) for k, _ in gen.shapes('GetSystemMetrics')}
    for key, a in gen.shapes('GetSystemMetrics'):
        A('GetSystemMetrics', [d(int(key[0], 0))], tkey=list(key))
    for idx in list(range(0, 140)) + list(range(0x2000, 0x2008)):
        if idx not in sm_seen:
            A('GetSystemMetrics', [d(idx)], note='probe: index sweep')
    for idx in range(0, 31):
        A('GetSysColor', [d(idx)], tkey=['8'] if idx == 8 else None, note='' if idx == 8 else 'probe: COLOR_* sweep')
    for key, a in gen.shapes('SystemParametersInfoW'):
        act, ui = int(key[0], 0), int(key[1], 0)
        size = {0x1f: 92, 0x29: 504, 0x30: 16}.get(act, 4)
        patches = [(0, le32(504))] if act == 0x29 else None
        A('SystemParametersInfoW', [d(act), d(ui), Z(size, patches), d(0)], tkey=list(key), note='get-only action; NONCLIENTMETRICSW cbSize=504 as in the trace' if act == 0x29 else 'get-only action')
    for act in (0x1, 0x3, 0x4a, 0x1000, 0x1002, 0x1008, 0x100a, 0x1012, 0x2000, 0x2006, 0x200e, 0x2010):
        A('SystemParametersInfoW', [d(act), d(0), O(16), d(0)], note='probe: more get-only actions (0x3 = SPI_GETMOUSE writes 3 integers)')

    gen.section('GetDeviceCaps on the screen DC and on a memory DC; EnumDisplay*, monitors')
    A('GetDC', ['NULL'], capture='sdc', tkey=['0'])
    for idx in range(0, 121):
        A('GetDeviceCaps', ['$sdc', d(idx)], tkey=['H', hex(idx) if idx > 9 else str(idx)], note='screen DC')
    A('CreateCompatibleDC', ['NULL'], capture='mdc0', tkey=['0'])
    for idx in range(0, 121):
        A('GetDeviceCaps', ['$mdc0', d(idx)], note='memory DC (default 1x1 mono bitmap)')
    A('DeleteDC', ['$mdc0'], tkey=['H'])
    A('GetClipCursor', [O(16)], tkey=['ptr'])
    A('GetKeyboardLayout', [d(0)], tkey=['0'])
    A('EnumDisplaySettingsW', ['NULL', d(0xffffffff), Z(220, [(68, u16h(220))])], tkey=['NULL', '0xffffffff', 'ptr'], note='DEVMODEW with dmSize=220')
    A('EnumDisplayMonitors', ['NULL', 'NULL', 'F:monitors', 'NULL'], tkey=['0', 'NULL', 'ptr', 'ptr'], note='built-in callback; monitor rectangles (reference)')
    A('MonitorFromPoint', [i64(0), d(1)], capture='mon1', note='helper: primary monitor')
    A('GetMonitorInfoW', ['$mon1', Z(40, [(0, le32(40))])], tkey=['H', 'ptr'])
    for face in ('', 'Segoe UI', 'MS Sans Serif', 'Arial', 'Calibri', 'Tahoma', 'Default', 'Courier New'):
        A('EnumFontFamiliesExW', ['$sdc', Z(92, [(23, '01')] + ([(28, wstr_hex(face))] if face else [])), 'F:fonts', 'NULL', d(0)], tkey=['H', 'ptr', 'ptr', 'ptr', '0'] if face == '' else None, note='built-in callback counts families; LOGFONTW contents are not in the trace; face=%r' % face)
    A('GetProcessDpiAwareness', [i64(0xffffffffffffffff), O(4)], tkey=['0xffffffffffffffff', 'ptr'])
    A('GetProcessDpiAwareness', ['NULL', O(4)], note='probe: NULL process')
    A('acmMetrics', ['NULL', d(0x32), O(4)], tkey=['0', '0x32', 'ptr'])
    A('ReleaseDC', ['NULL', '$sdc'], tkey=['0', 'H'])

    gen.section('Resources: LoadImageW / LoadIconW / LoadCursorW / DestroyCursor / LoadStringW / GetClassInfoW / GetClipboardFormatNameW')
    seenli = set()
    for key, a in gen.shapes('LoadImageW'):
        if key[0] != '0' or key[1:] in seenli:
            continue
        seenli.add(key[1:])
        A('LoadImageW', ['NULL', i64(int(key[1], 0)), d(int(key[2], 0)), d(int(key[3], 0)), d(int(key[4], 0)), d(int(key[5], 0))], capture='cur%d' % len(seenli), tkey=list(key))
    for n in range(1, len(seenli) + 1):
        A('DestroyCursor', ['$cur%d' % n], tkey=['H'], note='shared (LR_SHARED) cursor handle')
    A('LoadIconW', ['NULL', i64(0x7f00)], capture='ico1', tkey=['0', '0x7f00'])
    A('LoadIconW', ['NULL', Wt('MAINICON')], tkey=['0', 'ptr'], note='the trace passed a pointer; the name MAINICON is a probe (decision-0108 question)')
    A('LoadCursorW', ['NULL', i64(0x7f00)], tkey=['0', '0x7f00'])
    A('LoadStringW', ['M:self', d(1), O(512), d(256)], note='probe: failure path of LoadStringW (this exe has no string table); expect 0 and a last error')
    A('LoadStringW', ['M:self', d(0xffb6), O(512), d(256)], note='probe: id from the trace, in this exe')
    A('GetClassInfoW', ['M:self', Wt('TPUtilWindow'), O(72)], tkey=['0x400000', '"TPUtilWindow"', 'ptr'], note='unregistered class: expect 0 and ERROR_CLASS_DOES_NOT_EXIST')
    A('GetClassInfoW', ['NULL', Wt('Button'), O(72)], note='probe: system class')
    A('GetClipboardFormatNameW', [d(0), O(510), d(0xff)], tkey=['0', 'ptr', '0xff'])
    A('GetClipboardFormatNameW', [d(1), O(510), d(0xff)], note='probe: predefined format id (not a registered name)')

    gen.section('GDI on memory DCs: pipeline modelled on the traced sequence (CreateCompatibleDC / CreateDIBSection / SelectObject / PatBlt / ...)')
    bmi = Z(44, [(0, le32(40, 32, 32)), (12, u16h(1) + u16h(32))])
    A('GetDC', ['NULL'], capture='dc_s', tkey=['0'])
    A('CreateCompatibleDC', ['$dc_s'], capture='mdc1', tkey=['H'])
    A('CreateDIBSection', ['$dc_s', bmi, d(0), O(8, 'bits1'), i64(0), d(0)], capture='dib1', tkey=['H', 'ptr', '0', 'ptr', '0', '0'], note='BITMAPINFOHEADER 32x32, 1 plane, 32 bpp, BI_RGB: inferred from the traced 0x20x0x20 PatBlt/StretchDIBits and GetDeviceCaps(BITSPIXEL)=32; not recorded')
    A('SelectObject', ['$mdc1', '$dib1'], capture='prev1', tkey=['H', 'H'])
    A('GetObjectType', ['$mdc1'], note='helper: OBJ_MEMDC = 10')
    A('@peekp', ['$bits1', d(16)], note='DIB pixels right after CreateDIBSection (zero?)')
    A('PatBlt', ['$mdc1', d(0), d(0), d(0x20), d(0x20), d(0xff0062)], tkey=['H', '0', '0', '0x20', '0x20', '0xff0062'], note='WHITENESS')
    A('@peekp', ['$bits1', d(16)], note='DIB pixels after PatBlt WHITENESS (alpha byte?)')
    A('PatBlt', ['$mdc1', d(0), d(0), d(0x20), d(0x20), d(0x42)], note='probe: BLACKNESS')
    A('@peekp', ['$bits1', d(16)], note='DIB pixels after PatBlt BLACKNESS')
    A('PatBlt', ['$mdc1', d(0), d(0), d(0x20), d(0x20), d(0xf00021)], note='probe: PATCOPY (default brush)')
    A('@peekp', ['$bits1', d(16)], note='DIB pixels after PatBlt PATCOPY')
    src = Z(4096, [(0, '332211' + '00' + '665544' + 'ff')])
    A('StretchDIBits', ['$mdc1', d(0), d(0), d(0x20), d(0x20), d(0), d(0), d(0x20), d(0x20), src, bmi, d(0), d(0xcc0020)], tkey=['H', '0', '0', '0x20', '0x20', '0', '0', '0x20', '0x20', 'ptr', 'ptr', '0', '0xcc0020'], note='1:1 SRCCOPY; source pixel0=00112233 (alpha 0), pixel1=ff445566 (alpha ff)')
    A('@peekp', ['$bits1', d(16)], note='DIB pixels after StretchDIBits (alpha bytes of the two source pixels)')
    A('GetObjectW', ['$dib1', d(0x68), O(104)], tkey=['H', '0x68', 'ptr'], note='DIBSECTION of the 32 bpp section')
    A('SelectObject', ['$mdc1', '$prev1'], tkey=['H', 'H'])
    A('DeleteDC', ['$mdc1'], tkey=['H'])
    A('ReleaseDC', ['NULL', '$dc_s'], tkey=['0', 'H'])
    A('CreateCompatibleDC', ['NULL'], capture='mdc2', tkey=['0'])
    A('SelectObject', ['$mdc2', '$dib1'], capture='prev2', tkey=['H', 'H'])
    A('GetDIBColorTable', ['$mdc2', d(0), d(0x100), O(1024)], tkey=['H', '0', '0x100', 'ptr'], note='32 bpp DIB has no colour table: 0 expected')
    A('SelectObject', ['$mdc2', '$prev2'], tkey=['H', 'H'])
    A('DeleteDC', ['$mdc2'], tkey=['H'])
    gen.section('Memory DC defaults (a fresh CreateCompatibleDC(NULL)): Set* return the previous value')
    A('CreateCompatibleDC', ['NULL'], capture='mdc3', tkey=['0'])
    for g in ('GetTextColor', 'GetBkColor', 'GetBkMode', 'GetROP2', 'GetMapMode', 'GetStretchBltMode', 'GetPolyFillMode', 'GetObjectType'):
        A(g, ['$mdc3'], note='default read back (helper)')
    for ot in (1, 2, 5, 6, 7):
        A('GetCurrentObject', ['$mdc3', d(ot)], capture='co%d' % ot, note='OBJ_PEN=1 OBJ_BRUSH=2 OBJ_PAL=5 OBJ_FONT=6 OBJ_BITMAP=7; handle value is opaque, objtype shows what it is')
    A('SelectObject', ['$mdc3', '$dib1'], capture='prev3', tkey=['H', 'H'])
    A('MoveToEx', ['$mdc3', d(0), d(0), 'NULL'], tkey=['H', '0', '0', 'NULL'])
    A('GetCurrentPositionEx', ['$mdc3', O(8)], tkey=['H', 'ptr'])
    A('SetBkColor', ['$mdc3', d(0xffffff)], tkey=['H', '0xffffff'])
    A('GetSysColor', [d(8)], tkey=['8'])
    A('SetTextColor', ['$mdc3', d(0)], tkey=['H', '0'])
    A('SetBkMode', ['$mdc3', d(2)], tkey=['H', '2'])
    A('SetROP2', ['$mdc3', d(13)], tkey=['H', '0xd'])
    A('SetStretchBltMode', ['$mdc3', d(3)], tkey=['H', '3'])
    A('SetStretchBltMode', ['$mdc3', d(1)], tkey=['H', '1'])
    A('SetBkColor', ['$mdc3', d(0x123456)], note='probe: non-default colour, then back')
    A('SetBkColor', ['$mdc3', d(0xffffff)], note='probe')
    A('SetTextColor', ['$mdc3', d(0x654321)], note='probe')
    A('SetTextColor', ['$mdc3', d(0)], note='probe')
    A('SetBkMode', ['$mdc3', d(1)], note='probe: TRANSPARENT then back')
    A('SetBkMode', ['$mdc3', d(2)], note='probe')
    A('SetROP2', ['$mdc3', d(7)], note='probe')
    A('SetROP2', ['$mdc3', d(13)], note='probe')
    A('SelectPalette', ['$mdc3', i64(0), d(0)], tkey=['H', '0', '0'], note='NULL palette: trace shape')
    A('GdiFlush', [], tkey=[])
    A('SelectObject', ['$mdc3', '$prev3'], tkey=['H', 'H'])
    A('GetObjectType', ['$dib1'], note='helper: OBJ_BITMAP = 7')
    A('DeleteObject', ['$dib1'], tkey=['H'])
    A('DeleteDC', ['$mdc3'], tkey=['H'])

    gen.section('GDI objects: bitmap, brush, pen, palette, font, icon (in-process; deleted afterwards)')
    A('CreateBitmap', [d(0x20), d(0x20), d(1), d(1), 'NULL'], capture='bm1', tkey=['0x20', '0x20', '1', '1', 'NULL'])
    A('CreateBitmap', [d(0), d(0), d(1), d(1), 'NULL'], capture='bm0', note='probe: zero size (Learn: 1x1 monochrome)')
    A('GetObjectW', ['$bm1', d(32), O(32)], note='BITMAP of the 32x32 mono bitmap')
    A('GetObjectW', ['$bm0', d(32), O(32)], note='BITMAP of the zero-size bitmap')
    A('CreateBrushIndirect', [Z(16, [(0, le32(0, 0))])], capture='br1', tkey=['ptr'], note='LOGBRUSH BS_SOLID black (contents not in the trace)')
    A('CreateBrushIndirect', [Z(16, [(0, le32(1, 0))])], capture='br2', note='probe: BS_NULL')
    A('GetObjectW', ['$br1', d(16), O(16)], note='LOGBRUSH round trip')
    A('UnrealizeObject', ['$br1'], tkey=['H'], note='brush')
    A('CreatePenIndirect', [Z(16, [(0, le32(0, 1, 0, 0))])], capture='pen1', tkey=['ptr'], note='LOGPEN PS_SOLID width 1 black (contents not in the trace)')
    A('CreatePalette', [Z(12, [(0, u16h(0x300) + u16h(2) + '000000ff' + 'ffffff00')])], capture='pal1', tkey=['ptr'], note='LOGPALETTE version 0x300 with 2 entries (contents not in the trace)')
    A('CreatePalette', [Z(12, [(0, u16h(0x300) + u16h(0))])], capture='pal0', note='probe: 0 entries')
    A('CreatePalette', [Z(12, [(0, u16h(0x200) + u16h(1))])], capture='pal2', note='probe: wrong version')
    A('CreateCompatibleDC', ['NULL'], capture='mdc4')
    A('SelectPalette', ['$mdc4', '$pal1', d(0)], note='probe: select a palette into a memory DC (returns the default palette)')
    A('SelectPalette', ['$mdc4', '$pal1', d(1)], note='probe: background')
    A('SystemParametersInfoW', [d(0x1f), d(92), O(92, 'lf0'), d(0)], note='icon-title font into a persisted buffer')
    A('CreateFontIndirectW', ['$lf0'], capture='font1', tkey=['ptr'], note='LOGFONTW taken from SPI_GETICONTITLELOGFONT (the trace did not record the contents)')
    A('GetObjectW', ['$font1', d(0x5c), O(92)], tkey=['H', '0x5c', 'ptr'], note='LOGFONTW round trip')
    A('CreateFontIndirectW', [Z(92, [(0, le32(-12 & 0xffffffff)), (16, le32(400)), (23, '01'), (28, wstr_hex('MS Sans Serif'))])], capture='font2', note='probe: MS Sans Serif -12')
    A('GetObjectW', ['$font2', d(0x5c), O(92)], note='LOGFONTW round trip')
    A('SelectObject', ['$mdc4', '$font1'], capture='fprev', note='the previous font of a memory DC')
    A('GetObjectType', ['$fprev'], note='helper: OBJ_FONT = 6')
    A('SelectObject', ['$mdc4', '$fprev'], note='restore')
    A('SelectObject', ['$mdc4', '$br1'], capture='bprev', note='select a brush: returns the previous brush')
    A('SelectObject', ['$mdc4', '$bprev'], note='restore')
    A('DeleteDC', ['$mdc4'])
    A('GetStockObject', [d(7)], capture='so7', tkey=['7'])
    A('GetStockObject', [d(5)], capture='so5', tkey=['5'])
    A('GetStockObject', [d(0xd)], capture='so13', tkey=['0xd'])
    A('GetStockObject', [d(7)], capture='so7b', note='second call: same handle?')
    A('@eq', ['$so7', '$so7b'], note='stock object handles are stable: 1 expected')
    for sidx in (0, 1, 2, 3, 4, 6, 8, 10, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21):
        A('GetStockObject', [d(sidx)], note='probe: stock object index %d' % sidx)
    A('DeleteObject', ['$so7'], note='stock objects are not deleted by this tool (policy refuses)')
    A('CreateBitmap', [d(0x20), d(0x20), d(1), d(1), 'NULL'], capture='bm2', note='for CreateIconIndirect')
    A('CreateDIBSection', ['NULL', bmi, d(0), O(8, 'bits9'), i64(0), d(0)], capture='dib9', note='probe: DIB section with hdc=NULL (DIB_RGB_COLORS ignores it)')
    A('@box', ['$bm2'], capture='iibox', note='helper')
    A('@poke', [Z(32, [(0, le32(1, 0, 0))], 'ii1'), d(16), '$bm2'], note='ICONINFO.hbmMask')
    A('@poke', ['$ii1', d(24), '$dib9'], note='ICONINFO.hbmColor')
    A('CreateIconIndirect', ['$ii1'], capture='icon1', tkey=['ptr'], note='ICONINFO: fIcon=1, mask=1bpp 32x32, colour=32bpp DIB (contents not in the trace)')
    A('DestroyIcon', ['$icon1'], note='helper')
    A('DeleteObject', ['$bm2'], note='scenario')
    A('DeleteObject', ['$dib9'], note='scenario')
    A('DeleteObject', ['$bm1'], tkey=['H'])
    A('DeleteObject', ['$bm0'])
    A('DeleteObject', ['$br1'], tkey=['H'])
    A('DeleteObject', ['$pen1'])
    A('DeleteObject', ['$pal1'])
    A('DeleteObject', ['$font1'])
    A('DeleteObject', ['$font2'])

    gen.section('IntersectRect')
    def rect(l, t, r, b):
        return le32(l, t, r, b)
    for nm, r1, r2 in (('overlap', (0, 0, 10, 10), (5, 5, 20, 20)), ('touching edge', (0, 0, 10, 10), (10, 0, 20, 10)), ('disjoint', (0, 0, 10, 10), (20, 20, 30, 30)),
                       ('contained', (0, 0, 100, 100), (10, 10, 20, 20)), ('empty first', (5, 5, 5, 5), (0, 0, 10, 10)), ('negative', (-10, -10, 10, 10), (-5, -5, 0, 0)),
                       ('inverted', (10, 10, 0, 0), (0, 0, 10, 10))):
        A('IntersectRect', [O(16), Z(16, [(0, rect(*r1))]), Z(16, [(0, rect(*r2))])], tkey=['ptr', 'ptr', 'ptr'] if nm == 'overlap' else None, note='probe: %s (the traced rectangles were not recorded)' % nm)

def pseudo_ops_check(rows):
    ok = {'@eq', '@peek', '@peekp', '@bstr', '@bstrp', '@selfcode', '@box', '@poke', '@note'}
    for kind, r in rows:
        if kind == 'row' and r['api'].startswith('@') and r['api'] not in ok:
            raise SystemExit('unknown pseudo op ' + r['api'])

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--trace', default=os.path.join(TRDIR, 'trace.tsv.gz'))
    ap.add_argument('--apis', default=os.path.join(TRDIR, 'tools', 'apis.tsv'))
    ap.add_argument('--gpa', default=os.path.join(TRDIR, 'summary', 'getprocaddress.tsv'))
    ap.add_argument('--out', default=os.path.join(HERE, 'cases.tsv'))
    ap.add_argument('--coverage', default=os.path.join(HERE, 'coverage.tsv'))
    ap.add_argument('--audit-dir', default=os.path.join(REPO, 'docs', 'investigations', 'a2b-audit'))
    ap.add_argument('--whitelist', default=None, help='output of "replay.exe --list-whitelist": check that cases.tsv and the whitelist agree')
    args = ap.parse_args()
    apis = load_apis(args.apis)
    agg, calls, bad = load_trace(args.trace, apis)
    gpa = []
    gagg = {}
    for ln in open(args.gpa, encoding='utf-8'):
        f = ln.rstrip('\n').split('\t')
        if f[0] == 'module':
            continue
        gpa.append((f[0], f[1], f[2], f[4], f[-1]))
    gen = Gen(agg, calls, apis, gpa)
    gen.agg_gpa = {}
    build(gen)
    pseudo_ops_check(gen.rows)
    cols = ['case_id', 'kind', 'api', 'dll', 'args', 'capture', 'rtype', 'cls', 'trace_n', 'trace_seq', 'trace_ret', 'trace_le', 'trace_out', 'note']
    n = 0
    with open(args.out, 'w', encoding='utf-8', newline='') as f:
        f.write('\t'.join(cols) + '\n')
        for kind, r in gen.rows:
            if kind == '#':
                f.write('#== ' + r + '\n')
                continue
            n += 1
            cid = 'C%05d' % n
            vals = [cid, r['kind'], r['api'], r['dll'], r['args'], r['capture'], r['rtype'], r['cls'], str(r['n']), str(r['seq']), r['ret'], r['le'], r['out'], r['note']]
            f.write('\t'.join(v.replace('\t', ' ').replace('\n', ' ') for v in vals) + '\n')
    # coverage per API of the 171
    per = collections.defaultdict(lambda: [0, 0])
    for kind, r in gen.rows:
        if kind == 'row':
            per[r['api']][0] += 1
            if r['n']:
                per[r['api']][1] += 1
    tr_apis = collections.Counter()
    for (seq, ap_, toks, raw, ret, le, o) in calls:
        tr_apis[ap_] += 1
    audit = {}
    for nm in ('K1', 'K2', 'GU'):
        pth = os.path.join(args.audit_dir, 'audit-%s.tsv' % nm)
        if os.path.exists(pth):
            for ln in open(pth, encoding='utf-8'):
                f = ln.rstrip('\n').split('\t')
                if f[0] != 'dll' and len(f) > 3:
                    audit.setdefault(f[1], f[3])
    with open(args.coverage, 'w', encoding='utf-8') as f:
        f.write('api\ttrace_calls\taudit_class\ttool_class\tcases\tcases_with_runtime_answer\tnote\n')
        for a_ in sorted(set(list(apis) + list(per))):
            c = CLS.get(a_, 'helper' if a_ not in apis else '-')
            f.write('%s\t%d\t%s\t%s\t%d\t%d\t%s\n' % (a_, tr_apis.get(a_, 0), audit.get(a_, ''), c, per[a_][0], per[a_][1], KREASON.get(a_, '')))
    if args.whitelist:
        wl = {}
        for ln in open(args.whitelist, encoding='utf-8'):
            ln = ln.rstrip('\r\n')
            if ln.startswith('#') or ln.startswith('name\t'):
                continue
            ff = ln.split('\t')
            if len(ff) >= 5:
                wl[ff[0]] = ff[4]
        used = {r['api'] for k, r in gen.rows if k == 'row' and not r['api'].startswith('@')}
        notwl = sorted(used - set(wl))
        traced_wl = {a_ for a_, c in wl.items() if c == 'traced'}
        want = {a_ for a_ in apis if CLS.get(a_) in ('M', 'R', 'L')}
        print('whitelist: %d entries (%d traced, %d helper)' % (len(wl), len(traced_wl), len(wl) - len(traced_wl)))
        print('  APIs used by cases.tsv but not in the whitelist:', notwl)
        print('  traced whitelist entries that are not M/R classes:', sorted(traced_wl - want))
        print('  M/R classes missing from the whitelist:', sorted(want - traced_wl))
        print('  whitelisted but never used in a case:', sorted(set(wl) - used))
    cov_calls = sum(gen.agg[k].n for k in gen.used_trace_keys)
    gpa_calls = sum(r['n'] for k, r in gen.rows if k == 'row' and r['api'] == 'GetProcAddress')
    print('trace calls (of %d) whose argument shape is a case: %d via argument-shape rows + %d GetProcAddress calls (system modules)' % (len(calls), cov_calls, gpa_calls))
    cnt = collections.Counter(r['kind'] for k, r in gen.rows if k == 'row')
    print('cases: %d  %s' % (n, dict(cnt)))
    print('trace rows that could not be parsed:', dict(bad))
    cl = collections.Counter(CLS.get(a_, '?') for a_ in apis if a_ in apis)
    print('API classes over the traced APIs (%d):' % len(apis), dict(cl))
    missing = [a_ for a_ in apis if CLS.get(a_) is None]
    print('APIs without a class:', missing)
    nocase = [a_ for a_ in apis if CLS.get(a_) not in ('K', None) and per[a_][0] == 0]
    print('APIs of class M/R/L without any case:', nocase)

if __name__ == '__main__':
    main()
