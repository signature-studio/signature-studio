"""parity1 (decision 0226): parsing of the G3 output of a native run (`g3-cpu:` lines) and a wasm-in-node run (`parity1-maincpu:` lines), shared by summarize.py, mk-master.py and check.py."""
import re, hashlib

def parse(path, wasm):
    """list of snapshots {name, clock (virtual ms), cpu (ms), proc (ms), progress, surf {lid: sha}}"""
    snaps = []; cur = None; pend = None
    for ln in open(path, errors='replace'):
        m = re.match(r'g3-cpu: (\S+) user_ms (\d+) sys_ms (\d+) clock_ms (\d+)', ln)
        if m and not wasm:
            pend = (m.group(1), int(m.group(2)) + int(m.group(3))); continue
        m = re.match(r'g3-snap: #(\d+) (\S+) clock_ms (\d+)', ln)
        if m:
            cur = {'name': m.group(2), 'clock': int(m.group(3)), 'surf': {}, 'cpu': None, 'proc': None, 'progress': None}
            if not wasm and pend and pend[0] == cur['name']:
                cur['cpu'] = pend[1]; cur['proc'] = pend[1]
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

def deltas(snaps):
    """{op: {virtual_ms, cpu_ms, proc_ms, progress, surf}} for every snapshot after the first (delta to the previous snapshot with a cpu value)"""
    out = {}; prev = None
    for s in snaps:
        if s['cpu'] is None: continue
        if prev is not None:
            out[s['name']] = {'virtual_ms': s['clock'] - prev['clock'], 'cpu_ms': s['cpu'] - prev['cpu'], 'proc_ms': (s['proc'] - prev['proc']) if s['proc'] is not None and prev['proc'] is not None else None,
                              'progress': (s['progress'] - prev['progress']) if s['progress'] is not None and prev['progress'] is not None else None, 'surf': surfdig(s['surf'])}
        else:
            out[s['name']] = {'virtual_ms': 0, 'cpu_ms': s['cpu'], 'proc_ms': s['proc'], 'progress': s['progress'], 'surf': surfdig(s['surf'])}
        prev = s
    return out
