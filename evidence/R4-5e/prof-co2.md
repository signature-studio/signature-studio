## Per run (thread time of the Worker threads in the profiled window)

| run | wall s | Worker CPU s (cpuS) | profiled Workers | busy s | idle s | busy ms per wall s |
|---|---:|---:|---:|---:|---:|---:|
| r45e-mixprof-p-chrome-2 | 322.0 | 276.51 | 1 | 274.2 | 47.9 | 851.4 |

## Self time by function (ms of thread time per wall second, all Workers; top 25 by the largest run)

| function | r45e-mixprof-p-chrome-2 |
|---|---:|
| wasm-function[1367] (instr 388, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 94.75 |
| (program) | 54.73 |
| wasm-function[7350] (instr 110710, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 48.40 |
| wasm-function[7351] (instr 110710, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 40.78 |
| wasm-function[7158] (instr 17181, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 37.76 |
| wasm-function[3034] (instr 502, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 28.52 |
| wasm-function[121363] (instr 1277, wait32 0, notify 0, cmpxchg 0, calls 1, import calls 0) | 18.20 |
| wasm-function[7105] (instr 23867, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 15.19 |
| wasm-function[7160] (instr 27158, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 12.37 |
| wasm-function[7223] (instr 74820, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 12.27 |
| wasm-function[8375] (instr 8728, wait32 0, notify 0, cmpxchg 0, calls 1, import calls 0) | 11.10 |
| texSubImage2D | 10.96 |
| wasm-function[122518] (instr 551, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 9.62 |
| wasm-function[7153] (instr 3651, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 9.46 |
| wasm-function[8386] (instr 1652, wait32 0, notify 0, cmpxchg 0, calls 1, import calls 0) | 8.35 |
| wasm-function[122522] (instr 1187, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 8.16 |
| wasm-function[1745] (instr 31247, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 7.50 |
| wasm-function[8419] (instr 68, wait32 0, notify 0, cmpxchg 0, calls 1, import calls 0) | 7.43 |
| wasm-function[7099] (instr 1897, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 6.98 |
| wasm-function[7215] (instr 2705, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 6.84 |
| wasm-function[65378] (instr 10606, wait32 0, notify 0, cmpxchg 0, calls 33, import calls 0) | 6.80 |
| wasm-function[7213] (instr 2376, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 6.69 |
| wasm-function[7104] (instr 28148, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 6.59 |
| wasm-function[845] (instr 5344, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 6.44 |
| wasm-function[7264] (instr 9510, wait32 0, notify 0, cmpxchg 0, calls 0, import calls 0) | 6.41 |

### r45e-mixprof-p-chrome-2: (program) time by the function that called it (ms of thread time per wall second)

| caller | ms per wall s |
|---|---:|
| (root) | 54.73 |

### r45e-mixprof-p-chrome-2: per Worker file (busy s, idle s)

| file | busy s | idle s | samples |
|---|---:|---:|---:|
| r45e-p3-w0.cpuprofile | 274.16 | 47.86 | 523939 |

