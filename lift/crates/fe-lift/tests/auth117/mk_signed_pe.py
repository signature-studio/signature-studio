#!/usr/bin/env python3
"""Builds a small PE64 file with an embedded Authenticode-style PKCS#7 signature, for the tests of tools/authenticode_check.py (decision 0117).
Standard library only. Everything is deterministic (a fixed seed): the RSA key (1024 bits, generated here with Miller-Rabin), the self-signed certificate, the PE.

The signature is made independently of the tool under test: its own DER writer, its own Authenticode hash (a byte mask over the file: everything except the
checksum field, the certificate-table directory entry and the certificate table), its own PKCS#1 v1.5 signing. A file made with `--tamper` flips exactly
one thing after signing, so that exactly one of the tool's checks has to fail.

usage: mk_signed_pe.py OUT.exe [--hash sha256|sha1] [--tamper none|text|signature|messagedigest|contenttype|signer|sigalg]
                              [--unsigned] [--timestamp] [--cn NAME]
"""
import hashlib
import random
import struct
import sys

sys.dont_write_bytecode = True

OID = {
    "signedData": "1.2.840.113549.1.7.2", "spcIndirect": "1.3.6.1.4.1.311.2.1.4", "spcPEImageData": "1.3.6.1.4.1.311.2.1.15",
    "contentType": "1.2.840.113549.1.9.3", "messageDigest": "1.2.840.113549.1.9.4", "rsa": "1.2.840.113549.1.1.1", "sha256WithRSA": "1.2.840.113549.1.1.11",
    "sha1": "1.3.14.3.2.26", "sha256": "2.16.840.1.101.3.4.2.1", "cn": "2.5.4.3", "o": "2.5.4.10", "counterSignature": "1.2.840.113549.1.9.6",
    "signingTime": "1.2.840.113549.1.9.5", "data": "1.2.840.113549.1.7.1",
}


def length(n):
    if n < 0x80:
        return bytes([n])
    b = n.to_bytes((n.bit_length() + 7) // 8, "big")
    return bytes([0x80 | len(b)]) + b


def tlv(tag, body):
    return bytes([tag]) + length(len(body)) + body


def oid(dotted):
    a = [int(x) for x in dotted.split(".")]
    out = bytearray([a[0] * 40 + a[1]])
    for x in a[2:]:
        chunk = [x & 0x7F]
        x >>= 7
        while x:
            chunk.append(0x80 | (x & 0x7F))
            x >>= 7
        out += bytes(reversed(chunk))
    return tlv(0x06, bytes(out))


def integer(n):
    b = n.to_bytes(max(1, (n.bit_length() + 8) // 8), "big")
    return tlv(0x02, b)


def seq(*parts):
    return tlv(0x30, b"".join(parts))


def set_(*parts):
    return tlv(0x31, b"".join(parts))


def null():
    return b"\x05\x00"


def octets(b):
    return tlv(0x04, b)


def utf8(s):
    return tlv(0x0C, s.encode())


def utctime(s):
    return tlv(0x17, s.encode())


def alg(name):
    return seq(oid(OID[name]), null())


def name(cn):
    return seq(set_(seq(oid(OID["o"]), utf8("Test Org"))), set_(seq(oid(OID["cn"]), utf8(cn))))


def is_probable_prime(n, rng):
    if n < 4:
        return n in (2, 3)
    if n % 2 == 0:
        return False
    d, r = n - 1, 0
    while d % 2 == 0:
        d //= 2
        r += 1
    for _ in range(24):
        a = rng.randrange(2, n - 1)
        x = pow(a, d, n)
        if x in (1, n - 1):
            continue
        for _ in range(r - 1):
            x = pow(x, 2, n)
            if x == n - 1:
                break
        else:
            return False
    return True


def gen_prime(bits, rng):
    while True:
        c = rng.getrandbits(bits) | (1 << (bits - 1)) | 1
        if is_probable_prime(c, rng):
            return c


def make_key(seed=117):
    rng = random.Random(seed)
    e = 65537
    while True:
        p, q = gen_prime(512, rng), gen_prime(512, rng)
        if p == q:
            continue
        phi = (p - 1) * (q - 1)
        if phi % e == 0:
            continue
        return p * q, e, pow(e, -1, phi)


HASHES = {"sha256": (hashlib.sha256, "sha256"), "sha1": (hashlib.sha1, "sha1")}


def rsa_sign(n, d, hname, digest_bytes):
    k = (n.bit_length() + 7) // 8
    t = seq(alg(hname), octets(digest_bytes))
    em = b"\x00\x01" + b"\xff" * (k - len(t) - 3) + b"\x00" + t
    return pow(int.from_bytes(em, "big"), d, n).to_bytes(k, "big")


def build_pe(text_fill=0xC3):
    """PE32+: headers 0x400, .text 0x200 at 0x400, .data 0x200 at 0x600; no certificate table yet. Returns (bytes, offsets)."""
    size = 0x800
    b = bytearray(size)
    b[0:2] = b"MZ"
    lf = 0x80
    struct.pack_into("<I", b, 0x3C, lf)
    b[lf:lf + 4] = b"PE\0\0"
    struct.pack_into("<HHIIIHH", b, lf + 4, 0x8664, 2, 0x5F000000, 0, 0, 240, 0x2022)
    opt = lf + 24
    struct.pack_into("<H", b, opt, 0x20B)
    struct.pack_into("<I", b, opt + 16, 0x1000)                # AddressOfEntryPoint
    struct.pack_into("<Q", b, opt + 24, 0x140000000)           # ImageBase
    struct.pack_into("<II", b, opt + 32, 0x1000, 0x200)        # SectionAlignment, FileAlignment
    struct.pack_into("<IIII", b, opt + 56, 0x3000, 0x400, 0x1234ABCD, 0)  # SizeOfImage, SizeOfHeaders, CheckSum, Subsystem(+DllChar)
    struct.pack_into("<H", b, opt + 68, 3)
    struct.pack_into("<I", b, opt + 108, 16)                   # NumberOfRvaAndSizes
    sec = opt + 240
    for i, (nm, va, raw, ch) in enumerate([(b".text", 0x1000, 0x400, 0x60000020), (b".data", 0x2000, 0x600, 0xC0000040)]):
        o = sec + 40 * i
        b[o:o + len(nm)] = nm
        struct.pack_into("<IIIIIIHHI", b, o + 8, 0x200, va, 0x200, raw, 0, 0, 0, 0, ch)
    for i in range(0x400, 0x600):
        b[i] = text_fill if i % 7 else 0x90
    for i in range(0x600, 0x800):
        b[i] = (i * 31) & 0xFF
    return bytes(b), {"checksum": opt + 64, "certdir": opt + 112 + 4 * 8, "text": 0x400, "end": size}


def authenticode_hash(data, offs, hctor):
    """Independent formulation: hash the whole file with the three excluded ranges masked out (the certificate table is the tail)."""
    cert_off, cert_size = struct.unpack_from("<II", data, offs["certdir"])
    end = len(data) - cert_size if cert_size else len(data)
    skip = [(offs["checksum"], offs["checksum"] + 4), (offs["certdir"], offs["certdir"] + 8)]
    h = hctor()
    pos = 0
    for a, c in skip:
        h.update(data[pos:a])
        pos = c
    h.update(data[pos:end])
    return h.digest()


def build(out, hname="sha256", tamper="none", unsigned=False, timestamp=False, cn="Test Signer Ltd"):
    n, e, d = make_key()
    hctor = HASHES[hname][0]
    pe, offs = build_pe()
    if unsigned:
        open(out, "wb").write(pe)
        return
    serial = 0x0102030405060708090A0B0C0D0E0F10
    issuer = name("Test Root")
    subject = name(cn)
    validity = seq(utctime("260728000000Z"), utctime("271021235959Z"))
    spki = seq(seq(oid(OID["rsa"]), null()), tlv(0x03, b"\x00" + seq(integer(n), integer(e))))
    tbs = seq(tlv(0xA0, integer(2)), integer(serial), seq(oid(OID["sha256WithRSA"]), null()), issuer, validity, subject, spki)
    cert = seq(tbs, seq(oid(OID["sha256WithRSA"]), null()), tlv(0x03, b"\x00" + rsa_sign(n, d, "sha256", hashlib.sha256(tbs).digest())))
    # the PE with a zero certificate-table entry is hashed first; the table is appended after signing, and the directory entry is filled in
    pe_digest = authenticode_hash(pe, offs, hctor)
    spc_data = seq(oid(OID["spcPEImageData"]), seq(tlv(0x03, b"\x00\x00"), tlv(0xA0, tlv(0xA2, tlv(0x80, b"")))))
    spc = seq(spc_data, seq(alg(hname), octets(pe_digest)))
    # the content octets of the SEQUENCE: the bytes after its tag and length
    lenlen = 1 if spc[1] < 0x80 else 1 + (spc[1] & 0x7F)
    content_octets = spc[1 + lenlen:]
    md = hctor(content_octets).digest()
    ctype_oid = OID["spcIndirect"] if tamper != "contenttype" else OID["data"]
    attrs = [seq(oid(OID["contentType"]), set_(oid(ctype_oid))), seq(oid(OID["messageDigest"]), set_(octets(md if tamper != "messagedigest" else bytes(len(md)))))]
    attrs_body = b"".join(attrs)
    signed_for_hash = tlv(0x31, attrs_body)
    sig = rsa_sign(n, d, HASHES[hname][1], hctor(signed_for_hash).digest())
    if tamper == "signature":
        sig = bytes([sig[0] ^ 0x01]) + sig[1:]
    signer_serial = serial if tamper != "signer" else serial + 1
    unsigned_attrs = b""
    if timestamp:
        counter = seq(integer(1), seq(issuer, integer(serial)), alg(hname), tlv(0xA0, seq(oid(OID["signingTime"]), set_(utctime("260929143153Z")))), alg("sha256WithRSA"), octets(b"\x00" * 8))
        unsigned_attrs = tlv(0xA1, seq(oid(OID["counterSignature"]), set_(counter)))
    sigalg = OID["rsa"] if tamper != "sigalg" else "1.2.840.113549.1.1.5"   # sha1WithRSA while digestAlgorithm says otherwise (sha256), or the same (sha1) when --hash sha1
    signer_info = seq(integer(1), seq(issuer, integer(signer_serial)), alg(hname), tlv(0xA0, attrs_body), seq(oid(sigalg), null()), octets(sig), unsigned_attrs)
    signed_data = seq(integer(1), set_(alg(hname)), seq(oid(OID["spcIndirect"]), tlv(0xA0, spc)), tlv(0xA0, cert), set_(signer_info))
    p7 = seq(oid(OID["signedData"]), tlv(0xA0, signed_data))
    win = struct.pack("<IHH", 8 + len(p7), 0x0200, 0x0002) + p7
    win += b"\x00" * ((-len(win)) % 8)
    pe_arr = bytearray(pe) + win
    struct.pack_into("<II", pe_arr, offs["certdir"], len(pe), len(win))
    if tamper == "text":
        pe_arr[offs["text"] + 5] ^= 0x40
    open(out, "wb").write(bytes(pe_arr))


def main(argv):
    if not argv:
        print(__doc__)
        return 2
    out = argv[0]
    kw = {"hname": "sha256", "tamper": "none", "unsigned": False, "timestamp": False, "cn": "Test Signer Ltd"}
    i = 1
    while i < len(argv):
        a = argv[i]
        if a == "--hash":
            kw["hname"] = argv[i + 1]
            i += 2
        elif a == "--tamper":
            kw["tamper"] = argv[i + 1]
            i += 2
        elif a == "--cn":
            kw["cn"] = argv[i + 1]
            i += 2
        elif a == "--unsigned":
            kw["unsigned"] = True
            i += 1
        elif a == "--timestamp":
            kw["timestamp"] = True
            i += 1
        else:
            print("unknown option", a)
            return 2
    build(out, **kw)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
