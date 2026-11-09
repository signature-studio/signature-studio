# counts calls of fle_mxcsr_load (ldmxcsr) after the first note event and prints the distinct values loaded (the value is read from the guest address in rdi)
import gdb, collections, struct
cnt = collections.Counter(); n = [0]
class B(gdb.Breakpoint):
    def stop(self):
        a = int(gdb.parse_and_eval('$rdi'))
        v = struct.unpack('<I', bytes(gdb.selected_inferior().read_memory(a, 4)))[0]
        cnt[v] += 1; n[0] += 1
        return n[0] >= 200000
bb = B('fle_mxcsr_load'); bb.enabled = False
class E(gdb.Breakpoint):
    def stop(self):
        bb.enabled = True
        return False
E('vst_ev_note')
gdb.execute('run')
print('MXCSRW total ldmxcsr', n[0])
for k, v in cnt.most_common(): print('MXCSRW value %#x count %d' % (k, v))
gdb.execute('kill')
