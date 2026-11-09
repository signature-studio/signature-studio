#!/usr/bin/python3
"""Stream-extract one file from an NSIS installer (stdlib only, no intermediates).

Fixed version of extract_nsis.py. Differences:
  * the firstheader (28 bytes) is located by signature scan instead of a fixed offset
  * zero padding between the firstheader and the compressed header word is skipped
    (observed: 8 zero bytes in flstudio_win64_26.1.7.5653.exe)
  * the extract-file name is resolved with a verbose --list and exact/suffix matching

usage:
  extract_nsis2.py INSTALLER --list PATTERN
  extract_nsis2.py INSTALLER --hash  NAME_SUFFIX
  extract_nsis2.py INSTALLER --write NAME_SUFFIX OUTPUT   (OUTPUT must not exist)
"""
import hashlib
import lzma
import struct
import sys

EW_EXTRACTFILE = 20
ENTRY_SIZE = 28
MAGIC = b"\xef\xbe\xad\xde" + b"NullsoftInst"


def lzma_raw(props):
    lc = props[0] % 9
    lp = (props[0] // 9) % 5
    pb = props[0] // 45
    dict_size = struct.unpack("<I", props[1:5])[0]
    return lzma.LZMADecompressor(
        format=lzma.FORMAT_RAW,
        filters=[{"id": lzma.FILTER_LZMA1, "dict_size": dict_size, "lc": lc, "lp": lp, "pb": pb}],
    )


def find_firstheader(f):
    f.seek(0)
    pos = 0
    prev = b""
    while True:
        b = f.read(1 << 24)
        if not b:
            raise SystemExit("NSIS firstheader signature not found")
        d = prev + b
        i = d.find(MAGIC)
        if i >= 0:
            return pos - len(prev) + i - 4
        prev = d[-(len(MAGIC) - 1):]
        pos += len(b)


def read_header(f):
    fh_off = find_firstheader(f)
    f.seek(fh_off)
    fh = f.read(28)
    inf_hdr_len, total_len = struct.unpack("<II", fh[20:28])
    pos = fh_off + 28
    skipped = 0
    while True:
        f.seek(pos)
        if f.read(4) != b"\0\0\0\0":
            break
        pos += 4
        skipped += 4
        if skipped > 64:
            raise SystemExit("too much zero padding after firstheader")
    f.seek(pos)
    word = struct.unpack("<I", f.read(4))[0]
    if not word & 0x80000000:
        raise SystemExit("header is not LZMA-compressed (word=%#x)" % word)
    clen = word & 0x7FFFFFFF
    dec = lzma_raw(f.read(5))
    out = bytearray()
    remaining = clen - 5
    while len(out) < inf_hdr_len and remaining > 0:
        chunk = f.read(min(65536, remaining))
        remaining -= len(chunk)
        out += dec.decompress(chunk, max_length=inf_hdr_len - len(out))
    if len(out) != inf_hdr_len:
        raise SystemExit("header inflated to %d, expected %d" % (len(out), inf_hdr_len))
    data_start = pos + 4 + clen
    info = dict(fh_off=fh_off, skipped=skipped, clen=clen, inf_hdr_len=inf_hdr_len,
                total_len=total_len, data_start=data_start)
    return bytes(out), info


def parse_blocks(hdr):
    flags = struct.unpack_from("<I", hdr, 0)[0]
    blocks = [struct.unpack_from("<II", hdr, 4 + 8 * i) for i in range(8)]
    return flags, blocks


def read_string(hdr, strings_off, idx):
    pos = strings_off + idx * 2
    out = []
    while True:
        u = struct.unpack_from("<H", hdr, pos)[0]
        pos += 2
        if u == 0:
            break
        if u == 0xE000:
            nxt = struct.unpack_from("<H", hdr, pos)[0]
            pos += 2
            out.append(chr(nxt))
        elif u in (0xE001, 0xE002, 0xE003):
            pos += 2
            out.append("<%X>" % u)
        else:
            out.append(chr(u))
    return "".join(out)


def entry_size(blocks):
    off, num = blocks[2]
    span = blocks[3][0] - off
    if num == 0 or span % num:
        raise SystemExit("cannot derive entry size: span=%d num=%d" % (span, num))
    return span // num


def extract_files(hdr, blocks):
    strings_off = blocks[3][0]
    off, num = blocks[2]
    es = entry_size(blocks)
    for i in range(num):
        e = struct.unpack_from("<%di" % (es // 4), hdr, off + i * es)
        if e[0] == EW_EXTRACTFILE:
            yield i, read_string(hdr, strings_off, e[2]), e[3] & 0xFFFFFFFF


def stream_file(f, data_start, offset):
    f.seek(data_start + offset)
    word = struct.unpack("<I", f.read(4))[0]
    compressed = bool(word & 0x80000000)
    clen = word & 0x7FFFFFFF
    if not compressed:
        remaining = clen
        while remaining:
            chunk = f.read(min(1 << 20, remaining))
            if not chunk:
                raise SystemExit("unexpected EOF")
            remaining -= len(chunk)
            yield chunk
        return
    dec = lzma_raw(f.read(5))
    remaining = clen - 5
    while remaining > 0 and not dec.eof:
        chunk = f.read(min(1 << 20, remaining))
        if not chunk:
            raise SystemExit("unexpected EOF")
        remaining -= len(chunk)
        data = dec.decompress(chunk)
        if data:
            yield data


def main(argv):
    if len(argv) < 4:
        print(__doc__)
        return 64
    installer, mode, pattern = argv[1], argv[2], argv[3]
    with open(installer, "rb") as f:
        hdr, info = read_header(f)
        flags, blocks = parse_blocks(hdr)
        files = list(extract_files(hdr, blocks))
        print("info:", info, "blocks:", blocks, file=sys.stderr)
        if mode == "--list":
            for i, name, off in files:
                if pattern.lower() in name.lower():
                    print(i, hex(off), name)
            print("extract entries:", len(files))
            return 0
        pl = pattern.lower()
        matches = [x for x in files if x[1].lower().endswith(pl)]
        if len({m[2] for m in matches}) != 1:
            print("ambiguous/no match:", matches)
            return 2
        _, name, off = matches[0]
        if mode == "--hash":
            h = hashlib.sha256()
            n = 0
            for c in stream_file(f, info["data_start"], off):
                h.update(c)
                n += len(c)
            print(name, n, h.hexdigest())
            return 0
        if mode == "--write":
            with open(argv[4], "xb") as out:
                for c in stream_file(f, info["data_start"], off):
                    out.write(c)
            return 0
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
