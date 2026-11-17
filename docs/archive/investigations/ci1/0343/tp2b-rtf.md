## RT-F: UI worker = tid 3045506 (most RunTask: 14315); Workers: [3045506]

### thread CPU fraction inside W (tPage-200 ms .. tPage+200 ms), all File clicks + Escapes: d(tts)/d(ts) from the events of each thread
| thread | role | events | wall ms | CPU ms | CPU fraction |
|---|---|---|---|---|---|
| CrRendererMain 3045351 | CrRendererMain | 38049 | 16853 | 6820 | 0.40 |
| DedicatedWorker thread 3045506 | UI worker | 31307 | 17688 | 12324 | 0.70 |
| DedicatedWorker thread 3045709 | Worker | 24735 | 17562 | 4456 | 0.25 |
| DedicatedWorker thread 3045712 | Worker | 19316 | 17454 | 9279 | 0.53 |
| DedicatedWorker thread 3045711 | Worker | 17288 | 17559 | 4305 | 0.25 |
| Realtime AudioWorklet thread 3045505 | Realtime AudioWorklet thread | 15729 | 17487 | 1328 | 0.08 |
| DedicatedWorker thread 3045714 | Worker | 12960 | 17537 | 3687 | 0.21 |
| DedicatedWorker thread 3045708 | Worker | 11112 | 17470 | 3391 | 0.19 |
| DedicatedWorker thread 3045710 | Worker | 10740 | 17461 | 3442 | 0.20 |
| DedicatedWorker thread 3045713 | Worker | 10287 | 17480 | 3332 | 0.19 |
| Compositor 3045369 | Compositor | 10184 | 15105 | 333 | 0.02 |

### File click (n=12): ms per input by self class, phase C = tInject .. firstFrame, and window W (400 ms)
| thread | role | phase | idle | program | wasm | js | gc | total ms |
|---|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3045506 | UI worker | C | 0.2 | 8.3 | 33.5 | 0.2 | 0.0 | 42.2 |
| DedicatedWorker thread 3045506 | UI worker | W | 107.6 | 84.2 | 199.7 | 8.0 | 0.4 | 400.0 |
| CrRendererMain 3045351 | page main | C | 37.3 | 4.8 | 0.0 | 0.3 | 0.0 | 42.3 |
| CrRendererMain 3045351 | page main | W | 279.9 | 43.5 | 0.0 | 72.4 | 4.2 | 400.0 |
| Realtime AudioWorklet thread 3045505 | audio worklet | C | 42.3 | 0.0 | 0.0 | 0.0 | 0.0 | 42.3 |
| Realtime AudioWorklet thread 3045505 | audio worklet | W | 399.7 | 0.1 | 0.0 | 0.2 | 0.0 | 400.1 |
| DedicatedWorker thread 3045708 | Worker | C | 0.0 | 39.1 | 3.2 | 0.0 | 0.0 | 42.3 |
| DedicatedWorker thread 3045708 | Worker | W | 0.0 | 368.1 | 31.1 | 0.5 | 0.1 | 399.9 |
| DedicatedWorker thread 3045709 | Worker | C | 0.0 | 38.8 | 3.4 | 0.2 | 0.1 | 42.5 |
| DedicatedWorker thread 3045709 | Worker | W | 0.0 | 367.1 | 31.5 | 0.9 | 0.1 | 400.2 |
| DedicatedWorker thread 3045710 | Worker | C | 0.0 | 36.8 | 5.5 | 0.0 | 0.0 | 42.4 |
| DedicatedWorker thread 3045710 | Worker | W | 0.0 | 365.3 | 34.3 | 0.4 | 0.1 | 400.2 |
| DedicatedWorker thread 3045711 | Worker | C | 0.0 | 37.5 | 4.7 | 0.1 | 0.0 | 42.3 |
| DedicatedWorker thread 3045711 | Worker | W | 0.0 | 358.4 | 40.5 | 0.4 | 0.3 | 399.8 |
| DedicatedWorker thread 3045712 | Worker | C | 0.0 | 30.6 | 11.7 | 0.0 | 0.0 | 42.3 |
| DedicatedWorker thread 3045712 | Worker | W | 0.0 | 262.3 | 136.0 | 1.0 | 0.2 | 399.9 |
| DedicatedWorker thread 3045713 | Worker | C | 0.0 | 39.0 | 3.4 | -0.0 | 0.0 | 42.4 |
| DedicatedWorker thread 3045713 | Worker | W | 0.0 | 369.4 | 30.1 | 0.4 | 0.0 | 400.1 |
| DedicatedWorker thread 3045714 | Worker | C | 0.0 | 38.8 | 3.4 | 0.1 | 0.0 | 42.3 |
| DedicatedWorker thread 3045714 | Worker | W | 0.0 | 364.2 | 34.9 | 0.6 | 0.0 | 400.1 |

#### UI worker functions in phase C, File click: ms per input (total 42.2 ms)
| function | self ms | inclusive ms |
|---|---|---|
| g9_hook | 3.93 | 31.82 |
| wasm-function[4254] | 0.00 | 31.77 |
| f_1375990 | 0.00 | 31.77 |
| f_86a660 | 0.00 | 31.77 |
| f_1375950 | 0.00 | 31.77 |
| wasm-function[1390] | 0.00 | 31.77 |
| wasm-function[1389] | 0.00 | 31.77 |
| dispatch_message | 0.02 | 31.60 |
| fle_call_guest | 0.00 | 31.60 |
| __wrap_fle_imp_DispatchMessageW | 0.00 | 31.56 |
| wasm-function[3298] | 0.00 | 31.56 |
| fle_imp_DispatchMessageW | 0.00 | 31.56 |
| wasm-function[721] | 0.09 | 31.52 |
| f_612940 | 0.00 | 31.52 |
| wasm-function[3044] | 0.07 | 31.52 |
| wasm-function[2518] | 0.10 | 31.52 |
| wasm-function[669] | 0.05 | 31.52 |
| f_7250f0 | 0.00 | 31.49 |
| f_724fd0 | 0.00 | 31.47 |
| wasm-function[1362] | 0.00 | 27.88 |
| fle_dispatch | 0.11 | 27.23 |
| fle_hyb_dispatch | 0.06 | 25.84 |
| wasm-function[3754] | 0.02 | 24.47 |
| wasm-function[1064] | 0.06 | 24.45 |
| wasm-function[4930] | 0.04 | 24.30 |
| f_840960 | 0.00 | 24.18 |
| wasm-function[3037] | 0.05 | 24.18 |
| wasm-function[3154] | 0.00 | 24.13 |

(self-time top 12): wasm-function[1368] 12.41; (program) 8.30; wasm-function[3035] 4.45; g9_hook 3.93; fle_desk_compose 1.67; wasm-function[3145] 1.58; wasm-function[5212] 0.97; fle_gil_wake 0.51; wasm-function[3590] 0.36; wasm-function[973] 0.24; fle_gil_acquire_slow 0.23; poll_waiters 0.23

### Escape (n=11): ms per input by self class, phase C = tInject .. firstFrame, and window W (400 ms)
| thread | role | phase | idle | program | wasm | js | gc | total ms |
|---|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3045506 | UI worker | C | 0.0 | 2.0 | 8.4 | 0.3 | 0.0 | 10.8 |
| DedicatedWorker thread 3045506 | UI worker | W | 115.8 | 95.0 | 180.3 | 8.1 | 0.7 | 399.9 |
| CrRendererMain 3045351 | page main | C | 9.0 | 1.9 | 0.0 | 0.1 | 0.0 | 10.9 |
| CrRendererMain 3045351 | page main | W | 187.7 | 52.2 | 0.0 | 151.3 | 8.8 | 400.1 |
| Realtime AudioWorklet thread 3045505 | audio worklet | C | 11.0 | 0.0 | 0.0 | 0.0 | 0.0 | 11.0 |
| Realtime AudioWorklet thread 3045505 | audio worklet | W | 399.2 | 0.3 | 0.0 | 0.3 | 0.0 | 399.8 |
| DedicatedWorker thread 3045708 | Worker | C | 0.0 | 10.3 | 0.6 | 0.0 | 0.0 | 10.9 |
| DedicatedWorker thread 3045708 | Worker | W | 0.0 | 368.8 | 30.5 | 0.4 | 0.0 | 399.9 |
| DedicatedWorker thread 3045709 | Worker | C | 0.0 | 10.3 | 0.7 | 0.0 | 0.0 | 11.0 |
| DedicatedWorker thread 3045709 | Worker | W | 0.0 | 368.8 | 29.6 | 0.9 | 0.4 | 399.9 |
| DedicatedWorker thread 3045710 | Worker | C | 0.0 | 10.1 | 0.8 | -0.0 | 0.0 | 10.9 |
| DedicatedWorker thread 3045710 | Worker | W | 0.0 | 363.5 | 35.9 | 0.4 | 0.1 | 400.1 |
| DedicatedWorker thread 3045711 | Worker | C | 0.0 | 9.8 | 1.2 | 0.0 | -0.0 | 10.9 |
| DedicatedWorker thread 3045711 | Worker | W | 0.0 | 350.6 | 48.5 | 0.4 | 0.1 | 399.9 |
| DedicatedWorker thread 3045712 | Worker | C | 0.0 | 6.1 | 4.8 | 0.1 | 0.0 | 11.0 |
| DedicatedWorker thread 3045712 | Worker | W | 0.0 | 250.1 | 148.5 | 1.0 | 0.1 | 400.0 |
| DedicatedWorker thread 3045713 | Worker | C | 0.0 | 10.0 | 0.9 | 0.0 | 0.0 | 10.9 |
| DedicatedWorker thread 3045713 | Worker | W | 0.0 | 365.6 | 33.8 | 0.3 | 0.0 | 400.0 |
| DedicatedWorker thread 3045714 | Worker | C | 0.0 | 9.9 | 1.0 | 0.0 | 0.0 | 11.0 |
| DedicatedWorker thread 3045714 | Worker | W | 0.0 | 360.3 | 39.1 | 0.3 | 0.1 | 399.9 |

#### UI worker functions in phase C, Escape: ms per input (total 10.8 ms)
| function | self ms | inclusive ms |
|---|---|---|
| g9_hook | 0.64 | 7.72 |
| wasm-function[4254] | 0.00 | 7.67 |
| f_1375990 | 0.00 | 7.67 |
| f_86a660 | 0.00 | 7.67 |
| f_1375950 | 0.00 | 7.67 |
| wasm-function[1390] | 0.00 | 7.67 |
| fle_call_guest | 0.02 | 7.30 |
| wasm-function[3044] | 0.00 | 7.13 |
| wasm-function[721] | 0.00 | 7.09 |
| wasm-function[2518] | 0.03 | 7.09 |
| wasm-function[669] | 0.00 | 7.09 |
| wasm-function[1362] | 0.00 | 6.91 |
| fle_hyb_dispatch | 0.13 | 5.40 |
| f_86b440 | 0.00 | 4.59 |
| wasm-function[4930] | 0.00 | 4.48 |
| wasm-function[4255] | 0.00 | 4.46 |
| wasm-function[1366] | 0.12 | 4.46 |
| wasm-function[4225] | 0.00 | 4.46 |
| wasm-function[3254] | 0.03 | 4.34 |
| wasm-function[3037] | 0.06 | 4.31 |
| wasm-function[3154] | 0.00 | 4.24 |
| wasm-function[4261] | 0.00 | 4.21 |
| f_868c00 | 0.00 | 4.21 |
| wasm-function[1368] | 2.87 | 4.17 |
| wasm-function[4229] | 0.00 | 4.17 |
| dispatch_message | 0.00 | 3.08 |
| wasm-function[1389] | 0.00 | 3.08 |
| f_612940 | 0.00 | 2.92 |

(self-time top 12): wasm-function[1368] 2.87; (program) 2.03; fle_desk_compose 0.67; g9_hook 0.64; wasm-function[3145] 0.41; wasm-function[3035] 0.40; fle_gil_wake 0.36; wasm-function[6348] 0.19; postMessage 0.13; fle_hyb_dispatch 0.13; wasm-function[531] 0.12; wasm-function[1366] 0.12
