## E13: UI worker = tid 3042257 (most RunTask: 6670); Workers: [3042257]

### thread CPU fraction inside W (tPage-200 ms .. tPage+200 ms), all File clicks + Escapes: d(tts)/d(ts) from the events of each thread
| thread | role | events | wall ms | CPU ms | CPU fraction |
|---|---|---|---|---|---|
| DedicatedWorker thread 3042257 | UI worker | 92941 | 19445 | 18451 | 0.95 |
| CrRendererMain 3042107 | CrRendererMain | 41628 | 18347 | 6781 | 0.37 |
| Realtime AudioWorklet thread 3042256 | Realtime AudioWorklet thread | 17242 | 19166 | 1738 | 0.09 |
| Compositor 3042128 | Compositor | 12270 | 16655 | 389 | 0.02 |

### File click (n=12): ms per input by self class, phase C = tInject .. firstFrame, and window W (400 ms)
| thread | role | phase | idle | program | wasm | js | gc | total ms |
|---|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3042257 | UI worker | C | 0.0 | 0.2 | 19.1 | 0.4 | 0.0 | 19.8 |
| DedicatedWorker thread 3042257 | UI worker | W | 25.7 | 71.9 | 294.5 | 7.0 | 0.6 | 400.0 |
| CrRendererMain 3042107 | page main | C | 18.9 | 0.8 | 0.0 | 0.0 | 0.0 | 19.8 |
| CrRendererMain 3042107 | page main | W | 311.6 | 33.2 | 0.0 | 51.8 | 3.3 | 400.0 |
| Realtime AudioWorklet thread 3042256 | audio worklet | C | 19.8 | -0.0 | 0.0 | -0.0 | 0.0 | 19.8 |
| Realtime AudioWorklet thread 3042256 | audio worklet | W | 399.8 | -0.1 | 0.0 | 0.2 | 0.0 | 400.0 |

#### UI worker functions in phase C, File click: ms per input (total 19.8 ms)
| function | self ms | inclusive ms |
|---|---|---|
| wasm-function[99727] | 0.00 | 19.42 |
| wasm-function[99728] | 0.00 | 19.42 |
| wasm-function[4253] | 0.00 | 19.42 |
| wasm-function[60738] | 0.00 | 19.42 |
| wasm-function[1389] | 0.00 | 19.42 |
| wasm-function[99804] | 0.00 | 19.41 |
| wasm-function[1388] | 0.00 | 19.40 |
| wasm-function[3043] | 0.00 | 19.39 |
| wasm-function[52200] | 0.00 | 19.39 |
| wasm-function[8491] | 0.07 | 19.38 |
| wasm-function[100820] | 0.00 | 19.35 |
| wasm-function[720] | 0.00 | 19.35 |
| wasm-function[3297] | 0.00 | 19.35 |
| wasm-function[2517] | 0.08 | 19.34 |
| wasm-function[668] | 0.03 | 19.34 |
| wasm-function[56872] | 0.00 | 19.31 |
| wasm-function[56882] | 0.00 | 19.31 |
| wasm-function[8372] | 0.07 | 15.36 |
| wasm-function[1361] | 0.00 | 15.21 |
| wasm-function[3753] | 0.00 | 14.39 |
| wasm-function[1063] | 0.02 | 14.39 |
| wasm-function[4929] | 0.02 | 14.37 |
| wasm-function[60054] | 0.03 | 14.34 |
| wasm-function[3036] | 0.02 | 14.30 |
| wasm-function[3153] | 0.02 | 14.27 |
| wasm-function[60708] | 0.00 | 14.18 |
| wasm-function[4260] | 0.02 | 14.17 |
| wasm-function[4228] | 0.00 | 14.10 |

(self-time top 12): wasm-function[1367] 9.64; wasm-function[3034] 3.21; wasm-function[99865] 1.57; wasm-function[2129] 0.59; wasm-function[2137] 0.55; (program) 0.21; wasm-function[2040] 0.19; present 0.17; putImageData 0.12; wasm-function[2517] 0.08; wasm-function[100545] 0.07; wasm-function[8372] 0.07

### Escape (n=12): ms per input by self class, phase C = tInject .. firstFrame, and window W (400 ms)
| thread | role | phase | idle | program | wasm | js | gc | total ms |
|---|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3042257 | UI worker | C | 0.0 | 2.3 | 12.2 | 0.3 | 0.0 | 14.9 |
| DedicatedWorker thread 3042257 | UI worker | W | 33.7 | 77.4 | 282.1 | 6.3 | 0.2 | 400.1 |
| CrRendererMain 3042107 | page main | C | 13.6 | 1.2 | 0.0 | 0.0 | 0.0 | 14.8 |
| CrRendererMain 3042107 | page main | W | 213.9 | 39.2 | 0.0 | 138.5 | 8.5 | 400.1 |
| Realtime AudioWorklet thread 3042256 | audio worklet | C | 14.9 | 0.0 | 0.0 | -0.0 | 0.0 | 14.9 |
| Realtime AudioWorklet thread 3042256 | audio worklet | W | 399.9 | -0.1 | 0.0 | 0.2 | 0.0 | 400.0 |

#### UI worker functions in phase C, Escape: ms per input (total 14.9 ms)
| function | self ms | inclusive ms |
|---|---|---|
| wasm-function[99804] | 0.00 | 10.47 |
| wasm-function[8372] | 0.04 | 8.25 |
| wasm-function[8491] | 0.04 | 7.71 |
| wasm-function[99727] | 0.00 | 6.97 |
| wasm-function[99728] | 0.00 | 6.97 |
| wasm-function[4253] | 0.00 | 6.97 |
| wasm-function[60738] | 0.00 | 6.97 |
| wasm-function[1389] | 0.00 | 6.97 |
| wasm-function[44134] | 0.00 | 5.25 |
| wasm-function[47982] | 0.00 | 5.25 |
| wasm-function[100615] | 0.00 | 5.25 |
| wasm-function[1613] | 0.00 | 5.22 |
| wasm-function[4550] | 0.06 | 5.18 |
| wasm-function[80011] | 0.00 | 5.18 |
| wasm-function[3043] | 0.00 | 5.17 |
| wasm-function[2517] | 0.05 | 5.11 |
| wasm-function[720] | 0.00 | 5.11 |
| wasm-function[668] | 0.00 | 5.11 |
| wasm-function[1361] | 0.02 | 5.00 |
| wasm-function[23295] | 0.00 | 4.81 |
| wasm-function[95503] | 0.00 | 4.62 |
| wasm-function[97547] | 0.00 | 4.60 |
| wasm-function[1625] | 0.02 | 4.60 |
| wasm-function[60759] | 0.00 | 4.39 |
| wasm-function[4575] | 0.02 | 4.22 |
| wasm-function[54151] | 0.04 | 4.01 |
| wasm-function[4929] | 0.00 | 3.06 |
| wasm-function[103071] | 0.00 | 3.00 |

(self-time top 12): (program) 2.32; wasm-function[1367] 2.03; wasm-function[99865] 1.34; wasm-function[7350] 0.46; wasm-function[7351] 0.45; wasm-function[3034] 0.45; wasm-function[7158] 0.34; wasm-function[845] 0.26; wasm-function[100982] 0.19; wasm-function[5315] 0.19; wasm-function[8375] 0.18; wasm-function[1365] 0.17
