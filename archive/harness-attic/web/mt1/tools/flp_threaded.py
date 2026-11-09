#!/usr/bin/env python3
"""mt1 (decision 0173), premise check for the driving scheme: which plugin instances of an FLP have FL's per-plugin 'Allow threaded processing' switched on? STATIC reading of the FLP bytes; nothing is run.
  event 212 (PluginWrapper), data offset 16, u16 flags: bit 4 Generator, bit 5 SmartDisable, bit 6 ThreadedProcessing  (wrapper menu 'Allow threaded processing')
  event 213 (PluginParams) of a VST/VST3 wrapper: u32 version, then records [u32 cid][u64 size][data]; cid 2 = flags record: data offset 9 u32 flags, offset 13 u32 flags2; flags bit 12 AllowThreaded
     (wrapper settings 'Processing > Allow threaded processing'), bit 29 AllowSmartDisable
  (bit positions: PyFLP, a third-party GPL reverse engineering, pyflp/plugin.py fetched 2026-11-03 - not Image-Line documentation; the records()/build() reader is docs/investigations/reference-project/tools/flp_model.py)
usage: flp_threaded.py FLP OUT_TSV [path to flp_model.py's directory]"""
import struct, sys
flp, out = sys.argv[1], sys.argv[2]
sys.path.insert(0, sys.argv[3] if len(sys.argv) > 3 else '.')
sys.dont_write_bytecode = True
import flp_model as F

P = F.build(open(flp, 'rb').read())
OFF = {id(e.d): e.off for e in P['events']}   # file offset of the event (its id byte) by identity of the data object

def u16(d, o): return struct.unpack_from('<H', d, o)[0]
def u32(d, o): return struct.unpack_from('<I', d, o)[0]

def describe(where, ev212, ev213, ev201, ev203=None):
    row = {'where': where, 'internal': F.text(ev201) if ev201 else '', 'name': F.text(ev203) if ev203 else ''}
    row['w212_len'] = len(ev212) if ev212 else None
    row['ev212_file_off'] = OFF.get(id(ev212), '') if ev212 else ''
    row['ev213_file_off'] = OFF.get(id(ev213), '') if ev213 else ''
    if ev212 and len(ev212) >= 18:
        f = u16(ev212, 16)
        row.update(w212_flags='%#06x' % f, w_generator=(f >> 4) & 1, w_smart_disable=(f >> 5) & 1, w_threaded=(f >> 6) & 1)
    else:
        row.update(w212_flags='', w_generator='', w_smart_disable='', w_threaded='')
    row.update(format='native/other', plugin='', vst_flags='', vst_allow_threaded='', vst_allow_smart_disable='', cids='')
    if ev213:
        pp = F.parse_plugin_params(ev213)
        if pp:
            row['cids'] = ','.join(str(c) for c in sorted(pp['records']))
            row['plugin'] = pp['name']
            path = pp['file'].lower()
            row['format'] = 'VST3' if '.vst3' in path else ('VST2' if path.endswith('.dll') else ('CLAP' if '.clap' in path else 'non-VST plugin data'))
            is_vst = all(c in pp['records'] for c in (50, 53, 55))   # a VST/VST3 wrapper (a FLEX channel has cids 1-4 only: cid 2 there is not a flags record)
            if is_vst and 2 in pp['records']:
                o, s = pp['records'][2]
                if s >= 13:
                    fl = u32(ev213, o + 9)
                    row.update(vst_flags='%#010x' % fl, vst_allow_threaded=(fl >> 12) & 1, vst_allow_smart_disable=(fl >> 29) & 1)
    return row

rows = []
for ch in P['channels']:
    if ch['wrapper'] is not None or ch['internal']:
        # channel events: ch['ev'] holds (id -> data) pairs in order; wrapper = event 212, data = event 213
        evs = {}
        for i in ch['ev']:
            e = P['events'][i]
            evs.setdefault(e.id, e.d)
        rows.append(describe('channel:%d' % ch['iid'], ch['wrapper'], evs.get(213), evs.get(201), evs.get(203)))
for n, ins in enumerate(P['inserts']):
    for k, sl in enumerate(ins.get('slots') or []):
        ev = sl.get('ev') or {}
        if 212 in ev or 213 in ev:
            rows.append(describe('insert:%d/slot:%d' % (n, k), ev.get(212), ev.get(213), ev.get(201), ev.get(203)))

hdr = ['where', 'internal', 'name', 'format', 'plugin', 'ev212_file_off', 'ev213_file_off', 'w212_len', 'w212_flags', 'w_generator', 'w_smart_disable', 'w_threaded', 'vst_flags', 'vst_allow_threaded', 'vst_allow_smart_disable', 'cids']
with open(out, 'w') as f:
    f.write('\t'.join(hdr) + '\n')
    for r in rows:
        f.write('\t'.join(str(r[h]) for h in hdr) + '\n')

from collections import Counter
inst = [r for r in rows if r['w212_len']]
print('plugin instances with an event 212:', len(inst))
print('by format:', dict(Counter(r['format'] for r in inst)))
print('wrapper ThreadedProcessing (212 bit 6) set:', sum(1 for r in inst if r['w_threaded'] == 1), 'of', len(inst), '| by format', dict(Counter(r['format'] for r in inst if r['w_threaded'] == 1)))
print('wrapper flag values (212 u16 at offset 16):', dict(Counter(r['w212_flags'] for r in inst)))
vst = [r for r in inst if r['format'] in ('VST3', 'VST2') and r['vst_flags']]
print('VST/VST3 instances with the 213 flags record (cid 2):', len(vst), 'of', len([r for r in inst if r['format'] in ('VST3', 'VST2')]))
print('  AllowThreaded (213 flags bit 12) set:', sum(1 for r in vst if r['vst_allow_threaded'] == 1))
print('  both wrapper-threaded and AllowThreaded:', sum(1 for r in vst if r['vst_allow_threaded'] == 1 and r['w_threaded'] == 1))
print('  by plugin:', dict(Counter((r['plugin'], r['w_threaded'], r['vst_allow_threaded']) for r in vst)))
