#!/usr/bin/env python3
"""Unpacks a seed pack (pack_seed.py, decision 0134) into a directory tree, checking every file's SHA-256 against the pack's header. Used (a) to check that the pack round-trips and (b) to make the staging tree that
emcc's --embed-file / --preload-file take for the comparison of the seed hand-over methods. usage: unpack_seed.py PACK OUTDIR   (OUTDIR/seed/<path> for every entry; OUTDIR must not exist)"""
import hashlib
import json
import os
import struct
import sys

pack, out = sys.argv[1], sys.argv[2]
b = open(pack, "rb").read()
assert b[:8] == b"FLSEEDP1", "bad magic"
(hl,) = struct.unpack("<I", b[8:12])
h = json.loads(b[12:12 + hl])
base = 12 + hl
assert not os.path.exists(out), "exists: " + out
n = 0
for f in h["files"]:
    d = b[base + f["offset"]: base + f["offset"] + f["size"]]
    assert hashlib.sha256(d).hexdigest() == f["sha256"], "SHA-256 mismatch " + f["path"]
    p = os.path.join(out, f["path"])
    os.makedirs(os.path.dirname(p), exist_ok=True)
    open(p, "wb").write(d)
    n += 1
print("unpacked %d files, %d data bytes, all SHA-256 equal" % (n, h["data_bytes"]))
