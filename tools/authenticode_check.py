#!/usr/bin/env python3
"""Offline Authenticode check of PE files (decision 0117, PROVISIONAL). Standard library only.

What it checks (reading the file; nothing is written except the files named by --tsv / --report):
  (a) the Authenticode digest of the PE (Windows Authenticode PE Signature Format: everything except the checksum field, the
      certificate-table directory entry and the certificate table itself, sections in PointerToRawData order, then the trailing
      data) equals the digest in the SpcIndirectDataContent of the embedded PKCS#7 signature;
  (b) the SignerInfo is consistent with that content and its signature verifies: contentType and messageDigest in the signed
      attributes (messageDigest = hash of the content octets of SpcIndirectDataContent), and the RSA PKCS#1 v1.5 signature over
      the DER of the signed attributes verifies with the public key of the signer certificate named by the SignerInfo
      (issuerAndSerialNumber) in the embedded certificates;
  (c) it REPORTS the signer certificate (subject, issuer, validity) and whether a timestamp (counter) signature is present.

What it does NOT check (written into every output as `unverified`): the certificate chain up to a trusted root, revocation, and
the trustworthiness of any time (the validity period is printed, never compared with a clock; a timestamp is reported as present,
its own signature is not verified). A row that says (a)=ok and (b)=ok therefore means: the digest and the signature value match
the embedded signer certificate. It does not mean that Windows would call the file trusted.

usage:
  authenticode_check.py [--tsv OUT.tsv] [--report OUT.md] FILE [FILE...]
exit: 0 every file has (a)=ok and (b)=ok, 1 at least one does not (the outputs are still written), 2 usage or I/O error.
"""
import hashlib
import hmac
import os
import struct
import sys

sys.dont_write_bytecode = True

UNVERIFIED = "chain,revocation,time"

OID_SIGNED_DATA = "1.2.840.113549.1.7.2"
OID_SPC_INDIRECT = "1.3.6.1.4.1.311.2.1.4"
OID_CONTENT_TYPE = "1.2.840.113549.1.9.3"
OID_MESSAGE_DIGEST = "1.2.840.113549.1.9.4"
OID_SIGNING_TIME = "1.2.840.113549.1.9.5"
OID_COUNTER_SIGNATURE = "1.2.840.113549.1.9.6"
OID_MS_RFC3161_TIMESTAMP = "1.3.6.1.4.1.311.3.3.1"
OID_MS_NESTED_SIGNATURE = "1.3.6.1.4.1.311.2.4.1"
OID_RSA_ENCRYPTION = "1.2.840.113549.1.1.1"
OID_COMMON_NAME = "2.5.4.3"

HASHES = {
    "1.3.14.3.2.26": ("sha1", hashlib.sha1),
    "2.16.840.1.101.3.4.2.1": ("sha256", hashlib.sha256),
    "2.16.840.1.101.3.4.2.2": ("sha384", hashlib.sha384),
    "2.16.840.1.101.3.4.2.3": ("sha512", hashlib.sha512),
}
RSA_SIG_OIDS = {
    "1.2.840.113549.1.1.1": None,
    "1.2.840.113549.1.1.5": "sha1",
    "1.2.840.113549.1.1.11": "sha256",
    "1.2.840.113549.1.1.12": "sha384",
    "1.2.840.113549.1.1.13": "sha512",
}
NAME_ATTRS = {
    "2.5.4.3": "CN", "2.5.4.6": "C", "2.5.4.7": "L", "2.5.4.8": "ST", "2.5.4.10": "O", "2.5.4.11": "OU",
    "1.2.840.113549.1.9.1": "E", "2.5.4.5": "serialNumber", "2.5.4.9": "street", "2.5.4.15": "businessCategory",
    "1.3.6.1.4.1.311.60.2.1.3": "jurisdictionC", "1.3.6.1.4.1.311.60.2.1.2": "jurisdictionST",
    "1.3.6.1.4.1.311.60.2.1.1": "jurisdictionL", "2.5.4.17": "postalCode",
}


class AuthError(Exception):
    """The file (or a structure in it) cannot be read as the checks need; the message says where."""


# ---------------------------------------------------------------- DER / BER reader ----------------------------------------------------

class Node:
    __slots__ = ("buf", "tag", "cls", "constructed", "start", "vstart", "vend", "end", "indef")

    def __init__(self, buf, tag, cls, constructed, start, vstart, vend, end, indef):
        self.buf, self.tag, self.cls, self.constructed = buf, tag, cls, constructed
        self.start, self.vstart, self.vend, self.end, self.indef = start, vstart, vend, end, indef

    @property
    def raw(self):
        return self.buf[self.start:self.end]

    @property
    def value(self):
        return self.buf[self.vstart:self.vend]

    def children(self):
        if not self.constructed:
            raise AuthError("DER: a primitive element has no children (tag 0x%02x at %d)" % (self.first_byte, self.start))
        out, p = [], self.vstart
        while p < self.vend:
            if self.indef and self.buf[p:p + 2] == b"\x00\x00":
                break
            n = read_node(self.buf, p, self.vend)
            out.append(n)
            p = n.end
        return out

    @property
    def first_byte(self):
        return self.buf[self.start]

    def is_(self, cls, tag):
        return self.cls == cls and self.tag == tag


UNIVERSAL, APPLICATION, CONTEXT, PRIVATE = 0, 1, 2, 3


def read_node(buf, pos, limit):
    if pos >= limit:
        raise AuthError("DER: element starts at %d, past the end %d" % (pos, limit))
    b0 = buf[pos]
    cls, constructed, tag = b0 >> 6, bool(b0 & 0x20), b0 & 0x1F
    if tag == 0x1F:
        raise AuthError("DER: high tag numbers are not supported (at %d)" % pos)
    p = pos + 1
    if p >= limit:
        raise AuthError("DER: truncated length at %d" % pos)
    l0 = buf[p]
    p += 1
    if l0 < 0x80:
        length, indef = l0, False
    elif l0 == 0x80:
        if not constructed:
            raise AuthError("DER: indefinite length on a primitive element at %d" % pos)
        indef, length = True, None
    else:
        nb = l0 & 0x7F
        if nb > 4 or p + nb > limit:
            raise AuthError("DER: bad length at %d" % pos)
        length, indef = int.from_bytes(buf[p:p + nb], "big"), False
        p += nb
    if not indef:
        if p + length > limit:
            raise AuthError("DER: element at %d (length %d) runs past the end %d" % (pos, length, limit))
        return Node(buf, tag, cls, constructed, pos, p, p + length, p + length, False)
    # indefinite: children up to the end-of-contents marker
    q = p
    while True:
        if q + 2 > limit:
            raise AuthError("DER: indefinite-length element at %d has no end-of-contents" % pos)
        if buf[q:q + 2] == b"\x00\x00":
            return Node(buf, tag, cls, True, pos, p, q, q + 2, True)
        q = read_node(buf, q, limit).end


def parse_one(buf):
    n = read_node(buf, 0, len(buf))
    return n


def decode_oid(v):
    if not v:
        raise AuthError("DER: empty OID")
    arcs, acc = [], 0
    for i, b in enumerate(v):
        acc = (acc << 7) | (b & 0x7F)
        if not b & 0x80:
            arcs.append(acc)
            acc = 0
        elif i == len(v) - 1:
            raise AuthError("DER: truncated OID")
    first = arcs[0]
    head = [first // 40, first % 40] if first < 80 else [2, first - 80]
    return ".".join(str(x) for x in head + arcs[1:])


def encode_len(n):
    if n < 0x80:
        return bytes([n])
    b = n.to_bytes((n.bit_length() + 7) // 8, "big")
    return bytes([0x80 | len(b)]) + b


def tlv(tag, content):
    return bytes([tag]) + encode_len(len(content)) + content


def encode_oid(dotted):
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


def node_oid(n):
    if not n.is_(UNIVERSAL, 6):
        raise AuthError("DER: an OID was expected at %d" % n.start)
    return decode_oid(n.value)


def node_int(n):
    if not n.is_(UNIVERSAL, 2):
        raise AuthError("DER: an INTEGER was expected at %d" % n.start)
    return int.from_bytes(n.value, "big", signed=True)


def node_uint_bytes(n):
    """The content octets of an INTEGER without the sign padding (the value as an unsigned big-endian number)."""
    v = n.value
    return v.lstrip(b"\x00") or b"\x00"


def node_string(n):
    if n.cls != UNIVERSAL:
        raise AuthError("DER: a string was expected at %d" % n.start)
    v = n.value
    if n.tag in (0x0C, 0x13, 0x16, 0x12, 0x1A, 0x19, 0x1B):  # UTF8, Printable, IA5, Numeric, Visible, Graphic, General
        return v.decode("utf-8", "replace") if n.tag == 0x0C else v.decode("latin-1")
    if n.tag == 0x14:  # T61 (read as Latin-1)
        return v.decode("latin-1")
    if n.tag == 0x1E:  # BMPString
        return v.decode("utf-16-be", "replace")
    if n.tag == 0x1C:  # UniversalString
        return v.decode("utf-32-be", "replace")
    raise AuthError("DER: not a string type (tag 0x%02x) at %d" % (n.tag, n.start))


def node_time(n):
    """UTCTime / GeneralizedTime -> 'YYYY-MM-DDTHH:MM:SSZ' (only the 'Z' forms)."""
    s = n.value.decode("ascii", "replace")
    if n.is_(UNIVERSAL, 0x17):
        if len(s) == 13 and s.endswith("Z"):
            yy = int(s[0:2])
            year = (1900 + yy) if yy >= 50 else (2000 + yy)
            return "%04d-%s-%sT%s:%s:%sZ" % (year, s[2:4], s[4:6], s[6:8], s[8:10], s[10:12])
    elif n.is_(UNIVERSAL, 0x18):
        if len(s) == 15 and s.endswith("Z"):
            return "%s-%s-%sT%s:%s:%sZ" % (s[0:4], s[4:6], s[6:8], s[8:10], s[10:12], s[12:14])
    raise AuthError("DER: unsupported time %r at %d" % (s, n.start))


def parse_name(name_node):
    """A Name as a list of (short name or OID, text) in order, and the DER bytes."""
    out = []
    for rdn in name_node.children():
        for atv in rdn.children():
            kids = atv.children()
            oid = node_oid(kids[0])
            try:
                text = node_string(kids[1])
            except AuthError:
                text = kids[1].raw.hex()
            out.append((NAME_ATTRS.get(oid, oid), text))
    return out


def name_first(attrs, key):
    for k, v in attrs:
        if k == key:
            return v
    return None


def name_text(attrs):
    return ", ".join("%s=%s" % kv for kv in reversed(attrs))


# ---------------------------------------------------------------- certificates ----------------------------------------------------------

class Cert:
    def __init__(self, node):
        self.node = node
        self.der = node.raw
        tbs = node.children()[0]
        kids = tbs.children()
        i = 0
        if kids[0].is_(CONTEXT, 0):
            i = 1
        self.serial_node = kids[i]
        self.serial = node_uint_bytes(kids[i])
        self.issuer_node = kids[i + 2]
        self.issuer_der = kids[i + 2].raw
        self.issuer = parse_name(kids[i + 2])
        validity = kids[i + 3].children()
        self.not_before = node_time(validity[0])
        self.not_after = node_time(validity[1])
        self.subject_node = kids[i + 4]
        self.subject_der = kids[i + 4].raw
        self.subject = parse_name(kids[i + 4])
        spki = kids[i + 5].children()
        self.spki_alg = node_oid(spki[0].children()[0])
        self.rsa = None
        if self.spki_alg == OID_RSA_ENCRYPTION:
            bits = spki[1]
            if not bits.is_(UNIVERSAL, 3) or not bits.value or bits.value[0] != 0:
                raise AuthError("certificate: bad subjectPublicKey BIT STRING")
            key = read_node(bits.value, 1, len(bits.value)).children()
            self.rsa = (int.from_bytes(node_uint_bytes(key[0]), "big"), int.from_bytes(node_uint_bytes(key[1]), "big"))


def parse_cert_set(set_node):
    certs = []
    for c in set_node.children():
        if c.is_(UNIVERSAL, 16):
            try:
                certs.append(Cert(c))
            except (AuthError, IndexError) as e:
                raise AuthError("embedded certificate #%d: %s" % (len(certs), e))
    return certs


# ---------------------------------------------------------------- RSA PKCS#1 v1.5 (RFC 8017 section 8.2.2) ------------------------------

def rsa_pkcs1_v15_verify(n, e, signature, hash_name, digest):
    """True when `signature` is a valid RSASSA-PKCS1-v1_5 signature over `digest` (already hashed with `hash_name`)."""
    k = (n.bit_length() + 7) // 8
    if len(signature) != k:
        return False, "signature length %d != modulus length %d" % (len(signature), k)
    s = int.from_bytes(signature, "big")
    if s >= n:
        return False, "signature representative >= modulus"
    em = pow(s, e, n).to_bytes(k, "big")
    if em[0:2] != b"\x00\x01":
        return False, "encoded message does not start with 00 01"
    j = 2
    while j < len(em) and em[j] == 0xFF:
        j += 1
    if j - 2 < 8 or j >= len(em) or em[j] != 0x00:
        return False, "padding is not FF.. (at least 8) then 00"
    t = em[j + 1:]
    try:
        di = read_node(t, 0, len(t))
        if di.end != len(t) or not di.is_(UNIVERSAL, 16):
            return False, "DigestInfo has trailing bytes"
        alg, dg = di.children()
        oid = node_oid(alg.children()[0])
        if oid not in HASHES or HASHES[oid][0] != hash_name:
            return False, "DigestInfo algorithm %s is not %s" % (oid, hash_name)
        if not hmac.compare_digest(dg.value, digest):
            return False, "DigestInfo digest differs from the hash of the signed data"
    except AuthError as ex:
        return False, "DigestInfo: %s" % ex
    return True, "ok"


# ---------------------------------------------------------------- PE and the Authenticode digest ----------------------------------------

class PE:
    def __init__(self, data):
        self.data = data
        if len(data) < 0x40 or data[:2] != b"MZ":
            raise AuthError("not a PE file (no MZ header)")
        self.lfanew = struct.unpack_from("<I", data, 0x3C)[0]
        if self.lfanew + 24 > len(data) or data[self.lfanew:self.lfanew + 4] != b"PE\0\0":
            raise AuthError("not a PE file (no PE signature at e_lfanew)")
        self.nsec = struct.unpack_from("<H", data, self.lfanew + 6)[0]
        self.size_opt = struct.unpack_from("<H", data, self.lfanew + 20)[0]
        self.opt = self.lfanew + 24
        magic = struct.unpack_from("<H", data, self.opt)[0]
        if magic == 0x10B:
            dd_off = 96
        elif magic == 0x20B:
            dd_off = 112
        else:
            raise AuthError("unknown optional header magic 0x%x" % magic)
        self.magic = magic
        self.checksum_off = self.opt + 64
        self.size_of_headers = struct.unpack_from("<I", data, self.opt + 60)[0]
        ndd = struct.unpack_from("<I", data, self.opt + dd_off - 4)[0]
        if ndd < 5 or dd_off + 5 * 8 > self.size_opt:
            self.cert_dir_off = None
            self.cert_off = self.cert_size = 0
        else:
            self.cert_dir_off = self.opt + dd_off + 4 * 8
            self.cert_off, self.cert_size = struct.unpack_from("<II", data, self.cert_dir_off)
        sec0 = self.opt + self.size_opt
        if sec0 + 40 * self.nsec > len(data):
            raise AuthError("section table runs past the end of the file")
        self.sections = []
        for i in range(self.nsec):
            vsz, va, rsz, roff = struct.unpack_from("<IIII", data, sec0 + 40 * i + 8)
            self.sections.append((roff, rsz))

    def authenticode_hash(self, hash_ctor):
        d, n = self.data, len(self.data)
        if self.cert_dir_off is None or self.cert_size == 0:
            raise AuthError("the file has no certificate table")
        if self.cert_off + self.cert_size != n:
            raise AuthError("the certificate table (%d+%d) does not end at the end of the file (%d)" % (self.cert_off, self.cert_size, n))
        if self.size_of_headers > n or self.checksum_off + 4 > self.cert_dir_off or self.cert_dir_off + 8 > self.size_of_headers:
            raise AuthError("header layout is not the one the digest algorithm expects")
        h = hash_ctor()
        h.update(d[0:self.checksum_off])
        h.update(d[self.checksum_off + 4:self.cert_dir_off])
        h.update(d[self.cert_dir_off + 8:self.size_of_headers])
        total = self.size_of_headers
        for roff, rsz in sorted(self.sections, key=lambda s: s[0]):
            if rsz == 0:
                continue
            if roff + rsz > n:
                raise AuthError("a section's raw data runs past the end of the file")
            h.update(d[roff:roff + rsz])
            total += rsz
        if n > total:
            end = n - self.cert_size
            if end < total:
                raise AuthError("the certificate table overlaps the hashed sections")
            h.update(d[total:end])
        return h.digest()

    def certificate(self):
        """The PKCS#7 bytes of the first WIN_CERTIFICATE (type 2, PKCS_SIGNED_DATA) and the WIN_CERTIFICATE header fields."""
        d = self.data
        if self.cert_size == 0:
            raise AuthError("the file has no certificate table (not signed)")
        if self.cert_off + 8 > len(d) or self.cert_off % 8:
            raise AuthError("certificate table offset %d is outside the file or not 8-byte aligned" % self.cert_off)
        dw_len, rev, typ = struct.unpack_from("<IHH", d, self.cert_off)
        if typ != 2:
            raise AuthError("WIN_CERTIFICATE type 0x%x is not WIN_CERT_TYPE_PKCS_SIGNED_DATA" % typ)
        if dw_len < 8 or dw_len > self.cert_size:
            raise AuthError("WIN_CERTIFICATE dwLength %d is outside the table (%d)" % (dw_len, self.cert_size))
        return d[self.cert_off + 8:self.cert_off + dw_len], dw_len, rev, typ


# ---------------------------------------------------------------- PKCS#7 / Authenticode ------------------------------------------------

class SigResult:
    def __init__(self):
        self.which = "primary"
        self.hash_name = None
        self.pe_digest = None
        self.content_digest = None
        self.a_ok = False
        self.a_why = ""
        self.b_ok = False
        self.b_why = []
        self.signer = None
        self.cert_count = 0
        self.digest_alg_signer = None
        self.sig_alg = None
        self.timestamp = "none"
        self.timestamp_time = None
        self.nested = 0
        self.error = None


def _timestamp_info(unsigned_attrs):
    """('none' | 'countersignature' | 'rfc3161' | both joined by '+', time text or None). Presence only; no signature is verified."""
    kinds, when = [], None
    for attr in unsigned_attrs:
        k = attr.children()
        oid = node_oid(k[0])
        vals = k[1].children()
        if oid == OID_COUNTER_SIGNATURE:
            kinds.append("countersignature")
            for v in vals:
                try:
                    for a in _signer_info_attrs(v):
                        if a[0] == OID_SIGNING_TIME and when is None:
                            when = node_time(a[1].children()[0])
                except (AuthError, IndexError):
                    pass
        elif oid == OID_MS_RFC3161_TIMESTAMP:
            kinds.append("rfc3161")
            for v in vals:
                try:
                    when = when or _rfc3161_time(v)
                except (AuthError, IndexError):
                    pass
    return ("+".join(kinds) if kinds else "none"), when


def _signer_info_attrs(si_node):
    kids = si_node.children()
    for c in kids:
        if c.is_(CONTEXT, 0):
            return [(node_oid(a.children()[0]), a.children()[1]) for a in c.children()]
    return []


def _rfc3161_time(token_node):
    """genTime of the TSTInfo inside a timestamp token (a ContentInfo of SignedData)."""
    sd = token_node.children()[1].children()[0].children()
    eci = [c for c in sd if c.is_(UNIVERSAL, 16)][0]
    inner = eci.children()[1].children()[0]
    tst = read_node(inner.value, 0, len(inner.value)) if inner.is_(UNIVERSAL, 4) else inner
    tk = tst.children()
    return node_time(tk[4])


def check_signature(pe, pkcs7, which="primary"):
    r = SigResult()
    r.which = which
    try:
        top = parse_one(pkcs7)
        ci = top.children()
        if node_oid(ci[0]) != OID_SIGNED_DATA:
            raise AuthError("the outer ContentInfo is not SignedData")
        sd = ci[1].children()[0].children()
        # SignedData ::= version, digestAlgorithms, contentInfo, [0] certificates, [1] crls, signerInfos
        idx = 3
        certs_node = None
        while idx < len(sd) and sd[idx].cls == CONTEXT:
            if sd[idx].tag == 0:
                certs_node = sd[idx]
            idx += 1
        signer_infos = sd[idx]
        content_info = sd[2].children()
        if node_oid(content_info[0]) != OID_SPC_INDIRECT:
            raise AuthError("the encapsulated content type is not SpcIndirectDataContent (%s)" % node_oid(content_info[0]))
        content_seq = content_info[1].children()[0]
        if not content_seq.is_(UNIVERSAL, 16):
            raise AuthError("SpcIndirectDataContent is not a SEQUENCE")
        spc = content_seq.children()
        di = spc[1].children()
        dalg = node_oid(di[0].children()[0])
        if dalg not in HASHES:
            raise AuthError("PE digest algorithm %s is not supported" % dalg)
        r.hash_name = HASHES[dalg][0]
        r.content_digest = di[1].value
        certs = parse_cert_set(certs_node) if certs_node is not None else []
        r.cert_count = len(certs)
        sis = signer_infos.children()
        if len(sis) != 1:
            raise AuthError("SignedData has %d SignerInfos (exactly 1 is handled)" % len(sis))
        si = sis[0].children()
    except (AuthError, IndexError, ValueError) as e:
        r.error = "PKCS#7 structure: %s" % e
        return r

    # (a) the PE digest
    try:
        r.pe_digest = pe.authenticode_hash(HASHES[dalg][1])
        if hmac.compare_digest(r.pe_digest, r.content_digest):
            r.a_ok, r.a_why = True, "the %s Authenticode digest of the file equals the digest in SpcIndirectDataContent" % r.hash_name
        else:
            r.a_why = "the %s Authenticode digest of the file (%s) differs from the signed one (%s)" % (r.hash_name, r.pe_digest.hex(), r.content_digest.hex())
    except AuthError as e:
        r.a_why = str(e)

    # (b) the SignerInfo
    why = []
    try:
        sid = si[1].children()
        issuer_der, serial = sid[0].raw, node_uint_bytes(sid[1])
        signer = None
        for c in certs:
            if c.issuer_der == issuer_der and c.serial == serial:
                signer = c
                break
        if signer is None:
            raise AuthError("the signer certificate (issuerAndSerialNumber) is not among the %d embedded certificates" % len(certs))
        r.signer = signer
        sd_alg = node_oid(si[2].children()[0])
        if sd_alg not in HASHES:
            raise AuthError("SignerInfo digest algorithm %s is not supported" % sd_alg)
        r.digest_alg_signer = HASHES[sd_alg][0]
        k = 3
        if not si[k].is_(CONTEXT, 0):
            raise AuthError("the SignerInfo has no signed attributes")
        signed = si[k]
        attrs = {}
        for a in signed.children():
            ak = a.children()
            attrs.setdefault(node_oid(ak[0]), ak[1])
        k += 1
        sig_alg = node_oid(si[k].children()[0])
        r.sig_alg = sig_alg
        sig = si[k + 1]
        if not sig.is_(UNIVERSAL, 4):
            raise AuthError("the signature is not an OCTET STRING")
        unsigned = si[k + 2] if k + 2 < len(si) and si[k + 2].is_(CONTEXT, 1) else None
        ok_all = True
        ct = attrs.get(OID_CONTENT_TYPE)
        if ct is None or node_oid(ct.children()[0]) != OID_SPC_INDIRECT:
            why.append("signed attribute contentType is missing or is not SpcIndirectDataContent")
            ok_all = False
        md = attrs.get(OID_MESSAGE_DIGEST)
        content_octets = content_seq.value
        expect_md = HASHES[sd_alg][1](content_octets).digest()
        if md is None or not hmac.compare_digest(md.children()[0].value, expect_md):
            why.append("signed attribute messageDigest differs from the %s of the SpcIndirectDataContent content octets" % r.digest_alg_signer)
            ok_all = False
        if sig_alg not in RSA_SIG_OIDS or signer.rsa is None:
            why.append("signature algorithm %s / key type %s is not RSA PKCS#1 v1.5 (not supported by this tool)" % (sig_alg, signer.spki_alg))
            ok_all = False
        else:
            named = RSA_SIG_OIDS[sig_alg]
            if named is not None and named != r.digest_alg_signer:
                why.append("signatureAlgorithm names %s but digestAlgorithm is %s" % (named, r.digest_alg_signer))
                ok_all = False
            signed_der = b"\x31" + signed.raw[1:]  # the signed attributes are signed as a SET OF, not as [0] IMPLICIT
            dg = HASHES[sd_alg][1](signed_der).digest()
            good, msg = rsa_pkcs1_v15_verify(signer.rsa[0], signer.rsa[1], sig.value, r.digest_alg_signer, dg)
            if not good:
                why.append("RSA signature over the signed attributes: " + msg)
                ok_all = False
        r.b_ok = ok_all
        if unsigned is not None:
            uattrs = unsigned.children()
            r.timestamp, r.timestamp_time = _timestamp_info(uattrs)
            r.nested = sum(1 for a in uattrs if node_oid(a.children()[0]) == OID_MS_NESTED_SIGNATURE)
    except (AuthError, IndexError, ValueError) as e:
        why.append(str(e))
        r.b_ok = False
    r.b_why = why or ["contentType and messageDigest match; the RSA signature verifies with the signer certificate's public key"]
    return r


def nested_pkcs7(pkcs7):
    """The PKCS#7 blobs carried in the unsigned attribute 1.3.6.1.4.1.311.2.4.1 of the first SignerInfo (a second, e.g. SHA-256, signature)."""
    out = []
    try:
        sd = parse_one(pkcs7).children()[1].children()[0].children()
        idx = 3
        while idx < len(sd) and sd[idx].cls == CONTEXT:
            idx += 1
        si = sd[idx].children()[0].children()
        for c in si:
            if c.is_(CONTEXT, 1):
                for a in c.children():
                    k = a.children()
                    if node_oid(k[0]) == OID_MS_NESTED_SIGNATURE:
                        for v in k[1].children():
                            out.append(v.raw)
    except (AuthError, IndexError, ValueError):
        pass
    return out


class FileResult:
    def __init__(self, path, data):
        self.path = path
        self.basename = os.path.basename(path)
        self.sha256 = hashlib.sha256(data).hexdigest()
        self.size = len(data)
        self.sigs = []
        self.error = None
        self.win_cert = None


def check_bytes(path, data):
    fr = FileResult(path, data)
    try:
        pe = PE(data)
        pkcs7, dw_len, rev, typ = pe.certificate()
        fr.win_cert = (pe.cert_off, pe.cert_size, dw_len, rev, typ, len(pkcs7))
        fr.sigs.append(check_signature(pe, pkcs7, "primary"))
        for k, nb in enumerate(nested_pkcs7(pkcs7)):
            fr.sigs.append(check_signature(pe, nb, "nested #%d" % (k + 1)))
    except AuthError as e:
        fr.error = str(e)
    return fr


def check_file(path):
    with open(path, "rb") as f:
        data = f.read()
    return check_bytes(path, data)


# ---------------------------------------------------------------- outputs --------------------------------------------------------------

TSV_COLUMNS = ["file", "file_sha256", "authenticode_digest", "a_digest", "b_signature", "signer_cn", "issuer_cn", "not_before", "not_after",
               "timestamp", "unverified", "signer_issuer_der", "signer_serial"]


def _cell(s):
    s = "-" if s is None or s == "" else str(s)
    if "\t" in s or "\n" in s:
        raise AuthError("a TSV cell holds a tab or newline: %r" % s)
    return s


def tsv_row(fr):
    if fr.error or not fr.sigs:
        return [_cell(fr.basename), fr.sha256, "-", "ng", "ng", "-", "-", "-", "-", "none", UNVERIFIED, "-", "-"]
    s = fr.sigs[0]
    sg = s.signer
    return [_cell(fr.basename), fr.sha256,
            "%s:%s" % (s.hash_name, (s.pe_digest or b"").hex()) if s.pe_digest else "-",
            "ok" if s.a_ok else "ng", "ok" if s.b_ok else "ng",
            _cell(name_first(sg.subject, "CN") if sg else None), _cell(name_first(sg.issuer, "CN") if sg else None),
            _cell(sg.not_before if sg else None), _cell(sg.not_after if sg else None),
            _cell(s.timestamp + (("@" + s.timestamp_time) if s.timestamp_time else "")), UNVERIFIED,
            sg.issuer_der.hex() if sg else "-", sg.serial.hex() if sg else "-"]


def row_ok(row):
    return row[3] == "ok" and row[4] == "ok"


def tsv_text(results):
    lines = [
        "# authenticode-claims.tsv (decision 0117, PROVISIONAL). Written by tools/authenticode_check.py from the files named in the first column; do not edit by hand.",
        "# A row is 'established' only when a_digest and b_signature are both 'ok'. 'ok' means: the Authenticode digest of the file equals the signed digest, and the",
        "# SignerInfo signature verifies with the embedded signer certificate (offline, standard library only). It does NOT mean the file is trusted:",
        "# unverified = %s (no certificate chain to a root, no revocation check, no trusted time; validity dates and timestamp presence are reported only)." % UNVERIFIED,
        "# The last two columns are the signer certificate's issuer Name (DER, hex) and serial number (hex), read from the same certificate (for the crypt32 answers of the runtime).",
        "# " + "\t".join(TSV_COLUMNS),
    ]
    for fr in results:
        lines.append("\t".join(tsv_row(fr)))
    return "\n".join(lines) + "\n"


def report_text(results):
    out = ["# Authenticode check (tools/authenticode_check.py, decision 0117, PROVISIONAL)", "",
           "Offline: digest of the PE and signature value against the embedded signer certificate. **Not checked: %s.**" % UNVERIFIED.replace(",", ", "), ""]
    for fr in results:
        out += ["## %s" % fr.basename, "", "- path: `%s`" % fr.path, "- size: %d bytes, SHA-256 of the file: `%s`" % (fr.size, fr.sha256)]
        if fr.error:
            out += ["- **not checked**: %s" % fr.error, ""]
            continue
        off, size, dw_len, rev, typ, p7len = fr.win_cert
        out.append("- certificate table: offset %d, size %d; WIN_CERTIFICATE dwLength %d, revision 0x%04x, type 0x%04x; PKCS#7 %d bytes" % (off, size, dw_len, rev, typ, p7len))
        for s in fr.sigs:
            out += ["", "### signature: %s" % s.which, ""]
            if s.error:
                out += ["- **not checked**: %s" % s.error]
                continue
            out += ["- PE digest algorithm: %s; digest computed from the file: `%s`" % (s.hash_name, s.pe_digest.hex() if s.pe_digest else "-"),
                    "- digest in SpcIndirectDataContent: `%s`" % s.content_digest.hex(),
                    "- **(a) digest: %s** - %s" % ("ok" if s.a_ok else "ng", s.a_why),
                    "- **(b) signature: %s** - %s" % ("ok" if s.b_ok else "ng", "; ".join(s.b_why)),
                    "- SignerInfo digestAlgorithm: %s; signatureAlgorithm OID: %s; embedded certificates: %d" % (s.digest_alg_signer, s.sig_alg, s.cert_count)]
            sg = s.signer
            if sg:
                out += ["- signer certificate subject: `%s`" % name_text(sg.subject), "- signer certificate issuer: `%s`" % name_text(sg.issuer),
                        "- serial number: `%s`" % sg.serial.hex(), "- validity: notBefore %s, notAfter %s (printed; not compared with any clock)" % (sg.not_before, sg.not_after),
                        "- RSA modulus: %d bits" % sg.rsa[0].bit_length() if sg.rsa else "- public key: not RSA"]
            out += ["- timestamp: %s%s (presence reported; the timestamp's own signature and time are NOT verified)" % (s.timestamp, (", signing time " + s.timestamp_time) if s.timestamp_time else ""),
                    "- nested signatures carried: %d" % s.nested]
        out.append("")
    return "\n".join(out) + "\n"


def main(argv):
    tsv = report = None
    files = []
    i = 0
    while i < len(argv):
        a = argv[i]
        if a in ("--tsv", "--report"):
            if i + 1 >= len(argv):
                print("authenticode_check: %s needs a file name" % a, file=sys.stderr)
                return 2
            if a == "--tsv":
                tsv = argv[i + 1]
            else:
                report = argv[i + 1]
            i += 2
        elif a.startswith("--"):
            print("authenticode_check: unknown option %s" % a, file=sys.stderr)
            return 2
        else:
            files.append(a)
            i += 1
    if not files:
        print(__doc__, file=sys.stderr)
        return 2
    results = []
    for p in files:
        try:
            results.append(check_file(p))
        except OSError as e:
            print("authenticode_check: cannot read %s: %s" % (p, e), file=sys.stderr)
            return 2
    try:
        text_tsv = tsv_text(results)
    except AuthError as e:
        print("authenticode_check: %s" % e, file=sys.stderr)
        return 2
    for fr in results:
        row = tsv_row(fr)
        print("%s  sha256=%s  (a)=%s (b)=%s  signer CN=%s  issuer CN=%s  %s..%s  timestamp=%s  unverified=%s%s" % (
            row[0], row[1][:16], row[3], row[4], row[5], row[6], row[7], row[8], row[9], row[10], ("  ERROR: " + fr.error) if fr.error else ""))
    for path, text in ((tsv, text_tsv), (report, report_text(results))):
        if path:
            try:
                with open(path, "w", encoding="utf-8", newline="\n") as f:
                    f.write(text)
            except OSError as e:
                print("authenticode_check: cannot write %s: %s" % (path, e), file=sys.stderr)
                return 2
    return 0 if all(row_ok(tsv_row(fr)) for fr in results) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
