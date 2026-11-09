## RT-F: 38 answered records, profiles 10
- profile 0x2: thread 2975500 DedicatedWorker thread samples 76766
- profile 0x1: thread 2975166 DedicatedWorker thread samples 63650
- profile 0x3: thread 2975496 DedicatedWorker thread samples 71752
- profile 0x4: thread 2975497 DedicatedWorker thread samples 81971
- profile 0x5: thread 2975498 DedicatedWorker thread samples 71471
- profile 0x6: thread 2975499 DedicatedWorker thread samples 76138
- profile 0x8: thread 2975502 DedicatedWorker thread samples 73276
- profile 0x7: thread 2975501 DedicatedWorker thread samples 71567
- profile 0x9: thread 2974534 CrRendererMain samples 67097
- profile 0xa: thread 2975165 Realtime AudioWorklet thread samples 74400

### File click (button-down): n=12
| segment | median ms | p90 | max |
|---|---|---|---|
| A tEvent->tPage | 0.67 | 1.01 | 1.31 |
| B tPage->tInject | 0.90 | 7.11 | 9.30 |
| C tInject->firstFrame | 46.87 | 61.41 | 61.66 |
| D firstFrame->raf | 7.05 | 16.24 | 33.12 |
| T tPage->firstFrame | 49.33 | 62.13 | 66.10 |
| T2 tPage->raf | 58.15 | 76.51 | 87.28 |

#### samples by self class, W = tPage-200ms .. tPage+200ms (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 2975497 | 15566 | 0.0% | 89.2% | 10.4% | 0.4% | 0.0% | 0.0% |
| DedicatedWorker thread 2975500 | 14683 | 0.0% | 65.9% | 33.6% | 0.5% | 0.0% | 0.0% |
| DedicatedWorker thread 2975499 | 14606 | 0.0% | 88.8% | 10.8% | 0.3% | 0.1% | 0.0% |
| Realtime AudioWorklet thread 2975165 | 13881 | 99.8% | 0.1% | 0.0% | 0.1% | 0.0% | 0.0% |
| DedicatedWorker thread 2975502 | 13800 | 0.0% | 90.3% | 9.5% | 0.2% | 0.0% | 0.0% |
| DedicatedWorker thread 2975496 | 13609 | 0.0% | 89.4% | 10.3% | 0.2% | 0.0% | 0.0% |
| DedicatedWorker thread 2975501 | 13548 | 0.0% | 90.5% | 9.2% | 0.2% | 0.0% | 0.0% |
| DedicatedWorker thread 2975498 | 13547 | 0.0% | 90.8% | 8.9% | 0.3% | 0.0% | 0.0% |
| CrRendererMain 2974534 | 13117 | 69.8% | 12.2% | 0.0% | 17.1% | 1.0% | 0.0% |
| DedicatedWorker thread 2975166 | 12195 | 29.9% | 20.2% | 47.5% | 2.3% | 0.2% | 0.0% |

#### top self functions (wasm/js), W = tPage-200ms .. tPage+200ms
- DedicatedWorker thread 2975497: wasm:fle_gil_wake 1.3%; wasm:poll_waiters 1.1%; wasm:wasm-function[7159] 1.0%; wasm:wasm-function[7351] 0.6%; wasm:wasm-function[7352] 0.4%; wasm:fle_thread_wasm_idle_wait 0.4%; wasm:g9_hook 0.4%; wasm:fle_hyb_dispatch 0.3%
- DedicatedWorker thread 2975500: wasm:poll_waiters 2.6%; wasm:fle_gil_wake 2.0%; wasm:wasm-function[7159] 1.9%; wasm:g9_hook 1.4%; wasm:wasm-function[7351] 1.3%; wasm:round_pack 1.1%; wasm:wasm-function[7352] 1.1%; wasm:fle_x87_arith_st 0.8%
- DedicatedWorker thread 2975499: wasm:fle_gil_wake 1.1%; wasm:wasm-function[7159] 1.0%; wasm:poll_waiters 0.8%; wasm:g9_hook 0.6%; wasm:wasm-function[7351] 0.6%; wasm:wasm-function[7352] 0.5%; wasm:fle_gil_acquire_slow 0.3%; wasm:fle_hyb_dispatch 0.3%
- Realtime AudioWorklet thread 2975165: js:process 0.1%; js:postMessage 0.0%

#### samples by self class, B+C = tPage .. firstFrame (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 2975497 | 1937 | 0.0% | 90.8% | 8.9% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 2975500 | 1857 | 0.0% | 67.3% | 32.4% | 0.4% | 0.0% | 0.0% |
| DedicatedWorker thread 2975499 | 1835 | 0.0% | 88.3% | 11.4% | 0.2% | 0.1% | 0.0% |
| DedicatedWorker thread 2975496 | 1755 | 0.0% | 86.4% | 13.3% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 2975502 | 1741 | 0.0% | 88.9% | 11.0% | 0.1% | 0.0% | 0.0% |
| Realtime AudioWorklet thread 2975165 | 1741 | 99.8% | 0.1% | 0.0% | 0.1% | 0.0% | 0.0% |
| DedicatedWorker thread 2975498 | 1735 | 0.0% | 88.5% | 11.4% | 0.1% | 0.0% | 0.0% |
| DedicatedWorker thread 2975501 | 1691 | 0.0% | 89.2% | 10.6% | 0.1% | 0.0% | 0.0% |
| CrRendererMain 2974534 | 1666 | 89.2% | 10.3% | 0.0% | 0.5% | 0.0% | 0.0% |
| DedicatedWorker thread 2975166 | 1476 | 1.4% | 21.2% | 77.0% | 0.4% | 0.0% | 0.0% |

#### top self functions (wasm/js), B+C = tPage .. firstFrame
- DedicatedWorker thread 2975497: wasm:fle_gil_wake 1.2%; wasm:wasm-function[7159] 1.1%; wasm:poll_waiters 0.8%; wasm:wasm-function[7351] 0.4%; wasm:wasm-function[7352] 0.4%; wasm:wasm-function[7106] 0.4%; wasm:g9_hook 0.3%; wasm:fle_thread_wasm_idle_wait 0.3%
- DedicatedWorker thread 2975500: wasm:poll_waiters 2.7%; wasm:wasm-function[7159] 2.7%; wasm:fle_gil_wake 2.0%; wasm:round_pack 1.4%; wasm:g9_hook 1.3%; wasm:wasm-function[7351] 1.2%; wasm:wasm-function[7352] 0.9%; wasm:fle_x87_arith_st 0.7%
- DedicatedWorker thread 2975499: wasm:wasm-function[7159] 1.7%; wasm:fle_gil_wake 1.6%; wasm:poll_waiters 0.9%; wasm:g9_hook 0.4%; wasm:wasm-function[7352] 0.4%; wasm:wasm-function[7351] 0.4%; wasm:fle_gil_acquire_slow 0.3%; wasm:fle_hyb_dispatch 0.3%
- DedicatedWorker thread 2975496: wasm:wasm-function[7159] 1.7%; wasm:fle_gil_wake 1.4%; wasm:wasm-function[7351] 0.7%; wasm:poll_waiters 0.7%; wasm:wasm-function[7352] 0.6%; wasm:g9_hook 0.5%; wasm:fle_hyb_mx 0.5%; wasm:wasm-function[7267] 0.4%


### Escape (key-down): n=12
| segment | median ms | p90 | max |
|---|---|---|---|
| A tEvent->tPage | 0.45 | 0.50 | 0.55 |
| B tPage->tInject | 1.20 | 4.11 | 5.41 |
| C tInject->firstFrame | 6.64 | 16.96 | 17.98 |
| D firstFrame->raf | 2.91 | 11.82 | 14.84 |
| T tPage->firstFrame | 10.27 | 17.14 | 18.15 |
| T2 tPage->raf | 13.92 | 28.55 | 28.96 |

#### samples by self class, W = tPage-200ms .. tPage+200ms (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 2975497 | 15497 | 0.0% | 89.3% | 10.3% | 0.4% | 0.0% | 0.0% |
| DedicatedWorker thread 2975500 | 14403 | 0.0% | 65.5% | 33.9% | 0.5% | 0.1% | 0.0% |
| DedicatedWorker thread 2975499 | 14237 | 0.0% | 88.2% | 11.4% | 0.3% | 0.0% | 0.0% |
| Realtime AudioWorklet thread 2975165 | 13849 | 99.8% | 0.1% | 0.0% | 0.1% | 0.0% | 0.0% |
| DedicatedWorker thread 2975502 | 13719 | 0.0% | 90.0% | 9.6% | 0.3% | 0.1% | 0.0% |
| DedicatedWorker thread 2975496 | 13488 | 0.0% | 90.3% | 9.4% | 0.2% | 0.0% | 0.0% |
| DedicatedWorker thread 2975501 | 13461 | 0.0% | 91.9% | 7.8% | 0.2% | 0.0% | 0.0% |
| DedicatedWorker thread 2975498 | 13380 | 0.0% | 91.0% | 8.7% | 0.3% | 0.0% | 0.0% |
| CrRendererMain 2974534 | 12610 | 44.0% | 14.7% | 0.0% | 39.4% | 1.9% | 0.0% |
| DedicatedWorker thread 2975166 | 11975 | 31.1% | 24.7% | 42.1% | 2.0% | 0.2% | 0.0% |

#### top self functions (wasm/js), W = tPage-200ms .. tPage+200ms
- DedicatedWorker thread 2975497: wasm:fle_gil_wake 1.3%; wasm:poll_waiters 1.0%; wasm:wasm-function[7159] 0.8%; wasm:wasm-function[7351] 0.6%; wasm:wasm-function[7352] 0.5%; wasm:g9_hook 0.3%; wasm:fle_thread_wasm_idle_wait 0.3%; wasm:fle_gil_acquire_slow 0.3%
- DedicatedWorker thread 2975500: wasm:poll_waiters 2.8%; wasm:wasm-function[7159] 2.0%; wasm:fle_gil_wake 2.0%; wasm:wasm-function[7351] 1.3%; wasm:g9_hook 1.3%; wasm:round_pack 1.1%; wasm:wasm-function[7352] 1.0%; wasm:binop 0.9%
- DedicatedWorker thread 2975499: wasm:poll_waiters 1.1%; wasm:fle_gil_wake 1.0%; wasm:wasm-function[7159] 0.9%; wasm:wasm-function[7351] 0.8%; wasm:wasm-function[7352] 0.6%; wasm:g9_hook 0.6%; wasm:fle_hyb_dispatch 0.3%; wasm:wasm-function[7161] 0.2%
- Realtime AudioWorklet thread 2975165: js:process 0.1%; js:postMessage 0.0%

#### samples by self class, B+C = tPage .. firstFrame (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 2975497 | 439 | 0.0% | 90.7% | 9.1% | 0.2% | 0.0% | 0.0% |
| DedicatedWorker thread 2975500 | 419 | 0.0% | 56.8% | 42.2% | 0.7% | 0.2% | 0.0% |
| DedicatedWorker thread 2975499 | 400 | 0.0% | 92.5% | 7.2% | 0.2% | 0.0% | 0.0% |
| DedicatedWorker thread 2975502 | 394 | 0.0% | 94.7% | 4.8% | 0.5% | 0.0% | 0.0% |
| Realtime AudioWorklet thread 2975165 | 377 | 100.0% | 0.0% | 0.0% | 0.0% | 0.0% | 0.0% |
| DedicatedWorker thread 2975501 | 376 | 0.0% | 89.4% | 10.6% | 0.0% | 0.0% | 0.0% |
| CrRendererMain 2974534 | 372 | 85.2% | 13.7% | 0.0% | 1.1% | 0.0% | 0.0% |
| DedicatedWorker thread 2975496 | 369 | 0.0% | 91.3% | 8.4% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 2975498 | 360 | 0.0% | 91.9% | 8.1% | 0.0% | 0.0% | 0.0% |
| DedicatedWorker thread 2975166 | 343 | 5.8% | 18.4% | 74.6% | 1.2% | 0.0% | 0.0% |

#### top self functions (wasm/js), B+C = tPage .. firstFrame
- DedicatedWorker thread 2975497: wasm:fle_gil_wake 1.8%; wasm:wasm-function[7159] 1.6%; wasm:fle_gil_acquire_slow 0.7%; wasm:fle_hyb_mx 0.7%; wasm:fle_thread_wasm_idle_wait 0.5%; wasm:bf_mul 0.5%; wasm:wasm-function[7352] 0.5%; wasm:wasm-function[7154] 0.2%
- DedicatedWorker thread 2975500: wasm:wasm-function[7159] 3.1%; wasm:poll_waiters 2.9%; wasm:fle_gil_wake 2.9%; wasm:wasm-function[7224] 1.4%; wasm:wasm-function[1911] 1.2%; wasm:wasm-function[7267] 1.2%; wasm:fle_mx_subss_82753 1.0%; wasm:g9_hook 1.0%
- DedicatedWorker thread 2975499: wasm:wasm-function[7159] 1.5%; wasm:wasm-function[846] 0.5%; wasm:fle_gil_acquire_slow 0.5%; wasm:poll_waiters 0.5%; wasm:__wrap_fle_imp_WaitForSingleObject 0.5%; wasm:wasm-function[1626] 0.2%; wasm:wasm-function[1627] 0.2%; js:queueMicrotask 0.2%
- DedicatedWorker thread 2975502: wasm:fle_gil_wake 0.8%; wasm:wasm-function[846] 0.5%; js:queueMicrotask 0.5%; wasm:f_12c70f0 0.3%; wasm:printf_core 0.3%; wasm:wasm-function[7224] 0.3%; wasm:wasm-function[7159] 0.3%; wasm:wasm-function[7352] 0.3%


#### RunTask busy ms per input inside W (+-200 ms = 400 ms), File click (n=12)
| thread | busy ms per input | tasks per input | longest task ms |
|---|---|---|---|
| DedicatedWorker thread 2975166 | 288.0 | 203.5 | 61.40 |
| CrRendererMain 2974534 | 99.4 | 123.8 | 295.53 |
| Realtime AudioWorklet thread 2975165 | 5.8 | 37.6 | 0.51 |
| Compositor 2974558 | 3.7 | 74.2 | 0.59 |
| Chrome_ChildIOThread 2974545 | 0.4 | 11.1 | 0.24 |
| PerfettoTrace 2974540 | 0.2 | 4.8 | 0.23 |
| ThreadPoolServiceThread 2974541 | 0.0 | 0.1 | 0.10 |

#### RunTask busy ms per input inside W (+-200 ms = 400 ms), Escape (n=12)
| thread | busy ms per input | tasks per input | longest task ms |
|---|---|---|---|
| DedicatedWorker thread 2975166 | 287.6 | 227.8 | 32.10 |
| CrRendererMain 2974534 | 210.8 | 140.6 | 270.52 |
| Realtime AudioWorklet thread 2975165 | 5.8 | 37.4 | 0.71 |
| Compositor 2974558 | 4.1 | 73.5 | 1.02 |
| Chrome_ChildIOThread 2974545 | 0.8 | 23.5 | 0.10 |
| PerfettoTrace 2974540 | 0.2 | 3.8 | 0.25 |
| ThreadPoolServiceThread 2974541 | 0.0 | 0.5 | 0.10 |
