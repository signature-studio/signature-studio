#!/usr/bin/env python3
# 調査専用(製品と本番の処理には使わない。AGENTS.md の Python の決まり)。Issue #1521 の読み。
# FLEngine_x64.dll の中の Delphi の DFM(二値)と RTTI を読む。出力は pm-work/findings/manage-files-1521-*.txt に貼った。
#   python3 pm-work/findings/manage-files-1521-dfm-read.py DLL dfm FILE_OFFSET   # TPF0 の位置から、オブジェクトの木を出す
#   python3 pm-work/findings/manage-files-1521-dfm-read.py DLL rtti VA           # TTypeInfo(tkClass)の公開プロパティを出す
#   python3 pm-work/findings/manage-files-1521-dfm-read.py DLL find NAME          # 'NAME' の TypeInfo の位置(\x07 + 長さ + 名前)
import struct
import sys

d = open(sys.argv[1], 'rb').read()
pe = struct.unpack_from('<I', d, 0x3c)[0]
nsec = struct.unpack_from('<H', d, pe + 6)[0]
optsz = struct.unpack_from('<H', d, pe + 20)[0]
imgbase = struct.unpack_from('<Q', d, pe + 24 + 24)[0]
secs = []
for i in range(nsec):
    o = pe + 24 + optsz + 40 * i
    name = d[o:o + 8].rstrip(b'\0').decode()
    vsz, va, rsz, raw = struct.unpack_from('<IIII', d, o + 8)
    secs.append((name, va, vsz, raw, rsz))


def va2off(va):
    for _, sva, vsz, raw, rsz in secs:
        if sva <= va - imgbase < sva + max(vsz, rsz):
            return raw + (va - imgbase - sva)


def off2va(off):
    for _, sva, vsz, raw, rsz in secs:
        if raw <= off < raw + rsz:
            return imgbase + sva + (off - raw)


def ss(off):
    n = d[off]
    return d[off + 1:off + 1 + n].decode('latin-1')


# DFM: the values of the TPF0 stream (Delphi's TValueType). Only what the menus use is decoded; others stop the dump.
def dfm_value(p):
    t = d[p]
    p += 1
    if t == 0x00 or t == 0x0D:
        return None, p
    if t == 0x02:
        return d[p], p + 1
    if t == 0x03:
        return struct.unpack_from('<h', d, p)[0], p + 2
    if t == 0x04:
        return struct.unpack_from('<i', d, p)[0], p + 4
    if t == 0x06:
        return ss(p), p + 1 + d[p]
    if t == 0x07:
        return ('ident', ss(p)), p + 1 + d[p]
    if t == 0x0B:
        items = []
        while d[p] != 0:
            items.append(ss(p))
            p += 1 + d[p]
        return ('set', items), p + 1
    if t == 0x08:
        return False, p
    if t == 0x09:
        return True, p
    if t in (0x0C, 0x14):
        n = struct.unpack_from('<I', d, p)[0]
        return d[p + 4:p + 4 + n].decode('latin-1'), p + 4 + n
    if t == 0x12:
        n = struct.unpack_from('<I', d, p)[0]
        return d[p + 4:p + 4 + 2 * n].decode('utf-16-le'), p + 4 + 2 * n
    raise SystemExit(f'dfm: value type 0x{t:02x} is not decoded (at {p - 1})')


def dfm_obj(p, depth, out):
    cls = ss(p)
    p += 1 + len(cls)
    name = ss(p)
    p += 1 + len(name)
    props = {}
    while d[p] != 0:
        pn = ss(p)
        p += 1 + len(pn)
        props[pn], p = dfm_value(p)
    p += 1
    extra = [f'{k}={props[k]!r}' for k in ('Caption', 'Action', 'OnClick', 'OnExecute', 'OnUpdate', 'Visible', 'Tag', 'GroupIndex') if k in props]
    out.append('  ' * depth + f'{name}: {cls}' + (f'  [{", ".join(extra)}]' if extra else ''))
    while d[p] != 0:
        p = dfm_obj(p, depth + 1, out)
    return p + 1


def props(tva):
    o = va2off(tva)
    name = ss(o + 1)
    p = o + 2 + len(name)
    classtype, parent = struct.unpack_from('<QQ', d, p)
    pc = struct.unpack_from('<H', d, p + 16)[0]
    unit = ss(p + 18)
    p = p + 19 + len(unit) + 2
    print(f'typeinfo {name}: vmt 0x{classtype:x} parent-typeinfo-slot 0x{parent:x} propcount {pc}')
    for _ in range(pc):
        pt, gp, sp, stp = struct.unpack_from('<QQQQ', d, p)
        idx = struct.unpack_from('<i', d, p + 32)[0]
        pn = ss(p + 42)
        print(f'  {pn}: get 0x{gp:x} set 0x{sp:x} index {idx}')
        p = p + 43 + len(pn)


cmd, arg = sys.argv[2], sys.argv[3]
if cmd == 'dfm':
    out = []
    dfm_obj(int(arg, 0) + 4, 0, out)
    print('\n'.join(out))
elif cmd == 'rtti':
    props(int(arg, 0))
elif cmd == 'find':
    pat = bytes([7, len(arg)]) + arg.encode()
    i = d.find(pat)
    while i >= 0:
        print(f'{arg}: file offset {i} (VA 0x{off2va(i):x})')
        i = d.find(pat, i + 1)
else:
    raise SystemExit('usage: DLL dfm|rtti|find ARG')
