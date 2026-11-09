## RT-G: 45 answered records, profiles 10
- profile 0x1: thread 3099025 DedicatedWorker thread samples 72758
- profile 0x3: thread 3099026 DedicatedWorker thread samples 83111
- profile 0x4: thread 3099030 DedicatedWorker thread samples 72817
- profile 0x2: thread 3098898 DedicatedWorker thread samples 66467
- profile 0x5: thread 3099031 DedicatedWorker thread samples 74590
- profile 0x8: thread 3099028 DedicatedWorker thread samples 77388
- profile 0x6: thread 3099027 DedicatedWorker thread samples 72655
- profile 0x7: thread 3099029 DedicatedWorker thread samples 78188
- profile 0x9: thread 3098699 CrRendererMain samples 68252
- profile 0xa: thread 3098897 Realtime AudioWorklet thread samples 75700

### File click (button-down): n=12
| segment | median ms | p90 | max |
|---|---|---|---|
| A tEvent->tPage | 0.67 | 1.02 | 2.30 |
| B tPage->tInject | 0.24 | 2.78 | 8.32 |
| C tInject->firstFrame | 44.81 | 49.05 | 56.79 |
| D firstFrame->raf | 11.54 | 21.11 | 87.18 |
| T tPage->firstFrame | 45.04 | 57.38 | 57.40 |
| T2 tPage->raf | 52.16 | 72.48 | 94.09 |

#### samples by self class, W = tPage-200ms .. tPage+200ms (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3099026 | 15407 | 0.0% | 33.5% | 65.5% | 0.9% | 0.1% | 0.0% |
| DedicatedWorker thread 3099029 | 14375 | 0.0% | 66.9% | 32.4% | 0.6% | 0.1% | 0.0% |
| DedicatedWorker thread 3099028 | 14262 | 0.0% | 89.8% | 9.8% | 0.4% | 0.0% | 0.0% |
| DedicatedWorker thread 3099031 | 13734 | 0.0% | 90.7% | 8.9% | 0.3% | 0.1% | 0.0% |
| Realtime AudioWorklet thread 3098897 | 13731 | 99.8% | 0.1% | 0.0% | 0.2% | 0.0% | 0.0% |
| DedicatedWorker thread 3099025 | 13496 | 0.0% | 89.7% | 10.0% | 0.2% | 0.0% | 0.0% |
| DedicatedWorker thread 3099027 | 13360 | 0.0% | 89.9% | 9.7% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3099030 | 13358 | 0.0% | 90.7% | 8.9% | 0.4% | 0.0% | 0.0% |
| CrRendererMain 3098699 | 13047 | 68.8% | 12.7% | 0.0% | 17.2% | 1.2% | 0.0% |
| DedicatedWorker thread 3098898 | 12374 | 30.2% | 20.6% | 46.9% | 2.2% | 0.1% | 0.0% |

#### top self functions (wasm/js), W = tPage-200ms .. tPage+200ms
- DedicatedWorker thread 3099026: wasm:fle_thread_wasm_idle_wait 54.0%; wasm:fle_gil_acquire_slow 2.4%; wasm:fle_gil_wake 1.2%; wasm:poll_waiters 1.0%; wasm:wasm-function[7159] 0.8%; wasm:wasm-function[7351] 0.6%; wasm:wasm-function[7352] 0.5%; wasm:g9_hook 0.4%
- DedicatedWorker thread 3099029: wasm:poll_waiters 2.8%; wasm:fle_gil_wake 2.0%; wasm:wasm-function[7159] 2.0%; wasm:g9_hook 1.1%; wasm:wasm-function[7352] 1.1%; wasm:wasm-function[7351] 1.1%; wasm:binop 0.9%; wasm:round_pack 0.9%
- DedicatedWorker thread 3099028: wasm:fle_gil_wake 1.0%; wasm:wasm-function[7159] 0.9%; wasm:poll_waiters 0.7%; wasm:wasm-function[7351] 0.6%; wasm:wasm-function[7352] 0.6%; wasm:g9_hook 0.5%; wasm:fle_hyb_dispatch 0.3%; wasm:wasm-function[7157] 0.2%
- DedicatedWorker thread 3099031: wasm:fle_gil_wake 1.0%; wasm:poll_waiters 0.8%; wasm:wasm-function[7159] 0.8%; wasm:wasm-function[7351] 0.5%; wasm:wasm-function[7352] 0.4%; wasm:g9_hook 0.4%; wasm:fle_hyb_dispatch 0.2%; wasm:fle_hyb_mx 0.2%

#### samples by self class, B+C = tPage .. firstFrame (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3099026 | 1536 | 0.0% | 31.2% | 67.6% | 1.1% | 0.1% | 0.0% |
| DedicatedWorker thread 3099029 | 1488 | 0.0% | 69.7% | 29.5% | 0.8% | 0.0% | 0.0% |
| DedicatedWorker thread 3099031 | 1430 | 0.0% | 88.7% | 10.8% | 0.4% | 0.0% | 0.0% |
| DedicatedWorker thread 3099028 | 1424 | 0.0% | 90.9% | 8.8% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3099025 | 1390 | 0.0% | 87.3% | 12.4% | 0.2% | 0.0% | 0.0% |
| Realtime AudioWorklet thread 3098897 | 1387 | 99.7% | 0.0% | 0.0% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3099030 | 1384 | 0.0% | 91.0% | 8.7% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3099027 | 1374 | 0.0% | 91.3% | 8.4% | 0.3% | 0.0% | 0.0% |
| CrRendererMain 3098699 | 1306 | 88.7% | 10.5% | 0.0% | 0.7% | 0.1% | 0.0% |
| DedicatedWorker thread 3098898 | 1205 | 2.4% | 18.8% | 77.9% | 0.8% | 0.1% | 0.0% |

#### top self functions (wasm/js), B+C = tPage .. firstFrame
- DedicatedWorker thread 3099026: wasm:fle_thread_wasm_idle_wait 59.6%; wasm:fle_gil_acquire_slow 2.5%; wasm:poll_waiters 1.0%; wasm:fle_gil_wake 0.7%; wasm:wasm-function[7351] 0.4%; js:wasm-to-js 0.3%; js:fle_real_epoch_ms 0.3%; wasm:wasm-function[7352] 0.3%
- DedicatedWorker thread 3099029: wasm:poll_waiters 2.7%; wasm:wasm-function[7159] 2.5%; wasm:fle_gil_wake 2.1%; wasm:g9_hook 1.2%; wasm:wasm-function[7351] 1.1%; wasm:fle_x87_arith_st 0.9%; wasm:wasm-function[7352] 0.8%; wasm:fle_hyb_mx 0.7%
- DedicatedWorker thread 3099031: wasm:fle_gil_wake 1.2%; wasm:wasm-function[7351] 1.1%; wasm:g9_hook 0.8%; wasm:wasm-function[7159] 0.8%; wasm:poll_waiters 0.6%; wasm:wasm-function[7352] 0.3%; wasm:wasm-function[7161] 0.3%; wasm:fle_thread_wasm_idle_wait 0.3%
- DedicatedWorker thread 3099028: wasm:wasm-function[7159] 0.9%; wasm:poll_waiters 0.8%; wasm:fle_gil_wake 0.8%; wasm:wasm-function[7161] 0.4%; wasm:g9_hook 0.4%; wasm:wasm-function[7351] 0.4%; wasm:round_pack 0.3%; wasm:fle_gil_acquire_slow 0.3%


### Escape (key-down): n=12
| segment | median ms | p90 | max |
|---|---|---|---|
| A tEvent->tPage | 0.51 | 0.65 | 0.83 |
| B tPage->tInject | 0.38 | 4.44 | 7.30 |
| C tInject->firstFrame | 10.03 | 19.66 | 22.30 |
| D firstFrame->raf | 4.81 | 7.73 | 23.32 |
| T tPage->firstFrame | 13.31 | 19.91 | 26.74 |
| T2 tPage->raf | 18.44 | 30.62 | 32.36 |

#### samples by self class, W = tPage-200ms .. tPage+200ms (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3099026 | 15448 | 0.0% | 34.6% | 64.6% | 0.8% | 0.0% | 0.0% |
| DedicatedWorker thread 3099029 | 14664 | 0.0% | 63.7% | 35.6% | 0.6% | 0.0% | 0.0% |
| DedicatedWorker thread 3099028 | 14402 | 0.0% | 88.3% | 11.3% | 0.4% | 0.0% | 0.0% |
| DedicatedWorker thread 3099031 | 13910 | 0.0% | 90.6% | 9.1% | 0.3% | 0.0% | 0.0% |
| Realtime AudioWorklet thread 3098897 | 13831 | 99.8% | 0.1% | 0.0% | 0.1% | 0.0% | 0.0% |
| DedicatedWorker thread 3099025 | 13609 | 0.0% | 89.1% | 10.6% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3099030 | 13597 | 0.0% | 90.4% | 9.3% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3099027 | 13567 | 0.0% | 90.0% | 9.5% | 0.4% | 0.1% | 0.0% |
| CrRendererMain 3098699 | 12695 | 43.8% | 16.1% | 0.0% | 38.0% | 2.1% | 0.0% |
| DedicatedWorker thread 3098898 | 12291 | 28.2% | 25.9% | 43.7% | 2.0% | 0.1% | 0.0% |

#### top self functions (wasm/js), W = tPage-200ms .. tPage+200ms
- DedicatedWorker thread 3099026: wasm:fle_thread_wasm_idle_wait 53.1%; wasm:fle_gil_acquire_slow 2.5%; wasm:fle_gil_wake 1.2%; wasm:poll_waiters 1.0%; wasm:wasm-function[7159] 0.6%; wasm:wasm-function[7351] 0.6%; wasm:g9_hook 0.4%; wasm:wasm-function[7352] 0.4%
- DedicatedWorker thread 3099029: wasm:poll_waiters 2.7%; wasm:wasm-function[7159] 2.1%; wasm:fle_gil_wake 1.9%; wasm:wasm-function[7351] 1.7%; wasm:g9_hook 1.3%; wasm:wasm-function[7352] 1.3%; wasm:round_pack 1.0%; wasm:fle_x87_arith_st 0.9%
- DedicatedWorker thread 3099028: wasm:wasm-function[7159] 1.2%; wasm:fle_gil_wake 1.0%; wasm:poll_waiters 0.8%; wasm:wasm-function[7351] 0.8%; wasm:wasm-function[7352] 0.7%; wasm:g9_hook 0.5%; wasm:fle_hyb_dispatch 0.3%; wasm:wasm-function[7106] 0.3%
- DedicatedWorker thread 3099031: wasm:fle_gil_wake 1.0%; wasm:poll_waiters 0.8%; wasm:wasm-function[7159] 0.8%; wasm:wasm-function[7351] 0.7%; wasm:wasm-function[7352] 0.5%; wasm:g9_hook 0.4%; wasm:fle_hyb_mx 0.2%; wasm:fle_hyb_dispatch 0.2%

#### samples by self class, B+C = tPage .. firstFrame (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3099026 | 524 | 0.0% | 34.7% | 64.9% | 0.4% | 0.0% | 0.0% |
| DedicatedWorker thread 3099029 | 498 | 0.0% | 59.8% | 39.4% | 0.2% | 0.6% | 0.0% |
| CrRendererMain 3098699 | 473 | 81.6% | 16.5% | 0.0% | 1.9% | 0.0% | 0.0% |
| DedicatedWorker thread 3099028 | 467 | 0.0% | 91.4% | 7.7% | 0.9% | 0.0% | 0.0% |
| Realtime AudioWorklet thread 3098897 | 461 | 100.0% | 0.0% | 0.0% | 0.0% | 0.0% | 0.0% |
| DedicatedWorker thread 3099027 | 458 | 0.0% | 88.9% | 10.7% | 0.4% | 0.0% | 0.0% |
| DedicatedWorker thread 3099025 | 457 | 0.0% | 91.7% | 8.3% | 0.0% | 0.0% | 0.0% |
| DedicatedWorker thread 3099030 | 451 | 0.0% | 88.9% | 11.1% | 0.0% | 0.0% | 0.0% |
| DedicatedWorker thread 3099031 | 451 | 0.0% | 93.3% | 6.2% | 0.4% | 0.0% | 0.0% |
| DedicatedWorker thread 3098898 | 422 | 6.4% | 23.5% | 68.7% | 1.4% | 0.0% | 0.0% |

#### top self functions (wasm/js), B+C = tPage .. firstFrame
- DedicatedWorker thread 3099026: wasm:fle_thread_wasm_idle_wait 50.8%; wasm:fle_gil_acquire_slow 4.8%; wasm:fle_gil_wake 1.9%; wasm:wasm-function[7351] 1.0%; wasm:wasm-function[7159] 1.0%; wasm:g9_hook 0.8%; wasm:poll_waiters 0.8%; wasm:wasm-function[7157] 0.4%
- DedicatedWorker thread 3099029: wasm:wasm-function[7159] 3.6%; wasm:fle_gil_wake 3.4%; wasm:wasm-function[7351] 2.4%; wasm:wasm-function[7352] 2.0%; wasm:poll_waiters 2.0%; wasm:wasm-function[7224] 1.4%; wasm:g9_hook 1.2%; wasm:round_pack 1.0%
- CrRendererMain 3098699: js: 0.6%; js:postMessage 0.4%; js:sent 0.2%; js:sendInput 0.2%; js:_c 0.2%; js:mc 0.2%
- DedicatedWorker thread 3099028: wasm:wasm-function[7159] 1.1%; wasm:fle_thread_wasm_idle_wait 0.9%; wasm:fle_gil_wake 0.6%; wasm:g9_hook 0.4%; wasm:wasm-function[1803] 0.4%; wasm:binop 0.4%; wasm:wasm-function[7266] 0.4%; wasm:poll_waiters 0.4%


#### RunTask busy ms per input inside W (+-200 ms = 400 ms), File click (n=12)
| thread | busy ms per input | tasks per input | longest task ms |
|---|---|---|---|
| DedicatedWorker thread 3098898 | 290.3 | 222.8 | 60.50 |
| CrRendererMain 3098699 | 102.6 | 126.1 | 312.64 |
| Realtime AudioWorklet thread 3098897 | 6.0 | 37.1 | 1.09 |
| Compositor 3098720 | 4.4 | 82.2 | 0.64 |
| Chrome_ChildIOThread 3098708 | 0.4 | 11.9 | 0.12 |
| PerfettoTrace 3098703 | 0.2 | 3.8 | 0.26 |
| ThreadPoolServiceThread 3098704 | 0.0 | 0.5 | 0.14 |

#### RunTask busy ms per input inside W (+-200 ms = 400 ms), Escape (n=12)
| thread | busy ms per input | tasks per input | longest task ms |
|---|---|---|---|
| DedicatedWorker thread 3098898 | 303.9 | 241.8 | 34.28 |
| CrRendererMain 3098699 | 212.4 | 149.6 | 266.48 |
| Realtime AudioWorklet thread 3098897 | 6.3 | 37.5 | 0.80 |
| Compositor 3098720 | 4.9 | 79.1 | 1.50 |
| Chrome_ChildIOThread 3098708 | 0.8 | 22.2 | 0.32 |
| PerfettoTrace 3098703 | 0.4 | 8.1 | 0.22 |
| ThreadPoolServiceThread 3098704 | 0.1 | 0.8 | 0.12 |
