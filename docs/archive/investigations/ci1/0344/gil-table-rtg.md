GIL lines 6318; dumps per Worker: W0 93, W1 87, W2 87, W3 87, W4 87, W5 87, W6 87, W7 87

| Worker | acquisitions | wait <0.1 | <1 | <5 | <20 | <50 | >=50 ms | longest wait ms | held (sampled) n | held <0.1 | <1 | <5 | <20 | <50 | >=50 ms | longest hold ms | mean hold ms |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| W0 | 13758390 | 13745608 | 12758 | 24 | 0 | 0 | 0 | 3.74 | 954871 | 953930 | 193 | 320 | 428 | 0 | 0 | 17.955 | 0.0059 |
| W1 | 182656 | 174823 | 7802 | 29 | 2 | 0 | 0 | 13.38 | 47611 | 47584 | 25 | 2 | 0 | 0 | 0 | 1.585 | 0.0047 |
| W2 | 304044 | 293950 | 10051 | 42 | 1 | 0 | 0 | 6.535 | 73364 | 73337 | 26 | 1 | 0 | 0 | 0 | 2.82 | 0.0048 |
| W3 | 152300 | 144574 | 7710 | 16 | 0 | 0 | 0 | 3.905 | 45358 | 45336 | 21 | 1 | 0 | 0 | 0 | 2.22 | 0.0047 |
| W4 | 202744 | 193059 | 9663 | 22 | 0 | 0 | 0 | 4.67 | 58531 | 58495 | 35 | 1 | 0 | 0 | 0 | 1.155 | 0.0046 |
| W5 | 1897652 | 1881827 | 15786 | 37 | 1 | 0 | 1 | 89.095 | 220162 | 220062 | 98 | 2 | 0 | 0 | 0 | 1.63 | 0.0027 |
| W6 | 160034 | 152303 | 7709 | 22 | 0 | 0 | 0 | 3.92 | 46075 | 46054 | 20 | 1 | 0 | 0 | 0 | 2.87 | 0.0047 |
| W7 | 171220 | 163341 | 7841 | 36 | 2 | 0 | 0 | 14.04 | 48022 | 47992 | 29 | 1 | 0 | 0 | 0 | 1.52 | 0.0053 |

all Workers: acquisitions 16829040; wait histogram [16749485, 79320, 228, 6, 0, 1]; waits >= 0.1 ms: 79555 = 0.473% of the acquisitions
W0: waits >= 0.1 ms 12782 of 13758390 acquisitions (0.093%), longest single wait 3.74 ms

top-held APIs (entries over all dumps: count / longest ms), W0 and W1..W7:
- W0: > WaitMessage 686 / 17.955; > Sleep 13 / 0.295; > VirtualAlloc 8 / 0.455; > CreateThread 7 / 0.420; > CryptQueryObject 5 / 0.225; > ReadFile 5 / 0.240; > WriteFile 4 / 0.125; guest returned 4 / 0.165; call_guest 4 / 2.250; > CreateDIBSection 3 / 2.170
- W1..W7: switch_to 954 / 2.870; idle wait 909 / 2.820; > LeaveCriticalSection 728 / 1.380; idle wait over 348 / 1.630; > VirtualAlloc 294 / 0.100; > PostMessageW 284 / 0.100; > EnterCriticalSection 273 / 0.905; < EnterCriticalSection 175 / 0.760; > CreateDIBSection 171 / 0.230; < LeaveCriticalSection 124 / 0.905
