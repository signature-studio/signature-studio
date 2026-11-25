#!/usr/bin/env python3
"""Test input of realmod.rs (the negative control): copies a PE32+ image and changes only the ImageBase field of its optional header.
No relocation is applied, so the copy differs from a rebased image only where the relocations would have written.

usage: patch_base_only.py IN OUT BASE   (BASE: an integer, 0x hex accepted)
"""
import struct
import sys


def main():
    src, dst, base = sys.argv[1], sys.argv[2], int(sys.argv[3], 0)
    data = bytearray(open(src, "rb").read())
    if data[:2] != b"MZ":
        sys.exit("patch_base_only: not a PE image (no MZ)")
    e_lfanew = struct.unpack_from("<I", data, 0x3C)[0]
    if data[e_lfanew:e_lfanew + 4] != b"PE\0\0":
        sys.exit("patch_base_only: not a PE image (no PE signature)")
    optional = e_lfanew + 24
    if struct.unpack_from("<H", data, optional)[0] != 0x20B:
        sys.exit("patch_base_only: not a PE32+ image")
    struct.pack_into("<Q", data, optional + 24, base)
    open(dst, "wb").write(bytes(data))


main()
