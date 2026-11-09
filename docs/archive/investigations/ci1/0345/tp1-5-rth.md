## RT-H: 41 answered records, profiles 10
- profile 0x3: thread 3303947 DedicatedWorker thread samples 83592
- profile 0x6: thread 3303949 DedicatedWorker thread samples 78395
- profile 0x7: thread 3303951 DedicatedWorker thread samples 73602
- profile 0x1: thread 3303702 DedicatedWorker thread samples 64608
- profile 0x2: thread 3303946 DedicatedWorker thread samples 73570
- profile 0x4: thread 3303948 DedicatedWorker thread samples 73552
- profile 0x5: thread 3303950 DedicatedWorker thread samples 78740
- profile 0x8: thread 3303952 DedicatedWorker thread samples 75243
- profile 0x9: thread 3303574 CrRendererMain samples 68539
- profile 0xa: thread 3303701 Realtime AudioWorklet thread samples 76800

### File click (button-down): n=12
| segment | median ms | p90 | max |
|---|---|---|---|
| A tEvent->tPage | 0.72 | 1.53 | 3.85 |
| B tPage->tInject | 5.09 | 9.75 | 11.42 |
| C tInject->firstFrame | 38.92 | 42.61 | 67.74 |
| D firstFrame->raf | 4.13 | 15.86 | 16.29 |
| T tPage->firstFrame | 45.18 | 46.65 | 79.16 |
| T2 tPage->raf | 49.43 | 57.94 | 88.99 |

#### samples by self class, W = tPage-200ms .. tPage+200ms (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3303947 | 14815 | 0.0% | 89.1% | 10.5% | 0.4% | 0.1% | 0.0% |
| DedicatedWorker thread 3303950 | 13958 | 0.0% | 66.0% | 33.4% | 0.6% | 0.1% | 0.0% |
| DedicatedWorker thread 3303949 | 13855 | 0.0% | 90.1% | 9.5% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3303952 | 13301 | 0.0% | 89.7% | 9.9% | 0.4% | 0.0% | 0.0% |
| Realtime AudioWorklet thread 3303701 | 13209 | 99.7% | 0.1% | 0.0% | 0.2% | 0.0% | 0.0% |
| DedicatedWorker thread 3303951 | 13058 | 0.0% | 89.9% | 9.8% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3303946 | 13024 | 0.0% | 90.8% | 8.8% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3303948 | 12980 | 0.0% | 91.2% | 8.5% | 0.3% | 0.0% | 0.0% |
| CrRendererMain 3303574 | 12483 | 69.3% | 13.3% | 0.0% | 16.4% | 0.9% | 0.0% |
| DedicatedWorker thread 3303702 | 11401 | 26.7% | 22.0% | 48.6% | 2.5% | 0.3% | 0.0% |

#### top self functions (wasm/js), W = tPage-200ms .. tPage+200ms
- DedicatedWorker thread 3303947: wasm:fle_gil_wake 1.5%; wasm:poll_waiters 1.2%; wasm:wasm-function[7351] 0.6%; wasm:wasm-function[7352] 0.6%; wasm:wasm-function[7159] 0.5%; wasm:fle_thread_wasm_idle_wait 0.3%; wasm:wasm-function[7106] 0.2%; wasm:fle_gil_acquire_slow 0.2%
- DedicatedWorker thread 3303950: wasm:poll_waiters 2.6%; wasm:fle_gil_wake 2.4%; wasm:wasm-function[7351] 1.4%; wasm:wasm-function[7159] 1.2%; wasm:wasm-function[7352] 1.2%; wasm:round_pack 1.2%; wasm:binop 0.9%; wasm:fle_x87_arith_st 0.7%
- DedicatedWorker thread 3303949: wasm:fle_gil_wake 1.2%; wasm:poll_waiters 0.8%; wasm:wasm-function[7351] 0.6%; wasm:wasm-function[7159] 0.6%; wasm:wasm-function[7352] 0.5%; wasm:fle_hyb_mx 0.3%; wasm:fle_gil_acquire_slow 0.2%; wasm:round_pack 0.2%
- DedicatedWorker thread 3303952: wasm:fle_gil_wake 1.0%; wasm:poll_waiters 0.8%; wasm:wasm-function[7351] 0.7%; wasm:wasm-function[7352] 0.7%; wasm:wasm-function[7159] 0.5%; wasm:fle_thread_wasm_idle_wait 0.2%; wasm:fle_hyb_mx 0.2%; wasm:wasm-function[7161] 0.2%

#### samples by self class, B+C = tPage .. firstFrame (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3303947 | 1654 | 0.0% | 87.8% | 11.9% | 0.2% | 0.0% | 0.0% |
| DedicatedWorker thread 3303950 | 1543 | 0.0% | 66.0% | 33.6% | 0.4% | 0.0% | 0.0% |
| DedicatedWorker thread 3303949 | 1528 | 0.0% | 91.0% | 8.8% | 0.2% | 0.0% | 0.0% |
| Realtime AudioWorklet thread 3303701 | 1523 | 99.7% | 0.1% | 0.0% | 0.1% | 0.0% | 0.0% |
| DedicatedWorker thread 3303951 | 1467 | 0.0% | 89.4% | 10.2% | 0.4% | 0.0% | 0.0% |
| DedicatedWorker thread 3303952 | 1464 | 0.0% | 91.8% | 7.9% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3303946 | 1448 | 0.0% | 90.3% | 9.3% | 0.3% | 0.1% | 0.0% |
| DedicatedWorker thread 3303948 | 1445 | 0.0% | 92.3% | 7.2% | 0.5% | 0.0% | 0.0% |
| CrRendererMain 3303574 | 1390 | 88.1% | 11.4% | 0.0% | 0.4% | 0.0% | 0.0% |
| DedicatedWorker thread 3303702 | 1205 | 1.0% | 21.0% | 77.3% | 0.7% | 0.0% | 0.0% |

#### top self functions (wasm/js), B+C = tPage .. firstFrame
- DedicatedWorker thread 3303947: wasm:fle_gil_wake 2.1%; wasm:poll_waiters 1.1%; wasm:wasm-function[7351] 0.8%; wasm:wasm-function[7352] 0.7%; wasm:wasm-function[7159] 0.6%; wasm:wasm-function[7178] 0.4%; wasm:wasm-function[7157] 0.3%; wasm:fle_gil_acquire_slow 0.3%
- DedicatedWorker thread 3303950: wasm:fle_gil_wake 3.2%; wasm:poll_waiters 1.9%; wasm:wasm-function[7159] 1.6%; wasm:wasm-function[7351] 1.5%; wasm:round_pack 1.4%; wasm:wasm-function[7352] 1.3%; wasm:binop 1.2%; wasm:wasm-function[7179] 0.8%
- DedicatedWorker thread 3303949: wasm:fle_gil_wake 1.2%; wasm:wasm-function[7159] 0.9%; wasm:poll_waiters 0.6%; wasm:wasm-function[7352] 0.4%; wasm:wasm-function[7351] 0.3%; wasm:wasm-function[7106] 0.3%; wasm:round_pack 0.3%; wasm:fle_dispatch 0.2%
- Realtime AudioWorklet thread 3303701: js:process 0.1%


### Escape (key-down): n=12
| segment | median ms | p90 | max |
|---|---|---|---|
| A tEvent->tPage | 0.60 | 0.75 | 0.88 |
| B tPage->tInject | 0.29 | 2.72 | 4.15 |
| C tInject->firstFrame | 7.47 | 17.81 | 18.92 |
| D firstFrame->raf | 3.05 | 3.78 | 9.27 |
| T tPage->firstFrame | 10.10 | 18.01 | 19.14 |
| T2 tPage->raf | 13.44 | 21.31 | 22.22 |

#### samples by self class, W = tPage-200ms .. tPage+200ms (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3303947 | 14289 | 0.0% | 90.6% | 9.0% | 0.4% | 0.1% | 0.0% |
| DedicatedWorker thread 3303949 | 13637 | 0.0% | 89.4% | 10.3% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3303950 | 13414 | 0.0% | 64.7% | 34.7% | 0.5% | 0.1% | 0.0% |
| Realtime AudioWorklet thread 3303701 | 13139 | 99.7% | 0.1% | 0.0% | 0.1% | 0.0% | 0.0% |
| DedicatedWorker thread 3303952 | 13053 | 0.0% | 90.2% | 9.4% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3303946 | 12703 | 0.0% | 91.8% | 7.9% | 0.4% | 0.0% | 0.0% |
| DedicatedWorker thread 3303948 | 12671 | 0.0% | 91.7% | 8.0% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3303951 | 12652 | 0.0% | 90.3% | 9.4% | 0.3% | 0.1% | 0.0% |
| CrRendererMain 3303574 | 12193 | 44.5% | 17.7% | 0.0% | 35.3% | 2.5% | 0.0% |
| DedicatedWorker thread 3303702 | 11394 | 27.9% | 26.2% | 43.3% | 2.4% | 0.2% | 0.0% |

#### top self functions (wasm/js), W = tPage-200ms .. tPage+200ms
- DedicatedWorker thread 3303947: wasm:fle_gil_wake 1.3%; wasm:poll_waiters 1.1%; wasm:wasm-function[7352] 0.4%; wasm:wasm-function[7351] 0.4%; wasm:fle_gil_acquire_slow 0.4%; wasm:fle_thread_wasm_idle_wait 0.4%; wasm:wasm-function[7159] 0.4%; wasm:fle_hyb_mx 0.2%
- DedicatedWorker thread 3303949: wasm:fle_gil_wake 1.0%; wasm:poll_waiters 0.9%; wasm:wasm-function[7351] 0.8%; wasm:wasm-function[7352] 0.7%; wasm:wasm-function[7159] 0.6%; wasm:wasm-function[7106] 0.3%; wasm:fle_gil_acquire_slow 0.3%; wasm:fle_hyb_mx 0.2%
- DedicatedWorker thread 3303950: wasm:poll_waiters 2.6%; wasm:fle_gil_wake 1.8%; wasm:wasm-function[7351] 1.5%; wasm:wasm-function[7159] 1.4%; wasm:wasm-function[7352] 1.3%; wasm:binop 0.8%; wasm:round_pack 0.8%; wasm:fle_x87_arith_st 0.8%
- Realtime AudioWorklet thread 3303701: js:process 0.1%; js:postMessage 0.1%

#### samples by self class, B+C = tPage .. firstFrame (sum over 12 inputs)
| thread (profile) | samples | idle | program | wasm | js | gc | other |
|---|---|---|---|---|---|---|---|
| DedicatedWorker thread 3303947 | 397 | 0.0% | 91.2% | 8.8% | 0.0% | 0.0% | 0.0% |
| DedicatedWorker thread 3303950 | 378 | 0.0% | 64.8% | 34.1% | 0.0% | 1.1% | 0.0% |
| DedicatedWorker thread 3303949 | 371 | 0.0% | 88.4% | 11.1% | 0.5% | 0.0% | 0.0% |
| DedicatedWorker thread 3303951 | 354 | 0.0% | 90.4% | 9.0% | 0.6% | 0.0% | 0.0% |
| DedicatedWorker thread 3303946 | 353 | 0.0% | 90.7% | 8.2% | 1.1% | 0.0% | 0.0% |
| DedicatedWorker thread 3303952 | 352 | 0.0% | 90.9% | 8.8% | 0.3% | 0.0% | 0.0% |
| Realtime AudioWorklet thread 3303701 | 351 | 99.7% | 0.0% | 0.0% | 0.3% | 0.0% | 0.0% |
| CrRendererMain 3303574 | 341 | 82.4% | 15.5% | 0.0% | 2.1% | 0.0% | 0.0% |
| DedicatedWorker thread 3303948 | 337 | 0.0% | 90.8% | 8.9% | 0.3% | 0.0% | 0.0% |
| DedicatedWorker thread 3303702 | 312 | 5.4% | 24.4% | 68.9% | 1.3% | 0.0% | 0.0% |

#### top self functions (wasm/js), B+C = tPage .. firstFrame
- DedicatedWorker thread 3303947: wasm:fle_gil_wake 1.0%; wasm:poll_waiters 0.8%; wasm:wasm-function[7136] 0.5%; wasm:fle_gil_acquire_slow 0.5%; wasm:wasm-function[7351] 0.5%; wasm:fle_hyb_mx 0.5%; wasm:wasm-function[7157] 0.5%; wasm:wasm-function[7159] 0.5%
- DedicatedWorker thread 3303950: wasm:fle_gil_wake 2.4%; wasm:wasm-function[7159] 1.9%; wasm:binop 1.6%; wasm:wasm-function[7351] 1.3%; wasm:wasm-function[4551] 1.1%; wasm:poll_waiters 1.1%; wasm:wasm-function[7352] 1.1%; wasm:bf_mul 1.1%
- DedicatedWorker thread 3303949: wasm:poll_waiters 1.6%; wasm:fle_gil_wake 1.1%; wasm:fle_hyb_mx 0.8%; wasm:wasm-function[7352] 0.8%; wasm:fle_thread_wait_objects 0.5%; wasm:wasm-function[5349] 0.3%; wasm:schedule_ex 0.3%; wasm:wasm-function[1626] 0.3%
- DedicatedWorker thread 3303951: wasm:wasm-function[7352] 1.4%; wasm:fle_thread_wasm_idle_wait 0.8%; wasm:poll_waiters 0.8%; wasm:fle_gil_wake 0.6%; wasm:wasm-function[7159] 0.6%; wasm:wasm-function[7351] 0.6%; wasm:wasm-function[441] 0.3%; wasm:wasm-function[7105] 0.3%


#### RunTask busy ms per input inside W (+-200 ms = 400 ms), File click (n=12)
| thread | busy ms per input | tasks per input | longest task ms |
|---|---|---|---|
| DedicatedWorker thread 3303702 | 301.9 | 202.0 | 84.06 |
| CrRendererMain 3303574 | 102.1 | 125.0 | 368.52 |
| Realtime AudioWorklet thread 3303701 | 6.9 | 37.5 | 1.13 |
| Compositor 3303594 | 4.3 | 78.0 | 0.84 |
| Chrome_ChildIOThread 3303585 | 0.4 | 10.5 | 0.16 |
| PerfettoTrace 3303580 | 0.3 | 6.6 | 0.18 |
| ThreadPoolServiceThread 3303581 | 0.0 | 0.2 | 0.12 |

#### RunTask busy ms per input inside W (+-200 ms = 400 ms), Escape (n=12)
| thread | busy ms per input | tasks per input | longest task ms |
|---|---|---|---|
| DedicatedWorker thread 3303702 | 300.8 | 228.8 | 45.33 |
| CrRendererMain 3303574 | 205.2 | 151.4 | 294.08 |
| Realtime AudioWorklet thread 3303701 | 7.3 | 37.5 | 1.57 |
| Compositor 3303594 | 5.1 | 73.5 | 1.21 |
| Chrome_ChildIOThread 3303585 | 0.9 | 23.0 | 0.19 |
| PerfettoTrace 3303580 | 0.4 | 5.7 | 0.23 |
| ThreadPoolServiceThread 3303581 | 0.0 | 0.4 | 0.18 |
