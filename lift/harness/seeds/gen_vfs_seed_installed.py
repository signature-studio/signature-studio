#!/usr/bin/python3
"""Decision 0129 (S-vfs-install-tree): makes the VFS seed list vfs-seed-installed.tsv and extracts the host files.

Input: the installer (read only), the list of paths that the guest asked for (FILES_TOUCHED: lines `API TAB entry TAB path` made by the harness's FLE_FRUITY_API_TRACE of GetFileAttributesW, CreateFileW and
FindFirstFileW), and the tree that gen_install_tree.py derives. A guest path under the install directory is put in the seed when
  - it is a file of the derived tree: the file is extracted from the installer and listed (guest path TAB host path TAB sha256);
  - it is a directory of the derived tree (a directory that holds installed files): the directory is listed as a directory line (path ending with a backslash, host '-', sha '-'), and the files that are DIRECTLY in it are
    listed too when their number is at most DIRECT_MAX and their compressed bytes at most DIRECT_BYTES (a bigger directory is a directory line only: the guest sees the directory, empty);
  - anything else (not in the installer: generated at run time, a plugin location, a language file the installer does not ship): NOT put. Nothing the installer does not install is invented.
Fonts of the Windows directory and the files under Documents / AppData are not touched (the guest makes them). The host files go under HOST_ROOT/<relative path> (outside git).
usage: gen_vfs_seed_installed.py INSTALLER FILES_TOUCHED HOST_ROOT OUT.tsv [EXISTING_SEED.tsv]"""
import hashlib
import os
import subprocess
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import gen_install_tree as G   # noqa: E402

DIRECT_MAX = 100
DIRECT_BYTES = 8 * 1024 * 1024
INST = G.DEFAULT_INSTDIR


def main(argv):
    installer, touched, host_root, out = argv[1:5]
    existing = argv[5] if len(argv) > 5 else None
    script = G.D.Script(installer)
    rows = list(G.walk(script, INST))
    tree = {p.lower(): (p, off, n) for n, p, off in rows}
    kids = {}
    for q, (p, off, n) in tree.items():
        d = q.rsplit("\\", 1)[0]
        kids.setdefault(d, []).append(q)
    dirs = set()
    for q in tree:
        parts = q.split("\\")
        for k in range(1, len(parts)):
            dirs.add("\\".join(parts[:k]))
    want_files, want_dirs = {}, {}
    skipped_modules = []
    nsrc = 0
    for l in open(touched, encoding="utf-8"):
        f = l.rstrip("\n").split("\t")
        if len(f) < 3:
            continue
        nsrc += 1
        path = f[2].rstrip("\\")
        q = path.lower()
        if not q.startswith(INST.lower() + "\\") and q != INST.lower():
            continue
        if q in tree:
            if q.endswith(".dll") or q.endswith(".exe"):
                skipped_modules.append(path)      # the modules are the job of fle_realmod.c (decisions 0080-0106): a module file in the VFS would make LoadLibraryW(<full path>) stop ("a module name with a path is not supported")
                continue
            want_files[q] = 1
        elif q in dirs:
            want_dirs[q] = 1
    # a directory also seeds its direct files when it is small
    skipped_big = []
    for d in sorted(want_dirs):
        ks = kids.get(d, [])
        sizes = 0
        with open(installer, "rb") as fh:
            import struct
            hdr, info = G.X.read_header(fh)
            ds = info["data_start"]
            for k in ks:
                fh.seek(ds + tree[k][1])
                sizes += struct.unpack("<I", fh.read(4))[0] & 0x7FFFFFFF
        if len(ks) <= DIRECT_MAX and sizes <= DIRECT_BYTES:
            for k in ks:
                if k.endswith(".dll") or k.endswith(".exe"):
                    continue
                want_files[k] = 1
        else:
            skipped_big.append((d, len(ks), sizes))
    # the existing seed (decision 0110/0126) keeps its entries; a path already in it is not listed twice
    have = set()
    lines = []
    if existing:
        for l in open(existing, encoding="utf-8"):
            if l.strip() and not l.startswith("#"):
                have.add(l.split("\t")[0].lower())
    import struct
    entries = []
    with open(installer, "rb") as fh:
        hdr, info = G.X.read_header(fh)
        ds = info["data_start"]
        for q in sorted(want_files):
            p, off, n = tree[q]
            if q in have:
                continue
            host = os.path.join(host_root, p[len(INST) + 1:].replace("\\", "/"))
            if os.path.exists(host):
                h = hashlib.sha256(open(host, "rb").read()).hexdigest()
            else:
                os.makedirs(os.path.dirname(host), exist_ok=True)
                h = hashlib.sha256()
                with open(host, "xb") as o:
                    for c in G.X.stream_file(fh, ds, off):
                        o.write(c)
                        h.update(c)
                h = h.hexdigest()
            entries.append((p, os.path.relpath(host, os.environ["VFS_SEED_ROOT"]), h))   # VFS_SEED_ROOT: the directory the relative host paths of the list are relative to (no absolute path is kept in the repository)
    dir_lines = [d for d in sorted(want_dirs)]
    with open(out, "w") as o:
        o.write("# decision 0129 (S-vfs-install-tree): files of the installed product that the guest asked for, from the installer (SetOutPath/File of the section 'FL Studio (required)'), see harness/seeds/gen_vfs_seed_installed.py\n")
        for d in dir_lines:
            p = "\\".join([INST] + [x for x in d[len(INST.lower()):].split("\\") if x]) if d != INST.lower() else INST
            # restore the case of the directory from a file under it
            probe = next((tree[k][0] for k in tree if k.startswith(d + "\\")), None)
            if probe:
                p = probe[:len(d)]
            o.write("%s\\\t-\t-\n" % p)
        for p, h_, s in entries:
            o.write("%s\t%s\t%s\n" % (p, h_, s))
    print("touched lines %d; files %d; directories %d; big directories (listed as a directory only) %d: %s; module files NOT put: %d" % (nsrc, len(entries), len(dir_lines), len(skipped_big), skipped_big, len(skipped_modules)))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
