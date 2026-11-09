import os, re, sys, json, hashlib, subprocess
J = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ref = [l for l in open(J + '/ref/native1.out').read().split('\n') if not l.startswith('calling entry')]
rows = []
for dp, dn, fn in sorted(os.walk(J + '/runs')):
    for f in sorted(fn):
        if not f.endswith('.out'):
            continue
        p = os.path.join(dp, f)[:-4]
        name = os.path.relpath(p, J + '/runs')
        out = open(p + '.out', errors='replace').read()
        lines = out.split('\n')
        same = [l for l in lines if not l.startswith('calling entry')] == ref
        t = open(p + '.time').read() if os.path.exists(p + '.time') else ''
        wall = (re.search(r'Elapsed.*: (\S+)', t) or [0, ''])[1]
        rss = (re.search(r'Maximum resident.*: (\d+)', t) or [0, ''])[1]
        ex = (re.search(r'Exit status: (\d+)', t) or [0, ''])[1]
        a4 = 'a4: all_units_done=yes functions_distinct=5306 function_entries=1007942' in out
        err = os.path.getsize(p + '.err') if os.path.exists(p + '.err') else -1
        rows.append((name, ex, wall, rss, len(lines) - 1, err, 'yes' if a4 else 'no', 'yes' if same else 'NO', hashlib.sha256(out.encode()).hexdigest()[:12]))
print('run\texit\twall\tmaxrss_kB\tstdout_lines\tstderr_B\ta4_942_5306_1007942\tstdout_same_as_native_except_rsp_line\tstdout_sha12')
for r in rows:
    print('\t'.join(map(str, r)))
