# lift/harness/pick1 - measuring which drawing path FL takes (decision 0242)

**Everything here is a measuring tool of the NATIVE build and of the harness. It is not part of the product and changes nothing the guest or the runtime does.** The input of every run is a G3 script (a harness simulation, not a user operation); the engine that runs is the real one (the generated C of FLEngine_x64.dll and its five bundled DLLs, clang -O2, the runtime at -O2, upstream Blend2D).

| file | what |
|---|---|
| `apiw.h`, `apiw_core.c`, `gen-apiwrap.py` | API recorder. `gen-apiwrap.py` writes a link-time wrapper (`-Wl,--wrap=fle_imp_NAME`) for every runtime import (831). Every call is counted; the probe APIs (capability questions) and the event APIs (window creation / destruction, layered, Dwm, GDI blits) keep their arguments (strings, GUIDs, rectangles, margins) and the return value. Output file `$APIW_OUT`: `#dump N` blocks written at every `cpu LABEL` line of a native G3 script (the wrapper of `getrusage`): `A name count` (calls since the last dump), `P ...` (distinct probe rows), `E clock name ...` (events in order). |
| `pix_wrap.c` | counters of the raster primitives (`fle_r_blit`, `rop_rect`, `mix_rect`, `stretch`, `maskblt`, `line`): pixels written per call; file `$PARITY_BLIT_LOG`, read by `lift/harness/parity/blitvol.py`. |
| `link-native.sh` | links the N1 binary again with the recorder, the pixel counters and the SIGPROF sampler (`lift/harness/parity/parity_prof.c`) from the objects `lift/harness/parity/native-build.sh` compiled. |
| `link-plain.sh` | links N1 without the recorder from a changed runtime tree (only the changed runtime objects are rebuilt) |
| `run-native.sh` | runs a G3 script on such a binary with the environment of `lift/harness/parity/run.sh` (VFS seed `vfs-seed-g8.tsv`, registry seeds, window table per snapshot, `FLE_G3_DUMP_DIR` for the surfaces). `PROF=1` turns the sampler on. |
| `ops-pick1.tsv`, `mk-pick1-scripts.py` | the operation list (picker, menus, drop-down, tools, mixer) and the generator of `pk.native.g3` / `pk.wasm.g3`. |
| `mk-api-table.py`, `mk-path-matrix.py`, `prof-modules.py`, `parse-gdb-log.py` | tables from the recorder output (API counts per operation, the per-operation path matrix, module shares of the samples by the object file of the link map, calls by phase). |
| `gdb-api.py` | the same recording with gdb breakpoints on an N1 binary that was built without the recorder (slow, used for the first look; needs `set breakpoint pending on`). |
| `mk-cap-table.py` | the capability-probe table (`capability-probes.tsv` / `.md`). |
| `raw2png.py`, `html2txt.py`, `xref.py` | surface dump (BGRA) to PNG; Learn page to text; cross references in FLEngine_x64.dll (lea / call / string / function start). |

Reproduction (from a `git archive` of the repository with the generated C of the production preset in `$WORK`, see `docs/decisions/0242-fl-drawing-paths.md`, section 9):
```
JOBS=4 bash lift/harness/parity/native-build.sh $WORK OUT/two-native-plain UP_LIB UP_SRC
bash lift/harness/pick1/link-native.sh $WORK OUT/two-native-api-fc
python3 lift/harness/pick1/mk-pick1-scripts.py OUT/scripts
PICK1_DLLD=$WORK/onat/v-*/dll PROF=1 bash lift/harness/pick1/run-native.sh OUT/two-native-api-fc OUT/scripts/pk.native.g3 OUT/run
```

## The `stk=` guess of `InvalidateRect` (apiw_core.c, `APIW_D_STK`)
It lists the words of the guest stack that point just behind a call instruction (5-byte `call rel32`, `call [reg+disp]` of 4/6/7 bytes, and, since the 08:58 fix, 2- and 3-byte `call reg` / `call [reg+disp8]`). It is a GUESS: the window is 192 words, and words left on the stack by earlier calls (stale values, e.g. in a large local area) look the same as live return addresses. Use it to find candidates, not as a call chain.

## cr_plus_more (decision 0242, the extension of 2026-11-05)
| file | what |
|---|---|
| `gdb/cr.gdb`, `gdb/cr-e1.gdb`, `gdb/cr-e3.gdb` | gdb scripts for an N1 binary built with symbols (`send_to`, `do_activate`, `destroy_tree` are static functions of `fle_win.c` that stay in the symbol table): they print, in call order, the guest's window / activation calls (`API`), the messages the runtime sent because of them (`M`) and every activation (`ACT`). `cr-e1` / `cr-e3` are the diagnostic experiments (skip the activation of the pop-up windows, handles 136 / 137 = 0x7f990224 / 0x7f990228: all of it / only the one that comes from `ShowWindow`). Run: `RUNNER="gdb -q -batch -x gdb/cr.gdb --args" bash run-native.sh BIN scripts/cr2.native.g3 OUT` (RUNNER is read by `run-native.sh`). |
| `mk-act-seq.py` | the activation sequence table of the second pop-up out of a gdb event log |
| `mk-act-blocks.py`, `mk-act-matrix.py` | the `@@ step` rendering that `crates/fe-lift/tests/win2_act.rs` pins, and the cell-by-cell comparison Wine 10.0 / runtime of `harness/api2/win_act.c` scenario 14 (the 250-cell matrix: `SCEN=14 bash lift/harness/api2/build.sh act WORKDIR`, then `mk-act-blocks.py` on `act_wine.out` and `act_native.out`, then `mk-act-matrix.py WINE.blocks RUNTIME.blocks OUT.tsv`) |
| `cmp-g3.py` | two G3 runs compared snapshot by snapshot (surface hashes, window table) |
| `mk-before.sh` | the "before" recorder binary that differs from the fixed one only by `fle_win.o` |
| `mk-doc-edit.py` | the edit of the merged decision text (the correction of section 2.4 and the new section 2.5) |
