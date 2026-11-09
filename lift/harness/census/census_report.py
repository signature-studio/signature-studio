#!/usr/bin/env python3
"""Classify and summarize the raw TSV that the census harness writes (FLE_CENSUS=<file>, harness/census/census.c).
Decision 0097: THE CENSUS IS A DIAGNOSTIC. Its numbers are not A4 progress (decisions 0046, 0060): an abandoned unit did not complete.

usage: census_report.py --run LABEL=RAW.tsv [--run LABEL2=RAW2.tsv ...] --out-prefix PREFIX [--sites SITES.tsv] [--top 20]
  The first --run is the main run; the others are only used for the comparison section.
  Writes PREFIX-LABEL.tsv for every run (one row per unit, classified) and PREFIX.md (the summary).
  --sites: "0xVA<TAB>Mnemonic" of the fle_unsupported sites of the generated C (grep of `fle_unsupported(UINT64_C(0x`), used to
           name the instruction of an unsupported-instruction wall that is not an unanswered import.
Only the Python standard library is used. Nothing is overwritten (files are opened with mode 'x').

Categories (the classification is by the message and the signal, not by where the code is):
  a  unanswered import: fle_unsupported at an IAT thunk (the import has no response), a call through an import's pseudo-VA, or a named stop
     whose text says the whole API is "not answered"
  b  unsupported instruction / unresolved jmp / lift semantics: fle_unsupported elsewhere, fle_dispatch_miss to a non-import non-null target,
     fle_jumptab_miss, undefined flag, shift count, x87, MXCSR, ...
  c  LoadLibrary / GetProcAddress / GetModuleHandle / FreeLibrary: the module or procedure name
  d  stopped without a specification: any other named stop (fle_stop of a runtime stub)
  e  memory violation: SIGSEGV, SIGBUS, and a call through a null function pointer (indirect call to an address below 64 KiB)
  f  hang: wall time or function-entry limit of the unit
  g  other signal (SIGILL, SIGFPE)

Secondary damage (HEURISTIC, NOT CERTAIN): a unit that fails AFTER the first abandoned unit and whose message shows a cue that a value which an
earlier unit should have set is missing (a null function pointer call, a null-page access, 'NULL', a 0x0 handle or module) is marked
"secondary?". A unit with no cue is marked "primary" (it may still be a consequence of an earlier abandoned unit: nothing here proves it).
When two runs are given, a unit that is abandoned in the main run and completes in the other is listed: that is a measured dependence
(the runs differ in more than one thing; see the text of the summary)."""
import argparse
import re
import sys
from collections import OrderedDict, defaultdict

CAT_NAME = OrderedDict([
    ('a', '未応答 import'),
    ('b', '未対応命令・未解決 jmp・lift の意味の穴'),
    ('c', 'LoadLibrary／GetProcAddress の名前'),
    ('d', '仕様が無く停止（その他の名前付き停止）'),
    ('e', 'メモリ違反・null 呼び'),
    ('f', 'hang'),
    ('g', 'その他のシグナル'),
])
C_APIS = {'LoadLibraryW', 'LoadLibraryA', 'LoadLibraryExW', 'LoadLibraryExA', 'GetProcAddress', 'GetModuleHandleW', 'GetModuleHandleA',
          'GetModuleHandleExW', 'GetModuleHandleExA', 'FreeLibrary'}
B_PATTERNS = ['read of an undefined flag', 'shift count above operand width', 'bad condition code', 'misaligned 32-byte access',
              'fle: x87:', 'ldmxcsr reserved bits', 'xgetbv at', 'GS:[']
SECONDARY_CUES = [re.compile(p) for p in (r'\bNULL\b', r'\bnull\b', r'\bhModule 0x0\b', r'\bhandle 0x0\b', r'\bis 0\b')]
COLS = ['unit', 'proc', 'state', 'signal', 'fault_addr', 'stack_overflow', 'import', 'top', 'caller1', 'caller2', 'top_guest_return',
        'entries', 'new_fns', 'ms', 'cb_depth_leak', 'message']


def fn(va):
    return 'f_' + va[2:] if va.startswith('0x') else va


def read_raw(path):
    rows, end, counts = [], None, None
    with open(path) as f:
        for line in f:
            line = line.rstrip('\n')
            if not line:
                continue
            if line.startswith('#end\t'):
                p = line.split('\t')
                end = {'how': p[1], 'message': p[2] if len(p) > 2 else '-', 'import': p[3] if len(p) > 3 else '-',
                       'top': p[4] if len(p) > 4 else '-', 'caller1': p[5] if len(p) > 5 else '-', 'caller2': p[6] if len(p) > 6 else '-'}
                continue
            if line.startswith('#counts\t'):
                counts = line.split('\t', 1)[1]
                continue
            if line.startswith('#') or line.startswith('unit\t'):
                continue
            p = line.split('\t')
            if len(p) < len(COLS):
                p += ['-'] * (len(COLS) - len(p))
            r = dict(zip(COLS, p[:len(COLS)]))
            r['unit'] = int(r['unit'])
            rows.append(r)
    rows.sort(key=lambda r: r['unit'])
    return rows, end, counts


def norm_hex(s):
    return re.sub(r'0x[0-9a-fA-F]+', '0x…', s)


def classify_stop(msg, imp, sites):
    """-> (category, wall key, detail) for a named stop (SIGABRT) or the stop outside a unit."""
    m = re.match(r'fle: unsupported instruction at (0x[0-9a-f]+)$', msg)
    if m:
        va = m.group(1)
        if imp != '-':
            return 'a', imp, 'IAT thunk without a response (fle_unsupported at %s)' % va
        mn = sites.get(va, '?')
        return 'b', 'unsupported %s at %s' % (mn, va), 'fle_unsupported'
    m = re.match(r'fle: indirect call to (0x[0-9a-f]+|0) is not a known function entry$', msg)
    if m:
        t = int(m.group(1), 16) if m.group(1) != '0' else 0
        if imp != '-':
            return 'a', imp + ' (called through a pointer)', 'fle_dispatch_miss to an import pseudo-VA'
        if t < 0x10000:
            return 'e', 'call through a null function pointer', 'fle_dispatch_miss to %s' % m.group(1)
        return 'b', 'indirect call to %s (not a known function entry)' % m.group(1), 'fle_dispatch_miss'
    m = re.match(r'fle: jump table at (0x[0-9a-f]+): selector (0x[0-9a-f]+) is outside the table$', msg)
    if m:
        return 'b', 'jump table at %s: selector outside the table' % m.group(1), 'fle_jumptab_miss'
    for p in B_PATTERNS:
        if p in msg:
            return 'b', norm_hex(msg), 'lift semantics'
    m = re.match(r'fle: ([A-Za-z0-9_]+): (.*)$', msg, re.S)
    if m:
        api, why = m.group(1), m.group(2)
        if api in C_APIS:
            mm = re.match(r'module "([^"]+)" is not a virtual module', why)
            if mm:
                return 'c', '%s "%s"' % (api, mm.group(1)), 'fle_stop'
            return 'c', '%s: %s' % (api, norm_hex(why)), 'fle_stop'
        if why.startswith('not answered'):
            return 'a', '%s not answered' % api, 'fle_stop (the API is not answered)'
        return 'd', '%s: %s' % (api, norm_hex(why)), 'fle_stop'
    m = re.match(r'fle: (GetProcAddress|LoadLibrary\w*|GetModuleHandle\w*) (.*)$', msg)
    if m:
        return 'c', '%s %s' % (m.group(1), norm_hex(m.group(2))), 'fle_stop-like'
    if msg.startswith('fle: '):
        return 'd', norm_hex(msg), 'runtime message'
    return 'd', '(abort without a runtime message)' if msg in ('-', '') else norm_hex(msg), 'abort'


def classify(r, sites):
    st = r['state']
    if st == 'stop':
        return classify_stop(r['message'], r['import'], sites)
    if st == 'hang':
        what = 'function-entry limit' if r['signal'] == 'CALLS' else 'wall time limit'
        return 'f', 'hang (%s) in %s' % (what, fn(r['top'])), r['signal']
    if st == 'fault':
        sig = r['signal']
        if sig in ('SIGSEGV', 'SIGBUS'):
            a = int(r['fault_addr'], 16) if r['fault_addr'].startswith('0x') else None
            if r['stack_overflow'] == '1':
                return 'e', 'host stack overflow in %s' % fn(r['top']), sig
            if a is not None and a < 0x10000:
                return 'e', 'null-page access (%s) in %s' % (sig, fn(r['top'])), sig
            return 'e', 'wild address access (%s) in %s' % (sig, fn(r['top'])), sig
        return 'g', '%s in %s' % (sig, fn(r['top'])), sig
    return '-', '-', ''


def mark(rows, sites):
    bad = [r for r in rows if r['state'] in ('stop', 'fault', 'hang')]
    first_bad = min((r['unit'] for r in bad), default=None)
    for r in rows:
        r['cat'], r['wall'], r['detail'] = classify(r, sites)
        r['suspect'] = '-'
        r['reason'] = ''
        if r['state'] not in ('stop', 'fault', 'hang'):
            continue
        r['suspect'] = 'primary'
        if first_bad is None or r['unit'] <= first_bad:
            r['reason'] = 'the first abandoned unit'
            continue
        cues = []
        if r['cat'] == 'e' and ('null' in r['wall']):
            cues.append(r['wall'])
        for c in SECONDARY_CUES:
            if r['state'] == 'stop' and c.search(r['message']):
                cues.append(c.pattern)
        if cues:
            r['suspect'] = 'secondary?'
            r['reason'] = 'after unit %d; cue: %s' % (first_bad, '; '.join(cues))
    return first_bad


def walls(rows):
    """wall -> list of unit rows, ordered by count desc, first unit asc."""
    d = OrderedDict()
    for r in rows:
        if r['state'] in ('stop', 'fault', 'hang'):
            d.setdefault((r['cat'], r['wall']), []).append(r)
    return sorted(d.items(), key=lambda kv: (-len(kv[1]), kv[1][0]['unit']))


def chain(r):
    parts = [fn(r['top'])] if r['top'] != '-' else []
    for k in ('caller1', 'caller2'):
        if r[k] != '-':
            parts.append(fn(r[k]))
    s = ' ← '.join(parts) if parts else '-'
    if r['top_guest_return'] != '-':
        s += ' (ret %s)' % r['top_guest_return']
    return s


def write_tsv(path, rows, end):
    with open(path, 'x') as f:
        f.write('# census (decision 0097): a diagnostic. NOT A4 progress: an abandoned unit did not complete.\n')
        f.write('\t'.join(['unit', 'proc', 'state', 'category', 'wall', 'primary_or_secondary', 'reason', 'entries', 'new_fns', 'ms',
                           'top', 'caller1', 'caller2', 'top_guest_return', 'signal', 'fault_addr', 'message']) + '\n')
        for r in rows:
            f.write('\t'.join(str(x) for x in [r['unit'], r['proc'], r['state'], r['cat'], r['wall'], r['suspect'], r['reason'] or '-',
                                              r['entries'], r['new_fns'], r['ms'], r['top'], r['caller1'], r['caller2'],
                                              r['top_guest_return'], r['signal'], r['fault_addr'], r['message']]) + '\n')
        if end and end['how'] != 'returned from entry':
            c, w, d = classify_stop(end['message'], end['import'], SITES)
            f.write('\t'.join(['post', '-', 'post_stop', c, w, 'n/a (outside the unit table)', end['how'], '-', '-', '-', end['top'],
                               end['caller1'], end['caller2'], '-', '-', '-', end['message']]) + '\n')


def md_table(head, rows):
    out = ['| ' + ' | '.join(head) + ' |', '|' + '|'.join('---' for _ in head) + '|']
    for r in rows:
        out.append('| ' + ' | '.join(str(x).replace('|', '\\|') for x in r) + ' |')
    return out


def main():
    global SITES
    ap = argparse.ArgumentParser()
    ap.add_argument('--run', action='append', required=True)
    ap.add_argument('--out-prefix', required=True)
    ap.add_argument('--sites')
    ap.add_argument('--top', type=int, default=20)
    a = ap.parse_args()
    SITES = {}
    if a.sites:
        with open(a.sites) as f:
            for line in f:
                p = line.rstrip('\n').split('\t')
                if len(p) == 2:
                    SITES[p[0]] = p[1]
    runs = OrderedDict()
    for spec in a.run:
        label, path = spec.split('=', 1)
        rows, end, counts = read_raw(path)
        first_bad = mark(rows, SITES)
        runs[label] = {'rows': rows, 'end': end, 'counts': counts, 'first_bad': first_bad, 'path': path}
        write_tsv('%s-%s.tsv' % (a.out_prefix, label), rows, end)
    L = list(runs)
    main_label = L[0]
    md = []
    md.append('# census の結果（診断。A4 の進捗ではない）')
    md.append('')
    md.append('生成: `census_report.py`。census は決定 0097 の診断ツールで、**数は A4 の進捗ではない**（決定 0046、0060）。放棄した unit は完了していない。')
    md.append('')
    for label in L:
        R = runs[label]
        rows = R['rows']
        cnt = defaultdict(int)
        for r in rows:
            cnt[r['state']] += 1
        md.append('## 内訳: %s' % label)
        md.append('')
        md += md_table(['ok', 'stop', 'fault', 'hang', 'empty', 'noproc', 'not_reached', '計'],
                       [[cnt['ok'], cnt['stop'], cnt['fault'], cnt['hang'], cnt['empty'], cnt['noproc'], cnt['not_reached'], len(rows)]])
        md.append('')
        end = R['end']
        if end:
            if end['how'] == 'returned from entry':
                md.append('- entry は戻った。')
            else:
                c, w, d = classify_stop(end['message'], end['import'], SITES)
                md.append('- InitUnits を通し終えた後、**unit の外**で停止（%s）: `%s`（分類 %s: %s）。top %s ← %s ← %s。' % (
                    end['how'], end['message'], c, w, fn(end['top']), fn(end['caller1']), fn(end['caller2'])))
        md.append('- 最初の放棄 unit: %s' % (R['first_bad'] if R['first_bad'] is not None else 'なし'))
        md.append('')
    for label in L:
        R = runs[label]
        rows = R['rows']
        wl = walls(rows)
        md.append('## 分類別件数（%s）' % label)
        md.append('')
        bycat = OrderedDict((k, [0, 0]) for k in CAT_NAME)
        for (c, w), us in wl:
            bycat[c][0] += len(us)
            bycat[c][1] += 1
        md += md_table(['分類', '名前', '放棄 unit 数', '壁の種類数'], [[k, CAT_NAME[k], v[0], v[1]] for k, v in bycat.items()])
        md.append('')
        md.append('## 上位 %d の壁（%s。件数の多い順、同数は最初の unit 番号順）' % (a.top, label))
        md.append('')
        trs = []
        for i, ((c, w), us) in enumerate(wl[:a.top], 1):
            prim = sum(1 for u in us if u['suspect'] == 'primary')
            sec = sum(1 for u in us if u['suspect'] == 'secondary?')
            flag = '一次' if sec == 0 else ('二次の疑い' if prim == 0 else '一次 %d / 二次の疑い %d' % (prim, sec))
            trs.append([i, c, w, len(us), ','.join(str(u['unit']) for u in us[:6]) + ('…' if len(us) > 6 else ''), chain(us[0]), flag])
        md += md_table(['#', '分類', '壁（正規化したメッセージ）', '件数', 'unit', '代表の呼び出し元（最初の unit）', '一次／二次'], trs)
        md.append('')
        md.append('壁の種類は全部で %d。一次／二次はヒューリスティック（確実でない。下の注）。' % len(wl))
        md.append('')
    if len(L) > 1:
        for other in L[1:]:
            O = runs[other]
            rows = runs[main_label]['rows']
            orows = {r['unit']: r for r in O['rows']}
            md.append('## 比較: %s と %s' % (main_label, other))
            md.append('')
            both = []
            for r in rows:
                o = orows.get(r['unit'])
                if o is None:
                    continue
                bad_r = r['state'] in ('stop', 'fault', 'hang')
                bad_o = o['state'] in ('stop', 'fault', 'hang')
                if not (bad_r or bad_o):
                    continue
                if bad_r and bad_o and r['cat'] == o['cat'] and r['wall'] == o['wall']:
                    rel = '同じ壁'
                elif not bad_r:
                    rel = '%s では ok、%s では放棄（%s）' % (main_label, other, o['wall'])
                elif not bad_o:
                    rel = '%s では放棄（%s）、%s では ok' % (main_label, r['wall'], other)
                else:
                    rel = '壁が違う（%s: %s／%s: %s）' % (main_label, r['wall'], other, o['wall'])
                both.append([r['unit'], rel])
            same_n = sum(1 for _, rel in both if rel == '同じ壁')
            md.append('- 同じ unit で同じ壁: %d 件。違い: %d 件。' % (same_n, len(both) - same_n))
            md.append('')
            md += md_table(['unit', '関係'], [b for b in both if b[1] != '同じ壁'] or [['-', '違いなし']])
            md.append('')
            md.append('- **測定した依存**: 一方の構成でだけ放棄される unit は、その unit 自身の壁ではなく、**構成の違い**（実在モジュールの有無と、それに伴う前の unit の完了）に'
                      '依存した失敗である（二次被害の実測の証拠）。ただし 2 つの実行は実在モジュールの有無だけでなく前の unit の完了も変えるので、'
                      'どの変化が原因かはこの比較だけでは切り分けていない。')
            md.append('')
    md.append('## 二次被害のヒューリスティック（確実でない）')
    md.append('')
    for label in L:
        sec = [r for r in runs[label]['rows'] if r['suspect'] == 'secondary?']
        if sec:
            md.append('- %s:' % label)
            md.append('')
            md += md_table(['unit', '壁', '理由'], [[r['unit'], r['wall'], r['reason']] for r in sec])
            md.append('')
        else:
            md.append('- %s: 該当なし（最初の放棄 unit より後で、null 呼び・null 参照・NULL／0x0 ハンドルの手掛かりがある停止は無かった）。' % label)
    md.append('')
    md.append('定義: 最初に放棄された unit より後に失敗し、メッセージに「前の unit が設定すべき値が無い」手掛かり（null 関数ポインタ呼び、null ページへのアクセス、'
              '`NULL`、`hModule 0x0`、`is 0`）がある unit に印を付ける。手掛かりが無い unit は「一次」と書くが、それは**前の放棄の結果ではないことの証明ではない**'
              '（放棄した unit が書くはずだったメモリ・ランタイムの表は戻していない）。')
    md.append('')
    with open(a.out_prefix + '.md', 'x') as f:
        f.write('\n'.join(md) + '\n')
    print('wrote %s.md and %d tsv' % (a.out_prefix, len(runs)))


if __name__ == '__main__':
    main()
