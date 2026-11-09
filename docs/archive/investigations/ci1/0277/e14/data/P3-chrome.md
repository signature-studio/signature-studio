### P3 decider, chrome: old (previous) vs new, windows of 10 s from the play mark; counted = start load ≤8 and I/O pressure avg10 = 0

| run | bundle | counted | other Chrome browsers at the play mark / at +60 s / max | waited for the quiet window (s) | load start → end | I/O avg10 start → end | Worker CPU s (60 s) = CPU/wall x 60 | main CPU s | underrun quanta 0-60 s | episodes 0-60 s | engine gap frames 0-60 s | **underrun 30-60 s** | write rate mean 10-60 s | consumed rate mean 10-60 s | UI idles/s (10-60 s) | presents/s | lagCuts/s | virtual s per real s | stopped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | prev | yes | 0 / 0 / 0 | 2 | 0.9 → 2.7 | 0.00 → 0.00 | 50.3 | 0.0 | 8 | 2 | 852 | **0** | 0.962 | 1.000 | 284.9 | 15.8 | 19.3 | 0.707 | False |
| 2 | prev | yes | 0 / 0 / 0 | 7 | 3.2 → 3.1 | 0.00 → 0.00 | 50.1 | 0.0 | 2 | 1 | 170 | **0** | 0.910 | 1.000 | 298.0 | 15.0 | 18.4 | 0.675 | False |
| 3 | prev | yes | 0 / 0 / 0 | 2 | 2.9 → 2.9 | 0.00 → 0.00 | 51.4 | 0.0 | 8 | 2 | 852 | **0** | 0.879 | 1.000 | 262.4 | 12.4 | 16.3 | 0.636 | False |
| 4 | prev | yes | 0 / 0 / 0 | 7 | 2.0 → 2.0 | 0.00 → 0.00 | 51.2 | 0.0 | 14 | 3 | 1534 | **0** | 0.884 | 1.000 | 283.4 | 12.9 | 16.3 | 0.652 | False |
| 5 | prev | yes | 0 / 0 / 0 | 2 | 2.0 → 1.4 | 0.00 → 0.00 | 51.2 | 0.0 | 3 | 1 | 298 | **0** | 0.914 | 1.000 | 237.8 | 13.2 | 18.1 | 0.652 | False |
| 6 | prev | yes | 0 / 0 / 0 | 7 | 3.7 → 4.1 | 0.00 → 0.00 | 50.7 | 0.0 | 7 | 2 | 724 | **0** | 0.916 | 1.000 | 246.9 | 14.0 | 18.5 | 0.661 | False |
| 1 | new | yes | 0 / 0 / 0 | 2 | 2.5 → 3.6 | 0.00 → 0.00 | 70.5 | 0.0 | 4 | 2 | 340 | **0** | 0.953 | 0.997 | 856.7 | 29.9 | 0.2 | 0.978 | False |
| 2 | new | yes | 0 / 0 / 0 | 2 | 3.3 → 3.4 | 0.00 → 0.00 | 70.7 | 0.0 | 2 | 1 | 170 | **0** | 0.947 | 0.995 | 859.6 | 30.1 | 0.9 | 0.973 | False |
| 3 | new | yes | 0 / 0 / 0 | 2 | 2.9 → 2.8 | 0.00 → 0.00 | 74.3 | 0.0 | 7 | 3 | 638 | **0** | 0.918 | 0.995 | 717.9 | 29.4 | 4.7 | 0.886 | False |
| 4 | new | yes | 0 / 0 / 0 | 2 | 2.6 → 2.3 | 0.00 → 0.00 | 74.7 | 0.0 | 10 | 1 | 1194 | **0** | 0.877 | 0.995 | 730.2 | 29.2 | 4.6 | 0.864 | False |
| 5 | new | yes | 0 / 0 / 0 | 2 | 1.3 → 3.7 | 0.00 → 0.00 | 74.0 | 0.0 | 10 | 3 | 1022 | **0** | 0.927 | 1.000 | 759.2 | 29.3 | 4.9 | 0.911 | False |
| 6 | new | yes | 0 / 0 / 0 | 2 | 3.7 → 4.2 | 0.00 → 0.00 | 73.0 | 0.0 | 18 | 3 | 2046 | **0** | 0.947 | 0.995 | 736.6 | 29.5 | 3.0 | 0.913 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 6 | 8 | 2 | 788 | 0.98 | 1.00 | 1135.1 | 13.8 | 7.1 | 0.77 |
| 0-10 s | new | 6 | 8 | 2 | 830 | 0.98 | 1.00 | 1516.1 | 20.1 | 0.0 | 0.96 |
| 10-20 s | prev | 6 | 0 | 0 | 0 | 0.97 | 1.00 | 238.3 | 13.2 | 19.0 | 0.64 |
| 10-20 s | new | 6 | 0 | 0 | 0 | 0.97 | 1.00 | 889.8 | 29.3 | 0.5 | 0.98 |
| 20-30 s | prev | 6 | 0 | 0 | 0 | 0.97 | 1.00 | 302.3 | 14.8 | 17.1 | 0.71 |
| 20-30 s | new | 6 | 0 | 0 | 0 | 0.99 | 1.00 | 755.8 | 29.5 | 1.3 | 0.93 |
| 30-40 s | prev | 6 | 0 | 0 | 0 | 0.97 | 1.00 | 200.0 | 14.0 | 19.6 | 0.59 |
| 30-40 s | new | 6 | 0 | 0 | 0 | 0.98 | 1.00 | 576.9 | 29.7 | 7.1 | 0.79 |
| 40-50 s | prev | 6 | 0 | 0 | 0 | 0.92 | 1.00 | 196.9 | 12.6 | 18.2 | 0.63 |
| 40-50 s | new | 6 | 0 | 0 | 0 | 0.92 | 1.00 | 614.6 | 29.6 | 6.7 | 0.86 |
| 50-60 s | prev | 6 | 0 | 0 | 0 | 0.75 | 1.00 | 397.2 | 12.9 | 13.7 | 0.75 |
| 50-60 s | new | 6 | 0 | 0 | 0 | 0.79 | 0.98 | 991.4 | 29.2 | 0.3 | 1.03 |

Music-matched windows (the same music in both bundles): the music position is the virtual clock since the play mark (gpu1 matched-music.py); the bins are complete 10 s bins of music that each run reached within its 60 s of wall time. wall = the wall time the bin took; distinct frames = Unlock coverage per wall second (1.0 = real time); Unlocks / wall s; underrun quanta and episodes of the page and the engine gap frames over the wall interval of the bin (the 10 s snapshots and the 1 Hz engine reports, linearly interpolated). Median over the counted runs.

| music bin | bundle | n | wall time of the bin (s) | distinct frames per wall s | Unlocks per wall s | underrun quanta | episodes | engine gap frames |
|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 6 | 14.1 | 0.938 | 10.53 | 8 | 2 | 788 |
| 0-10 s | new | 6 | 10.4 | 0.953 | 10.46 | 8 | 2 | 830 |
| 10-20 s | prev | 6 | 14.0 | 0.857 | 10.45 | 0 | 0 | 0 |
| 10-20 s | new | 6 | 10.4 | 0.887 | 10.48 | 0 | 0 | 0 |
| 20-30 s | prev | 6 | 17.1 | 0.803 | 10.45 | 0 | 0 | 0 |
| 20-30 s | new | 6 | 11.1 | 0.828 | 10.73 | 0 | 0 | 0 |
| 30-40 s | prev | 5 | 13.6 | 0.758 | 8.77 | 0 | 0 | 0 |
| 30-40 s | new | 6 | 12.6 | 0.786 | 10.65 | 0 | 0 | 0 |
| 40-50 s | new | 6 | 10.1 | 0.775 | 9.02 | 0 | 0 | 0 |

Old vs new at the same music position (counted runs; exact permutation test; only bins both bundles reached):

| music bin | measure | old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|
| 0-10 s | wall time of the bin (s) | 14.119 (6) | 10.428 (6) | -3.690 | 0.002 |
| 0-10 s | distinct frames per wall s | 0.938 (6) | 0.953 (6) | +0.015 | 0.139 |
| 0-10 s | underrun quanta | 7.500 (6) | 8.500 (6) | +1.000 | 0.654 |
| 0-10 s | episodes | 2.000 (6) | 2.500 (6) | +0.500 | 0.732 |
| 0-10 s | engine gap frames | 788.000 (6) | 830.000 (6) | +42.000 | 0.639 |
| 10-20 s | wall time of the bin (s) | 13.995 (6) | 10.432 (6) | -3.563 | 0.002 |
| 10-20 s | distinct frames per wall s | 0.857 (6) | 0.887 (6) | +0.029 | 0.240 |
| 10-20 s | underrun quanta | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 10-20 s | episodes | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 10-20 s | engine gap frames | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 20-30 s | wall time of the bin (s) | 17.121 (6) | 11.144 (6) | -5.977 | 0.002 |
| 20-30 s | distinct frames per wall s | 0.803 (6) | 0.828 (6) | +0.025 | 0.416 |
| 20-30 s | underrun quanta | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 20-30 s | episodes | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 20-30 s | engine gap frames | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 30-40 s | wall time of the bin (s) | 13.600 (5) | 12.608 (6) | -0.992 | 0.071 |
| 30-40 s | distinct frames per wall s | 0.758 (5) | 0.786 (6) | +0.028 | 0.582 |
| 30-40 s | underrun quanta | 0.000 (5) | 0.000 (6) | +0.000 | 1.000 |
| 30-40 s | episodes | 0.000 (5) | 0.000 (6) | +0.000 | 1.000 |
| 30-40 s | engine gap frames | 0.000 (5) | 0.000 (6) | +0.000 | 1.000 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 7.500 (6) | 8.500 (6) | +1.000 | 0.654 | 7.500 (6) | 8.500 (6) | +1.000 | 0.654 |
| underrun quanta 30-60 s (heavy interval) | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| episodes 0-60 s | 2.000 (6) | 2.500 (6) | +0.500 | 0.732 | 2.000 (6) | 2.500 (6) | +0.500 | 0.732 |
| engine gap frames 0-60 s | 788.000 (6) | 830.000 (6) | +42.000 | 0.677 | 788.000 (6) | 830.000 (6) | +42.000 | 0.677 |
| write rate mean 10-60 s | 0.912 (6) | 0.937 (6) | +0.025 | 0.348 | 0.912 (6) | 0.937 (6) | +0.025 | 0.348 |
| Worker CPU s (60 s) | 50.955 (6) | 73.530 (6) | +22.575 | 0.002 | 50.955 (6) | 73.530 (6) | +22.575 | 0.002 |
| UI idles/s (10-60 s) | 272.927 (6) | 747.919 (6) | +474.992 | 0.002 | 272.927 (6) | 747.919 (6) | +474.992 | 0.002 |
| presents/s (10-60 s) | 13.584 (6) | 29.441 (6) | +15.857 | 0.002 | 13.584 (6) | 29.441 (6) | +15.857 | 0.002 |
| lagCuts/s (10-60 s) | 18.248 (6) | 3.774 (6) | -14.474 | 0.002 | 18.248 (6) | 3.774 (6) | -14.474 | 0.002 |
| virtual s per real s (10-60 s) | 0.656 (6) | 0.912 (6) | +0.255 | 0.002 | 0.656 (6) | 0.912 (6) | +0.255 | 0.002 |

