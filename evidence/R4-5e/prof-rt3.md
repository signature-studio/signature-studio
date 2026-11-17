## Per run (thread time of the Worker threads in the profiled window)

| run | wall s | Worker CPU s (cpuS) | profiled Workers | busy s | idle s | busy ms per wall s |
|---|---:|---:|---:|---:|---:|---:|
| r45e-mixprof-p-chrome-3 | 322.5 | 643.75 | 8 | 2500.1 | 90.4 | 7753.0 |

## Self time by function (ms of thread time per wall second, all Workers; top 25 by the largest run)

| function | r45e-mixprof-p-chrome-3 |
|---|---:|
| (program) | 6388.98 |
| wasm-function[122519] (instr 9, wait32 0, notify 1, cmpxchg 0, calls 0, import calls 0) | 95.96 |
| wasm-function[1367] (instr 388, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 90.38 |
| wasm-function[122945] (instr 491, wait32 0, notify 1, cmpxchg 0, calls 1, import calls 0) | 80.04 |
| wasm-function[7350] (instr 110710, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 65.96 |
| wasm-function[7158] (instr 17181, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 57.76 |
| wasm-function[7351] (instr 110710, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 56.22 |
| wasm-function[122203] (instr 1277, wait32 0, notify 0, cmpxchg 0, calls 1, import calls 0) | 33.67 |
| wasm-function[3034] (instr 502, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 25.49 |
| wasm-function[9213] (instr 8854, wait32 0, notify 0, cmpxchg 0, calls 1, import calls 0) | 24.15 |
| wasm-function[7105] (instr 23867, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 20.55 |
| wasm-function[122520] (instr 306, wait32 1, notify 0, cmpxchg 1, calls 2, import calls 0) | 17.47 |
| wasm-function[7223] (instr 74820, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 16.65 |
| wasm-function[7160] (instr 27158, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 15.92 |
| wasm-function[122989] (instr 71, wait32 1, notify 0, cmpxchg 1, calls 0, import calls 0) | 14.28 |
| wasm-function[9257] (instr 68, wait32 0, notify 0, cmpxchg 0, calls 1, import calls 0) | 12.36 |
| wasm-function[7153] (instr 3651, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 11.24 |
| wasm-function[9224] (instr 1653, wait32 0, notify 0, cmpxchg 0, calls 1, import calls 0) | 11.03 |
| wasm-function[123359] (instr 555, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 10.94 |
| wasm-function[1745] (instr 31247, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 10.76 |
| wasm-function[7177] (instr 26, wait32 0, notify 0, cmpxchg 0, calls 1, import calls 0) | 9.99 |
| wasm-function[7178] (instr 26, wait32 0, notify 0, cmpxchg 0, calls 1, import calls 0) | 9.97 |
| wasm-function[7099] (instr 1897, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 9.77 |
| wasm-function[7135] (instr 1004, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 9.38 |
| wasm-function[7104] (instr 28148, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 8.57 |

### r45e-mixprof-p-chrome-3: (program) time by the function that called it (ms of thread time per wall second)

| caller | ms per wall s |
|---|---:|
| (root) | 6388.98 |

### r45e-mixprof-p-chrome-3: per Worker file (busy s, idle s)

| file | busy s | idle s | samples |
|---|---:|---:|---:|
| r45e-p3-w0.cpuprofile | 232.40 | 90.40 | 480372 |
| r45e-p3-w1.cpuprofile | 323.20 | 0.00 | 493281 |
| r45e-p3-w2.cpuprofile | 323.44 | 0.00 | 492215 |
| r45e-p3-w3.cpuprofile | 323.68 | 0.00 | 493617 |
| r45e-p3-w4.cpuprofile | 323.92 | 0.00 | 493856 |
| r45e-p3-w5.cpuprofile | 324.17 | 0.00 | 491663 |
| r45e-p3-w6.cpuprofile | 324.51 | 0.00 | 494703 |
| r45e-p3-w7.cpuprofile | 324.77 | 0.00 | 495391 |

