#!/usr/bin/env python3
"""Extracts, from the FL Studio NSIS installer, the product files that the engine bundle build reads, into HOST_ROOT, and checks every file's SHA-256.

  extract_fl.py INSTALLER HOST_ROOT --dlls ci/extract-dlls.tsv --seed lift/harness/seeds/vfs-seed-idle1.tsv [--manifest OUT.sha256]

  * --dlls: the engine DLL and the five bundled DLLs, found by name in the installer's EXTRACTFILE list, each with the SHA-256 it must have.
  * --seed: every row of the VFS seed list that has a host file (guest path TAB host path TAB SHA-256): the guest path is looked up in the install tree that
            lift/harness/seeds/gen_install_tree.py derives from the NSIS script (decision 0129), the stream is extracted to HOST_ROOT/<host path>.
            Exception (decision 0236): a row whose host path is under lift/harness/seeds/os-fonts/ names a file of THIS repository (a substitute for an operating-system font,
            which the installer does not carry): it is copied to HOST_ROOT/<host path> and checked against the row's SHA-256 like the others.
The installer is only read (stdlib only: the extractor of tools/extract_nsis2.py). Nothing is written when a SHA-256 differs: the run stops with exit 2.
--manifest writes `SHA-256  relative path` for every written file (sorted): its SHA-256 is the identity of the extraction.
"""
import hashlib
import os
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools"))
sys.path.insert(0, os.path.join(ROOT, "lift", "harness", "seeds"))
import extract_nsis2 as X       # noqa: E402
import gen_install_tree as G    # noqa: E402


def rows(path):
    for ln in open(path, encoding="utf-8"):
        if ln.strip() and not ln.startswith("#"):
            yield ln.rstrip("\n").split("\t")


def write_checked(f, data_start, off, dest, want):
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    tmp = dest + ".part"
    h = hashlib.sha256()
    with open(tmp, "wb") as o:
        for c in X.stream_file(f, data_start, off):
            o.write(c)
            h.update(c)
    if h.hexdigest() != want:
        os.unlink(tmp)
        return False, h.hexdigest()
    os.replace(tmp, dest)
    return True, want


def copy_checked(src, dest, want):
    h = hashlib.sha256(open(src, "rb").read()).hexdigest()
    if h != want:
        return False, h
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    tmp = dest + ".part"
    with open(src, "rb") as i, open(tmp, "wb") as o:
        o.write(i.read())
    os.replace(tmp, dest)
    return True, want


REPO_ROWS = "lift/harness/seeds/os-fonts/"   # decision 0236


def main(argv):
    if len(argv) < 3 or "--dlls" not in argv or "--seed" not in argv:
        print(__doc__); return 64
    inst, root = argv[1], os.path.abspath(argv[2])
    dlls, seed = argv[argv.index("--dlls") + 1], argv[argv.index("--seed") + 1]
    manifest = argv[argv.index("--manifest") + 1] if "--manifest" in argv else None
    written = {}
    with open(inst, "rb") as f:
        hdr, info = X.read_header(f)
        _, blocks = X.parse_blocks(hdr)
        entries = list(X.extract_files(hdr, blocks))
        ds = info["data_start"]
        for name, rel, want in rows(dlls):
            cands = sorted({off for _, n, off in entries if n.lower().endswith(name.lower())})
            ok = False
            for off in cands:
                ok, got = write_checked(f, ds, off, os.path.join(root, rel), want)
                if ok:
                    break
            if not ok:
                print("extract_fl: %s: %d candidate stream(s), none has SHA-256 %s" % (name, len(cands), want), file=sys.stderr); return 2
            written[rel] = want
        script = G.D.Script(inst)
        tree = {p.lower(): off for _, p, off in G.walk(script, G.DEFAULT_INSTDIR)}
        n = 0
        for r in rows(seed):
            if len(r) < 3 or r[1] == "-":
                continue
            guest, rel, want = r[0], r[1], r[2]
            if rel.startswith(REPO_ROWS):
                ok, got = copy_checked(os.path.join(ROOT, rel), os.path.join(root, rel), want)
                if not ok:
                    print("extract_fl: %s has SHA-256 %s, the seed list says %s" % (rel, got, want), file=sys.stderr); return 2
                written[rel] = want
                n += 1
                continue
            off = tree.get(guest.lower())
            if off is None:
                print("extract_fl: seed row %r is not in the install tree of the NSIS script" % guest, file=sys.stderr); return 2
            ok, got = write_checked(f, ds, off, os.path.join(root, rel), want)
            if not ok:
                print("extract_fl: %s has SHA-256 %s, the seed list says %s" % (guest, got, want), file=sys.stderr); return 2
            written[rel] = want
            n += 1
    if manifest:
        with open(manifest, "w") as m:
            for rel in sorted(written):
                m.write("%s  %s\n" % (written[rel], rel))
    print("extract_fl: %d DLL(s) + %d seed file(s) written under %s, all SHA-256 verified" % (len(written) - n, n, root))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
