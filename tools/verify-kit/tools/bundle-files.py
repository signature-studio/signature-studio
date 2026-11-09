#!/usr/bin/env python3
# bundle-files.py DIR [verify]: the asset files of the manifest in DIR (names), or verify their size and sha256 (exit 1 on a mismatch)
import json, os, sys, hashlib
d = sys.argv[1]; m = json.load(open(d + '/manifest.json')); out = []
def walk(o):
    if isinstance(o, dict):
        if 'url' in o and 'sha256' in o: out.append((os.path.basename(o['url']), o['size'], o['sha256']))
        for v in o.values(): walk(v)
    elif isinstance(o, list):
        for v in o: walk(v)
walk(m)
if len(sys.argv) > 2:
    bad = 0
    for n, size, sha in out:
        b = open(f'{d}/{n}', 'rb').read(); ok = len(b) == size and hashlib.sha256(b).hexdigest() == sha
        print(('OK  ' if ok else 'BAD ') + n, len(b), sha[:12]); bad += 0 if ok else 1
    sys.exit(1 if bad else 0)
for n, _, _ in out: print(n)
