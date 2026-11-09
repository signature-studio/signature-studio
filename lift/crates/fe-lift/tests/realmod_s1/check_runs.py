"""Checks the logs of a run directory against expectations written from the specification (Learn DllMain, LoadLibrary, GetProcAddress),
not copied from an output. usage: check_runs.py RUNDIR   (read only; prints PASS/FAIL per check, exit 1 if any FAIL)
The copy of lift/out-stage1/check_runs.py for tests/realmod.rs: the negative-control logs are read from RUNDIR (neg-header-only.out/.err)."""
import re, sys, os
d = sys.argv[1]
BASE = 0x2000000
KERNEL32_HANDLE = 0x7FF800000000 + 0x10000 * 9      # fle_modules[8] = kernel32.dll (decision 0015 order)
MON_HANDLE = 0x7FEA0004                               # FLE_MONITOR_HANDLE (decision 0065)

def load(name):
    p = os.path.join(d, name + '.out')
    log, meta = [], {}
    for l in open(p):
        m = re.match(r'log\s+(\d+) (.{4}) (0x[0-9a-f]+|0)$', l.rstrip('\n'))
        if m: log.append((m.group(2), int(m.group(3), 16)))
        m = re.match(r'exit=(\d+)', l)
        if m: meta['exit'] = int(m.group(1))
        m = re.match(r'max callback depth at a DllMain call: (\d+)', l)
        if m: meta['depth'] = int(m.group(1))
        if l.startswith('scenario returned'): meta['returned'] = True
    meta['err'] = open(os.path.join(d, name + '.err')).read()
    return log, meta

bad = 0
def chk(name, cond, detail=''):
    global bad
    print(('PASS ' if cond else 'FAIL ') + name + ((' :: ' + detail) if (detail and not cond) else ''))
    if not cond: bad += 1

def tags(log): return [t for t, _ in log]
def val(log, t, n=0):
    v = [x for k, x in log if k == t]
    return v[n] if len(v) > n else None

# --- 1/3/4: basic ---------------------------------------------------------------------------
log, meta = load('s1-basic')
chk('basic: scenario returned and exit 0', meta.get('returned') and meta['exit'] == 0)
# order: the host logs HBEF, then DllMain runs entirely (DENT .. DEMR) BEFORE LoadLibraryW returns to the host (HAFT)
t = tags(log)
chk('order: HBEF < DENT(attach) < DEMR < HAFT', t.index('HBEF') < t.index('DENT') < t.index('DEMR') < t.index('HAFT'), str(t))
chk('DllMain called once with reason 1 (DLL_PROCESS_ATTACH)', t.count('DENT') == 1 and val(log, 'DENT') == 1)
chk('DllMain hinstDLL == base 0x2000000', val(log, 'DHIN') == BASE)
chk('DllMain lpvReserved == NULL (dynamic load)', val(log, 'DRES') == 0)
chk('LoadLibraryW returned the base as the handle', val(log, 'HAFT') == BASE)
chk('DllMain reached its own function (Add(20,22) == 42)', val(log, 'DADD') == 42)
chk('DllMain used an import (SetLastError/GetLastError == 0x1111)', val(log, 'DLE ') == 0x1111)
chk('DllMain: GetProcAddress(kernel32, ..) callable from the second module', val(log, 'DPEQ') == 1)
chk('re-entry: DllMain -> EnumDisplayMonitors -> callback in the 2nd module (dwData 0xABCD)', val(log, 'DECB') == 0xABCD)
chk('re-entry: callback saw monitor handle, NULL hdc, right == 1920', val(log, 'DEC1') == MON_HANDLE and val(log, 'DEC2') == 0 and val(log, 'DEC3') == 1920)
chk('re-entry: EnumDisplayMonitors returned TRUE to DllMain', val(log, 'DEMR') == 1)
chk('re-entry depth: fle_call_guest depth was 1 when LoadLibraryW called DllMain (scenario is depth 1 -> DllMain depth 2)', meta.get('depth') == 1)
# 3
chk('GetProcAddress("Add") == base + export RVA (0x2001000 from the export table)', val(log, 'HPAD') == 0x2001000)
chk('missing export: NULL and ERROR_PROC_NOT_FOUND (127)', val(log, 'HMIS') == 0 and val(log, 'HMER') == 127)
chk('call [rax] (memory form) through the returned VA: Add(40,2) == 42', val(log, 'HADD') == 42)
chk('call rax (register form) through the returned VA: Twice(21) == 42', val(log, 'HTWI') == 42)
chk('relocated pointer in .data == export VA (g_fnptr == Add)', val(log, 'HSTO') == 0x2001000 and val(log, 'HEQ ') == 1)
chk('call through the relocated .data pointer: CallStored(5,6) == 11', val(log, 'HCST') == 11)
chk('host function called from the 2nd module (dispatch both ways): RunCb(HostCb,7) == 15 and HostCb saw 7', val(log, 'HRCB') == 15 and val(log, 'HCB ') == 7)

# --- negative control: header-only rebase -------------------------------------------------------
p = os.path.join(d, 'neg-header-only.out')
if os.path.exists(p):
    txt = open(p).read()
    chk('negative control (ImageBase changed, no DIR64 applied): the .data pointer is NOT the export VA (0x180001000)', 'HSTO 0x180001000' in txt and re.search(r'HEQ  0$', txt, re.M) is not None)
    e = open(os.path.join(d, 'neg-header-only.err')).read()
    chk('negative control: the call through it stops on a dispatch miss', 'indirect call to 0x180001000 is not a known function entry' in e)

# --- 2: DllMain returns FALSE --------------------------------------------------------------------
log, meta = load('s1-fail-always')
t = tags(log)
chk('fail: first LoadLibraryW returned NULL', val(log, 'HAF1') == 0)
chk('fail: last error after the failed load == 0x5678 (set by DllMain, left alone)', val(log, 'HLE1') == 0x5678)
chk('fail: DllMain(DETACH, NULL) ran after ATTACH returned FALSE and before LoadLibraryW returned (Learn)', t.index('DEMR') < t.index('DRS0') < t.index('HAF1') and val(log, 'DENT', 1) == 0 and val(log, 'DRS0') == 0)
chk('fail: a second LoadLibraryW attaches again (module was unloaded): ATTACH twice', [x for k, x in log if k == 'DENT'] == [1, 0, 1, 0])
chk('fail: second LoadLibraryW also NULL', val(log, 'HAF2') == 0)
log, meta = load('s1-fail-first')
chk('fail-first: NULL then handle (DllMain true the second time)', val(log, 'HAF1') == 0 and val(log, 'HAF2') == BASE)
chk('fail-first: attach, detach, attach (no second detach)', [x for k, x in log if k == 'DENT'] == [1, 0, 1])

# --- self-load from DllMain (re-entry into the loader) -------------------------------------------
log, meta = load('s1-basic-selfload')
chk('selfload: DllMain LoadLibraryW(itself) during ATTACH returns the base, no second DllMain entry', val(log, 'DSLF') == BASE and [x for k, x in log if k == 'DENT'] == [1])
chk('selfload: DllMain LoadLibraryW(kernel32) then FreeLibrary returns TRUE', val(log, 'DK32') == KERNEL32_HANDLE and val(log, 'DFK ') == 1)
chk('selfload: rest of the scenario unchanged', val(log, 'HADD') == 42 and val(log, 'HRCB') == 15 and meta['exit'] == 0)

# --- unmodified runtime ----------------------------------------------------------------------------
# decision 0233 (old: the run stopped at LoadLibraryW("mod2.dll") with exit 134 and 'module "mod2.dll" is not a virtual module'): a name that no module claims is searched for on the virtual volume and not found, which is the
# page's failure (LoadLibraryW: "If the function cannot find the module, the function fails", "the return value is NULL"): the host gets NULL (HAFT 0) and its own code goes on (HostBasic returns 1 for a NULL handle)
for n in ('base-loadonly', 'base-basic'):
    log, meta = load(n)
    chk(n + ': unmodified runtime: LoadLibraryW("mod2.dll") returns NULL (no handle invented, nothing of the module ran)', meta['exit'] == 0 and val(log, 'HAFT') == 0 and 'DENT' not in tags(log))
sys.exit(1 if bad else 0)
