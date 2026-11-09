#!/usr/bin/python3
"""Decision 0129: the files that the FL Studio installer puts under $INSTDIR, derived from the NSIS script (read only), as a list of (guest path, file name, compressed size, stream offset).

It does NOT decide anything about what the guest looks for: it walks the instructions of the section 'FL Studio (required)' (the one that is always installed: flags 17 = SF_SELECTED | SF_RO) in
order and records every EXTRACTFILE whose target is under $INSTDIR. The current output directory ($OUTDIR) follows CREATEDIR with its second operand 1 (= SetOutPath; the first operand
may use $_OUTDIR, which the script assigns from $OUTDIR at the start of the section). What the script does at install time and this walk does not follow: conditional jumps (IFFILEEXISTS, INTCMP, IFFLAG:
every EXTRACTFILE of the section is taken as if the installation were a fresh one with default choices), the other sections (Desktop icon, Plugin database, FL plugin (64bit), VSTi, FL Studio ASIO, Settings, FL Cloud Plugins),
files that the installer creates by other means (WriteINI, registry, ILSetup.dll plugin calls, shortcuts), and everything under $PLUGINSDIR (the installer's own temporary directory). The walk stops at the end of the section.

usage:
  gen_install_tree.py INSTALLER --list [--instdir 'C:\\Program Files\\Image-Line\\FL Studio 21']   > tree.tsv     (guest path TAB size-of-the-compressed-stream TAB stream offset TAB entry number)
  gen_install_tree.py INSTALLER --summary
  gen_install_tree.py INSTALLER --write-seed OUTDIR SEEDFILE [--match PREFIX]...   extracts the matching files (all when none) to OUTDIR/<relative path>, writes SEEDFILE (guest TAB host path relative to --root TAB sha256)
Pure reading of the installer (the standard library only)."""
import hashlib
import os
import struct
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
for cand in (os.path.join(HERE, "..", "..", "..", "tools"), os.path.join(HERE, "..", "..", "..", "..", "tools")):
    if os.path.exists(os.path.join(cand, "nsis_script_dump.py")):
        sys.path.insert(0, os.path.abspath(cand))
        break
import extract_nsis2 as X       # noqa: E402
import nsis_script_dump as D    # noqa: E402

DEFAULT_INSTDIR = "C:\\Program Files\\Image-Line\\FL Studio 21"
OP_EXTRACTFILE, OP_CREATEDIR, OP_ASSIGNVAR = 20, 11, 27


def walk(script, instdir):
    """Yields (entry number, guest path, stream offset)."""
    sec0 = None
    for (i, np_, fl, code, cs, name) in script.entries_of_section():
        if i == 0:
            sec0 = (code, cs)
    first, size = sec0
    outdir = instdir
    var_outdir = instdir          # $_OUTDIR (user variable 31)

    def sub(s):
        return s.replace("$_OUTDIR", var_outdir).replace("$INSTDIR", instdir).replace("$OUTDIR", outdir)
    for n in range(first, first + size):
        e = script.words[n]
        op = e[0]
        if op == OP_CREATEDIR:
            p = sub(script.string(e[1]))
            if e[2] == 1:
                outdir = p
        elif op == OP_ASSIGNVAR and e[1] == 31:
            # $_OUTDIR = '$OUTDIR' at the start of the section: the value is the current output directory
            if script.string(e[2]) == "$OUTDIR":
                var_outdir = outdir
        elif op == OP_EXTRACTFILE:
            name = sub(script.string(e[2]))
            off = e[3] & 0xFFFFFFFF
            if "$PLUGINSDIR" in name or "$TEMP" in name:
                continue
            if "\\" in name and ":" in name:
                path = name
            else:
                path = outdir.rstrip("\\") + "\\" + name
            if path.lower().startswith(instdir.lower() + "\\"):
                yield n, path, off


def main(argv):
    if len(argv) < 3:
        print(__doc__)
        return 64
    inst = argv[1]
    mode = argv[2]
    instdir = DEFAULT_INSTDIR
    if "--instdir" in argv:
        instdir = argv[argv.index("--instdir") + 1]
    script = D.Script(inst)
    rows = list(walk(script, instdir))
    with open(inst, "rb") as f:
        hdr, info = X.read_header(f)
        ds = info["data_start"]
        sizes = {}
        for n, path, off in rows:
            if off not in sizes:
                f.seek(ds + off)
                word = struct.unpack("<I", f.read(4))[0]
                sizes[off] = word & 0x7FFFFFFF
        if mode == "--list":
            for n, path, off in rows:
                print("%s\t%d\t%d\t%d" % (path, sizes[off], off, n))
            return 0
        if mode == "--summary":
            by = {}
            for n, path, off in rows:
                top = path[len(instdir) + 1:].split("\\")[0] if "\\" in path[len(instdir) + 1:] else "(root)"
                c = by.setdefault(top, [0, 0])
                c[0] += 1
                c[1] += sizes[off]
            tot = sum(v[1] for v in by.values())
            print("files %d, distinct streams %d, compressed bytes (one per file) %d" % (len(rows), len(sizes), tot))
            for k in sorted(by, key=lambda k: -by[k][1]):
                print("%-24s %6d files %14d compressed bytes" % (k, by[k][0], by[k][1]))
            return 0
        if mode == "--write-seed":
            outdir, seedfile = argv[3], argv[4]
            root = os.path.dirname(os.path.abspath(seedfile)) if "--root" not in argv else argv[argv.index("--root") + 1]
            matches = [argv[i + 1] for i, a in enumerate(argv) if a == "--match"]
            lines = []
            for n, path, off in rows:
                rel = path[len(instdir) + 1:]
                if matches and not any(rel.lower().startswith(m.lower()) for m in matches):
                    continue
                host = os.path.join(outdir, rel.replace("\\", "/"))
                os.makedirs(os.path.dirname(host), exist_ok=True)
                h = hashlib.sha256()
                with open(host, "xb") as out:
                    for c in X.stream_file(f, ds, off):
                        out.write(c)
                        h.update(c)
                lines.append("%s\t%s\t%s" % (path, os.path.relpath(host, root), h.hexdigest()))
            with open(seedfile, "x") as o:
                o.write("\n".join(lines) + "\n")
            print("wrote %d files" % len(lines))
            return 0
    print(__doc__)
    return 64


if __name__ == "__main__":
    sys.exit(main(sys.argv))
