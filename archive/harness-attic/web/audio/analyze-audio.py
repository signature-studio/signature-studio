#!/usr/bin/env python3
"""Summary of one run of run-audio.mjs (decision 0139, PROVISIONAL): counters of the ring, the delay from the engine's Unlock to the AudioWorklet's read, the context's timing values, the pacing and the per-thread run times.
usage: analyze-audio.py <name>.json
The delay of a committed block = (time at which the worklet read the region's first frame) - (time of the Unlock). The worklet's read time is taken from its own samples (frame counter -> context time at the worklet) mapped to
the page's clock at the arrival of the sample, corrected by the audio time the message had spent in flight; the error of this mapping is a few ms (one quantum is 2.9 ms; the message goes through the main thread)."""
import json, sys, statistics as st

def main(path):
    d = json.load(open(path))
    r = d.get('result') or {}
    out = {'name': d['name'], 'completed': d['completed'], 'stalled': d.get('stalled'), 'wall_s': round(d['wallMs'] / 1000, 1), 'playing': r.get('playing'), 'ms_to_playing': round(r.get('tPlayAfterStartMs', 0), 1)}
    a, b = r.get('snap0'), r.get('snap1')
    if a and b:
        q = b['qPlaying'] - a['qPlaying']
        out['measure_s'] = round((b['perf'] - a['perf']) / 1000, 2)
        out['ctx_time_ratio'] = round((b['ctxTime'] - a['ctxTime']) / ((b['perf'] - a['perf']) / 1000), 4)
        out['playing_quanta'] = q
        out['underrun_quanta'] = b['urQuanta'] - a['urQuanta']
        out['underrun_frames'] = b['urFrames'] - a['urFrames']
        out['underrun_quanta_pct'] = round(100.0 * (b['urQuanta'] - a['urQuanta']) / q, 2) if q else None
        out['fill_min_frames'] = b['fillMin']; out['fill_max_frames'] = b['fillMax']
        out['published_frames'] = b['published'] - a['published']; out['publishes'] = b['publishes'] - a['publishes']; out['consumed_frames'] = b['rd'] - a['rd']
        out['behind_frames_at_end'] = b['rd'] - b['wr']
    out['ctxStart'] = r.get('ctxStart'); out['ctxEnd'] = r.get('ctxEnd'); out['max_page_event_loop_gap_ms'] = round(r.get('maxPageGapMs', 0), 1)
    w = (r.get('worker') or [None])[0]
    if w:
        out['pace'] = w.get('pace'); ws = w.get('state') or {}
        out['worker'] = {k: ws.get(k) for k in ('waits', 'publishes', 'skipped', 'gapFrames', 'maxWaitDrift')}
        unl = ws.get('unlocks') or []
        samples = r.get('samples') or []
        org = r.get('pageTimeOrigin')
        pts = []
        for s in samples:
            if s.get('state') != 2: continue
            ep = org + s['pagePerf'] - (s['pageCtxTime'] - s['time']) * 1000.0
            pts.append((s['rd'], ep))
        lat = []
        for u in unl:
            f = u['first'] % (1 << 32)
            for (r0, e0), (r1, e1) in zip(pts, pts[1:]):
                if r0 <= f < r1 and r1 > r0:
                    tc = e0 + (e1 - e0) * (f - r0) / (r1 - r0)
                    lat.append(tc - u['t'])
                    break
        if lat:
            out['unlock_to_worklet_read_ms'] = {'n': len(lat), 'min': round(min(lat), 1), 'median': round(st.median(lat), 1), 'mean': round(st.mean(lat), 1), 'max': round(max(lat), 1)}
    ps = r.get('pulses') or []
    if ps:
        last = ps[-1]
        out['engine_pulses'] = {'n': len(ps), 'last_real_ms': round(last['real']), 'last_virtual_ms_since_start': round((last['virt'] - 36000000000) / 1e4), 'mixer_thread': last['mixer'], 'unlock_compute': last['unlockCompute'],
                                'threads_run_ms(total,max,slices)': [[round(x[0]), round(x[1]), int(x[2])] for x in last['threads'][:12]]}
    print(json.dumps(out, indent=1))

main(sys.argv[1])
