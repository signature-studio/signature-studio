## RT-H: UI worker = tid 3303702 (most RunTask: 14691); Workers: [3303702]

### thread CPU fraction inside W (tPage-200 ms .. tPage+200 ms), all File clicks + Escapes: d(tts)/d(ts) from the events of each thread
| thread | role | events | wall ms | CPU ms | CPU fraction |
|---|---|---|---|---|---|
| CrRendererMain 3303574 | CrRendererMain | 35047 | 15317 | 6336 | 0.41 |
| DedicatedWorker thread 3303702 | UI worker | 29916 | 16007 | 11177 | 0.70 |
| DedicatedWorker thread 3303947 | Worker | 21798 | 15865 | 3995 | 0.25 |
| DedicatedWorker thread 3303950 | Worker | 17816 | 15788 | 8472 | 0.54 |
| DedicatedWorker thread 3303949 | Worker | 15681 | 15798 | 3487 | 0.22 |
| Realtime AudioWorklet thread 3303701 | Realtime AudioWorklet thread | 14384 | 15975 | 1231 | 0.08 |
| DedicatedWorker thread 3303952 | Worker | 11703 | 15872 | 3378 | 0.21 |
| DedicatedWorker thread 3303951 | Worker | 9842 | 15735 | 3154 | 0.20 |
| DedicatedWorker thread 3303946 | Worker | 9846 | 15777 | 3003 | 0.19 |
| DedicatedWorker thread 3303948 | Worker | 9368 | 15770 | 2951 | 0.19 |
| Compositor 3303594 | Compositor | 9542 | 14257 | 320 | 0.02 |

### File click (n=12): ms per input by self class, phase C = tInject .. firstFrame, and window W (400 ms)
| thread | role | phase | idle | program | wasm | js | gc | total ms |
|---|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3303702 | UI worker | C | 0.2 | 8.2 | 31.2 | 0.3 | 0.0 | 39.8 |
| DedicatedWorker thread 3303702 | UI worker | W | 106.4 | 88.7 | 195.0 | 8.9 | 1.0 | 400.2 |
| CrRendererMain 3303574 | page main | C | 35.7 | 4.2 | 0.0 | 0.1 | 0.0 | 40.0 |
| CrRendererMain 3303574 | page main | W | 279.2 | 46.0 | 0.0 | 70.5 | 4.2 | 399.9 |
| Realtime AudioWorklet thread 3303701 | audio worklet | C | 39.8 | 0.1 | 0.0 | 0.0 | 0.0 | 39.9 |
| Realtime AudioWorklet thread 3303701 | audio worklet | W | 399.4 | 0.5 | 0.0 | 0.3 | 0.0 | 400.2 |
| DedicatedWorker thread 3303946 | Worker | C | 0.0 | 36.7 | 3.2 | 0.1 | 0.0 | 40.0 |
| DedicatedWorker thread 3303946 | Worker | W | 0.0 | 368.0 | 30.9 | 0.7 | 0.1 | 399.9 |
| DedicatedWorker thread 3303947 | Worker | C | 0.0 | 36.5 | 3.4 | 0.0 | 0.0 | 39.9 |
| DedicatedWorker thread 3303947 | Worker | W | 0.0 | 365.2 | 33.3 | 0.7 | 0.2 | 399.9 |
| DedicatedWorker thread 3303948 | Worker | C | 0.0 | 37.7 | 2.3 | 0.0 | 0.0 | 40.0 |
| DedicatedWorker thread 3303948 | Worker | W | 0.0 | 369.7 | 29.5 | 0.7 | 0.1 | 400.0 |
| DedicatedWorker thread 3303949 | Worker | C | 0.0 | 37.5 | 2.4 | 0.0 | 0.0 | 40.0 |
| DedicatedWorker thread 3303949 | Worker | W | 0.0 | 367.5 | 31.7 | 0.4 | 0.1 | 400.1 |
| DedicatedWorker thread 3303950 | Worker | C | 0.0 | 25.9 | 13.8 | 0.1 | 0.0 | 39.9 |
| DedicatedWorker thread 3303950 | Worker | W | 0.0 | 261.0 | 136.9 | 1.1 | 0.3 | 400.1 |
| DedicatedWorker thread 3303951 | Worker | C | 0.0 | 36.2 | 3.6 | 0.1 | 0.0 | 39.9 |
| DedicatedWorker thread 3303951 | Worker | W | 0.0 | 364.4 | 34.8 | 0.4 | 0.2 | 399.9 |
| DedicatedWorker thread 3303952 | Worker | C | 0.0 | 37.1 | 2.7 | 0.2 | 0.0 | 40.0 |
| DedicatedWorker thread 3303952 | Worker | W | 0.0 | 363.0 | 35.6 | 0.9 | 0.1 | 399.9 |

#### UI worker functions in phase C, File click: ms per input (total 39.8 ms)
| function | self ms | inclusive ms |
|---|---|---|
| f_1375950 | 0.00 | 29.29 |
| f_1375990 | 0.00 | 29.29 |
| wasm-function[1390] | 0.00 | 29.29 |
| wasm-function[4254] | 0.00 | 29.29 |
| f_86a660 | 0.00 | 29.29 |
| fle_call_guest | 0.07 | 29.15 |
| wasm-function[3044] | 0.06 | 29.15 |
| wasm-function[2518] | 0.19 | 29.13 |
| wasm-function[721] | 0.00 | 29.13 |
| wasm-function[669] | 0.05 | 29.13 |
| wasm-function[1389] | 0.00 | 29.03 |
| f_612940 | 0.07 | 28.97 |
| dispatch_message | 0.00 | 28.95 |
| __wrap_fle_imp_DispatchMessageW | 0.00 | 28.91 |
| fle_imp_DispatchMessageW | 0.00 | 28.91 |
| wasm-function[3298] | 0.00 | 28.74 |
| f_724fd0 | 0.00 | 28.58 |
| f_7250f0 | 0.00 | 28.58 |
| wasm-function[1362] | 0.02 | 23.75 |
| fle_dispatch | 0.09 | 22.72 |
| fle_hyb_dispatch | 0.21 | 20.79 |
| wasm-function[3037] | 0.13 | 19.18 |
| wasm-function[1064] | 0.02 | 19.17 |
| wasm-function[3754] | 0.00 | 19.17 |
| f_840960 | 0.06 | 19.06 |
| wasm-function[3154] | 0.17 | 19.05 |
| wasm-function[4930] | 0.00 | 19.04 |
| f_868c00 | 0.08 | 18.57 |

(self-time top 12): wasm-function[1368] 12.48; (program) 8.16; wasm-function[3035] 3.94; fle_desk_compose 1.83; fle_gil_wake 0.82; fle_gil_acquire_slow 0.70; wasm-function[2138] 0.68; fle_r_blit 0.40; poll_waiters 0.25; wasm-function[2130] 0.25; wasm-function[1746] 0.22; fle_hyb_dispatch 0.21

### Escape (n=12): ms per input by self class, phase C = tInject .. firstFrame, and window W (400 ms)
| thread | role | phase | idle | program | wasm | js | gc | total ms |
|---|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3303702 | UI worker | C | 0.0 | 2.4 | 7.1 | 0.1 | 0.0 | 9.7 |
| DedicatedWorker thread 3303702 | UI worker | W | 115.8 | 103.6 | 171.3 | 8.5 | 0.6 | 399.9 |
| CrRendererMain 3303574 | page main | C | 8.6 | 1.1 | 0.0 | 0.0 | 0.0 | 9.7 |
| CrRendererMain 3303574 | page main | W | 181.8 | 55.7 | 0.0 | 152.0 | 10.5 | 400.1 |
| Realtime AudioWorklet thread 3303701 | audio worklet | C | 9.8 | 0.0 | 0.0 | -0.0 | 0.0 | 9.8 |
| Realtime AudioWorklet thread 3303701 | audio worklet | W | 399.5 | 0.2 | 0.0 | 0.3 | 0.0 | 400.0 |
| DedicatedWorker thread 3303946 | Worker | C | 0.0 | 8.8 | 0.8 | 0.1 | 0.0 | 9.7 |
| DedicatedWorker thread 3303946 | Worker | W | 0.0 | 372.4 | 26.6 | 0.8 | 0.0 | 400.0 |
| DedicatedWorker thread 3303947 | Worker | C | 0.0 | 9.2 | 0.6 | 0.0 | 0.0 | 9.7 |
| DedicatedWorker thread 3303947 | Worker | W | 0.0 | 372.4 | 26.5 | 0.9 | 0.3 | 400.1 |
| DedicatedWorker thread 3303948 | Worker | C | 0.0 | 9.2 | 0.7 | 0.0 | 0.0 | 9.9 |
| DedicatedWorker thread 3303948 | Worker | W | 0.0 | 371.1 | 28.0 | 0.5 | 0.1 | 400.1 |
| DedicatedWorker thread 3303949 | Worker | C | 0.0 | 8.8 | 0.9 | 0.0 | 0.0 | 9.7 |
| DedicatedWorker thread 3303949 | Worker | W | 0.0 | 363.6 | 35.8 | 0.5 | 0.0 | 400.1 |
| DedicatedWorker thread 3303950 | Worker | C | 0.0 | 5.9 | 3.6 | 0.0 | 0.1 | 9.6 |
| DedicatedWorker thread 3303950 | Worker | W | 0.0 | 251.0 | 147.0 | 1.3 | 0.3 | 400.0 |
| DedicatedWorker thread 3303951 | Worker | C | 0.0 | 9.1 | 0.6 | 0.0 | 0.0 | 9.7 |
| DedicatedWorker thread 3303951 | Worker | W | 0.0 | 365.0 | 34.3 | 0.3 | 0.1 | 399.9 |
| DedicatedWorker thread 3303952 | Worker | C | 0.0 | 9.3 | 0.6 | -0.0 | 0.0 | 9.9 |
| DedicatedWorker thread 3303952 | Worker | W | 0.0 | 364.6 | 34.4 | 0.6 | 0.1 | 400.0 |

#### UI worker functions in phase C, Escape: ms per input (total 9.7 ms)
| function | self ms | inclusive ms |
|---|---|---|
| wasm-function[1390] | 0.00 | 5.71 |
| wasm-function[4254] | 0.00 | 5.71 |
| f_1375950 | 0.00 | 5.71 |
| f_1375990 | 0.00 | 5.71 |
| f_86a660 | 0.00 | 5.71 |
| fle_call_guest | 0.00 | 5.31 |
| wasm-function[3044] | 0.00 | 5.24 |
| wasm-function[721] | 0.08 | 5.13 |
| wasm-function[2518] | 0.00 | 5.06 |
| wasm-function[669] | 0.00 | 5.06 |
| wasm-function[1362] | 0.00 | 4.95 |
| fle_hyb_dispatch | 0.05 | 3.64 |
| wasm-function[4930] | 0.05 | 3.42 |
| f_86b440 | 0.00 | 3.29 |
| wasm-function[4255] | 0.08 | 3.25 |
| wasm-function[4225] | 0.04 | 3.16 |
| wasm-function[1366] | 0.21 | 3.13 |
| wasm-function[3037] | 0.03 | 2.91 |
| wasm-function[3254] | 0.00 | 2.91 |
| f_868c00 | 0.00 | 2.88 |
| wasm-function[4261] | 0.00 | 2.88 |
| wasm-function[3154] | 0.00 | 2.88 |
| wasm-function[4229] | 0.00 | 2.77 |
| wasm-function[1368] | 2.00 | 2.76 |
| (program) | 2.45 | 2.45 |
| dispatch_message | 0.00 | 2.42 |
| wasm-function[1389] | 0.00 | 2.42 |
| f_612940 | 0.06 | 2.36 |

(self-time top 12): (program) 2.45; wasm-function[1368] 2.00; fle_desk_compose 1.36; wasm-function[3035] 0.64; wasm-function[1366] 0.21; fle_gil_wake 0.21; wasm-function[531] 0.16; fle_gil_acquire_slow 0.14; wasm-function[5235] 0.11; wasm-function[1321] 0.10; wasm-function[3590] 0.10; wasm-function[48] 0.09
