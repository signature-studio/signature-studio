# Production bundle check, main 81f6638c (CI run 41956980092), real Chrome 156 and Firefox 158

Real engine: the lifted FLEngine_x64.dll (wasm) ran in both browsers, served with the page built from the same commit,
opened at the default entry `/` with no parameters. No stand-in, no replayed image.

## Bundle (local rebuild with ci/stage-*.sh, SHIP_PRESET=F2xH7 VARIANT=Oz, from `git archive`) vs CI

| item | CI log | local rebuild | same |
|---|---|---|---|
| wasm size | 133,934,996 B | 133,934,996 B | yes |
| wasm sha256 | 15bedd6ec646a11e4c269477b0f2d8371d30680ae409b304e6e5a459ba2436bb | same | yes |
| glue sha256 prefix | 96b2ad2bf1ea | 96b2ad2bf1ea | yes |
| seed sha256 prefix | f7726113 | f7726113 | yes |
| dll sha256 prefix | 50206405 | 50206405 | yes |
| br wasm / glue / seed / dll prefix | e21f27fc4d969f27 / 823d2d39f5ef3155 / 6f8cf851197d727b / ffcbd680a95ec4e2 | same | yes |

## Default entry `/`

| check | Chrome 156 | Firefox 158 |
|---|---|---|
| first idle | 8.84 s (machine load average 9-18 during the run) | 6.08 s |
| title (foreground window, from the engine) | Welcome to FL Studio - in ?trial mode? | same |
| console errors | 0 | 0 |
| beacon `POST /__perf` | 204, stored, `credentials: same-origin` | 204, stored, `credentials: same-origin` |
| beacon `desk.mode` | gl (all 46 beacons of the whole run, both browsers) | gl |

## Owner-report scenes (11) and the new fixes

| scene | Chrome | Firefox (one process per scene) |
|---|---|---|
| d1 sampler-envelope | ok | ok |
| d2 browser-folder | ok | ok |
| d3 settings-all-tabs (TMIDIForm appears) | ok | ok |
| d4 playlist-middle-drag | ok | ok |
| d5 mixer-front (pixel differs) | ok | ok |
| d6 playlist-close | ok | ok |
| d7 main-close (engine ended code 0) | ok | ok |
| d8 file-menu-100 | 100/100 cycles, 100 popup surfaces | 100/100, 100 popup surfaces |
| d9 keys | ok | ok |
| d10 pianoroll-f12 | ok | ok |
| d11 menu-hover-escape | ok | ok |
| `+` > More plugins...: list window stays | yes (TPluginListForm 'Select generator plugin' z1) | yes |
| Esc: list window gone, main window takes input | gone; `+` opens the popup again (TQuickPopupMenuWindow 176x132) | same |
| Add menu fade, row 'Simple' (40 shots) | 197,212,221 -> 31,40,45, 0 black | 137,149,157 -> 31,40,45, 0 black |
| F8 picker fade-in (25 shots, 4 points) | 95,101,109 -> 31,39,45 monotonic, not black; closed: 94,101,108 | 86,94,101 -> 31,39,45 monotonic, not black; closed: 95,102,110 |

Point (200,200) of the Add-menu scene reads 0,0,0 after the menu is gone, in both browsers and also in 1a6383d2:
that is the left Browser pane list area, which is black in the page (known, unchanged).

## Worker CPU with the F8 picker (ms per second, Worker thread; load average 10-17, noisy)

| phase | Chrome | Firefox |
|---|---|---|
| idle before | 150 | 215 |
| fade-in 0.8 s | 560 (17 presents) | 870 (23 presents) |
| steady, picker open (3 s) | 533 (6 presents/s) | 430 (1.33 presents/s) |
| fade-out 1.2 s | 770 | 1240 |
| idle after | 145 | 200 |

Firefox runs: `openProject` (Files window) times out when one Firefox process reloads the page several times,
so the Firefox run uses one process per scene that opens the project (tools/run-final.sh).
