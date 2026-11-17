## RT-H: UI Worker tid 3303702; File clicks between the sync events: 12; profile samples 64608

Units: ms (not per input) inside each window. W = tPage +-200 ms (400 ms), C = tInject .. first frame.

| # | tPage (ms after sync-a) | A tEvent->tPage | B tPage->tInject | C span (tInject->first frame) | C: total | C: (program) | C: (idle) | C: wasm anon self | C: named self | C: fle_gil_* self | C: top self functions (ms) | W: total | W: (program) | W: (idle) | W: wasm anon self | W: named self | W: fle_gil_* self | W: top self functions (ms) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 83 | 1.53 | 11.42 | 67.74 | 67.90 | 13.98 | 0.00 | 51.79 | 0.00 | 1.75 | wasm-function[1368] 11.92; wasm-function[3035] 4.98; fle_desk_compose 2.94 | 399.90 | 81.43 | 89.49 | 207.48 | 0.00 | 5.76 | wasm-function[1368] 49.07; fle_desk_compose 18.69; wasm-function[3035] 12.30 |
| 2 | 2504 | 0.72 | 6.51 | 38.59 | 38.38 | 6.79 | 0.77 | 25.60 | 0.00 | 4.62 | wasm-function[1368] 14.72; fle_gil_acquire_slow 3.25; wasm-function[3035] 2.26 | 400.71 | 108.09 | 48.40 | 219.44 | 0.00 | 13.15 | wasm-function[1368] 50.11; fle_desk_compose 38.67; wasm-function[3035] 14.88 |
| 3 | 4739 | 0.73 | 0.69 | 32.32 | 31.89 | 6.44 | 1.22 | 22.49 | 0.00 | 0.75 | wasm-function[1368] 7.74; wasm-function[3035] 7.35; fle_desk_compose 2.07 | 400.46 | 103.77 | 55.40 | 213.27 | 1.02 | 11.74 | wasm-function[1368] 43.62; fle_desk_compose 35.70; wasm-function[3035] 21.15 |
| 4 | 7047 | 0.76 | 2.81 | 36.01 | 36.08 | 7.28 | 0.00 | 26.85 | 0.00 | 1.95 | wasm-function[1368] 11.38; wasm-function[3035] 3.59; f_5191e0 1.21 | 400.38 | 80.66 | 137.40 | 163.17 | 0.00 | 11.23 | wasm-function[1368] 47.57; fle_desk_compose 17.49; wasm-function[3035] 10.23 |
| 5 | 9193 | 0.67 | 0.23 | 41.85 | 41.66 | 11.76 | 0.00 | 28.80 | 0.00 | 1.10 | wasm-function[1368] 14.03; wasm-function[3035] 5.91; fle_desk_compose 1.49 | 400.30 | 99.37 | 105.33 | 175.90 | 0.00 | 13.67 | wasm-function[1368] 48.03; fle_desk_compose 28.67; wasm-function[3035] 14.79 |
| 6 | 11350 | 0.74 | 4.78 | 40.70 | 40.58 | 9.70 | 0.00 | 29.03 | 0.00 | 1.85 | wasm-function[1368] 14.92; wasm-function[3035] 2.84; fle_desk_compose 2.13 | 399.79 | 90.73 | 124.42 | 167.17 | 0.70 | 8.23 | wasm-function[1368] 45.56; fle_desk_compose 26.56; wasm-function[3035] 12.82 |
| 7 | 13546 | 0.72 | 9.48 | 37.18 | 36.69 | 9.31 | 0.00 | 27.15 | 0.00 | 0.23 | wasm-function[1368] 11.15; wasm-function[3035] 2.45; fle_desk_compose 2.17 | 399.86 | 87.28 | 126.94 | 168.71 | 0.00 | 8.19 | wasm-function[1368] 44.13; fle_desk_compose 21.92; wasm-function[3035] 10.17 |
| 8 | 15697 | 0.70 | 5.41 | 41.00 | 40.57 | 9.41 | 0.00 | 29.49 | 0.00 | 1.66 | wasm-function[1368] 13.48; wasm-function[3035] 3.11; fle_desk_compose 1.56 | 400.43 | 79.18 | 125.83 | 182.50 | 0.00 | 6.51 | wasm-function[1368] 47.15; fle_desk_compose 22.06; wasm-function[3035] 9.88 |
| 9 | 17812 | 3.85 | 9.75 | 36.56 | 36.00 | 6.19 | 0.00 | 29.37 | 0.00 | 0.45 | wasm-function[1368] 10.70; wasm-function[3035] 4.85; fle_desk_compose 2.25 | 399.83 | 75.40 | 114.56 | 193.09 | 0.00 | 5.45 | wasm-function[1368] 39.01; fle_desk_compose 31.07; wasm-function[3035] 16.22 |
| 10 | 19961 | 0.64 | 0.31 | 42.61 | 42.94 | 6.08 | 0.00 | 35.70 | 0.00 | 1.16 | wasm-function[1368] 13.33; wasm-function[3035] 2.73; wasm-function[1746] 1.67 | 400.39 | 81.40 | 111.68 | 194.60 | 0.44 | 4.41 | wasm-function[1368] 42.17; fle_desk_compose 36.31; wasm-function[3035] 9.31 |
| 11 | 22082 | 0.60 | 6.02 | 39.24 | 39.16 | 7.51 | 0.00 | 27.78 | 0.00 | 2.51 | wasm-function[1368] 13.57; wasm-function[3035] 4.39; fle_desk_compose 2.05 | 400.02 | 86.82 | 105.22 | 190.82 | 0.00 | 5.78 | wasm-function[1368] 44.98; fle_desk_compose 31.36; wasm-function[3035] 14.98 |
| 12 | 24222 | 0.73 | 0.26 | 25.92 | 25.90 | 3.49 | 0.00 | 22.14 | 0.00 | 0.26 | wasm-function[1368] 12.81; wasm-function[3035] 2.84; fle_desk_compose 0.88 | 400.00 | 90.79 | 132.01 | 166.25 | 0.00 | 2.97 | wasm-function[1368] 43.23; fle_desk_compose 29.46; wasm-function[3035] 13.32 |

### RT-H: sum over the 12 inputs, ms per input (the same columns)
| window | total | (program) | (idle) | wasm anon self | named self | fle_gil_* self | js | gc | top self functions, ms per input |
|---|---|---|---|---|---|---|---|---|---|
| C | 39.81 | 8.16 | 0.17 | 29.68 | 0.00 | 1.52 | 0.28 | 0.00 | wasm-function[1368] 12.48; wasm-function[3035] 3.94; fle_desk_compose 1.83; fle_gil_wake 0.82; fle_gil_acquire_slow 0.70; wasm-function[2138] 0.68 |
| W (400 ms) | 400.17 | 88.74 | 106.39 | 186.87 | 0.18 | 8.09 | 8.90 | 1.00 | wasm-function[1368] 45.39; fle_desk_compose 28.16; wasm-function[3035] 13.34; wasm-function[1746] 6.05; fle_gil_wake 5.32; wasm-function[973] 4.76 |

fle_gil_* self, ms per input: C: fle_gil_wake 0.82, fle_gil_acquire_slow 0.70 | W: fle_gil_wake 5.32, fle_gil_acquire_slow 2.77
