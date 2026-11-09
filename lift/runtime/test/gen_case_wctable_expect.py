#!/usr/bin/env python3
"""Writes the expected case mapping over the range D and the WCTABLE of code page 1252 for tests/locale.rs (decision 0087),
independent of runtime/fle_locale.c.

usage: gen_case_wctable_expect.py MS8_UPPER_TABLE.txt DATAFILES.zip UnicodeData-16.0.0.txt OUT_CASE.txt OUT_WCTABLE.txt

MS8_UPPER_TABLE.txt: "Windows 8 Upper Case Mapping Table" ([MSDN-UCMT/Win8] of [MS-UCODEREF] 1.2.1; Download Center id=10921).
DATAFILES.zip: "Windows Supported Code Page Data Files.zip" (decision 0084). UnicodeData-16.0.0.txt: unicode.org (a check only).
All SHA-256 values below are checked first. The outputs must not exist (they are never overwritten).
Only the Python standard library is used. The inputs are gitignored work files: the tests read the outputs
(lift/runtime/test/case_ms8_d.txt, wctable_1252_expect.txt).
Reproduce:  python3 lift/runtime/test/gen_case_wctable_expect.py lift/out-ctype/ref/Windows_8_Upper_Case_Mapping_Table.txt \\
              lift/out-ctype/ref/Windows_Supported_Code_Page_Data_Files.zip lift/out-ctype/ref/UnicodeData-16.0.0.txt OUT_CASE OUT_WCTABLE

Range D = U+0000..U+00FF + the 27 units above U+00FF that ATTACH passes (investigation 3.1) + the closure of those under the MS8
upper mapping and its inverse (the lower mapping read from it: a reading of the investigator, not a Microsoft statement).
"""
import hashlib
import io
import sys
import zipfile

MS8_SHA256 = "76a66e29298eafc744fa1e8e11c80c3de28219c70539b3fb10efbccadb6ae335"
ZIP_SHA256 = "5074e6dd253056ba61fc6c870c9a955467855129c6ad3a51761c386b301b125a"
MEMBER = "CodpageFiles/1252.txt"
MEMBER_SHA256 = "72ea23c939c5b26fae7aded0207b327e2f3902d7d3c168d7087f5cfc38ee76a9"
UNICODEDATA_SHA256 = "ff58e5823bd095166564a006e47d111130813dcf8bf234ef79fa51a870edb48f"
ATTACH_HIGH = [
    0x0152, 0x0153, 0x0160, 0x0161, 0x0178, 0x017D, 0x017E, 0x0192, 0x02C6, 0x02DC, 0x2013, 0x2014, 0x2018, 0x2019,
    0x201A, 0x201C, 0x201D, 0x201E, 0x2020, 0x2021, 0x2022, 0x2026, 0x2030, 0x2039, 0x203A, 0x20AC, 0x2122,
]


def checked(raw: bytes, sha: str, what: str) -> bytes:
    if hashlib.sha256(raw).hexdigest() != sha:
        raise SystemExit(what + ": SHA-256 differs from the one in decision 0087")
    return raw


def ms8_table(text: str) -> dict:
    """lower -> upper (the file has the columns 'Lower Case', 'Upper Case', 'Note:')."""
    rows = {}
    for l in text.replace("\r", "").split("\n")[1:]:
        l = l.split(";")[0].strip()
        if not l:
            continue
        lo, up = [int(x, 16) for x in l.split()]
        if lo in rows:
            raise SystemExit("MS8 row twice: %#06x" % lo)
        rows[lo] = up
    return rows


def sections(text: str):
    """MBTABLE (byte -> unit), WCTABLE (unit -> byte), the CPINFO default char."""
    sec, mb, wc, default = None, {}, {}, None
    for l in text.replace("\r", "").split("\n"):
        l = l.split(";")[0].strip()
        if not l:
            continue
        if l.startswith("CPINFO"):
            default = int(l.split()[2], 16)
        elif l.startswith("MBTABLE"):
            sec = "mb"
        elif l.startswith("WCTABLE"):
            sec = "wc"
        elif l.startswith("ENDCODEPAGE"):
            sec = None
        elif l.startswith("0x") and sec:
            a, b = [int(x, 16) for x in l.split()]
            t = mb if sec == "mb" else wc
            if a in t:
                raise SystemExit("row twice: %#x" % a)
            t[a] = b
    return mb, wc, default


def ucd_simple_case(text: str) -> dict:
    out = {}
    for l in text.split("\n"):
        f = l.split(";")
        if len(f) > 13 and not f[1].endswith(", First>") and not f[1].endswith(", Last>"):
            u = int(f[0], 16)
            out[u] = (int(f[12], 16) if f[12] else u, int(f[13], 16) if f[13] else u)  # (simple upper, simple lower)
    return out


def main() -> int:
    if len(sys.argv) != 6:
        print(__doc__, file=sys.stderr)
        return 64
    ms8_path, zpath, ud_path, out_case, out_wc = sys.argv[1:]
    ms8 = ms8_table(checked(open(ms8_path, "rb").read(), MS8_SHA256, "MS8 table").decode("ascii"))
    zbytes = checked(open(zpath, "rb").read(), ZIP_SHA256, "zip")
    raw = checked(zipfile.ZipFile(io.BytesIO(zbytes)).read(MEMBER), MEMBER_SHA256, MEMBER)
    mb, wc, default = sections(raw.decode("ascii"))
    ucd = ucd_simple_case(checked(open(ud_path, "rb").read(), UNICODEDATA_SHA256, "UnicodeData").decode("utf-8"))
    if len(ms8) != 973 or len(set(ms8.values())) != 973:
        raise SystemExit("the MS8 table must have 973 rows with distinct upper cases")
    if (len(mb), len(wc), default) != (256, 698, 0x3F):
        raise SystemExit("1252.txt: expected MBTABLE 256, WCTABLE 698, default char 0x3f")
    inv = {up: lo for lo, up in ms8.items()}
    upper = lambda u: ms8.get(u, u)
    lower = lambda u: inv.get(u, u)

    d = set(range(256)) | set(ATTACH_HIGH)
    while True:
        n = d | {upper(u) for u in d} | {lower(u) for u in d}
        if n == d:
            break
        d = n
    extra = sorted(d - set(range(256)) - set(ATTACH_HIGH))
    if len(d) != 284 or extra != [0x191]:
        raise SystemExit("the closure is not the 284 units of decision 0087: extra %s" % [hex(u) for u in extra])
    touching = [(lo, up) for lo, up in ms8.items() if lo in d or up in d]
    if len(touching) != 61 or any(lo not in d or up not in d for lo, up in touching):
        raise SystemExit("the MS8 rows that touch D are not the 61 rows inside D")

    # checks against the UCD (simple case mappings): the lower mapping read from MS8 equals it on all of D; the upper mapping
    # equals it except on U+00B5 (the UCD says U+039C, MS8 has no row: unchanged)
    bad_lower = [u for u in sorted(d) if ucd[u][1] != lower(u)]
    bad_upper = [u for u in sorted(d) if ucd[u][0] != upper(u)]
    if bad_lower or bad_upper != [0xB5]:
        raise SystemExit("UCD check: lower differs on %s, upper differs on %s" % ([hex(u) for u in bad_lower], [hex(u) for u in bad_upper]))

    rev = {u: b for b, u in mb.items()}
    if len(rev) != 256 or any(wc.get(u) != b for u, b in rev.items()):
        raise SystemExit("the exact inverse of MBTABLE is not a subset of WCTABLE")

    case = [
        "# GENERATED by lift/runtime/test/gen_case_wctable_expect.py (decision 0087). Do not edit.",
        "# The range D (284 units) with the upper and lower mapping. Upper: [MS-UCODEREF] 3.1.5.3, \"Windows 8 Upper Case Mapping Table\"",
        "# (SHA-256 " + MS8_SHA256 + "); a unit without a row is unchanged (U+00B5, U+00DF, U+00AA, U+00BA are not in the table).",
        "# Lower: the inverse of that table (a reading of the investigator, not a Microsoft statement). Checked here against",
        "# UnicodeData-16.0.0.txt (SHA-256 " + UNICODEDATA_SHA256 + "): the lower mapping equals the UCD simple lowercase on all of D;",
        "# the upper mapping equals the UCD simple uppercase on all of D except U+00B5 (UCD U+039C, MS8: unchanged).",
        "# D = U+0000..U+00FF + the 27 units above U+00FF that ATTACH passes + U+0191 (the upper case of U+0192). One row per unit: unit, upper, lower.",
    ]
    for u in sorted(d):
        case.append("0x%04X 0x%04X 0x%04X" % (u, upper(u), lower(u)))

    wct = [
        "# GENERATED by lift/runtime/test/gen_case_wctable_expect.py (decision 0087). Do not edit.",
        "# The WCTABLE section of " + MEMBER + " in \"Windows Supported Code Page Data Files.zip\" (zip SHA-256 " + ZIP_SHA256 + ",",
        "# " + MEMBER + " SHA-256 " + MEMBER_SHA256 + "), 698 rows. One row per unit: unit, byte, exact (1 when the byte maps back to the",
        "# unit in MBTABLE: the exact inverse, 256 rows; 0 for a best-fit row, 442 rows). The CPINFO default character is the DEFAULTCHAR row.",
        "DEFAULTCHAR 0x%02X" % default,
    ]
    for u in sorted(wc):
        wct.append("0x%04X 0x%02X %d" % (u, wc[u], int(rev.get(u) == wc[u] and mb[wc[u]] == u)))
    exact = sum(1 for u in wc if mb.get(wc[u]) == u)
    if exact != 256:
        raise SystemExit("expected 256 exact rows, got %d" % exact)

    with open(out_case, "x") as f:
        f.write("\n".join(case) + "\n")
    with open(out_wc, "x") as f:
        f.write("\n".join(wct) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
