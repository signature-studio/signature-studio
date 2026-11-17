## RT-G: UI worker = tid 3098898 (most RunTask: 15316); Workers: [3098898]

### thread CPU fraction inside W (tPage-200 ms .. tPage+200 ms), all File clicks + Escapes: d(tts)/d(ts) from the events of each thread
| thread | role | events | wall ms | CPU ms | CPU fraction |
|---|---|---|---|---|---|
| DedicatedWorker thread 3098898 | UI worker | 36349 | 17598 | 12079 | 0.69 |
| CrRendererMain 3098699 | CrRendererMain | 39970 | 16612 | 6822 | 0.41 |
| DedicatedWorker thread 3099026 | Worker | 25405 | 17529 | 4447 | 0.25 |
| DedicatedWorker thread 3099029 | Worker | 20671 | 17489 | 9370 | 0.54 |
| DedicatedWorker thread 3099028 | Worker | 17443 | 17466 | 3998 | 0.23 |
| Realtime AudioWorklet thread 3098897 | Realtime AudioWorklet thread | 15690 | 17467 | 1284 | 0.07 |
| DedicatedWorker thread 3099031 | Worker | 12926 | 17595 | 3550 | 0.20 |
| DedicatedWorker thread 3099027 | Worker | 11022 | 17417 | 3432 | 0.20 |
| DedicatedWorker thread 3099025 | Worker | 10943 | 17457 | 3622 | 0.21 |
| DedicatedWorker thread 3099030 | Worker | 10814 | 17419 | 3413 | 0.20 |
| Compositor 3098720 | Compositor | 11051 | 15378 | 334 | 0.02 |

### File click (n=12): ms per input by self class, phase C = tInject .. firstFrame, and window W (400 ms)
| thread | role | phase | idle | program | wasm | js | gc | total ms |
|---|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3098898 | UI worker | C | 0.0 | 7.7 | 31.5 | 0.1 | 0.0 | 39.3 |
| DedicatedWorker thread 3098898 | UI worker | W | 116.6 | 84.7 | 190.1 | 7.6 | 0.6 | 400.1 |
| CrRendererMain 3098699 | page main | C | 35.7 | 3.6 | 0.0 | 0.1 | -0.0 | 39.4 |
| CrRendererMain 3098699 | page main | W | 279.6 | 42.5 | 0.0 | 72.7 | 5.3 | 400.0 |
| Realtime AudioWorklet thread 3098897 | audio worklet | C | 39.3 | 0.0 | 0.0 | 0.0 | 0.0 | 39.3 |
| Realtime AudioWorklet thread 3098897 | audio worklet | W | 399.6 | 0.0 | 0.0 | 0.4 | 0.0 | 400.0 |
| DedicatedWorker thread 3099025 | Worker | C | 0.0 | 34.4 | 4.8 | 0.0 | 0.0 | 39.3 |
| DedicatedWorker thread 3099025 | Worker | W | 0.0 | 363.0 | 36.5 | 0.3 | 0.0 | 400.0 |
| DedicatedWorker thread 3099026 | Worker | C | 0.0 | 9.7 | 29.3 | 0.1 | 0.1 | 39.3 |
| DedicatedWorker thread 3099026 | Worker | W | 0.0 | 110.0 | 287.2 | 2.1 | 0.2 | 400.1 |
| DedicatedWorker thread 3099027 | Worker | C | 0.0 | 36.3 | 3.0 | 0.0 | 0.0 | 39.3 |
| DedicatedWorker thread 3099027 | Worker | W | 0.0 | 363.7 | 35.6 | 0.5 | 0.0 | 400.0 |
| DedicatedWorker thread 3099028 | Worker | C | 0.0 | 36.3 | 3.0 | 0.0 | 0.0 | 39.4 |
| DedicatedWorker thread 3099028 | Worker | W | 0.0 | 362.7 | 36.1 | 0.8 | 0.2 | 400.0 |
| DedicatedWorker thread 3099029 | Worker | C | 0.0 | 27.2 | 12.3 | 0.0 | 0.0 | 39.6 |
| DedicatedWorker thread 3099029 | Worker | W | 0.0 | 264.2 | 133.7 | 1.2 | 0.3 | 400.0 |
| DedicatedWorker thread 3099030 | Worker | C | 0.0 | 36.5 | 2.8 | 0.0 | 0.0 | 39.4 |
| DedicatedWorker thread 3099030 | Worker | W | 0.0 | 366.3 | 32.6 | 0.5 | 0.1 | 399.8 |
| DedicatedWorker thread 3099031 | Worker | C | 0.0 | 35.8 | 3.6 | 0.1 | 0.0 | 39.5 |
| DedicatedWorker thread 3099031 | Worker | W | 0.0 | 367.9 | 31.3 | 0.5 | 0.2 | 400.2 |

#### UI worker functions in phase C, File click: ms per input (total 39.3 ms)
| function | self ms | inclusive ms |
|---|---|---|
| g9_hook | 3.75 | 30.02 |
| wasm-function[1390] | 0.00 | 30.02 |
| f_86a660 | 0.00 | 30.02 |
| wasm-function[4254] | 0.00 | 30.02 |
| f_1375950 | 0.00 | 30.02 |
| f_1375990 | 0.00 | 30.02 |
| wasm-function[1389] | 0.00 | 29.99 |
| fle_call_guest | 0.00 | 29.94 |
| wasm-function[669] | 0.04 | 29.85 |
| f_612940 | 0.00 | 29.85 |
| wasm-function[721] | 0.02 | 29.85 |
| wasm-function[3044] | 0.00 | 29.85 |
| wasm-function[2518] | 0.13 | 29.85 |
| dispatch_message | 0.00 | 29.76 |
| fle_imp_DispatchMessageW | 0.00 | 29.70 |
| __wrap_fle_imp_DispatchMessageW | 0.00 | 29.70 |
| wasm-function[3298] | 0.00 | 29.50 |
| f_7250f0 | 0.00 | 29.28 |
| f_724fd0 | 0.00 | 29.28 |
| wasm-function[1362] | 0.00 | 26.39 |
| fle_dispatch | 0.04 | 25.54 |
| fle_hyb_dispatch | 0.07 | 24.25 |
| wasm-function[1064] | 0.02 | 22.93 |
| wasm-function[3754] | 0.00 | 22.93 |
| f_840960 | 0.04 | 22.76 |
| wasm-function[3037] | 0.04 | 22.72 |
| wasm-function[3154] | 0.09 | 22.68 |
| wasm-function[4930] | 0.00 | 22.46 |

(self-time top 12): wasm-function[1368] 10.74; (program) 7.66; wasm-function[3035] 5.05; g9_hook 3.75; fle_desk_compose 1.43; wasm-function[3145] 1.40; wasm-function[5212] 1.25; fle_gil_wake 0.51; wasm-function[965] 0.29; wasm-function[3590] 0.21; wasm-function[1746] 0.21; fle_gil_acquire_slow 0.16

### Escape (n=12): ms per input by self class, phase C = tInject .. firstFrame, and window W (400 ms)
| thread | role | phase | idle | program | wasm | js | gc | total ms |
|---|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3098898 | UI worker | C | 0.1 | 3.1 | 8.6 | 0.1 | 0.0 | 11.9 |
| DedicatedWorker thread 3098898 | UI worker | W | 111.0 | 105.5 | 174.5 | 7.7 | 0.7 | 399.8 |
| CrRendererMain 3098699 | page main | C | 10.5 | 1.4 | 0.0 | 0.1 | 0.0 | 12.0 |
| CrRendererMain 3098699 | page main | W | 175.5 | 51.6 | 0.0 | 163.1 | 9.7 | 399.8 |
| Realtime AudioWorklet thread 3098897 | audio worklet | C | 11.8 | 0.0 | 0.0 | 0.0 | 0.0 | 11.8 |
| Realtime AudioWorklet thread 3098897 | audio worklet | W | 399.7 | 0.1 | 0.0 | 0.2 | 0.0 | 400.0 |
| DedicatedWorker thread 3099025 | Worker | C | 0.0 | 11.2 | 0.7 | 0.0 | 0.0 | 12.0 |
| DedicatedWorker thread 3099025 | Worker | W | 0.0 | 359.2 | 39.9 | 0.4 | 0.1 | 399.9 |
| DedicatedWorker thread 3099026 | Worker | C | 0.0 | 3.2 | 8.5 | 0.1 | 0.0 | 11.8 |
| DedicatedWorker thread 3099026 | Worker | W | 0.0 | 117.7 | 280.2 | 1.5 | 0.1 | 400.0 |
| DedicatedWorker thread 3099027 | Worker | C | 0.0 | 10.4 | 1.4 | 0.1 | 0.0 | 11.9 |
| DedicatedWorker thread 3099027 | Worker | W | 0.0 | 363.8 | 34.7 | 0.7 | 0.2 | 399.9 |
| DedicatedWorker thread 3099028 | Worker | C | 0.0 | 10.9 | 1.0 | 0.0 | 0.0 | 12.0 |
| DedicatedWorker thread 3099028 | Worker | W | 0.0 | 356.9 | 42.1 | 0.4 | 0.3 | 400.0 |
| DedicatedWorker thread 3099029 | Worker | C | 0.0 | 6.8 | 5.0 | 0.1 | 0.1 | 12.0 |
| DedicatedWorker thread 3099029 | Worker | W | 0.0 | 247.6 | 150.0 | 1.4 | 0.1 | 399.9 |
| DedicatedWorker thread 3099030 | Worker | C | 0.0 | 10.6 | 1.4 | 0.0 | 0.0 | 11.9 |
| DedicatedWorker thread 3099030 | Worker | W | 0.0 | 366.1 | 33.1 | 0.5 | 0.1 | 400.0 |
| DedicatedWorker thread 3099031 | Worker | C | 0.0 | 11.2 | 0.7 | 0.1 | 0.0 | 12.0 |
| DedicatedWorker thread 3099031 | Worker | W | 0.0 | 366.4 | 32.6 | 0.7 | 0.2 | 400.1 |

#### UI worker functions in phase C, Escape: ms per input (total 11.9 ms)
| function | self ms | inclusive ms |
|---|---|---|
| g9_hook | 0.48 | 7.49 |
| f_86a660 | 0.00 | 7.49 |
| wasm-function[4254] | 0.05 | 7.49 |
| f_1375950 | 0.00 | 7.49 |
| f_1375990 | 0.00 | 7.49 |
| wasm-function[1390] | 0.02 | 7.43 |
| fle_call_guest | 0.00 | 7.16 |
| wasm-function[3044] | 0.04 | 7.07 |
| wasm-function[721] | 0.04 | 6.95 |
| wasm-function[669] | 0.02 | 6.91 |
| wasm-function[2518] | 0.04 | 6.89 |
| wasm-function[1362] | 0.07 | 6.65 |
| fle_hyb_dispatch | 0.09 | 5.45 |
| f_86b440 | 0.00 | 4.54 |
| wasm-function[4930] | 0.00 | 4.54 |
| wasm-function[4255] | 0.02 | 4.51 |
| wasm-function[4225] | 0.00 | 4.49 |
| wasm-function[1366] | 0.07 | 4.49 |
| wasm-function[3254] | 0.00 | 4.42 |
| wasm-function[3037] | 0.02 | 4.42 |
| wasm-function[3154] | 0.00 | 4.41 |
| f_868c00 | 0.04 | 4.37 |
| wasm-function[4261] | 0.00 | 4.28 |
| wasm-function[4229] | 0.00 | 4.19 |
| wasm-function[1368] | 2.46 | 4.17 |
| (program) | 3.14 | 3.14 |
| wasm-function[1389] | 0.02 | 2.87 |
| dispatch_message | 0.00 | 2.77 |

(self-time top 12): (program) 3.14; wasm-function[1368] 2.46; fle_desk_compose 1.08; wasm-function[3035] 0.70; g9_hook 0.48; wasm-function[3145] 0.37; wasm-function[5212] 0.23; wasm-function[3590] 0.16; wasm-function[5247] 0.14; fle_gil_acquire_slow 0.12; wasm-function[1746] 0.11; wasm-function[965] 0.11
