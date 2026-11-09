#!/usr/bin/python3
"""Collects the numbers that feed a performance budget (counts only; no time is estimated here) from the tables.
usage: budget.py TABLES_DIR
"""
import csv
import json
import os
import sys

sys.dont_write_bytecode = True


def rd(p):
    return list(csv.DictReader(open(p, encoding="utf-8"), delimiter="\t"))


def main(d):
    S = json.load(open(os.path.join(d, "summary.json")))
    tl = rd(os.path.join(d, "timeline_intervals.tsv"))
    au = rd(os.path.join(d, "audio_files.tsv"))
    pl = rd(os.path.join(d, "plugins.tsv"))
    tempo, ppq = S["tempo_bpm"], S["ppq"]
    spt = 60.0 / (tempo * ppq)
    tot = sum(int(r["end_tick"]) - int(r["start_tick"]) for r in tl)

    def avg(k):
        return sum((int(r["end_tick"]) - int(r["start_tick"])) * int(r[k]) for r in tl) / tot

    def secs(pred):
        return sum((int(r["end_tick"]) - int(r["start_tick"])) for r in tl if pred(r)) * spt

    rows = []
    add = lambda k, v, note="": rows.append((k, v, note))
    add("song_length_s", "%.1f" % (S["song_end_tick"] * spt), "last clip end; 108 bars at 200 BPM, PPQ 96, 4/4 (events 17/18 = 4,4)")
    add("song_first_clip_s", "%.2f" % (S["song_first_start_tick"] * spt), "nothing is placed before this")
    add("blocks_of_4096_frames_at_44100", "%.0f" % (S["song_end_tick"] * spt * 44100 / 4096), "engine default block 4096 frames = 92.9 ms (task statement)")
    add("playlist_items_total", S["playlist_items"], "audio %d / pattern %d / automation %d" % (S["playlist_kinds"]["audio"], S["playlist_kinds"]["pattern"], S["playlist_kinds"]["automation"]))
    add("playlist_tracks_used_of_500", S["playlist_tracks_used"], "")
    for k, col in (("audio_clips", "audio_clips"), ("pattern_clips", "pattern_clips"), ("automation_clips", "automation_clips"), ("active_notes", "active_notes"),
                   ("instrument_channels_with_notes", "channels_with_notes"), ("live_inserts", "live_inserts(source+route chain)"),
                   ("third_party_effect_instances_in_live_inserts", "vst_instances_in_live_inserts"), ("native_effect_instances_in_live_inserts", "native_effect_instances_in_live_inserts")):
        add("peak_" + k, max(int(r[col]) for r in tl), "average over the song %.2f" % avg(col))
    add("seconds_with_instrument_notes", "%.1f" % secs(lambda r: int(r["active_notes"]) > 0), "")
    add("seconds_with_5_instrument_channels_sounding", "%.1f" % secs(lambda r: int(r["channels_with_notes"]) == 5), "notes only; release tails of the plugins are not counted")
    add("peak_serum2_channels_with_notes", max(int(r["serum2_channels_with_notes"]) for r in tl), "seconds with 4 at once: %.1f" % secs(lambda r: int(r["serum2_channels_with_notes"]) == 4))
    add("seconds_with_ge_9_audio_clips", "%.1f" % secs(lambda r: int(r["audio_clips"]) >= 9), "")
    add("seconds_with_ge_9_third_party_effects_live", "%.1f" % secs(lambda r: int(r["vst_instances_in_live_inserts"]) >= 9), "")
    add("plugin_instances_total", S["plugin_instances_total"], "VST3 %d / VST2 %d / native %d" % (S["plugin_formats"]["VST3"], S["plugin_formats"]["VST2"], S["plugin_formats"]["native"]))
    add("plugin_state_event213_bytes", S["plugin_event213_bytes_total"], "61.9%% of the FLP file; VST3 %d, native %d, VST2 %d" % (
        S["plugin_event213_bytes_by_format"]["VST3"], S["plugin_event213_bytes_by_format"]["native"], S["plugin_event213_bytes_by_format"]["VST2"]))
    master = [r for r in pl if r["loc"].startswith("insert:0/")]
    add("master_chain_always_on", len(master), "; ".join("%s[%s]" % (r["name"], r["format"]) for r in master))
    add("audio_files", len(au), "")
    add("audio_total_seconds", "%.1f" % sum(float(r["seconds"]) for r in au), "")
    add("audio_file_bytes", sum(int(r["bytes"]) for r in au), "")
    add("audio_decoded_float32_bytes", sum(int(r["float32_bytes"]) for r in au), "reading: FL keeps decoded samples as 32-bit float at the file's own rate (not verified in the engine)")
    add("audio_decoded_pcm16_bytes", sum(int(r["pcm16_bytes"]) for r in au), "if kept as 16-bit instead (not verified)")
    add("audio_mp3_share_of_float32", "%.1f%%" % (100.0 * sum(int(r["float32_bytes"]) for r in au if r["container"] == "MP3") / sum(int(r["float32_bytes"]) for r in au)), "4 files")
    add("mixer_insert_records", S["inserts_records"], "named %d, with plugins %d" % (S["inserts_named"], S["inserts_with_plugins"]))
    add("mixer_params_items", S["mixer_params_items"], "")
    add("channels", S["channels"], "AudioClip %d, AutomationClip %d, plugin instruments 8, Sampler 1" % (S["channel_kinds"]["AudioClip"], S["channel_kinds"]["AutomationClip"]))
    add("pattern_notes_total", S["notes"], "7 patterns")
    out = os.path.join(d, "budget_inputs.tsv")
    with open(out, "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f, delimiter="\t", lineterminator="\n")
        w.writerow(["item", "value", "note"])
        w.writerows(rows)
    for r in rows:
        print("\t".join(str(x) for x in r))


if __name__ == "__main__":
    main(sys.argv[1])
