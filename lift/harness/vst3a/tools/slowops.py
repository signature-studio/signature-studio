# gdb -batch -x slowops.py : classifies the operands of the first N calls of fle_mx_do2_slow_p (the MXCSR-exact slow path of arithmetic) during a run (SysV: rdi=op, esi=f64, edx=scalar, rcx=d, r8=s)
import gdb, struct, collections
N = 4000
cnt = collections.Counter()
ops = collections.Counter()
class B(gdb.Breakpoint):
    def stop(self):
        inf = gdb.selected_inferior()
        f = lambda r: int(gdb.parse_and_eval('$' + r))
        op, f64, scalar, d, s = f('rdi') & 0xffffffff, f('rsi') & 0xffffffff, f('rdx') & 0xffffffff, f('rcx'), f('r8')
        dm = bytes(inf.read_memory(d, 16)); sm = bytes(inf.read_memory(s, 16))
        lanes = 1 if scalar else (2 if f64 else 4)
        ops[(op, 'f64' if f64 else 'f32', 'scalar' if scalar else 'packed')] += 1
        for name, mem in (('d', dm), ('s', sm)):
            for i in range(lanes):
                if f64:
                    u = struct.unpack_from('<Q', mem, 8 * i)[0]; e = (u >> 52) & 0x7ff; m = u & ((1 << 52) - 1); lo, hi = 1023 - 510, 1023 + 510
                else:
                    u = struct.unpack_from('<I', mem, 4 * i)[0]; e = (u >> 23) & 0xff; m = u & ((1 << 23) - 1); lo, hi = 127 - 62, 127 + 62
                full = 0x7ff if f64 else 0xff
                if e == 0 and m == 0: c = 'zero'
                elif e == 0: c = 'denormal'
                elif e == full: c = 'inf/nan'
                elif e < lo: c = 'tiny normal (below window)'
                elif e > hi: c = 'huge normal (above window)'
                else: c = 'in window'
                cnt[(('f64' if f64 else 'f32'), c)] += 1
        if sum(ops.values()) >= N:
            return True
        return False
bb = B('fle_mx_do2_slow_p')
bb.enabled = False
class E(gdb.Breakpoint):
    def stop(self):
        bb.enabled = True
        return False
E('vst_ev_note')
gdb.execute('run')
tot = sum(cnt.values())
print('SLOWOPS calls', sum(ops.values()))
for k, v in ops.most_common(12): print('SLOWOPS op', k, v)
for k, v in cnt.most_common(): print('SLOWOPS operand', k, v, '%.1f%%' % (100.0 * v / tot))
gdb.execute('kill')
