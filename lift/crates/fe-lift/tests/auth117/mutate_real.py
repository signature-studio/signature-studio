#!/usr/bin/env python3
"""Writes a copy of a signed PE with ONE byte changed (tests of tools/authenticode_check.py, decision 0117). Standard library only.

usage: mutate_real.py IN OUT text|signature|certificate|timestamp

  text         one byte of the first section's raw data (the Authenticode digest changes; the signature is untouched)
  signature    one byte of the SignerInfo signature value inside the PKCS#7 (the digest of the PE is untouched; the signature no longer verifies)
  certificate  one byte inside the signer certificate's TBS part (the digest of the PE is untouched; the signature is untouched, the certificate changes)
  timestamp    one byte of the timestamp (unsigned attribute) payload: neither check looks at it
The offsets are found with the tool's own reader (tools/authenticode_check.py); only WHICH byte is chosen depends on it, and any byte of that region would do.
"""
import os
import shutil
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "..", "..", "..", "..", "tools"))
import authenticode_check as ac  # noqa: E402


def main(argv):
    src, dst, what = argv
    data = bytearray(open(src, "rb").read())
    pe = ac.PE(bytes(data))
    p7, dw_len, rev, typ = pe.certificate()
    base = pe.cert_off + 8
    top = ac.parse_one(p7)
    sd = top.children()[1].children()[0].children()
    idx = 3
    certs_node = None
    while idx < len(sd) and sd[idx].cls == ac.CONTEXT:
        if sd[idx].tag == 0:
            certs_node = sd[idx]
        idx += 1
    si = sd[idx].children()[0].children()
    if what == "text":
        roff, rsz = sorted(x for x in pe.sections if x[1] > 0)[0]
        off = roff + min(0x100, rsz - 1)
    elif what == "signature":
        sig = [c for c in si if c.is_(ac.UNIVERSAL, 4)][0]
        off = base + sig.vstart + 3
    elif what == "certificate":
        c0 = certs_node.children()[0]
        tbs = c0.children()[0]
        subj = tbs.children()[5 if tbs.children()[0].is_(ac.CONTEXT, 0) else 4]
        off = base + subj.vend - 2
    elif what == "timestamp":
        un = [c for c in si if c.is_(ac.CONTEXT, 1)][0]
        off = base + un.vend - 4
    else:
        print("unknown mutation", what)
        return 2
    data[off] ^= 0x01
    with open(dst, "wb") as f:
        f.write(data)
    print(off)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
