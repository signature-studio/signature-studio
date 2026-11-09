### P3 decider, firefox: old (previous) vs new, windows of 10 s from the play mark; counted = start load ≤8 and I/O pressure avg10 = 0

| run | bundle | counted | load start → end | I/O avg10 start → end | Worker CPU s (60 s) | main CPU s | underrun quanta 0-60 s | episodes 0-60 s | engine gap frames 0-60 s | **underrun 30-60 s** | write rate mean 10-60 s | consumed rate mean 10-60 s | UI idles/s (10-60 s) | presents/s | lagCuts/s | virtual s per real s | stopped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | prev | yes | 2.4 → 4.1 | 0.00 → 0.00 | 49.0 | 0.0 | 443 | 60 | 51520 | **399** | 0.394 | 0.996 | - | - | - | 1.000 | False |
| 2 | prev | yes | 6.7 → 12.3 | 0.00 → 0.01 | 56.8 | 0.0 | 8 | 2 | 837 | **8** | 0.312 | 0.996 | - | - | - | 0.892 | False |
| 3 | prev | NO (not counted) | 13.3 → 11.9 | 0.18 → 0.00 | 49.0 | 0.0 | 451 | 64 | 52144 | **436** | 0.381 | 1.000 | - | - | - | 1.000 | False |
| 1 | new | yes | 4.0 → 4.8 | 0.00 → 0.00 | 49.6 | 0.0 | 425 | 62 | 49055 | **379** | 0.388 | 1.000 | - | - | - | 1.000 | False |
| 2 | new | yes | 4.8 → 5.7 | 0.00 → 0.00 | 50.7 | 0.0 | 299 | 45 | 34270 | **289** | 0.385 | 0.999 | - | - | - | 1.000 | False |
| 3 | new | NO (not counted) | 12.1 → 13.8 | 0.00 → 0.00 | 53.8 | 0.0 | 64 | 19 | 10197 | **64** | 0.359 | 0.997 | - | - | - | 0.997 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 2 | 0 | 0 | 0 | 0.69 | 1.00 | - | - | - | 1.00 |
| 0-10 s | new | 2 | 0 | 0 | 0 | 0.72 | 1.00 | - | - | - | 1.00 |
| 10-20 s | prev | 2 | 0 | 0 | 0 | 0.43 | 1.00 | - | - | - | 1.00 |
| 10-20 s | new | 2 | 0 | 0 | 0 | 0.44 | 1.00 | - | - | - | 1.00 |
| 20-30 s | prev | 2 | 22 | 3 | 2490 | 0.34 | 1.00 | - | - | - | 0.84 |
| 20-30 s | new | 2 | 28 | 5 | 1626 | 0.42 | 1.00 | - | - | - | 1.00 |
| 30-40 s | prev | 2 | 76 | 12 | 8752 | 0.29 | 1.00 | - | - | - | 0.86 |
| 30-40 s | new | 2 | 114 | 18 | 13619 | 0.33 | 1.00 | - | - | - | 1.00 |
| 40-50 s | prev | 2 | 114 | 13 | 13485 | 0.33 | 1.00 | - | - | - | 1.01 |
| 40-50 s | new | 2 | 196 | 26 | 22528 | 0.33 | 1.00 | - | - | - | 1.00 |
| 50-60 s | prev | 2 | 13 | 2 | 1451 | 0.37 | 0.98 | - | - | - | 1.03 |
| 50-60 s | new | 2 | 25 | 4 | 3889 | 0.41 | 1.00 | - | - | - | 0.99 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 225.328 (2) | 362.185 (2) | +136.858 | 1.000 | 442.798 (3) | 299.472 (3) | -143.326 | 0.800 |
| underrun quanta 30-60 s (heavy interval) | 203.542 (2) | 333.946 (2) | +130.403 | 1.000 | 399.227 (3) | 288.583 (3) | -110.644 | 0.700 |
| episodes 0-60 s | 30.969 (2) | 53.445 (2) | +22.476 | 0.667 | 59.973 (3) | 44.914 (3) | -15.059 | 1.000 |
| engine gap frames 0-60 s | 26178.645 (2) | 41662.521 (2) | +15483.876 | 1.000 | 51520.437 (3) | 34269.953 (3) | -17250.484 | 0.800 |
| write rate mean 10-60 s | 0.353 (2) | 0.386 (2) | +0.033 | 1.000 | 0.381 (3) | 0.385 (3) | +0.004 | 0.800 |
| Worker CPU s (60 s) | 52.865 (2) | 50.160 (2) | -2.705 | 1.000 | 49.030 (3) | 50.690 (3) | +1.660 | 1.000 |
| UI idles/s (10-60 s) | - | - | - | - | - | - | - | - |
| presents/s (10-60 s) | - | - | - | - | - | - | - | - |
| lagCuts/s (10-60 s) | - | - | - | - | - | - | - | - |
| virtual s per real s (10-60 s) | 0.946 (2) | 1.000 (2) | +0.054 | 0.667 | 1.000 (3) | 1.000 (3) | -0.000 | 0.800 |

