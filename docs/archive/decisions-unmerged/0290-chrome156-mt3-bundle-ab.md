# 0290 Chrome 156 A/B: mt3's real bundles (0293: E10 + branch hints, 27,570 hints) vs plain, on ci1's page-9fdc492f (main 9fdc492f, no engine)
Real engine runs (HeadlessChrome 156, the reference project opened, Space, V8 profile of the Worker, window 30-50 s after Space). Order plain-1, hint-1, plain-2, hint-2 (16:33:44-16:54:08). Other Chrome drivers at the start/end of every run: 0. Start loads 7.36, 7.43, 12.17, 8.71 (> 8: plain-2 and hint-2 are reference).

| run | engine | start -> end load | blocks 30-50 s | write rate r | v (virtual s / real s) | idle % | (program) ms/block | mixer thread ms/block | FL main ms/block | f_b34aca0+f_b34eaf0 ms (30-50 s) | A / other wasm | first idle ms |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| plain-1 | 4d0019aaa3ee | 7.36 -> 7.83 | 193 | 0.896 | 0.577 | 13.2 | 5.99 | 47.14 | 21.03 | 2219 | 0.1615 | 6063 |
| hint-1 | 267214838ef6 | 7.43 -> 12.25 | 166 | 0.771 | 0.518 | 15.6 | 13.86 | 44.64 | 24.73 | 1571 | 0.1233 | 5498 |
| plain-2 | 4d0019aaa3ee | 12.17 -> 9.56 | 207 | 0.961 | 0.577 | 13.9 | 6.13 | 42.47 | 19.67 | 2019 | 0.1475 | 5811 |
| hint-2 | 267214838ef6 | 8.71 -> 12.01 | 160 | 0.743 | 0.505 | 16.2 | 13.08 | 46.97 | 25.75 | 1686 | 0.1328 | 5072 |
| mean plain | | | 200 | 0.929 | 0.577 | 13.6 | 6.06 | 44.8 | 20.4 | 2119 | 0.1545 | 5937 |
| mean hint | | | 163 | 0.757 | 0.512 | 15.9 | 13.5 | 45.8 | 25.2 | 1629 | 0.1281 | 5285 |
| hint vs plain | | | -18% | -18% | -11% | +2.3 pt | x2.2 | +2% | +24% | -23% | -17% | -11% |
