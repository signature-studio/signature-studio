## RT-G: names resolved from the wasm name section: 126911

### File click (n=12): phase C span mean 39.42 / median 44.81 ms; UI worker profile covers 39.34 ms/input
| thread | role | CPU ms per input in phase C | wall ms per input | CPU/wall |
|---|---|---|---|---|
| DedicatedWorker thread 3098898 | UI worker | 3.1 | 3.5 | 0.89 |
| CrRendererMain 3098699 | CrRendererMain | 3.5 | 33.4 | 0.11 |
| Realtime AudioWorklet thread 3098897 | Realtime AudioWorklet thread | 2.0 | 27.7 | 0.07 |
| DedicatedWorker thread 3099025 | Worker | 6.3 | 26.7 | 0.24 |
| DedicatedWorker thread 3099026 | Worker | 6.9 | 27.8 | 0.25 |
| DedicatedWorker thread 3099027 | Worker | 4.6 | 26.5 | 0.17 |
| DedicatedWorker thread 3099028 | Worker | 6.5 | 26.8 | 0.24 |
| DedicatedWorker thread 3099029 | Worker | 15.5 | 26.3 | 0.59 |
| DedicatedWorker thread 3099030 | Worker | 5.7 | 24.9 | 0.23 |
| DedicatedWorker thread 3099031 | Worker | 7.1 | 30.6 | 0.23 |

#### UI worker self top 14 in phase C with names from the name section (ms/input)
| function | ms/input |
|---|---|
| wasm-function[1368] = (no name in the section) | 10.74 |
| (program) | 7.66 |
| wasm-function[3035] = (no name in the section) | 5.05 |
| g9_hook | 3.75 |
| fle_desk_compose | 1.43 |
| wasm-function[3145] = (no name in the section) | 1.40 |
| wasm-function[5212] = (no name in the section) | 1.25 |
| fle_gil_wake | 0.51 |
| wasm-function[965] = (no name in the section) | 0.29 |
| wasm-function[3590] = (no name in the section) | 0.21 |
| wasm-function[1746] = (no name in the section) | 0.21 |
| fle_gil_acquire_slow | 0.16 |
| wasm-function[2908] = (no name in the section) | 0.16 |
| poll_waiters | 0.14 |

### Escape (n=12): phase C span mean 11.91 / median 10.03 ms; UI worker profile covers 11.90 ms/input
| thread | role | CPU ms per input in phase C | wall ms per input | CPU/wall |
|---|---|---|---|---|
| DedicatedWorker thread 3098898 | UI worker | 2.9 | 3.4 | 0.87 |
| CrRendererMain 3098699 | CrRendererMain | 1.1 | 7.7 | 0.15 |
| Realtime AudioWorklet thread 3098897 | Realtime AudioWorklet thread | 0.3 | 2.7 | 0.09 |
| DedicatedWorker thread 3099025 | Worker | 1.0 | 6.0 | 0.17 |
| DedicatedWorker thread 3099026 | Worker | 2.2 | 7.7 | 0.29 |
| DedicatedWorker thread 3099027 | Worker | 1.7 | 5.7 | 0.30 |
| DedicatedWorker thread 3099028 | Worker | 2.0 | 7.2 | 0.27 |
| DedicatedWorker thread 3099029 | Worker | 3.5 | 6.0 | 0.58 |
| DedicatedWorker thread 3099030 | Worker | 1.2 | 5.5 | 0.22 |
| DedicatedWorker thread 3099031 | Worker | 1.4 | 6.7 | 0.21 |

#### UI worker self top 14 in phase C with names from the name section (ms/input)
| function | ms/input |
|---|---|
| (program) | 3.14 |
| wasm-function[1368] = (no name in the section) | 2.46 |
| fle_desk_compose | 1.08 |
| wasm-function[3035] = (no name in the section) | 0.70 |
| g9_hook | 0.48 |
| wasm-function[3145] = (no name in the section) | 0.37 |
| wasm-function[5212] = (no name in the section) | 0.23 |
| wasm-function[3590] = (no name in the section) | 0.16 |
| wasm-function[5247] = (no name in the section) | 0.14 |
| fle_gil_acquire_slow | 0.12 |
| wasm-function[1746] = (no name in the section) | 0.11 |
| wasm-function[965] = (no name in the section) | 0.11 |
| wasm-function[531] = (no name in the section) | 0.10 |
| fle_hyb_dispatch | 0.09 |
