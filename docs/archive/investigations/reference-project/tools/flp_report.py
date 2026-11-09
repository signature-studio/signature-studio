#!/usr/bin/python3
"""Builds the TSV tables and a JSON summary for the reference project from the FLP (own parser: flp_model.py).

usage: flp_report.py FLP ZIPDIR_WITH_AUDIO OUTDIR [PLUGINS_INI] [G8_PLUGINS_TSV]
STATIC ANALYSIS ONLY. The FLP itself is not copied anywhere. Standard library only.
"""
import collections
import csv
import json
import os
import re
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import flp_model as M  # noqa: E402

PPQ_BAR = 4  # beats per bar (4/4 assumed: time signature events 17/18 are 4/4 in this file, read)


def tsv(path, header, rows):
    with open(path, "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f, delimiter="\t", lineterminator="\n")
        w.writerow(header)
        for r in rows:
            w.writerow(r)


def guid_text(uid):
    if not uid or len(uid) != 16:
        return ""
    a = uid
    return "{%02X%02X%02X%02X-%02X%02X-%02X%02X-%02X%02X-%s}" % (a[3], a[2], a[1], a[0], a[5], a[4], a[7], a[6], a[8], a[9], a[10:16].hex().upper())


def load_ini(path):
    secs = {}
    if not path or not os.path.exists(path):
        return secs
    s = open(path, encoding="utf-8-sig").read()
    for blk in re.split(r"\r?\n\r?\n", s):
        lines = blk.strip().splitlines()
        if not lines or not lines[0].startswith("["):
            continue
        d = {}
        for ln in lines[1:]:
            if "=" in ln:
                k, v = ln.split("=", 1)
                d[k] = v
        secs[lines[0].strip("[]")] = d
    by_guid = {}
    for sec, d in secs.items():
        n = int(d.get("ps_files", "0") or 0)
        for k in range(n):
            g = d.get("ps_file_guid_%d" % k, "").upper()
            if g:
                by_guid.setdefault(g, []).append((sec, d, k))
    return by_guid


def load_native(path):
    m = {}
    if not path or not os.path.exists(path):
        return m
    for r in csv.DictReader(open(path, encoding="utf-8"), delimiter="\t"):
        m.setdefault(r["plugin_name"], r)
    return m


def plugin_row(loc, host_name, internal, data, wrapper_len, ini, native):
    row = {"loc": loc, "host_name": host_name, "internal": internal, "format": "", "name": "", "vendor": "", "file": "", "event213": len(data or b""),
           "wrapper212": wrapper_len, "state53": "", "inner3": "", "inner2": "", "inner4": "", "param_list": "", "xfer": "", "guid": "", "ini_file_size": "",
           "ini_section": "", "native_dll": "", "native_dll_bytes": ""}
    if internal == "Fruity Wrapper" and data:
        r = M.parse_plugin_params(data)
        if r is None:
            row["format"] = "wrapper(unparsed)"
            return row
        f = r["file"]
        low = f.lower()
        row["format"] = "VST3" if ".vst3" in low else ("VST2" if low.endswith(".dll") else ("CLAP" if ".clap" in low else "wrapper(?)"))
        row["name"], row["vendor"], row["file"] = r["name"], r["vendor"], f
        row["guid"] = guid_text(r["uid"]) if row["format"] == "VST3" else ("fourcc:" + (r["fourcc"] or b"").decode("latin-1") if r["fourcc"] else "")
        row["state53"] = r["rec_sizes"].get(53, "")
        if row["format"] == "VST3":
            st = M.parse_vst3_state(r["state"])
            if st:
                row["inner3"] = ",".join(str(x) for x in st["inner_sizes"].get(3, []))
                row["inner2"] = ",".join(str(x) for x in st["inner_sizes"].get(2, []))
                row["inner4"] = ",".join(str(x) for x in st["inner_sizes"].get(4, []))
                row["param_list"] = st.get("param_list_count", "")
                x = st.get("xfer_processor")
                if x:
                    try:
                        j = json.loads(x["json"])
                    except ValueError:
                        j = {}
                    row["xfer"] = "XferJson product=%s productVersion=%s version=%s zstd_uncompressed=%d zstd_compressed=%d" % (
                        j.get("product"), j.get("productVersion"), j.get("version"), x["uncompressed"], x["compressed"])
            hits = ini.get(row["guid"].upper(), [])
            if hits:
                sec, d, k = hits[0]
                row["ini_file_size"] = d.get("ps_file_size_%d" % k, "")
                row["ini_section"] = sec
        elif row["format"] == "VST2":
            base = f.replace("\\", "/").split("/")[-1].lower()
            for g, lst in ini.items():
                for sec, d, k in lst:
                    if d.get("ps_file_filename_%d" % k, "").replace("\\", "/").split("/")[-1].lower() == base:
                        row["ini_file_size"] = d.get("ps_file_size_%d" % k, "")
                        row["ini_section"] = sec
                        break
    else:
        row["format"] = "native"
        row["name"] = host_name or internal
        n = native.get(internal)
        if n:
            row["native_dll"] = n["dll_in_installer"]
            row["native_dll_bytes"] = n["dll_uncompressed_bytes"]
    return row


def main(argv):
    flp, audio_dir, out = argv[1], argv[2], argv[3]
    ini = load_ini(argv[4] if len(argv) > 4 else None)
    native = load_native(argv[5] if len(argv) > 5 else None)
    os.makedirs(out, exist_ok=True)
    buf = open(flp, "rb").read()
    P = M.build(buf)
    E = P["events"]
    chmap = {c["iid"]: c for c in P["channels"]}
    tempo = P["proj"]["tempo_milli"] / 1000.0
    ppq = P["hdr"]["ppq"]
    sec_per_tick = 60.0 / (tempo * ppq)
    arr = P["arrangements"][0]
    plsz, items = arr["playlist"]

    # events by id (bytes) ---------------------------------------------------------------------------------------
    by_id = collections.defaultdict(lambda: [0, 0])
    for e in E:
        by_id[e.id][0] += 1
        by_id[e.id][1] += len(e.d)
    rows = []
    for k, (n, b) in sorted(by_id.items(), key=lambda kv: -kv[1][1]):
        rows.append((k, M.NAMES.get(k, "?"), n, b, "%.2f" % (100.0 * b / len(buf))))
    tsv(os.path.join(out, "event_bytes.tsv"), ["event_id", "pyflp_name_or_?", "count", "payload_bytes", "percent_of_flp_file"], rows)

    # channel details ----------------------------------------------------------------------------------------------
    notes_per_ch = collections.Counter()
    for p in P["patterns"]:
        for n in p["notes"]:
            notes_per_ch[n["ch"]] += 1
    items_per_ch = collections.Counter(it["idx"] for it in items if it["idx"] <= 0x5000)
    ch_rows = []
    plug_rows = []
    for c in P["channels"]:
        t = c["type"]
        if t == 5:
            kind = "AutomationClip"
        elif t == 4 and c["sample"]:
            kind = "AudioClip"
        elif t == 0:
            kind = "Sampler"
        elif t == 3:
            kind = "Layer"
        elif c["internal"] == "Fruity Wrapper":
            kind = "Instrument(plugin wrapper)"
        elif c["internal"]:
            kind = "Instrument(native generator)"
        else:
            kind = "Channel?"
        d215 = [E[i].d for i in c["ev"] if E[i].id == 215]
        stretch = ""
        if d215 and len(d215[0]) >= 112 and kind == "AudioClip":
            stretch = "mode=%d pitch=%d time=%d mult=%d" % (M.i32(d215[0], 108), M.i32(d215[0], 100), M.i32(d215[0], 96), M.i32(d215[0], 104))
        e104 = [M.u16(E[i].d) for i in c["ev"] if E[i].id == 104]
        routed = (e104[0] - 1) if e104 else ""
        pr = None
        if c["internal"]:
            pr = plugin_row("channel:%d" % c["iid"], c["name"], c["internal"], c["data"], len(c["wrapper"] or b""), ini, native)
            plug_rows.append(pr)
        sample = c["sample"] or ""
        base = sample.replace("\\", "/").split("/")[-1] if sample else ""
        ch_rows.append((c["iid"], t, M.CHANNEL_TYPES.get(t, "?"), kind, c["name"], c["internal"] or "", pr["format"] if pr else "", pr["name"] if pr else "",
                        len(c["data"] or b""), base, "yes" if (base and os.path.exists(os.path.join(audio_dir, base))) else ("no" if base else ""),
                        routed, items_per_ch.get(c["iid"], 0), notes_per_ch.get(c["iid"], 0), stretch))
    tsv(os.path.join(out, "channels.tsv"),
        ["iid", "type_code", "type_pyflp_name", "kind", "channel_name", "plugin_internal_name", "plugin_format", "plugin_display", "plugin_state_event213_bytes",
         "sample_basename", "sample_in_zip", "mixer_insert(ev104-1)", "playlist_items", "pattern_notes", "stretch(ev215 offsets from PyFLP)"], ch_rows)

    # mixer ---------------------------------------------------------------------------------------------------------
    mp = M.mixer_params(P["mixer_params"]) if P["mixer_params"] else []
    mp_by = collections.defaultdict(lambda: collections.defaultdict(list))
    for x in mp:
        mp_by[x["insert"]][x["id"]].append((x["slot"], x["msg"]))
    mrows = []
    nslots_plugins = 0
    for n, ins in enumerate(P["inserts"]):
        plugs = []
        for s in ins["slots"]:
            ev = s["ev"]
            if 201 in ev:
                internal = M.text(ev[201])
                hn = M.text(ev[203]) if 203 in ev else ""
                pr = plugin_row("insert:%d/slot:%d" % (n, s["index"]), hn, internal, ev.get(213), len(ev.get(212, b"")), ini, native)
                plug_rows.append(pr)
                plugs.append("%s[%s]" % (pr["name"] or internal, pr["format"]))
                nslots_plugins += 1
        route = [j for j, b in enumerate(ins["routing"] or b"") if b]
        recv = [c["iid"] for c in P["channels"] if any(M.u16(E[i].d) - 1 == n for i in c["ev"] if E[i].id == 104)]
        vol = mp_by[n].get(192)
        dis = [sl for (sl, m) in mp_by[n].get(0, []) if m == 0]
        mrows.append((n, "master" if n == 0 else ("current?" if n == 126 else ""), ins["name"] or "", hex(ins["flags"]) if ins["flags"] is not None else "",
                      len(plugs), ";".join(plugs), ";".join(str(j) for j in route), ";".join(str(i) for i in recv),
                      vol[0][1] if vol else "", ";".join(str(s) for s in dis)))
    tsv(os.path.join(out, "mixer.tsv"),
        ["insert_record", "role(read)", "name", "flags_u32", "plugin_slots_used", "plugins", "routes_to_inserts", "channels_routed_here(ev104-1)", "volume_param_msg(id192)",
         "slots_disabled(MixerParams id0 msg0)"], mrows)

    pcols = ["loc", "host_name", "internal", "format", "name", "vendor", "file", "event213", "wrapper212", "state53", "inner3", "inner2", "inner4", "param_list", "xfer",
             "guid", "ini_file_size", "ini_section", "native_dll", "native_dll_bytes"]
    tsv(os.path.join(out, "plugins.tsv"), pcols, [[r[k] for k in pcols] for r in plug_rows])

    # patterns -----------------------------------------------------------------------------------------------------
    prow = []
    for p in P["patterns"]:
        nn = p["notes"]
        end = max((x["pos"] + x["len"] for x in nn), default=0)
        chs = collections.Counter(x["ch"] for x in nn)
        prow.append((p["iid"], p["name"] or "", len(nn), end, ";".join("%d:%d" % kv for kv in sorted(chs.items())),
                     sum(1 for it in items if it["idx"] == 0x5000 + p["iid"])))
    tsv(os.path.join(out, "patterns.tsv"), ["pattern_iid", "name", "notes", "last_note_end_tick", "notes_by_channel_iid", "playlist_clips"], prow)

    # playlist ------------------------------------------------------------------------------------------------------
    for it in items:
        if it["idx"] <= 0x5000:
            c = chmap.get(it["idx"])
            it["kind"] = "audio" if c and c["type"] == 4 else ("automation" if c and c["type"] == 5 else "channel?")
            it["ch"] = it["idx"]
        else:
            it["kind"] = "pattern"
            it["ch"] = it["idx"] - 0x5000
    tsv(os.path.join(out, "playlist_items.tsv"), ["track0", "kind", "channel_or_pattern_iid", "start_tick", "length_tick", "start_s", "length_s", "start_offset", "end_offset"],
        [(it["track"], it["kind"], it["ch"], it["pos"], it["len"], "%.3f" % (it["pos"] * sec_per_tick), "%.3f" % (it["len"] * sec_per_tick),
          "%.1f" % it["s_off"], "%.1f" % it["e_off"]) for it in sorted(items, key=lambda x: (x["track"], x["pos"]))])
    trk = collections.defaultdict(list)
    for it in items:
        trk[it["track"]].append(it)
    trows = []
    for t in sorted(trk):
        its = trk[t]
        kc = collections.Counter(i["kind"] for i in its)
        trows.append((t, len(its), kc["audio"], kc["pattern"], kc["automation"], min(i["pos"] for i in its), max(i["pos"] + i["len"] for i in its),
                      "%.1f" % (min(i["pos"] for i in its) * sec_per_tick), "%.1f" % (max(i["pos"] + i["len"] for i in its) * sec_per_tick)))
    tsv(os.path.join(out, "playlist_tracks.tsv"), ["track0", "clips", "audio_clips", "pattern_clips", "automation_clips", "first_start_tick", "last_end_tick", "first_start_s", "last_end_s"], trows)

    # automation ----------------------------------------------------------------------------------------------------
    arows = []
    for c in P["channels"]:
        if c["type"] == 5:
            a = c["auto"]
            arows.append((c["iid"], c["name"], M.u32(a, 17), len(a), items_per_ch.get(c["iid"], 0)))
    tsv(os.path.join(out, "automation_clips.tsv"), ["channel_iid", "clip_name(auto-named after target; link bytes not decoded)", "points", "event234_bytes", "playlist_clips"], arows)

    # timeline sweep: concurrency ---------------------------------------------------------------------------------------
    # pattern length (read): ceil(last note end / one bar) bars, minimum 1 bar
    bar = ppq * PPQ_BAR
    plen = {}
    for p in P["patterns"]:
        end = max((x["pos"] + x["len"] for x in p["notes"]), default=0)
        plen[p["iid"]] = max(bar, -(-end // bar) * bar)
    note_iv = []  # (start, end, channel)
    for it in items:
        if it["kind"] != "pattern":
            continue
        p = next((q for q in P["patterns"] if q["iid"] == it["ch"]), None)
        if not p:
            continue
        L = plen[p["iid"]]
        rep = 0
        while rep * L < it["len"]:
            for n in p["notes"]:
                s = rep * L + n["pos"]
                if s >= it["len"]:
                    continue
                note_iv.append((it["pos"] + s, it["pos"] + min(s + n["len"], it["len"]), n["ch"]))
            rep += 1
    marks = sorted({0} | {it["pos"] for it in items} | {it["pos"] + it["len"] for it in items} | {a for a, b, c in note_iv} | {b for a, b, c in note_iv})
    ins_of_ch = {}
    for c in P["channels"]:
        e = [M.u16(E[i].d) - 1 for i in c["ev"] if E[i].id == 104]
        ins_of_ch[c["iid"]] = e[0] if e else -1
    route = {n: [j for j, b in enumerate(i["routing"] or b"") if b] for n, i in enumerate(P["inserts"])}
    inserts_plugins = collections.defaultdict(list)
    for r in plug_rows:
        m = re.match(r"insert:(\d+)/", r["loc"])
        if m:
            inserts_plugins[int(m.group(1))].append(r)
    chplug = {}
    for r in plug_rows:
        m = re.match(r"channel:(\d+)$", r["loc"])
        if m:
            chplug[int(m.group(1))] = r["name"]
    # sweep with naive recomputation (3.5 k items, ~ 4 k marks: fine)
    tl = []
    for t in marks:
        act = [it for it in items if it["pos"] <= t < it["pos"] + it["len"]]
        na = sum(1 for i in act if i["kind"] == "audio")
        npat = sum(1 for i in act if i["kind"] == "pattern")
        nau = sum(1 for i in act if i["kind"] == "automation")
        notes = collections.Counter(c for a, b, c in note_iv if a <= t < b)
        srcs = set()
        for i in act:
            if i["kind"] == "audio":
                srcs.add(ins_of_ch.get(i["ch"], -1))
        for ch_, n_ in notes.items():
            srcs.add(ins_of_ch.get(ch_, -1))
        live = set()
        stack = [s for s in srcs if s >= 0]
        while stack:
            x = stack.pop()
            if x in live:
                continue
            live.add(x)
            stack.extend(route.get(x, []) or ([0] if x != 0 else []))
        if srcs - {-1}:
            live.add(0)
        vst = sum(1 for n in live for r in inserts_plugins.get(n, []) if r["format"] in ("VST3", "VST2"))
        nat = sum(1 for n in live for r in inserts_plugins.get(n, []) if r["format"] == "native")
        s2 = sum(1 for ch_ in notes if chplug.get(ch_) == "Serum2")
        vt = sum(1 for ch_ in notes if chplug.get(ch_) == "Vital")
        fx = sum(1 for ch_ in notes if chplug.get(ch_) == "Agogo" or (chmap.get(ch_) and chmap[ch_]["internal"] == "FLEX"))
        tl.append((t, na, npat, nau, sum(notes.values()), len(notes), len(live), vst, nat, s2, vt, fx))
    # collapse equal rows into intervals
    rows2 = []
    for k, r in enumerate(tl):
        end = tl[k + 1][0] if k + 1 < len(tl) else r[0]
        if end > r[0]:
            rows2.append((r[0], end, "%.2f" % (r[0] * sec_per_tick), "%.2f" % (end * sec_per_tick)) + r[1:])
    tsv(os.path.join(out, "timeline_intervals.tsv"),
        ["start_tick", "end_tick", "start_s", "end_s", "audio_clips", "pattern_clips", "automation_clips", "active_notes", "channels_with_notes", "live_inserts(source+route chain)",
         "vst_instances_in_live_inserts", "native_effect_instances_in_live_inserts", "serum2_channels_with_notes", "vital_channels_with_notes", "flex_channels_with_notes"], rows2)

    # summary json ----------------------------------------------------------------------------------------------------
    song_end = max(it["pos"] + it["len"] for it in items)
    song_start = min(it["pos"] for it in items)
    kc = collections.Counter(i["kind"] for i in items)
    fm = collections.Counter(r["format"] for r in plug_rows)
    summ = {
        "flp_bytes": len(buf), "fl_version": P["proj"].get("version"), "format": P["hdr"]["format"], "header_channels": P["hdr"]["channels"], "ppq": ppq,
        "tempo_bpm": tempo, "events": P["n_events"], "channels": len(P["channels"]),
        "channel_kinds": dict(collections.Counter(r[3] for r in ch_rows)), "patterns": len(P["patterns"]), "notes": sum(len(p["notes"]) for p in P["patterns"]),
        "arrangements": len(P["arrangements"]), "playlist_item_size": plsz, "playlist_items": len(items), "playlist_kinds": dict(kc),
        "playlist_tracks_used": len(trk), "playlist_tracks_total": len(arr["tracks"]),
        "song_first_start_tick": song_start, "song_end_tick": song_end, "song_end_s": song_end * sec_per_tick, "song_end_bars": song_end / bar,
        "inserts_records": len(P["inserts"]), "inserts_named": sum(1 for i in P["inserts"] if i["name"]),
        "inserts_with_plugins": sum(1 for r in mrows if r[4] > 0), "mixer_plugin_instances": nslots_plugins,
        "plugin_instances_total": len(plug_rows), "plugin_formats": dict(fm),
        "plugin_event213_bytes_total": sum(r["event213"] for r in plug_rows),
        "plugin_event213_bytes_by_format": {k: sum(r["event213"] for r in plug_rows if r["format"] == k) for k in fm},
        "mixer_params_items": len(mp), "events_top": rows[:8],
        "peak": {"audio": max(r[4] for r in rows2), "pattern": max(r[5] for r in rows2), "automation": max(r[6] for r in rows2),
                 "active_notes": max(r[7] for r in rows2), "live_inserts": max(r[9] for r in rows2), "vst": max(r[10] for r in rows2), "native_fx": max(r[11] for r in rows2),
                 "serum2_with_notes": max(r[12] for r in rows2)},
    }
    json.dump(summ, open(os.path.join(out, "summary.json"), "w"), indent=1, ensure_ascii=False)
    print(json.dumps(summ, indent=1, ensure_ascii=False)[:6000])


if __name__ == "__main__":
    main(sys.argv)
