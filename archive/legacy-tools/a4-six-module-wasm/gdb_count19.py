# measurement only: counting breakpoints (never stop, never write) on the 19 functions that wasm compiled at -O1, and controls.
import gdb
names = [l.split('\t')[1] for l in open('/tmp/opencode/wasm6/results/logs/force/o1/list-all.tsv').read().split('\n') if l.strip()]
ctl = ['f_37caca8', 'f_1376820', 'f_1e9c1f0', 'f_62b35f0', 'f_4b580f0', 'f_489e584', 'f_1d0f9c0', 'f_600e090', 'f_4120c0']
class Cnt(gdb.Breakpoint):
    def __init__(self, spec):
        super().__init__(spec, internal=True)
        self.n = 0
    def stop(self):
        self.n += 1
        return False
gdb.execute('set pagination off')
gdb.execute('set confirm off')
bps = {n: Cnt(n) for n in names + ctl}
gdb.execute('run')
print('---- hit counts (19 functions compiled at -O1, then controls)')
for n in names + ctl:
    print(n, bps[n].n)
