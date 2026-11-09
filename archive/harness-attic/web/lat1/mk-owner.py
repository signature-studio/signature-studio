#!/usr/bin/env python3
"""lat1: who owns the time? medians of the stages grouped by owner. usage: mk-owner.py summary.json CONFIG > table.md"""
import json, sys
S = json.load(open(sys.argv[1])); cfg = sys.argv[2]
order = ['a_click_step', 'b_hover', 'c_drag_knob', 'c_drag_fader', 'd_wheel_tempo', 'e_key_Space_start_stop', 'e_key_F8_open_close', 'f_open_file_menu', 'f_open_context_menu']
def m(s, k):
    v = s[k]['p50']; return 0.0 if v is None else v
def f(x): return '-' if x is None else ('%.1f' % x if x < 100 else '%.0f' % x)
print('| 入力 | 状態 | n | ブラウザ(入口)S0 | app S0b+S1+S1b+S3 | ゲスト S2 | ブラウザ(出口)S4a | 合計 イベント→rAF | S2 が占める割合 | 図(1 字 = 2 ms。B 入口 / a app / G ゲスト / D 出口) |\n|---|---|---:|---:|---:|---:|---:|---:|---:|---|')
for phase in ('idle', 'play'):
    for s in sorted([x for x in S if x['config'] == cfg and x['phase'] == phase and x['role'] in ('down', 'hover-move', 'drag-move', 'wheel', 'Space(start)', 'Space(stop)', 'F8(open)', 'Escape(close)', 'open(down)')], key=lambda x: (order.index(x['cell']) if x['cell'] in order else 99, x['role'])):
        if s['nResp'] == 0: continue
        a = m(s, 's0_evt_to_handler'); b = m(s, 's0b_handler_to_page_send') + m(s, 's1_page_to_worker_recv') + m(s, 's1b_worker_recv_to_inject') + m(s, 's3a_post_to_page_recv') + m(s, 's3b_recv_to_put')
        g = m(s, 's2_inject_to_first_post'); d = m(s, 's4a_put_to_raf'); tot = s['total_event_to_raf']['p50']
        bar = 'B' * round(a / 2) + 'a' * round(b / 2) + 'G' * round(g / 2) + 'D' * round(d / 2)
        print('| %s %s | %s | %d/%d | %s | %s | %s | %s | %s | %s | `%s` |' % (s['cell'], s['role'], 'idle' if phase == 'idle' else 'play', s['nResp'], s['n'], f(a), f(b), f(g), f(d), f(tot), ('%.0f%%' % (100 * g / tot)) if tot else '-', bar[:70]))
