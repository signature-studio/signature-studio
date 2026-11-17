# pw1 (decision 0186): the lifted Serum 2 (hybrid wasm) in K Chrome Workers, driven in real time

New files only. The measurement kit of `mt1/` (decision 0173) and `srm1/` (decision 0177), extended with a real-time engine that releases one block every 4096 / 44100 s = 92.879 ms. **What runs is the lifted third-party plug-in Serum2.vst3
(and the lifted OTT of decision 0164), not the FL engine**, on vst3a's stand-ins (failing Direct2D/DirectWrite, installer data on a virtual volume). The 32 KiB exchanged per block are stand-ins; the lifted `process()` is real.

| file | what |
|---|---|
| `lib.mjs` | localhost server with COOP/COEP (`/coi/` = `lift/harness/web/`, `/coi/data/` = `$PW1_DATA`), isolated headless Chrome that can be started under `taskset -c .. nice -n ..`, per-thread CPU and run-queue delay from `/proc/<pid>/task/<tid>/schedstat` (ns), per-CPU busy time from `/proc/stat`, PSS |
| `plugin-worker.js` | one plug-in instance in one Worker (`kind` serum or ott), paced by a SAB slot: the unchanged harness `main()` calls the CPU clock before and after each `process()`; the first call waits for the engine's block, the second answers. Records enter / request seen / process start / process end per block |
| `engine-worker.js` | the "engine": every period it publishes a 32 KiB block to every plug-in (fan-out) and waits for all answers; a late block is not skipped (the next one starts at once) |
| `pw1.html` | `?serum=0,1,3,4,5,3&ott=12&ottmod=c&blocks=650&warm=1&warmms=25000&settle=3000&mod=h123`; entries `<state>[c|s]` (c = 4-note chord, s = silent). `warm=1`: throw-away instances (one per distinct entry) run free for `warmms` ms of wall time (or `warmblocks` blocks when warmms=0) on the same `WebAssembly.Module` first, then `settle` ms of rest; `warm=0`: cold start |
| `run-pw1.mjs` | one fresh Chrome per repetition (`hc=4` -> `taskset -c 0-3`, `hc=8` -> `0-7`, `hc=16` -> none; `nice -n 5`), CPU over the paced window, late counts, per-plug-in statistics, `results/<name>.json` (raw) and `results/<tag>-rows.jsonl` |
| `ring.html`, `ring-plugin-worker.js`, `ring-engine-worker.js`, `run-ring.mjs`, `matrix4.sh` | the decoupled variant: every instance renders on its own schedule (phase-staggered, golden-ratio phases) into a 4-slot output ring; a reader takes block b at T0 + (b+1+lead) * period and counts the instances that have not produced it (`stagger=0`: all phases 0). Same harness, Module, warm-up |
| `matrix.sh`, `matrix2.sh`, `matrix3.sh` | the measurement matrix in three parts (K=1/4/6 Serum, Serum + 12 OTT interleaved over HC=4/8/16, physical-core placements, cold start, OTT alone) |
| `summarize.py`, `estimate.py`, `mdtable.py`, `backfill.py` | rows -> TSV (late counts, process times, cores, loads) / measured cores against the formula of decisions 0173/0181 / Markdown tables / adds ownOverrun to old rows |
| `attribute.py` | which plug-in answered last in the late blocks |
| `simulate.py`, `simulate_scale.py`, `simulate_mix.py`, `simulate_node.py`, `simulate_stagger.py` | **calculations, not measurements**: the measured per-block process times replayed on c ideal cores (processor sharing, closed-loop engine), optionally with one plug-in class scaled or demands composed from several runs |

```
export PW1_OUT=<work dir>  PW1_DATA=$PW1_OUT/data  PW1_RESULTS=$PW1_OUT/results
# $PW1_DATA: h123.wasm (the hybrid Serum build), serum.vst3 (rebased PE), seed-small.tsv, content/, state/, ott.vst3, ott-seed.tsv, ott-c.wasm (and ott-mx9.wasm)
cd lift/harness/web/pw1
nice -n 15 node run-pw1.mjs serum=0,1,3,4,5,3 hc=4 blocks=650 reps=3 tag=s6-hc4
REPS=3 BLOCKS=650 bash matrix.sh s6 s4
python3 summarize.py out.tsv $PW1_RESULTS/*-rows.jsonl; python3 estimate.py est.tsv $PW1_RESULTS/*-rows.jsonl
```
Late = the answer of block b arrives after (b+1) * 92.879 ms from the start of the schedule (A1); A2 = after (b+2) * period (one block of output buffering). The piano (state 2) needs the JSPI thread build, which this host does not have: excluded.
