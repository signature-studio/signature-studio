#!/usr/bin/env python3
"""Extracts the GetStringTypeW(CT_CTYPE1) values of Wine 10.0 (en-US) for the range D of decision 0087 into the data file
lift/runtime/test/c1_wine_10_0_enus.txt. The values are UNOFFICIAL (Wine, not Windows) and cannot be regenerated: they are the
output of a Wine run. What this script reproduces is only the extraction from the record of that run.

usage: gen_c1_wine_data.py WINE_RECORD.txt OUT.txt

WINE_RECORD.txt: lift/out-ctype/run/wine-enUS-out.txt of the investigation docs/investigations/dsp-ippv2-ctype-locale-data.md
(a gitignored work file), the stdout of wine/ct_wine.exe. Its SHA-256 is checked first. OUT.txt must not exist (it is never
overwritten). Only the Python standard library is used.
Reproduce:  python3 lift/runtime/test/gen_c1_wine_data.py lift/out-ctype/run/wine-enUS-out.txt OUT.txt
"""
import hashlib
import re
import sys

RECORD_SHA256 = "36f6ecd1c1c147925fb952c6714c95bbf1f3e67cefe2fa0594d527ed368b18ec"
# The 27 units above U+00FF that ATTACH passes (investigation 3.1); the record measured these and U+0000..U+00FF.
ATTACH_HIGH = [
    0x0152, 0x0153, 0x0160, 0x0161, 0x0178, 0x017D, 0x017E, 0x0192, 0x02C6, 0x02DC, 0x2013, 0x2014, 0x2018, 0x2019,
    0x201A, 0x201C, 0x201D, 0x201E, 0x2020, 0x2021, 0x2022, 0x2026, 0x2030, 0x2039, 0x203A, 0x20AC, 0x2122,
]
INFO = "INFO GetACP=1252 GetOEMCP=437 UserDefaultLCID=0x409 ThreadLocale=0x409"


def sections(text: str) -> dict:
    """(tag, ty) -> {index: (unit, value)} for the GSTW sections of the record."""
    out, cur = {}, None
    for l in text.split("\n"):
        m = re.fullmatch(r"GSTW (\w+) ty=(\d+) ok=(\d) err=(\d+)", l)
        if m:
            if m.group(3) != "1" or m.group(4) != "0":
                raise SystemExit("a GetStringTypeW call failed in the record: " + l)
            cur = (m.group(1), int(m.group(2)))
            out[cur] = {}
            continue
        m = re.fullmatch(r"GSTW (\w+) (\d+) ([0-9a-f]{4}) ([0-9a-f]{4})", l)
        if m and cur and cur[0] == m.group(1):
            out[cur][int(m.group(2))] = (int(m.group(3), 16), int(m.group(4), 16))
    return out


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 64
    rpath, out = sys.argv[1], sys.argv[2]
    raw = open(rpath, "rb").read()
    if hashlib.sha256(raw).hexdigest() != RECORD_SHA256:
        print("record SHA-256 differs from the one in decision 0087", file=sys.stderr)
        return 1
    text = raw.decode("ascii").replace("\r", "")  # the record is CRLF (stdout of a mingw program)
    if text.split("\n")[0] != INFO:
        print("the record is not the en-US run (GetACP 1252, GetOEMCP 437, LCID 0x409)", file=sys.stderr)
        return 1
    sec = sections(text)
    latin, attach = sec[("latin", 1)], sec[("in", 1)]
    if len(latin) != 256 or [latin[i][0] for i in range(256)] != list(range(256)):
        raise SystemExit("the 'latin' section is not U+0000..U+00FF in order")
    if len(attach) != 256:
        raise SystemExit("the 'in' section does not have 256 units")
    c1 = {u: v for u, v in latin.values()}
    for u, v in attach.values():
        if c1.setdefault(u, v) != v:
            raise SystemExit("the two sections disagree on U+%04X" % u)
    high = sorted(u for u in c1 if u > 0xFF)
    if high != sorted(ATTACH_HIGH):
        raise SystemExit("the units above U+00FF in the record are not the 27 units of the investigation")
    if len(c1) != 283:
        raise SystemExit("expected 283 units, got %d" % len(c1))
    lines = [
        "# DATA, NOT GENERATED FROM A SPECIFICATION (decision 0087). UNOFFICIAL: Wine 10.0, not Windows. Do not edit.",
        "# GetStringTypeW(CT_CTYPE1) of Wine 10.0 (Ubuntu 10.0~repack-12ubuntu1), en-US: LC_ALL=en_US.UTF-8; the run itself reports",
        "# GetACP=1252 GetOEMCP=437 UserDefaultLCID=0x409. Measured by wine/ct_wine.c of the investigation",
        "# docs/investigations/dsp-ippv2-ctype-locale-data.md (5.1; work file lift/out-ctype/wine/ct_wine.c SHA-256",
        "# 0989d54b5e654fcea9f40aba435f4e99fc2a123829203ae49343b3b23af0edb7, built with x86_64-w64-mingw32-gcc to ct_wine.exe SHA-256",
        "# a4feab56b11cd7d25267a7ce61b016b95824ce07216991a4a8486a808230d20b), on U+0000..U+00FF and on the 256 units ATTACH passes.",
        "# The record is lift/out-ctype/run/wine-enUS-out.txt, SHA-256 " + RECORD_SHA256 + " (a gitignored work file).",
        "# The values cannot be regenerated here (they are Wine's output). gen_c1_wine_data.py only extracts them from the record.",
        "# 283 rows: U+0000..U+00FF (256) and the 27 units above U+00FF that ATTACH passes. U+0191 (in the range D) was not measured.",
        "# Bits (Learn, GetStringTypeW): UPPER 0x1 LOWER 0x2 DIGIT 0x4 SPACE 0x8 PUNCT 0x10 CNTRL 0x20 BLANK 0x40 XDIGIT 0x80",
        "# ALPHA 0x100 DEFINED 0x200. One row per unit: UTF-16 unit, C1 value.",
    ]
    for u in sorted(c1):
        lines.append("0x%04X 0x%04X" % (u, c1[u]))
    with open(out, "x") as f:
        f.write("\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
