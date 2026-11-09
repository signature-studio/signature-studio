### P3 decider, chrome: old (previous) vs new, windows of 10 s from the play mark; counted = start load ≤8 and I/O pressure avg10 = 0

| run | bundle | counted | load start → end | I/O avg10 start → end | Worker CPU s (60 s) | main CPU s | underrun quanta 0-60 s | episodes 0-60 s | engine gap frames 0-60 s | **underrun 30-60 s** | write rate mean 10-60 s | consumed rate mean 10-60 s | UI idles/s (10-60 s) | presents/s | lagCuts/s | virtual s per real s | stopped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | prev | NO (not counted) | 8.6 → 5.5 | - → - | 42.7 | 0.0 | 1105 | 99 | 132877 | **904** | 0.405 | 0.999 | - | - | - | 1.013 | False |
| 2 | prev | yes | 5.5 → 5.5 | 0.00 → 0.00 | 44.7 | 0.0 | 960 | 100 | 114215 | **809** | 0.405 | 0.998 | - | - | - | 1.007 | False |
| 3 | prev | yes | 5.3 → 5.2 | 0.00 → 0.00 | 45.6 | 0.0 | 766 | 85 | 90708 | **656** | 0.398 | 1.000 | - | - | - | 0.997 | False |
| 4 | prev | yes | 6.8 → 5.4 | 0.00 → 0.00 | 47.3 | 0.0 | 657 | 69 | 78079 | **591** | 0.392 | 0.993 | - | - | - | 1.001 | False |
| 5 | prev | yes | 5.6 → 5.4 | 0.00 → 0.00 | 45.9 | 0.0 | 785 | 85 | 92735 | **687** | 0.403 | 0.999 | - | - | - | 1.007 | False |
| 6 | prev | yes | 5.8 → 3.1 | 0.00 → 0.00 | 44.9 | 0.0 | 966 | 88 | 116072 | **810** | 0.401 | 0.994 | - | - | - | 1.006 | False |
| 1 | new | yes | 6.2 → 7.7 | - → - | 46.7 | 0.0 | 605 | 66 | 71709 | **571** | 0.398 | 0.998 | - | - | - | 1.002 | False |
| 2 | new | NO (not counted) | 7.7 → 5.5 | 0.04 → 0.00 | 44.0 | 0.0 | 927 | 90 | 110901 | **787** | 0.405 | 0.992 | - | - | - | 1.007 | False |
| 3 | new | yes | 5.2 → 10.1 | 0.00 → 0.00 | 50.1 | 0.0 | 158 | 35 | 17178 | **115** | 0.386 | 0.997 | - | - | - | 0.990 | False |
| 4 | new | NO (not counted) | 10.8 → 6.8 | 0.00 → 0.00 | 45.3 | 0.0 | 1003 | 95 | 120081 | **849** | 0.403 | 0.993 | - | - | - | 1.011 | False |
| 5 | new | yes | 5.2 → 4.9 | 0.00 → 0.00 | 45.0 | 0.0 | 845 | 89 | 100438 | **736** | 0.396 | 1.000 | - | - | - | 1.004 | False |
| 6 | new | yes | 4.9 → 6.0 | 0.00 → 0.00 | 46.0 | 0.0 | 870 | 89 | 103332 | **735** | 0.401 | 0.997 | - | - | - | 1.006 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 5 | 10 | 3 | 1015 | 0.72 | 1.00 | - | - | - | 1.00 |
| 0-10 s | new | 4 | 13 | 3 | 1436 | 0.72 | 1.00 | - | - | - | 1.00 |
| 10-20 s | prev | 5 | 0 | 0 | 7 | 0.48 | 1.00 | - | - | - | 1.00 |
| 10-20 s | new | 4 | 0 | 0 | 8 | 0.48 | 1.00 | - | - | - | 1.00 |
| 20-30 s | prev | 5 | 99 | 15 | 10850 | 0.41 | 1.00 | - | - | - | 1.00 |
| 20-30 s | new | 4 | 65 | 9 | 6692 | 0.41 | 1.00 | - | - | - | 1.00 |
| 30-40 s | prev | 5 | 244 | 29 | 28232 | 0.33 | 1.00 | - | - | - | 1.00 |
| 30-40 s | new | 4 | 185 | 23 | 20370 | 0.33 | 1.00 | - | - | - | 1.00 |
| 40-50 s | prev | 5 | 426 | 33 | 50302 | 0.33 | 1.00 | - | - | - | 1.02 |
| 40-50 s | new | 4 | 419 | 33 | 49120 | 0.33 | 1.00 | - | - | - | 1.01 |
| 50-60 s | prev | 5 | 35 | 4 | 5904 | 0.45 | 0.99 | - | - | - | 0.99 |
| 50-60 s | new | 4 | 31 | 7 | 7641 | 0.44 | 0.99 | - | - | - | 0.99 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 784.838 (5) | 724.784 (4) | -60.054 | 0.238 | 872.259 (6) | 857.463 (6) | -14.796 | 0.407 |
| underrun quanta 30-60 s (heavy interval) | 687.494 (5) | 653.023 (4) | -34.471 | 0.262 | 748.020 (6) | 735.682 (6) | -12.337 | 0.468 |
| episodes 0-60 s | 84.959 (5) | 77.421 (4) | -7.538 | 0.278 | 86.467 (6) | 88.910 (6) | +2.443 | 0.381 |
| engine gap frames 0-60 s | 92734.646 (5) | 86073.067 (4) | -6661.579 | 0.238 | 103474.769 (6) | 101884.855 (6) | -1589.914 | 0.407 |
| write rate mean 10-60 s | 0.401 (5) | 0.397 (4) | -0.005 | 0.325 | 0.402 (6) | 0.399 (6) | -0.003 | 0.558 |
| Worker CPU s (60 s) | 45.560 (5) | 46.340 (4) | +0.780 | 0.294 | 45.245 (6) | 45.635 (6) | +0.390 | 0.396 |
| UI idles/s (10-60 s) | - | - | - | - | - | - | - | - |
| presents/s (10-60 s) | - | - | - | - | - | - | - | - |
| lagCuts/s (10-60 s) | - | - | - | - | - | - | - | - |
| virtual s per real s (10-60 s) | 1.006 (5) | 1.003 (4) | -0.003 | 0.500 | 1.007 (6) | 1.005 (6) | -0.002 | 0.671 |

