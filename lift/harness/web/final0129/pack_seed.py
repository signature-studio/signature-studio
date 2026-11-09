#!/usr/bin/env python3
"""Seed pack of the browser build (decision 0134, PROVISIONAL): ONE file that holds every host file the harness reads before the guest runs, so that a Worker can fetch it once and give the files to the engine (the page answers the engine's file reads from the pack bytes: app/src/engine/hostfiles.ts, decision 0221; the harness of decision 0134 wrote them into the glue's file system)
before main() (the C side is unchanged: vfs_seed.h fopen()s the same relative paths under FLE_VFS_SEED_ROOT).

What goes in (all of it is read from the repository checkout, nothing is made up):
  - the VFS seed list (TSV, lift/harness/seeds/vfs-seed-idle1.tsv) and, for every line of it that names a host file, that host file (path relative to the repository root, as in the list). The SHA-256 of every host file is checked
    against the list's third column before it is packed (the C side checks it again when it seeds the volume);
  - the registry seeds and the Authenticode claims (lift/harness/seeds/reg-seed-nsis.tsv, reg-seed-app.tsv, authenticode-claims.tsv).
Layout (little endian):   'FLSEEDP1' (8 bytes) | u32 header_length | header (UTF-8 JSON) | the data region.
  header = {"version":1,"root":"seed","count":N,"data_bytes":B,"files":[{"path":"seed/<path under the pack root>","size":S,"offset":O,"sha256":"<hex>"}, ...]}  (offset: from the start of the data region)
  The files are in the order of the list (then the extra files); a host file used by two lines of the list is stored once. Output is deterministic.
usage: pack_seed.py --root <repository root> --tsv <vfs seed list> --extra <file> [--extra <file> ...] --out <pack file>   (a JSON manifest is written next to it: <pack file>.manifest.json)
"""
import argparse
import hashlib
import json
import os
import struct
import sys


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", required=True)
    ap.add_argument("--tsv", required=True)
    ap.add_argument("--extra", action="append", default=[])
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    root = os.path.abspath(a.root)
    entries = []      # (pack path, absolute host path)
    seen = {}

    def add(pack_path, host):
        if pack_path in seen:
            if seen[pack_path] != host:
                sys.exit("pack path %s from two different files" % pack_path)
            return
        seen[pack_path] = host
        entries.append((pack_path, host))

    tsv = os.path.abspath(a.tsv)
    add("seed/" + os.path.basename(tsv), tsv)
    want = {}
    guest_seen = {}
    with open(tsv, encoding="utf-8") as f:
        for ln, line in enumerate(f, 1):
            line = line.rstrip("\r\n")
            if not line or line.startswith("#"):
                continue
            g, h, s = line.split("\t")
            # the rule of vfs_seed.h (vs_parse): a guest path may be listed once, without regard to ASCII case; a directory line is compared without its trailing backslash.
            # The loader refuses the whole list at start-up ("vfs-seed: ...: the guest path ... is listed twice"), so the pack must not be made from such a list.
            gk = g[:-1] if (h == "-" and s == "-" and g.endswith("\\")) else g
            gk = "".join(chr(ord(c) + 32) if "A" <= c <= "Z" else c for c in gk)
            if gk in guest_seen:
                sys.exit("vfs-seed: %s:%d: the guest path %s is listed twice (line %d)" % (tsv, ln, g, guest_seen[gk]))
            guest_seen[gk] = ln
            if h == "-" and s == "-":
                continue
            if h.startswith("/") or ".." in h.split("/"):
                sys.exit("%s:%d: host path must be relative without '..': %s" % (tsv, ln, h))
            add("seed/" + h, os.path.join(root, h))
            want["seed/" + h] = s.lower()
    for x in a.extra:
        x = os.path.abspath(x)
        add("seed/" + os.path.basename(x), x)
    files, blobs, off = [], [], 0
    for pack_path, host in entries:
        data = open(host, "rb").read()
        sha = hashlib.sha256(data).hexdigest()
        if pack_path in want and want[pack_path] != sha:
            sys.exit("SHA-256 mismatch for %s: list %s, file %s" % (host, want[pack_path], sha))
        files.append({"path": pack_path, "size": len(data), "offset": off, "sha256": sha})
        blobs.append(data)
        off += len(data)
    header = json.dumps({"version": 1, "root": "seed", "count": len(files), "data_bytes": off, "files": files}, separators=(",", ":")).encode("utf-8")
    with open(a.out, "wb") as o:
        o.write(b"FLSEEDP1")
        o.write(struct.pack("<I", len(header)))
        o.write(header)
        for b in blobs:
            o.write(b)
    pk = open(a.out, "rb").read()
    man = {"pack": os.path.basename(a.out), "bytes": len(pk), "sha256": hashlib.sha256(pk).hexdigest(), "header_bytes": len(header), "data_bytes": off, "files": len(files),
           "files_checked_against_list_sha256": len(want), "largest": sorted(((f["size"], f["path"]) for f in files), reverse=True)[:5]}
    json.dump(man, open(a.out + ".manifest.json", "w"), indent=1)
    print(json.dumps(man, indent=1))


if __name__ == "__main__":
    main()
