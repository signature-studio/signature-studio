## RT-G: UI worker = tid 3103623 (most RunTask: 15589); Workers: [3103623]

### thread CPU fraction inside W (tPage-200 ms .. tPage+200 ms), all File clicks + Escapes: d(tts)/d(ts) from the events of each thread
| thread | role | events | wall ms | CPU ms | CPU fraction |
|---|---|---|---|---|---|
| DedicatedWorker thread 3103623 | UI worker | 35765 | 16861 | 11817 | 0.70 |
| CrRendererMain 3103359 | CrRendererMain | 37631 | 15886 | 6597 | 0.42 |
| DedicatedWorker thread 3103829 | Worker | 24767 | 16754 | 4193 | 0.25 |
| DedicatedWorker thread 3103832 | Worker | 21180 | 16704 | 8990 | 0.54 |
| DedicatedWorker thread 3103831 | Worker | 17630 | 16720 | 4090 | 0.24 |
| Realtime AudioWorklet thread 3103622 | Realtime AudioWorklet thread | 15008 | 16687 | 1243 | 0.07 |
| DedicatedWorker thread 3103834 | Worker | 13224 | 16788 | 3614 | 0.22 |
| DedicatedWorker thread 3103828 | Worker | 11174 | 16725 | 3438 | 0.21 |
| DedicatedWorker thread 3103833 | Worker | 10934 | 16660 | 3310 | 0.20 |
| DedicatedWorker thread 3103830 | Worker | 10881 | 16687 | 3398 | 0.20 |
| Compositor 3103379 | Compositor | 10144 | 15212 | 308 | 0.02 |

### File click (n=12): ms per input by self class, phase C = tInject .. firstFrame, and window W (400 ms)
| thread | role | phase | idle | program | wasm | js | gc | total ms |
|---|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3103623 | UI worker | C | 0.2 | 10.6 | 33.0 | 0.3 | 0.0 | 44.0 |
| DedicatedWorker thread 3103623 | UI worker | W | 107.3 | 86.4 | 196.2 | 9.3 | 0.4 | 400.0 |
| CrRendererMain 3103359 | page main | C | 39.5 | 4.6 | 0.0 | 0.2 | 0.0 | 44.2 |
| CrRendererMain 3103359 | page main | W | 279.8 | 43.0 | 0.0 | 72.3 | 4.8 | 399.9 |
| Realtime AudioWorklet thread 3103622 | audio worklet | C | 44.4 | -0.0 | 0.0 | 0.0 | 0.0 | 44.4 |
| Realtime AudioWorklet thread 3103622 | audio worklet | W | 399.9 | 0.0 | 0.0 | 0.1 | 0.0 | 400.0 |
| DedicatedWorker thread 3103828 | Worker | C | 0.0 | 39.6 | 4.8 | 0.0 | 0.0 | 44.4 |
| DedicatedWorker thread 3103828 | Worker | W | 0.0 | 364.2 | 35.2 | 0.4 | 0.0 | 400.1 |
| DedicatedWorker thread 3103829 | Worker | C | 0.0 | 41.4 | 2.7 | 0.1 | 0.0 | 44.3 |
| DedicatedWorker thread 3103829 | Worker | W | 0.0 | 370.1 | 28.3 | 0.9 | 0.2 | 399.9 |
| DedicatedWorker thread 3103830 | Worker | C | 0.0 | 40.6 | 3.8 | -0.0 | 0.0 | 44.3 |
| DedicatedWorker thread 3103830 | Worker | W | 0.0 | 362.9 | 36.4 | 0.3 | 0.0 | 399.9 |
| DedicatedWorker thread 3103831 | Worker | C | 0.0 | 39.0 | 5.3 | 0.0 | 0.0 | 44.4 |
| DedicatedWorker thread 3103831 | Worker | W | 0.0 | 359.6 | 39.4 | 0.5 | 0.1 | 400.0 |
| DedicatedWorker thread 3103832 | Worker | C | 0.0 | 29.0 | 14.9 | 0.2 | 0.0 | 44.1 |
| DedicatedWorker thread 3103832 | Worker | W | 0.0 | 257.0 | 140.8 | 1.3 | 0.0 | 400.0 |
| DedicatedWorker thread 3103833 | Worker | C | 0.0 | 41.2 | 3.1 | -0.0 | 0.1 | 44.4 |
| DedicatedWorker thread 3103833 | Worker | W | 0.0 | 364.6 | 34.5 | 0.4 | 0.1 | 400.1 |
| DedicatedWorker thread 3103834 | Worker | C | 0.0 | 39.1 | 5.1 | 0.1 | 0.0 | 44.2 |
| DedicatedWorker thread 3103834 | Worker | W | 0.0 | 362.6 | 36.7 | 0.4 | 0.1 | 400.0 |

#### UI worker functions in phase C, File click: ms per input (total 44.0 ms)
| function | self ms | inclusive ms |
|---|---|---|
| g9_hook | 3.27 | 31.58 |
| wasm-function[4254] | 0.00 | 31.57 |
| wasm-function[1390] | 0.00 | 31.57 |
| f_1375950 | 0.00 | 31.57 |
| f_86a660 | 0.00 | 31.57 |
| f_1375990 | 0.00 | 31.57 |
| wasm-function[1389] | 0.00 | 31.57 |
| dispatch_message | 0.04 | 31.53 |
| fle_call_guest | 0.02 | 31.53 |
| __wrap_fle_imp_DispatchMessageW | 0.00 | 31.50 |
| fle_imp_DispatchMessageW | 0.00 | 31.50 |
| f_612940 | 0.00 | 31.49 |
| wasm-function[721] | 0.07 | 31.49 |
| wasm-function[3298] | 0.00 | 31.49 |
| wasm-function[3044] | 0.00 | 31.49 |
| wasm-function[669] | 0.10 | 31.48 |
| wasm-function[2518] | 0.08 | 31.48 |
| f_724fd0 | 0.00 | 31.36 |
| f_7250f0 | 0.00 | 31.36 |
| wasm-function[1362] | 0.15 | 27.95 |
| fle_dispatch | 0.22 | 26.84 |
| fle_hyb_dispatch | 0.11 | 26.12 |
| wasm-function[1064] | 0.08 | 24.20 |
| wasm-function[3754] | 0.00 | 24.20 |
| wasm-function[4930] | 0.05 | 24.04 |
| f_840960 | 0.02 | 24.00 |
| wasm-function[3037] | 0.04 | 23.87 |
| wasm-function[3154] | 0.09 | 23.79 |

(self-time top 12): wasm-function[1368] 11.35; (program) 10.58; wasm-function[3035] 5.04; g9_hook 3.27; wasm-function[3145] 1.94; fle_desk_compose 1.27; wasm-function[5212] 1.18; fle_gil_wake 1.06; wasm-function[965] 0.38; wasm-function[973] 0.33; wasm-function[1746] 0.26; fle_dispatch 0.22

### Escape (n=12): ms per input by self class, phase C = tInject .. firstFrame, and window W (400 ms)
| thread | role | phase | idle | program | wasm | js | gc | total ms |
|---|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3103623 | UI worker | C | 0.0 | 1.9 | 6.7 | 0.1 | 0.0 | 8.6 |
| DedicatedWorker thread 3103623 | UI worker | W | 111.5 | 103.2 | 175.9 | 8.3 | 0.6 | 400.0 |
| CrRendererMain 3103359 | page main | C | 7.7 | 1.3 | 0.0 | 0.0 | 0.0 | 9.0 |
| CrRendererMain 3103359 | page main | W | 174.7 | 49.2 | 0.0 | 165.6 | 10.4 | 399.9 |
| Realtime AudioWorklet thread 3103622 | audio worklet | C | 9.0 | 0.0 | 0.0 | 0.0 | 0.0 | 9.0 |
| Realtime AudioWorklet thread 3103622 | audio worklet | W | 399.3 | 0.3 | 0.0 | 0.3 | 0.0 | 400.0 |
| DedicatedWorker thread 3103828 | Worker | C | 0.0 | 8.4 | 0.6 | 0.0 | 0.0 | 9.0 |
| DedicatedWorker thread 3103828 | Worker | W | 0.0 | 364.9 | 34.1 | 0.7 | 0.2 | 400.0 |
| DedicatedWorker thread 3103829 | Worker | C | 0.0 | 8.2 | 0.9 | 0.0 | 0.0 | 9.1 |
| DedicatedWorker thread 3103829 | Worker | W | 0.0 | 368.9 | 30.3 | 0.5 | 0.2 | 400.0 |
| DedicatedWorker thread 3103830 | Worker | C | 0.0 | 8.2 | 0.9 | 0.0 | 0.0 | 9.1 |
| DedicatedWorker thread 3103830 | Worker | W | 0.0 | 362.5 | 36.6 | 0.3 | 0.1 | 399.9 |
| DedicatedWorker thread 3103831 | Worker | C | 0.0 | 7.6 | 1.3 | 0.0 | 0.0 | 8.9 |
| DedicatedWorker thread 3103831 | Worker | W | 0.0 | 355.2 | 43.1 | 1.2 | 0.2 | 400.0 |
| DedicatedWorker thread 3103832 | Worker | C | 0.0 | 5.7 | 3.0 | 0.2 | 0.0 | 9.0 |
| DedicatedWorker thread 3103832 | Worker | W | 0.0 | 254.6 | 142.9 | 1.7 | 0.1 | 400.1 |
| DedicatedWorker thread 3103833 | Worker | C | 0.0 | 7.9 | 1.0 | 0.0 | 0.0 | 9.0 |
| DedicatedWorker thread 3103833 | Worker | W | 0.0 | 365.8 | 33.3 | 0.9 | 0.1 | 400.2 |
| DedicatedWorker thread 3103834 | Worker | C | 0.0 | 8.0 | 0.9 | 0.0 | 0.0 | 8.9 |
| DedicatedWorker thread 3103834 | Worker | W | 0.0 | 363.1 | 35.9 | 0.7 | 0.2 | 400.1 |

#### UI worker functions in phase C, Escape: ms per input (total 8.6 ms)
| function | self ms | inclusive ms |
|---|---|---|
| wasm-function[1390] | 0.00 | 5.85 |
| f_86a660 | 0.00 | 5.85 |
| g9_hook | 0.55 | 5.85 |
| wasm-function[4254] | 0.00 | 5.85 |
| f_1375950 | 0.00 | 5.85 |
| f_1375990 | 0.00 | 5.85 |
| fle_call_guest | 0.00 | 5.53 |
| wasm-function[721] | 0.00 | 5.42 |
| wasm-function[669] | 0.00 | 5.42 |
| wasm-function[2518] | 0.00 | 5.42 |
| wasm-function[3044] | 0.00 | 5.42 |
| wasm-function[1362] | 0.00 | 5.26 |
| fle_hyb_dispatch | 0.05 | 4.34 |
| wasm-function[4930] | 0.00 | 3.93 |
| f_86b440 | 0.00 | 3.85 |
| wasm-function[1366] | 0.11 | 3.71 |
| wasm-function[4225] | 0.00 | 3.71 |
| wasm-function[4255] | 0.00 | 3.71 |
| wasm-function[3154] | 0.00 | 3.60 |
| wasm-function[3037] | 0.00 | 3.60 |
| wasm-function[3254] | 0.00 | 3.60 |
| wasm-function[4229] | 0.00 | 3.54 |
| f_868c00 | 0.00 | 3.54 |
| wasm-function[1368] | 2.05 | 3.54 |
| wasm-function[4261] | 0.00 | 3.54 |
| wasm-function[1389] | 0.00 | 2.00 |
| dispatch_message | 0.00 | 2.00 |
| f_612940 | 0.00 | 1.89 |

(self-time top 12): wasm-function[1368] 2.05; (program) 1.86; fle_desk_compose 0.82; wasm-function[3035] 0.60; g9_hook 0.55; wasm-function[3145] 0.34; wasm-function[5247] 0.18; fle_gil_wake 0.11; wasm-function[5212] 0.11; wasm-function[1366] 0.11; f_732d90 0.09; wasm-function[135378] 0.08
