#!/usr/bin/python3
"""Read-only carve of the file chunk of an Inno Setup 6.7.0 installer (setup loader revision 2). Never runs it.

innoextract 1.9 only knows Inno Setup <= 6.0.5, so this does the minimum by hand (stdlib + pefile for checks):
  1. find the loader offset table (ID 'rDlPtS' cd e6 d7 7b 0b 2a, version 2): u64 fields; Offset0 = setup header,
     Offset1 = file data.
  2. file data = 4-byte chunk magic 'zlb\\x1a', 1-byte LZMA2 dictionary property, raw LZMA2 stream (python lzma).
     VitalInstaller.exe: one solid chunk, 87,567,245 B, ends exactly at Offset0.
  3. the files are concatenated in the chunk. The compiler stored the executables with the 'call instruction
     optimisation' filter (x86 E8/E9 operands made absolute). It is undone with the algorithm that the public
     innoextract source documents for Inno Setup >= 5.3.9 (exefilter.hpp, 'inno_exe_decoder_5200' with
     flip_high_byte): 64 KiB blocks, instructions spanning a block are skipped, 24-bit address, high byte flipped
     if bit 23 of the result is set; offset counted from the start of each file.
  4. PE files are located by 'MZ' + 'PE\\0\\0'; the length of each is read from its (decoded) section table and
     certificate-table entry. Verified: for the three signed files the Authenticode SHA-256 recomputed over the
     decoded file equals the digest stored in its own signature (tools/authenticode_check.py).
usage: inno67_carve.py INSTALLER OUTDIR
"""
import lzma
import mmap
import os
import re
import struct
import sys

BS = 0x10000


def read_table(m):
    i = m.find(b'rDlPtS\xcd\xe6\xd7{\x0b*')
    ver = struct.unpack_from('<I', m, i + 12)[0]
    f = struct.unpack_from('<' + 'I' * 14, m, i + 16)
    return dict(table_at=i, version=ver, total=f[0] | f[1] << 32, off_exe=f[2] | f[3] << 32,
                exe_size=f[4], exe_crc=f[5], off0=f[6] | f[7] << 32, off1=f[8] | f[9] << 32)


def decode_chunk(m, off1):
    assert m[off1:off1 + 4] == b'zlb\x1a'
    prop = m[off1 + 4]
    dsz = (2 | (prop & 1)) << (prop // 2 + 11)
    dec = lzma.LZMADecompressor(format=lzma.FORMAT_RAW, filters=[{'id': lzma.FILTER_LZMA2, 'dict_size': dsz}])
    out = bytearray()
    pos = off1 + 5
    while not dec.eof:
        b = m[pos:pos + (1 << 20)]
        if not b:
            break
        out += dec.decompress(b)
        pos += len(b)
    return bytes(out), pos - len(dec.unused_data)


def call_filter_decode(raw):
    out = bytearray(raw)
    n = len(out)
    rx = re.compile(b'[\xe8\xe9]')
    i = 0
    while True:
        mm = rx.search(out, i)
        if not mm:
            break
        j = mm.start()
        if BS - (j % BS) < 5 or j + 5 > n:
            i = j + 1
            continue
        top = out[j + 4]
        if top == 0 or top == 0xFF:
            addr = (j + 5) & 0xFFFFFF
            rel = ((out[j + 1] | out[j + 2] << 8 | out[j + 3] << 16) - addr) & 0xFFFFFFFF
            out[j + 1] = rel & 0xFF
            out[j + 2] = (rel >> 8) & 0xFF
            out[j + 3] = (rel >> 16) & 0xFF
            if rel & 0x800000:
                out[j + 4] = (~top) & 0xFF
        i = j + 5
    return bytes(out)


def pe_length(prefix):
    lf = struct.unpack_from('<I', prefix, 0x3c)[0]
    nsec = struct.unpack_from('<H', prefix, lf + 6)[0]
    optsz = struct.unpack_from('<H', prefix, lf + 20)[0]
    magic = struct.unpack_from('<H', prefix, lf + 24)[0]
    dd = lf + 24 + (112 if magic == 0x20b else 96)
    cert_off, cert_sz = struct.unpack_from('<II', prefix, dd + 32)
    st = lf + 24 + optsz
    end = 0
    for k in range(nsec):
        rs, rp = struct.unpack_from('<II', prefix, st + k * 40 + 16)
        end = max(end, rp + rs)
    return max(end, cert_off + cert_sz)


if __name__ == '__main__':
    path, outdir = sys.argv[1], sys.argv[2]
    f = open(path, 'rb')
    m = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
    t = read_table(m)
    print('loader table', t)
    data, end = decode_chunk(m, t['off1'])
    print('chunk decoded %d B; compressed stream ends at %d (Offset0 = %d)' % (len(data), end, t['off0']))
    os.makedirs(outdir, exist_ok=True)
    starts = []
    for mm in re.finditer(b'MZ', data):
        o = mm.start()
        lf = struct.unpack_from('<I', data, o + 0x3c)[0] if o + 0x40 <= len(data) else 0
        if 0x40 <= lf <= 0x400 and data[o + lf:o + lf + 4] == b'PE\0\0':
            starts.append(o)
    print('PE starts in chunk:', starts)
    for k, o in enumerate(starts):
        ln = pe_length(call_filter_decode(data[o:o + 4096]))
        nxt = starts[k + 1] if k + 1 < len(starts) else None
        print('file %d at %d length %d (next start %s)' % (k, o, ln, nxt))
        d = call_filter_decode(data[o:o + ln])
        with open(os.path.join(outdir, 'carved_%d.bin' % k), 'xb') as w:
            w.write(d)
