#!/usr/bin/env python3
"""lat1: first use vs later repetitions of the same input (cold code paths): for each cell, the answer time of the first repetition's primary input against the median of the others. usage: firstuse.py RUN_JSON..."""
import json, sys, statistics
prim = {'a_click_step': 'button L down', 'b_hover': 'move', 'c_drag_knob': 'button L down', 'c_drag_fader': 'button L down', 'd_wheel_tempo': 'wheel', 'e_key_Space_start_stop': 'key down vk 0x20', 'e_key_F8_open_close': 'key down vk 0x77', 'f_open_file_menu': 'button L down', 'f_open_context_menu': 'button R down'}
print('| run | phase | cell | first rep: inject->first post (ms) | later reps median (min-max) | first rep total event->put | later median |')
print('|---|---|---|---:|---|---:|---:|')
for f in sys.argv[1:]:
    r = json.load(open(f))
    for c in r['cells']:
        if c['cell'] not in prim or not c.get('inputs'): continue
        p = prim[c['cell']]
        rows = [i for i in c['inputs'] if i['desc'].startswith(p) and i.get('s2_inject_to_first_post') is not None]
        if c['cell'] == 'b_hover': rows = rows  # alternating FILE / mixer: compare the first two against later ones of the same target
        if len(rows) < 4: continue
        first = rows[0]; rest = rows[1:]
        if c['cell'] == 'b_hover': first = rows[0]; rest = rows[2::2]
        s2 = [x['s2_inject_to_first_post'] for x in rest]; tp = [x['total_event_to_put'] for x in rest if x['total_event_to_put'] is not None]
        print('| %s-r%s | %s | %s | %.1f | %.1f (%.1f-%.1f) | %.1f | %.1f |' % (r['config'], r['round'], c['phase'], c['cell'], first['s2_inject_to_first_post'], statistics.median(s2), min(s2), max(s2), first['total_event_to_put'] or -1, statistics.median(tp) if tp else -1))
