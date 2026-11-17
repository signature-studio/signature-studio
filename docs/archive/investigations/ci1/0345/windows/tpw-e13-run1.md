## E13-run1: UI Worker tid 2959497; File clicks between the sync events: 12; profile samples 183170

Units: ms (not per input) inside each window. W = tPage +-200 ms (400 ms), C = tInject .. first frame.

| # | tPage (ms after sync-a) | A tEvent->tPage | B tPage->tInject | C span (tInject->first frame) | C: total | C: (program) | C: (idle) | C: wasm anon self | C: named self | C: fle_gil_* self | C: top self functions (ms) | W: total | W: (program) | W: (idle) | W: wasm anon self | W: named self | W: fle_gil_* self | W: top self functions (ms) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 67 | 2.26 | 0.34 | 27.12 | 27.28 | 1.69 | 0.00 | 24.51 | 0.25 | 0.00 | wasm-function[1367] 9.53; wasm-function[3034] 3.57; wasm-function[99865] 1.79 | 399.99 | 67.36 | 9.58 | 322.58 | 0.59 | 0.00 | wasm-function[1367] 42.48; wasm-function[7350] 15.57; wasm-function[7351] 14.62 |
| 2 | 2163 | 0.65 | 0.16 | 16.10 | 15.93 | 0.36 | 0.00 | 15.34 | 0.00 | 0.00 | wasm-function[1367] 9.11; wasm-function[3034] 1.88; wasm-function[2129] 0.53 | 400.26 | 62.62 | 57.39 | 271.94 | 0.71 | 0.00 | wasm-function[1367] 37.42; wasm-function[99865] 14.35; wasm-function[7350] 13.70 |
| 3 | 4180 | 0.66 | 1.22 | 17.68 | 17.91 | 0.64 | 0.00 | 16.38 | 0.00 | 0.00 | wasm-function[1367] 8.17; wasm-function[3034] 2.73; wasm-function[99865] 1.55 | 399.94 | 87.24 | 4.57 | 299.37 | 0.48 | 0.00 | wasm-function[1367] 24.58; wasm-function[7350] 19.82; wasm-function[7351] 16.27 |
| 4 | 6177 | 0.80 | 0.26 | 25.96 | 25.50 | 0.05 | 0.00 | 24.92 | 0.00 | 0.00 | wasm-function[1367] 14.16; wasm-function[3034] 4.34; wasm-function[99865] 1.79 | 399.94 | 72.18 | 3.33 | 316.83 | -0.08 | 0.00 | wasm-function[1367] 31.49; wasm-function[7350] 22.44; wasm-function[7351] 21.17 |
| 5 | 8858 | 0.76 | 0.09 | 16.67 | 16.77 | 0.41 | 0.00 | 16.17 | 0.00 | 0.00 | wasm-function[1367] 8.55; wasm-function[3034] 3.17; wasm-function[2137] 0.89 | 399.95 | 98.01 | 6.57 | 290.61 | 0.07 | 0.00 | wasm-function[1367] 25.21; wasm-function[7350] 20.99; wasm-function[7351] 16.71 |
| 6 | 10978 | 0.65 | 0.23 | 16.83 | 16.74 | 0.51 | 0.00 | 16.04 | 0.00 | 0.00 | wasm-function[1367] 9.71; wasm-function[3034] 1.87; wasm-function[2040] 0.47 | 399.99 | 83.38 | 68.63 | 242.09 | -0.04 | 0.00 | wasm-function[1367] 34.07; wasm-function[7350] 18.25; wasm-function[7351] 14.20 |
| 7 | 12951 | 0.65 | 0.58 | 18.80 | 18.85 | 0.50 | 0.00 | 18.35 | 0.00 | 0.00 | wasm-function[1367] 9.67; wasm-function[3034] 3.19; wasm-function[2040] 0.63 | 400.08 | 93.57 | 21.71 | 279.73 | 0.12 | 0.00 | wasm-function[1367] 31.95; wasm-function[7350] 18.84; wasm-function[7351] 16.78 |
| 8 | 15171 | 1.42 | 5.91 | 27.25 | 27.36 | 0.52 | 0.00 | 26.66 | 0.00 | 0.00 | wasm-function[1367] 11.78; wasm-function[3034] 8.98; wasm-function[99865] 1.04 | 402.00 | 87.71 | 12.66 | 294.03 | 0.34 | 0.00 | wasm-function[1367] 43.47; wasm-function[7350] 20.25; wasm-function[3034] 19.57 |
| 9 | 17350 | 0.56 | 0.73 | 19.12 | 18.85 | 0.18 | 0.00 | 18.67 | 0.00 | 0.00 | wasm-function[1367] 11.38; wasm-function[3034] 2.47; wasm-function[99865] 2.17 | 400.12 | 67.07 | 7.71 | 317.75 | 0.36 | 0.00 | wasm-function[1367] 30.22; wasm-function[7351] 21.21; wasm-function[7350] 20.97 |
| 10 | 19562 | 0.58 | 0.36 | 22.23 | 22.29 | 0.49 | 0.00 | 21.41 | 0.20 | 0.00 | wasm-function[1367] 13.07; wasm-function[3034] 3.14; wasm-function[99865] 1.70 | 400.02 | 68.44 | 39.58 | 286.42 | -0.16 | 0.00 | wasm-function[1367] 25.01; wasm-function[7350] 21.57; wasm-function[7351] 17.40 |
| 11 | 21625 | 0.58 | 3.86 | 17.20 | 17.32 | 0.45 | 0.00 | 16.87 | 0.00 | 0.00 | wasm-function[1367] 8.34; wasm-function[3034] 3.76; wasm-function[99865] 1.60 | 400.00 | 82.97 | 19.02 | 293.45 | 0.33 | 0.00 | wasm-function[1367] 24.64; wasm-function[7350] 17.31; wasm-function[7351] 14.92 |
| 12 | 23702 | 0.92 | 0.27 | 21.99 | 22.55 | 0.78 | 0.00 | 21.59 | 0.00 | 0.00 | wasm-function[1367] 11.06; wasm-function[3034] 4.67; wasm-function[2137] 0.68 | 399.96 | 74.75 | 11.68 | 307.14 | 0.31 | 0.00 | wasm-function[1367] 28.85; wasm-function[7350] 20.23; wasm-function[99865] 16.57 |

### E13-run1: sum over the 12 inputs, ms per input (the same columns)
| window | total | (program) | (idle) | wasm anon self | named self | fle_gil_* self | js | gc | top self functions, ms per input |
|---|---|---|---|---|---|---|---|---|---|
| C | 20.61 | 0.55 | 0.00 | 19.74 | 0.04 | 0.00 | 0.25 | 0.04 | wasm-function[1367] 10.38; wasm-function[3034] 3.65; wasm-function[99865] 1.14; wasm-function[2137] 0.36; wasm-function[2040] 0.28; wasm-function[2517] 0.26 |
| W (400 ms) | 400.19 | 78.77 | 21.87 | 293.49 | 0.25 | 0.00 | 5.59 | 0.21 | wasm-function[1367] 31.62; wasm-function[7350] 19.16; wasm-function[7351] 16.26; wasm-function[99865] 11.27; wasm-function[3034] 10.35; wasm-function[7158] 9.58 |

fle_gil_* self, ms per input: C: none | W: none
