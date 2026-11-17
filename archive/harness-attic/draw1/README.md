# lift/harness/draw1 — the cost of the F8 full-screen plug-in list (decision 0203)

Measurement tools of draw1 and the two opt-in changes it tested. Paths in the scripts point at the layout of the work tree `lift/out-draw1/` (a `git archive` of HEAD in `src/`, builds in `F1xHn/` ...); override with the environment variables named at the top of each script.

Opt-in changes (the default build and the default generated C are unchanged):
- SRCCOPY (always on, decision 0243; it was the compile switch `FLE_R_FAST_COPY`) between two direct 32-bit surfaces without the X byte kept is a masked word copy instead of the pixel loop. Same pixels.

Tools: `f8run.mjs` (Chrome: F8 open/close on a bundle, Worker CPU, draws, V8 profile), `prof-*.mjs` (profile tables), `ab.sh` / `ab-sum.mjs` (alternating sessions), `g3node.sh` / `g3cmp16.sh` / `g3cpu.sh` (node: G3 scripts `f8.g3`, `f8-control.g3`, the 16 snapshots), `extract-fn.sh` (disassemble wasm functions by index).

More: `ab2-sum.mjs` / `g3cpu-sum.mjs` / `prof-diff.mjs` (tables), `play60.g3` / `menus.g3` (scripts for the check runs).
