# ci1 → PM 決定 0277（最終）: E7b の本番 bundle `fl-engine-ci-d9c160aa`（Worker 51f102d8、ページ = HEAD 0b228624）の実ブラウザ確認 — 検証キット、停止なし、Export 一致、first idle、File ×100、再生中の underrun の窓の定義

## 先頭の 3 点（実エンジン・実ブラウザ Chrome 156 / Firefox 158、この機械・headless・模擬入力。**依頼者の機械は 0 件**。代役・再生・推測なし。Firefox の値は全て WebGL あり（`DISPLAY=:77` の Xvfb、beacon の `webglRenderer` = llvmpipe、`desk.mode` = gl）。各値に load と I/O 圧 avg10 を併記）
1. **測ったこと**:
   - 配信バケット `fl-web-assets` の bundle を**読み取りで取得**し（`wrangler r2 object get`、局所の再ビルドなし）、manifest の全ファイルの size と sha256 を照合（wasm `d6e3b018accb…` 142,587,221 B、glue `8dc2abf01efc…`、seed `f7726113ae34…`、dll `2f494419c1ad…` の全て一致。比較の相手 `fl-engine-ci-ff5ee429` も一致）。同じページ（CI 42137885173 の headSha 0b228624 の app）で新旧を配信して比べた。
   - 11 シーン + more plugins + Add メニューの fade + F8 picker の 2 種（Chrome・Firefox の各 15）、File メニュー ×100、alpha7 の主窓の位置、first idle（単独 run・冷間・旧新交互）、Export（Ctrl+R）、再生 60 s、Mixer ヘッダードラッグの Worker CPU/移動。
2. **結論**: 決定 0277 の判定基準 4 つを**満たした**。
   - 停止なし: 15 シーン × 2 ブラウザ、再生 60 s × 2 ブラウザ、Export で停止 0（d7 は意図した終了）。
   - Export の WAV が参照と一致: 22,863,828 B、sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`。
   - first idle が ff5ee429 に対し +500 ms 以内（新が速い）: Chrome の静かな run（load ≤12・I/O <1、各 6 run）で旧 5,996 → 新 5,403 ms（−593）、Firefox（各 6 run）で旧 5,276 → 新 4,900 ms（−376）。
   - File メニュー ×100: Chrome 100/100、Firefox 100/100。
   - alpha7: 主窓は @0,0 1920x1080、スクリーンショットで UI は崩れていない（ff5ee429 と同じ見え方）。
   - **決着していない物**: 再生中の underrun は Chrome の旧新 3 組すべてで新が多い向き（新 946 / 444 / 852、旧 116 / 14 / 486 quanta）だが、範囲が重なり n=3、結論にしない（第 5 節）。Mixer ドラッグは Chrome で差なし、Firefox は判定できない。
3. **未確認**: オーナーの機械（0 件）。Firefox の旧 ff5ee429 の再生中 underrun（未実施）。再生中の episode ごとの最長（beacon に無い）。mixer スレッド側の CPU（Worker busy は主 Worker のみ）。warm 起動。load ≤8・I/O 0 の静かな窓での再生比較（この文書の数値は load 4〜15 の run）。

## 1 判定基準（新 bundle、run の load と I/O 圧は各表）
| criterion | value | pass |
|---|---|---|
| Firefox has WebGL (preflight) | preflight: webglRenderer llvmpipe, or similar desk.mode gl first idle 4724 | yes |
| No stop (11 scenes + more, both browsers, play, export; d7 is the intended exit) | stops: none; not ok: none | yes |
| Export WAV equals the reference | 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` (expected 22863828 B, `381cdb5acf891a16…`) | yes |
| First idle ≤ previous + 500 ms (chrome, WebGL runs) | all runs: new median 5958 ms, previous 6396 ms, difference -438 ms; quiet runs only (load ≤12, I/O avg10 <1; n new 6 / prev 6): new 5403, previous 5996, difference -593 ms | yes |
| First idle ≤ previous + 500 ms (firefox, WebGL runs) | all runs: new median 4900 ms, previous 5276 ms, difference -376 ms; quiet runs only (load ≤12, I/O avg10 <1; n new 4 / prev 3): new 4945, previous 5297, difference -352 ms | yes |
| File menu ×100 (chrome) | 100/100 | yes |
| File menu ×100 (firefox) | 100/100 | yes |

## 2 11 シーン + 追加（新 bundle、Chrome・Firefox、各行の load と I/O 圧）
| browser | scene | result | first idle ms | File ×100 | load, I/O pressure (start→end) |
|---|---|---|---|---|---|
| chrome | d1-sampler-envelope | ok | 5995 | - | load 10.2→10.1, io 0.0→0.0 |
| chrome | d2-browser-folder | ok | 9743 | - | load 16.2→23.8, io 0.0→0.0 |
| chrome | d3-settings-all-tabs | ok | 8883 | - | load 23.8→28.1, io 0.0→0.0 |
| chrome | d4-playlist-middle-drag | ok | 6959 | - | load 27.7→26.8, io 0.0→0.1 |
| chrome | d5-mixer-front | ok | 5543 | - | load 26.8→23.2, io 0.1→0.0 |
| chrome | d6-playlist-close | ok | 5565 | - | load 23.2→21.7, io 0.0→0.0 |
| chrome | d7-main-close | ok (intended exit) | 5587 | - | load 21.7→17.9, io 0.0→0.1 |
| chrome | d8-file-menu-100 | ok | 4884 | 100/100 | load 17.9→13.2, io 0.1→0.0 |
| chrome | d9-keys | ok | 5476 | - | load 13.2→16.9, io 0.0→0.0 |
| chrome | d10-pianoroll-f12 | ok | 5912 | - | load 10.1→9.9, io 0.0→0.0 |
| chrome | d11-menu-hover-escape | ok | 6133 | - | load 9.4→10.2, io 0.0→0.0 |
| chrome | add-menu-simple-fade | ok | 6380 | - | load 14.2→10.8, io 0.3→0.0 |
| chrome | picker-cost | ok | 4220 | - | load 8.7→11.7, io 0.1→0.2 |
| chrome | picker-f8-fade | ok | 4389 | - | load 10.8→8.7, io 0.0→0.1 |
| chrome | plus-more-plugins | ok | 8749 | - | load 16.2→14.2, io 0.0→0.4 |
| firefox | d1-sampler-envelope | ok | 4905 | - | load 9.9→10.6, io 0.0→0.0 |
| firefox | d2-browser-folder | ok | 8928 | - | load 17.9→25.5, io 0.0→0.0 |
| firefox | d3-settings-all-tabs | ok | 8962 | - | load 27.0→27.3, io 0.2→0.0 |
| firefox | d4-playlist-middle-drag | ok | 5805 | - | load 27.3→23.8, io 0.2→0.0 |
| firefox | d5-mixer-front | ok | 5373 | - | load 23.8→20.6, io 0.0→0.0 |
| firefox | d6-playlist-close | ok | 4852 | - | load 19.8→17.2, io 0.0→0.0 |
| firefox | d7-main-close | ok (intended exit) | 4962 | - | load 8.2→8.5, io 0.0→0.0 |
| firefox | d8-file-menu-100 | ok | 6463 | 100/100 | load 15.6→18.7, io 0.0→0.0 |
| firefox | d9-keys | ok | 6524 | - | load 18.7→25.3, io 0.0→0.0 |
| firefox | d10-pianoroll-f12 | ok | 5389 | - | load 10.5→8.9, io 0.0→0.0 |
| firefox | d11-menu-hover-escape | ok | 5237 | - | load 10.0→10.2, io 0.2→0.0 |
| firefox | add-menu-simple-fade | ok | 4836 | - | load 11.7→13.1, io 0.1→2.0 |
| firefox | picker-cost | ok | 6904 | - | load 11.6→12.5, io 0.4→0.0 |
| firefox | picker-f8-fade | ok | 4493 | - | load 13.1→11.6, io 2.0→0.4 |
| firefox | plus-more-plugins | ok | 5277 | - | load 8.3→11.7, io 0.3→0.1 |

The scenes ran as two parallel jobs (Chrome and Firefox), so the first idle values of this table are reference values (the load is next to each).

Firefox d7 (the intended exit): the first run lost the WebDriver session (`POST /session/d7a665e4-87dd-40c3-a2d0-27ec3f115412/execute/sync: 404 {"value":{"error":"invalid session id","message":"T`; at load 17.23 io / io 0.03, two jobs in parallel; the engine's exit was not read). Re-runs: 3, all `ok (the intended exit)`. Not reproduced; the cause of the first loss is unknown. The table shows the latest run; the first run is kept in `out/d7-ff-first-session-lost/`.

Invalid runs, kept aside in `out/invalid-noproj/` and NOT counted above: d1, d10, d11 (Chrome and Firefox) and the two d11 runs on the new and the previous bundle, taken while the kit served no `Sampler.flp` / `SHP_Sawing Synth.wav` (a 404 body was opened as the project; the engine stopped with a named CompareStringW stop on both bundles). They were re-run after the kit served the real files (rows above).

## 3 alpha7（Chrome、1920x1080 のゲスト）
result ok; load 11.6→13.7, io 0.1→1.6; windows visible at the end: 1; windows larger than 1800 px wide:
- `14 TFruityLoopsMainForm 'FL Studio 2026' @0,0 1920x1080 z0`
- expected after alpha7: the main window @0,0 1920x1080 (previous df0512b0: @-5,-6 1930x1092). Screenshots: `main-1920x1080.png, scenario-alpha7-look.png`
- スクリーンショット: `shots/alpha7-d9c160aa-main-1920x1080.png`（ツールバーが上端から始まり、ブラウザ枠・チャンネルラック・ミキサー（Master と Insert 1〜9）・プレイリスト（Track 1〜19）・右上の最小化/最大化/閉じるが全て見える）。

## 4 first idle（単独 run・冷間・毎回新しいプロファイル・旧新交互。run 番号の `q` は Chrome の静かな窓での取り直し（11:27、load 5.5・I/O 0））
| browser | bundle | run | first idle ms | load, I/O pressure at the start | at the end | quiet (load ≤12, I/O <1) | webglRenderer | desk.mode |
|---|---|---|---|---|---|---|---|---|
| chrome | prev | 1 | 6771 | 6.40 / io 0.18 | 6.99 / io 0.05 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 1 | 6618 | 6.99 / io 0.05 | 7.71 / io 0.08 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 2 | 5647 | 7.71 / io 0.08 | 7.45 / io 0.10 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 2 | 6378 | 7.45 / io 0.10 | 9.16 / io 0.21 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 3 | 9349 | 9.16 / io 0.21 | 12.58 / io 0.05 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 3 | 10723 | 12.58 / io 0.05 | 15.36 / io 0.00 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 4 | 9682 | 15.36 / io 0.00 | 18.30 / io 0.00 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 4 | 9766 | 18.30 / io 0.00 | 19.94 / io 0.00 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 5 | 9122 | 19.94 / io 0.00 | 20.26 / io 0.00 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 5 | 6269 | 20.26 / io 0.00 | 17.83 / io 0.00 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 6 | 6411 | 17.83 / io 0.00 | 16.89 / io 0.00 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 6 | 6414 | 16.89 / io 0.00 | 15.75 / io 0.00 | no (reference) | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 1q | 6243 | 4.76 / io 0.00 | 5.47 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 1q | 5457 | 5.47 / io 0.00 | 5.77 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 2q | 5349 | 5.77 / io 0.00 | 5.80 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 2q | 5349 | 5.80 / io 0.00 | 5.85 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 3q | 5529 | 5.85 / io 0.00 | 5.79 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 3q | 4830 | 5.79 / io 0.00 | 5.37 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | new | 4q | 4946 | 5.37 / io 0.00 | 5.60 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| chrome | prev | 4q | 5749 | 5.60 / io 0.00 | 5.49 / io 0.00 | yes | ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0 | gl |
| firefox | prev | 1 | 5786 | 14.89 / io 0.00 | 14.42 / io 0.09 | no (reference) | llvmpipe, or similar | gl |
| firefox | new | 1 | 4890 | 14.42 / io 0.09 | 13.21 / io 0.02 | no (reference) | llvmpipe, or similar | gl |
| firefox | new | 2 | 4778 | 13.21 / io 0.02 | 12.41 / io 0.00 | no (reference) | llvmpipe, or similar | gl |
| firefox | prev | 2 | 5254 | 12.41 / io 0.00 | 12.02 / io 0.08 | no (reference) | llvmpipe, or similar | gl |
| firefox | prev | 3 | 4721 | 12.02 / io 0.08 | 11.57 / io 0.03 | no (reference) | llvmpipe, or similar | gl |
| firefox | new | 3 | 4981 | 11.57 / io 0.03 | 10.80 / io 0.01 | yes | llvmpipe, or similar | gl |
| firefox | new | 4 | 4909 | 10.80 / io 0.01 | 9.35 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 4 | 4454 | 9.35 / io 0.00 | 8.84 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 5 | 5297 | 8.84 / io 0.00 | 8.64 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 5 | 4701 | 8.64 / io 0.00 | 8.53 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | new | 6 | 5718 | 8.53 / io 0.00 | 9.23 / io 0.00 | yes | llvmpipe, or similar | gl |
| firefox | prev | 6 | 5934 | 9.23 / io 0.00 | 9.36 / io 0.08 | yes | llvmpipe, or similar | gl |

| browser | mode | previous median (n) | new median (n) | difference |
|---|---|---|---|---|
| chrome | gl | 6396 (10) | 5958 (10) | -438 |
| firefox | gl | 5276 (6) | 4900 (6) | -376 |

## 5 Export と再生中の underrun

### 5.1 Export（Chrome、参照プロジェクトを開き Ctrl+R）
result ok; load 8.5→4.3, io 0.0→0.0; seconds 358
- WAV `serina1-serum2.wav`: 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`; expected 22863828 B / `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`: MATCH

### 5.2 再生 60 s と underrun の量の定義
**数えた量**: ページの `window.__app.audio.detail()` の `underrunQuanta`（AudioWorklet の 128 フレーム ≈ 2.9 ms の描画単位で、出力リングが空だった数。音のデバイスが鳴り始めてからの**累計**。`app/src/audio/underrun.ts`、`ring-layout.ts`）と `episodes`（連続した underrun の塊の数）。opt1 の audio-b.json の `detail.underrunQuanta` と**同じカウンタ**。

**窓の定義の違い**:
- opt1 の値（ff5ee429 の 60 s 再生で 1,603 / 1,565 quanta など）は**累計**で、プロジェクトを開く間のエンジンの停止（私の run では再生前に 1,375〜2,118 quanta・1〜2 episodes が既に付く）を含む。
- 私の『再生中』は、`playing-a`（Space の 4 s 後 + 10 s の再生、再生前の読みの約 19 s 後）の終わりの読みと、停止後の読みの**差**（約 52 s の再生）で、開く間の分を含まない。
- 累計で比べると私の値も旧 1,809 / 2,140 / 1,942、新 2,342 / 1,908 / 2,407 で opt1 の 1,600 台と同じ桁、**旧新は区別できない**。再生前の停止は旧の方が長い run が多い（旧 1,684 / 2,118 / 1,456、新 1,375 / 1,432 / 1,522）ため、累計では差が小さくなる。桁の違いは窓の違い。

Counter definition: `window.__app.audio.detail()` of the page: `underrunQuanta` = AudioWorklet render quanta (128 frames ≈ 2.9 ms) in which the output ring was empty, counted cumulatively from when the audio device began to play (page start), `episodes` = maximal runs of consecutive underrun quanta (`app/src/audio/underrun.ts`, `ring-layout.ts`). This is the same counter as `underrunQuanta` in opt1's audio-b.json `detail`. **cumulative** = the value at the end (includes the stall while the project is opened); **during playback** = the difference between the reading at the end of `playing-a` (≈19 s after the first reading, 4 s after Space + 10 s playing) and the reading after the stop (≈52 s of playback).

| run | bundle | load at start → end | Worker busy ms/s while playing (min / median / max) | cumulative at the end: quanta / episodes | before the playback (project-open stall) | **during playback: quanta / episodes** | gap frames the engine wrote itself during playback (`engineGapFrames` delta; the engine wrote silence for a hole) |
|---|---|---|---|---|---|---|---|
| chrome | new | 4.1 → 4.8 (io 0.0 → 0.0) | 679 / 760 / 862 | 2342 / 94 | 1375 / 1 | **946 / 90** | 113348 |
| chrome | previous ff5ee429 | 15.2 → 9.3 (io 0.0 → 0.0) | 819 / 878 / 900 | 1809 / 32 | 1684 / 1 | **116 / 30** | 12268 |
| chrome r2 | new | 8.8 → 8.6 (io 0.0 → 0.0) | 753 / 807 / 833 | 1908 / 66 | 1432 / 2 | **444 / 61** | 51586 |
| chrome r2 | previous ff5ee429 | 8.3 → 11.4 (io 0.0 → 0.0) | 885 / 898 / 908 | 2140 / 8 | 2118 / 1 | **14 / 5** | 1362 |
| chrome r3 | new | 10.4 → 8.8 (io 0.0 → 0.0) | 683 / 798 / 885 | 2407 / 86 | 1522 / 1 | **852 / 82** | 102004 |
| chrome r3 | previous ff5ee429 | 8.8 → 5.7 (io 0.0 → 0.0) | 745 / 824 / 894 | 1942 / 61 | 1456 / 1 | **486 / 60** | 57048 |
| firefox | new | 4.8 → 6.3 (io 0.0 → 0.0) | 706 / 866 / 991 | 3223 / 70 | 2538 / 1 | **685 / 69** | 81746 |
- 時間順は 新 r1（10:54）→ 旧 r1（11:13）→ 新 r2（11:16）→ 旧 r2（11:18）→ 新 r3（11:21）→ 旧 r3（11:23）。I/O 圧は全 run 0、load は 4〜15 で旧新が揃っていない（参考値）。
- Chrome の 3 組すべてで新が多い（946 vs 116、444 vs 14、852 vs 486）が、旧 r3（486）は新 r2（444）より多く、旧の run 間で 14〜486 と 35 倍動く。n=3 ずつで**結論にしない**。
- 『エンジンが自分で書いた gap フレーム』（`engineGapFrames` の差）は、エンジンが publish に間に合わず無音で埋めた量（decision 0211）。underrun の多い run で多い（新 113k / 52k / 102k、旧 12k / 1.4k / 57k）= 遅れはページ側のリングではなく**エンジン側**で起きている（読み）。
- 再生の Worker busy（CPU ÷ wall）は新が低い（新 679〜885 ms/s、旧 745〜908 ms/s）のに underrun は新が多い。underrun は再生の開始から約 10〜40 s 目（audio 時刻 63〜93 s）に集中する（Chrome r1: 10 s ごとに +0 / +228 / +389 / +308 / +21 quanta、Firefox: +0 / +80 / +267 / +338 / +0）。
- Firefox の再生は新 1 回だけ（load 3〜6）: Worker busy 706 / 866 / 991 ms/s（最小 / 中央 / 最大）、再生中 685 quanta・69 episodes。旧は未実施。
- **仮説（検証していない。私の読み）**: (H1) Worker busy は主 Worker の CPU だけで、音を作る別スレッド（ゲストの mixer スレッド）の遅れを含まない。(H2) 新は Worker が空く（IdleDriver の lead 待ちが増える）ため publish の周期が揃わず、重い区間で 1 周期遅れると hole になる。どちらも run の順序・時刻・ブラウザの背景負荷などの他の読みを排除していない。

## 6 Mixer ヘッダードラッグの Worker CPU / 移動（1920x1080、旧新交互）
**chrome** (1920x1080 guest)

| bundle | n | median ms/move (all) | median (quiet runs: load ≤12, I/O <1) | runs (round, ms, load and I/O pressure at the start; * = load >12, reference) |
|---|---|---|---|---|
| prev | 2 | 8.75 | 8.75 (n=2) | #1 8.75 (5.97 / io 0.00); #2 8.76 (6.81 / io 0.00) |
| new | 2 | 8.68 | 8.68 (n=2) | #1 8.69 (5.01 / io 0.00); #2 8.67 (8.29 / io 0.00) |

(the runs of 3 sessions, a / b / c = ab-firefox-a.tsv / ab-firefox-b.tsv / ab-firefox.tsv, in this order in time)

**firefox** (1920x1080 guest)

| bundle | n | median ms/move (all) | median (quiet runs: load ≤12, I/O <1) | runs (round, ms, load and I/O pressure at the start; * = load >12, reference) |
|---|---|---|---|---|
| prev | 8 | 9.86 | 7.82 (n=4) | #c1 7.78 (10.35 / io 0.00); #c2 7.86 (10.54 / io 0.00); #c3 13.36 (9.08 / io 0.00); #a1 5.75 (6.17 / io 0.00); #a2 10.81 (15.46 / io 0.00)*; #b1 11.28 (17.46 / io 0.00)*; #b2 13.13 (23.83 / io 0.00)*; #b3 8.91 (17.97 / io 0.01)* |
| new | 8 | 9.96 | 8.38 (n=3) | #c1 8.38 (11.81 / io 0.00); #c2 8.34 (9.43 / io 0.00); #c3 11.25 (14.27 / io 0.00)*; #a1 11.84 (5.23 / io 0.18); #a2 13.93 (18.04 / io 0.00)*; #b1 13.76 (22.04 / io 0.00)*; #b2 8.32 (23.28 / io 0.00)*; #b3 8.66 (14.09 / io 0.00)* |
- Chrome は差なし（−1 %）。Firefox は run ごとのばらつきが中央値の −42 〜 +35 % で、旧新の差より大きく**判定できない**。E7b の mixer CPU の効果（locshare・hyb +18）はこの測定では見えない。

## 7 自己申告の逸脱
1. **d1・d10・d11 の最初の run は無効だった（私の誤り）**。検証キットの `proj`（配信する `/assets/proj/`）に `Sampler.flp` と `SHP_Sawing Synth.wav` が無く、404 の本文（35 バイト）が『Sampler.flp』として開かれた。d11 は Chrome で新旧 bundle とも、Firefox で新 bundle が、`CompareStringW` の名前つき停止（exit 134）になり、d1・d10 は ok と出たが Sampler を操作していない無効 run だった。本物のファイル（git 管理の `docs/investigations/crash1/tools/Sampler-project/` と同一）を配信して再実行し、全て ok（d11 は 207 s → 27 s）。無効 run は表に数えていない。再発防止: `verify-bundle.sh proj` と、preflight の『4 ファイルが 200 とサイズで返る』検査（違えば rc 3 で停止）。
2. Firefox の d7（意図した終了）で WebDriver の `invalid session id` が 1 回（load 17.2、2 ジョブ並行）。同じ条件の再実行 3 回は全て ok（意図した終了）で、再現しなかった。原因は不明。
3. 局所の `.br`（品質 6）は私の作成物（転送サイズは本番と違う。内容の sha256 は manifest と一致）。
4. Firefox の Mixer 測定は 3 セッションに分けた（load 5〜29 の変動、他の作業者の job と PM の検証）。『差が無い』ではなく『判定できない』。
5. 旧新の再生比較は Chrome だけで、load が揃っていない（4〜15）。
6. `bash` ツールを今回も 1 回呼んだ（無いツール。`shell` で再実行）。`sleep` / `&` / `pkill -f` は使っていない。

## 8 検証キット（`ci1-results/0277/kit/`、README に順番と判定基準）
`verify-bundle.sh` のモード: fetch / proj / page / preflight / scenes / alpha7 / idle / export / play / ab / report / clean。`NEW`・`REF`（ページの git ref）・`WORKER` を環境変数で渡し、`PREV` は既定 ff5ee429。preflight は Firefox の beacon が `webglRenderer` 非 null かつ `desk.mode` = gl でなければ rc 2 で止まる。結果は `out/RESULTS.md`（1 つの表、各値に load と I/O 圧）。ブラウザテストは CI に入れない。

## PM 記入欄（ci1 は書かない）

### PM の判断と取り込み(2026-11-06 11:45、docs のみ)

取り込み: ci1 の最終(11:28〜11:34)の文書・結果表・キット(`kit/`)・データ(20 KB 以下のファイルのみ)を `docs/investigations/ci1/0277/` に置いた(alpha7 の画 1 枚を含む。他の画像と、400 KB 超のファイル、局所の bundle は入れない)。測定は Chrome 156 と Firefox 158(Xvfb :77、llvmpipe、desk.mode gl)の本番 E7b bundle `fl-engine-ci-d9c160aa`(Worker `51f102d8-a15e-4972-b2fc-0288705a224f`、ページ 0b228624)で、依頼者の機械・実 Windows は 0 件。
判断:
1. **E7b の本番 bundle は判定基準 4 つを満たした**(停止なし〔15 シーン × 2 ブラウザ、再生 60 s × 2 ブラウザ〕、Export の WAV が参照と一致〔22,863,828 B、sha256 `381cdb5a…f979`、ci1 自身の sha256sum〕、first idle が ff5ee429 以内〔新のほうが速い〕、File ×100 が Chrome・Firefox とも 100/100)。
2. first idle(単独、冷間、旧新交互): Chrome 静かな run(load ≤12・I/O <1、6 組ずつ)で旧 5,996 → 新 5,403 ms(−593)、Firefox(WebGL あり、n=6)5,276 → 4,900 ms(−376)。P1 の 10 s 以内は満たすが、依頼者の機械は未測定。
3. **再生中の underrun は旧新の差を結論にしない**(Chrome 3 組で新が多い向きだが範囲が重なる。ci1 の値は『再生中の差』、opt1 の `underrunQuanta` は『累計』で窓が違うだけ。累計で見ると旧新は同じ桁で重なる)。**決着は ci1 の `p3` モード(旧新交互、10 s 窓、開始時 load ≤8・I/O 圧 0 の run のみ)で行う**。
4. 数値の注意: 無効だった d1・d10・d11 の最初の run(キットの proj に Sampler.flp が無く FL のエラー窓 → CompareStringW で止まった。旧新とも同じ止まり方)は表に数えていない。この止まり方自体は決定 0284 の対象(記号の比較の `CompareStringW` が停止する)。
5. 未確認: Firefox の旧 bundle の再生、Mixer ドラッグ(Firefox)の旧新差、再生中の episode ごとの最長、E8 の bundle の確認(デプロイ後に同じキットで)。
