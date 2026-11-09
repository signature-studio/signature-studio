| run | ok | load start/end | first idle ms | N (Workers) | CPU s (cores) | underrun pre-play (q/ep) | underrun after play (q/ep) | longest q | lag cuts after play | engine gap frames after play | Unlock span s | write rate mean (min) / <0.95 | write slope | Unlock gap p99/max ms | present/s |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| s3a-r9-e13-song | True | 3.36/5.8 | 5614 | 1 | 259.25 (0.86) | 1521/1 | 34/4 | 1521 | 4065 | 4008 | 38-300 | 1.000 (0.985) / 0/26 | 1.000 | 231/265 | 13.3 |
| s3a-r9-rta-song | True | 4.56/11.57 | 8848 | 8 | 569.13 (1.89) | 0/0 | 0/0 | 0 | 0 | 0 | 22-301 | 1.000 (0.985) / 0/27 | 1.000 | 227/326 | 26.6 |
| s3a-r9-rtaw2-song | True | 7.44/8.54 | 8699 | 2 | 444.77 (1.48) | 0/0 | 22/3 | 10 | 0 | 2558 | 9-300 | 1.000 (0.975) / 0/29 | 1.000 | 236/304 | 29.4 |
s3a-r9-e13-song workers CPU s [259.25]
s3a-r9-rta-song workers CPU s [191.23, 138.79, 50.02, 45.28, 37.86, 35.36, 35.31, 35.28]
s3a-r9-rtaw2-song workers CPU s [248.36, 196.41]
