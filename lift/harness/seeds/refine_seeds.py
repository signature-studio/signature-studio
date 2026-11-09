"""measurement aid: keep the candidate seeds whose lifted closure has no undecodable node (the 'Node::Bad' lines of fe-lift).
usage: refine_seeds.py FE_LIFT DLL CANDIDATES.txt OUT.txt   (OUT must not exist). Prints one line per round."""
import re, subprocess, sys, collections
fe, dll, cand_path, out_path = sys.argv[1:5]
cand = [int(l, 16) for l in open(cand_path) if l.strip()]
seeds = set(cand)
fd = re.compile(r'^void f_([0-9a-f]+)\(void\)$')
call = re.compile(r'\bf_([0-9a-f]+)\(\)')
bad = re.compile(r'^L_[0-9a-f]+: \{ fle_unsupported\(UINT64_C\(0x[0-9a-f]+\)\); \}$')
def run(seeds):
    sp = out_path + '.round.tmp'
    with open(sp, 'w') as f:
        f.write(''.join('0x%x\n' % v for v in sorted(seeds)))
    p = subprocess.run([fe, 'lift', dll, '/dev/stdout', '--pdata', '--seeds', sp], capture_output=True)
    txt = p.stdout.decode()
    return txt
rounds = 0
while True:
    rounds += 1
    txt = run(seeds)
    i = txt.rfind('closure_instructions   ')
    stats, c = txt[i:], txt[:i]
    lines = c.split('\n')
    funcs = {}
    cur = None
    for l in lines:
        m = fd.match(l)
        if m:
            cur = int(m.group(1), 16); funcs[cur] = [False, set()]
            continue
        if cur is None: continue
        if bad.match(l): funcs[cur][0] = True
        else:
            for m2 in call.finditer(l):
                funcs[cur][1].add(int(m2.group(1), 16))
    badf = {f for f, v in funcs.items() if v[0]}
    nb = int(re.search(r'^bad_nodes\s+(\d+)', stats, re.M).group(1))
    print('round', rounds, 'seeds', len(seeds), 'functions', len(funcs), 'bad_nodes', nb, 'bad functions', len(badf), flush=True)
    if not badf: break
    # seeds that reach a bad function
    rev = collections.defaultdict(set)
    for f, v in funcs.items():
        for t in v[1]: rev[t].add(f)
    reach_bad = set(badf)
    stack = list(badf)
    while stack:
        x = stack.pop()
        for y in rev.get(x, ()):
            if y not in reach_bad:
                reach_bad.add(y); stack.append(y)
    drop = seeds & reach_bad
    if not drop:
        print('bad functions are reached from .pdata starts only; stop'); break
    seeds -= drop
with open(out_path, 'x') as f:
    f.write(''.join('0x%x\n' % v for v in sorted(seeds)))
print('kept', len(seeds), 'of', len(cand), 'rounds', rounds)
