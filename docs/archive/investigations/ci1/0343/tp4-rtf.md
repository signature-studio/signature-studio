## RT-F: names resolved from the wasm name section: 126910

### File click (n=12): phase C span mean 47.85 / median 46.87 ms; UI worker profile covers 47.65 ms/input
| thread | role | CPU ms per input in phase C | wall ms per input | CPU/wall |
|---|---|---|---|---|
| DedicatedWorker thread 2975166 | UI worker | 4.3 | 5.1 | 0.84 |
| CrRendererMain 2974534 | CrRendererMain | 5.0 | 39.7 | 0.13 |
| Realtime AudioWorklet thread 2975165 | Realtime AudioWorklet thread | 2.8 | 36.8 | 0.08 |
| DedicatedWorker thread 2975496 | Worker | 10.3 | 38.3 | 0.27 |
| DedicatedWorker thread 2975497 | Worker | 10.0 | 36.8 | 0.27 |
| DedicatedWorker thread 2975498 | Worker | 8.2 | 36.5 | 0.22 |
| DedicatedWorker thread 2975499 | Worker | 8.7 | 37.0 | 0.24 |
| DedicatedWorker thread 2975500 | Worker | 20.9 | 37.1 | 0.56 |
| DedicatedWorker thread 2975501 | Worker | 8.2 | 36.6 | 0.22 |
| DedicatedWorker thread 2975502 | Worker | 8.3 | 39.2 | 0.21 |

#### UI worker self top 14 in phase C with names from the name section (ms/input)
| function | ms/input |
|---|---|
| wasm-function[1368] = (no name in the section) | 12.41 |
| (program) | 10.19 |
| wasm-function[3035] = (no name in the section) | 5.59 |
| g9_hook | 3.93 |
| fle_desk_compose | 2.07 |
| wasm-function[3145] = (no name in the section) | 1.79 |
| wasm-function[5212] = (no name in the section) | 0.79 |
| fle_gil_wake | 0.57 |
| wasm-function[3590] = (no name in the section) | 0.48 |
| wasm-function[1746] = (no name in the section) | 0.44 |
| fle_gil_acquire_slow | 0.40 |
| (idle) | 0.30 |
| wasm-function[2908] = (no name in the section) | 0.29 |
| wasm-function[973] = (no name in the section) | 0.21 |

### Escape (n=12): phase C span mean 8.97 / median 6.64 ms; UI worker profile covers 8.93 ms/input
| thread | role | CPU ms per input in phase C | wall ms per input | CPU/wall |
|---|---|---|---|---|
| DedicatedWorker thread 2975166 | UI worker | 1.0 | 1.2 | 0.89 |
| CrRendererMain 2974534 | CrRendererMain | 0.6 | 4.2 | 0.14 |
| Realtime AudioWorklet thread 2975165 | Realtime AudioWorklet thread | 0.1 | 0.1 | 0.88 |
| DedicatedWorker thread 2975496 | Worker | 0.8 | 3.2 | 0.25 |
| DedicatedWorker thread 2975497 | Worker | 1.5 | 4.7 | 0.33 |
| DedicatedWorker thread 2975498 | Worker | 1.1 | 4.1 | 0.27 |
| DedicatedWorker thread 2975499 | Worker | 1.0 | 3.9 | 0.27 |
| DedicatedWorker thread 2975500 | Worker | 3.2 | 5.1 | 0.63 |
| DedicatedWorker thread 2975501 | Worker | 1.3 | 4.3 | 0.30 |
| DedicatedWorker thread 2975502 | Worker | 0.6 | 3.3 | 0.17 |

#### UI worker self top 14 in phase C with names from the name section (ms/input)
| function | ms/input |
|---|---|
| wasm-function[1368] = (no name in the section) | 2.36 |
| (program) | 1.50 |
| fle_desk_compose | 1.04 |
| wasm-function[3035] = (no name in the section) | 0.82 |
| g9_hook | 0.40 |
| wasm-function[973] = (no name in the section) | 0.18 |
| wasm-function[3145] = (no name in the section) | 0.16 |
| putImageData | 0.15 |
| fle_gil_wake | 0.14 |
| wasm-function[5212] = (no name in the section) | 0.13 |
| wasm-function[529] = (no name in the section) | 0.13 |
| wasm-function[531] = (no name in the section) | 0.12 |
| fle_ld | 0.10 |
| wasm-function[1746] = (no name in the section) | 0.09 |
