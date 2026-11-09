#!/usr/bin/python3
"""Header-only probe of WAV / MP3 / OGG files (no decoding). Standard library only.

usage: audio_probe.py DIR OUT.tsv [channels.tsv]
Duration rules: WAV PCM/float = data bytes / block align / rate; WAV with a 'fact' chunk uses it for compressed tags;
MP3 = walk every frame header (exact frame count; Xing/Info/VBRI header frames are reported, not counted as audio);
OGG = last page granule position (Vorbis: / rate; Opus: (granule - pre-skip) / 48000).
"""
import csv
import os
import struct
import sys

sys.dont_write_bytecode = True

WAV_TAGS = {0x0001: "PCM", 0x0003: "IEEE_FLOAT", 0x0011: "IMA_ADPCM", 0x0002: "MS_ADPCM", 0x0055: "MPEG_LAYER3", 0x674F: "OggVorbis(0x674f)", 0x6750: "OggVorbis(0x6750)",
            0x6751: "OggVorbis(0x6751)", 0xFFFE: "EXTENSIBLE", 0x0006: "ALAW", 0x0007: "MULAW"}

MP3_BR = {
    (1, 1): [0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448], (1, 2): [0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384],
    (1, 3): [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320], (2, 1): [0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256],
    (2, 2): [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160], (2, 3): [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160]}
MP3_SR = {1: [44100, 48000, 32000], 2: [22050, 24000, 16000], 25: [11025, 12000, 8000]}


def probe_wav(b):
    r = {"container": "RIFF/WAVE"}
    if b[:4] != b"RIFF" or b[8:12] != b"WAVE":
        r["error"] = "not RIFF/WAVE"
        return r
    pos = 12
    chunks = []
    fact = None
    data_size = None
    fmt = None
    smpl = None
    while pos + 8 <= len(b):
        cid = b[pos:pos + 4]
        sz = struct.unpack_from("<I", b, pos + 4)[0]
        body = pos + 8
        chunks.append(cid.decode("latin-1"))
        if cid == b"fmt ":
            tag, ch, rate, bps, align, bits = struct.unpack_from("<HHIIHH", b, body)
            sub = None
            valid = None
            if tag == 0xFFFE and sz >= 40:
                valid = struct.unpack_from("<H", b, body + 18)[0]
                sub = struct.unpack_from("<H", b, body + 24)[0]
            fmt = (tag, ch, rate, bps, align, bits, sub, valid, sz)
        elif cid == b"fact":
            fact = struct.unpack_from("<I", b, body)[0]
        elif cid == b"data":
            data_size = min(sz, len(b) - body)
            data_off = body
        elif cid == b"smpl":
            smpl = struct.unpack_from("<I", b, body + 28)[0]
        pos = body + sz + (sz & 1)
    r["chunks"] = ",".join(chunks)
    if fmt is None:
        r["error"] = "no fmt"
        return r
    tag, ch, rate, bps, align, bits, sub, valid, fsz = fmt
    eff = sub if tag == 0xFFFE and sub is not None else tag
    r["codec"] = WAV_TAGS.get(eff, "tag_0x%04x" % eff) + ("(via EXTENSIBLE)" if tag == 0xFFFE else "")
    r["tag"] = "0x%04x" % tag
    r["channels"], r["rate"], r["bits"] = ch, rate, bits
    r["data_bytes"] = data_size
    if eff in (1, 3) and align:
        frames = data_size // align
        r["frames"] = frames
    elif fact is not None:
        r["frames"] = fact
    elif align and bps:
        r["frames"] = int(data_size / bps * rate)
    r["smpl_loops"] = smpl if smpl is not None else ""
    if eff in (0x674F, 0x6750, 0x6751) and data_size:
        r["inner"] = "OggS" if b[data_off:data_off + 4] == b"OggS" else "?%s" % b[data_off:data_off + 4].hex()
    return r


def syncsafe(x):
    return (x[0] << 21) | (x[1] << 14) | (x[2] << 7) | x[3]


def probe_mp3(b):
    r = {"container": "MP3"}
    pos = 0
    if b[:3] == b"ID3":
        pos = 10 + syncsafe(b[6:10])
        r["id3v2_bytes"] = pos
    frames = 0
    first = None
    br_hist = {}
    xing = ""
    n = len(b)
    samples = 0
    while pos + 4 <= n:
        h = struct.unpack_from(">I", b, pos)[0]
        if (h >> 21) != 0x7FF:
            nxt = b.find(b"\xff", pos + 1)
            if nxt < 0:
                break
            pos = nxt
            continue
        vid = (h >> 19) & 3
        lay = (h >> 17) & 3
        if vid == 1 or lay == 0:
            pos += 1
            continue
        ver = {3: 1, 2: 2, 0: 25}[vid]
        layer = {3: 1, 2: 2, 1: 3}[lay]
        bri = (h >> 12) & 15
        sri = (h >> 10) & 3
        pad = (h >> 9) & 1
        mode = (h >> 6) & 3
        if bri in (0, 15) or sri == 3:
            pos += 1
            continue
        vtab = 1 if ver == 1 else 2
        br = MP3_BR[(vtab, layer)][bri] * 1000
        sr = MP3_SR[ver][sri]
        if layer == 1:
            fl = (12 * br // sr + pad) * 4
            spf = 384
        elif layer == 2 or ver == 1:
            fl = 144 * br // sr + pad
            spf = 1152
        else:
            fl = 72 * br // sr + pad
            spf = 576
        if first is None:
            first = (ver, layer, sr, mode, br, spf)
            body = pos + 4 + (32 if ver == 1 and mode != 3 else 17 if ver == 1 else 17 if mode != 3 else 9) + (2 if (h >> 16) & 1 == 0 else 0)
            tagpos = pos + 4 + (2 if (h >> 16) & 1 == 0 else 0) + (32 if (ver == 1 and mode != 3) else 17 if (ver == 1 or mode != 3) else 9)
            if b[tagpos:tagpos + 4] in (b"Xing", b"Info"):
                xing = b[tagpos:tagpos + 4].decode()
                pos += fl
                continue
            if b[pos + 36:pos + 40] == b"VBRI":
                xing = "VBRI"
                pos += fl
                continue
        frames += 1
        samples += spf
        br_hist[br] = br_hist.get(br, 0) + 1
        pos += fl
    if first is None:
        r["error"] = "no mpeg frames"
        return r
    ver, layer, sr, mode, br, spf = first
    r["codec"] = "MPEG-%s Layer %d" % ({1: "1", 2: "2", 25: "2.5"}[ver], layer)
    r["rate"] = sr
    r["channels"] = 1 if mode == 3 else 2
    r["bits"] = ""
    r["frames"] = samples
    r["mpeg_frames"] = frames
    r["bitrate"] = ("CBR %d kbps" % (br // 1000)) if len(br_hist) == 1 else ("VBR %d..%d kbps" % (min(br_hist) // 1000, max(br_hist) // 1000))
    r["xing"] = xing
    r["tail_id3v1"] = "yes" if b[-128:-125] == b"TAG" else ""
    return r


def probe_ogg(b):
    r = {"container": "Ogg"}
    pos = 0
    pages = 0
    last_gp = None
    first_pkt = None
    n = len(b)
    streams = set()
    while pos + 27 <= n and b[pos:pos + 4] == b"OggS":
        nseg = b[pos + 26]
        segs = b[pos + 27:pos + 27 + nseg]
        body = pos + 27 + nseg
        gp = struct.unpack_from("<q", b, pos + 6)[0]
        serial = struct.unpack_from("<I", b, pos + 14)[0]
        streams.add(serial)
        if first_pkt is None:
            first_pkt = b[body:body + 64]
        if gp != -1:
            last_gp = gp
        pages += 1
        pos = body + sum(segs)
    r["pages"] = pages
    r["logical_streams"] = len(streams)
    p = first_pkt or b""
    if p[:7] == b"\x01vorbis":
        ver, ch, rate, bmax, bnom, bmin = struct.unpack_from("<IBIiii", p, 7)
        r.update(codec="Vorbis", channels=ch, rate=rate, bits="", bitrate="nominal %d kbps (max %d min %d)" % (bnom // 1000, bmax // 1000, bmin // 1000))
        r["frames"] = last_gp
    elif p[:8] == b"OpusHead":
        ch = p[9]
        pre = struct.unpack_from("<H", p, 10)[0]
        rate = struct.unpack_from("<I", p, 12)[0]
        r.update(codec="Opus", channels=ch, rate=48000, bits="", bitrate="input rate %d" % rate)
        r["frames"] = (last_gp or 0) - pre
    else:
        r["codec"] = "unknown(%s)" % p[:8].hex()
    return r


def main(argv):
    d, out = argv[1], argv[2]
    use = {}
    if len(argv) > 3:
        for row in csv.DictReader(open(argv[3], encoding="utf-8"), delimiter="\t"):
            if row["sample_basename"]:
                use.setdefault(row["sample_basename"], []).append((row["iid"], row["playlist_items"]))
    rows = []
    for fn in sorted(os.listdir(d)):
        if fn.lower().endswith(".flp"):
            continue
        b = open(os.path.join(d, fn), "rb").read()
        if b[:4] == b"RIFF":
            r = probe_wav(b)
        elif b[:4] == b"OggS":
            r = probe_ogg(b)
        elif b[:3] == b"ID3" or (len(b) > 1 and b[0] == 0xFF and (b[1] & 0xE0) == 0xE0):
            r = probe_mp3(b)
        else:
            r = {"container": "?" + b[:4].hex(), "error": "unknown header"}
        r["file"] = fn
        r["bytes"] = len(b)
        fr, ch, rate = r.get("frames"), r.get("channels"), r.get("rate")
        if fr and ch and rate:
            r["seconds"] = fr / rate
            r["float32_bytes"] = fr * ch * 4
            r["pcm16_bytes"] = fr * ch * 2
        u = use.get(fn, [])
        r["channel_iid"] = ";".join(x[0] for x in u)
        r["playlist_items"] = sum(int(x[1]) for x in u)
        rows.append(r)
    cols = ["file", "bytes", "container", "codec", "tag", "channels", "rate", "bits", "frames", "seconds", "float32_bytes", "pcm16_bytes", "bitrate", "mpeg_frames", "xing",
            "id3v2_bytes", "tail_id3v1", "inner", "smpl_loops", "pages", "logical_streams", "chunks", "channel_iid", "playlist_items", "error"]
    with open(out, "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f, delimiter="\t", lineterminator="\n")
        w.writerow(cols)
        for r in rows:
            w.writerow([("%.3f" % r[c]) if c == "seconds" and c in r else r.get(c, "") for c in cols])
    print(len(rows), "files")


if __name__ == "__main__":
    main(sys.argv)
