#!/usr/bin/env python3
# mk-r44d-scn.py: writes four R4-4d scenarios (mouse moves at 0, 5, 10 and 20 per second, 150 s of the P3 song each) from scn-p3-mixmove-300.json. The head (closeWizard to the first key Space) and the tail (the ops after the last audioSnap: cpuStop, audioDump, key Space, wait, expectRunning) are kept; the 2 s blocks are rebuilt, 75 of them. Standard library only. No arguments, no environment variables. Writes only scn-p3-mix0-150.json, scn-p3-mix5-150.json, scn-p3-mix10-150.json and scn-p3-mix20-150.json next to this script.
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, 'scn-p3-mixmove-300.json')
BLOCKS = 75
BLOCK_MS = 2000
X_LO, X_HI, Y = 420, 850, 800
MOVES_PER_BLOCK = {0: 0, 5: 10, 10: 20, 20: 40}


def sweep():
    n = 20
    up = [round(X_LO + i * (X_HI - X_LO) / (n - 1)) for i in range(n)]
    down = up[-2:0:-1]
    return up + down


def blocks(rate, seq):
    per = MOVES_PER_BLOCK[rate]
    ops = []
    k = 0
    for b in range(1, BLOCKS + 1):
        if per == 0:
            ops.append({'op': 'wait', 'ms': BLOCK_MS})
        else:
            for _ in range(per):
                ops.append({'op': 'move', 'x': seq[k % len(seq)], 'y': Y})
                ops.append({'op': 'wait', 'ms': BLOCK_MS // per})
                k += 1
        ops.append({'op': 'audioSnap', 'name': f'w{b}'})
    return ops


def main():
    src = json.load(open(SRC, encoding='utf-8'))[0]['ops']
    first_space = next(i for i, o in enumerate(src) if o.get('op') == 'key' and o.get('code') == 'Space')
    last_snap = max(i for i, o in enumerate(src) if o.get('op') == 'audioSnap')
    head = src[:first_space + 1]
    tail = src[last_snap + 1:]
    assert head[0] == {'op': 'closeWizard'}, head[0]
    assert tail[0] == {'op': 'cpuStop', 'name': 'p3-play'}, tail[0]
    seq = sweep()
    for rate in MOVES_PER_BLOCK:
        ops = head + blocks(rate, seq) + tail
        sc = {
            'name': f'p3-mix{rate}-150',
            'what': f'R4-4d: P3 song (as scn-p3-mixmove-300) for 150 s of playback from the play mark (75 blocks of 2 s), the Mixer shown, the mouse moved at {rate} per second during the measured playback (cpuStart to cpuStop). Each block ends with audioSnap wN.',
            'ops': ops,
        }
        path = os.path.join(HERE, f'scn-p3-mix{rate}-150.json')
        with open(path, 'w', encoding='utf-8') as f:
            json.dump([sc], f, ensure_ascii=False, separators=(',', ':'))
            f.write('\n')
        moves = sum(1 for o in ops if o.get('op') == 'move')
        snaps = sum(1 for o in ops if o.get('op') == 'audioSnap')
        print(f'{os.path.basename(path)} bytes={os.path.getsize(path)} ops={len(ops)} move={moves} audioSnap={snaps}')


if __name__ == '__main__':
    main()
