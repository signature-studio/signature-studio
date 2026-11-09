#!/usr/bin/env python3
"""parity1 (decision 0226) N2: the real FL64.exe (FL Studio 26.1.7.5653 trial, installed from the user's installer by tools/wine-install.sh, no licence handling) under Wine 10.0 + Xvfb, driven in REAL TIME by the same G3 script as the lifted engine (scripts/gen/<session>.wasm.g3).
It interprets the G3 commands (move/down/up/click/wheel/key/tap/advance/snap/end; `open` is skipped: the project is passed on the command line) with X11 XTest events and samples the CPU time (utime+stime) of every process of the Wine prefix and of the X server at each `snap`.
THE INPUT IS A HARNESS SIMULATION, NOT A USER OPERATION.   usage: wine-run.py SCRIPT.g3 OUTDIR [--project WINPATH] [--prefix DIR] [--size WxH] [--display :77]"""
import os, sys, time, subprocess, ctypes, ctypes.util, re, json, signal, struct, zlib
a = sys.argv[1:]
script, out = a[0], a[1]
def opt(n, d=None):
    return a[a.index('--' + n) + 1] if '--' + n in a else d
W = '/home/ubuntu/work/signature-studio-develop/lift/out-parity1'
prefix = opt('prefix', W + '/wine/prefix'); size = opt('size', '1920x1080'); disp = opt('display', ':77'); project = opt('project')
os.makedirs(out, exist_ok=True)
sw, sh = [int(x) for x in size.split('x')]
env = dict(os.environ, DISPLAY=disp, WINEPREFIX=prefix, WINEARCH='win64', WINEDEBUG='-all', WINEDLLOVERRIDES='mscoree,mshtml=d')
fb = out + '/fb'; os.makedirs(fb, exist_ok=True)
xvfb = subprocess.Popen(['Xvfb', disp, '-screen', '0', '%dx%dx24' % (sw, sh), '-fbdir', fb, '-nolisten', 'tcp'], stdout=open(out + '/xvfb.out', 'w'), stderr=subprocess.STDOUT)
time.sleep(2)
x11 = ctypes.CDLL(ctypes.util.find_library('X11')); xtst = ctypes.CDLL(ctypes.util.find_library('Xtst'))
x11.XOpenDisplay.restype = ctypes.c_void_p; x11.XOpenDisplay.argtypes = [ctypes.c_char_p]
x11.XKeysymToKeycode.restype = ctypes.c_ubyte; x11.XKeysymToKeycode.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
dpy = x11.XOpenDisplay(disp.encode())
assert dpy, 'cannot open the display'
def flush(): x11.XFlush(ctypes.c_void_p(dpy))
cx = cy = 0
def motion(x, y):
    global cx, cy
    cx, cy = x, y; xtst.XTestFakeMotionEvent(ctypes.c_void_p(dpy), 0, x, y, 0); flush()
BTN = {'L': 1, 'M': 2, 'R': 3}
def button(b, press): xtst.XTestFakeButtonEvent(ctypes.c_void_p(dpy), BTN[b], 1 if press else 0, 0); flush()
def wheel(delta):
    n = 4 if delta > 0 else 5
    for _ in range(max(1, abs(delta) // 120)):
        button_n(n)
def button_n(n):
    xtst.XTestFakeButtonEvent(ctypes.c_void_p(dpy), n, 1, 0); xtst.XTestFakeButtonEvent(ctypes.c_void_p(dpy), n, 0, 0); flush()
def vk_to_keysym(vk):
    if vk == 13: return 0xff0d
    if vk == 27: return 0xff1b
    if vk == 32: return 0x20
    if 0x70 <= vk <= 0x7b: return 0xffbe + (vk - 0x70)
    if vk == 0x10: return 0xffe1
    if vk == 0x11: return 0xffe3
    if vk == 0x12: return 0xffe9
    if 0x41 <= vk <= 0x5a: return vk + 32
    if 0x30 <= vk <= 0x39: return vk
    raise SystemExit('vk %#x not mapped' % vk)
def key(vk, press):
    kc = x11.XKeysymToKeycode(ctypes.c_void_p(dpy), vk_to_keysym(vk))
    xtst.XTestFakeKeyEvent(ctypes.c_void_p(dpy), kc, 1 if press else 0, 0); flush()
CLK = os.sysconf('SC_CLK_TCK')
def cpu_all():
    """(wine_ticks, xserver_ticks, wine_nproc) over every process whose environment has our WINEPREFIX (the FL64.exe process, wineserver, services), plus Xvfb"""
    wt = 0; n = 0
    for pid in os.listdir('/proc'):
        if not pid.isdigit(): continue
        try:
            e = open('/proc/%s/environ' % pid, 'rb').read()
            if b'WINEPREFIX=' + prefix.encode() + b'\0' not in e: continue
            st = open('/proc/%s/stat' % pid).read(); f = st[st.rindex(')') + 2:].split()
            wt += int(f[11]) + int(f[12]); n += 1
        except Exception: pass
    try:
        st = open('/proc/%d/stat' % xvfb.pid).read(); f = st[st.rindex(')') + 2:].split(); xt = int(f[11]) + int(f[12])
    except Exception: xt = 0
    return wt, xt, n
def screenshot(name):
    # Xvfb -fbdir: Xvfb_screen0 is an XWD file (header, colormap, 32-bit pixels)
    try:
        d = open(fb + '/Xvfb_screen0', 'rb').read()
        hs = struct.unpack('>I', d[:4])[0]; w, h = struct.unpack('>II', d[16:24])
        ncol = struct.unpack('>I', d[76:80])[0]; bpl = struct.unpack('>I', d[48:52])[0]
        off = hs + ncol * 12
        rows = []
        for y in range(h):
            row = d[off + y * bpl: off + y * bpl + w * 4]
            rows.append(b'\x00' + b''.join(row[i + 2:i + 3] + row[i + 1:i + 2] + row[i:i + 1] for i in range(0, w * 4, 4)))
        raw = b''.join(rows)
        def ch(t, dd): c = struct.pack('>I', len(dd)) + t + dd; return c + struct.pack('>I', zlib.crc32(t + dd) & 0xffffffff)
        png = b'\x89PNG\r\n\x1a\n' + ch(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 2, 0, 0, 0)) + ch(b'IDAT', zlib.compress(raw, 3)) + ch(b'IEND', b'')
        open(out + '/' + name + '.png', 'wb').write(png)
    except Exception as ex:
        open(out + '/screenshot-error.txt', 'a').write('%s: %r\n' % (name, ex))
cmd = ['wine', 'C:\\Program Files\\Image-Line\\FL Studio 2026\\FL64.exe'] + ([project] if project else [])
t_start = time.time()
c0 = cpu_all()
fl = subprocess.Popen(cmd, env=env, stdout=open(out + '/wine.out', 'w'), stderr=subprocess.STDOUT)
res = []
def sample(label, extra=''):
    wt, xt, n = cpu_all()
    r = {'label': label, 'wall_s': round(time.time() - t_start, 2), 'wine_cpu_s': round((wt - c0[0]) / CLK, 2), 'xserver_cpu_s': round((xt - c0[1]) / CLK, 2), 'wine_procs': n, 'load1': os.getloadavg()[0]}
    res.append(r); open(out + '/samples.jsonl', 'a').write(json.dumps(r) + '\n'); return r
warmup = float(opt('startup-wait', '8'))
# "wiz0" = the first moment the main window answers: here a fixed wait (no event from the engine to hook); the CPU at that moment is the start-up CPU (a LOWER bound if the wait is shorter than the start-up)
for ln in open(script):
    ln = ln.strip()
    if not ln or ln.startswith('#'): continue
    t = ln.split()
    k = t[0]
    if k == 'snap':
        if t[1] == 'wiz0':
            # first idle = the Wine processes of the prefix used < 0.10 CPU s in each of 3 consecutive 2 s windows (at least `warmup` s after the launch, at most 180 s)
            quiet = 0; last = cpu_all()[0]
            while time.time() - t_start < 180 and (quiet < 3 or time.time() - t_start < warmup):
                time.sleep(2); now = cpu_all()[0]
                quiet = quiet + 1 if (now - last) / CLK < 0.10 else 0; last = now
            screenshot('wiz0')
        sample(t[1]); 
        if t[1] in ('e03', 'opened', 'end'): screenshot(t[1])
    elif k == 'cpu': pass
    elif k == 'end': break
    elif k == 'advance': time.sleep(int(t[1]) / 1000.0)
    elif k == 'move': motion(int(t[1]), int(t[2]))
    elif k == 'down': button(t[1], True)
    elif k == 'up': button(t[1], False)
    elif k == 'click':
        motion(int(t[1]), int(t[2])); b = t[3] if len(t) > 3 else 'L'; button(b, True); button(b, False)
    elif k == 'wheel':
        motion(int(t[1]), int(t[2])); wheel(int(t[3]))
    elif k == 'tap': key(int(t[1], 0), True); key(int(t[1], 0), False)
    elif k == 'key': key(int(t[2], 0), t[1] == 'down')
    elif k == 'open': pass
    elif k == 'display': pass
    else: raise SystemExit('unknown command ' + ln)
screenshot('final')
for p in (fl,):
    p.terminate()
subprocess.run(['/usr/lib/x86_64-linux-gnu/wine/wineserver', '-k'], env=env)
xvfb.terminate()
json.dump({'script': script, 'project': project, 'size': size, 'startup_wait_s': warmup, 'samples': res}, open(out + '/result.json', 'w'), indent=1)
