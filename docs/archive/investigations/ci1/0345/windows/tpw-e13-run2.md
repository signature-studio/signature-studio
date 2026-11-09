## E13-run2: UI Worker tid 3042257; File clicks between the sync events: 12; profile samples 180595

Units: ms (not per input) inside each window. W = tPage +-200 ms (400 ms), C = tInject .. first frame.

| # | tPage (ms after sync-a) | A tEvent->tPage | B tPage->tInject | C span (tInject->first frame) | C: total | C: (program) | C: (idle) | C: wasm anon self | C: named self | C: fle_gil_* self | C: top self functions (ms) | W: total | W: (program) | W: (idle) | W: wasm anon self | W: named self | W: fle_gil_* self | W: top self functions (ms) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 67 | 1.00 | 0.17 | 23.37 | 23.30 | 0.24 | 0.00 | 21.96 | 0.00 | 0.00 | wasm-function[1367] 8.48; wasm-function[3034] 3.79; wasm-function[99865] 1.83 | 399.82 | 52.72 | 7.46 | 330.73 | 0.59 | 0.00 | wasm-function[1367] 25.14; wasm-function[7350] 22.14; wasm-function[7351] 16.28 |
| 2 | 2258 | 0.71 | 2.51 | 18.35 | 18.40 | 0.00 | 0.00 | 17.84 | 0.00 | 0.00 | wasm-function[1367] 8.62; wasm-function[2129] 3.99; wasm-function[3034] 1.20 | 400.01 | 68.00 | 29.10 | 294.07 | 0.24 | 0.00 | wasm-function[1367] 28.67; wasm-function[7351] 14.92; wasm-function[100982] 14.59 |
| 3 | 4507 | 0.61 | 5.31 | 18.54 | 18.63 | 0.22 | 0.00 | 17.75 | 0.00 | 0.00 | wasm-function[1367] 9.66; wasm-function[3034] 2.48; wasm-function[99865] 1.80 | 399.90 | 79.46 | 53.96 | 254.80 | 0.01 | 0.00 | wasm-function[1367] 28.36; wasm-function[7350] 12.84; wasm-function[7351] 12.52 |
| 4 | 6495 | 0.60 | 4.39 | 18.40 | 18.20 | 0.18 | 0.00 | 17.82 | 0.00 | 0.00 | wasm-function[1367] 10.36; wasm-function[3034] 2.68; wasm-function[99865] 1.58 | 399.94 | 80.80 | 5.55 | 308.19 | -0.00 | 0.00 | wasm-function[1367] 27.38; wasm-function[7350] 21.69; wasm-function[7351] 18.74 |
| 5 | 8541 | 0.68 | 1.49 | 20.82 | 20.93 | 0.23 | 0.00 | 20.53 | 0.00 | 0.00 | wasm-function[1367] 11.20; wasm-function[3034] 2.70; wasm-function[99865] 2.15 | 400.15 | 79.51 | 17.07 | 294.87 | 1.11 | 0.00 | wasm-function[1367] 26.91; wasm-function[7350] 23.69; wasm-function[7351] 16.82 |
| 6 | 10740 | 1.08 | 3.50 | 21.44 | 21.47 | 0.27 | 0.00 | 20.99 | 0.00 | 0.00 | wasm-function[1367] 9.81; wasm-function[3034] 4.86; wasm-function[99865] 2.06 | 399.99 | 88.37 | 4.74 | 300.51 | 0.44 | 0.00 | wasm-function[1367] 30.40; wasm-function[7350] 21.30; wasm-function[7351] 19.44 |
| 7 | 12819 | 0.62 | 0.17 | 18.30 | 18.20 | 0.15 | 0.00 | 17.58 | 0.00 | 0.00 | wasm-function[1367] 8.86; wasm-function[3034] 3.88; wasm-function[2137] 0.71 | 399.93 | 70.15 | 93.81 | 228.70 | 0.00 | 0.00 | wasm-function[1367] 37.94; wasm-function[3034] 14.60; wasm-function[7351] 12.65 |
| 8 | 14855 | 0.80 | 0.10 | 19.91 | 20.04 | 0.10 | 0.00 | 19.32 | 0.00 | 0.00 | wasm-function[1367] 10.65; wasm-function[3034] 2.59; wasm-function[99865] 1.90 | 400.02 | 78.74 | 9.53 | 305.24 | 0.19 | 0.00 | wasm-function[1367] 28.00; wasm-function[7350] 18.40; wasm-function[7351] 17.39 |
| 9 | 16885 | 0.56 | 0.48 | 22.69 | 22.64 | 0.20 | 0.00 | 22.11 | 0.00 | 0.00 | wasm-function[1367] 12.42; wasm-function[3034] 3.11; wasm-function[99865] 2.16 | 399.92 | 60.16 | 5.59 | 327.77 | 0.21 | 0.00 | wasm-function[1367] 24.53; wasm-function[7350] 19.13; wasm-function[7351] 18.36 |
| 10 | 19274 | 0.65 | 0.18 | 18.51 | 18.54 | 0.20 | 0.00 | 18.08 | 0.00 | 0.00 | wasm-function[1367] 8.04; wasm-function[3034] 4.26; wasm-function[99865] 2.15 | 400.05 | 62.08 | 8.79 | 320.14 | 0.47 | 0.00 | wasm-function[1367] 24.42; wasm-function[7350] 18.88; wasm-function[7351] 18.30 |
| 11 | 21300 | 0.86 | 2.83 | 18.40 | 18.40 | 0.47 | 0.00 | 17.73 | 0.00 | 0.00 | wasm-function[1367] 8.43; wasm-function[3034] 3.15; wasm-function[99865] 2.02 | 399.93 | 77.52 | 8.70 | 307.35 | 0.37 | 0.00 | wasm-function[1367] 33.98; wasm-function[7351] 19.19; wasm-function[7350] 15.23 |
| 12 | 23288 | 0.60 | 0.80 | 18.35 | 18.47 | 0.30 | 0.00 | 17.97 | 0.00 | 0.00 | wasm-function[1367] 9.14; wasm-function[3034] 3.87; wasm-function[2137] 1.10 | 399.95 | 65.25 | 64.11 | 261.44 | 0.72 | 0.00 | wasm-function[1367] 31.48; wasm-function[7350] 13.93; wasm-function[99865] 10.84 |

### E13-run2: sum over the 12 inputs, ms per input (the same columns)
| window | total | (program) | (idle) | wasm anon self | named self | fle_gil_* self | js | gc | top self functions, ms per input |
|---|---|---|---|---|---|---|---|---|---|
| C | 19.77 | 0.21 | 0.00 | 19.14 | 0.00 | 0.00 | 0.41 | 0.00 | wasm-function[1367] 9.64; wasm-function[3034] 3.21; wasm-function[99865] 1.57; wasm-function[2129] 0.59; wasm-function[2137] 0.55; wasm-function[2040] 0.19 |
| W (400 ms) | 399.97 | 71.90 | 25.70 | 294.48 | 0.36 | 0.00 | 6.97 | 0.55 | wasm-function[1367] 28.93; wasm-function[7350] 17.76; wasm-function[7351] 16.17; wasm-function[99865] 11.11; wasm-function[7158] 9.36; wasm-function[3034] 8.74 |

fle_gil_* self, ms per input: C: none | W: none
