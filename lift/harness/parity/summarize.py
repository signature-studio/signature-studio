#!/usr/bin/env python3
"""parity1 (decision 0226): per-operation CPU of a native run and a wasm run of the same G3 script.
usage: summarize.py NATIVE_STDOUT WASM_STDOUT [--ops ops.tsv] [--md OUT.md] [--tsv OUT.tsv]
native CPU = (user+sys) from `g3-cpu:` lines (getrusage of the process, i.e. the engine's single thread); wasm CPU = node main-thread CPU (process.threadCpuUsage, where the wasm runs) from `parity1-maincpu:` lines; procuser/procsys = the whole process (adds V8's compile/tier-up threads).
Delta of snapshot i-1 -> i, both include the tail of one snapshot (surface SHA-256 etc.); `cal_snap` measures that tail."""
import re, sys, hashlib, os
def parse(path, wasm):
    snaps = []; cur = None
    for ln in open(path, errors='replace'):
        m = re.match(r'g3-cpu: (\S+) user_ms (\d+) sys_ms (\d+) clock_ms (\d+) entries (\d+)', ln)
        if m and not wasm:
            cpu = (int(m.group(2)) + int(m.group(3)), int(m.group(2)) + int(m.group(3)))
            cur_cpu = cpu; cur_cpu_label = m.group(1); continue
        m = re.match(r'g3-snap: #(\d+) (\S+) clock_ms (\d+) entries (\d+)', ln)
        if m:
            cur = {'n': int(m.group(1)), 'name': m.group(2), 'clock': int(m.group(3)), 'surf': {}, 'cpu': None, 'proc': None, 'progress': None}
            if not wasm and 'cur_cpu_label' in dir() and cur_cpu_label == cur['name']:
                cur['cpu'] = cur_cpu[0]; cur['proc'] = cur_cpu[0]
            snaps.append(cur); continue
        m = re.match(r'parity1-maincpu: ([\d.]+) s procuser ([\d.]+) procsys ([\d.]+)', ln)
        if m and wasm and cur is not None:
            cur['cpu'] = round(float(m.group(1)) * 1000); cur['proc'] = round((float(m.group(2)) + float(m.group(3))) * 1000); continue
        m = re.match(r'g3-progress: #(\d+) (\S+) progress (\d+)', ln)
        if m and cur is not None:
            cur['progress'] = int(m.group(3)); continue
        m = re.match(r'g3-surface: lid (\d+) (\S+) nonzero (\d+) sha256 (\S+)', ln)
        if m and cur is not None:
            cur['surf'][int(m.group(1))] = m.group(4)
    return snaps
def surfdig(s):
    return hashlib.sha256(' '.join('%d:%s' % (k, s[k]) for k in sorted(s)).encode()).hexdigest()[:10]
nat = parse(sys.argv[1], False); was = parse(sys.argv[2], True)
ops = {}
args = sys.argv[3:]
opsfile = args[args.index('--ops') + 1] if '--ops' in args else os.path.join(os.path.dirname(os.path.abspath(__file__)), 'ops.tsv')
for ln in open(opsfile, encoding='utf-8'):
    if ln.startswith('#') or not ln.strip(): continue
    f = ln.rstrip('\n').split('\t'); ops.setdefault(f[1], (f[2], f[5], f[0]))
nd = {s['name']: s for s in nat}; wd = {s['name']: s for s in was}
names = [s['name'] for s in was]
rows = []
prev_n = prev_w = None
for nm in names:
    w = wd[nm]; n = nd.get(nm)
    pw = prev_w; prev_w = w
    if n is None or n['cpu'] is None or w['cpu'] is None:
        continue
    pn = prev_n; prev_n = n
    if pw is None or pn is None: 
        rows.append((nm, 'start', '-', w['clock'], n['cpu'], w['cpu'], w['proc'], n['progress'], w['progress'], surfdig(n['surf']) == surfdig(w['surf']))); continue
    cat, kind, ses = ops.get(nm, ('prelude', 'D', '-'))
    rows.append((nm, cat, kind, w['clock'] - pw['clock'], n['cpu'] - pn['cpu'], w['cpu'] - pw['cpu'], w['proc'] - pw['proc'], n['progress'] - pn['progress'] if n['progress'] is not None and pn['progress'] is not None else None, w['progress'] - pw['progress'] if w['progress'] is not None and pw['progress'] is not None else None, surfdig(n['surf']) == surfdig(w['surf'])))
hdr = ['op', 'category', 'kind', 'virtual_ms', 'native_cpu_ms', 'wasm_main_cpu_ms', 'wasm_proc_cpu_ms', 'ratio_main', 'native_progress', 'wasm_progress', 'progress_equal', 'surfaces_equal', 'native_cpu_per_vs', 'wasm_cpu_per_vs']
out = []
for r in rows:
    nm, cat, kind, vms, nc, wc, wp, npg, wpg, eq = r
    ratio = (wc / nc) if nc else None
    out.append([nm, cat, kind, vms, nc, wc, wp, ('%.2f' % ratio) if ratio else '-', npg, wpg, ('yes' if npg == wpg else 'no') if npg is not None and wpg is not None else '-', 'yes' if eq else 'NO',
                ('%.3f' % (nc / vms)) if vms else '-', ('%.3f' % (wc / vms)) if vms else '-'])
if '--tsv' in args:
    open(args[args.index('--tsv') + 1], 'w').write('\t'.join(hdr) + '\n' + '\n'.join('\t'.join(str(x) for x in o) for o in out) + '\n')
lines = ['| ' + ' | '.join(hdr) + ' |', '|' + '---|' * len(hdr)] + ['| ' + ' | '.join(str(x) for x in o) + ' |' for o in out]
if '--md' in args:
    open(args[args.index('--md') + 1], 'w').write('\n'.join(lines) + '\n')
else:
    print('\n'.join(lines))
