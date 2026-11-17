### P3 decider, chrome: old (previous) vs new, windows of 10 s from the play mark; counted = start load ≤8 and I/O pressure avg10 = 0

| run | bundle | counted | other Chrome browsers at the play mark / at +60 s / max | waited for the quiet window (s) | load start → end | I/O avg10 start → end | Worker CPU s (60 s) = CPU/wall x 60 | main CPU s | underrun quanta 0-60 s | episodes 0-60 s | engine gap frames 0-60 s | **underrun 30-60 s** | write rate mean 10-60 s | consumed rate mean 10-60 s | UI idles/s (10-60 s) | presents/s | lagCuts/s | virtual s per real s | stopped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | prev | yes | 0 / 0 / 0 | 0 | 1.0 → 1.4 | 0.00 → 0.00 | 49.4 | 0.0 | 2 | 1 | 170 | **0** | 0.988 | 0.996 | 403.1 | 18.5 | 16.9 | 0.791 | False |
| 2 | prev | yes | 0 / 0 / 0 | 0 | 1.9 → 2.1 | 0.00 → 0.00 | 49.6 | 0.0 | 2 | 1 | 170 | **0** | 0.983 | 1.000 | 397.5 | 18.2 | 17.9 | 0.783 | False |
| 3 | prev | yes | 0 / 0 / 0 | 0 | 1.9 → 1.7 | 0.00 → 0.00 | 49.6 | 0.0 | 2 | 1 | 170 | **0** | 0.981 | 0.994 | 458.5 | 18.3 | 15.3 | 0.812 | False |
| 4 | prev | yes | 0 / 0 / 0 | 0 | 1.7 → 3.4 | 0.00 → 0.00 | 49.5 | 0.0 | 2 | 1 | 170 | **0** | 0.985 | 0.997 | 450.4 | 18.5 | 16.4 | 0.808 | False |
| 5 | prev | yes | 0 / 0 / 0 | 0 | 3.4 → 2.5 | 0.00 → 0.00 | 49.3 | 0.0 | 2 | 1 | 170 | **0** | 0.985 | 0.997 | 470.2 | 18.7 | 15.9 | 0.817 | False |
| 6 | prev | yes | 0 / 0 / 0 | 0 | 2.4 → 2.6 | 0.00 → 0.00 | 49.5 | 0.0 | 2 | 1 | 170 | **0** | 0.990 | 0.998 | 430.3 | 18.6 | 16.8 | 0.803 | False |
| 1 | new | yes | 0 / 0 / 0 | 0 | 2.3 → 1.7 | 0.00 → 0.04 | 49.3 | 0.0 | 4 | 2 | 340 | **0** | 0.986 | 0.994 | 468.1 | 19.0 | 16.4 | 0.819 | False |
| 2 | new | yes | 0 / 0 / 0 | 15 | 1.4 → 1.9 | 0.00 → 0.00 | 49.4 | 0.0 | 2 | 1 | 170 | **0** | 0.994 | 0.997 | 440.3 | 18.6 | 16.5 | 0.808 | False |
| 3 | new | yes | 0 / 0 / 0 | 0 | 1.7 → 1.1 | 0.00 → 0.00 | 49.4 | 0.0 | 0 | 0 | 0 | **0** | 0.992 | 0.995 | 454.8 | 18.5 | 15.8 | 0.814 | False |
| 4 | new | yes | 0 / 0 / 0 | 0 | 1.1 → 1.9 | 0.00 → 0.00 | 49.2 | 0.0 | 0 | 0 | 0 | **0** | 0.992 | 0.995 | 439.0 | 18.9 | 16.6 | 0.806 | False |
| 5 | new | yes | 0 / 0 / 0 | 0 | 2.5 → 1.6 | 0.00 → 0.00 | 49.4 | 0.0 | 11 | 2 | 1236 | **0** | 0.992 | 0.994 | 430.1 | 18.7 | 16.9 | 0.805 | False |
| 6 | new | yes | 0 / 0 / 0 | 0 | 1.5 → 2.4 | 0.00 → 0.00 | 49.5 | 0.0 | 3 | 1 | 298 | **0** | 0.996 | 0.996 | 496.8 | 18.7 | 14.4 | 0.834 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 6 | 2 | 1 | 170 | 0.98 | 1.00 | 1308.1 | 16.2 | 3.4 | 0.86 |
| 0-10 s | new | 6 | 2 | 1 | 234 | 0.98 | 1.00 | 1378.3 | 16.3 | 0.6 | 0.90 |
| 10-20 s | prev | 6 | 0 | 0 | 0 | 0.99 | 1.00 | 553.3 | 20.3 | 15.9 | 0.83 |
| 10-20 s | new | 6 | 0 | 0 | 0 | 0.99 | 1.00 | 581.1 | 20.2 | 15.0 | 0.85 |
| 20-30 s | prev | 6 | 0 | 0 | 0 | 1.00 | 1.00 | 714.7 | 21.3 | 9.3 | 0.90 |
| 20-30 s | new | 6 | 0 | 0 | 0 | 1.00 | 1.00 | 721.0 | 21.8 | 7.9 | 0.91 |
| 30-40 s | prev | 6 | 0 | 0 | 0 | 0.99 | 1.00 | 458.4 | 20.7 | 21.8 | 0.74 |
| 30-40 s | new | 6 | 0 | 0 | 0 | 0.99 | 1.00 | 442.1 | 20.9 | 23.4 | 0.73 |
| 40-50 s | prev | 6 | 0 | 0 | 0 | 0.99 | 1.00 | 307.2 | 17.0 | 18.8 | 0.76 |
| 40-50 s | new | 6 | 0 | 0 | 0 | 1.00 | 1.00 | 354.6 | 17.2 | 17.1 | 0.79 |
| 50-60 s | prev | 6 | 0 | 0 | 0 | 0.95 | 0.98 | 177.6 | 13.3 | 16.0 | 0.79 |
| 50-60 s | new | 6 | 0 | 0 | 0 | 0.97 | 0.97 | 137.1 | 13.7 | 17.1 | 0.78 |

Music-matched windows (the same music in both bundles): the music position is the virtual clock since the play mark (gpu1 matched-music.py); the bins are complete 10 s bins of music that each run reached within its 60 s of wall time. wall = the wall time the bin took; distinct frames = Unlock coverage per wall second (1.0 = real time); Unlocks / wall s; underrun quanta and episodes of the page and the engine gap frames over the wall interval of the bin (the 10 s snapshots and the 1 Hz engine reports, linearly interpolated). Median over the counted runs.

| music bin | bundle | n | wall time of the bin (s) | distinct frames per wall s | Unlocks per wall s | underrun quanta | episodes | engine gap frames |
|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 6 | 12.2 | 0.981 | 10.56 | 2 | 1 | 170 |
| 0-10 s | new | 6 | 11.3 | 0.978 | 10.53 | 2 | 1 | 234 |
| 10-20 s | prev | 6 | 11.2 | 0.976 | 10.73 | 0 | 0 | 0 |
| 10-20 s | new | 6 | 11.5 | 0.973 | 10.70 | 0 | 0 | 0 |
| 20-30 s | prev | 6 | 12.5 | 0.948 | 10.79 | 0 | 0 | 0 |
| 20-30 s | new | 6 | 11.9 | 0.966 | 10.80 | 0 | 0 | 0 |
| 30-40 s | prev | 6 | 13.5 | 0.917 | 10.70 | 0 | 0 | 0 |
| 30-40 s | new | 6 | 13.0 | 0.935 | 10.73 | 0 | 0 | 0 |
| 40-50 s | new | 1 | 12.6 | 0.802 | 10.61 | 0 | 0 | 0 |

Old vs new at the same music position (counted runs; exact permutation test; only bins both bundles reached):

| music bin | measure | old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|
| 0-10 s | wall time of the bin (s) | 12.158 (6) | 11.278 (6) | -0.880 | 0.041 |
| 0-10 s | distinct frames per wall s | 0.981 (6) | 0.978 (6) | -0.002 | 0.613 |
| 0-10 s | underrun quanta | 2.000 (6) | 2.500 (6) | +0.500 | 0.682 |
| 0-10 s | episodes | 1.000 (6) | 1.000 (6) | +0.000 | 1.000 |
| 0-10 s | engine gap frames | 170.000 (6) | 234.000 (6) | +64.000 | 0.682 |
| 10-20 s | wall time of the bin (s) | 11.206 (6) | 11.508 (6) | +0.302 | 0.277 |
| 10-20 s | distinct frames per wall s | 0.976 (6) | 0.973 (6) | -0.003 | 0.883 |
| 10-20 s | underrun quanta | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 10-20 s | episodes | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 10-20 s | engine gap frames | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 20-30 s | wall time of the bin (s) | 12.510 (6) | 11.869 (6) | -0.641 | 0.076 |
| 20-30 s | distinct frames per wall s | 0.948 (6) | 0.966 (6) | +0.018 | 0.032 |
| 20-30 s | underrun quanta | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 20-30 s | episodes | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 20-30 s | engine gap frames | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 30-40 s | wall time of the bin (s) | 13.457 (6) | 13.010 (6) | -0.447 | 0.076 |
| 30-40 s | distinct frames per wall s | 0.917 (6) | 0.935 (6) | +0.019 | 0.284 |
| 30-40 s | underrun quanta | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 30-40 s | episodes | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 30-40 s | engine gap frames | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 2.000 (6) | 2.500 (6) | +0.500 | 0.682 | 2.000 (6) | 2.500 (6) | +0.500 | 0.682 |
| underrun quanta 30-60 s (heavy interval) | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| episodes 0-60 s | 1.000 (6) | 1.000 (6) | +0.000 | 1.000 | 1.000 (6) | 1.000 (6) | +0.000 | 1.000 |
| engine gap frames 0-60 s | 170.000 (6) | 234.000 (6) | +64.000 | 0.682 | 170.000 (6) | 234.000 (6) | +64.000 | 0.682 |
| write rate mean 10-60 s | 0.985 (6) | 0.992 (6) | +0.007 | 0.013 | 0.985 (6) | 0.992 (6) | +0.007 | 0.013 |
| Worker CPU s (60 s) | 49.530 (6) | 49.415 (6) | -0.115 | 0.043 | 49.530 (6) | 49.415 (6) | -0.115 | 0.043 |
| UI idles/s (10-60 s) | 440.363 (6) | 447.553 (6) | +7.190 | 0.240 | 440.363 (6) | 447.553 (6) | +7.190 | 0.240 |
| presents/s (10-60 s) | 18.483 (6) | 18.703 (6) | +0.220 | 0.032 | 18.483 (6) | 18.703 (6) | +0.220 | 0.032 |
| lagCuts/s (10-60 s) | 16.582 (6) | 16.434 (6) | -0.148 | 0.446 | 16.582 (6) | 16.434 (6) | -0.148 | 0.446 |
| virtual s per real s (10-60 s) | 0.806 (6) | 0.811 (6) | +0.005 | 0.123 | 0.806 (6) | 0.811 (6) | +0.005 | 0.123 |

