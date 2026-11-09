#!/usr/bin/python3
"""Invariants that tie the model to the raw event stream (a check on flp_model.py / flp_report.py, not on FL).
usage: selfcheck.py FLP
"""
import collections
import os
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import flp_model as M  # noqa: E402


def main(path):
    buf = open(path, "rb").read()
    P = M.build(buf)
    E = P["events"]
    cnt = collections.Counter(e.id for e in E)
    ok = True

    def chk(name, a, b):
        nonlocal ok
        good = a == b
        ok &= good
        print("%-4s %-70s %s vs %s" % ("OK" if good else "FAIL", name, a, b))

    def vlen(n):
        k = 1
        while n >= 128:
            n >>= 7
            k += 1
        return k

    total = sum((1 + len(e.d)) if e.id < 192 else (1 + vlen(len(e.d)) + len(e.d)) for e in E)
    chk("FLdt length == re-encoded size of all events", P["hdr"]["fldt_len"], total)
    chk("trailing bytes after FLdt", P["hdr"]["trailing"], 0)
    chk("header channel count == NewChan events", P["hdr"]["channels"], cnt[64])
    chk("channels in model", len(P["channels"]), cnt[64])
    chk("event 213 (plugin data) bytes == sum over plugin instances", sum(len(e.d) for e in E if e.id == 213),
        sum(len(c["data"] or b"") for c in P["channels"]) + sum(len(s["ev"].get(213, b"")) for i in P["inserts"] for s in i["slots"]))
    chk("event 201 non-empty == plugin instances (channels + mixer slots)", sum(1 for e in E if e.id == 201 and M.text(e.d)),
        sum(1 for c in P["channels"] if c["internal"]) + sum(1 for i in P["inserts"] for s in i["slots"] if M.text(s["ev"].get(201, b""))))
    chk("slot-index events (98) == 10 slots x inserts", cnt[98], 10 * len(P["inserts"]))
    chk("insert-flags events (236) == inserts", cnt[236], len(P["inserts"]))
    chk("insert-output events (147) == inserts", cnt[147], len(P["inserts"]))
    chk("insert names (204) in model", cnt[204], sum(1 for i in P["inserts"] if i["name"]))
    chk("sample paths (196) == audio-clip channels", cnt[196], sum(1 for c in P["channels"] if c["sample"]))
    chk("automation event 234 == automation channels", cnt[234], sum(1 for c in P["channels"] if c["type"] == 5))
    chk("pattern note events (224) == patterns with notes", cnt[224], sum(1 for p in P["patterns"] if p["notes"]))
    a = P["arrangements"][0]
    sz, items = a["playlist"]
    chk("playlist event length == items x item size", len(next(e.d for e in E if e.id == 233)), sz * len(items))
    chk("every playlist item has pattern_base 0x5000", all(M.u16(next(e.d for e in E if e.id == 233), i * sz + 4) == 0x5000 for i in range(len(items))), True)
    chk("track data events (238) == 500", cnt[238], 500)
    chk("mixer params items * 12 == event 225 length", 12 * len(M.mixer_params(P["mixer_params"])), len(P["mixer_params"]))
    print("ALL OK" if ok else "SOME FAILED")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
