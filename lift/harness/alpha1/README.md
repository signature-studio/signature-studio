# harness/alpha1 (decision 0244: translucent windows)

| file | what |
|---|---|
| `glass_seq.c` | A native program on the runtime itself (fle_win, fle_gdi, fle_raster, fle_dwm, fle_desk): the sheet of glass, the margins, the alpha byte of BitBlt, SetWindowRgn (shown part and hit test), a fade, and the layered windows (ULW / SLWA). The expected pixels are literals made by `docs/investigations/alpha1/tools/mk-expect.py` from the formula of Microsoft Learn in exact arithmetic. Built and run by `lift/crates/fe-lift/tests/alpha1_glass.rs` (cargo test). |
| `check-fade.py`, `run-fade.sh`, `g3/` | The same on the real engine: the G3 scripts (`pull1.g3`: File menu then the Add menu, a row chosen; `f8.g3`: the plug-in picker) are run on the native build with the recorder; `check-fade.py` takes the BitBlt source of the fading row (A and premultiplied colour of the guest) and the composed pixel of every frame and checks `composite = src + (1 - A/255) * under` (+-1) and that the colour stays between the first frame and `under`. Needs a native build of the lifted engine, so it is not part of the CI. |

The mutation list for `glass_seq.c` (27 mutants, all detected) is `docs/investigations/alpha1/tools/mutate-glass.py`.
