#!/usr/bin/python3
"""Identify (and optionally write) an NSIS payload by size / SHA-256 instead of by name.

Reuses extract_nsis2.py. Streams each candidate, no intermediate files.

usage:
  find_nsis_by_size.py INSTALLER --scan NAME_SUBSTR [--size N]
      hash every extract entry whose name contains NAME_SUBSTR (optionally only report size N)
  find_nsis_by_size.py INSTALLER --write INDEX SHA256 SIZE OUTPUT
      stream entry INDEX, verify SHA256/SIZE while writing, OUTPUT must not exist
"""
import hashlib
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import extract_nsis2 as x  # noqa: E402


def main(argv):
    installer, mode = argv[1], argv[2]
    with open(installer, "rb") as f:
        hdr, info = x.read_header(f)
        _, blocks = x.parse_blocks(hdr)
        files = list(x.extract_files(hdr, blocks))
        if mode == "--scan":
            sub = argv[3].lower()
            want = int(argv[5]) if len(argv) > 5 and argv[4] == "--size" else None
            for i, name, off in files:
                if sub not in name.lower():
                    continue
                h = hashlib.sha256()
                n = 0
                for c in x.stream_file(f, info["data_start"], off):
                    h.update(c)
                    n += len(c)
                if want is None or n == want:
                    print(i, hex(off), name, n, h.hexdigest(), flush=True)
            return 0
        if mode == "--write":
            idx, sha, size, outp = int(argv[3]), argv[4].lower(), int(argv[5]), argv[6]
            ent = [e for e in files if e[0] == idx]
            if len(ent) != 1:
                print("no such entry", idx)
                return 2
            h = hashlib.sha256()
            n = 0
            with open(outp, "xb") as out:
                for c in x.stream_file(f, info["data_start"], ent[0][2]):
                    h.update(c)
                    n += len(c)
                    out.write(c)
            print(ent[0][1], n, h.hexdigest())
            if n != size or h.hexdigest() != sha:
                print("MISMATCH (output file left in place for inspection)")
                return 3
            print("MATCH")
            return 0
    return 64


if __name__ == "__main__":
    sys.exit(main(sys.argv))
