# mt1 (decision 0173): parallel plugins — prototypes

New files only; nothing here is part of the runtime, the lifted code or the app. Measurement aids for `docs/decisions/0173-parallel-plugins.md`.
Everything runs in a throw-away headless Chrome (`lift/out-mt1/chrome-profile/<name>`, removed afterwards) behind a localhost-only server that adds COOP same-origin + COEP require-corp + CORP
same-origin (`/coi/…`) or nothing (`/plain/…`); `../cdp.mjs` is only imported. The wasm files are the existing OTT builds of decision 0164 (not rebuilt): symlinks in `lift/out-mt1/data/`:
`mx9.wasm` (`lift/out-wasmgen2/wasm/hyb2/mx9/vst.wasm`, hybrid with the MXCSR window), `c.wasm` (`…/ottw-O2-b1/vst.wasm`, C path wasm -O2), `nomx.wasm` (`…/hyb2/nomx3/vst.wasm`), `ott.vst3`
(`…/rebased/OTT.2000000.vst3`), `seed.tsv` (`lift/harness/vst/vfs-seed-vst.tsv`).

| file | what |
|---|---|
| `lib.mjs` | server, Chrome launcher, `withBrowser`, per-thread CPU from /proc (`threadCpu`/`cpuDiff`) |
| `features.html`, `worklet-probe.js`, `run-features.mjs` | feature detection in the page, a Worker and an AudioWorklet (SAB, Atomics, JSPI, shared Memory sizes, AudioContext rates) |
| `vst-host.js`, `node-check.mjs` | a browser/node host for the 22 imports of the emscripten-built OTT harness wasm (no emscripten glue); `node-check.mjs` = single-instance self-check |
| `vst-worker.js`, `scale.html`, `run-scale.mjs` | N Workers on ONE compiled `WebAssembly.Module`, barrier start, per-block wall time inside the Worker, CPU per Worker thread, RSS |
| `rtt.html`, `rtt-worker.js`, `run-rtt.mjs` | SAB request/response round trips with 32 KiB in and out (Worker↔Worker, main thread `waitAsync`/spin, `postMessage` baseline, fan-out to K plugins) |
| `pipe.html`, `pipe-plugin.js`, `pipe-producer.js`, `run-pipe.mjs` | the REAL lifted OTT process loop in K Workers, paced by SAB slots from an "engine" Worker (parallel or chain, real-time paced or full speed); the 32 KiB exchanged are stand-ins |
| `smem/` (`smem.c`, `build.sh`, `jspi_pthread.c`, `build-jspi.sh`), `smem.html`, `jspi_pthread.html`, `run-smem.mjs`, `run-jspi-pthread.mjs` | emscripten pthreads on a shared Memory: lift-style register array shared / TLS / pointer, non-atomic vs atomic RMW, JSPI on a pthread |
| `vfs.html`, `vfs-worker.js`, `run-vfs.mjs` | one SharedArrayBuffer read by N Workers vs a private copy per Worker |
| `jspi-overlap-worker.js`, `jspi-overlap.html`, `run-jspi-overlap.mjs`, `guest.wat` | a JSPI-suspending process proxy lets K guest threads of ONE Worker keep K plugin Workers busy at once (vs one thread calling them in turn) |
| `plugin-worker/` (`main_vst_ext.c`, `build.sh`) | S1: the OTT harness with an EXTERNAL-INPUT mode (`--ext`, imports `env.mt1_ext_wait` / `mt1_ext_done`) and `--preroll`; `build.sh` builds `vst-ext-O2.wasm` from the same generated units / runtime as the existing OTT wasm. The product code that drives it is `app/src/plugins/*` (protocol, host, Worker, client, driver) |
| `tools/thread-trace.{sh,gdb}`, `tools/{play-short,threads-only}.g3` | premise check (a): every `CreateThread` of the lifted engine (native -O0 of the ui1 build, read-only) with the Delphi class name, around a play button press |
| `tools/summarize_*.py`, `tools/budget.py` | tables; `budget.py` = the P3 core-count arithmetic (a READING, not a measurement) |

```
export MT1_OUT=<repo>/lift/out-mt1           # results/, logs/, data/ live there
cd lift/harness/web/mt1
node run-features.mjs coi=1
nice -n 15 node run-scale.mjs mod=mx9 n=1,2,4,8,16 blocks=300 reps=3 tag=sc1      # mod=c: blocks=120
nice -n 15 node run-rtt.mjs cases="worker:park:park:1:0,worker:park:park:16:0" rounds=3000 reps=3 tag=rtt1   # case = producer:mode:pluginMode:K:workUs
nice -n 15 node run-pipe.mjs cases="par:1:1,par:16:1,chain:4:1,par:16:0" blocks=150 reps=2 tag=pipe1          # case = topology:K:paced
smem/build.sh <out>; node run-smem.mjs; smem/build-jspi.sh <out>; node run-jspi-pthread.mjs      # needs the emsdk (EM_FROZEN_CACHE=1 reads its cache)
nice -n 15 node run-vfs.mjs cases="shared:4,private:4" mb=105 reps=2 tag=vfs1
python3 tools/summarize_scale.py $MT1_OUT/results sc1 out.tsv; python3 tools/budget.py out.tsv
```

S1 (plugin Worker bridge):
```
REPO=<repo> OUT=<repo>/lift/out-mt1/wasm-ext JOBS=2 nice -n 15 bash plugin-worker/build.sh        # -> $OUT/vst-ext-O2.wasm (about 5 CPU minutes: 12 generated units at -O2, 16-39 s each, plus the runtime)
ln -s <that file> lift/out-mt1/data/vst-ext.wasm      # data/ also holds ott.vst3 (rebased PE) and seed.tsv
cd app && MT1_OTT_DIR=<repo>/lift/out-mt1/data npx vitest run test/unit/plugins-protocol.test.ts test/unit/plugins-ott.test.ts     # node worker_threads
MT1_OTT_DIR=... node test/e2e/plugin-bridge.mjs [--startup]                                                                      # Chrome 156
```
