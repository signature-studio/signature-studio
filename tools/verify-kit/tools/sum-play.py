#!/usr/bin/env python3
# sum-play.py DIR...: the play-busy scenario (tools5 ops cpuWindow, audioDetail): the engine Worker's busy ms per second and the page's audio statistics
import json, sys
for d in sys.argv[1:]:
    r = json.load(open(d + '/report.json'))[0]
    print('##', d, 'ok', bool(r.get('ok')), 'error', r.get('error'), 'opErrors', r.get('opErrors'))
    for m in r.get('measures') or []:
        print(f"  cpu {m['name']:24s} wall {m['wallS']:5.1f} s  Worker {m['workerMsPerS']:5d} ms/s  main {m['mainMsPerS']:5d} ms/s  load {m['load']}")
    for a in r.get('audios') or []:
        x = a.get('detail') or {}
        print(f"  audio {a['name']:14s} load {a['load']} status {x.get('status')} playingSeconds {x.get('playingSeconds')} underrunQuanta {x.get('underrunQuanta')} episodes {x.get('episodes')} longestRunQuanta {x.get('longestRunQuanta')} fillFrames {x.get('fillFrames')} fillMin {x.get('fillMinFrames')} engineGapFrames {x.get('engineGapFrames')} lateWrites {x.get('engineLateWrites')}")
