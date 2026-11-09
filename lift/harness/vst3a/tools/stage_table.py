#!/usr/bin/env python3
"""Decision 0177 (srm1): start-up stage times (CPU ms as printed by the harness) from full harness outputs: file name = label.
columns: label, DllMain(LoadLibraryW), factory..initialize (cumulative from the start), setState(processor), setComponentState, controller setState, DllMain..setActive (total), second instance init.
usage: stage_table.py OUT.tsv FILE.full.txt..."""
import re, sys, os
rows = []
for fn in sys.argv[2:]:
    t = open(fn, errors='replace').read()
    g = lambda pat: (re.search(pat, t) or [None, ''])[1]
    rows.append([os.path.basename(os.path.dirname(fn)) + '/' + os.path.basename(fn).replace('.full.txt', ''),
                 g(r'LoadLibraryW\([^\n]*?DllMain ran inside\)\s+cpu ([\d.]+) ms'), g(r'factory\.\.initialize cpu ([\d.]+) ms'), g(r'IComponent::setState\([^\n]*?cpu ([\d.]+) ms'),
                 g(r'IEditController::setComponentState[^\n]*?cpu ([\d.]+) ms'), g(r'IEditController::setState\([^\n]*?cpu ([\d.]+) ms'), g(r'DllMain\.\.setActive cpu ([\d.]+) ms'), g(r'second_instance_init_cpu_ms=([\d.]+)')])
with open(sys.argv[1], 'w') as f:
    f.write('label\tdllmain_ms\tfactory_initialize_cum_ms\tsetState_proc_ms\tsetComponentState_ms\tsetState_ctrl_ms\tdllmain_to_setActive_ms\tsecond_instance_init_ms\n')
    for r in rows: f.write('\t'.join(r) + '\n')
print(open(sys.argv[1]).read())
