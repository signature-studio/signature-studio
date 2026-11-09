#!/usr/bin/env python3
"""s3a-plan.py [ROUNDS=2]  (S3a sweep, ci1 0326): writes out/s3a-plan.tsv, the order of the runs. 5 conditions x 2 scenarios = 10 runs a round.
A condition is a server (port) with an assets dir; the page is the same for all (page-d0447902 = the production page E13): e13 = the production bundle (the control), w2 w4 w8 w16 = the B2 bundle with FLE_WORKERS = n.
The order of the conditions differs from round to round (a balanced rotation, so that a drift of the machine does not belong to one condition); both scenarios of a condition run one after the other (song first in odd rounds, solo12 first in even ones).
A line: id, round, condition, scenario, port, assets, status (todo|done). The runner (run-s3a.sh) takes the first todo line and marks it done when its report.json exists."""
import sys, os
K = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
R = int(sys.argv[1]) if len(sys.argv) > 1 else 2
COND = {'e13': (5053, 'fl-engine-ci-0363b050'), 'w2': (5072, 'b2w2'), 'w4': (5074, 'b2w4'), 'w8': (5078, 'b2w8'), 'w16': (5079, 'b2w16')}
ORDERS = [['e13', 'w8', 'w2', 'w16', 'w4'], ['w4', 'w16', 'w2', 'w8', 'e13'], ['w16', 'e13', 'w4', 'w2', 'w8'], ['w2', 'w4', 'w8', 'e13', 'w16']]
rows = []
for r in range(1, R + 1):
    for c in ORDERS[(r - 1) % len(ORDERS)]:
        for s in (['song', 'solo12'] if r % 2 == 1 else ['solo12', 'song']):
            rows.append((f's3a-r{r}-{c}-{s}', r, c, s, COND[c][0], COND[c][1], 'todo'))
os.makedirs(K + '/out', exist_ok=True)
with open(K + '/out/s3a-plan.tsv', 'w') as f:
    f.write('id\tround\tcondition\tscenario\tport\tassets\tstatus\n')
    for x in rows: f.write('\t'.join(map(str, x)) + '\n')
print(len(rows), 'runs in', R, 'rounds ->', K + '/out/s3a-plan.tsv')
