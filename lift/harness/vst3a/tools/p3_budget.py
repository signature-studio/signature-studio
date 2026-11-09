#!/usr/bin/env python3
"""Decision 0177 (srm1): the P3 core-count arithmetic with MEASURED wasm block times of Serum 2 (the inputs are in the TSV given as the first argument; every output is a READING
[arithmetic on measured numbers and stated assumptions], not a measurement).
usage: p3_budget.py BLOCKS.tsv [OUT_TSV]
BLOCKS.tsv: label<TAB>state<TAB>ms_per_block (one row per measured case; labels: the build, e.g. wasm-O2; states 0 1 2 3 4 5 and chord4 [state 3, 4 notes]); a state with no row is skipped.
Scenarios (the reference project: 6 Serum 2 [states 0..5 = FLP channels 1,2,3,28,33,37], OTT x2, ~12 other VST3, one VST2):
  all6   : every Serum 2 instance holds one note (the heaviest time per instance, as in decision 0165's scenario E)
  max4   : the 4 lightest-to-heaviest simultaneously sounding instances (the project's maximum polyphony of instances, decision 0165's C/D)
Assumptions (S-srm1-others): OTT = OTT_MS per block (decision 0173: C-path wasm in a Worker 11.3 ms; decision 0164: 9.92 ms node); the ~12 other VST3 = OTHER_MS each (OTT-like; NOT measured);
silent Serum 2 = SILENT_MS (decision 0165, native 4.5 ms; wasm value measured here if given as label 'silent')."""
import sys, collections
P = 4096 / 44100 * 1000
OTT_MS = float(__import__('os').environ.get('OTT_MS', 11.3)); OTHER_MS = float(__import__('os').environ.get('OTHER_MS', OTT_MS)); N_OTHER = 12; N_OTT = 2
rows = collections.defaultdict(dict)
for l in open(sys.argv[1]):
    f = l.rstrip('\n').split('\t')
    if len(f) < 3 or f[0] == 'label' or f[0].startswith('#'):
        continue
    rows[f[0]][f[1]] = float(f[2])
out = []
def line(*a): out.append('\t'.join(str(x) for x in a))
for lab, st in rows.items():
    per = [st[s] for s in ('0', '1', '2', '3', '4', '5') if s in st]
    silent = st.get('silent', 4.5)
    if len(per) < 6:
        line(lab, 'incomplete: states present', ','.join(sorted(st)), '(all6 uses the states that are present; the missing ones are counted as the mean of the present ones)')
    mean = sum(per) / len(per) if per else 0
    all6 = sum(st.get(s, mean) for s in ('0', '1', '2', '3', '4', '5'))
    srt = sorted(st.get(s, mean) for s in ('0', '1', '2', '3', '4', '5'))
    fixed = N_OTT * OTT_MS + N_OTHER * OTHER_MS
    ordered = sorted(('0', '1', '2', '3', '4', '5'), key=lambda x: st.get(x, mean))
    sets = (('all6 sounding', ordered, []), ('4 heaviest sounding + 2 silent', ordered[2:], ordered[:2]), ('4 lightest sounding + 2 silent', ordered[:4], ordered[4:]), ('no Serum sounding (6 silent)', [], ordered))
    for scen, loud, quiet in sets:
        serum = sum(st.get(x, mean) for x in loud) + len(quiet) * silent
        tot = serum + fixed
        hv = max([st.get(x, mean) for x in loud] or [silent])
        line(lab, scen, f'serum_ms={serum:.0f}', f'ott+others_ms={fixed:.0f}', f'plugin_ms={tot:.0f}', f'cores_needed={tot / P:.2f}', f'heaviest_single_instance={hv:.0f} ms ({"OVER" if hv > P else "within"} the {P:.2f} ms period)',
             *[f'HC={h},e={e}:' + ('fits' if (tot / P <= max(0, h - 2) * e and hv <= P) else 'does NOT fit') for h in (4, 8, 16) for e in (0.6, 0.8, 1.0)])
line('')
line('capacity: (HC - 2) * e core-equivalents for plugin Workers (decision 0173: 2 logical threads reserved for the engine Worker and the browser; e = useful fraction of a logical thread)', 'HC=4', 'HC=8', 'HC=16')
for e in (0.6, 0.8, 1.0):
    line(f'e={e}', *[f'{max(0, h - 2) * e:.1f}' for h in (4, 8, 16)])
open(sys.argv[2] if len(sys.argv) > 2 else '/dev/stdout', 'w').write('\n'.join(out) + '\n')
if len(sys.argv) > 2: print('\n'.join(out))
