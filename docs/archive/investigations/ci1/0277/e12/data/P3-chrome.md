### P3 decider, chrome: old (previous) vs new, windows of 10 s from the play mark; counted = start load ≤8 and I/O pressure avg10 = 0

| run | bundle | counted | other Chrome browsers at the play mark / at +60 s / max | waited for the quiet window (s) | load start → end | I/O avg10 start → end | Worker CPU s (60 s) = CPU/wall x 60 | main CPU s | underrun quanta 0-60 s | episodes 0-60 s | engine gap frames 0-60 s | **underrun 30-60 s** | write rate mean 10-60 s | consumed rate mean 10-60 s | UI idles/s (10-60 s) | presents/s | lagCuts/s | virtual s per real s | stopped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | prev | yes | 0 / 0 / 0 | 0 | 4.3 → 3.3 | 0.00 → 0.00 | 49.7 | 0.0 | 0 | 0 | 0 | **0** | 0.996 | 0.996 | 407.2 | 18.0 | 16.5 | 0.791 | False |
| 2 | prev | yes | 0 / 0 / 0 | 65 | 3.7 → 4.6 | 0.00 → 0.00 | 50.1 | 0.0 | 8 | 4 | 680 | **4** | 0.973 | 0.996 | 316.9 | 16.0 | 17.4 | 0.731 | False |
| 3 | prev | reference (PM verification work overlapped, I/O pressure avg10 3.0 at the end) | 0 / 0 / 0 | 0 | 4.6 → 6.3 | 0.00 → 3.00 | 50.4 | 0.0 | 12 | 2 | 1364 | **0** | 0.938 | 0.998 | 258.8 | 14.2 | 18.2 | 0.678 | False |
| 4 | prev | yes | 0 / 0 / 0 | 0 | 7.6 → 7.6 | 0.00 → 0.00 | 50.0 | 0.0 | 29 | 3 | 3454 | **0** | 0.968 | 0.997 | 326.0 | 15.8 | 17.5 | 0.732 | False |
| 5 | prev | reference (opt1 wasm-opt link overlapped, load 13.0 at the end) | 0 / 0 / 0 | 0 | 7.7 → 12.0 | 0.00 → 0.00 | 49.1 | 0.0 | 46 | 12 | 4836 | **34** | 0.890 | 1.000 | 330.2 | 15.0 | 17.0 | 0.677 | False |
| 6 | prev | yes | 0 / 0 / 0 | 0 | 6.9 → 6.0 | 0.00 → 0.00 | 50.1 | 0.0 | 6 | 3 | 510 | **0** | 0.981 | 0.998 | 337.4 | 16.6 | 18.7 | 0.745 | False |
| 7 | prev | reference (alpha1 node Export + mt3 clang x2 + gpu1 smoke Chrome overlapped (about 1 minute, PM 20:26)) | 1 / 1 / 1 | 0 | 6.0 → 8.4 | 0.00 → 0.00 | 49.9 | 0.0 | 4 | 2 | 340 | **0** | 0.957 | 0.996 | 354.9 | 16.0 | 17.1 | 0.737 | False |
| 8 | prev | yes | 0 / 0 / 0 | 0 | 1.7 → 1.1 | 0.00 → 0.00 | 49.4 | 0.0 | 0 | 0 | 0 | **0** | 0.983 | 0.999 | 431.2 | 18.7 | 17.1 | 0.801 | False |
| 1 | new | yes | 0 / 0 / 0 | 0 | 3.3 → 6.2 | 0.00 → 0.11 | 50.8 | 0.0 | 71 | 8 | 8400 | **0** | 0.884 | 1.000 | 325.8 | 14.5 | 18.4 | 0.661 | False |
| 2 | new | yes | 0 / 0 / 0 | 140 | 5.4 → 4.7 | 0.00 → 0.17 | 49.9 | 0.0 | 6 | 1 | 682 | **0** | 0.966 | 1.000 | 339.2 | 16.8 | 18.9 | 0.744 | False |
| 3 | new | yes | 0 / 0 / 0 | 531 | 2.6 → 8.0 | 0.00 → 0.00 | 50.0 | 0.0 | 6 | 3 | 510 | **0** | 0.959 | 0.996 | 285.9 | 15.2 | 17.9 | 0.707 | False |
| 4 | new | yes | 0 / 0 / 0 | 65 | 6.3 → 7.6 | 0.00 → 0.00 | 50.2 | 0.0 | 8 | 2 | 852 | **0** | 0.960 | 0.997 | 317.5 | 15.0 | 17.9 | 0.717 | False |
| 5 | new | yes | 0 / 0 / 0 | 255 | 8.0 → 7.3 | 0.00 → 0.00 | 49.4 | 0.0 | 8 | 2 | 852 | **0** | 0.975 | 1.000 | 366.2 | 17.2 | 17.3 | 0.759 | False |
| 6 | new | yes | 0 / 0 / 0 | 0 | 7.3 → 6.9 | 0.00 → 0.00 | 50.0 | 0.0 | 10 | 1 | 1194 | **0** | 0.992 | 0.997 | 331.5 | 16.7 | 18.1 | 0.753 | False |
| 7 | new | reference (gpu1 smoke (rt-psmoke-p2, Chrome, 20:26:00-20:30:16) overlapped) | 1 / 1 / 1 | 70 | 3.8 → 2.1 | 0.00 → 0.00 | 49.8 | 0.0 | 5 | 1 | 554 | **0** | 0.988 | 1.000 | 365.1 | 17.6 | 17.9 | 0.772 | False |
| 8 | new | reference (gpu1 smoke (until 20:30:16) overlapped the first 50 s of this run (PM 20:31)) | 1 / 0 / 1 | 0 | 2.1 → 1.7 | 0.00 → 0.00 | 49.7 | 0.0 | 3 | 1 | 298 | **0** | 0.992 | 1.000 | 410.1 | 18.4 | 17.7 | 0.791 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 5 | 4 | 2 | 340 | 0.98 | 1.00 | 1218.5 | 15.6 | 5.8 | 0.81 |
| 0-10 s | new | 6 | 8 | 2 | 852 | 0.98 | 1.00 | 1158.4 | 14.9 | 7.2 | 0.78 |
| 10-20 s | prev | 5 | 0 | 0 | 0 | 0.99 | 1.00 | 424.9 | 18.6 | 19.3 | 0.77 |
| 10-20 s | new | 6 | 0 | 0 | 0 | 0.99 | 1.00 | 399.2 | 16.5 | 20.1 | 0.72 |
| 20-30 s | prev | 5 | 0 | 0 | 0 | 1.00 | 1.00 | 589.3 | 18.9 | 13.8 | 0.84 |
| 20-30 s | new | 6 | 0 | 0 | 0 | 1.00 | 1.00 | 455.2 | 18.4 | 15.7 | 0.79 |
| 30-40 s | prev | 5 | 0 | 0 | 0 | 0.99 | 1.00 | 329.3 | 18.4 | 22.3 | 0.67 |
| 30-40 s | new | 6 | 0 | 0 | 0 | 1.00 | 1.00 | 306.2 | 18.5 | 23.3 | 0.66 |
| 40-50 s | prev | 5 | 0 | 0 | 0 | 0.98 | 1.00 | 282.1 | 15.3 | 19.8 | 0.73 |
| 40-50 s | new | 6 | 0 | 0 | 0 | 0.97 | 1.00 | 232.2 | 15.5 | 18.4 | 0.71 |
| 50-60 s | prev | 5 | 0 | 0 | 0 | 0.92 | 0.99 | 219.9 | 12.9 | 15.1 | 0.77 |
| 50-60 s | new | 6 | 0 | 0 | 0 | 0.86 | 0.99 | 270.6 | 12.3 | 13.8 | 0.76 |

Music-matched windows (the same music in both bundles): the music position is the virtual clock since the play mark (gpu1 matched-music.py); the bins are complete 10 s bins of music that each run reached within its 60 s of wall time. wall = the wall time the bin took; distinct frames = Unlock coverage per wall second (1.0 = real time); Unlocks / wall s; underrun quanta and episodes of the page and the engine gap frames over the wall interval of the bin (the 10 s snapshots and the 1 Hz engine reports, linearly interpolated). Median over the counted runs.

| music bin | bundle | n | wall time of the bin (s) | distinct frames per wall s | Unlocks per wall s | underrun quanta | episodes | engine gap frames |
|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 5 | 12.9 | 0.976 | 10.51 | 4 | 2 | 340 |
| 0-10 s | new | 6 | 13.5 | 0.965 | 10.55 | 8 | 2 | 852 |
| 10-20 s | prev | 5 | 11.5 | 0.947 | 10.75 | 0 | 0 | 0 |
| 10-20 s | new | 6 | 12.3 | 0.918 | 10.75 | 0 | 0 | 0 |
| 20-30 s | prev | 5 | 14.3 | 0.941 | 10.76 | 0 | 0 | 0 |
| 20-30 s | new | 6 | 15.0 | 0.918 | 10.74 | 0 | 0 | 0 |
| 30-40 s | prev | 5 | 13.4 | 0.874 | 10.41 | 0 | 0 | 0 |
| 30-40 s | new | 5 | 13.6 | 0.861 | 10.43 | 0 | 0 | 0 |

Old vs new at the same music position (counted runs; exact permutation test; only bins both bundles reached):

| music bin | measure | old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|
| 0-10 s | wall time of the bin (s) | 12.939 (5) | 13.483 (6) | +0.545 | 0.154 |
| 0-10 s | distinct frames per wall s | 0.976 (5) | 0.965 (6) | -0.011 | 0.489 |
| 0-10 s | underrun quanta | 4.000 (5) | 8.000 (6) | +4.000 | 0.216 |
| 0-10 s | episodes | 2.000 (5) | 2.000 (6) | +0.000 | 0.424 |
| 0-10 s | engine gap frames | 340.000 (5) | 852.000 (6) | +512.000 | 0.223 |
| 10-20 s | wall time of the bin (s) | 11.542 (5) | 12.344 (6) | +0.802 | 0.277 |
| 10-20 s | distinct frames per wall s | 0.947 (5) | 0.918 (6) | -0.028 | 0.195 |
| 10-20 s | underrun quanta | 0.000 (5) | 0.000 (6) | +0.000 | 1.000 |
| 10-20 s | episodes | 0.000 (5) | 0.000 (6) | +0.000 | 1.000 |
| 10-20 s | engine gap frames | 0.000 (5) | 0.000 (6) | +0.000 | 1.000 |
| 20-30 s | wall time of the bin (s) | 14.311 (5) | 15.007 (6) | +0.696 | 0.214 |
| 20-30 s | distinct frames per wall s | 0.941 (5) | 0.918 (6) | -0.023 | 0.580 |
| 20-30 s | underrun quanta | 0.000 (5) | 0.000 (6) | +0.000 | 0.182 |
| 20-30 s | episodes | 0.000 (5) | 0.000 (6) | +0.000 | 0.182 |
| 20-30 s | engine gap frames | 0.000 (5) | 0.000 (6) | +0.000 | 0.182 |
| 30-40 s | wall time of the bin (s) | 13.376 (5) | 13.591 (5) | +0.215 | 0.730 |
| 30-40 s | distinct frames per wall s | 0.874 (5) | 0.861 (5) | -0.013 | 0.532 |
| 30-40 s | underrun quanta | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |
| 30-40 s | episodes | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |
| 30-40 s | engine gap frames | 0.000 (5) | 0.000 (5) | +0.000 | 1.000 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 6.000 (5) | 8.000 (6) | +2.000 | 0.569 | 7.000 (8) | 7.000 (8) | +0.000 | 0.832 |
| underrun quanta 30-60 s (heavy interval) | 0.000 (5) | 0.000 (6) | +0.000 | 0.455 | 0.000 (8) | 0.000 (8) | +0.000 | 0.200 |
| episodes 0-60 s | 3.000 (5) | 2.000 (6) | -1.000 | 0.686 | 2.500 (8) | 1.500 (8) | -1.000 | 0.646 |
| engine gap frames 0-60 s | 510.000 (5) | 852.000 (6) | +342.000 | 0.578 | 595.000 (8) | 767.000 (8) | +172.000 | 0.785 |
| write rate mean 10-60 s | 0.981 (5) | 0.963 (6) | -0.018 | 0.177 | 0.971 (8) | 0.971 (8) | +0.000 | 0.811 |
| Worker CPU s (60 s) | 49.990 (5) | 49.965 (6) | -0.025 | 0.450 | 49.945 (8) | 49.905 (8) | -0.040 | 0.522 |
| UI idles/s (10-60 s) | 337.387 (5) | 328.680 (6) | -8.707 | 0.190 | 333.786 (8) | 335.378 (8) | +1.592 | 0.915 |
| presents/s (10-60 s) | 16.641 (5) | 15.944 (6) | -0.697 | 0.165 | 15.997 (8) | 16.763 (8) | +0.766 | 0.860 |
| lagCuts/s (10-60 s) | 17.376 (5) | 18.019 (6) | +0.643 | 0.149 | 17.249 (8) | 17.887 (8) | +0.638 | 0.086 |
| virtual s per real s (10-60 s) | 0.745 (5) | 0.731 (6) | -0.015 | 0.128 | 0.734 (8) | 0.748 (8) | +0.014 | 0.941 |

