#!/usr/bin/env python3
"""alpha1: the observation tables from the recorder's api.log files (native -O2 N1, G3 scripts; THE INPUT IS A HARNESS SIMULATION). usage: mk-tables.py"""
import re, collections, os
R = '/home/ubuntu/work/signature-studio-develop/lift/out-alpha1'
def events(run):
    for ln in open('%s/runs/%s/api.log' % (R, run), errors='replace'):
        m = re.match(r'E (\d+) (\w+) cls=(\S*) (.*)$', ln.rstrip('\n'))
        if m: yield int(m.group(1)), m.group(2), m.group(3), m.group(4)
# 1. the calls that make a glass window and its fade (File menu: Basic.flp chosen; Add menu; F8)
keep = {'CreateWindowExW', 'DwmExtendFrameIntoClientArea', 'DwmEnableBlurBehindWindow', 'ShowWindow', 'DestroyWindow', 'SetWindowRgn', 'UpdateLayeredWindowIndirect', 'UpdateLayeredWindow', 'SetLayeredWindowAttributes', 'AnimateWindow'}
with open(R + '/tables/glass-calls-timeline.tsv', 'w') as f:
    f.write('run\tclock_ms\tapi\twindow_class\targs\n')
    for run in ('pull1b', 'pick1s', 'f8b'):
        for ms, api, cls, args in events(run):
            if api in ('CreateWindowExW',) and not any(k in args for k in ('TQuickPopupMenuWindow', 'TPlugListForm')): continue
            if api in ('ShowWindow', 'DestroyWindow') and cls not in ('TQuickPopupMenuWindow', 'TPlugListForm'): continue
            if api == 'SetWindowLongPtrW' and not (cls in ('TQuickPopupMenuWindow', 'TPlugListForm') and 'idx=-20' in args): continue
            if api not in keep and api != 'SetWindowLongPtrW': continue
            if api == 'UpdateLayeredWindowIndirect': continue      # 93 calls of the start-up logo (TLightweightLayeredControl); counted in the summary table
            f.write('%s\t%d\t%s\t%s\t%s\n' % (run, ms, api, cls, re.sub(r' ra=0x[0-9a-f]+', '', args)[:240]))
# 2. the capability probes
rows = collections.OrderedDict()
for ln in open(R + '/runs/pick1s/api.log', errors='replace'):
    m = re.match(r'P (\w+) (\d+) (.*) ra=0x[0-9a-f]+ ret=(0|0x[0-9a-f]+)$', ln.rstrip('\n'))
    if m: rows[(m.group(1), m.group(3), m.group(4))] = rows.get((m.group(1), m.group(3), m.group(4)), 0) + int(m.group(2))
with open(R + '/tables/capability-probes.tsv', 'w') as f:
    f.write('api\targuments\treturn_value\tcalls_in_the_run\n')
    for (api, args, ret), n in sorted(rows.items()):
        if api in ('SystemParametersInfoW', 'GetSystemMetrics', 'GetDeviceCaps', 'DwmIsCompositionEnabled', 'DwmExtendFrameIntoClientArea', 'BufferedPaintInit', 'GetSysColor', 'IsThemeActive', 'DwmGetColorizationColor'):
            f.write('%s\t%s\t%s\t%d\n' % (api, args, ret, n))
# 3. API call counts of the whole run
cnt = collections.Counter()
for ln in open(R + '/runs/pick1s/api.log', errors='replace'):
    m = re.match(r'A (\w+) (\d+)$', ln.strip())
    if m: cnt[m.group(1)] += int(m.group(2))
with open(R + '/tables/api-call-counts-pick1s.tsv', 'w') as f:
    f.write('api\tcalls\n')
    for k in ('UpdateLayeredWindow', 'UpdateLayeredWindowIndirect', 'SetLayeredWindowAttributes', 'GetLayeredWindowAttributes', 'AnimateWindow', 'AlphaBlend', 'GradientFill', 'TransparentBlt', 'DwmEnableBlurBehindWindow', 'DwmExtendFrameIntoClientArea', 'DwmSetWindowAttribute', 'DwmGetColorizationColor', 'DwmIsCompositionEnabled', 'IsThemeActive', 'BufferedPaintInit', 'BeginBufferedPaint', 'SetWindowRgn', 'BitBlt', 'StretchBlt', 'StretchDIBits', 'SetDIBitsToDevice', 'CreateDIBSection', 'SystemParametersInfoW', 'GetDeviceCaps', 'GetSystemMetrics'):
        f.write('%s\t%d\n' % (k, cnt.get(k, 0)))
# 4. the picker's source alpha (BitBlt of 1920x1080 into TPlugListForm), F8, run f8b (BBPIX 10,10;500,300;960,540;1500,900;1900,1070;300,60)
with open(R + '/tables/picker-source-alpha.tsv', 'w') as f:
    f.write('clock_ms\tsrc_px_partial\tpix(10,10)=A.R.G.B\tpix(500,300)\tpix(960,540)\tpix(1500,900)\n')
    for ms, api, cls, args in events('f8b'):
        if api == 'BitBlt' and '(0,0 1920x1080)' in args:
            m = re.search(r'partial=(\d+).*pix\(10,10\)=(\S+) pix\(500,300\)=(\S+) pix\(960,540\)=(\S+) pix\(1500,900\)=(\S+)', args)
            if m: f.write('%d\t%s\t%s\t%s\t%s\t%s\n' % (ms, m.group(1), m.group(2), m.group(3), m.group(4), m.group(5)))
print('ok')
