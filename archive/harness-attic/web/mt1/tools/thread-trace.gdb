# mt1 (decision 0173), premise check (a): every kernel32!CreateThread the lifted engine makes (native -O0 build of the same generated C), with the caller VA, the start address, the 8 qwords of the start
# parameter and every Delphi class name that can be read from them (an object's first qword is its VMT; vmtClassName = VMT - 0x18 is a ShortString). g3_snapshot marks the script's snapshots.
set pagination off
set confirm off
python
import gdb
CT_OFF, R_OFF = 0x1c9dcb30, 0x1de3e580      # `nm` of the same binary: T fle_imp_CreateThread, B R
inf = gdb.selected_inferior()
def rd(a, n=8):
    try:
        return int.from_bytes(bytes(inf.read_memory(a, n)), 'little')
    except Exception:
        return None
def cls(obj):
    vmt = rd(obj)
    if vmt is None or not (0x400000 < vmt < 0x4000000):
        return None
    cn = rd(vmt - 0x18)
    if cn is None or not (0x400000 < cn < 0x4000000):
        return None
    ln = rd(cn, 1)
    if not ln or ln > 80:
        return None
    s = bytes(inf.read_memory(cn + 1, ln))
    return s.decode('latin1') if all(32 <= c < 127 for c in s) else None
class CT(gdb.Breakpoint):
    n = 0
    def stop(self):
        CT.n += 1
        base = int(gdb.parse_and_eval('(unsigned long)&fle_imp_CreateThread')) - CT_OFF
        R = base + R_OFF
        reg = lambda i: rd(R + 8 * i)
        rsp, start, param = reg(4), reg(8), reg(9)
        flags = rd(rsp + 0x28, 4)
        rec = [rd(param + 8 * k) for k in range(8)] if param else []
        names = []
        for k, q in enumerate(rec):
            if q and q > 0x10000:
                c = cls(q)
                if c: names.append('rec[%d]=%s' % (k, c))
        c0 = cls(param) if param else None
        if c0: names.append('param=%s' % c0)
        o = rec[1] if len(rec) > 1 else None
        vm = rd(o) if o else None
        cnp = rd(vm - 0x18) if vm else None
        gdb.write('DBG obj=%s vmt=%s clsname_ptr=%s\n' % (hex(o or 0), hex(vm or 0), hex(cnp or 0)))
        gdb.write('CT #%d caller=%#x start=%#x param=%#x flags=%#x rec=%s %s\n' % (CT.n, rd(rsp), start, param, flags, ','.join('%x' % (q or 0) for q in rec), ' '.join(names)))
        return False
class SN(gdb.Breakpoint):
    def stop(self):
        s = gdb.parse_and_eval('(char*)$rdi')
        gdb.write('SNAP %s\n' % s.string())
        return False
CT('fle_imp_CreateThread')
SN('g3_snapshot')
end
run
quit
