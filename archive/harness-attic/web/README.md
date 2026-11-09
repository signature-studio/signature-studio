# Browser harness for the 6-module wasm (flengine6.wasm)

**Historical (decisions 0128-0134, 2026-10).** The harnesses in this directory and its subdirectories `fruity/`, `jspi/`, `perf1/`, `perf2/`, `final0129/`, `g3/` (the loader pages and Workers `fle-worker.js`,
`fle-fruity-worker.js`, `fle-worker-jspi.js`, `fle-perf-worker.js`, `fle-perf2-worker.js`, `fle-final-worker.js`, `fle-g3-worker.js`) were the measurement vehicles before the page in `app/` existed. They put the engine's input
files into the Emscripten glue's in-memory file system before `main()`, which is what the text below says. The shipped page does **not** do that any more (decision 0221): the engine is linked with `-sFILESYSTEM=0`
(`NO_FS=1` of `app/scripts/build-engine-bundle.sh`, `WEB_NO_FS=1` of `fruity/link-web.sh`) and `app/src/engine/hostfiles.ts` answers its file reads from the pack bytes. What is still live here: `fruity/link-web.sh`,
`fruity/fle_idle_lib.js`, `final0129/pack_seed.py`, `threads/`, `filedlg/`, `opfs1/`. The rest is kept as the record of those measurements and is not maintained.

Measurement aid (docs/investigations/browser-feasibility.md). Nothing here is part of the runtime or of the lifted code; no binary is in this
directory. It runs the same `flengine6.wasm` that `lift/harness/two/run_node_nmod.cjs` runs under node, in a Web Worker of a (headless) Chrome,
with the input files supplied over HTTP into the in-memory file system (MEMFS) instead of read from the host (NODERAWFS).

## Files

| file | purpose |
|---|---|
| `fle-worker.js`, `index.html` | the loader: the page starts a Worker; the Worker fetches the 9 input files, writes them to MEMFS (`FS.writeFile`, `canOwn`), sets `FLE_VFS_SEED`/`FLE_VFS_SEED_ROOT`, instantiates the wasm (`instantiateStreaming`, or `compileStreaming` + `instantiate`, or fetch + `compile`) and runs `main`; progress marks and every stdout line go back with `postMessage`. `fast=1` replaces `emscripten_return_address` by `() => 0` (= `FLE_WASM_FAST_RETADDR=1`). Page parameters: `?auto=1&fast=1&mode=instantiate\|compile\|buffer\|cachestorage&enc=identity\|gzip\|br\|zstd&rate=<MB/s>&root=<guest dir>&cache=1&keep=1` |
| `serve.mjs` | dependency-free HTTP server: `/dist/` = `$WEB_DIST`, `?enc=` serves the pre-compressed file with `Content-Encoding`, `?rate=` throttles, `/coi/` adds COOP/COEP |
| `cdp.mjs`, `measure.mjs`, `suite.sh` | one case = one foreground node process: server + a new headless Chrome with a throw-away profile + CDP over node's built-in WebSocket + RSS sampling of the Chrome process tree from /proc; writes `runs/<name>.json` and `.stdout`, compares the stdout with a node run |
| `features.html`, `features.js` | feature tests in the page and in a Worker (SharedArrayBuffer, wasm features and limits, OffscreenCanvas, OPFS, AudioWorklet on an OfflineAudioContext, storage, DecompressionStream, cost of `new Error().stack`) |
| `cachetest.html`, `idbtest.html` | Cache Storage `put()` of large responses; IndexedDB as a cache for the wasm bytes |
| `compress.mjs`, `v8flags.mjs`, `summarize.mjs` | gzip -9 / brotli q11 (/ zstd) of the wasm; the V8 flags of the Chrome in use; TSV of the run jsons |
| `build/` | the commands that made the wasm and its web glue (gen_two.sh wrapper, build_nmod_wasm.sh wrapper, relink for the web) with the `/tmp/opencode/web` paths of the measurement |
| `results/` | the small result files quoted in the report |

## dist layout (not in git)

```
dist/flengine6.wasm                 389,439,333 B, SHA-256 6e52a849b2aca06c3f78d43454ce32006105e2b12a2dbb10e8aa9372a590dcc7
dist/flengine6.web.js               glue linked with build/link.sh web (ENVIRONMENT=web,worker, MEMFS, FS and ENV exported)
dist/flengine6.wasm.{gz,br,zst}     optional, for enc=
dist/data/FLEngine_x64.dll, dsp_ippv2_x64.2000000.dll, Blend2D.4700000.dll, quickfontcache_x64.1d00000.dll, freetype_x64.4b00000.dll,
          ILTools_x64.6000000.dll   the rebased files that gen_two.sh `rebase` made (the same files as the node run)
dist/data/seed/vfs-seed-fonts.tsv, dist/data/seed/lift/out-inventory/extracted/fonts/{ILCursors,ILGlyphsEx}.ilfont
```

## Run

```
node lift/harness/web/serve.mjs                      # PORT=8080 WEB_DIST=<dist>; open /index.html?auto=1&root=/data in Chrome
node lift/harness/web/measure.mjs run name=x fast=1 mode=instantiate enc=br root=/tmp/opencode/web/dist/data ref=<node stdout>
node lift/harness/web/measure.mjs features name=f coi=1
```

`fast=0` (the unmodified `emscripten_return_address`, about 30 minutes under node) is refused by `measure.mjs` unless `allowstock=1`.
Reading `/proc/<pid>/smaps_rollup` (`pss=1`) slows the renderer: leave it off for timing runs.
