#!/usr/bin/env python3
"""alpha1: mutation check of lift/harness/alpha1/glass_seq.c. Each mutation edits one line of a copy of the runtime, rebuilds that one file and runs the test; the test must FAIL (exit status 1 or a build failure is not counted as detected).
usage: mutate-glass.py [SRC]  -> prints one line per mutation: DETECTED / SURVIVED, and the number of FAIL lines."""
import os, shutil, subprocess, sys, re
A = '/home/ubuntu/work/signature-studio-develop/lift/out-alpha1'
SRC = sys.argv[1] if len(sys.argv) > 1 else A + '/src'
BASE = A + '/work/glass'
MUT = [
 ('M01 the margins are not kept', 'fle_dwm.c', 'fle_win_set_frame_margins(R[1], m);', '(void)m;'),
 ('M02 the DIB alpha is not carried into the window surface', 'fle_gdi.c', 't->s.keep_x = 1;', 't->s.keep_x = 0;'),
 ('M03 the compositor ignores the extended frame', 'fle_desk.c', 'int glass = !w->layered && (w->lflags & LF_GLASS) != 0;', 'int glass = 0;'),
 ('M04 the compositor ignores the window region', 'fle_desk.c', 'w->cl = (int32_t)imax(0, imin(l, (int64_t)w->w));', 'w->cl = 0; (void)l;'),
 ('M05 the window region is not applied to the right edge', 'fle_desk.c', 'w->cr = (int32_t)imax(w->cl, imin(r, (int64_t)w->w));', 'w->cr = (int32_t)w->w; (void)r;'),
 ('M06 the bottom of the window region is off by one', 'fle_desk.c', 'w->cb = (int32_t)imax(w->ct, imin(b, (int64_t)w->h));', 'w->cb = (int32_t)imax(w->ct, imin(b + 1, (int64_t)w->h));'),
 ('M07 WindowFromPoint ignores the region', 'fle_win.c', 'if (!in_window_region(i, px - ox - tab[i].px, py - oy - tab[i].py)) continue;\n        {\n            int64_t c = hit_test(i, px, py);', '{\n            int64_t c = hit_test(i, px, py);'),
 ('M08 the input hit test ignores the region', 'fle_win.c', 'if (!in_window_region(i, px - ox - tab[i].px, py - oy - tab[i].py)) continue;\n        {\n            int64_t c = hit_visible(i, px, py);', '{\n            int64_t c = hit_visible(i, px, py);'),
 ('M09 the rounding of the blend is truncated', 'fle_desk.c', 'v = (r * sca * 255u + (d & 0xFFu) * k + 32512u) / 65025u;', 'v = (r * sca * 255u + (d & 0xFFu) * k) / 65025u;'),
 ('M10 the colour is added without the (1 - A) of the destination', 'fle_desk.c', 'k = 65025u - a * sca;', 'k = 65025u;'),
 ('M11 the glass strip is the inner part', 'fle_desk.c', 'if (inner_row && wx >= w->gx0 && wx < w->gx1) dst[i] = bgrx_to_rgba(src[i]);', 'if (!(inner_row && wx >= w->gx0 && wx < w->gx1)) dst[i] = bgrx_to_rgba(src[i]);'),
 ('M12 a negative margin is not the sheet of glass', 'fle_desk.c', 'if (m[0] < 0 || m[1] < 0 || m[2] < 0 || m[3] < 0) {', 'if (0) {'),
 ('M13 the right margin is ignored', 'fle_desk.c', 'w->gx1 = (int32_t)imax(iw - m[1], w->gx0);', 'w->gx1 = (int32_t)iw;'),
 ('M14 an A of 0 with a colour is transparent (no added light)', 'fle_desk.c', 'if (a == 0 && (b | g | r) == 0) return d;', 'if (a == 0) return d;'),
 ('M15 a window with zero margins is still extended', 'fle_win.c', 'if (!(tab[i].dwm_margins[0] | tab[i].dwm_margins[1] | tab[i].dwm_margins[2] | tab[i].dwm_margins[3])) return 0;', ''),
 ('M16 the result is not clamped', 'fle_desk.c', 'o = v > 255u ? 255u : v;', 'o = v & 255u;'),

 ('M18 the top margin is ignored', 'fle_desk.c', 'w->gy0 = (int32_t)imin(m[2], ih);', 'w->gy0 = 0;'),
 ('M19 the bottom margin is ignored', 'fle_desk.c', 'w->gy1 = (int32_t)imax(ih - m[3], w->gy0);', 'w->gy1 = (int32_t)ih;'),
 ('M20 a region that starts before the window is not clamped', 'fle_desk.c', 'w->ct = (int32_t)imax(0, imin(t, (int64_t)w->h));', 'w->ct = (int32_t)imin(t, (int64_t)w->h);'),
 ('M21 LWA_ALPHA complement is 256 - alpha', 'fle_desk.c', 'uint32_t ia = 255u - alpha;', 'uint32_t ia = 256u - alpha;'),
 ('M22 LWA_COLORKEY ignores the blue byte', 'fle_desk.c', 'if (keyed && r == kr && g == kg && b == kb) continue;', 'if (keyed && r == kr && g == kg) continue;'),
 ('M23 ULW_OPAQUE keeps the blend alpha', 'fle_layered.c', 'l->in.alpha = kind == FLE_ULW_ALPHA ? sca : 255u;', 'l->in.alpha = kind == FLE_ULW_ALPHA ? sca : 0u;'),
 ('M24 ULW_COLORKEY compares one byte', 'fle_layered.c', '((c ^ crkey) & 0xFFFFFFu) == 0', '((c ^ crkey) & 0xFFu) == 0'),
 ('M25 ULW without AC_SRC_ALPHA uses the A byte of the DIB', 'fle_layered.c', 'if (afmt == FLE_AC_SRC_ALPHA) {\n                if (src.bpp != 32) {', 'if (1) {\n                if (src.bpp != 32) {'),
 ('M26 the premultiplied path ignores SourceConstantAlpha', 'fle_desk.c', 'for (int32_t i = 0; i < n; i++) dst[i] = over_premul(src[i], dst[i], alpha);', 'for (int32_t i = 0; i < n; i++) dst[i] = over_premul(src[i], dst[i], 255u);'),
 ('M27 the windows are drawn from the top', 'fle_desk.c', 'for (unsigned k = n; k-- > 0;) {    /* the list is top first: the picture is drawn from the bottom */', 'for (unsigned k = 0; k < n; k++) {'),
 ('M17 the margins are kept on the wrong side (top for left)', 'fle_dwm.c', 'm[2] = (int32_t)fle_ld(R[2] + 8, 4);', 'm[2] = (int32_t)fle_ld(R[2] + 0, 4);'),
]
res = []
for name, f, old, new in MUT:
    d = A + '/work/mut/' + name.split()[0]
    shutil.rmtree(d, ignore_errors=True)
    os.makedirs(d)
    rt = d + '/lift/runtime'
    shutil.copytree(SRC + '/lift/runtime', rt)
    s = open(rt + '/' + f).read()
    if old not in s:
        res.append((name, 'NOT APPLIED')); continue
    open(rt + '/' + f, 'w').write(s.replace(old, new, 1))
    out = d + '/o'
    shutil.copytree(BASE, out, ignore=shutil.ignore_patterns('glass_seq', '*.log'))
    cc = subprocess.run(['clang', '-O1', '-std=gnu11', '-w', '-fPIE', '-DFLE_DESKTOP', '-c', '-I' + rt, rt + '/' + f, '-o', out + '/' + f.replace('.c', '.o')], capture_output=True, text=True)
    if cc.returncode: res.append((name, 'BUILD FAILED ' + cc.stderr[:200])); continue
    objs = [out + '/' + x for x in os.listdir(out) if x.endswith('.o')]
    ln = subprocess.run(['clang', '-O1', '-std=gnu11', '-w', '-fPIE', '-pthread', '-DFLE_DESKTOP', '-I' + rt, SRC + '/lift/harness/alpha1/glass_seq.c'] + objs + ['-lm', '-o', out + '/glass_seq'], capture_output=True, text=True)
    if ln.returncode: res.append((name, 'LINK FAILED ' + ln.stderr[:200])); continue
    r = subprocess.run([out + '/glass_seq'], capture_output=True, text=True)
    nf = len(re.findall(r'^FAIL', r.stdout, re.M))
    res.append((name, ('DETECTED' if r.returncode != 0 or nf else 'SURVIVED') + ' (%d FAIL lines, rc %d)' % (nf, r.returncode)))
    shutil.rmtree(d, ignore_errors=True)
for n, r in res: print('%-70s %s' % (n, r))
