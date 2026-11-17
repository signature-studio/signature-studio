### P3 decider, chrome: old (previous) vs new, windows of 10 s from the play mark; counted = start load ≤8 and I/O pressure avg10 = 0

| run | bundle | counted | other Chrome browsers at the play mark / at +60 s / max | waited for the quiet window (s) | load start → end | I/O avg10 start → end | Worker CPU s (60 s) = CPU/wall x 60 | main CPU s | underrun quanta 0-60 s | episodes 0-60 s | engine gap frames 0-60 s | **underrun 30-60 s** | write rate mean 10-60 s | consumed rate mean 10-60 s | UI idles/s (10-60 s) | presents/s | lagCuts/s | virtual s per real s | stopped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | prev | yes | 1 / 1 / 1 | 0 | 7.4 → 2.8 | 0.00 → 0.00 | 39.6 | 0.0 | 1514 | 135 | 182175 | **1152** | 0.411 | 0.996 | 200.6 | 7.7 | 0.0 | 1.035 | False |
| 2 | prev | yes | 1 / 1 / 1 | 95 | 3.4 → 4.2 | 0.00 → 0.00 | 41.3 | 0.0 | 1167 | 112 | 137289 | **912** | 0.398 | 0.993 | 196.4 | 7.2 | 0.0 | 1.016 | False |
| 3 | prev | yes | 1 / 1 / 1 | 0 | 3.9 → 3.4 | 0.00 → 0.00 | 40.7 | 0.0 | 1249 | 121 | 149503 | **1007** | 0.403 | 0.999 | 186.6 | 7.5 | 0.0 | 1.024 | False |
| 4 | prev | yes | 1 / 1 / 1 | 65 | 7.6 → 8.2 | 0.00 → 0.12 | 43.4 | 0.0 | 1025 | 103 | 122298 | **832** | 0.409 | 0.997 | 151.4 | 7.0 | 0.0 | 1.009 | False |
| 5 | prev | yes | 1 / 1 / 1 | 115 | 8.0 → 7.4 | 0.00 → 0.00 | 43.7 | 0.0 | 1075 | 104 | 125873 | **941** | 0.401 | 1.000 | 177.8 | 7.0 | 0.0 | 1.019 | False |
| 6 | prev | yes | 1 / 1 / 1 | 25 | 7.8 → 9.1 | 0.00 → 0.03 | 43.7 | 0.0 | 975 | 88 | 117211 | **844** | 0.396 | 1.000 | 200.4 | 7.2 | 0.0 | 1.009 | False |
| 1 | new | yes | 1 / 1 / 1 | 0 | 2.6 → 3.3 | 0.00 → 0.00 | 51.2 | 0.0 | 4 | 2 | 340 | **0** | 0.793 | 0.995 | 333.7 | 13.1 | 15.1 | 0.615 | False |
| 2 | new | yes | 1 / 1 / 1 | 0 | 3.2 → 2.6 | 0.00 → 5.86 | 51.4 | 0.0 | 0 | 0 | 0 | **0** | 0.830 | 0.993 | 269.1 | 13.2 | 15.7 | 0.607 | False |
| 3 | new | yes | 1 / 1 / 1 | 0 | 3.5 → 8.9 | 0.00 → 0.00 | 50.2 | 0.0 | 18 | 5 | 1870 | **8** | 0.762 | 0.995 | 292.7 | 12.0 | 15.0 | 0.569 | False |
| 4 | new | yes | 1 / 1 / 1 | 15 | 7.9 → 11.9 | 0.00 → 0.00 | 50.2 | 0.0 | 36 | 5 | 5935 | **0** | 0.765 | 0.994 | 321.2 | 12.4 | 14.9 | 0.574 | False |
| 5 | new | yes | 1 / 1 / 1 | 0 | 6.8 → 8.2 | 0.00 → 0.00 | 50.0 | 0.0 | 12 | 4 | 1189 | **4** | 0.749 | 0.998 | 326.3 | 12.6 | 15.2 | 0.570 | False |
| 6 | new | yes | 1 / 1 / 1 | 540 | 8.0 → 8.3 | 0.00 → 0.02 | 50.1 | 0.0 | 44 | 9 | 4854 | **17** | 0.682 | 0.997 | 380.0 | 11.3 | 14.7 | 0.553 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 6 | 14 | 3 | 1460 | 0.72 | 1.00 | 1079.2 | 10.0 | 0.0 | 1.00 |
| 0-10 s | new | 6 | 9 | 1 | 1016 | 0.96 | 1.00 | 1057.4 | 12.4 | 7.1 | 0.73 |
| 10-20 s | prev | 6 | 12 | 4 | 768 | 0.50 | 1.00 | 259.8 | 8.0 | 0.0 | 1.00 |
| 10-20 s | new | 6 | 0 | 0 | 6 | 0.89 | 1.00 | 252.5 | 12.6 | 16.4 | 0.61 |
| 20-30 s | prev | 6 | 188 | 18 | 18884 | 0.41 | 1.00 | 189.6 | 7.2 | 0.0 | 1.01 |
| 20-30 s | new | 6 | 0 | 0 | 1 | 0.84 | 1.00 | 385.7 | 13.4 | 14.4 | 0.69 |
| 30-40 s | prev | 6 | 379 | 34 | 47606 | 0.33 | 1.00 | 136.3 | 6.6 | 0.0 | 1.02 |
| 30-40 s | new | 6 | 0 | 0 | 85 | 0.76 | 1.00 | 311.4 | 12.5 | 16.6 | 0.53 |
| 40-50 s | prev | 6 | 474 | 34 | 57954 | 0.33 | 1.00 | 88.1 | 5.0 | 0.0 | 1.04 |
| 40-50 s | new | 6 | 0 | 0 | 7 | 0.70 | 1.00 | 291.6 | 12.1 | 15.7 | 0.50 |
| 50-60 s | prev | 6 | 109 | 21 | 14364 | 0.43 | 0.99 | 224.6 | 9.8 | 0.0 | 1.00 |
| 50-60 s | new | 6 | 0 | 0 | 8 | 0.66 | 0.97 | 352.2 | 11.6 | 13.2 | 0.57 |

Music-matched windows (the same music in both bundles): the music position is the virtual clock since the play mark (gpu1 matched-music.py); the bins are complete 10 s bins of music that each run reached within its 60 s of wall time. wall = the wall time the bin took; distinct frames = Unlock coverage per wall second (1.0 = real time); Unlocks / wall s; underrun quanta and episodes of the page and the engine gap frames over the wall interval of the bin (the 10 s snapshots and the 1 Hz engine reports, linearly interpolated). Median over the counted runs.

| music bin | bundle | n | wall time of the bin (s) | distinct frames per wall s | Unlocks per wall s | underrun quanta | episodes | engine gap frames |
|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 6 | 10.0 | 0.720 | 7.75 | 14 | 3 | 1537 |
| 0-10 s | new | 6 | 15.2 | 0.858 | 10.31 | 9 | 1 | 1023 |
| 10-20 s | prev | 6 | 10.0 | 0.495 | 5.33 | 12 | 4 | 1209 |
| 10-20 s | new | 6 | 14.5 | 0.797 | 9.01 | 1 | 0 | 119 |
| 20-30 s | prev | 6 | 9.9 | 0.411 | 4.42 | 186 | 18 | 22258 |
| 20-30 s | new | 6 | 19.1 | 0.729 | 8.01 | 1 | 1 | 306 |
| 30-40 s | prev | 6 | 9.8 | 0.332 | 3.57 | 372 | 33 | 44622 |
| 40-50 s | prev | 6 | 9.6 | 0.334 | 3.60 | 444 | 33 | 55361 |
| 50-60 s | prev | 6 | 10.0 | 0.442 | 4.80 | 147 | 23 | 15384 |

Old vs new at the same music position (counted runs; exact permutation test; only bins both bundles reached):

| music bin | measure | old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|
| 0-10 s | wall time of the bin (s) | 9.996 (6) | 15.237 (6) | +5.241 | 0.002 |
| 0-10 s | distinct frames per wall s | 0.720 (6) | 0.858 (6) | +0.138 | 0.002 |
| 0-10 s | underrun quanta | 14.383 (6) | 8.984 (6) | -5.399 | 0.771 |
| 0-10 s | episodes | 2.977 (6) | 1.498 (6) | -1.480 | 0.260 |
| 0-10 s | engine gap frames | 1536.816 (6) | 1023.000 (6) | -513.816 | 0.838 |
| 10-20 s | wall time of the bin (s) | 9.988 (6) | 14.508 (6) | +4.520 | 0.002 |
| 10-20 s | distinct frames per wall s | 0.495 (6) | 0.797 (6) | +0.301 | 0.002 |
| 10-20 s | underrun quanta | 11.966 (6) | 0.520 (6) | -11.447 | 0.004 |
| 10-20 s | episodes | 3.982 (6) | 0.260 (6) | -3.722 | 0.011 |
| 10-20 s | engine gap frames | 1209.396 (6) | 118.575 (6) | -1090.821 | 0.041 |
| 20-30 s | wall time of the bin (s) | 9.938 (6) | 19.080 (6) | +9.142 | 0.002 |
| 20-30 s | distinct frames per wall s | 0.411 (6) | 0.729 (6) | +0.318 | 0.002 |
| 20-30 s | underrun quanta | 186.217 (6) | 1.045 (6) | -185.172 | 0.002 |
| 20-30 s | episodes | 18.256 (6) | 0.523 (6) | -17.734 | 0.002 |
| 20-30 s | engine gap frames | 22257.545 (6) | 306.425 (6) | -21951.119 | 0.002 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 1121.102 (6) | 14.966 (6) | -1106.136 | 0.002 | 1121.102 (6) | 14.966 (6) | -1106.136 | 0.002 |
| underrun quanta 30-60 s (heavy interval) | 926.679 (6) | 1.998 (6) | -924.681 | 0.002 | 926.679 (6) | 1.998 (6) | -924.681 | 0.002 |
| episodes 0-60 s | 107.777 (6) | 4.489 (6) | -103.288 | 0.002 | 107.777 (6) | 4.489 (6) | -103.288 | 0.002 |
| engine gap frames 0-60 s | 131581.240 (6) | 1529.635 (6) | -130051.606 | 0.002 | 131581.240 (6) | 1529.635 (6) | -130051.606 | 0.002 |
| write rate mean 10-60 s | 0.402 (6) | 0.763 (6) | +0.361 | 0.002 | 0.402 (6) | 0.763 (6) | +0.361 | 0.002 |
| Worker CPU s (60 s) | 42.335 (6) | 50.205 (6) | +7.870 | 0.002 | 42.335 (6) | 50.205 (6) | +7.870 | 0.002 |
| UI idles/s (10-60 s) | 191.505 (6) | 323.760 (6) | +132.255 | 0.002 | 191.505 (6) | 323.760 (6) | +132.255 | 0.002 |
| presents/s (10-60 s) | 7.194 (6) | 12.485 (6) | +5.291 | 0.002 | 7.194 (6) | 12.485 (6) | +5.291 | 0.002 |
| lagCuts/s (10-60 s) | 0.000 (6) | 15.036 (6) | +15.036 | 0.002 | 0.000 (6) | 15.036 (6) | +15.036 | 0.002 |
| virtual s per real s (10-60 s) | 1.017 (6) | 0.572 (6) | -0.445 | 0.002 | 1.017 (6) | 0.572 (6) | -0.445 | 0.002 |

