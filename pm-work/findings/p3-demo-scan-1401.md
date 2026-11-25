# インストーラの demo 46 件の走査(#1401 の前回の試行の記録)

実エンジン: `lift/out-app/bundle/raw/fl-Oz` を起動し、`Data\Demo projects\` の `.flp` 46 件を、1 件ずつ `tools/p3-audio-probe/measure.mjs` で開いて Space で 10 秒再生した(run 45608455537。job「作業 #1401」。`scan-demos.mjs` の 1 回の走査は 28 分)。

runner の値。この機械とは別枠。nproc 4、loadavg 0.70 1.39 1.43(報告時点。走査の途中の値は取っていない)。

判定: 結論は「選べない」。開けた 40 件はすべて `nonSilentQuanta` が 0。`notInInstaller` の件数は、素材の欠落ではなくエンジンの既定の探索(`.ZGR` を `.wav` の前に探す・`.NFO`・Plugin database・`*.*`・ユーザーのフォルダ)を多く含む。

この表は記録。判定(受け入れ)には入れない。

| # | 曲名 | error | notInInstaller | nonSilentQuanta | underrunDelta |
|---|---|---|---|---|---|
| 1 | Asher Postman - Future House | | 4860 | 0 | 0 |
| 2 | Astes - Bien Duro | | 16162 | 0 | 0 |
| 3 | Atthar Feat Mehran Abbasi & Elise Chantelle - Right Night feeling | | 5242 | 0 | 0 |
| 4 | Bombs Away ft KARRA - I'm Awake | | 12077 | 0 | 0 |
| 5 | Chan Gibbons x Ceebeaats - Time | | 4806 | 0 | 0 |
| 6 | Cuebur & Thandi Draai - Africa's Calling | | 3153 | 0 | 3391 |
| 7 | DEDZII - NIGHT TERROR | 開く途中で停止 | 66 | - | - |
| 8 | Edlan & Ella Noël - Song For You | | 3795 | 0 | 0 |
| 9 | Electroconductor - Inner Days - Original | | 1035 | 0 | 0 |
| 10 | Electroconductor - Inner Days - SH-1 Rework (feat. Maja Mannila) | | 1480 | 0 | 0 |
| 11 | Eric Kauffmann & NrgMind - Exoplanet | | 2302 | 0 | 3448 |
| 12 | Jay Cactus x Confz - Mic Check Ready | | 4545 | 0 | 3445 |
| 13 | LeriQ - SHINE | | 2856 | 0 | 0 |
| 14 | LollieVox & Lezina J - I'll Be Your Madonna | | 3771 | 0 | 0 |
| 15 | Mehdi Ebrahiminejad - Never Out | | 5944 | 0 | 0 |
| 16 | Mehran Abbasi - With You | 開く途中で停止 | 18 | - | - |
| 17 | Ookay - Thief | | 3165 | 0 | 0 |
| 18 | P.Priime - Use Me | | 3165 | 0 | 0 |
| 19 | Purps On The Beat Feat Luclover - Stroll Up | | 9693 | 0 | 0 |
| 20 | Sid De La Cruz - Secret Agent Rendezvous | | 2720 | 0 | 0 |
| 21 | NewStuff | インストーラの木に無い(起動せず) | 0 | - | - |
| 22 | Charles Deluxe - Performance Demo | | 1227 | 0 | 0 |
| 23 | Drumloop Masher | | 27 | 0 | 0 |
| 24 | nucleon - Complextreon | | 160 | 0 | 0 |
| 25 | nucleon - Housework | | 158 | 0 | 0 |
| 26 | nucleon - Steptime | | 463 | 0 | 3445 |
| 27 | SeamlessR - Bass Antics Performance | | 3135 | 0 | 0 |
| 28 | Electric Loop - The Ogun Dream | | 30 | 0 | 0 |
| 29 | nucleon - Fruity Delay 3 | | 1226 | 0 | 0 |
| 30 | nucleon - Harmless Galactic | | 23 | 0 | 0 |
| 31 | nucleon - Transient Processor | | 730 | 0 | 0 |
| 32 | Olbaid - Harmless Essentials | | 1640 | 0 | 0 |
| 33 | Olbaid - Sytrus Essentials | 開く途中で停止 | 44 | - | - |
| 34 | Square a Saw - Lightning (VEELA Vocals Resynthesized) | | 1677 | 0 | 0 |
| 35 | Square a Saw - Oxygen (Tevlo Vocals Resynthesized) | | 2985 | 0 | 0 |
| 36 | Synth_dfr - Overcome - Sytrus | 開く途中で停止 | 44 | - | - |
| 37 | Vocodex demo | | 180 | 0 | 3448 |
| 38 | nucleon - Razer Chroma | | 865 | 0 | 3201 |
| 39 | Astes - Raubana - LIFE | | 466 | 0 | 3133 |
| 40 | JuiceBass - SeamlessR - Menagerie | | 1967 | 0 | 3449 |
| 41 | Sacco - Expanding universe | | 156 | 0 | 0 |
| 42 | Sacco - Exploding Chips | | 156 | 0 | 0 |
| 43 | Sacco - FlashyWaves | | 156 | 0 | 0 |
| 44 | Sacco - Pumping objects | | 155 | 0 | 0 |
| 45 | Treben - Gimbal and Sinan - RawFL | | 2725 | 0 | 0 |
| 46 | Treben - Umziky - All the same | 開く途中で停止 | 153 | - | - |

「開く途中で停止」は、`Song info` の窓が出た後にエンジンが `exited` になったもの(error は「the engine is exited during the open」)。

## 最小の曲(`nucleon - Harmless Galactic`、notInInstaller 23)の内訳

- `Data\Patches\Packs\Legacy\Drums\Dance\Basic 808 Kick.ZGR`・Clap・HiHat・Snare の `.ZGR`(4 件。実物は `.wav`。エンジンが `.ZGR` を先に探す)。
- Plugin database の `.NFO` 5 件・`.Plugins.ini`・`*.*` の探索。
- `C:\Windows\Fonts\OpenSans-Regular.ttf`。
- ユーザーのフォルダ(Settings\Mapping\...・Settings\Skins\Default\StartSnd.wv・Presets)の探索。
- DLL は notInInstaller に入らない。この曲は DLL が 23 件欠け、「Problems loading the project」(`TMissingPluginsForm`)が出た。
