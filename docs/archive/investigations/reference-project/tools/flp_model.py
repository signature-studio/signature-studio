#!/usr/bin/python3
"""Own FLP (FL Studio project) reader and model builder. Standard library only.

STATIC ANALYSIS ONLY. Nothing here runs FL Studio or the lifted engine.

Container / event framing (public sources, fetched 2026-11-03, see reference-project.md section 1):
  * https://pyflp.readthedocs.io/en/latest/architecture/flp-format.html  ('FLhd' u32=6 {u16 format, u16 channels, u16 ppq};
    'FLdt' u32 size; event ids 0-63 -> 1 byte, 64-127 -> 2, 128-191 -> 4, 192-255 -> varint length + data)
  * PyFLP sources (GPL-3.0, third party RE, not Image-Line documentation): _events/channel/plugin/mixer/pattern/arrangement/project .py
  * github.com/<OWNER-HANDLE>/flp-extract-fxp (the client's earlier project; docs/flp-serum2-conversion.md, src/flp.rs):
    event 213 = PluginParams = u32 version + records [u32 cid][u64 size][data]; cid 53 state, 54 name, 55 file, 56 vendor.

Empirical rule found on this machine's files (not in the public sources): in FL 25.2/26.1 files event id 172 carries a
3-byte value (with the generic 4-byte rule 38 spurious 1-byte events with values > 1 appear and the stream only
re-synchronises by luck; with 3 bytes none appear). Everything else follows the generic rule. See reference-project.md.
"""
import struct
import sys

sys.dont_write_bytecode = True

SIZE_OVERRIDE = {172: 3}

NAMES = {
    0: "ChanEnabled", 9: "LoopActive", 10: "ShowInfo", 11: "RackSwing", 12: "ProjVolume", 20: "PingPong", 21: "ChanType", 22: "RoutedTo",
    23: "PanLaw", 26: "PatLooped", 28: "Licensed", 30: "PlayTrunc", 32: "ChanLocked",
    64: "NewChan", 65: "NewPat", 67: "CurPat", 80: "Pitch", 98: "SlotIndex", 99: "NewArrangement", 100: "CurArrangement",
    128: "Color", 129: "PlayListItem", 147: "InsertOutput", 149: "InsertColor", 154: "InsertInput", 156: "Tempo", 159: "FLBuild",
    192: "ChanName0", 193: "PatName", 194: "Title", 195: "Comments", 196: "SamplePath", 199: "FLVersion", 200: "Licensee",
    201: "PluginInternalName", 202: "DataPath", 203: "PluginName", 204: "InsertName", 206: "Genre", 207: "Artists",
    209: "Delay", 212: "PluginWrapper", 213: "PluginData", 215: "ChanParams", 218: "EnvLfo", 219: "Levels", 221: "Polyphony",
    224: "PatNotes", 225: "MixerParams", 226: "LevelAdjusts", 227: "Tracking", 229: "PLSelection", 233: "Playlist",
    234: "AutomationPoints", 235: "InsertRouting", 236: "InsertFlags", 237: "Timestamp", 238: "TrackData", 241: "ArrName",
}

CHANNEL_TYPES = {0: "Sampler", 2: "Native", 3: "Layer", 4: "Instrument", 5: "Automation"}


class Ev:
    __slots__ = ("i", "off", "id", "d")

    def __init__(self, i, off, eid, d):
        self.i, self.off, self.id, self.d = i, off, eid, d


def read_events(buf):
    if buf[:4] != b"FLhd":
        raise ValueError("no FLhd")
    hlen, fmt, nch, ppq = struct.unpack_from("<IHHH", buf, 4)
    pos = 8 + hlen
    if buf[pos:pos + 4] != b"FLdt":
        raise ValueError("no FLdt")
    dlen = struct.unpack_from("<I", buf, pos + 4)[0]
    pos += 8
    end = pos + dlen
    evs = []
    while pos < end:
        off = pos
        eid = buf[pos]
        pos += 1
        if eid in SIZE_OVERRIDE:
            n = SIZE_OVERRIDE[eid]
        elif eid < 64:
            n = 1
        elif eid < 128:
            n = 2
        elif eid < 192:
            n = 4
        else:
            n = 0
            sh = 0
            while True:
                b = buf[pos]
                pos += 1
                n |= (b & 0x7F) << sh
                sh += 7
                if not b & 0x80:
                    break
        evs.append(Ev(len(evs), off, eid, buf[pos:pos + n]))
        pos += n
    if pos != end:
        raise ValueError("event stream overruns FLdt: %d != %d" % (pos, end))
    return {"format": fmt, "channels": nch, "ppq": ppq, "fldt_len": dlen, "trailing": len(buf) - end}, evs


def u16(d, o=0):
    return struct.unpack_from("<H", d, o)[0]


def u32(d, o=0):
    return struct.unpack_from("<I", d, o)[0]


def i32(d, o=0):
    return struct.unpack_from("<i", d, o)[0]


def f32(d, o=0):
    return struct.unpack_from("<f", d, o)[0]


def f64(d, o=0):
    return struct.unpack_from("<d", d, o)[0]


def text(d):
    """UTF-16LE (FL 20+ writes UTF-16LE NUL-terminated for the text ids); ASCII for FLVersion and a few old ones."""
    if len(d) >= 2 and len(d) % 2 == 0 and d[-2:] == b"\x00\x00":
        return d.decode("utf-16-le", "replace").rstrip("\x00")
    if len(d) >= 2 and len(d) % 2 == 0 and d[1::2].count(0) == len(d) // 2:
        return d.decode("utf-16-le", "replace").rstrip("\x00")
    return d.decode("latin-1").rstrip("\x00")


def records(d, start=0, hdr=12):
    """[u32 cid][u64 size][data] records. Returns (list of (cid, offset_of_data, size), ok)."""
    out = []
    p = start
    while p + hdr <= len(d):
        cid = u32(d, p)
        sz = struct.unpack_from("<Q", d, p + 4)[0]
        p += hdr
        if sz > len(d) - p:
            return out, False
        out.append((cid, p, sz))
        p += sz
    return out, p == len(d)


def parse_plugin_params(d):
    """Event 213 of a Fruity Wrapper (VST/VST3/CLAP): u32 version + records. Returns dict or None when not parseable."""
    if len(d) < 16:
        return None
    ver = u32(d, 0)
    recs, ok = records(d, 4)
    if not ok or not recs:
        return None
    r = {"version": ver, "records": {}}
    for cid, off, sz in recs:
        r["records"][cid] = (off, sz)
    g = lambda c: d[r["records"][c][0]:r["records"][c][0] + r["records"][c][1]] if c in r["records"] else None
    r["name"] = (g(54) or b"").decode("utf-8", "replace")
    r["file"] = (g(55) or b"").decode("utf-8", "replace")
    r["vendor"] = (g(56) or b"").decode("utf-8", "replace")
    r["uid"] = g(52)
    r["fourcc"] = g(51)
    r["state"] = g(53)
    r["rec_sizes"] = {c: s for c, (o, s) in r["records"].items()}
    return r


def parse_vst3_state(st):
    """cid 53 payload of a VST3 wrapper: u32 prologue + records. Returns dict of inner record sizes and, for XferJson, header facts."""
    if st is None or len(st) < 4:
        return None
    recs, ok = records(st, 4)
    inner = {}
    for cid, off, sz in recs:
        inner.setdefault(cid, []).append((off, sz))
    info = {"prologue": u32(st, 0), "ok": ok, "inner_sizes": {c: [s for o, s in v] for c, v in inner.items()}}
    for cid in (3, 2):
        for off, sz in inner.get(cid, []):
            blob = st[off:off + sz]
            if blob[:9] == b"XferJson\x00":
                jl = struct.unpack_from("<Q", blob, 9)[0]
                js = blob[17:17 + jl].decode("utf-8", "replace")
                un = u32(blob, 17 + jl)
                info["xfer_%s" % ("processor" if cid == 3 else "controller")] = {
                    "json": js, "uncompressed": un, "compressed": sz - (17 + jl + 8),
                    "zstd": blob[17 + jl + 8:17 + jl + 12] == b"\x28\xb5\x2f\xfd"}
    if 4 in inner:
        off, sz = inner[4][0]
        info["param_list_count"] = u32(st, off)
    return info


def parse_playlist(d):
    n88, n60, n32 = len(d) % 88 == 0, len(d) % 60 == 0, len(d) % 32 == 0
    # item size by divisibility + sanity (pattern_base always 0x5000)
    for sz in (88, 60, 32):
        if len(d) % sz == 0 and all(u16(d, i * sz + 4) == 0x5000 for i in range(len(d) // sz)):
            break
    else:
        raise ValueError("playlist item size not 88/60/32 (len %d)" % len(d))
    items = []
    for i in range(len(d) // sz):
        b = i * sz
        items.append({
            "pos": u32(d, b), "idx": u16(d, b + 6), "len": u32(d, b + 8), "track": 499 - u16(d, b + 12),
            "group": u16(d, b + 14), "flags": u16(d, b + 18),
            "s_off": f32(d, b + 24), "e_off": f32(d, b + 28),
        })
    return sz, items


def notes_of(d):
    out = []
    for i in range(len(d) // 24):
        b = i * 24
        out.append({"pos": u32(d, b), "ch": u16(d, b + 6), "len": u32(d, b + 8), "key": u16(d, b + 12), "vel": d[b + 21]})
    return out


def build(buf):
    hdr, evs = read_events(buf)
    P = {"hdr": hdr, "n_events": len(evs), "events": evs, "channels": [], "patterns": [], "arrangements": [], "inserts": [],
         "proj": {}, "mixer_params": None}
    ch = pat = arr = ins = slot = None
    ctx = "head"
    for e in evs:
        i, d = e.id, e.d
        if i == 64:
            ch = {"iid": u16(d), "ev": [], "first_ev": e.i, "type": None, "internal": None, "name": None, "wrapper": None, "data": None,
                  "sample": None, "routed": None, "auto": None}
            P["channels"].append(ch)
            ctx = "ch"
            continue
        if i == 65:
            iid = u16(d)
            if pat is None or pat["iid"] != iid:
                pat = {"iid": iid, "notes": [], "name": None, "ev": []}
                P["patterns"].append(pat)
            ctx = "pat"
            continue
        if i == 99:
            arr = {"iid": u16(d), "name": None, "playlist": None, "tracks": [], "track_names": {}}
            P["arrangements"].append(arr)
            ctx = "arr"
            continue
        if ctx == "arr" and i == 236 and arr is not None:
            ctx = "mix"
        if ctx == "mix" and i == 225:
            ctx = "tail"
        if ctx == "head":
            if i == 199:
                P["proj"]["version"] = text(d)
            elif i == 159:
                P["proj"]["build"] = u32(d)
            elif i == 156:
                P["proj"]["tempo_milli"] = u32(d)
            elif i == 194:
                P["proj"]["title"] = text(d)
            elif i == 192:
                P["proj"]["sig0"] = text(d)
        elif ctx == "ch":
            ch["ev"].append(e.i)
            if i == 21:
                ch["type"] = d[0]
            elif i == 201:
                ch["internal"] = text(d)
            elif i == 203:
                ch["name"] = text(d)
            elif i == 212:
                ch["wrapper"] = d
            elif i == 213:
                ch["data"] = d
            elif i == 196:
                ch["sample"] = text(d)
            elif i == 22:
                ch["routed"] = struct.unpack("b", d)[0]
            elif i == 234:
                ch["auto"] = d
            elif i == 215:
                ch["params_len"] = len(d)
        elif ctx == "pat":
            pat["ev"].append(e.i)
            if i == 224:
                pat["notes"].extend(notes_of(d))
            elif i == 193:
                pat["name"] = text(d)
        elif ctx == "arr":
            if i == 241:
                arr["name"] = text(d)
            elif i == 233:
                arr["playlist"] = parse_playlist(d)
            elif i == 238:
                arr["tracks"].append(d)
            elif i == 100:
                arr["current"] = u16(d)
        elif ctx == "mix":
            if i == 236:
                ins = {"flags": u32(d, 4) if len(d) >= 8 else None, "flags_raw": d, "slots": [], "name": None, "color": None, "out": None,
                       "inp": None, "routing": None}
                P["inserts"].append(ins)
                slot = {"ev": {}}
            elif i == 98:
                slot["index"] = u16(d)
                ins["slots"].append(slot)
                slot = {"ev": {}}
            elif i in (201, 212, 203, 213, 155, 128, 41):
                slot["ev"][i] = d
            elif i == 235:
                ins["routing"] = d
            elif i == 147:
                ins["out"] = i32(d)
            elif i == 154:
                ins["inp"] = i32(d)
            elif i == 149:
                ins["color"] = u32(d)
            elif i == 204:
                ins["name"] = text(d)
        elif ctx == "tail":
            if i == 225:
                P["mixer_params"] = d
    # 225 comes after the inserts: handled above by ctx switch; store it
    for e in evs:
        if e.id == 225:
            P["mixer_params"] = e.d
    return P


def mixer_params(d):
    """12-byte items [u32][u8 id][u8][u16 channel_data][i32 msg]; insert=(cd>>6)&0x7f slot=cd&0x3f (PyFLP mixer.py)."""
    out = []
    for k in range(len(d) // 12):
        b = k * 12
        cd = u16(d, b + 6)
        out.append({"id": d[b + 4], "insert": (cd >> 6) & 0x7F, "slot": cd & 0x3F, "msg": i32(d, b + 8)})
    return out


if __name__ == "__main__":
    P = build(open(sys.argv[1], "rb").read())
    print(P["hdr"], P["n_events"], P["proj"], len(P["channels"]), len(P["patterns"]), len(P["inserts"]))
