# Event loop / input / drawing / audio prototypes for the browser (docs/investigations/browser-event-loop.md)

Measurement aids. Nothing here is part of the runtime or of the lifted code; **no FL Studio code and no 389 MB wasm is used**. The only wasm files are
tiny test programs built from `mb/mb.c` and from generated synthetic code (`mb/synth.mjs`); they are rebuilt by the scripts and are not kept here (they go to
`lift/out-web2/`). Everything runs in a headless Chrome with a throw-away profile (`lift/out-web2/chrome-profile/<name>`, removed afterwards), a localhost-only
server and CDP over node's built-in WebSocket; `../cdp.mjs` is only imported (read-only). No network access, no npm.

## Layout

| file | what |
|---|---|
| `lib.mjs`, `run-page.mjs` | local server (`/coi/<file>` adds COOP same-origin + COEP require-corp + CORP same-origin, `/plain/<file>` adds nothing, `/build/<file>` = `lift/out-web2/mb/`), Chrome launcher, `withBrowser`; `run-page.mjs page=<html> coi=0\|1 name=<result> q=<query>` runs any page that sets `window.__done` / `window.__result` |
| `ring.js`, `ma-worker.js`, `ma-main.js`, `ma.html`, `run-ma.mjs`, `pm-echo.js` | **M-A**: SharedArrayBuffer event ring (SPSC, plus a coalescing "latest mouse position" slot), a Worker that blocks in `Atomics.wait` (GetMessage stand-in), round trips, timeouts, busy-worker and busy-page tests |
| `mb/mb.c`, `mb/build.sh`, `mb-worker.js`, `mb-main.js`, `mb.html` | **M-B**: C GetMessage loop whose import is a Promise; built three ways (sync baseline, `-sASYNCIFY`, `-sJSPI`) at `-O0` and `-O2`; round trips, suspend/resume micro benchmark, recursion depth |
| `mb/synth.mjs`, `mb/synth-measure.sh`, `synthrun-*`, `st-*` | synthetic lifted-shaped C (guest registers in globals, `call` = push + C call, indirect call through a sorted table): Asyncify/JSPI code size, link time/RSS, run-time cost; `setTimeout` vs `Atomics.waitAsync` lateness |
| `mb/cps-build.sh`, `cpsrun-*` | **M-C'** (not M-C): unwind-by-return / rewind-by-resume-switch made by the generator on the same synthetic code, with a checksum comparison against the non-suspending build |
| `draw-*.js`, `draw.html`, `run-draw.mjs` | pixel buffer in a Worker's memory -> `ImageData` -> `putImageData` / `transferToImageBitmap`; what is actually displayed for four Worker loop styles (tiny CDP screenshots + screencast frame counts) |
| `audio-*.js`, `audio.html` | Worker -> SharedArrayBuffer ring -> `AudioWorklet` on a real-time `AudioContext`; under-runs vs ring size and producer stalls |
| `input-*.js`, `input.html`, `run-input.mjs`, `vk-map.js` | PointerEvent -> ring -> Worker latency with CDP-injected input, probes for drop / IME / clipboard / fullscreen / pointer lock / cursor, `KeyboardEvent.code` -> VK table (not verified) |
| `summarize.mjs` | prints the numbers of the report from `results/*.json` |
| `results/` | raw results (small JSON / TSV); `*.json` carry the Chrome version, flags, loadavg before/after, console messages |

## Reproduce (about 25 minutes, most of it the synthetic builds)

```
export TMPDIR=<repo>/lift/out-web2/tmp          # mkdir -p lift/out-web2/{tmp,mb,runs,chrome-profile}
cd lift/harness/web/eventloop
bash mb/build.sh                                  # mb_{sync,asyncify,jspi}_{O0,O2}.wasm -> lift/out-web2/mb/ (emsdk 6.0.12 under /opt/emsdk, EM_FROZEN_CACHE=1)
node run-ma.mjs coi=1 name=ma-coi                 # M-A, full set (~2.5 min); repeat with tests=env,rtt,pp name=ma-rtt-r1 ...
node run-ma.mjs coi=0 tests=env,rtt name=ma-plain # without COOP/COEP: SharedArrayBuffer is undefined
node run-page.mjs page=mb.html coi=1 name=mb-coi q='n=10000&opts=O0,O2'
node run-page.mjs page=mb.html coi=1 name=mb-bench-coi q='tests=env,bench&opts=O0,O2'
node run-page.mjs page=mb.html coi=0 name=mb-depth q='tests=env,depth&opts=O0,O2'
for F in 500 2000 8000; do for O in O0 O2; do bash mb/synth-measure.sh $F $O; done; done          # Asyncify size / link cost (F=8000 -O2 compiles for ~7 minutes)
BUDGET=1 KEEP_DIR=<repo>/lift/out-web2/mb bash mb/synth-measure.sh 2000 O0 <repo>/lift/out-web2/synth-rt   # run-time builds (same for O2)
node run-page.mjs page=synthrun.html coi=0 name=synthrun-r1 q='budget=3000000&reps=7'
bash mb/cps-build.sh 2000 O0; bash mb/cps-build.sh 2000 O2; node run-page.mjs page=cpsrun.html coi=0 name=cpsrun-F2000-micro q='f=2000&budget=3000000&reps=7&wait=micro'
node run-page.mjs page=st.html coi=1 name=settimeout-worker
node run-draw.mjs name=draw-coi n=200
node run-page.mjs page=audio.html coi=1 name=audio-coi q='matrix=1' flags="--autoplay-policy=no-user-gesture-required"
node run-input.mjs name=input-coi moves=2000
node summarize.mjs
```

Timings depend on the machine and on what else runs on it (loadavg is recorded in every result); the reported figures come from an AMD Ryzen 9 9950X (16 cores),
Linux 7.0, Chrome 156.0.8078.47 (V8 15.6.75.19), with 1 to 4 other jobs on the machine.
