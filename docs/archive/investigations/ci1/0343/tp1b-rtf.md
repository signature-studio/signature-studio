## RT-F: 45 answered records, profiles 10
- profile 0x1: thread 3045506 DedicatedWorker thread samples 64154
- profile 0x2: thread 3045711 DedicatedWorker thread samples 76772
- profile 0x4: thread 3045709 DedicatedWorker thread samples 82060
- profile 0x3: thread 3045708 DedicatedWorker thread samples 72559
- profile 0x6: thread 3045712 DedicatedWorker thread samples 76586
- profile 0x5: thread 3045710 DedicatedWorker thread samples 72287
- profile 0x8: thread 3045714 DedicatedWorker thread samples 73639
- profile 0x7: thread 3045713 DedicatedWorker thread samples 71904
- profile 0x9: thread 3045351 CrRendererMain samples 67638
- profile 0xa: thread 3045505 Realtime AudioWorklet thread samples 75500

### File click (button-down): n=12
| segment | median ms | p90 | max |
|---|---|---|---|
| A tEvent->tPage | 0.76 | 1.06 | 1.11 |
| B tPage->tInject | 2.85 | 7.74 | 8.34 |
| C tInject->firstFrame | 44.39 | 59.10 | 61.73 |
| D firstFrame->raf | 10.85 | 25.59 | 83.16 |
| T tPage->firstFrame | 45.38 | 59.49 | 69.47 |
| T2 tPage->raf | 61.52 | 78.30 | 88.75 |

#### samples by self class, W = tPage-200ms .. tPage+200ms (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3045709 | 14675 | 0.0% | 89.2% | 10.3% | 0.5% | 0.0% | 0.0% |
| DedicatedWorker thread 3045711 | 13746 | 0.0% | 88.5% | 11.1% | 0.4% | 0.1% | 0.0% |
| DedicatedWorker thread 3045712 | 13624 | 0.0% | 66.4% | 33.1% | 0.5% | 0.0% | 0.0% |
| Realtime AudioWorklet thread 3045505 | 13200 | 99.7% | 0.1% | 0.0% | 0.2% | 0.0% | 0.0% |
| DedicatedWorker thread 3045714 | 13133 | 0.0% | 89.9% | 9.6% | 0.4% | 0.0% | 0.0% |
| DedicatedWorker thread 3045710 | 13003 | 0.0% | 90.3% | 9.3% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3045708 | 12884 | 0.0% | 91.1% | 8.6% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3045713 | 12828 | 0.0% | 91.3% | 8.4% | 0.2% | 0.0% | 0.0% |
| CrRendererMain 3045351 | 12430 | 69.3% | 12.8% | 0.0% | 16.9% | 1.0% | 0.0% |
| DedicatedWorker thread 3045506 | 11538 | 26.9% | 20.8% | 50.2% | 2.0% | 0.1% | 0.0% |

#### top self functions (wasm/js), W = tPage-200ms .. tPage+200ms
- DedicatedWorker thread 3045709: wasm:fle_gil_wake 1.2%; wasm:poll_waiters 1.0%; wasm:wasm-function[7159] 0.9%; wasm:wasm-function[7351] 0.6%; wasm:wasm-function[7352] 0.6%; wasm:g9_hook 0.5%; wasm:fle_thread_wasm_idle_wait 0.4%; wasm:fle_gil_acquire_slow 0.2%
- DedicatedWorker thread 3045711: wasm:fle_gil_wake 1.2%; wasm:wasm-function[7159] 1.1%; wasm:wasm-function[7351] 0.7%; wasm:poll_waiters 0.7%; wasm:wasm-function[7352] 0.6%; wasm:g9_hook 0.5%; wasm:fle_hyb_dispatch 0.4%; wasm:wasm-function[7106] 0.3%
- DedicatedWorker thread 3045712: wasm:poll_waiters 2.6%; wasm:fle_gil_wake 2.1%; wasm:wasm-function[7159] 1.8%; wasm:wasm-function[7351] 1.5%; wasm:g9_hook 1.5%; wasm:wasm-function[7352] 1.1%; wasm:round_pack 0.9%; wasm:fle_x87_arith_st 0.8%
- Realtime AudioWorklet thread 3045505: js:process 0.1%; js:postMessage 0.1%

#### samples by self class, B+C = tPage .. firstFrame (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3045709 | 1690 | 0.0% | 88.0% | 11.4% | 0.5% | 0.1% | 0.0% |
| DedicatedWorker thread 3045711 | 1622 | 0.0% | 87.1% | 12.4% | 0.4% | 0.1% | 0.0% |
| DedicatedWorker thread 3045712 | 1585 | 0.0% | 70.9% | 28.6% | 0.4% | 0.0% | 0.0% |
| Realtime AudioWorklet thread 3045505 | 1517 | 99.7% | 0.0% | 0.0% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3045714 | 1505 | 0.0% | 89.4% | 10.0% | 0.6% | 0.0% | 0.0% |
| DedicatedWorker thread 3045708 | 1474 | 0.0% | 91.7% | 7.9% | 0.3% | 0.1% | 0.0% |
| DedicatedWorker thread 3045710 | 1454 | 0.0% | 87.6% | 12.1% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3045713 | 1444 | 0.0% | 90.9% | 9.0% | 0.1% | 0.0% | 0.0% |
| CrRendererMain 3045351 | 1435 | 88.0% | 11.1% | 0.0% | 0.9% | 0.0% | 0.0% |
| DedicatedWorker thread 3045506 | 1287 | 1.8% | 19.7% | 78.1% | 0.4% | 0.0% | 0.0% |

#### top self functions (wasm/js), B+C = tPage .. firstFrame
- DedicatedWorker thread 3045709: wasm:wasm-function[7159] 1.6%; wasm:fle_gil_wake 1.2%; wasm:wasm-function[7352] 1.0%; wasm:poll_waiters 0.9%; wasm:g9_hook 0.6%; wasm:wasm-function[7351] 0.5%; wasm:wasm-function[7267] 0.5%; wasm:wasm-function[846] 0.3%
- DedicatedWorker thread 3045711: wasm:wasm-function[7159] 1.8%; wasm:fle_gil_wake 1.1%; wasm:fle_hyb_dispatch 0.9%; wasm:poll_waiters 0.7%; wasm:wasm-function[7351] 0.7%; wasm:wasm-function[7106] 0.4%; wasm:g9_hook 0.4%; wasm:wasm-function[7157] 0.4%
- DedicatedWorker thread 3045712: wasm:poll_waiters 2.4%; wasm:fle_gil_wake 2.1%; wasm:wasm-function[7159] 1.5%; wasm:g9_hook 1.4%; wasm:round_pack 1.1%; wasm:f_72cc3b0 0.9%; wasm:wasm-function[7352] 0.8%; wasm:wasm-function[7351] 0.8%
- Realtime AudioWorklet thread 3045505: js:postMessage 0.2%; js:process 0.1%


### Escape (key-down): n=11
| segment | median ms | p90 | max |
|---|---|---|---|
| A tEvent->tPage | 0.52 | 0.60 | 0.82 |
| B tPage->tInject | 0.46 | 3.34 | 3.61 |
| C tInject->firstFrame | 6.62 | 20.47 | 22.86 |
| D firstFrame->raf | 3.42 | 20.17 | 20.39 |
| T tPage->firstFrame | 9.06 | 20.93 | 23.14 |
| T2 tPage->raf | 15.84 | 39.95 | 40.14 |

#### samples by self class, W = tPage-200ms .. tPage+200ms (sum over 11 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3045709 | 13563 | 0.0% | 90.0% | 9.4% | 0.4% | 0.1% | 0.0% |
| DedicatedWorker thread 3045712 | 12760 | 0.0% | 64.1% | 35.4% | 0.5% | 0.0% | 0.0% |
| DedicatedWorker thread 3045711 | 12746 | 0.0% | 86.7% | 12.9% | 0.3% | 0.1% | 0.0% |
| Realtime AudioWorklet thread 3045505 | 12328 | 99.8% | 0.1% | 0.0% | 0.1% | 0.0% | 0.0% |
| DedicatedWorker thread 3045714 | 12148 | 0.0% | 89.1% | 10.5% | 0.4% | 0.0% | 0.0% |
| DedicatedWorker thread 3045708 | 12081 | 0.0% | 91.2% | 8.6% | 0.2% | 0.0% | 0.0% |
| DedicatedWorker thread 3045710 | 11949 | 0.0% | 89.7% | 10.0% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3045713 | 11838 | 0.0% | 90.6% | 9.1% | 0.3% | 0.0% | 0.0% |
| CrRendererMain 3045351 | 11327 | 46.3% | 16.4% | 0.0% | 35.1% | 2.1% | 0.0% |
| DedicatedWorker thread 3045506 | 10544 | 28.4% | 23.7% | 45.6% | 2.2% | 0.1% | 0.0% |

#### top self functions (wasm/js), W = tPage-200ms .. tPage+200ms
- DedicatedWorker thread 3045709: wasm:fle_gil_wake 1.3%; wasm:poll_waiters 1.1%; wasm:wasm-function[7159] 0.6%; wasm:wasm-function[7351] 0.5%; wasm:wasm-function[7352] 0.4%; wasm:g9_hook 0.3%; wasm:fle_thread_wasm_idle_wait 0.3%; wasm:fle_hyb_dispatch 0.2%
- DedicatedWorker thread 3045712: wasm:poll_waiters 2.7%; wasm:wasm-function[7159] 2.2%; wasm:fle_gil_wake 2.1%; wasm:wasm-function[7351] 1.5%; wasm:g9_hook 1.2%; wasm:wasm-function[7352] 1.1%; wasm:round_pack 1.1%; wasm:binop 0.8%
- DedicatedWorker thread 3045711: wasm:wasm-function[7159] 1.2%; wasm:fle_gil_wake 1.2%; wasm:wasm-function[7351] 0.9%; wasm:poll_waiters 0.9%; wasm:wasm-function[7352] 0.8%; wasm:g9_hook 0.8%; wasm:fle_hyb_dispatch 0.4%; wasm:wasm-function[7106] 0.3%
- Realtime AudioWorklet thread 3045505: js:process 0.1%; js:postMessage 0.0%

#### samples by self class, B+C = tPage .. firstFrame (sum over 11 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3045709 | 391 | 0.0% | 92.1% | 6.9% | 1.0% | 0.0% | 0.0% |
| CrRendererMain 3045351 | 372 | 75.3% | 21.5% | 0.0% | 3.2% | 0.0% | 0.0% |
| Realtime AudioWorklet thread 3045505 | 366 | 100.0% | 0.0% | 0.0% | 0.0% | 0.0% | 0.0% |
| DedicatedWorker thread 3045714 | 364 | 0.0% | 89.6% | 9.9% | 0.5% | 0.0% | 0.0% |
| DedicatedWorker thread 3045711 | 362 | 0.0% | 85.9% | 13.3% | 0.6% | 0.3% | 0.0% |
| DedicatedWorker thread 3045712 | 357 | 0.0% | 56.6% | 42.6% | 0.8% | 0.0% | 0.0% |
| DedicatedWorker thread 3045708 | 345 | 0.0% | 93.0% | 6.7% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3045713 | 345 | 0.0% | 90.4% | 9.3% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3045710 | 339 | 0.0% | 91.4% | 8.3% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3045506 | 327 | 7.3% | 19.0% | 70.0% | 3.7% | 0.0% | 0.0% |

#### top self functions (wasm/js), B+C = tPage .. firstFrame
- DedicatedWorker thread 3045709: wasm:fle_gil_wake 1.0%; wasm:poll_waiters 0.8%; js:handover 0.5%; wasm:wasm-function[7159] 0.5%; wasm:wasm-function[7352] 0.5%; wasm:audio_sync 0.5%; wasm:f_bb0920 0.5%; js:now 0.5%
- CrRendererMain 3045351: js: 1.3%; js:setCursor 0.3%; js:postMessage 0.3%; js:desk 0.3%; js:inputDone 0.3%; js:_c 0.3%; js:r 0.3%; js:now 0.3%
- DedicatedWorker thread 3045714: wasm:poll_waiters 1.1%; wasm:wasm-function[7159] 1.1%; wasm:wasm-function[7351] 0.5%; wasm:wasm-function[7352] 0.5%; wasm:audio_sync 0.5%; wasm:g9_hook 0.5%; wasm:wasm-function[7098] 0.5%; wasm:fle_gil_wake 0.5%


#### RunTask busy ms per input inside W (+-200 ms = 400 ms), File click (n=12)
| thread | busy ms per input | tasks per input | longest task ms |
|---|---|---|---|
| DedicatedWorker thread 3045506 | 300.5 | 206.0 | 75.00 |
| CrRendererMain 3045351 | 101.1 | 123.8 | 305.41 |
| Realtime AudioWorklet thread 3045505 | 6.4 | 37.3 | 0.42 |
| Compositor 3045369 | 4.1 | 76.2 | 0.56 |
| PerfettoTrace 3045355 | 0.4 | 6.2 | 0.56 |
| Chrome_ChildIOThread 3045360 | 0.3 | 9.9 | 0.11 |

#### RunTask busy ms per input inside W (+-200 ms = 400 ms), Escape (n=11)
| thread | busy ms per input | tasks per input | longest task ms |
|---|---|---|---|
| DedicatedWorker thread 3045506 | 302.0 | 218.1 | 41.97 |
| CrRendererMain 3045351 | 197.5 | 143.5 | 288.37 |
| Realtime AudioWorklet thread 3045505 | 6.6 | 37.4 | 0.87 |
| Compositor 3045369 | 4.9 | 71.8 | 1.38 |
| Chrome_ChildIOThread 3045360 | 0.7 | 19.4 | 0.10 |
| PerfettoTrace 3045355 | 0.3 | 6.0 | 0.30 |
| ThreadPoolServiceThread 3045356 | 0.0 | 0.4 | 0.11 |
