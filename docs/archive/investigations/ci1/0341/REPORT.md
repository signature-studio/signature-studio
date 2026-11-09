# RT-F ins1-lp diagnostic Export (g9mix) — 2026-11-07

Status: see also rtf-ins1lp/ engine-log.log

## Actually run

- Real Chrome 156 headless, page-rt1, scenario scn-export-g9c.json, RT-F bundle (fl-engine-ci-d45afb0e) manifest copy with FLE_DIAG_G9F=1 only.
- Export run 19:00:56–19:05:34 (WAV 22,919,504 bytes, sha256 df212eb06b6b9fa53b9ff8b7f608d397d51f9c1a7425d84fc2ea191803b4cf22).
- engine-log.log: 50,000 lines total. g9m lines 40,416 (exceeds 20,000 cap) — cap HIT. g9f lines 9,582 (also present in this log because the diag build counts both).

## g9m summary (from archive log, parsed by io counts)

- g9m entries: 10,104 (4 slots × 2,526 calls each).
- Last scum/expos (max): scum 6,633,478 / expos 5,781,510. Both are the same as max (ceiling reached at the end).
- Expos window 210,000–212,500: 0 calls (the first logged expos is 215,539 starting from scum 1,067,507; before this the diagnostic window had not yet flushed). So there is no call in 210,000–212,500 in this run.
- Insert 1 slots:
  - slot 0 Balance (k=2, slot 0x17679b40): 2,526 calls, nonzero-in 2,351. max in peak 2.34501, max outpost peak 2.34501. Sum peaks in 1589.553 / outpost 1327.214. first src8 = [0.351889, 0.204776, 0.34527, 0.201554, 0.334198, 0.200893, 0.317794, 0.202688], first dst8 = same (dry passthrough). Typical out_a8 == src8 (balance does not change samples).
  - slot 1 Filter (k=3, slot 0x17679cc0): 2,526 calls, nonzero-in 2,332. max in peak 2.34501, max outpost peak 1.06217. Sum in 1327.107 / outpost 236.174. first src8 = [0.351889, 0.204776, 0.34527, 0.201554, 0.334198, 0.200893, 0.317794, 0.202688]; first out_a8 = [0.0178409, 0.0282996, 0.0181006, 0.0283413, 0.0182106, 0.0282894, 0.0181769, 0.0281455] (Filter heavily attenuates vs input). Filter analysis compatible with observed RW audio behavior (Filter reduces level by ~-20 dB).
- Other slots:
  - k=0 slot 0x1767aec0 si=2 insert=2: 2,526 calls, all peaks 0 (silenced/silent skip; silence 176400->176400).
  - k=1 slot 0x17648490 si=2 insert=47: 2,526 calls, all peaks 0.

## Interpretation for the diagnostic

- The Insert 1 Balance (k=2) and Filter (k=3) are BOTH called from the same task 0x176730b0/0x17673130. Balance's out_after_first8 equals its input (passthrough), Filter's out_after_first8 is heavily attenuated. The Filter slot's outpost sum (236.17) is much lower than its input sum (1327.11).
- Filter output level vs Balance: this is the classic FLE path (also seen in Wine vs wasm differences).
