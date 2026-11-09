#!/usr/bin/env python3
"""mt1 (decision 0173): the P3 budget as arithmetic on cited numbers. EVERY OUTPUT IS A READING (推測), not a measurement.
Inputs and where they come from are printed first. usage: budget.py [OUT_TSV]"""
import sys, itertools
P = 4096 / 44100 * 1000                       # 92.88 ms
# measured by vst3a (decision 0165), native -O2 lifted C, one held note C4, thread CPU, load 12-33 (SMT inflates 10-60%)
serum_before = {0: 202, 1: 123, 2: 823, 3: 190, 4: 184, 5: 258}
# rz1 (decision 0172, PM's message): state 3 190 -> 33 ms (5.8x). S-mt1-rz1: the same factor for every state (NOT measured); the older interim figure was 3.4x
rz1 = {'rz1 5.8x': 5.8, 'rz1 3.4x': 3.4}
# decision 0156: OTT C path wasm -O2 11.6 ms vs native -O2 9.7 ms (x1.2, range 1.0-1.4); decision 0164: direct wasm backend 3.0x faster than the C path (MXCSR window), 6.8x without
wasm_over_native = {'wasm C path x1.0': 1.0, 'wasm C path x1.4': 1.4}
direct = {'C path (no gain)': 1.0, 'direct backend /3.0': 3.0}
# S-mt1-others: the other VST3 (Vital, Kilohearts x8, Ozone Imager x2, Spectral Gate, OTT x2 = 14) and the VST2: OTT measured 3.4 ms (this session, hybrid, load 13-16); others unmeasured -> 2..10 ms each
others = {'low': 14 * 3.4 * 0.0 + 14 * 2.0, 'mid': 14 * 5.0, 'high': 14 * 10.0}
# S-mt1-fl: FL core mixer + 12 native FX in the engine Worker (not parallel): unmeasured, 15..40 ms/block (ui1: the empty mixer chain alone is 0.19 ms/cycle)
flcore = {'low': 15.0, 'mid': 25.0, 'high': 40.0}
# capacity: logical threads minus 2 reserved (engine Worker ~1.0-1.2 cores per ui1 decision 0166; page main + browser + GPU + AudioWorklet ~0.5), times e = useful fraction of a logical thread (SMT, other load); the scaling test saw CPU/Worker +11..15% at N>=2
e_scen = {'e=0.6': 0.6, 'e=0.8': 0.8, 'e=1.0': 1.0}
out = []
def line(*a): out.append('\t'.join(str(x) for x in a))
line('scenario', 'serum_total_ms_per_block(6 inst)', 'heaviest_instance_ms(piano)', 'plugin_work_ms(serum+others)', 'cores_needed_plugins(period=%.2f)' % P)
rows = []
for (rn, rf), (wn, wf), (dn, df) in itertools.product(rz1.items(), wasm_over_native.items(), direct.items()):
    states = {k: v / rf * wf / df for k, v in serum_before.items()}
    tot = sum(states.values()); heavy = max(states.values())
    for on, ov in others.items():
        work = tot + ov
        rows.append((rn, wn, dn, on, tot, heavy, work))
        line(f'{rn} | {wn} | {dn} | others {on}', f'{tot:.0f}', f'{heavy:.0f}', f'{work:.0f}', f'{work / P:.2f}')
line('')
line('capacity (core-equivalents for plugin Workers) = (hardwareConcurrency - 2) * e', *['HC=%d' % h for h in (4, 8, 16)])
for en, ev in e_scen.items():
    line(en, *[f'{max(0, h - 2) * ev:.1f}' for h in (4, 8, 16)])
line('')
line('verdict for the reference project (6 Serum + 14 other + VST2) per hardwareConcurrency, central assumptions (rz1 5.8x, wasm x1.2, others mid, e=0.8): needed cores vs capacity, and how many Serum instances of the AVERAGE state fit after the others')
for dn, df in direct.items():
    st = {k: v / 5.8 * 1.2 / df for k, v in serum_before.items()}
    avg = sum(st.values()) / 6; heavy = max(st.values()); tot = sum(st.values())
    need = (tot + others['mid']) / P
    for h in (4, 8, 16):
        cap = max(0, h - 2) * 0.8
        fit = max(0, int((cap - others['mid'] / P) / (avg / P)))
        line(f'{dn}', f'HC={h}', f'capacity {cap:.1f}', f'needed {need:.2f}', 'fits' if cap >= need and heavy < P else 'does not fit', f'avg Serum {avg:.0f} ms/block', f'heaviest {heavy:.0f} ms ({"<" if heavy < P else ">"} period)', f'Serum instances that fit after others: {fit}')
open(sys.argv[1] if len(sys.argv) > 1 else '/dev/stdout', 'w').write('\n'.join(out) + '\n')
print('\n'.join(out))
