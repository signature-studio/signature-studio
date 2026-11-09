#!/usr/bin/env python3
# lat1: the G3 script of the native fork-probe measurement: every probe runs 2000 virtual ms from the same engine state (idle main window, or Space playback); the control probe has no input.
T = 2000
FILE = (18, 19); MIXER = (666, 672); STEP1 = (597, 144); FADER = (526, 800); TEMPO = (463, 19); CHAN = (522, 144)
def probe(name, lines, used):
    return ['probe %s' % name] + lines + ['advance %d' % (T - used), 'probe-end']
def probes(prefix):
    P = []
    P += probe(prefix + 'ctl', [], 0)
    P += probe(prefix + 'hover_file', ['move %d %d' % FILE], 0)
    P += probe(prefix + 'hover_mixer', ['move %d %d' % MIXER], 0)
    P += probe(prefix + 'click_step', ['move %d %d' % STEP1, 'advance 100', 'down L', 'advance 200', 'up L'], 300)
    d = ['move %d %d' % FADER, 'advance 100', 'down L', 'advance 30']
    for k in range(1, 13): d += ['move %d %d' % (FADER[0], FADER[1] - 6 * k), 'advance 16']
    d += ['up L']
    P += probe(prefix + 'drag_fader', d, 100 + 30 + 12 * 16)
    P += probe(prefix + 'wheel_tempo', ['move %d %d' % TEMPO, 'advance 100', 'wheel %d %d -120' % TEMPO], 100)
    P += probe(prefix + 'open_file_menu', ['move %d %d' % FILE, 'advance 100', 'down L', 'advance 200', 'up L'], 300)
    P += probe(prefix + 'open_ctx_menu', ['move %d %d' % CHAN, 'advance 100', 'down R', 'advance 200', 'up R'], 300)
    P += probe(prefix + 'key_F8', ['tap 0x77'], 0)
    P += probe(prefix + 'key_Space', ['tap 0x20'], 0)
    return P
L = ['snap wiz0', 'advance 3000', 'tap 13', 'advance 3000', 'snap e03', 'advance 3000', 'snap main-idle']
L += probes('i_')
L += ['tap 0x20', 'advance 3000', 'snap main-play']
L += probes('p_')
L += ['end']
open('nat/scripts/lat1.g3', 'w').write('\n'.join(L) + '\n')
print(len(L), 'lines')
