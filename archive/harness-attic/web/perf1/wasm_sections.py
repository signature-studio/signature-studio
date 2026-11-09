"""perf1 (decision 0130, PROVISIONAL): a small reader of a .wasm file that prints where the bytes are. Standard library only; reads the file with mmap (read only).
usage: wasm_sections.py FILE.wasm [--json OUT.json] [--top N]
Prints: every section (id, name, bytes, % of the file), the counts (types, imports, functions, tables, globals, exports, elements, data segments), the code section (number of functions, total body bytes,
the largest bodies, a histogram of body sizes), the data section (number of segments, active/passive, total bytes, the largest segments with their memory offset, how many bytes are all zero), the
`name` custom section (function-name bytes). `compressed` figures are for the whole file only (see tools/compress.mjs): per-section compression is NOT done here."""
import json
import mmap
import sys

SEC = {0: 'custom', 1: 'type', 2: 'import', 3: 'function', 4: 'table', 5: 'memory', 6: 'global', 7: 'export', 8: 'start', 9: 'element', 10: 'code', 11: 'data', 12: 'datacount', 13: 'tag'}


def uleb(d, p):
    r = 0
    s = 0
    while True:
        b = d[p]
        p += 1
        r |= (b & 0x7F) << s
        s += 7
        if not b & 0x80:
            return r, p


def sleb(d, p):
    r = 0
    s = 0
    while True:
        b = d[p]
        p += 1
        r |= (b & 0x7F) << s
        s += 7
        if not b & 0x80:
            if b & 0x40:
                r -= 1 << s
            return r, p


def const_expr(d, p):
    """returns (value or None, end) of a constant expression ending with 0x0b (i32.const/i64.const/global.get only)"""
    val = None
    while True:
        op = d[p]
        p += 1
        if op == 0x0B:
            return val, p
        if op == 0x41:
            val, p = sleb(d, p)
        elif op == 0x42:
            val, p = sleb(d, p)
        elif op == 0x23:
            _, p = uleb(d, p)
        else:
            raise ValueError('constant expression opcode %#x' % op)


def main():
    path = sys.argv[1]
    top = 12
    out_json = None
    a = sys.argv[2:]
    while a:
        if a[0] == '--json':
            out_json = a[1]
            a = a[2:]
        elif a[0] == '--top':
            top = int(a[1])
            a = a[2:]
        else:
            raise SystemExit('bad argument ' + a[0])
    f = open(path, 'rb')
    d = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
    total = len(d)
    assert d[:4] == b'\0asm', 'not a wasm file'
    p = 8
    secs = []
    info = {'file': path, 'bytes': total}
    while p < total:
        sid = d[p]
        p += 1
        sz, p = uleb(d, p)
        name = SEC.get(sid, '?%d' % sid)
        start = p
        if sid == 0:
            nl, q = uleb(d, p)
            name = 'custom:' + bytes(d[q:q + nl]).decode('utf8', 'replace')
        secs.append({'id': sid, 'name': name, 'offset': start, 'bytes': sz})
        p += sz
    info['sections'] = secs
    counts = {}
    for s in secs:
        o = s['offset']
        if s['id'] in (1, 2, 3, 4, 6, 7, 9, 11, 13):
            counts[s['name']], _ = uleb(d, o)
    info['counts'] = counts
    for s in secs:
        if s['id'] == 10:
            o = s['offset']
            n, q = uleb(d, o)
            sizes = []
            local_decl_bytes = 0
            for i in range(n):
                l, q2 = uleb(d, q)
                nl, r = uleb(d, q2)
                for g in range(nl):
                    c, r = uleb(d, r)
                    r += 1
                local_decl_bytes += r - q2
                sizes.append(l)
                q = q2 + l
            sizes_sorted = sorted(sizes, reverse=True)
            hist = {}
            for z in sizes:
                k = 0 if z < 100 else 1 if z < 1000 else 2 if z < 10000 else 3 if z < 100000 else 4 if z < 1000000 else 5
                hist[k] = hist.get(k, 0) + z
            labels = ['<100 B', '100 B-1 kB', '1-10 kB', '10-100 kB', '100 kB-1 MB', '>=1 MB']
            cnt = {}
            for z in sizes:
                k = 0 if z < 100 else 1 if z < 1000 else 2 if z < 10000 else 3 if z < 100000 else 4 if z < 1000000 else 5
                cnt[k] = cnt.get(k, 0) + 1
            info['code'] = {'functions': n, 'body_bytes': sum(sizes), 'local_decl_bytes': local_decl_bytes, 'largest_bodies': sizes_sorted[:top],
                            'median_body': sorted(sizes)[n // 2] if n else 0,
                            'histogram': [{'range': labels[k], 'functions': cnt.get(k, 0), 'bytes': hist.get(k, 0)} for k in range(6)]}
        if s['id'] == 11:
            o = s['offset']
            n, q = uleb(d, o)
            segs = []
            zero_bytes = 0
            for i in range(n):
                flag, q = uleb(d, q)
                off = None
                if flag == 0:
                    off, q = const_expr(d, q)
                elif flag == 2:
                    _, q = uleb(d, q)
                    off, q = const_expr(d, q)
                elif flag != 1:
                    raise ValueError('data segment flag %d' % flag)
                l, q = uleb(d, q)
                seg = bytes(d[q:q + l]) if l <= (64 << 20) else None
                z = None
                if seg is not None:
                    z = len(seg) - len(seg.lstrip(b'\0')) if False else seg.count(0)
                    zero_bytes += z
                segs.append({'flag': flag, 'offset': off, 'bytes': l, 'zero_bytes': z})
                q += l
            segs_sorted = sorted(segs, key=lambda x: -x['bytes'])
            info['data'] = {'segments': n, 'active': sum(1 for x in segs if x['flag'] != 1), 'passive': sum(1 for x in segs if x['flag'] == 1), 'bytes': sum(x['bytes'] for x in segs),
                            'zero_bytes_in_segments_up_to_64MiB': zero_bytes, 'largest': segs_sorted[:top],
                            'lowest_offset': min((x['offset'] for x in segs if x['offset'] is not None), default=None),
                            'highest_end': max((x['offset'] + x['bytes'] for x in segs if x['offset'] is not None), default=None)}
        if s['name'] == 'custom:name':
            info['name_section_bytes'] = s['bytes']
    # a short report
    print('%s  %d bytes' % (path, total))
    for s in secs:
        print('  section %-3d %-22s %12d B  %6.2f %%' % (s['id'], s['name'], s['bytes'], 100.0 * s['bytes'] / total))
    print('  counts:', counts)
    if 'code' in info:
        c = info['code']
        print('  code: %d functions, bodies %d B (local declarations %d B), median body %d B, largest %s' % (c['functions'], c['body_bytes'], c['local_decl_bytes'], c['median_body'], c['largest_bodies'][:6]))
        for h in c['histogram']:
            print('     bodies %-12s %8d functions %12d B' % (h['range'], h['functions'], h['bytes']))
    if 'data' in info:
        x = info['data']
        print('  data: %d segments (%d active, %d passive), %d B, zero bytes in segments <= 64 MiB: %d, memory offsets %s .. %s' % (x['segments'], x['active'], x['passive'], x['bytes'], x['zero_bytes_in_segments_up_to_64MiB'], x['lowest_offset'], x['highest_end']))
        for g in x['largest'][:top]:
            print('     segment offset %s  %d B  (zero bytes %s)' % (g['offset'], g['bytes'], g['zero_bytes']))
    if out_json:
        json.dump(info, open(out_json, 'w'), indent=1)


main()
