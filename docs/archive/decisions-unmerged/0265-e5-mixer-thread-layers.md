profile b-play-w0.cpuprofile: total 45.11 s, idle 5.00 s; samples with an ancestor `fle_wasm_thread_entry` (the mixer thread): 15.11 s = 37.7% of the busy time

| layer | ms | of the mixer thread |
|---|---|---|
| 1 direct wasm (hybrid) | 7933 | 52.5% |
| 2 C-path guest function | 1822 | 12.1% |
| 3 SSE helper (fle_mx_*) | 2523 | 16.7% |
| 4 x87 helper (fle_x87_*) | 1416 | 9.4% |
| 5 fle_ld / fle_st / fle_cc / fle_op_* | 620 | 4.1% |
| 6 libc and the rest | 802 | 5.3% |

### who calls the helper layers (self time of the helper functions by the nearest guest function above them)

| helper layer | below a direct-wasm function ms | below a C-path function ms | no guest function above ms |
|---|---|---|---|
| SSE helper (fle_mx_*) | 368 | 2155 | 0 |
| x87 helper (fle_x87_*) | 1415 | 1 | 0 |
| fle_ld / fle_st / fle_cc / fle_op_* | 61 | 559 | 0 |

### the top 20 guest functions on the C path by cost (self + the helper layers below them)

| # | function | module | cost ms | self ms | SSE ms | x87 ms | ld/st/cc ms | reason it is not direct (fe-wasmgen report of the final tree) |
|---|---|---|---|---|---|---|---|---|
| 1 | `f_b1ce5e0` | elastique | 1193 | 438 | 541 | 1 | 213 | flag merge from a definition without recipe: Call Opaque flag 0x8 @ 0xb1cf384 |
| 2 | `f_310fb40` | dsp | 924 | 285 | 532 | 0 | 107 | lowerable (report ok) but not in the hybrid list |
| 3 | `f_b034096` | elastique | 643 | 198 | 418 | 0 | 27 | calls a function that is opaque |
| 4 | `f_b04a921` | elastique | 554 | 145 | 381 | 0 | 27 | calls a function that is opaque |
| 5 | `f_b25ce20` | elastique | 299 | 160 | 118 | 0 | 21 | flag merge from a definition without recipe: Call Opaque flag 0x8 @ 0xb25dd4c |
| 6 | `f_b2cf0c0` | elastique | 277 | 127 | 108 | 0 | 42 | flag read from a definition without recipe: Shr Opaque flag 0x1 @ 0xb2cf61b |
| 7 | `f_67ec50` | fle | 56 | 49 | 4 | 0 | 3 | lowerable (report ok) but not in the hybrid list |
| 8 | `f_30c7e20` | dsp | 45 | 31 | 10 | 0 | 4 | lowerable (report ok) but not in the hybrid list |
| 9 | `f_bb0b00` | fle | 40 | 24 | 0 | 0 | 16 | lowerable (report ok) but not in the hybrid list |
| 10 | `f_9ce490` | fle | 38 | 9 | 0 | 0 | 29 | lowerable (report ok) but not in the hybrid list |
| 11 | `f_b04a8c0` | elastique | 36 | 22 | 7 | 0 | 7 | RSP at a return or tail jump is not its entry value (a continuation entry inside another frame, or a |
| 12 | `f_b337800` | elastique | 32 | 31 | 0 | 0 | 1 | mnemonic Bsf |
| 13 | `f_b345cc0` | elastique | 32 | 17 | 14 | 0 | 1 | mnemonic Addsubps |
| 14 | `f_b034060` | elastique | 26 | 12 | 6 | 0 | 9 | RSP at a return or tail jump is not its entry value (a continuation entry inside another frame, or a |
| 15 | `f_418190` | fle | 23 | 23 | 0 | 0 | 0 | atomic read-modify-write stays on the C path (fle_cas, fle_xchg, fle_xadd) |
| 16 | `f_b00cc40` | elastique | 19 | 10 | 2 | 0 | 7 | calls a function that is opaque |
| 17 | `f_30663a0` | dsp | 18 | 14 | 4 | 0 | 1 | lowerable (report ok) but not in the hybrid list |
| 18 | `f_b00d600` | elastique | 17 | 12 | 1 | 0 | 5 | calls a function that is opaque |
| 19 | `f_1e850b0` | qfc | 12 | 7 | 0 | 0 | 5 | atomic read-modify-write stays on the C path (fle_cas, fle_xchg, fle_xadd) |
| 20 | `f_b1bb100` | elastique | 12 | 2 | 7 | 0 | 3 | lowerable (report ok) but not in the hybrid list |

### the top 20 helper / runtime functions by self time (layers 3-6)

| # | function | layer | self ms |
|---|---|---|---|
| 1 | `round_pack` | 4 | 403 |
| 2 | `fle_hyb_dispatch` | 6 | 337 |
| 3 | `fle_mx_subps_40186` | 3 | 313 |
| 4 | `fle_mx_mulps_41556` | 3 | 290 |
| 5 | `fle_mx_mulps_39890` | 3 | 253 |
| 6 | `fle_hyb_mx` | 3 | 246 |
| 7 | `fle_mx_addps_39891` | 3 | 246 |
| 8 | `binop` | 4 | 243 |
| 9 | `fle_mx_addps_41557` | 3 | 229 |
| 10 | `fle_mx_mulpd_9381` | 3 | 229 |
| 11 | `fle_ld` | 5 | 217 |
| 12 | `fle_stxa` | 5 | 212 |
| 13 | `fle_mx_addpd_9380` | 3 | 192 |
| 14 | `fle_x87_fst_m` | 4 | 145 |
| 15 | `poll_waiters` | 6 | 144 |
| 16 | `fle_mx_subpd_9379` | 3 | 128 |
| 17 | `bf_div_small` | 4 | 114 |
| 18 | `bf_mul` | 4 | 108 |
| 19 | `bf_add` | 4 | 96 |
| 20 | `fle_x87_arith_st` | 4 | 93 |
