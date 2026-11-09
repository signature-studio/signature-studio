### P3 decider, chrome: old (previous) vs new, windows of 10 s from the play mark; counted = start load ≤8 and I/O pressure avg10 = 0

| run | bundle | counted | other Chrome browsers at the play mark / at +60 s / max | waited for the quiet window (s) | load start → end | I/O avg10 start → end | Worker CPU s (60 s) = CPU/wall x 60 | main CPU s | underrun quanta 0-60 s | episodes 0-60 s | engine gap frames 0-60 s | **underrun 30-60 s** | write rate mean 10-60 s | consumed rate mean 10-60 s | UI idles/s (10-60 s) | presents/s | lagCuts/s | virtual s per real s | stopped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | prev | yes | 1 / 1 / 1 | 105 | 7.8 → 8.6 | 0.00 → 0.00 | 49.7 | 0.0 | 18 | 7 | 1702 | **5** | 0.875 | 0.996 | 260.4 | 13.2 | 17.4 | 0.637 | False |
| 2 | prev | yes | 1 / 0 / 1 | 0 | 5.5 → 4.7 | 0.00 → 0.00 | 50.4 | 0.0 | 4 | 2 | 340 | **0** | 0.959 | 0.994 | 243.6 | 14.9 | 18.7 | 0.689 | False |
| 3 | prev | yes | 0 / 0 / 0 | 0 | 4.7 → 3.8 | 0.00 → 0.00 | 50.7 | 0.0 | 14 | 3 | 1534 | **0** | 0.951 | 1.000 | 253.9 | 14.9 | 18.7 | 0.684 | False |
| 4 | prev | yes | 0 / 0 / 0 | 0 | 4.9 → 5.1 | 0.00 → 0.00 | 50.6 | 0.0 | 8 | 2 | 852 | **0** | 0.951 | 0.998 | 255.7 | 15.2 | 18.8 | 0.692 | False |
| 5 | prev | yes | 0 / 0 / 0 | 0 | 5.1 → 5.6 | 0.00 → 0.00 | 50.4 | 0.0 | 37 | 4 | 4392 | **26** | 0.923 | 1.000 | 247.3 | 14.8 | 19.2 | 0.664 | False |
| 6 | prev | yes | 0 / 0 / 0 | 0 | 7.4 → 6.4 | 0.00 → 0.00 | 50.5 | 0.0 | 11 | 2 | 1236 | **0** | 0.933 | 1.000 | 257.8 | 14.4 | 18.3 | 0.674 | False |
| 1 | new | yes | 1 / 1 / 1 | 30 | 7.8 → 8.3 | 0.00 → 0.00 | 50.1 | 0.0 | 12 | 4 | 1192 | **6** | 0.929 | 1.000 | 270.3 | 14.5 | 18.6 | 0.676 | False |
| 2 | new | NO (not counted: load / I/O) | 1 / 1 / 1 | 5 | 7.9 → 5.5 | 0.18 → 0.00 | 49.6 | 0.0 | 17 | 3 | 1918 | **0** | 0.973 | 0.998 | 309.3 | 15.9 | 19.0 | 0.734 | False |
| 3 | new | yes | 0 / 0 / 0 | 0 | 3.8 → 4.4 | 0.00 → 0.00 | 50.1 | 0.0 | 7 | 3 | 638 | **0** | 0.964 | 1.000 | 299.4 | 16.0 | 18.6 | 0.719 | False |
| 4 | new | yes | 0 / 0 / 0 | 0 | 4.4 → 4.9 | 0.00 → 0.00 | 50.0 | 0.0 | 4 | 2 | 340 | **0** | 0.964 | 0.998 | 289.4 | 16.1 | 19.9 | 0.716 | False |
| 5 | new | yes | 0 / 0 / 0 | 0 | 5.6 → 8.0 | 0.00 → 0.00 | 50.0 | 0.0 | 10 | 3 | 1022 | **0** | 0.955 | 0.996 | 297.7 | 15.6 | 17.8 | 0.708 | False |
| 6 | new | yes | 0 / 0 / 0 | 5 | 7.8 → 7.4 | 0.00 → 0.00 | 50.0 | 0.0 | 9 | 3 | 894 | **0** | 0.977 | 1.000 | 297.1 | 15.9 | 18.6 | 0.726 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 6 | 10 | 2 | 1065 | 0.98 | 1.00 | 1126.9 | 14.6 | 7.7 | 0.77 |
| 0-10 s | new | 5 | 7 | 3 | 638 | 0.98 | 1.00 | 1146.7 | 15.1 | 8.0 | 0.78 |
| 10-20 s | prev | 6 | 0 | 0 | 0 | 0.97 | 1.00 | 250.0 | 15.6 | 21.3 | 0.66 |
| 10-20 s | new | 5 | 0 | 0 | 0 | 0.99 | 1.00 | 327.1 | 17.2 | 20.9 | 0.72 |
| 20-30 s | prev | 6 | 0 | 0 | 0 | 1.00 | 1.00 | 287.8 | 16.4 | 18.4 | 0.70 |
| 20-30 s | new | 5 | 0 | 0 | 0 | 0.99 | 1.00 | 417.6 | 17.9 | 17.6 | 0.76 |
| 30-40 s | prev | 6 | 0 | 0 | 0 | 1.00 | 1.00 | 197.0 | 15.5 | 21.6 | 0.60 |
| 30-40 s | new | 5 | 0 | 0 | 0 | 1.00 | 1.00 | 257.2 | 16.7 | 22.1 | 0.64 |
| 40-50 s | prev | 6 | 0 | 0 | 0 | 0.93 | 1.00 | 190.2 | 13.3 | 17.6 | 0.65 |
| 40-50 s | new | 5 | 0 | 0 | 0 | 0.94 | 1.00 | 216.4 | 14.6 | 19.9 | 0.68 |
| 50-60 s | prev | 6 | 0 | 0 | 0 | 0.81 | 1.00 | 341.6 | 12.8 | 14.2 | 0.76 |
| 50-60 s | new | 5 | 0 | 0 | 0 | 0.88 | 1.00 | 278.4 | 12.6 | 13.5 | 0.78 |

Music-matched windows (the same music in both bundles): the music position is the virtual clock since the play mark (gpu1 matched-music.py); the bins are complete 10 s bins of music that each run reached within its 60 s of wall time. wall = the wall time the bin took; distinct frames = Unlock coverage per wall second (1.0 = real time); Unlocks / wall s; underrun quanta and episodes of the page and the engine gap frames over the wall interval of the bin (the 10 s snapshots and the 1 Hz engine reports, linearly interpolated). Median over the counted runs.

| music bin | bundle | n | wall time of the bin (s) | distinct frames per wall s | Unlocks per wall s | underrun quanta | episodes | engine gap frames |
|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 6 | 14.0 | 0.950 | 10.55 | 10 | 2 | 1065 |
| 0-10 s | new | 5 | 13.6 | 0.967 | 10.55 | 7 | 3 | 638 |
| 10-20 s | prev | 6 | 13.5 | 0.840 | 10.55 | 0 | 0 | 0 |
| 10-20 s | new | 5 | 12.6 | 0.879 | 10.74 | 0 | 0 | 0 |
| 20-30 s | prev | 6 | 17.0 | 0.809 | 10.73 | 0 | 0 | 0 |
| 20-30 s | new | 5 | 15.5 | 0.852 | 10.78 | 0 | 0 | 0 |
| 30-40 s | prev | 5 | 13.5 | 0.810 | 9.44 | 0 | 0 | 0 |
| 30-40 s | new | 5 | 13.8 | 0.830 | 9.99 | 0 | 0 | 0 |

Old vs new at the same music position (counted runs; exact permutation test; only bins both bundles reached):

| music bin | measure | old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|
| 0-10 s | wall time of the bin (s) | 13.971 (6) | 13.633 (5) | -0.338 | 0.058 |
| 0-10 s | distinct frames per wall s | 0.950 (6) | 0.967 (5) | +0.018 | 0.212 |
| 0-10 s | underrun quanta | 10.000 (6) | 7.000 (5) | -3.000 | 0.253 |
| 0-10 s | episodes | 2.000 (6) | 3.000 (5) | +1.000 | 0.242 |
| 0-10 s | engine gap frames | 1065.000 (6) | 638.000 (5) | -427.000 | 0.162 |
| 10-20 s | wall time of the bin (s) | 13.546 (6) | 12.574 (5) | -0.973 | 0.009 |
| 10-20 s | distinct frames per wall s | 0.840 (6) | 0.879 (5) | +0.039 | 0.030 |
| 10-20 s | underrun quanta | 0.000 (6) | 0.000 (5) | +0.000 | 1.000 |
| 10-20 s | episodes | 0.000 (6) | 0.000 (5) | +0.000 | 1.000 |
| 10-20 s | engine gap frames | 0.000 (6) | 0.000 (5) | +0.000 | 1.000 |
| 20-30 s | wall time of the bin (s) | 16.950 (6) | 15.487 (5) | -1.463 | 0.026 |
| 20-30 s | distinct frames per wall s | 0.809 (6) | 0.852 (5) | +0.043 | 0.128 |
| 20-30 s | underrun quanta | 0.000 (6) | 0.000 (5) | +0.000 | 1.000 |
| 20-30 s | episodes | 0.000 (6) | 0.000 (5) | +0.000 | 1.000 |
| 20-30 s | engine gap frames | 0.000 (6) | 0.000 (5) | +0.000 | 1.000 |
| 30-40 s | wall time of the bin (s) | 13.503 (5) | 13.793 (5) | +0.290 | 0.048 |
| 30-40 s | distinct frames per wall s | 0.810 (5) | 0.830 (5) | +0.020 | 0.397 |
| 30-40 s | underrun quanta | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |
| 30-40 s | episodes | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |
| 30-40 s | engine gap frames | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 12.500 (6) | 9.000 (5) | -3.500 | 0.253 | 12.500 (6) | 9.500 (6) | -3.000 | 0.370 |
| underrun quanta 30-60 s (heavy interval) | 0.000 (6) | 0.000 (5) | +0.000 | 0.727 | 0.000 (6) | 0.000 (6) | +0.000 | 0.727 |
| episodes 0-60 s | 2.500 (6) | 3.000 (5) | +0.500 | 0.909 | 2.500 (6) | 3.000 (6) | +0.500 | 0.909 |
| engine gap frames 0-60 s | 1385.000 (6) | 894.000 (5) | -491.000 | 0.201 | 1385.000 (6) | 958.000 (6) | -427.000 | 0.351 |
| write rate mean 10-60 s | 0.942 (6) | 0.964 (5) | +0.022 | 0.126 | 0.942 (6) | 0.964 (6) | +0.022 | 0.061 |
| Worker CPU s (60 s) | 50.455 (6) | 50.010 (5) | -0.445 | 0.071 | 50.455 (6) | 49.995 (6) | -0.460 | 0.032 |
| UI idles/s (10-60 s) | 254.805 (6) | 297.053 (5) | +42.248 | 0.002 | 254.805 (6) | 297.393 (6) | +42.588 | 0.002 |
| presents/s (10-60 s) | 14.811 (6) | 15.863 (5) | +1.052 | 0.028 | 14.811 (6) | 15.895 (6) | +1.084 | 0.011 |
| lagCuts/s (10-60 s) | 18.710 (6) | 18.609 (5) | -0.101 | 0.677 | 18.710 (6) | 18.623 (6) | -0.087 | 0.545 |
| virtual s per real s (10-60 s) | 0.679 (6) | 0.716 (5) | +0.036 | 0.019 | 0.679 (6) | 0.717 (6) | +0.038 | 0.009 |

