import os, re, subprocess, sys, tempfile

WT = sys.argv[1]
DIFFS = sys.argv[2:]

def split_files(text):
    lines = text.splitlines(keepends=True)
    sections = []
    cur = None
    i = 0
    while i < len(lines):
        ln = lines[i]
        if ln.startswith('diff --git '):
            if cur:
                sections.append(cur)
            cur = {'hdr_old': '', 'hdr_new': '', 'body': [], 'git': ln}
            i += 1
            continue
        if ln.startswith('--- ') and i + 1 < len(lines) and lines[i + 1].startswith('+++ '):
            if cur is None or cur.get('hdr_new'):
                if cur:
                    sections.append(cur)
                cur = {'hdr_old': ln, 'hdr_new': lines[i + 1], 'body': []}
            else:
                cur['hdr_old'] = ln
                cur['hdr_new'] = lines[i + 1]
            cur['body'] += [ln, lines[i + 1]]
            i += 2
            continue
        if cur is not None and (cur.get('hdr_new') or not ln.startswith(('index ', 'new file', 'deleted file', 'similarity', 'rename', 'old mode', 'new mode'))):
            cur['body'].append(ln)
        i += 1
    if cur:
        sections.append(cur)
    return sections

def fname(sec):
    if not sec['hdr_new'] and sec.get('git'):
        return re.sub(r'^(a/|b/)', '', sec['git'].split()[3])
    m = re.match(r'\+\+\+ (\S+)', sec['hdr_new'])
    p = m.group(1)
    p = re.sub(r'^(a/|b/|tree/)', '', p)
    return p

def count_pm(body):
    add = dele = 0
    hunks = 0
    for ln in body:
        if ln.startswith('@@'):
            hunks += 1
        elif ln.startswith('+') and not ln.startswith('+++ '):
            add += 1
        elif ln.startswith('-') and not ln.startswith('--- '):
            dele += 1
    return add, dele, hunks

def run_patch(sec_text, reverse=False):
    fd, path = tempfile.mkstemp(suffix='.diff', dir=os.environ.get('TMPDIR', '/tmp/opencode/r4-5a'))
    with os.fdopen(fd, 'w') as f:
        f.write(sec_text)
    cmd = ['patch', '-p1', '--dry-run'] + (['-F0'] if os.environ.get('STRICT') == '1' else [])
    if reverse:
        cmd.append('-R')
    cmd += ['-i', path]
    p = subprocess.run(cmd, cwd=WT, capture_output=True, text=True)
    os.unlink(path)
    out = p.stdout + p.stderr
    failed = len(re.findall(r'^Hunk #\d+ FAILED', out, re.M))
    return p.returncode, failed, out

print('diff\tfile\t+\t-\thunks\tfwd_failed_hunks\tfwd_rc\trev_failed_hunks\tstatus')
for d in DIFFS:
    with open(d, encoding='utf-8', errors='replace') as f:
        text = f.read()
    for sec in split_files(text):
        name = fname(sec)
        body = sec['body']
        add, dele, hunks = count_pm(body)
        sec_text = ''.join(body)
        if not sec_text.endswith('\n'):
            sec_text += '\n'
        frc, ffail, fout = run_patch(sec_text)
        rrc, rfail, rout = (None, None, '')
        if frc != 0:
            rrc, rfail, rout = run_patch(sec_text, reverse=True)
        if frc == 0:
            status = 'applies'
        elif rrc == 0:
            status = 'already-present(reverse applies)'
        else:
            status = 'FAILS'
        exists = os.path.exists(os.path.join(WT, name))
        creates = any(re.match(r'@@ -0,0 ', ln) for ln in body)
        if not exists:
            if creates:
                status = 'NEW(creates file, ' + ('applies' if frc == 0 else 'FAILS') + ')'
            else:
                status = 'TARGET-ABSENT(modifies a file not in HEAD)'
        elif creates:
            status = status + '+creates'
        print('%s\t%s\t%d\t%d\t%d\t%d\t%d\t%s\t%s' % (
            os.path.basename(d).replace('.diff', ''), name, add, dele, hunks,
            ffail, frc, rfail if rfail is not None else '-', status))
