## RT-F: UI worker = tid 2975166 (most RunTask: 13898); Workers: [2975166]

### thread CPU fraction inside W (tPage-200 ms .. tPage+200 ms), all File clicks + Escapes: d(tts)/d(ts) from the events of each thread
| thread | role | events | wall ms | CPU ms | CPU fraction |
|---|---|---|---|---|---|
| CrRendererMain 2974534 | CrRendererMain | 33544 | 14002 | 5720 | 0.41 |
| DedicatedWorker thread 2975166 | UI worker | 28485 | 14955 | 10119 | 0.68 |
| DedicatedWorker thread 2975497 | Worker | 22380 | 14878 | 3866 | 0.26 |
| DedicatedWorker thread 2975500 | Worker | 17625 | 14847 | 7839 | 0.53 |
| DedicatedWorker thread 2975499 | Worker | 15521 | 14859 | 3434 | 0.23 |
| Realtime AudioWorklet thread 2975165 | Realtime AudioWorklet thread | 13332 | 14821 | 1103 | 0.07 |
| DedicatedWorker thread 2975502 | Worker | 11623 | 14896 | 3089 | 0.21 |
| DedicatedWorker thread 2975496 | Worker | 9775 | 14814 | 3001 | 0.20 |
| DedicatedWorker thread 2975498 | Worker | 9496 | 14844 | 2868 | 0.19 |
| DedicatedWorker thread 2975501 | Worker | 9606 | 14832 | 2792 | 0.19 |
| Compositor 2974558 | Compositor | 9014 | 13522 | 260 | 0.02 |

### File click (n=12): ms per input by self class, phase C = tInject .. firstFrame, and window W (400 ms)
| thread | role | phase | idle | program | wasm | js | gc | total ms |
|---|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 2975166 | UI worker | C | 0.3 | 10.2 | 37.0 | 0.1 | 0.0 | 47.6 |
| DedicatedWorker thread 2975166 | UI worker | W | 117.9 | 82.9 | 190.9 | 7.9 | 0.4 | 400.3 |
| CrRendererMain 2974534 | page main | C | 43.0 | 4.6 | 0.0 | 0.1 | 0.0 | 47.6 |
| CrRendererMain 2974534 | page main | W | 283.8 | 39.6 | 0.0 | 72.6 | 4.3 | 400.2 |
| Realtime AudioWorklet thread 2975165 | audio worklet | C | 47.8 | 0.0 | 0.0 | 0.0 | 0.0 | 47.9 |
| Realtime AudioWorklet thread 2975165 | audio worklet | W | 399.5 | 0.3 | 0.0 | 0.2 | 0.0 | 400.0 |
| DedicatedWorker thread 2975496 | Worker | C | 0.0 | 41.6 | 6.2 | 0.0 | 0.0 | 47.8 |
| DedicatedWorker thread 2975496 | Worker | W | 0.0 | 362.0 | 37.8 | 0.4 | 0.0 | 400.3 |
| DedicatedWorker thread 2975497 | Worker | C | 0.0 | 44.3 | 3.4 | 0.0 | 0.0 | 47.8 |
| DedicatedWorker thread 2975497 | Worker | W | 0.0 | 367.2 | 32.0 | 0.6 | 0.1 | 400.2 |
| DedicatedWorker thread 2975498 | Worker | C | 0.0 | 42.9 | 5.0 | 0.0 | 0.0 | 47.9 |
| DedicatedWorker thread 2975498 | Worker | W | 0.0 | 366.3 | 33.2 | 0.3 | 0.1 | 400.0 |
| DedicatedWorker thread 2975499 | Worker | C | 0.0 | 43.0 | 4.9 | 0.0 | 0.0 | 47.9 |
| DedicatedWorker thread 2975499 | Worker | W | 0.0 | 361.6 | 37.3 | 0.6 | 0.2 | 400.0 |
| DedicatedWorker thread 2975500 | Worker | C | 0.0 | 31.5 | 16.3 | 0.1 | 0.0 | 47.9 |
| DedicatedWorker thread 2975500 | Worker | W | 0.0 | 259.0 | 139.6 | 0.8 | 0.1 | 400.1 |
| DedicatedWorker thread 2975501 | Worker | C | 0.0 | 42.8 | 5.0 | -0.0 | 0.0 | 47.8 |
| DedicatedWorker thread 2975501 | Worker | W | 0.0 | 365.0 | 34.5 | 0.5 | 0.1 | 400.1 |
| DedicatedWorker thread 2975502 | Worker | C | 0.0 | 42.8 | 4.9 | 0.1 | 0.0 | 47.8 |
| DedicatedWorker thread 2975502 | Worker | W | 0.0 | 365.1 | 34.3 | 0.5 | 0.0 | 400.1 |

#### UI worker functions in phase C, File click: ms per input (total 47.6 ms)
| function | self ms | inclusive ms |
|---|---|---|
| g9_hook | 3.93 | 34.90 |
| f_86a660 | 0.00 | 34.85 |
| wasm-function[1390] | 0.00 | 34.85 |
| wasm-function[4254] | 0.00 | 34.85 |
| f_1375990 | 0.00 | 34.85 |
| f_1375950 | 0.00 | 34.85 |
| wasm-function[3044] | 0.00 | 34.80 |
| fle_call_guest | 0.14 | 34.80 |
| wasm-function[669] | 0.06 | 34.76 |
| wasm-function[721] | 0.08 | 34.76 |
| wasm-function[2518] | 0.12 | 34.76 |
| wasm-function[1389] | 0.01 | 34.61 |
| f_612940 | 0.00 | 34.57 |
| dispatch_message | 0.00 | 34.55 |
| wasm-function[3298] | 0.00 | 33.92 |
| fle_imp_DispatchMessageW | 0.00 | 33.90 |
| __wrap_fle_imp_DispatchMessageW | 0.00 | 33.90 |
| f_724fd0 | 0.00 | 33.88 |
| f_7250f0 | 0.00 | 33.88 |
| wasm-function[1362] | 0.00 | 30.59 |
| fle_dispatch | 0.10 | 28.50 |
| fle_hyb_dispatch | 0.14 | 27.57 |
| wasm-function[3754] | 0.00 | 25.56 |
| wasm-function[1064] | 0.00 | 25.56 |
| wasm-function[3037] | 0.02 | 25.52 |
| wasm-function[3154] | 0.00 | 25.48 |
| f_840960 | 0.05 | 25.39 |
| wasm-function[4930] | 0.00 | 25.01 |

(self-time top 12): wasm-function[1368] 12.41; (program) 10.19; wasm-function[3035] 5.59; g9_hook 3.93; fle_desk_compose 2.07; wasm-function[3145] 1.79; wasm-function[5212] 0.79; fle_gil_wake 0.57; wasm-function[3590] 0.48; wasm-function[1746] 0.44; fle_gil_acquire_slow 0.40; (idle) 0.30

### Escape (n=12): ms per input by self class, phase C = tInject .. firstFrame, and window W (400 ms)
| thread | role | phase | idle | program | wasm | js | gc | total ms |
|---|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 2975166 | UI worker | C | 0.0 | 1.5 | 7.2 | 0.2 | 0.0 | 8.9 |
| DedicatedWorker thread 2975166 | UI worker | W | 123.5 | 101.2 | 167.1 | 7.4 | 0.5 | 399.9 |
| CrRendererMain 2974534 | page main | C | 7.9 | 1.1 | 0.0 | -0.0 | 0.0 | 8.9 |
| CrRendererMain 2974534 | page main | W | 175.9 | 45.3 | 0.0 | 170.0 | 8.6 | 399.9 |
| Realtime AudioWorklet thread 2975165 | audio worklet | C | 9.0 | 0.0 | 0.0 | 0.0 | 0.0 | 9.0 |
| Realtime AudioWorklet thread 2975165 | audio worklet | W | 399.7 | 0.1 | 0.0 | 0.2 | 0.0 | 399.9 |
| DedicatedWorker thread 2975496 | Worker | C | 0.0 | 8.2 | 0.7 | 0.0 | 0.0 | 8.9 |
| DedicatedWorker thread 2975496 | Worker | W | 0.0 | 364.9 | 34.4 | 0.5 | 0.1 | 400.1 |
| DedicatedWorker thread 2975497 | Worker | C | 0.0 | 8.3 | 0.6 | -0.0 | 0.0 | 9.0 |
| DedicatedWorker thread 2975497 | Worker | W | 0.0 | 366.7 | 32.1 | 0.8 | -0.1 | 400.0 |
| DedicatedWorker thread 2975498 | Worker | C | 0.0 | 8.2 | 0.7 | 0.0 | 0.0 | 8.9 |
| DedicatedWorker thread 2975498 | Worker | W | 0.0 | 367.8 | 31.5 | 0.3 | 0.0 | 399.9 |
| DedicatedWorker thread 2975499 | Worker | C | 0.0 | 8.0 | 0.8 | 0.0 | 0.0 | 8.8 |
| DedicatedWorker thread 2975499 | Worker | W | 0.0 | 357.0 | 42.0 | 0.6 | 0.2 | 400.0 |
| DedicatedWorker thread 2975500 | Worker | C | 0.0 | 4.8 | 4.1 | 0.0 | 0.0 | 8.9 |
| DedicatedWorker thread 2975500 | Worker | W | 0.0 | 255.1 | 143.2 | 1.0 | 0.4 | 400.1 |
| DedicatedWorker thread 2975501 | Worker | C | 0.0 | 7.8 | 1.2 | 0.0 | 0.0 | 9.0 |
| DedicatedWorker thread 2975501 | Worker | W | 0.0 | 371.8 | 27.7 | 0.4 | 0.0 | 400.1 |
| DedicatedWorker thread 2975502 | Worker | C | 0.0 | 8.9 | 0.2 | -0.0 | 0.0 | 9.0 |
| DedicatedWorker thread 2975502 | Worker | W | 0.0 | 363.1 | 36.2 | 0.5 | 0.1 | 400.1 |

#### UI worker functions in phase C, Escape: ms per input (total 8.9 ms)
| function | self ms | inclusive ms |
|---|---|---|
| f_86a660 | 0.00 | 6.18 |
| g9_hook | 0.40 | 6.18 |
| wasm-function[1390] | 0.00 | 6.18 |
| wasm-function[4254] | 0.00 | 6.18 |
| f_1375990 | 0.00 | 6.18 |
| f_1375950 | 0.00 | 6.18 |
| fle_call_guest | 0.00 | 5.89 |
| wasm-function[3044] | 0.00 | 5.75 |
| wasm-function[669] | 0.03 | 5.63 |
| wasm-function[721] | 0.00 | 5.63 |
| wasm-function[2518] | 0.04 | 5.60 |
| wasm-function[1362] | 0.00 | 5.58 |
| fle_hyb_dispatch | 0.00 | 4.73 |
| f_86b440 | 0.00 | 4.19 |
| wasm-function[4255] | 0.00 | 4.10 |
| wasm-function[4225] | 0.00 | 4.10 |
| wasm-function[1366] | 0.04 | 4.06 |
| wasm-function[3254] | 0.06 | 4.05 |
| wasm-function[4930] | 0.00 | 4.02 |
| wasm-function[3037] | 0.00 | 3.99 |
| wasm-function[3154] | 0.00 | 3.97 |
| f_868c00 | 0.00 | 3.91 |
| wasm-function[4261] | 0.08 | 3.86 |
| wasm-function[4229] | 0.00 | 3.72 |
| wasm-function[1368] | 2.36 | 3.72 |
| dispatch_message | 0.00 | 1.98 |
| wasm-function[1389] | 0.00 | 1.98 |
| f_612940 | 0.00 | 1.84 |

(self-time top 12): wasm-function[1368] 2.36; (program) 1.50; fle_desk_compose 1.04; wasm-function[3035] 0.82; g9_hook 0.40; wasm-function[973] 0.18; wasm-function[3145] 0.16; putImageData 0.15; fle_gil_wake 0.14; wasm-function[5212] 0.13; wasm-function[529] 0.13; wasm-function[531] 0.12
