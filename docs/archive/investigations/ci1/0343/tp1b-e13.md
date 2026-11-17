## E13: 49 answered records, profiles 3
- profile 0x1: thread 3042257 DedicatedWorker thread samples 180595
- profile 0x2: thread 3042256 Realtime AudioWorklet thread samples 129400
- profile 0x3: thread 3042107 CrRendererMain samples 125373

### File click (button-down): n=12
| segment | median ms | p90 | max |
|---|---|---|---|
| A tEvent->tPage | 0.66 | 1.00 | 1.08 |
| B tPage->tInject | 1.14 | 4.39 | 5.31 |
| C tInject->firstFrame | 18.52 | 22.69 | 23.37 |
| D firstFrame->raf | 2.00 | 8.80 | 16.13 |
| T tPage->firstFrame | 21.77 | 23.85 | 24.95 |
| T2 tPage->raf | 24.79 | 27.27 | 38.44 |

#### samples by self class, W = tPage-200ms .. tPage+200ms (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3042257 | 31879 | 5.2% | 33.4% | 59.6% | 1.7% | 0.1% | 0.0% |
| Realtime AudioWorklet thread 3042256 | 23353 | 99.7% | 0.1% | 0.0% | 0.2% | 0.0% | 0.0% |
| CrRendererMain 3042107 | 23025 | 76.1% | 10.1% | 0.0% | 13.0% | 0.8% | 0.0% |

#### top self functions (wasm/js), W = tPage-200ms .. tPage+200ms
- DedicatedWorker thread 3042257: wasm:wasm-function[1367] 4.9%; wasm:wasm-function[7350] 3.8%; wasm:wasm-function[7351] 3.4%; wasm:wasm-function[7158] 3.2%; wasm:wasm-function[99865] 1.9%; wasm:wasm-function[3034] 1.5%; wasm:wasm-function[7105] 1.5%; wasm:wasm-function[7160] 1.1%
- Realtime AudioWorklet thread 3042256: js:process 0.1%; js:postMessage 0.1%
- CrRendererMain 3042107: js:deskDumpBase64 12.3%; js: 0.2%; js:mc 0.1%; js:t.port.onmessage 0.0%; js:pc 0.0%; js:now 0.0%; js:requestAnimationFrame 0.0%; js:Wt 0.0%

#### samples by self class, B+C = tPage .. firstFrame (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| Realtime AudioWorklet thread 3042256 | 1260 | 99.7% | 0.2% | 0.0% | 0.2% | 0.0% | 0.0% |
| CrRendererMain 3042107 | 1187 | 93.9% | 5.6% | 0.0% | 0.6% | 0.0% | 0.0% |
| DedicatedWorker thread 3042257 | 1167 | 0.3% | 1.8% | 95.6% | 2.2% | 0.1% | 0.0% |

#### top self functions (wasm/js), B+C = tPage .. firstFrame
- Realtime AudioWorklet thread 3042256: js:process 0.2%
- CrRendererMain 3042107: js:now 0.2%; js:getBoundingClientRect 0.2%; js:requestAnimationFrame 0.1%; js:pointer 0.1%; js:d 0.1%
- DedicatedWorker thread 3042257: wasm:wasm-function[1367] 47.3%; wasm:wasm-function[3034] 16.7%; wasm:wasm-function[99865] 7.1%; wasm:wasm-function[2137] 2.9%; wasm:wasm-function[2129] 1.3%; wasm:wasm-function[2040] 0.9%; js:present 0.8%; js:putImageData 0.6%


### Escape (key-down): n=12
| segment | median ms | p90 | max |
|---|---|---|---|
| A tEvent->tPage | 0.53 | 0.62 | 0.91 |
| B tPage->tInject | 0.23 | 1.64 | 1.77 |
| C tInject->firstFrame | 4.61 | 7.17 | 127.54 |
| D firstFrame->raf | 2.92 | 16.67 | 17.46 |
| T tPage->firstFrame | 5.03 | 7.30 | 127.74 |
| T2 tPage->raf | 9.40 | 23.15 | 130.74 |

#### samples by self class, W = tPage-200ms .. tPage+200ms (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3042257 | 30929 | 7.0% | 31.6% | 59.6% | 1.7% | 0.1% | 0.0% |
| Realtime AudioWorklet thread 3042256 | 23076 | 99.6% | 0.1% | 0.0% | 0.2% | 0.0% | 0.0% |
| CrRendererMain 3042107 | 22679 | 52.5% | 12.2% | 0.0% | 33.2% | 2.1% | 0.0% |

#### top self functions (wasm/js), W = tPage-200ms .. tPage+200ms
- DedicatedWorker thread 3042257: wasm:wasm-function[7350] 4.1%; wasm:wasm-function[7351] 3.6%; wasm:wasm-function[7158] 3.4%; wasm:wasm-function[1367] 3.1%; wasm:wasm-function[99865] 2.0%; wasm:wasm-function[7105] 1.6%; wasm:wasm-function[7223] 1.3%; wasm:wasm-function[7160] 1.2%
- Realtime AudioWorklet thread 3042256: js:process 0.2%; js:postMessage 0.1%
- CrRendererMain 3042107: js:deskDumpBase64 24.2%; js: 8.4%; js:Wt 0.1%; js:mc 0.1%; js:onMessage 0.0%; js:requestAnimationFrame 0.0%; js:t.port.onmessage 0.0%; js:X 0.0%

#### samples by self class, B+C = tPage .. firstFrame (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3042257 | 1027 | 1.0% | 21.4% | 75.9% | 1.8% | 0.0% | 0.0% |
| Realtime AudioWorklet thread 3042256 | 894 | 99.8% | 0.0% | 0.0% | 0.2% | 0.0% | 0.0% |
| CrRendererMain 3042107 | 880 | 89.3% | 9.7% | 0.0% | 1.0% | 0.0% | 0.0% |

#### top self functions (wasm/js), B+C = tPage .. firstFrame
- DedicatedWorker thread 3042257: wasm:wasm-function[1367] 10.5%; wasm:wasm-function[99865] 6.8%; wasm:wasm-function[7158] 3.4%; wasm:wasm-function[7351] 3.4%; wasm:wasm-function[7350] 3.0%; wasm:wasm-function[3034] 2.4%; wasm:wasm-function[8375] 1.6%; wasm:wasm-function[845] 1.5%
- Realtime AudioWorklet thread 3042256: js:postMessage 0.1%; js:process 0.1%
- CrRendererMain 3042107: js: 0.2%; js:keyDown 0.2%; js:onSample 0.1%; js:requestAnimationFrame 0.1%; js:sent 0.1%; js:dc 0.1%; js:push 0.1%


#### RunTask busy ms per input inside W (+-200 ms = 400 ms), File click (n=12)
| thread | busy ms per input | tasks per input | longest task ms |
|---|---|---|---|
| DedicatedWorker thread 3042257 | 381.2 | 98.1 | 126.32 |
| CrRendererMain 3042107 | 79.1 | 125.5 | 326.17 |
| Realtime AudioWorklet thread 3042256 | 6.8 | 37.6 | 0.73 |
| Compositor 3042128 | 4.9 | 91.2 | 2.56 |
| Chrome_ChildIOThread 3042118 | 0.2 | 8.2 | 0.08 |
| PerfettoTrace 3042113 | 0.2 | 2.9 | 0.28 |
| ThreadPoolServiceThread 3042114 | 0.0 | 0.4 | 0.14 |

#### RunTask busy ms per input inside W (+-200 ms = 400 ms), Escape (n=12)
| thread | busy ms per input | tasks per input | longest task ms |
|---|---|---|---|
| DedicatedWorker thread 3042257 | 373.5 | 95.8 | 148.20 |
| CrRendererMain 3042107 | 179.4 | 132.0 | 295.32 |
| Realtime AudioWorklet thread 3042256 | 7.2 | 37.6 | 0.80 |
| Compositor 3042128 | 4.6 | 76.2 | 0.76 |
| Chrome_ChildIOThread 3042118 | 0.4 | 10.4 | 0.09 |
| ThreadPoolServiceThread 3042114 | 0.1 | 0.8 | 0.53 |
| PerfettoTrace 3042113 | 0.1 | 1.5 | 0.14 |
