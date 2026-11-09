# srm1 (decision 0177): Serum 2 in a Chrome Worker

New files only. A copy of mt1's measurement kit (decision 0173, `lift/harness/web/mt1/`: `lib.mjs`, `vst-host.js`) with the input of the lifted Serum2.vst3 (`lift/harness/vst3a/build_vst3a_wasm.sh`):

| file | what |
|---|---|
| `lib.mjs` | localhost server with COOP/COEP (`/coi/…`; `/coi/data/…` is `$SRM1_OUT/data`), throw-away headless Chrome, CDP, per-thread CPU from /proc |
| `vst-host.js` | the host of the 23 imports of the emscripten-built harness wasm (no emscripten glue); adds `srm1_thread_cpu_s` (wall time in a Worker), a 3.75 GiB memory limit, `memBytes()` |
| `serum-worker.js`, `serum.html` | N Workers on ONE compiled Module; the plug-in file, the VFS seed, the content and the state files are SharedArrayBuffer-backed (one copy); `?mod=O2&states=0,1,3,4,5,3&blocks=40&notes=60,64,67,72&seed=small|full` |
| `run-serum.mjs`, `series.sh` | the runner (one fresh Chrome per run; per-Worker block times, hashes, PSS/RSS, Worker CPU from /proc); `series.sh` = states 0,1,3,4,5 and the 4-note chord |
| `summarize.py` | the JSON lines of the runner -> TSV |

`$SRM1_OUT/data/` must hold: `<mod>.wasm` (the `vst.wasm` of the build, e.g. `O2.wasm`, `O2simd.wasm`), `serum.vst3` (the REBASED PE), `seed-small.tsv` / `seed-full.tsv` (the VFS seed lists with the host column rewritten to `/content/<relative path>`), `content/` (the installer files the states reference), `state/` (`sN_c2.bin`, `sN_c3.bin`). The piano (state 2) needs the JSPI thread build, which this host does not support.

```
export SRM1_OUT=<work dir>; cd lift/harness/web/srm1
nice -n 15 node run-serum.mjs mod=O2 states=0,1,3,4,5,3 blocks=40 reps=2 tag=six
taskset -c 0-3 node run-serum.mjs mod=O2simd states=0,1,3,4,5,3 blocks=40 reps=2 tag=hc4     # hardwareConcurrency 4
```
