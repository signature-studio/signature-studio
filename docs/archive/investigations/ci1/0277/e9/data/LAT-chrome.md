### Input latency T (ms, page sent -> answer presented), chrome; counted = start load <=8 and I/O pressure avg10 = 0

| run | bundle | counted | other Chrome at the start / end / max during the run | waited (s) | load start -> end | I/O avg10 start -> end | section | input | n sent | answered | T median / p90 / p99 / max |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | prev | NO (load / I/O) | 0 / 0 / 0 | 305 | 8.66 -> 9.13 | 0 -> 0 | a-idle-menu | Esc (key down) | 35 | 35 | 2.2 / 3.5 / 16.6 / 16.6 |
| 1 | prev | NO (load / I/O) | 0 / 0 / 0 | 305 | 8.66 -> 9.13 | 0 -> 0 | a-idle-menu | File click (button down) | 36 | 36 | 7.9 / 11.6 / 25.2 / 25.2 |
| 1 | prev | NO (load / I/O) | 0 / 0 / 0 | 305 | 8.66 -> 9.13 | 0 -> 0 | b-play-menu | Esc (key down) | 36 | 36 | 26.2 / 105.0 / 179.5 / 179.5 |
| 1 | prev | NO (load / I/O) | 0 / 0 / 0 | 305 | 8.66 -> 9.13 | 0 -> 0 | b-play-menu | File click (button down) | 35 | 35 | 35.6 / 161.6 / 204.0 / 204.0 |
| 2 | prev | yes | 1 / 1 / 1 | 50 | 7.77 -> 15.06 | 0 -> 0 | a-idle-menu | Esc (key down) | 35 | 35 | 2.1 / 2.7 / 3.3 / 3.3 |
| 2 | prev | yes | 1 / 1 / 1 | 50 | 7.77 -> 15.06 | 0 -> 0 | a-idle-menu | File click (button down) | 36 | 36 | 7.8 / 12.3 / 27.8 / 27.8 |
| 2 | prev | yes | 1 / 1 / 1 | 50 | 7.77 -> 15.06 | 0 -> 0 | b-play-menu | Esc (key down) | 36 | 36 | 50.2 / 134.3 / 303.5 / 303.5 |
| 2 | prev | yes | 1 / 1 / 1 | 50 | 7.77 -> 15.06 | 0 -> 0 | b-play-menu | File click (button down) | 35 | 35 | 137.3 / 232.9 / 356.2 / 356.2 |
| 3 | prev | yes | 1 / 1 / 1 | 55 | 7.93 -> 11.32 | 0 -> 0 | a-idle-menu | Esc (key down) | 35 | 35 | 1.9 / 2.7 / 6.8 / 6.8 |
| 3 | prev | yes | 1 / 1 / 1 | 55 | 7.93 -> 11.32 | 0 -> 0 | a-idle-menu | File click (button down) | 36 | 36 | 7.3 / 11.1 / 19.6 / 19.6 |
| 3 | prev | yes | 1 / 1 / 1 | 55 | 7.93 -> 11.32 | 0 -> 0 | b-play-menu | Esc (key down) | 36 | 36 | 19.6 / 113.1 / 202.0 / 202.0 |
| 3 | prev | yes | 1 / 1 / 1 | 55 | 7.93 -> 11.32 | 0 -> 0 | b-play-menu | File click (button down) | 35 | 35 | 137.9 / 216.5 / 391.8 / 391.8 |
| 1 | new | NO (load / I/O) | 0 / 1 / 1 | 160 | 7.63 -> 17.57 | 0.18 -> 0.07 | a-idle-menu | Esc (key down) | 35 | 34 | 1.9 / 2.5 / 5.0 / 5.0 |
| 1 | new | NO (load / I/O) | 0 / 1 / 1 | 160 | 7.63 -> 17.57 | 0.18 -> 0.07 | a-idle-menu | File click (button down) | 36 | 36 | 7.0 / 12.8 / 24.8 / 24.8 |
| 1 | new | NO (load / I/O) | 0 / 1 / 1 | 160 | 7.63 -> 17.57 | 0.18 -> 0.07 | b-play-menu | Esc (key down) | 36 | 34 | 6.3 / 21.1 / 40.6 / 40.6 |
| 1 | new | NO (load / I/O) | 0 / 1 / 1 | 160 | 7.63 -> 17.57 | 0.18 -> 0.07 | b-play-menu | File click (button down) | 35 | 35 | 23.9 / 37.1 / 61.5 / 61.5 |
| 2 | new | yes | 1 / 0 / 1 | 686 | 7.21 -> 10.52 | 0 -> 0.02 | a-idle-menu | Esc (key down) | 35 | 29 | 2.2 / 3.0 / 4.6 / 4.6 |
| 2 | new | yes | 1 / 0 / 1 | 686 | 7.21 -> 10.52 | 0 -> 0.02 | a-idle-menu | File click (button down) | 36 | 36 | 7.8 / 12.2 / 25.6 / 25.6 |
| 2 | new | yes | 1 / 0 / 1 | 686 | 7.21 -> 10.52 | 0 -> 0.02 | b-play-menu | Esc (key down) | 36 | 36 | 7.0 / 17.6 / 36.1 / 36.1 |
| 2 | new | yes | 1 / 0 / 1 | 686 | 7.21 -> 10.52 | 0 -> 0.02 | b-play-menu | File click (button down) | 35 | 35 | 23.7 / 36.7 / 49.5 / 49.5 |
| 3 | new | yes | 1 / 1 / 1 | 100 | 7.86 -> 5.97 | 0 -> 0 | a-idle-menu | Esc (key down) | 35 | 34 | 2.2 / 3.0 / 5.3 / 5.3 |
| 3 | new | yes | 1 / 1 / 1 | 100 | 7.86 -> 5.97 | 0 -> 0 | a-idle-menu | File click (button down) | 36 | 36 | 7.0 / 9.5 / 23.6 / 23.6 |
| 3 | new | yes | 1 / 1 / 1 | 100 | 7.86 -> 5.97 | 0 -> 0 | b-play-menu | Esc (key down) | 36 | 35 | 4.4 / 11.1 / 29.3 / 29.3 |
| 3 | new | yes | 1 / 1 / 1 | 100 | 7.86 -> 5.97 | 0 -> 0 | b-play-menu | File click (button down) | 35 | 35 | 21.2 / 29.2 / 39.5 / 39.5 |

Per bundle (all answered records of the counted runs pooled):

| section | input | bundle | runs | answered | T median / p90 / p99 / max |
|---|---|---|---|---|---|
| a-idle-menu | Esc (key down) | prev | 2 | 70 | 2.0 / 2.7 / 6.8 / 6.8 |
| a-idle-menu | Esc (key down) | new | 2 | 63 | 2.2 / 3.0 / 5.3 / 5.3 |
| a-idle-menu | File click (button down) | prev | 2 | 72 | 7.7 / 11.1 / 27.8 / 27.8 |
| a-idle-menu | File click (button down) | new | 2 | 72 | 7.3 / 11.4 / 25.6 / 25.6 |
| b-play-menu | Esc (key down) | prev | 2 | 72 | 33.8 / 117.7 / 303.5 / 303.5 |
| b-play-menu | Esc (key down) | new | 2 | 71 | 5.4 / 16.9 / 36.1 / 36.1 |
| b-play-menu | File click (button down) | prev | 2 | 70 | 137.3 / 226.0 / 391.8 / 391.8 |
| b-play-menu | File click (button down) | new | 2 | 70 | 22.1 / 31.4 / 49.5 / 49.5 |
