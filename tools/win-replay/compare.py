#!/usr/bin/env python3
"""compare.py - compare observed.tsv (real Windows) with the answers of the project's runtime recorded in the trace.

  python compare.py --cases cases.tsv --observed observed.tsv [--observed-manifest observed-win10manifest.tsv] [--report compare-report.md]

The 'runtime answer' is what the project's response functions returned in the 6-module run (trace_ret / trace_le /
trace_out columns of cases.tsv). For every case that has a trace counterpart this script says: match / mismatch / reference.
Cases without a trace counterpart (kind=probe) are real-Windows values only ("real-only").

Reading the verdicts:
  match      the observed return value, last error (failure path) and decoded outputs equal one of the answers the runtime gave
  mismatch   they differ in a field Windows defines (a spec fact)
  reference  they differ, but the field depends on the machine / user / time / screen / the way the process was started; recorded, not a spec fact
  real-only  no runtime answer to compare (probes)
  skipped    the tool refused or could not run the case (status column), or the module / export is absent on this Windows
Handles and pointers are compared only as 'non-null opaque'.
"""
import argparse, collections, json, os, re, sys

HERE = os.path.dirname(os.path.abspath(__file__))

# fields / APIs whose value depends on the machine, the user, the clock, the screen or the process start (compared as reference)
# APIs of class R / L in cases.tsv (column cls, written by gen_cases.py) are compared as reference; fields below inside M-class APIs too
REF_FIELDS = {'SYSTEM_INFO.nproc', 'SYSTEM_INFO.mask', 'SYSTEM_INFO.level', 'SYSTEM_INFO.revision', 'OSVERSIONINFO.build', 'EX.suite'}
OPAQUE_RET = {'H', 'PTR'}
HARNESS_APIS = {'GetLastError'}   # the tool sets a sentinel last error before each call, so the value is defined by the tool
LOCALE_APIS = set('GetACP GetOEMCP GetThreadLocale GetUserDefaultUILanguage GetThreadUILanguage GetSystemDefaultLocaleName GetUserDefaultLocaleName GetThreadPreferredUILanguages GetUserDefaultLCID GetSystemDefaultLCID'.split())

def locale_dependent(c):
    """the answer depends on the locale / ANSI code page of the machine (the runtime claims en-US / 1252)"""
    api, a = c['api'], c['args'].split('|')
    first = parse_int(a[0][2:]) if a and a[0][:2] in ('d:', 'i:') else None
    if api in LOCALE_APIS:
        return True
    if api in ('GetLocaleInfoW', 'CompareStringW', 'LCMapStringW', 'IsValidLocale') and first in (0, 0x400, 0x800):
        return True
    if api in ('MultiByteToWideChar', 'WideCharToMultiByte', 'GetCPInfo', 'IsValidCodePage') and first in (0, 1, 2, 3):
        return True
    return False

def unesc(s):
    return re.sub(r'\\u([0-9a-f]{4})', lambda m: chr(int(m.group(1), 16)), s)

def load_tsv(path):
    meta, rows, cols = {}, [], None
    for ln in open(path, encoding='utf-8'):
        ln = ln.rstrip('\r\n')
        if not ln:
            continue
        if ln.startswith('#=='):
            continue
        if ln.startswith('#'):
            f = ln[1:].strip().split('\t')
            if f[0] == 'columns':
                cols = f[1:]
            elif len(f) >= 2:
                meta[f[0]] = f[1]
            continue
        if cols is None and ln.startswith('case_id'):
            cols = ln.split('\t'); continue
        f = ln.split('\t')
        if cols is None:
            continue
        f += [''] * (len(cols) - len(f))
        rows.append(dict(zip(cols, f)))
    return meta, rows

def parse_int(s):
    try:
        return int(s, 0)
    except (ValueError, TypeError):
        return None

def exp_ret_class(e):
    if e in OPAQUE_RET:
        return 'OPAQUE'
    if e == 'NULL':
        return 0
    if e == '-':
        return '-'
    v = parse_int(e)
    return v if v is not None else e

def obs_ret_class(rtype, r):
    if r in ('-', ''):
        return '-'
    v = parse_int(r)
    if v is None:
        return r
    if rtype == 'q' and 0x10000 <= v < (1 << 64) - 2:
        return 'OPAQUE'
    return v & 0xffffffff if rtype == 'd' else v

FIELD_RE = re.compile(r'([A-Za-z_][\w.\-]*)=("(?:[^"\\]|\\.)*"|\([^)]*\)|\[[^\]]*\]|\S+)')
def fields(text):
    out = {}
    text = unesc(text)
    # NAME{ ... } -> prefix the inner keys
    for m in re.finditer(r'([A-Z][A-Za-z_]+)\{([^{}]*)\}', text):
        pre, body = m.group(1), m.group(2)
        if '=' not in body:
            out[pre] = body.replace(' ', '')
            continue
        for k, v in FIELD_RE.findall(body):
            out[pre + '.' + k] = v
    rest = re.sub(r'([A-Z][A-Za-z_]+)\{([^{}]*)\}', ' ', text)
    for m in re.finditer(r'\b(RECT|POINT)(\([^)]*\))', rest):
        out[m.group(1)] = m.group(2).replace(' ', '')
    rest = re.sub(r'\b(RECT|POINT)(\([^)]*\))', ' ', rest)
    for k, v in FIELD_RE.findall(rest):
        out[k] = v
    for k in list(out):
        v = out[k]
        n = parse_int(v)
        if n is not None:
            out[k] = n
        elif isinstance(v, str) and v.startswith('"'):
            out[k] = v
    return out

# the trace decoder (fleanalyze.py) printed byte 25 of LOGFONTW as 'quality'; byte 25 is lfClipPrecision (lfQuality is byte 26;
# found by layout_check.c). The trace side is renamed so that it is compared with the observed 'clip'.
TRACE_ALIAS = {'LOGFONTW.quality': 'LOGFONTW.clip'}

def out_verdict(api, trace_outs, obs_dec):
    """('match'|'mismatch'|'reference'|'none', detail)"""
    if not trace_outs:
        return 'none', ''
    if not obs_dec:
        return 'none', 'no observed output'
    od = fields(obs_dec)
    best = None
    for text, cnt in trace_outs:
        td = {TRACE_ALIAS.get(k, k): v for k, v in fields(text).items()}
        keys = [k for k in td if k in od]
        if not keys:
            continue
        diff = [(k, td[k], od[k]) for k in keys if td[k] != od[k] and not (isinstance(od[k], str) and '<' in od[k])]
        spec = [x for x in diff if x[0] not in REF_FIELDS]
        ref = [x for x in diff if x[0] in REF_FIELDS]
        if not diff:
            return 'match', ''
        cand = ('mismatch' if spec else 'reference', '; '.join('%s: runtime=%s real=%s' % (k, a, b) for k, a, b in (spec or ref)[:4]))
        if best is None:
            best = cand
    return best or ('none', 'no comparable fields')

def le_verdict(trace_le, changed, val, ret_is_fail):
    if not trace_le:
        return None
    for e, cnt in trace_le:
        if e.startswith('same:') and not changed:
            return True
        if e.startswith('set:') and changed and parse_int(e[4:]) == val:
            return True
        if e.startswith('same:') and changed and parse_int(e[5:]) == val:
            return True
    return False

def fmt_le(trace_le):
    return ','.join('%s×%d' % (e.replace('same:', 'unchanged(').replace('set:', 'set ') + (')' if e.startswith('same:') else ''), c) for e, c in trace_le[:3])

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--cases', default=os.path.join(HERE, 'cases.tsv'))
    ap.add_argument('--observed', default='observed.tsv')
    ap.add_argument('--observed-manifest', default=None, help='second run with a Windows 10 compatibility manifest (optional)')
    ap.add_argument('--report', default='compare-report.md')
    ap.add_argument('--top', type=int, default=20)
    args = ap.parse_args()

    _, cases = load_tsv(args.cases)
    meta, obs_rows = load_tsv(args.observed)
    obs = {r['case_id']: r for r in obs_rows}
    meta2, obs2 = (None, {})
    if args.observed_manifest:
        meta2, rows2 = load_tsv(args.observed_manifest)
        obs2 = {r['case_id']: r for r in rows2}

    V = collections.Counter(); per = collections.defaultdict(collections.Counter); mism = []; refs = []; real_only = collections.Counter()
    status_c = collections.Counter(); le_info = collections.Counter(); notrun = 0
    rows_out = []
    for c in cases:
        cid, api = c['case_id'], c['api']
        o = obs.get(cid)
        if o is None:
            notrun += 1; per[api]['not-run'] += 1; continue
        st = o.get('status', '')
        status_c[st.split(':')[0]] += 1
        if st != 'ok':
            v = 'skipped'
            per[api][v] += 1; V[v] += 1
            rows_out.append((cid, api, v, st, c, o))
            continue
        tn = int(c['trace_n'] or 0)
        if tn == 0 or api in HARNESS_APIS:
            per[api]['real-only'] += 1; V['real-only'] += 1
            continue
        trace_ret = json.loads(c['trace_ret']) if c['trace_ret'] else []
        trace_le = json.loads(c['trace_le']) if c['trace_le'] else []
        trace_out = json.loads(c['trace_out']) if c['trace_out'] else []
        is_ref_api = c['cls'] in ('R', 'L') or locale_dependent(c)
        problems = []; refprob = []
        rcls = obs_ret_class(c['rtype'], o['ret'])
        if c['rtype'] != 'v' and trace_ret:
            exp = {exp_ret_class(e) for e, _ in trace_ret}
            if api == 'GetVersion':
                ok = any(isinstance(e, int) and (e & 0xffff) == (rcls & 0xffff) for e in exp)
                if ok and rcls not in exp:
                    refprob.append('GetVersion build/word differs (runtime=%s real=%s)' % ('/'.join(hex(e) for e in exp if isinstance(e, int)), hex(rcls)))
            else:
                ok = rcls in exp
            if not ok:
                (refprob if is_ref_api else problems).append('return: runtime=%s real=%s' % ('/'.join(str(e) if not isinstance(e, int) else hex(e) for e in sorted(exp, key=str)), rcls if not isinstance(rcls, int) else hex(rcls)))
        ret_fail = rcls in (0, 0xffffffff, (1 << 64) - 1)
        lev = le_verdict(trace_le, o['le_changed'] == '1', parse_int(o['last_error']), ret_fail)
        if lev is False:
            msg = 'last error: runtime=%s real=%s' % (fmt_le(trace_le), ('set %s' % o['last_error']) if o['le_changed'] == '1' else 'unchanged')
            if ret_fail:
                (refprob if is_ref_api else problems).append(msg)
            else:
                le_info[api] += 1
        ov, od = out_verdict(api, trace_out, o.get('decoded', ''))
        if ov == 'mismatch':
            (refprob if is_ref_api else problems).append('output: ' + od)
        elif ov == 'reference':
            refprob.append('output: ' + od)
        if problems:
            v = 'mismatch'
        elif refprob:
            v = 'reference'
        else:
            v = 'match'
        per[api][v] += 1; V[v] += 1
        rows_out.append((cid, api, v, '; '.join(problems + refprob), c, o))
        if v == 'mismatch':
            mism.append((tn, cid, api, c, o, problems))
        elif v == 'reference':
            refs.append((tn, cid, api, c, o, refprob))

    # manifest difference
    mdiff = collections.defaultdict(list)
    if obs2:
        VARY = set('GetTickCount QueryPerformanceCounter GetSystemTimeAsFileTime GetLocalTime GetCurrentThreadId GetCurrentProcessId GetCommandLineW GetCommandLineA GetModuleFileNameW GetModuleFileNameA GetStartupInfoW GetStartupInfoA GetEnvironmentStringsW'.split())
        rt = {c['case_id']: c['rtype'] for c in cases}
        def sig(o, cid):
            return (o.get('status'), obs_ret_class(rt.get(cid, 'q'), o.get('ret', '')), o.get('decoded', ''), o.get('last_error') if o.get('le_changed') == '1' else 'unchanged')
        for cid, o in obs.items():
            o2 = obs2.get(cid)
            if o2 is None or o.get('api') in VARY:
                continue
            if sig(o, cid) != sig(o2, cid):
                mdiff[o.get('api')].append((cid, o, o2))

    api_cls = collections.defaultdict(str)
    case_by = {c['case_id']: c for c in cases}
    L = []
    w = L.append
    w('# compare-report')
    w('')
    w('observed: `%s`, cases: `%s`' % (os.path.basename(args.observed), os.path.basename(args.cases)))
    w('')
    w('## Environment of the observed run')
    for k in ('replay-version', 'tag', 'utc', 'compiler', 'RtlGetVersion', 'native-arch', 'ACP', 'OEMCP', 'mask-private'):
        if k in meta:
            w('- %s: %s' % (k, meta[k]))
    w('')
    ntr = sum(1 for c in cases if int(c['trace_n'] or 0) > 0)
    w('## Totals')
    w('')
    w('- cases in cases.tsv: %d (run: %d, not run: %d); with a runtime answer to compare (trace_n>0): %d' % (len(cases), len(cases) - notrun, notrun, ntr))
    w('- row status: ' + ', '.join('%s=%d' % kv for kv in sorted(status_c.items())))
    w('- verdicts: ' + ', '.join('%s=%d' % (k, V[k]) for k in ('match', 'mismatch', 'reference', 'real-only', 'skipped')))
    w('- match / mismatch / reference are counted per case; one case stands for one distinct argument shape (not for every call). trace calls weigh in the ranking below')
    w('')
    w('## Per API')
    w('')
    w('| API | class | match | mismatch | reference | real-only | skipped |')
    w('|---|---|--:|--:|--:|--:|--:|')
    clsof = {c['api']: c['cls'] for c in cases}
    for a_ in sorted(per):
        p = per[a_]
        w('| %s | %s | %d | %d | %d | %d | %d |' % (a_, clsof.get(a_, ''), p['match'], p['mismatch'], p['reference'], p['real-only'], p['skipped']))
    w('')
    w('## Top %d mismatches (spec fields; ranked by the number of calls the trace made with this argument shape)' % args.top)
    w('')
    w('| # | calls | API | args | difference |')
    w('|--:|--:|---|---|---|')
    for i, (tn, cid, api, c, o, pr) in enumerate(sorted(mism, key=lambda x: -x[0])[:args.top], 1):
        w('| %d | %d | %s `%s` | `%s` | %s |' % (i, tn, api, cid, c['args'][:90].replace('|', '/'), '; '.join(pr)[:300].replace('|', '/')))
    w('')
    w('## All mismatches by API')
    w('')
    by_api = collections.defaultdict(list)
    for tn, cid, api, c, o, pr in mism:
        by_api[api].append((tn, cid, c, pr))
    for api in sorted(by_api, key=lambda a: -sum(x[0] for x in by_api[a])):
        lst = by_api[api]
        w('### %s: %d cases, %d trace calls' % (api, len(lst), sum(x[0] for x in lst)))
        w('')
        for tn, cid, c, pr in sorted(lst, key=lambda x: -x[0])[:40]:
            w('- `%s` ×%d `%s` — %s' % (cid, tn, c['args'][:100].replace('`', "'"), '; '.join(pr)[:300]))
        if len(lst) > 40:
            w('- ... %d more' % (len(lst) - 40))
        w('')
    w('## Reference differences (machine / user / time / screen dependent; not spec facts)')
    w('')
    ref_by = collections.defaultdict(list)
    for tn, cid, api, c, o, pr in refs:
        ref_by[api].append((tn, cid, c, pr))
    for api in sorted(ref_by):
        lst = ref_by[api]
        w('- **%s** (%d cases, %d trace calls): %s' % (api, len(lst), sum(x[0] for x in lst), '; '.join(lst[0][3])[:260]))
    w('')
    w('## Last error on the success path (informational; Microsoft documents no last-error value after success)')
    w('')
    w(', '.join('%s=%d' % kv for kv in sorted(le_info.items(), key=lambda x: -x[1])) or '(none)')
    w('')
    if obs2:
        w('## Effect of a Windows 10 compatibility manifest (second run)')
        w('')
        w('observed-manifest tag: %s; RtlGetVersion: %s' % (meta2.get('tag'), meta2.get('RtlGetVersion')))
        w('')
        if not mdiff:
            w('No case differs between the two runs.')
        for api in sorted(mdiff):
            w('- **%s**: %d cases differ, e.g. `%s`: ret %s -> %s; decoded `%s` -> `%s`' % (api, len(mdiff[api]), mdiff[api][0][0], mdiff[api][0][1].get('ret'), mdiff[api][0][2].get('ret'),
                                                                                           mdiff[api][0][1].get('decoded', '')[:80], mdiff[api][0][2].get('decoded', '')[:80]))
        w('')
    w('## Cases the tool skipped or Windows could not run')
    w('')
    sk = collections.Counter((r[1], r[3]) for r in rows_out if r[2] == 'skipped')
    for (api, st), n in sorted(sk.items()):
        w('- %s: %s ×%d' % (api, st, n))
    w('')
    open(args.report, 'w', encoding='utf-8').write('\n'.join(L) + '\n')
    print('verdicts: ' + ', '.join('%s=%d' % (k, V[k]) for k in ('match', 'mismatch', 'reference', 'real-only', 'skipped')))
    print('not run:', notrun)
    print('top mismatches:')
    for i, (tn, cid, api, c, o, pr) in enumerate(sorted(mism, key=lambda x: -x[0])[:args.top], 1):
        print('%2d. %s %s x%d %s' % (i, cid, api, tn, '; '.join(pr)[:140]))
    print('report written to', args.report)

if __name__ == '__main__':
    main()
