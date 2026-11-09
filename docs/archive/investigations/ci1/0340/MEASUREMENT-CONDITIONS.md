# G2/G3/G6 measurement conditions

- Machine/browser: this Linux VM, Chrome 156 headless and Firefox 158, simulated input, real lifted engine. No owner hardware.
- Counted only if start load <= 8, I/O pressure avg10 = 0, swap in/out = 0, MemAvailable >= 8 GB (script gate 8192 MB), no other automated Chrome, no Wine. free/vmstat/load/I/O are recorded before every run in `out/steps.log` and in each run's `gate.txt`.
- G2 neutral benchmark: `tools/scn-lat-ff.json` (the 0312 rec0/snd13 code path as used here: File menu pointer event, Mixer-header pointermove/button drag with the page recording the input in LatencyTracker, Space keys, and File menu during playback). Latency records are dumped by op `latDump`. The table reports:
  - mouse queue insertion: `tInject - tEvent` (and `tInject - tPage`) for pointermove kind=1 records in the Mixer-drag section;
  - user-visible T: first answered frame - `tPage` for button-down kind=2 (File click) and key-down kind=5 (Esc / Space), in idle and playback sections. p50/p90/p99/max.
  - Direction: the gate is RT-D3 median T - E13 median T <= +10 ms, using the same record filter in the two runs. This is code-identification for DRAG, not overall driver input variation.
- G3 present rate: existing O19 round1/round2/C tables (E13 and RT-A) use `audioSnap.presents` deltas over the play window. The D3 row uses a 60 s shown song window with `deskSnap` (deskStats presents) at play-start and play-end; this is the same definition (presenter putImageData completions per second), not the song's full 300 s.
- G6 Firefox: the existing gpu8 scenario files are used with the same driver (WebDriver), reported as start, About window/About text, Japanese glyph rendering, and playback; no unsupported stop is accepted as a pass.
