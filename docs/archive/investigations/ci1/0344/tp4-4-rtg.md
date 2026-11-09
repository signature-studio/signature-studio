## RT-G: names resolved from the wasm name section: 126911

### File click (n=12): phase C span mean 44.36 / median 47.17 ms; UI worker profile covers 44.02 ms/input
| thread | role | CPU ms per input in phase C | wall ms per input | CPU/wall |
|---|---|---|---|---|
| DedicatedWorker thread 3103623 | UI worker | 0.4 | 0.4 | 0.92 |
| CrRendererMain 3103359 | CrRendererMain | 4.7 | 40.2 | 0.12 |
| Realtime AudioWorklet thread 3103622 | Realtime AudioWorklet thread | 2.8 | 37.4 | 0.08 |
| DedicatedWorker thread 3103828 | Worker | 9.2 | 38.3 | 0.24 |
| DedicatedWorker thread 3103829 | Worker | 10.2 | 40.1 | 0.25 |
| DedicatedWorker thread 3103830 | Worker | 7.4 | 38.4 | 0.19 |
| DedicatedWorker thread 3103831 | Worker | 11.2 | 38.8 | 0.29 |
| DedicatedWorker thread 3103832 | Worker | 20.3 | 39.2 | 0.52 |
| DedicatedWorker thread 3103833 | Worker | 7.8 | 38.1 | 0.20 |
| DedicatedWorker thread 3103834 | Worker | 9.7 | 38.4 | 0.25 |

#### UI worker self top 14 in phase C with names from the name section (ms/input)
| function | ms/input |
|---|---|
| wasm-function[1368] = (no name in the section) | 11.35 |
| (program) | 10.58 |
| wasm-function[3035] = (no name in the section) | 5.04 |
| g9_hook | 3.27 |
| wasm-function[3145] = (no name in the section) | 1.94 |
| fle_desk_compose | 1.27 |
| wasm-function[5212] = (no name in the section) | 1.18 |
| fle_gil_wake | 1.06 |
| wasm-function[965] = (no name in the section) | 0.38 |
| wasm-function[973] = (no name in the section) | 0.33 |
| wasm-function[1746] = (no name in the section) | 0.26 |
| fle_dispatch | 0.22 |
| fle_gil_acquire_slow | 0.20 |
| wasm-function[6347] = (no name in the section) | 0.20 |

### Escape (n=12): phase C span mean 8.96 / median 7.29 ms; UI worker profile covers 8.62 ms/input
| thread | role | CPU ms per input in phase C | wall ms per input | CPU/wall |
|---|---|---|---|---|
| DedicatedWorker thread 3103623 | UI worker | 3.5 | 3.9 | 0.90 |
| CrRendererMain 3103359 | CrRendererMain | 0.9 | 6.6 | 0.13 |
| Realtime AudioWorklet thread 3103622 | Realtime AudioWorklet thread | 0.3 | 1.9 | 0.13 |
| DedicatedWorker thread 3103828 | Worker | 0.8 | 3.3 | 0.25 |
| DedicatedWorker thread 3103829 | Worker | 1.7 | 4.8 | 0.34 |
| DedicatedWorker thread 3103830 | Worker | 0.6 | 3.6 | 0.17 |
| DedicatedWorker thread 3103831 | Worker | 1.0 | 4.3 | 0.23 |
| DedicatedWorker thread 3103832 | Worker | 2.3 | 4.0 | 0.58 |
| DedicatedWorker thread 3103833 | Worker | 1.0 | 3.2 | 0.32 |
| DedicatedWorker thread 3103834 | Worker | 1.2 | 4.4 | 0.26 |

#### UI worker self top 14 in phase C with names from the name section (ms/input)
| function | ms/input |
|---|---|
| wasm-function[1368] = (no name in the section) | 2.05 |
| (program) | 1.86 |
| fle_desk_compose | 0.82 |
| wasm-function[3035] = (no name in the section) | 0.60 |
| g9_hook | 0.55 |
| wasm-function[3145] = (no name in the section) | 0.34 |
| wasm-function[5247] = (no name in the section) | 0.18 |
| fle_gil_wake | 0.11 |
| wasm-function[5212] = (no name in the section) | 0.11 |
| wasm-function[1366] = (no name in the section) | 0.11 |
| f_732d90 | 0.09 |
| wasm-function[135378] = (no name in the section) | 0.08 |
| wasm-function[531] = (no name in the section) | 0.07 |
| wasm-function[5234] = (no name in the section) | 0.07 |
