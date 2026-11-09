# app/ — FL Studio web front end (lane F, stage F2)

**The page runs the real lifted FL engine (`engine.kind: emscripten-fruity-v1`) and nothing else** (decision 0243 removed the stand-in engine, its banner and the URL parameters that chose between ways of showing it; rules: `docs/policies/progress-integrity.md`). The engine is hundreds of MB of wasm; the first idle takes about 7 s on the test machine (decision 0235).

**Private. Not for public release.** The app is meant to run locally, or behind Cloudflare Access (or another access-protected deployment) for the owner only.
Product-derived binaries (the lifted engine wasm, DLLs, installer data) are never in git: they are served from a directory (`FL_ASSETS_DIR`) or from R2.

What it is: a Vite + TypeScript project that loads an **engine bundle** (described by a `manifest.json`), runs the engine in a module Worker, and shows the engine's windows on a **virtual desktop**
(F2, decision 0141, `docs/decisions/0141-frontend-input.md`). The **real lifted FL engine** (`engine.kind: emscripten-fruity-v1`; F1, decision 0137): you open the page, the browser downloads and compiles the engine wasm, the engine runs in the Worker, and since F2 you can
**point at it and type to it**: the pointer, wheel and keyboard events of the desktop go to the engine's input queue (decision 0136), the engine's windows are drawn where it has them in its z-order, and the cursor
the guest chose is the desktop's CSS cursor. With the bundle built from HEAD fa11947 (SEH on, 8 MiB guest-thread C stack) the first idle comes about 30-37 s after the page opens; the welcome wizard can be clicked
('No thanks') and closed with Enter, and the engine then stops at its next wall (`user32!UpdateLayeredWindow`, no layered-window model yet) - the page says what stopped it and where. **Reactions take 0.5-7 s**:
the engine takes input only between its animation frames; in this build (wasm -Oz, headless Chrome) a frame takes about 3 s for the first ~50 s after the first idle, then about 0.5 s, and about 1.0 s from ~145 s on (measured with no input; decision 0141 section 4.3).

## Commands

| command | what it does |
|---|---|
| `npm install` | install the dev dependencies (vite, typescript, vitest, wrangler, workers types) from the registry; nothing else is fetched |
| `npm run dev` | Vite dev server on http://127.0.0.1:5173 (COOP/COEP/CORP, `/assets/` from `FL_ASSETS_DIR`) |
| `npm run build` | type check, then `vite build` into `dist/` (the app's own files go under `/static/`, because `/assets/` is the big-asset route) |
| `npm run preview` | serve `dist/` on http://127.0.0.1:4173 with the production headers (including the CSP) and `/assets/` from `FL_ASSETS_DIR` |
| `npm run typecheck` | `tsc --noEmit` for the page, the engine Worker, the tests and the Cloudflare Worker |
| `npm test` | vitest unit tests (`test/unit`: also keymap, input state machine, idle driver, desktop layout, appview reader, cursor, stop message). `PACK_SEED_PY=<path of pack_seed.py>` (default `../lift/harness/web/final0129/pack_seed.py`) enables the test that reads a pack the real script wrote |
| `npm run test:worker` | the Cloudflare Worker under `wrangler dev --local` (local R2 simulation): Range, ETag/304, Content-Encoding, headers, 404/416, Access JWT, the default-manifest refusal (a Worker of its own whose R2 starts empty), POST /__perf |
| `npm run test:e2e` | headless Chrome (isolated context, network blocked except 127.0.0.1) against the dev server, `vite preview` and the Worker (+ local R2); needs `google-chrome` (`CHROME_BIN` to override). With `FL_ENGINE_BUNDLE=<bundle dir>` it also runs the REAL engine: error variants, and `engine-desktop` (decision 0141: the desktop, real pointer and key events, the wizard click with the SHA-256 values of decision 0136, Enter, the stop; about 6-8 minutes per target) |
| `node test/mutation/run.mjs` | mutation check on a copy of this tree (never edits the tree itself) |
| `npm run bundle:engine` | `bash scripts/build-engine-bundle.sh`: builds the REAL engine bundle from the tree (heavy: see "Real engine bundle") |
| `FL_ENGINE_BUNDLE=<bundle dir> npm run test:e2e` | the e2e suite plus the real-engine scenarios (the bundle is never in git) |

**Default manifest path (decision 0152).** `/assets/manifest.json` (R2 key `manifest.json`) is for the REAL engine's bundle only. The dev / preview server and the Worker read it before serving it: `engine.kind` `emscripten-fruity-v1` is served; any other kind is refused with **409** and a plain-text explanation; JSON that does not parse is 500; a missing file is 404 with "the real engine's bundle is not in place". The check is made after decoding and normalizing the path (`//`, `./`, `%2e`, case, trailing dots), on the bytes that are then served (`shared/manifest-guard.ts`). The page shows the server's explanation as it is when the manifest request fails; it never falls back to another bundle.

**Measurement beacon (decision 0152).** The page measures its own speed (start-up to the first idle, UI updates per second, input -> update latency, audio under-runs: not measured yet) and POSTs it to its own origin (`POST /__perf`, same origin only, JSON, at most 16 KB): once at the engine's first idle, then every 30 s (at most 20), and once when the engine ends or the page fails. `vite dev` / `vite preview` append one JSON line per beacon to `FL_PERF_LOG` (default: `perf-log.jsonl` in the PARENT directory of `FL_ASSETS_DIR`): `{"t": ISO time, "remote": "loopback|lan|tailnet|other", "perf": {...}}`. The peer's address is not stored, only its class (decision 0192: behind `tailscale serve`, a reverse proxy on this machine, the socket peer is always `127.0.0.1`, so a loopback peer is classified by the first address of `X-Forwarded-For`, which `tailscale serve` sets to the visitor's tailnet address: the owner's records read `tailnet`). The Worker (decision 0224) checks the beacon the same way and keeps it as ONE R2 object, `perf/<UTC date>/<sid>-<seq>.json` in the bucket bound as `PERF_BUCKET` (`fl-web-cache`; the object is one line `{"t", "remote": "workers", "perf"}`: no IP address, Access identity or cookie), and answers `X-Perf-Stored: yes` (`no` when the write fails or the binding is missing); `GET /__perf/summary` (behind Access) prints the summary of the last days, and `node scripts/perf-log-summary.mjs --dir <directory of synced objects>` does the same on a machine. The "Performance" panel (closed by default) shows the same numbers next to the targets P1-P3 of `docs/milestones/performance-targets.md`. Decision 0192 (beacon version 2): the beacon also carries what the page sees of the SCREEN (`src/perf/display.ts`): the requestAnimationFrame rate and its distribution (the display's refresh rate as far as a page can tell, hidden time excluded), how many of those screen frames had NEW canvas content, the receive -> drawn time of the Worker's frames, long tasks, slow input events (Event Timing), the visible/hidden time, the engine's rate against the screen's rate, and the browser/OS name, viewport and colour depth (`client`; the physical core count is not available to a page: the block says so). The Performance panel shows them (Japanese rows) next to the P2 targets, with a notice of what is sent to the server's log and what is not. Decision 0202 (beacon version 3): the input -> update latency (`input.eventToRaf`, the P2 number, judged at p95 <= 17 ms) is measured from the DOM event's own `event.timeStamp` to the first requestAnimationFrame callback after the answer's `putImageData` (the browser's wait before it delivers the event and its wait for the next animation frame are in it); version 2's number (page listener -> `putImageData`, 10-25 ms smaller) is kept as `input.listenerToPut`, and `input.stages` says where the time goes. `scripts/perf-log-summary.mjs` judges version 3 records by the new number and shows version 1 and 2 records' number apart, under its old name. The screen meter (decision 0192) asks for animation frames in 2 s BURSTS (at the start, at the first idle, every 60 s; `DisplayMonitor` mode `burst`, the default) instead of a loop: a page that asks for a frame at every vsync makes Chrome hold pointer moves until the next frame, which added 10-20 ms to the input -> update delay of a page that otherwise draws nothing (decision 0202 section 4.2). The rAF figures of the beacon and the panel (`display.raf.recent10s` and so on) are those of the latest burst; `display.mode` says so. `window.__app.perf` exposes the recorder and the history of what was sent. `node scripts/perf-log-summary.mjs <perf-log.jsonl> [--machine tailnet]` prints one line per record, the real engine's numbers against the targets (records of an older page that ran a stand-in are marked and not counted) and a table per machine / browser / screen-rate class (decision 0192). `node test/e2e/perf-real-engine.mjs` (with `FL_ASSETS_DIR` of a real-engine bundle, READ ONLY, and `FL_PERF_LOG` of your own) opens the default entrance in the isolated Chrome and checks what the beacons delivered.

**Start-up assets: the page's own stored copy (decision 0220).** The four big assets of the real engine (wasm 130 MB, DLL pack 70.6 MB, seed pack 5.7 MB, glue 0.47 MB, decoded) are also kept by the page in OPFS, as the DECODED bytes in `/fl/cache/assets/<manifest SHA-256>.<wasm|flpack|js>` (`src/bundle/assetstore.ts`). The next load asks for the manifest (`no-cache`, a 304 when unchanged), finds each asset by the hash the manifest names, reads the file, checks its SHA-256 with WebCrypto (`crypto.subtle.digest`, native) and uses it: no request for the assets, no brotli decoding, no JS hashing. A file is written through `createWritable` (invisible until `close()`) only after the bytes from the network passed the size and SHA-256 check, so a file that exists is whole; a damaged, short or changed file is a miss (it is deleted and the asset is fetched again). Files the manifest no longer names are removed after a load, unless another tab holds them (shared Web Lock `fl-asset:<sha>`; without Web Locks nothing is removed). A browser without OPFS, a full quota or a private window loads without the stored copy (the SHA-256 is then taken in JS while the bytes arrive). The hash of a stored file is one WebCrypto digest over all the bytes before they are used; the wasm is still compiled while it arrives. Nothing is ever run before its SHA-256 matched. The page does not call `navigator.storage.persist()` (Firefox asks the person); the Files window's button does. The beacon's optional `assets` block (still version 3) and the Performance panel say per asset where the bytes came from (`network` / `http-cache` / `storage-cache`), the bytes on the wire, and the read / hash / copy / pack-index / write / compile milliseconds; `scripts/perf-log-summary.mjs` has a table by way of loading. Why: with a warm HTTP cache the "download" window of the page was still 2.1 s in Chrome and 4.2 s in Firefox on the test machine, all of it CPU (decoding 207 MB, hashing it in JS), see `docs/0220-asset-cache.md`.

**Audio output (decisions 0139 / 0175 / 0211).** Always on (decision 0243: no `?audio=` parameter). The engine's DirectSound device writes into a SharedArrayBuffer ring (`src/audio/ring-layout.ts`) that an AudioWorklet (`src/audio/fle-out.worklet.js`) plays; the page makes the AudioContext when the engine Worker is made (one task later; a page without an engine, for example one whose manifest failed, makes none and says `音: エンジンの起動待ち`), and a browser keeps it `suspended` until the first gesture (pointerdown, keydown, ...), when `src/audio/install.ts` calls `resume()`. The header line `#audio-status` says which state it is in (`音: 停止中 (クリックまたはキー入力で有効になります)`, `音: 動作中 44.1 kHz · 出力遅延 N ms`, `音: ブラウザが開始を許可していません ...`, `音: 使えません (...)`). While the worklet does not run (context suspended, refused, no device) `src/audio/soft-sink.ts` moves the ring's read position with the page's clock so that the song still advances, silently; without it the engine's play cursor stands still and the transport freezes. The beacon's `audio.detail` carries `status`, `ctxState`, `sampleRate`, `baseLatencyMs`, `outputLatencyMs`, `autoplayPolicy`, `gestures`, `lastResume` (`running` / `pending` / `rejected` / `suspended`), `fillFrames`/`fillMinFrames` (the ring's filling), the underrun counts, `softSinkFrames`, `nodeReady`, and what the engine itself lost (`engineGapFrames`: silence it wrote for a hole it was too late to fill, which the worklet does not count as an underrun once the engine has filled it; `engineSkippedFrames`). Checked by `beaconProblems`, summarised by `scripts/perf-log-summary.mjs`. A parameter the ring cannot take (`?audiodelay=9999`, `?audiocap=1000`) turns the audio off with a reason in the header line instead of stopping the page. `audio.underruns` stays `null` (P3 not measured) until the device has played a quantum.

Set `WRANGLER_SEND_METRICS=false` for every wrangler command (the scripts do). Nothing in this directory logs in to Cloudflare, creates resources or deploys.

## Try it locally

```
cd app
npm install
FL_ASSETS_DIR=<directory whose manifest.json is the real engine's bundle> npm run dev
# open http://127.0.0.1:5173/
```

The bundle comes from `scripts/build-engine-bundle.sh` (see "Real engine bundle"): copy or link the bundle's files under `$FL_ASSETS_DIR/<prefix>/` and its `manifest.json` to `$FL_ASSETS_DIR/manifest.json`. The page has no URL parameters (decision 0243); the four numbers of the audio
output (`audiorate`, `audiolead`, `audiodelay`, `audiocap`) are still read from the URL until decision 0241 fixes them.

`window.__app` exposes the state (`state`: idle / loading / loaded / running / engine-idle / exited / error), `timeline` (the start-up instants in ms since the navigation started), `details`, `summary` (what the engine's
printed lines say: stop message, windows, threads), `checks` (the three-way SHA-256 comparisons), `exit`, `error`, `features`, `readbackAll()`, `verifyExpected()`, `stop()` for the tests; since F2 also `inputEvents`, `latency` (every input event with its times), `appView` (the engine's window list, cursor, pointer), `cursor`, `engineStatus()`, `desktopLayout()`, `idles` (every idle of the engine with how long it computed before), `driverStats`, `sendRaw(ev)`.

## Layout

```
index.html, src/main.ts, src/style.css     the page
src/bundle/   manifest.ts (types + schema), schema.ts (validator), loader.ts (fetch, progress, SHA-256, decode, compileStreaming; decision 0220: the stored-copy and WebCrypto path), assetstore.ts (the OPFS copy: find, write, clean up, Web Locks), assets-report.ts (where the bytes came from, the beacon's `assets` block),
              pack.ts (flpack1), sha256.ts (incremental SHA-256)
src/engine/   worker.ts (the engine Worker), client.ts (page side), protocol.ts (messages), idle-driver.ts (the engine's idle: input queue, wall clock), appview.ts (reader of lift/runtime/fle_appview.h),
              fruity.ts (adapter for emscripten-fruity-v1: glue import, the host files, argv/env, instantiateWasm, exit; ported from lift/harness/web/final0129/fle-final-worker.js),
              hostfiles.ts (the files the engine reads from the host, answered from the pack bytes at the libc system calls),
              fruity-log.ts (reads the engine's printed lines: stop message, windows, threads, surface SHA), features.ts (JSPI etc. detection)
src/perf/     stats.ts (the page's own speed numbers: update rate and spacing, input -> update latency), browser-info.ts (cores, memory, user agent, screen, GPU renderer; decision 0192: browser name, viewport, colour depth, host class), beacon.ts (the payload, when it is sent, POST /__perf), panel.ts (the Performance panel and its notice), display.ts (decision 0192: the rAF / canvas / long-task / event meters and the ?hz= comparison), display-host.ts (their browser wiring); decisions 0152, 0192
src/ui/       desktop.ts (the desktop: ONE canvas, deskCanvas()), cursor.ts (guest cursor -> CSS cursor), engine-status.ts, latency.ts, stop-message.ts,
              phases.ts (start-up stages from a timeline), progress.ts, panels.ts (the windows the engine printed, threads, engine log)
src/input/    events.ts (the raw events of lift/runtime/fle_input.h), keymap.ts (KeyboardEvent.code -> virtual key / scan code / extended flag; which keys the browser keeps), tracker.ts (pointer / key state machine: client -> screen
              coordinates, buttons from the `buttons` mask, release on blur, wheel), pointer.ts (the DOM listeners of the desktop: pointer capture, focus, contextmenu, blur)
shared/       range.ts (HTTP Range / ETag helpers), headers.ts (COOP/COEP/CORP/robots/CSP, MIME, Cache-Control), manifest-guard.ts (the default manifest is for the real engine only), perf.ts (what POST /__perf accepts and stores) - used by the dev server and the Worker
tools/        assets-middleware.ts (serves FL_ASSETS_DIR at /assets/ for dev and preview), perf-middleware.ts (POST /__perf -> JSONL for dev and preview)
worker/       index.ts (the Cloudflare Worker), access.ts (Cloudflare Access JWT validation), tsconfig.json
wrangler.jsonc
scripts/      pack-assets.mjs (builds a bundle directory + upload commands), build-engine-bundle.sh (builds the real engine bundle from the tree), lib/png.mjs (PNG decoder copied from the harness)
test/         unit/ worker/ e2e/ mutation/ helpers/
```

## Engine bundle

A bundle is a directory of files plus `manifest.json` (schema 1, `src/bundle/manifest.ts`):

- `engine.wasm` — url, decoded size, SHA-256 of the decoded bytes, `compression` (`identity`, `gzip`, `br`).
- `seedPack` — the guest's virtual files, as one pack object (`src/bundle/pack.ts`): `flpack1` (this app's format, built by `pack-assets.mjs`) or `flseedp1` (the seed pack of decision 0134, written by `lift/harness/web/final0129/pack_seed.py`; name it with `--seed-pack FILE`); `dllPack` — DLL-derived data, or `null`.
- `engine.kind` (`emscripten-fruity-v1`, the only kind), `args`, `env`, `options`. The wasm has the desktop compositor, `fle_desk_compose` and `fle_desk_resync` in its exports (decision 0243: `scripts/pack-assets.mjs` fails when they are missing; the manifest does not say so). The manifest has no `expect`, `desk`, `hostFiles` or `mainLid` field any more.
  - `options` = `{ glue, root, hostFilesAbi, hints?, stopNotes? }` — `glue` is the emscripten JS module (an `AssetRef`: url, size, SHA-256), `root` the directory
    in which the engine finds the packs' files (`root/<pack path>`; `engine.args[0]` must be under it). The glue is linked with `-sFILESYSTEM=0` (`NO_FS=1` of `scripts/build-engine-bundle.sh`) and the page answers the engine's
    file reads (`open`, `stat`, `pread`, `fopen` ...) from the pack bytes (`src/engine/hostfiles.ts`); `hostFilesAbi` (`emscripten-6.0.12`) is the Emscripten that linked it, which the page checks. `hints.firstPixelSeconds` is the
    "about N seconds from the engine start to the first pixel" the start-up display shows (measured on the machine that built the bundle), `stopNotes[]` extra words for known stops
    (`contains` a text of the engine's stop message → `note`).
- All URLs are paths on the page's own origin. The loader refuses anything else.

Delivery: the server may send a pre-compressed object with `Content-Encoding` (the browser decodes it; the Worker does this for R2 objects that have `contentEncoding` metadata,
`tools/assets-middleware.ts` does it for `NAME.br` / `NAME.gz` next to `NAME`). A stored `.gz` without `Content-Encoding` is decoded in the page with `DecompressionStream`.
**Chrome 156 has no `DecompressionStream('brotli')`**, so brotli must always be sent with `Content-Encoding: br`. Either way the SHA-256 and size are checked on the decoded bytes
while they stream into `WebAssembly.compileStreaming`.

`node scripts/pack-assets.mjs --help` lists the options.

### Real engine bundle (F1)

`scripts/build-engine-bundle.sh` builds the bundle of the real engine from this tree. It follows decision 0134's procedure (lift → generated C → size-cut translation units → emscripten → web link →
seed pack, DLL pack, manifest). Heavy: it needs the toolchain of the lift work (`/opt/emsdk` with emcc 6.0.12, clang, cargo with the crates vendored offline, python3, node ≥ 24) and the product-derived inputs that are
not in git (`binaries/win64/FLEngine_x64.dll` and `lift/out-ext`, `out-blend2d`, `out-qfc`, `out-wallQ`, `out-inventory`, `out-idle1`, `out-fruity11`: the script checks the engine DLL's SHA-256).

```
cd <repo root>      # or a `git archive` of HEAD with HOST_ROOT=<checkout that has the inputs>
export WORK=lift/out-app-engine HOST_ROOT=$PWD
VARIANT=Oz JOBS=1 CAP_MB=5120 bash app/scripts/build-engine-bundle.sh gen resplit          # ~10 min, native check of the generated C; JOBS=1 (5 GB cgroup)
VARIANT=Oz JOBS=3 CAP_MB=5120 bash app/scripts/build-engine-bundle.sh wasm weblink seed bundle   # ~10 min + brotli
FL_ASSETS_DIR=$WORK/bundle npm --prefix app run preview          # then open  http://127.0.0.1:4173/   (its manifest.json copied to $WORK/bundle/manifest.json)
```

`VARIANT`: `Oz` (no hooks, -Oz: development; the wasm is about 250 MB, brotli 14 MB), `O2` (decision 0134's C2: 667 MB, brotli 26.5 MB, first pixel about 33 s; the units stage needs `UNIT_SLICE=1/2` and `2/2`
under a 60-minute command limit), `H` (hooks, -O0: counts function entries; slow). Everything is cgroup-capped (`systemd-run --user --scope -p MemoryMax=… -p MemorySwapMax=0`); when `MemAvailable` is
below 4 GB the stage drops to `JOBS=1` and does not wait. The last stage writes `$WORK/bundle/<PREFIX>/{engine.<sha>.wasm(.br), engine-glue.<sha>.js(.br), seed.<sha>.flpack(.br), dll.<sha>.flpack(.br), manifest.json, SHA256SUMS,
upload-commands.txt}` and **prints the upload commands without running them**.

Decision 0132's SEH is opt-in: `FLE_SEH_HANDLERS=0x112c0` in the environment is passed through to `a4-multi.sh` (the stage list and everything else is unchanged). The `o1` stage is run for `VARIANT=H` only (Oz and O2 have no
function over V8's 50,000-locals limit; its scanner crashed on two units of the SEH tree). `NATIVE_SECONDS` (default 900) is the wall-clock limit of the native check run. `BUCKET` names the R2 bucket in the printed upload commands.
To watch an engine whose stop is not known in advance (no assertions, screenshot and engine log at the end): `FL_ASSETS_DIR=<dir with <prefix>/manifest.json> node test/e2e/watch-engine.mjs <prefix> --minutes 30 --out <dir>`.

**Replacing the engine** (after the SEH and G3 work is merged and the wasm is rebuilt): run the `gen`…`bundle` stages again on the new tree (move or delete `$WORK/onat`, `wsrc`, `w-*` first: stages skip what exists) and
serve the new directory; the app does not change. `manifest.json` names every file by SHA-256, so a new bundle is a new directory (`PREFIX=fl-Oz-2`) and the old one keeps working until the manifest URL is changed.
If the new engine stops somewhere else than wall 16, the UI shows the engine's own stop message; the `stopNotes` of the manifest only add words for stops they know.

## Display

The engine is shown by **display method D** (decision 0223, below). There is no other method.

### Display method D: the runtime composes the desktop (decision 0223, PROVISIONAL)

The real engine's windows are composed by the **runtime** (`lift/runtime/fle_desk.c`; what Windows' Desktop Window Manager does): the window table, each window's surface and the layered-window state are read and ONE
RGBA picture of the virtual screen is kept in the engine's memory, with the rectangles that changed (only pixels that differ: `-DFLE_DESK_NO_TRIM` reports the damaged rectangles whole). The engine Worker puts the changed rectangles on the
**one** `OffscreenCanvas` the page transferred (`src/engine/desk.ts`: `putImageData` of an `ImageData` that is a view of the picture, no copy, no conversion, no message to the page). The page knows no window: no canvas per window, no
position, z-order, visibility, layering, main-window choice or title choice. What it gets besides input is small: `osstate` (the cursor, the title of the foreground window, the screen size: `src/engine/os-state.ts`) and `desk` (counters and
the times of the presents; `answers` names the inputs a present answers, for the latency tracker). Input goes through a SharedArrayBuffer ring (`src/input/ring.ts`); a `wake` message is sent only when the Worker is parked. Needs OffscreenCanvas and
a cross-origin isolated page (the page stops with a message before any download when either is missing). The engine wasm exports the compositor (`scripts/pack-assets.mjs` refuses a wasm without `fle_desk_compose`; there is no other display path). The Worker's frame loop sleeps when the picture has not changed for 100 ms (`src/engine/desk-sleep.ts`; the engine's idle finds the next change and wakes it). Test aids (call-only): `window.__app.deskDumpBase64(peek)` (the compositor's window text and the picture), `deskStats`.

**The guest's screen is the room (decision 0212).** The virtual screen is no longer 1920x1080: the page measures the room it has for the desktop (everything below the header and left of the 340 px monitor panel) before the engine
starts and tells the engine (`FLE_MONITOR_W` / `FLE_MONITOR_H` in the manifest's `engine.env`; `lift/runtime/fle_screen.h`), and the engine reports what it uses in `fle_av_poll`. The guest's screen is the room in CSS pixels (one guest pixel = one CSS pixel; the browser enlarges it on a HiDPI screen), the engine is told when the browser window is resized (`WM_DISPLAYCHANGE`), and the monitor panel is closed by its header button. Decision 0243 removed the other modes (`?screen=`, `?dpi=`, `?panel=`, `?follow=`, the header's 標準 / 高精細 choice): there is the one. A bundle built before decision 0212 does not read the size: the page says so and fits 1920x1080 into the room. Measurements: `docs/decisions/0212-screen-fit.md`;
tool: `test/e2e/screen-run-ff.mjs` (Firefox at a device pixel ratio: the sizes, the canvas, a screenshot; the Chrome checks are `engine-desk-resize` of `test/e2e/run.mjs`).

### The desktop and the input (F2, decision 0141)

- **Desktop**: one `<div id="desktop">` of the virtual screen's pixels (scaled by CSS) that clips to the screen. The engine's windows are one picture on ONE `<canvas>` (display method D).
- **Input**: pointer events (`pointerdown/move/up/cancel`, with `setPointerCapture` and focus on pointerdown; buttons are taken from the `buttons` mask), the wheel (100 px = one notch of 120, direction flipped), the keyboard
  (`KeyboardEvent.code` -> the virtual key / scan code / extended flag of a US keyboard, `src/input/keymap.ts`), the context menu suppressed, blur / hidden tab / pointercancel releasing every held key and button. The desktop
  must have the focus (click it) for keys to go to the engine. **Tab and Ctrl+W are left to the browser** (Tab leaves the desktop unless 'send Tab to the engine' is ticked; F5 is the engine's: it is sent and the reload is cancelled); every other key is sent and its browser action suppressed
  while the desktop has the focus. The page sends *screen coordinates only*; the engine does the hit test and the delivery (decision 0136). With D the input is written to a shared ring (`src/input/ring.ts`).
- **Before the first idle** (about 30-37 s) input is ignored: the desktop shows 'starting' and sends nothing. While the engine computes (0.5-3 s per frame in the -Oz bundle, see Limits) events wait in the Worker's queue (consecutive moves are
  merged, at most 256) and are given to the engine at its next idle; the status line shows 'computing (N s)', the pending count and the last input's latency.
- **Cursor**: the cursor the guest chose with `SetCursor` (standard cursors -> CSS; `SetCursor(NULL)` -> `none`). The real engine, at the welcome wizard, uses cursors it made itself (`CreateIconIndirect`, no pixels kept by the
  runtime), shown as the arrow.

### The main window (decision 0150; removed by decision 0223)

The page used to choose "the main window" by its class (`src/engine/main-window.ts`) for the title, the first-pixel mark and `expect` entries of `window: "main"`. It no longer knows any window: `document.title` is the title of the foreground window as the Worker reports it (`os-state.ts`); the first pixel is the first present of the picture; The real-engine e2e (`test/e2e/engine-desk.mjs`) finds the welcome wizard by its size, because the compositor's text names no class.

### Layered windows (decision 0145; moved by decision 0223)

Layered windows (`WS_EX_LAYERED`: `UpdateLayeredWindow`, `SetLayeredWindowAttributes`) are composed by the runtime's compositor from the layered-window state of the runtime (`fle_layered.c`), by Learn's formulas (`Dst = Src + (1 - Src.A) * Dst`, one rounding), not by the page. A layered window that neither call has been made for is not drawn. Tests: `lift/crates/fe-lift/tests/desk1_compose.rs` (values worked out by hand) and the e2e group `compositor-synth` (the scene of `scripts/lib/layered-scene.mjs`, 21 probes).

## Cloudflare Worker (`worker/`, `wrangler.jsonc`)

- `/assets/*` → R2 binding `ASSETS_BUCKET`: single byte Range (206/416), `ETag`/`If-None-Match` (304), `If-Range`, `HEAD`, stored `contentEncoding` → `Content-Encoding`
  (response created with `encodeBody: 'manual'`), `application/wasm`, immutable caching for versioned names.
- everything else → Static Assets binding `ASSETS` (`dist/`), with `run_worker_first` so that this code sees every request.
- every response: COOP `same-origin`, COEP `require-corp`, CORP `same-origin`, `X-Robots-Tag: noindex, nofollow`, `nosniff`; HTML/JS also get a CSP that allows only `'self'`.
- Cloudflare Access is configured at the edge (dashboard/API), not here. The Worker validates `Cf-Access-Jwt-Assertion` (RS256, `kid` from
  `https://<team>.cloudflareaccess.com/cdn-cgi/access/certs`, `iss`, `aud`, `exp`, `nbf`) as a second layer when `ACCESS_TEAM_DOMAIN` and `ACCESS_AUD` are set.
  **If they are not set, every request gets 403** (fail closed; the shipped `wrangler.jsonc` has both empty; there is no switch that skips the check: the Worker tests use a test team, AUD and key set, `test/helpers/wrangler.mjs`).
- `workers_dev: false` and `preview_urls: false`: no `*.workers.dev` / preview hostname exists that Access would also have to cover. No `account_id` is written anywhere.

The steps only the owner can do (wrangler login, bucket, uploads, Access application and policy, filling in the two variables) are in `docs/decisions/0135-frontend-app.md`, section 12 (updated for the real engine in `docs/decisions/0137-frontend-real-engine.md`).

## Limits

F2: the engine takes input only between its frames. A frame takes about 3 s for the first ~50 s after the first idle, about 0.5 s until ~145 s, and about 1.0 s after that (wasm -Oz, headless Chrome, a shared machine: the same order as decision 0138's wasm -Oz figure of 0.89 s per frame at its steady state), so an input is processed after 0.4-3 s and a click shows its effect after about 2 s (steady state) to 3.4-5.7 s (in the first 100 s); the first idle comes 30-37 s after the page opens. The cause of the three phases was not investigated. The engine stops at `UpdateLayeredWindow` once the wizard is closed (decision 0140 is the lane for that).
Not done: audio (F3), a real deployment (F4), file import UI (F5). Not measured: Firefox, Safari, headed Chrome, a real GPU, HiDPI hardware, touch, IME, non-US keyboards, real Windows.
The Worker has been run only under `wrangler dev --local` (workerd 1.20261023.1), never on Cloudflare's edge. See the decision documents for the full lists.
