## RT-G: 43 answered records, profiles 10
- profile 0x1: thread 3103359 CrRendererMain samples 68993
- profile 0x2: thread 3103623 DedicatedWorker thread samples 67154
- profile 0x3: thread 3103831 DedicatedWorker thread samples 78751
- profile 0x5: thread 3103829 DedicatedWorker thread samples 83964
- profile 0x6: thread 3103830 DedicatedWorker thread samples 73507
- profile 0x9: thread 3103834 DedicatedWorker thread samples 75285
- profile 0x8: thread 3103833 DedicatedWorker thread samples 73700
- profile 0x7: thread 3103832 DedicatedWorker thread samples 79143
- profile 0x4: thread 3103828 DedicatedWorker thread samples 73764
- profile 0xa: thread 3103622 Realtime AudioWorklet thread samples 76500

### File click (button-down): n=12
| segment | median ms | p90 | max |
|---|---|---|---|
| A tEvent->tPage | 0.66 | 1.06 | 2.61 |
| B tPage->tInject | 4.92 | 7.90 | 8.20 |
| C tInject->firstFrame | 47.17 | 57.18 | 60.03 |
| D firstFrame->raf | 9.72 | 22.90 | 57.99 |
| T tPage->firstFrame | 51.12 | 62.14 | 62.30 |
| T2 tPage->raf | 63.41 | 72.45 | 80.52 |

#### samples by self class, W = tPage-200ms .. tPage+200ms (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3103829 | 15498 | 0.0% | 90.3% | 9.1% | 0.6% | 0.1% | 0.0% |
| DedicatedWorker thread 3103832 | 14838 | 0.0% | 65.8% | 33.6% | 0.6% | 0.0% | 0.0% |
| DedicatedWorker thread 3103831 | 14564 | 0.0% | 88.3% | 11.2% | 0.4% | 0.0% | 0.0% |
| DedicatedWorker thread 3103834 | 13936 | 0.0% | 89.6% | 10.1% | 0.3% | 0.0% | 0.0% |
| Realtime AudioWorklet thread 3103622 | 13762 | 99.8% | 0.0% | 0.0% | 0.1% | 0.0% | 0.0% |
| DedicatedWorker thread 3103828 | 13682 | 0.0% | 90.0% | 9.6% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3103830 | 13628 | 0.0% | 89.8% | 9.9% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3103833 | 13528 | 0.0% | 90.3% | 9.3% | 0.4% | 0.0% | 0.0% |
| CrRendererMain 3103359 | 12938 | 69.9% | 12.5% | 0.0% | 16.6% | 1.0% | 0.0% |
| DedicatedWorker thread 3103623 | 12471 | 29.6% | 20.7% | 47.3% | 2.3% | 0.1% | 0.0% |

#### top self functions (wasm/js), W = tPage-200ms .. tPage+200ms
- DedicatedWorker thread 3103829: wasm:fle_gil_wake 1.2%; wasm:poll_waiters 1.0%; wasm:wasm-function[7159] 0.7%; wasm:wasm-function[7351] 0.5%; wasm:wasm-function[7352] 0.4%; wasm:g9_hook 0.3%; js:wasm-to-js 0.2%; wasm:fle_gil_acquire_slow 0.2%
- DedicatedWorker thread 3103832: wasm:poll_waiters 2.6%; wasm:fle_gil_wake 2.1%; wasm:wasm-function[7159] 2.1%; wasm:wasm-function[7351] 1.4%; wasm:g9_hook 1.3%; wasm:round_pack 1.0%; wasm:binop 0.9%; wasm:wasm-function[7352] 0.9%
- DedicatedWorker thread 3103831: wasm:fle_gil_wake 1.2%; wasm:wasm-function[7159] 1.2%; wasm:poll_waiters 0.8%; wasm:wasm-function[7351] 0.7%; wasm:g9_hook 0.6%; wasm:wasm-function[7352] 0.5%; wasm:fle_hyb_dispatch 0.3%; wasm:fle_hyb_mx 0.3%
- DedicatedWorker thread 3103834: wasm:fle_gil_wake 1.0%; wasm:poll_waiters 0.9%; wasm:wasm-function[7159] 0.8%; wasm:wasm-function[7351] 0.6%; wasm:wasm-function[7352] 0.6%; wasm:g9_hook 0.5%; wasm:fle_hyb_dispatch 0.2%; wasm:fle_gil_acquire_slow 0.2%

#### samples by self class, B+C = tPage .. firstFrame (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3103829 | 1951 | 0.0% | 91.0% | 8.4% | 0.6% | 0.0% | 0.0% |
| DedicatedWorker thread 3103832 | 1836 | 0.0% | 66.0% | 33.3% | 0.7% | 0.0% | 0.0% |
| DedicatedWorker thread 3103831 | 1834 | 0.0% | 86.0% | 13.5% | 0.4% | 0.2% | 0.0% |
| Realtime AudioWorklet thread 3103622 | 1706 | 99.8% | 0.1% | 0.0% | 0.1% | 0.0% | 0.0% |
| DedicatedWorker thread 3103828 | 1703 | 0.0% | 88.3% | 11.2% | 0.5% | 0.1% | 0.0% |
| DedicatedWorker thread 3103830 | 1687 | 0.0% | 90.5% | 9.0% | 0.5% | 0.0% | 0.0% |
| DedicatedWorker thread 3103834 | 1676 | 0.0% | 87.1% | 12.8% | 0.2% | 0.0% | 0.0% |
| DedicatedWorker thread 3103833 | 1636 | 0.0% | 91.7% | 7.9% | 0.3% | 0.1% | 0.0% |
| CrRendererMain 3103359 | 1599 | 89.2% | 10.0% | 0.0% | 0.8% | 0.0% | 0.0% |
| DedicatedWorker thread 3103623 | 1462 | 0.9% | 23.7% | 74.9% | 0.5% | 0.0% | 0.0% |

#### top self functions (wasm/js), B+C = tPage .. firstFrame
- DedicatedWorker thread 3103829: wasm:fle_gil_wake 1.3%; wasm:poll_waiters 1.0%; wasm:wasm-function[7159] 0.7%; wasm:g9_hook 0.5%; wasm:wasm-function[7351] 0.4%; wasm:fle_gil_acquire_slow 0.2%; wasm:wasm-function[7136] 0.2%; wasm:wasm-function[7266] 0.2%
- DedicatedWorker thread 3103832: wasm:fle_gil_wake 2.8%; wasm:poll_waiters 2.6%; wasm:wasm-function[7159] 2.5%; wasm:wasm-function[7351] 1.1%; wasm:g9_hook 1.1%; wasm:binop 1.0%; wasm:round_pack 0.9%; wasm:fle_hyb_mx 0.9%
- DedicatedWorker thread 3103831: wasm:wasm-function[7159] 2.1%; wasm:fle_gil_wake 1.1%; wasm:poll_waiters 1.0%; wasm:wasm-function[7352] 0.8%; wasm:wasm-function[7351] 0.8%; wasm:g9_hook 0.6%; wasm:fle_hyb_mx 0.5%; wasm:fle_hyb_dispatch 0.5%
- Realtime AudioWorklet thread 3103622: js:postMessage 0.1%


### Escape (key-down): n=12
| segment | median ms | p90 | max |
|---|---|---|---|
| A tEvent->tPage | 0.47 | 0.73 | 0.82 |
| B tPage->tInject | 0.27 | 3.99 | 4.06 |
| C tInject->firstFrame | 7.29 | 15.29 | 18.96 |
| D firstFrame->raf | 2.84 | 3.06 | 13.19 |
| T tPage->firstFrame | 9.00 | 15.56 | 20.56 |
| T2 tPage->raf | 12.63 | 19.92 | 23.19 |

#### samples by self class, W = tPage-200ms .. tPage+200ms (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3103829 | 15519 | 0.0% | 90.0% | 9.5% | 0.4% | 0.0% | 0.0% |
| DedicatedWorker thread 3103832 | 14555 | 0.0% | 64.8% | 34.5% | 0.6% | 0.0% | 0.0% |
| DedicatedWorker thread 3103831 | 14415 | 0.0% | 88.2% | 11.3% | 0.5% | 0.0% | 0.0% |
| Realtime AudioWorklet thread 3103622 | 13868 | 99.7% | 0.1% | 0.0% | 0.2% | 0.0% | 0.0% |
| DedicatedWorker thread 3103834 | 13850 | 0.0% | 89.8% | 9.7% | 0.4% | 0.1% | 0.0% |
| DedicatedWorker thread 3103828 | 13605 | 0.0% | 90.4% | 9.3% | 0.3% | 0.1% | 0.0% |
| DedicatedWorker thread 3103833 | 13569 | 0.0% | 90.7% | 8.9% | 0.4% | 0.0% | 0.0% |
| DedicatedWorker thread 3103830 | 13469 | 0.0% | 89.8% | 9.8% | 0.3% | 0.0% | 0.0% |
| CrRendererMain 3103359 | 12759 | 43.8% | 15.7% | 0.0% | 38.3% | 2.3% | 0.0% |
| DedicatedWorker thread 3103623 | 12266 | 28.5% | 25.0% | 44.0% | 2.3% | 0.2% | 0.0% |

#### top self functions (wasm/js), W = tPage-200ms .. tPage+200ms
- DedicatedWorker thread 3103829: wasm:fle_gil_wake 1.2%; wasm:poll_waiters 1.1%; wasm:wasm-function[7159] 0.6%; wasm:wasm-function[7351] 0.6%; wasm:wasm-function[7352] 0.4%; wasm:g9_hook 0.4%; wasm:fle_thread_wasm_idle_wait 0.3%; wasm:fle_gil_acquire_slow 0.3%
- DedicatedWorker thread 3103832: wasm:poll_waiters 2.5%; wasm:fle_gil_wake 2.0%; wasm:wasm-function[7159] 1.6%; wasm:wasm-function[7351] 1.5%; wasm:g9_hook 1.3%; wasm:wasm-function[7352] 1.2%; wasm:round_pack 0.9%; wasm:binop 0.8%
- DedicatedWorker thread 3103831: wasm:fle_gil_wake 1.2%; wasm:wasm-function[7159] 1.0%; wasm:poll_waiters 0.9%; wasm:wasm-function[7351] 0.7%; wasm:wasm-function[7352] 0.6%; wasm:g9_hook 0.5%; wasm:fle_hyb_dispatch 0.3%; wasm:fle_gil_acquire_slow 0.3%
- Realtime AudioWorklet thread 3103622: js:process 0.1%; js:postMessage 0.1%

#### samples by self class, B+C = tPage .. firstFrame (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| Realtime AudioWorklet thread 3103622 | 384 | 99.5% | 0.0% | 0.0% | 0.5% | 0.0% | 0.0% |
| DedicatedWorker thread 3103829 | 370 | 0.0% | 87.6% | 12.2% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3103831 | 360 | 0.0% | 86.4% | 13.1% | 0.6% | 0.0% | 0.0% |
| CrRendererMain 3103359 | 358 | 78.5% | 20.1% | 0.0% | 1.4% | 0.0% | 0.0% |
| DedicatedWorker thread 3103832 | 353 | 0.0% | 65.2% | 33.4% | 1.4% | 0.0% | 0.0% |
| DedicatedWorker thread 3103833 | 349 | 0.0% | 87.4% | 12.6% | 0.0% | 0.0% | 0.0% |
| DedicatedWorker thread 3103830 | 348 | 0.0% | 91.7% | 8.0% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3103834 | 346 | 0.0% | 89.3% | 10.7% | 0.0% | 0.0% | 0.0% |
| DedicatedWorker thread 3103828 | 340 | 0.0% | 93.8% | 6.2% | 0.0% | 0.0% | 0.0% |
| DedicatedWorker thread 3103623 | 320 | 6.9% | 20.0% | 71.2% | 1.9% | 0.0% | 0.0% |

#### top self functions (wasm/js), B+C = tPage .. firstFrame
- Realtime AudioWorklet thread 3103622: js:process 0.3%; js:postMessage 0.3%
- DedicatedWorker thread 3103829: wasm:fle_gil_wake 1.1%; wasm:fle_gil_acquire_slow 0.8%; wasm:wasm-function[7352] 0.8%; wasm:wasm-function[7100] 0.5%; wasm:g9_hook 0.5%; wasm:fle_hyb_dispatch 0.5%; wasm:poll_waiters 0.5%; wasm:wasm-function[7351] 0.5%
- DedicatedWorker thread 3103831: wasm:wasm-function[7159] 2.2%; wasm:fle_gil_wake 1.7%; wasm:poll_waiters 1.1%; wasm:wasm-function[7352] 1.1%; wasm:fle_hyb_mx 0.8%; wasm:fle_thread_wasm_idle_wait 0.6%; wasm:__wrap_fle_imp_WaitForSingleObject 0.6%; wasm:wasm-function[7351] 0.6%
- CrRendererMain 3103359: js: 0.6%; js:l 0.3%; js:t.port.onmessage 0.3%; js:onMessage 0.3%


#### RunTask busy ms per input inside W (+-200 ms = 400 ms), File click (n=12)
| thread | busy ms per input | tasks per input | longest task ms |
|---|---|---|---|
| DedicatedWorker thread 3103623 | 298.2 | 231.7 | 61.76 |
| CrRendererMain 3103359 | 100.7 | 121.8 | 292.66 |
| Realtime AudioWorklet thread 3103622 | 6.1 | 37.2 | 0.47 |
| Compositor 3103379 | 4.0 | 77.5 | 0.53 |
| Chrome_ChildIOThread 3103369 | 0.4 | 10.2 | 0.12 |
| PerfettoTrace 3103364 | 0.3 | 4.1 | 0.23 |
| ThreadPoolServiceThread 3103365 | 0.0 | 0.2 | 0.09 |

#### RunTask busy ms per input inside W (+-200 ms = 400 ms), Escape (n=12)
| thread | busy ms per input | tasks per input | longest task ms |
|---|---|---|---|
| DedicatedWorker thread 3103623 | 301.4 | 242.4 | 44.39 |
| CrRendererMain 3103359 | 212.1 | 148.5 | 279.55 |
| Realtime AudioWorklet thread 3103622 | 6.2 | 37.4 | 0.48 |
| Compositor 3103379 | 4.5 | 74.7 | 1.23 |
| Chrome_ChildIOThread 3103369 | 0.8 | 22.9 | 0.18 |
| PerfettoTrace 3103364 | 0.3 | 5.4 | 0.25 |
| ThreadPoolServiceThread 3103365 | 0.1 | 0.8 | 0.13 |
