# Detached-window experiment (docs/investigations/detach-window.md)

Measurement aid only. No FL Studio code, no wasm. Plain HTML/JS + one node driver. `engine-worker.js` is a stand-in that draws 512x384 frames and answers input events;
`main.js` is the opener page (owns the worker, relays); `popup.js` is the detached window (one canvas = one guest window).

Run (foreground, one case = one temporary headless Chrome, localhost-only server, results to `docs/investigations/detach-window/<name>.json`):

```
cd lift/harness/web/detach
node run-detach.mjs probe name=probe                                   # window.open / Document PiP / getScreenDetails without and with a (CDP-injected) click
node run-detach.mjs measure via=main|port|canvas mode=loop|block name=m-<via>-<mode>-r1 ms=5000 nIn=300 prefix=/plain|/coi inp=msg|sab [late=1]
node run-detach.mjs pip | piptwo | close | busy | busy2 | geom | coop | wm | multi | act | proc name=<name>
PYTHONDONTWRITEBYTECODE=1 python3 ../../../../docs/investigations/detach-window/summarize.py   # TSV tables
```

`via`: how frames reach the popup (`main` = worker -> opener -> popup relay, `port` = MessageChannel worker <-> popup, `canvas` = popup's OffscreenCanvas drawn by the worker).
`mode`: `loop` = worker returns to its event loop between frames (what a JSPI-suspended engine looks like from outside), `block` = worker stays in `Atomics.wait`.
`/plain` = no COOP/COEP, `/coi` = COOP same-origin + COEP require-corp (also on popup.html), `/coos` = popup.html without COOP, `/coap` = COOP same-origin-allow-popups.

Needs `/usr/bin/google-chrome` (override with CHROME_BIN) and `../cdp.mjs`. The profile is `/tmp/opencode/detach/chrome-profile` (override DETACH_PROFILE), removed afterwards.
Chrome is started with its background networking switched off and every host except 127.0.0.1 made unresolvable (`--host-resolver-rules`).
All pages run in an isolated browser context (`Target.createBrowserContext`): on the machine of this report the default context did not complete http://127.0.0.1 loads (cause not found).
Headless Chrome has no window manager: positions, always-on-top and focus between windows are not real there.
