"""usage: dis.py FILE VA [COUNT] [BEFORE_BYTES]   disassemble the PE (read only) at a VA with capstone; BEFORE_BYTES (default 0) starts earlier."""
import sys, struct, capstone
fn = sys.argv[1]; va = int(sys.argv[2], 16); cnt = int(sys.argv[3]) if len(sys.argv) > 3 else 12
before = int(sys.argv[4]) if len(sys.argv) > 4 else 0
d = open(fn, 'rb').read()
lfa = struct.unpack_from('<I', d, 0x3c)[0]
nsec = struct.unpack_from('<H', d, lfa + 6)[0]; osz = struct.unpack_from('<H', d, lfa + 20)[0]
base = struct.unpack_from('<Q', d, lfa + 24 + 24)[0]
rva = va - base - before
for i in range(nsec):
    s = lfa + 24 + osz + 40 * i
    vs, sva, rs, rp = struct.unpack_from('<IIII', d, s + 8)
    if sva <= rva < sva + max(vs, rs):
        o = rp + rva - sva
        md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64); md.detail = False
        for k, ins in enumerate(md.disasm(d[o:o + 64 + cnt * 15], base + rva)):
            if k >= cnt: break
            print('%#x: %-30s %s %s' % (ins.address, ins.bytes.hex(), ins.mnemonic, ins.op_str))
        break
