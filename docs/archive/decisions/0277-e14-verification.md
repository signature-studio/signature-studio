# ci1 → PM 決定 0277（E14）: E14 の最終の組 P + V7（`page-e14-PV7`、port 5063、bundle は E13 の本番 `fl-engine-ci-0363b050`）の実ブラウザ確認 — 非計時の確認（15 シーン・File ×100・Export・alpha7）、first idle（E13 比、Chrome 10 組・Firefox 6 組）、純再生 pure（E13 対 P + V7、Chrome 6 組）、G9-a の test bundle の Export（参考）

## 先頭の 3 点（実エンジン・実ブラウザ Chrome 156 / Firefox 158〔Xvfb :77、WebGL あり、llvmpipe〕、この機械・headless・模擬入力。**依頼者の機械は 0 件**。代役・再生・推測なし。各値に load と I/O 圧 avg10、swap の入出力を併記し、計時の run は『開始時 load ≤8・I/O 圧 0・swap の入出力 0』の run だけ数えた〔first idle は全 16 組〔32 run〕、pure は 12 run が数えた run〕）
1. **測ったこと**: PM の検証済みの木（`lift/out-pmchk-e14f/tree/app` = main の app + gpu1 の present Worker P〔0311〕+ V7）の page を ci1 の kit で `vite build` し（`page-e14-PV7`、`vite.kit.config.ts` の追加以外は木と byte 一致、P 単独の page との差は `desk-worker.ts` の +4 / −1 だけ）、E13 の本番 bundle `fl-engine-ci-0363b050`（R2 から読み取りで取得した物）と組み合わせた。A = E13 の page（5053、d0447902）対 B = P + V7（5063）で: 非計時の確認（Chrome 15 シーン、File ×100、Export、alpha7）、first idle（Chrome 10 組・Firefox 6 組、交互）、純再生 pure（Chrome 6 組・12 run、交互）。別に、gpu1 の port 4969 の G9-a の test bundle の Export を 1 本（参考。製品の bundle ではない）。
2. **結論**:
   - **first idle は基準（E13 比 +100 ms 以内）を両ブラウザで満たす**: Chrome の中央値 A 4,591 / B 4,570 ms、組ごとの差（B − A）の中央値 −54 ms、平均 −46 ± 47（SE）、7/10 が負、両側 p 0.38。Firefox は 4,206 / 4,107 ms、差の中央値 −136 ms、平均 −86 ± 161（SE）、4/6 が負、p 0.63。外れ値なし。P の page は first idle を遅らせていない。
   - **純再生 pure は P + V7 が時計を実時間に近づけ、書き込み率と underrun は区別できない**: 仮想 s/実 s 0.656 → 0.912、present/s 13.6 → 29.4、lagCuts/s 18.2 → 3.8（いずれも p 0.002）、音楽 10 s の bin の壁時計 14〜17 s → 約 10.4〜11.1 s。書き込み率（10〜60 s）0.912 → 0.937（p 0.35）、60 s の underrun quanta 7.5 → 8.5（p 0.65）、30〜60 s は両方 0。**代償**: Worker スレッド全体（engine Worker + P Worker）の CPU が 60 s で 51.0 → 73.5 s（+44%）。**基準が動いている**: 同じ A（E13）が 01:47 の E13 の pure より遅いので、A 対 B の同窓の差だけを読む。
   - 非計時の確認はすべて通った: Chrome 15/15 シーン ok、File ×100 = 100/100、Export の WAV は 22,863,828 B・sha256 `381cdb5a…f979` で参照と一致、alpha7 の画は E13 と 0 画素差（主窓 @0,0 1920x1080）。
   - **G9-a の test bundle（参考）**: 完走、`fle: unsupported` 0 行、g9-diag 2 行、WAV の sha256 は `94c93666…4090`（基準と違う。差は Balance の自動化 clip の内側とその終端の尾だけ〔alpha1 の解析、0310 §6〕）。Wine の参照との照合待ちで G9 は未到達。
3. **未確認**: 依頼者の機械（0 件）。**e2e（`test:e2e` preview → dev → worker）は未実施**（mt3 の build の後の PM の info を待つ。手順は `E2E-INVENTORY.md`）。Firefox の 15 シーン・File ×100・Export・alpha7、lat（入力の true T）、5 分再生、warm の first idle は未実施。Esc の late の原因は gpu1 の領域。

## 1 判定基準（手で作成。`report.py` の判定表は P の page が beacon に `desk.mode` を出さず A と B が別の mode の行になるため、改修〔`desk.mode` が None で `webglRenderer` が非 null の run を gl とみなす〕後の出力を `RESULTS-E14.md` に置いた）
| 基準 | 値 | 判定 |
|---|---|---|
| 停止なし（Chrome の 15 シーン、d7 は意図した終了） | 15/15 ok | yes（Firefox は未実施） |
| Export の WAV が参照と一致 | 22,863,828 B、sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` | yes |
| File ×100 | Chrome 100/100（Firefox 未実施） | yes |
| first idle ≤ E13 + 100 ms（Chrome、WebGL） | 中央値 B − A = −21.5 ms（組ごとの差の中央値 −54 ms）、10 組 | yes |
| first idle ≤ E13 + 100 ms（Firefox、WebGL あり） | 中央値 B − A = −99.5 ms（組ごとの差の中央値 −136 ms）、6 組 | yes |
| 純再生が E13 より悪化しない | 書き込み率・underrun は区別できない、仮想 s/実 s と present は改善 | yes（代償: Worker の CPU +44%） |
| alpha7（E13 と画素比較） | 0 画素差 | yes |

## 2 page（B3 = P + V7）
### B3 = P + V7（`page-e14-PV7`。03:38 machine date、nice 19 の vite build 1 回、ブラウザなしで作成）
- 木: `lift/out-pmchk-e14f/tree/app`（PM が検証: tsc 4 構成 rc 0、vitest 1,320 passed / 8 skipped / 0 failed）。bundle は E13 の本番 `fl-engine-ci-0363b050` のまま（5053 と同じ `assets-fl-engine-ci-0363b050`、manifest は 200・3,098 B・sha256 `57817472d5a0ea2d…` で同一）。サーバー: **5063**（`exec ./serve.sh 5063 fl-engine-ci-0363b050 e14-PV7`）。A は 5053（E13、d0447902）のまま。
- 木とページの差: `diff -rq lift/out-pmchk-e14f/tree/app kit/page-e14-PV7/app -x node_modules -x dist` は `vite.kit.config.ts` の追加だけ。**P 単独の `page-e14-P` との差は `src/engine/desk-worker.ts` の 1 ファイルだけ**（`diff` で +4 / −1: 『アニメーション frame の scene 要求を frame の処理の後の task で出す』コメント 1 行と `MessageChannel` 2 行を足し、`L.requestScene()` を `requestAfterFrame.port2.postMessage(0)` に替えた）。
- dist の sha256（served-same = 5063 から curl で取った bytes と一致。6 ファイルすべて一致）:

| file | bytes | sha256 |
|---|---|---|
| `index.html` | 3,736 | `84667441857a1e76ed4950c4d222f4538cfa5e7bbb3771646c91e98115694275` |
| `static/index-V7sD7FGQ.js`（main chunk） | 219,706 | `951188b14a74e5a6e0590af996c04f5400f422c91bd9397bd6b5aeafd556292c` |
| `static/desk-worker-gHF_7GvZ.js`（P の present Worker + V7） | 12,737 | `92ad2a66125e78c03a93116b829d3633506447a0fb035b04b0d4e50675cf86d9`（`page-e14-P` の `desk-worker-CEZo7ZuQ.js` は 12,662 B・`5e50ce8d…`） |
| `static/worker-QQJ6NzDL.js` | 47,841 | `c19672a042aa44d662599635d87522fb111f125c23bdd44edfa6648cb5b62399`（`page-e14-P` と同一） |
| `static/fle-out.worklet-CbgfZo7e.js` | 7,234 | `15ddab9adde905618a977fd81b05c38427203bf5653f033c3cc7914a33b7637a`（全 page 共通） |
| `static/index-bY1HsUKC.css` | 12,822 | `ff678f3853d353cadfa3019f40bc07a5fa6c7848d83a219a2fe2bed6464e6cb9`（全 page 共通） |

### B3 の計時でない確認（03:39〜03:53、Chrome 156、5063、bundle は E13 の本番。実エンジン、この機械、模擬入力。依頼者の機械は 0 件。load はこの間 mt3 の冷間 build が走っていたので 1.4〜8.1 で、参考値）
| 項目 | 結果 |
|---|---|
| 15 シーン（Chrome） | **15/15 ok**（d7 は意図した終了）。first idle の値は load 1.4〜8.1 の中の単独 run で参考（4,025〜5,902 ms） |
| File メニュー ×100（d8） | **100/100** |
| Export（Ctrl+R、参照プロジェクト） | **22,863,828 B・sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` で参照と一致**（319 s、load 5.2 → 3.0） |
| alpha7（主窓の配置と画） | 主窓 `TFruityLoopsMainForm 'FL Studio 2026' @0,0 1920x1080`、見える窓 1。**画は E13（`out-archive-e13/alpha7/`）と 0 画素差**（`main-1920x1080.png`・`scenario-alpha7-look.png` とも） |

置き場所: `0277/e14/RESULTS-PV7-functional.md`、`0277/e14/data-pv7/`。first idle・pure・lat は未実施（mt3 の build が終わった後の PM の info を待つ）。

## 3 first idle（A = E13 対 B = P + V7）

**事実（先頭）**: 実エンジン・実ブラウザ（Chrome 156 / Firefox 158〔Xvfb :77、WebGL あり〕）、この機械、headless、模擬入力。依頼者の機械は 0 件。単独・冷間・各 run 新しいプロファイル・A と B を交互（奇数組は A→B、偶数組は B→A）。各 run の入口は `wait-quiet.mjs`（load ≤8・I/O 圧 0・swap の入出力 0 を待つ。待ちは各 run の `wait.txt` と `steps.log` の行に MemAvailable・swap とともに残る）。開始前: ps で他の Chrome/Firefox/Wine/clang/wasm-opt/node の Export は 0 本、`vmstat` の si/so は 0。**全 16 組（Chrome 10・Firefox 6）が数えた run**（開始 load は Chrome 0.70〜2.52、Firefox 1.01〜1.79、I/O 圧は全て 0.00、他の Chrome は全 run で 0 本〔`chrome-count` の観測〕、MemAvailable 14.8〜23.6 GB、swap 使用 2.88〜2.92 GB・入出力 0）。

### Chrome 10 組（A = E13、B = P + V7。first idle ms）

| 組 | A | B | B − A | 開始 load（A / B） | 開始 I/O 圧 avg10（A / B） | 終了 load（A / B） | webglRenderer |
|---|---|---|---|---|---|---|---|
| 1 | 4645 | 4637 | -8 | 1.17 / 0.99 | 0.00 / 0.00 | 1.52 / 1.14 | ANGLE (Google, Vulkan 1.3.0 (SwiftShader |
| 2 | 4541 | 4544 | +3 | 0.89 / 0.70 | 0.00 / 0.00 | 1.20 / 0.89 | ANGLE (Google, Vulkan 1.3.0 (SwiftShader |
| 3 | 4522 | 4421 | -101 | 1.10 / 1.34 | 0.00 / 0.00 | 1.46 / 1.28 | ANGLE (Google, Vulkan 1.3.0 (SwiftShader |
| 4 | 4609 | 4414 | -195 | 1.22 / 1.00 | 0.00 / 0.00 | 1.63 / 1.22 | ANGLE (Google, Vulkan 1.3.0 (SwiftShader |
| 5 | 4486 | 4596 | +110 | 1.38 / 1.11 | 0.00 / 0.00 | 1.32 / 1.17 | ANGLE (Google, Vulkan 1.3.0 (SwiftShader |
| 6 | 4933 | 4639 | -294 | 2.42 / 1.17 | 0.00 / 0.00 | 2.34 / 2.63 | ANGLE (Google, Vulkan 1.3.0 (SwiftShader |
| 7 | 4601 | 4504 | -97 | 2.34 / 2.21 | 0.00 / 0.00 | 2.21 / 2.40 | ANGLE (Google, Vulkan 1.3.0 (SwiftShader |
| 8 | 4581 | 4571 | -10 | 2.52 / 2.36 | 0.00 / 0.00 | 2.29 / 2.52 | ANGLE (Google, Vulkan 1.3.0 (SwiftShader |
| 9 | 4665 | 4568 | -97 | 2.29 / 2.11 | 0.00 / 0.00 | 2.31 / 2.01 | ANGLE (Google, Vulkan 1.3.0 (SwiftShader |
| 10 | 4434 | 4660 | +226 | 2.07 / 2.01 | 0.00 / 0.00 | 1.98 / 2.07 | ANGLE (Google, Vulkan 1.3.0 (SwiftShader |

**Chrome の結果**: 中央値 A 4591 ms（4434〜4933）対 B 4570 ms（4414〜4660）、**組ごとの差（B − A）の中央値 -54 ms、平均 -46.3 ± 46.6（SE）、7/10 組が負**（符号反転の厳密検定、両側 p = 0.381）。基準（E13 比 +100 ms 以内）を満たす。

### Firefox 6 組（A = E13、B = P + V7。first idle ms）

| 組 | A | B | B − A | 開始 load（A / B） | 開始 I/O 圧 avg10（A / B） | 終了 load（A / B） | webglRenderer |
|---|---|---|---|---|---|---|---|
| 1 | 3941 | 4115 | +174 | 1.03 / 1.01 | 0.00 / 0.00 | 1.10 / 1.57 | llvmpipe, or similar |
| 2 | 3887 | 4393 | +506 | 1.37 / 1.44 | 0.00 / 0.00 | 1.47 / 1.37 | llvmpipe, or similar |
| 3 | 4208 | 4103 | -105 | 1.35 / 1.22 | 0.00 / 0.00 | 1.32 / 1.33 | llvmpipe, or similar |
| 4 | 4271 | 4002 | -269 | 1.35 / 1.33 | 0.00 / 0.00 | 1.60 / 1.35 | llvmpipe, or similar |
| 5 | 4205 | 4039 | -166 | 1.60 / 1.60 | 0.00 / 0.00 | 1.65 / 1.95 | llvmpipe, or similar |
| 6 | 4764 | 4111 | -653 | 1.46 / 1.79 | 0.00 / 0.00 | 1.54 / 1.73 | llvmpipe, or similar |

**Firefox の結果**: 中央値 A 4206 ms（3887〜4764）対 B 4107 ms（4002〜4393）、**組ごとの差（B − A）の中央値 -136 ms、平均 -85.5 ± 161.2（SE）、4/6 組が負**（符号反転の厳密検定、両側 p = 0.625）。基準（E13 比 +100 ms 以内）を満たす。

### 注意（`desk.mode` と preflight）
- B の page（P）は beacon の `desk.mode` を出さない（`None`。A は `gl`）。`verify-bundle.sh preflight` は『Firefox の `webglRenderer` が非 null かつ `desk.mode = gl`』を要求するので、**B の Firefox の preflight は rc 2 で『FAILED: no WebGL in Firefox』と出たが、これは `desk.mode` の欄が P の page で空になるための基準の不一致で、WebGL が無いのではない**（`webglRenderer` は `llvmpipe, or similar`、A も B も同じ）。連鎖は止めずに Firefox の 6 組を走らせた。`report` の『同じ mode どうしの比較』の表は A（gl）と B（None）が別の行になって差が出ないので、本書の対の表を使う。
- B の run で P の Worker が作られたことは、netlog に `desk-worker-gHF_7GvZ.js` の取得があること（A の run には無い）で見た。P が WebGL で提示している間の `present` の率などは first idle の run では見ていない（pure で見る）。
- 外れ値: 無い（Chrome の最大 A 4,933〔組 6〕・B 4,660、Firefox の最大 A 4,764〔組 6〕・B 4,393）。外れ値が出たときの追加 6 組と段階時刻は不要だった。
- 未確認: 依頼者の機械、warm（同じプロファイルの 2 回目）の first idle、I/O が混む窓。

置き場所: `0277/e14/data-first-idle/`（`idle-chrome.tsv`、`idle-firefox.tsv`、`preflight.txt`）。run ごとの生データは `lift/out-ci1/kit/out/idle-*`。

## 4 純再生 pure（A = E13 対 B = P + V7、Chrome 6 組）

**事実（先頭）**: 実エンジン・実 Chrome 156、この機械、headless、模擬入力。依頼者の機械は 0 件。bundle は両方 E13 の本番 `fl-engine-ci-0363b050`、page だけが違う（B は `desk-worker.ts` が P の present Worker）。参照プロジェクトの zip を開き Space で 60 s、10 s 窓。A と B を交互（prev・new・new・prev・prev・new・new・prev・prev・new・new・prev）。各 run の入口は `wait-quiet.mjs`（load ≤8・I/O 圧 0・swap の入出力 0）。**12 run すべてが数えた run**（開始 load 0.87〜3.70、I/O 圧 0.00、他の Chrome 0 本〔観測〕、MemAvailable 23.5〜25.4 GB、swap 使用 2.87〜2.88 GB・入出力 0）。表の全文と生データ: `data/P3-chrome.md`、`data/p3-chrome-*`。

### 中央値（数えた run 6 本ずつ、10〜60 s の窓。p は exact permutation の両側）
| 量 | A（E13） | B（P + V7） | B − A | p |
|---|---|---|---|---|
| **仮想 s/実 s** | 0.656 | **0.912** | +0.255 | 0.002 |
| **present/s** | 13.6 | **29.4** | +15.9 | 0.002 |
| lagCuts/s | 18.2 | 3.8 | −14.5 | 0.002 |
| UI idles/s | 273 | 748 | +475 | 0.002 |
| 音楽 0〜10 s の bin の壁時計（s） | 14.1 | 10.4 | −3.7 | 0.002 |
| 音楽 10〜20 s の bin の壁時計 | 14.0 | 10.4 | −3.6 | 0.002 |
| 音楽 20〜30 s の bin の壁時計 | 17.1 | 11.1 | −6.0 | 0.002 |
| 音楽 30〜40 s の bin の壁時計 | 13.6（n=5） | 12.6 | −1.0 | 0.071 |
| 書き込み率（10〜60 s の平均） | 0.912 | 0.937 | +0.025 | 0.348（区別できない） |
| 60 s の underrun quanta | 7.5 | 8.5 | +1.0 | 0.654（区別できない） |
| 30〜60 s の underrun | 0 | 0 | 0 | 1.0 |
| episodes 0〜60 s | 2.0 | 2.5 | +0.5 | 0.732 |
| 音楽位置で揃えた distinct frames/壁 s（0〜10・10〜20・20〜30・30〜40 s） | 0.938・0.857・0.803・0.758 | 0.953・0.887・0.828・0.786 | +0.015・+0.029・+0.025・+0.028 | 0.14・0.24・0.42・0.58（区別できない） |
| **Worker スレッド全体の CPU（60 s。engine Worker + P Worker）** | 51.0 s | **73.5 s** | **+22.6 s** | 0.002 |

### 読み（事実と区別する）
- **事実**: P + V7 は仮想時計を実時間に近づけ（0.66 → 0.91。B の 10〜20 s 窓は 0.98）、present を 2 倍強（13.6 → 29.4/s）にし、lag の切り捨てを 1/5 にした。音楽の 10 s ぶんの壁時計が 14〜17 s から約 10.4〜11.1 s に縮み（実時間に近い）、60 s の再生で音楽 50 s 前後まで進む（A は 40 s 前後。B は 40〜50 s の bin まで完走した run が 6 本、A は 0 本）。**書き込み率と underrun は区別できない**（underrun は両側とも先頭 0〜10 s の 8 quanta 前後で、30〜60 s は 0）。**代償**: Worker スレッド全体の CPU が 60 s で +22.6 s（+44%。P の Worker の分。gpu1 の測定の『エンジン Worker 0.89 + P 0.27 コア』と同じ向き）。
- **基準が動いている**: 同じ A（E13、同じ page）の値が、01:47〜02:09 の E13 の純再生（開始 load 1.0〜3.4）の E13 側（仮想 s/実 s 0.811、present 18.7/s、書き込み率 0.992）より今回は遅い（0.656、13.6/s、0.912）。PM が記す『03:00 以降に基準が遅い』（VM のメモリの変動の時期。原因未確認）と同じ向き。A と B は同じ窓の中で交互なので、A 対 B の比較は成り立つが、**絶対値を過去の run と比べてはいけない**。
- 未確認: Esc の late など入力の T（pure は入力なし。gpu1 の ABBA の領域）、lat（時間のため未実施）、実 GPU、依頼者の画面の Hz。

### 4.1 `p3-win.py` の全文（表の原文）
### P3 decider, chrome: old (previous) vs new, windows of 10 s from the play mark; counted = start load ≤8 and I/O pressure avg10 = 0

| run | bundle | counted | other Chrome browsers at the play mark / at +60 s / max | waited for the quiet window (s) | load start → end | I/O avg10 start → end | Worker CPU s (60 s) = CPU/wall x 60 | main CPU s | underrun quanta 0-60 s | episodes 0-60 s | engine gap frames 0-60 s | **underrun 30-60 s** | write rate mean 10-60 s | consumed rate mean 10-60 s | UI idles/s (10-60 s) | presents/s | lagCuts/s | virtual s per real s | stopped |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | prev | yes | 0 / 0 / 0 | 2 | 0.9 → 2.7 | 0.00 → 0.00 | 50.3 | 0.0 | 8 | 2 | 852 | **0** | 0.962 | 1.000 | 284.9 | 15.8 | 19.3 | 0.707 | False |
| 2 | prev | yes | 0 / 0 / 0 | 7 | 3.2 → 3.1 | 0.00 → 0.00 | 50.1 | 0.0 | 2 | 1 | 170 | **0** | 0.910 | 1.000 | 298.0 | 15.0 | 18.4 | 0.675 | False |
| 3 | prev | yes | 0 / 0 / 0 | 2 | 2.9 → 2.9 | 0.00 → 0.00 | 51.4 | 0.0 | 8 | 2 | 852 | **0** | 0.879 | 1.000 | 262.4 | 12.4 | 16.3 | 0.636 | False |
| 4 | prev | yes | 0 / 0 / 0 | 7 | 2.0 → 2.0 | 0.00 → 0.00 | 51.2 | 0.0 | 14 | 3 | 1534 | **0** | 0.884 | 1.000 | 283.4 | 12.9 | 16.3 | 0.652 | False |
| 5 | prev | yes | 0 / 0 / 0 | 2 | 2.0 → 1.4 | 0.00 → 0.00 | 51.2 | 0.0 | 3 | 1 | 298 | **0** | 0.914 | 1.000 | 237.8 | 13.2 | 18.1 | 0.652 | False |
| 6 | prev | yes | 0 / 0 / 0 | 7 | 3.7 → 4.1 | 0.00 → 0.00 | 50.7 | 0.0 | 7 | 2 | 724 | **0** | 0.916 | 1.000 | 246.9 | 14.0 | 18.5 | 0.661 | False |
| 1 | new | yes | 0 / 0 / 0 | 2 | 2.5 → 3.6 | 0.00 → 0.00 | 70.5 | 0.0 | 4 | 2 | 340 | **0** | 0.953 | 0.997 | 856.7 | 29.9 | 0.2 | 0.978 | False |
| 2 | new | yes | 0 / 0 / 0 | 2 | 3.3 → 3.4 | 0.00 → 0.00 | 70.7 | 0.0 | 2 | 1 | 170 | **0** | 0.947 | 0.995 | 859.6 | 30.1 | 0.9 | 0.973 | False |
| 3 | new | yes | 0 / 0 / 0 | 2 | 2.9 → 2.8 | 0.00 → 0.00 | 74.3 | 0.0 | 7 | 3 | 638 | **0** | 0.918 | 0.995 | 717.9 | 29.4 | 4.7 | 0.886 | False |
| 4 | new | yes | 0 / 0 / 0 | 2 | 2.6 → 2.3 | 0.00 → 0.00 | 74.7 | 0.0 | 10 | 1 | 1194 | **0** | 0.877 | 0.995 | 730.2 | 29.2 | 4.6 | 0.864 | False |
| 5 | new | yes | 0 / 0 / 0 | 2 | 1.3 → 3.7 | 0.00 → 0.00 | 74.0 | 0.0 | 10 | 3 | 1022 | **0** | 0.927 | 1.000 | 759.2 | 29.3 | 4.9 | 0.911 | False |
| 6 | new | yes | 0 / 0 / 0 | 2 | 3.7 → 4.2 | 0.00 → 0.00 | 73.0 | 0.0 | 18 | 3 | 2046 | **0** | 0.947 | 0.995 | 736.6 | 29.5 | 3.0 | 0.913 | False |

Per 10 s window (median over the counted runs; n in the first column). underrun quanta / episodes / engine gap frames (differences) and write rate (Unlock frames per real-time frame, 1.0 = real time) / consumed rate (worklet):

| window | bundle | n | underrun quanta | episodes | engine gap frames | write rate | consumed rate | UI idles/s | presents/s | lagCuts/s | virtual s per real s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 6 | 8 | 2 | 788 | 0.98 | 1.00 | 1135.1 | 13.8 | 7.1 | 0.77 |
| 0-10 s | new | 6 | 8 | 2 | 830 | 0.98 | 1.00 | 1516.1 | 20.1 | 0.0 | 0.96 |
| 10-20 s | prev | 6 | 0 | 0 | 0 | 0.97 | 1.00 | 238.3 | 13.2 | 19.0 | 0.64 |
| 10-20 s | new | 6 | 0 | 0 | 0 | 0.97 | 1.00 | 889.8 | 29.3 | 0.5 | 0.98 |
| 20-30 s | prev | 6 | 0 | 0 | 0 | 0.97 | 1.00 | 302.3 | 14.8 | 17.1 | 0.71 |
| 20-30 s | new | 6 | 0 | 0 | 0 | 0.99 | 1.00 | 755.8 | 29.5 | 1.3 | 0.93 |
| 30-40 s | prev | 6 | 0 | 0 | 0 | 0.97 | 1.00 | 200.0 | 14.0 | 19.6 | 0.59 |
| 30-40 s | new | 6 | 0 | 0 | 0 | 0.98 | 1.00 | 576.9 | 29.7 | 7.1 | 0.79 |
| 40-50 s | prev | 6 | 0 | 0 | 0 | 0.92 | 1.00 | 196.9 | 12.6 | 18.2 | 0.63 |
| 40-50 s | new | 6 | 0 | 0 | 0 | 0.92 | 1.00 | 614.6 | 29.6 | 6.7 | 0.86 |
| 50-60 s | prev | 6 | 0 | 0 | 0 | 0.75 | 1.00 | 397.2 | 12.9 | 13.7 | 0.75 |
| 50-60 s | new | 6 | 0 | 0 | 0 | 0.79 | 0.98 | 991.4 | 29.2 | 0.3 | 1.03 |

Music-matched windows (the same music in both bundles): the music position is the virtual clock since the play mark (gpu1 matched-music.py); the bins are complete 10 s bins of music that each run reached within its 60 s of wall time. wall = the wall time the bin took; distinct frames = Unlock coverage per wall second (1.0 = real time); Unlocks / wall s; underrun quanta and episodes of the page and the engine gap frames over the wall interval of the bin (the 10 s snapshots and the 1 Hz engine reports, linearly interpolated). Median over the counted runs.

| music bin | bundle | n | wall time of the bin (s) | distinct frames per wall s | Unlocks per wall s | underrun quanta | episodes | engine gap frames |
|---|---|---|---|---|---|---|---|---|
| 0-10 s | prev | 6 | 14.1 | 0.938 | 10.53 | 8 | 2 | 788 |
| 0-10 s | new | 6 | 10.4 | 0.953 | 10.46 | 8 | 2 | 830 |
| 10-20 s | prev | 6 | 14.0 | 0.857 | 10.45 | 0 | 0 | 0 |
| 10-20 s | new | 6 | 10.4 | 0.887 | 10.48 | 0 | 0 | 0 |
| 20-30 s | prev | 6 | 17.1 | 0.803 | 10.45 | 0 | 0 | 0 |
| 20-30 s | new | 6 | 11.1 | 0.828 | 10.73 | 0 | 0 | 0 |
| 30-40 s | prev | 5 | 13.6 | 0.758 | 8.77 | 0 | 0 | 0 |
| 30-40 s | new | 6 | 12.6 | 0.786 | 10.65 | 0 | 0 | 0 |
| 40-50 s | new | 6 | 10.1 | 0.775 | 9.02 | 0 | 0 | 0 |

Old vs new at the same music position (counted runs; exact permutation test; only bins both bundles reached):

| music bin | measure | old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|
| 0-10 s | wall time of the bin (s) | 14.119 (6) | 10.428 (6) | -3.690 | 0.002 |
| 0-10 s | distinct frames per wall s | 0.938 (6) | 0.953 (6) | +0.015 | 0.139 |
| 0-10 s | underrun quanta | 7.500 (6) | 8.500 (6) | +1.000 | 0.654 |
| 0-10 s | episodes | 2.000 (6) | 2.500 (6) | +0.500 | 0.732 |
| 0-10 s | engine gap frames | 788.000 (6) | 830.000 (6) | +42.000 | 0.639 |
| 10-20 s | wall time of the bin (s) | 13.995 (6) | 10.432 (6) | -3.563 | 0.002 |
| 10-20 s | distinct frames per wall s | 0.857 (6) | 0.887 (6) | +0.029 | 0.240 |
| 10-20 s | underrun quanta | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 10-20 s | episodes | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 10-20 s | engine gap frames | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 20-30 s | wall time of the bin (s) | 17.121 (6) | 11.144 (6) | -5.977 | 0.002 |
| 20-30 s | distinct frames per wall s | 0.803 (6) | 0.828 (6) | +0.025 | 0.416 |
| 20-30 s | underrun quanta | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 20-30 s | episodes | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 20-30 s | engine gap frames | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| 30-40 s | wall time of the bin (s) | 13.600 (5) | 12.608 (6) | -0.992 | 0.071 |
| 30-40 s | distinct frames per wall s | 0.758 (5) | 0.786 (6) | +0.028 | 0.582 |
| 30-40 s | underrun quanta | 0.000 (5) | 0.000 (6) | +0.000 | 1.000 |
| 30-40 s | episodes | 0.000 (5) | 0.000 (6) | +0.000 | 1.000 |
| 30-40 s | engine gap frames | 0.000 (5) | 0.000 (6) | +0.000 | 1.000 |

Old vs new (exact two-sided permutation test on the means; p < 0.05 would be a difference). Left: counted runs only (the rule); right: all runs (sensitivity, whatever the load at the start):

| measure | old median (n) | new median (n) | new − old (medians) | p | all runs: old median (n) | new median (n) | new − old | p |
|---|---|---|---|---|---|---|---|---|
| underrun quanta 0-60 s | 7.500 (6) | 8.500 (6) | +1.000 | 0.654 | 7.500 (6) | 8.500 (6) | +1.000 | 0.654 |
| underrun quanta 30-60 s (heavy interval) | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 | 0.000 (6) | 0.000 (6) | +0.000 | 1.000 |
| episodes 0-60 s | 2.000 (6) | 2.500 (6) | +0.500 | 0.732 | 2.000 (6) | 2.500 (6) | +0.500 | 0.732 |
| engine gap frames 0-60 s | 788.000 (6) | 830.000 (6) | +42.000 | 0.677 | 788.000 (6) | 830.000 (6) | +42.000 | 0.677 |
| write rate mean 10-60 s | 0.912 (6) | 0.937 (6) | +0.025 | 0.348 | 0.912 (6) | 0.937 (6) | +0.025 | 0.348 |
| Worker CPU s (60 s) | 50.955 (6) | 73.530 (6) | +22.575 | 0.002 | 50.955 (6) | 73.530 (6) | +22.575 | 0.002 |
| UI idles/s (10-60 s) | 272.927 (6) | 747.919 (6) | +474.992 | 0.002 | 272.927 (6) | 747.919 (6) | +474.992 | 0.002 |
| presents/s (10-60 s) | 13.584 (6) | 29.441 (6) | +15.857 | 0.002 | 13.584 (6) | 29.441 (6) | +15.857 | 0.002 |
| lagCuts/s (10-60 s) | 18.248 (6) | 3.774 (6) | -14.474 | 0.002 | 18.248 (6) | 3.774 (6) | -14.474 | 0.002 |
| virtual s per real s (10-60 s) | 0.656 (6) | 0.912 (6) | +0.255 | 0.002 | 0.656 (6) | 0.912 (6) | +0.255 | 0.002 |

## 5 非計時の確認の表（`RESULTS-E14.md` から。first idle の値は mt3 の build が走っていた時間の run で参考）
### 5.1 シーン
| browser | scene | result | first idle ms | File ×100 | load, I/O pressure (start→end) |
|---|---|---|---|---|---|
| chrome | d1-sampler-envelope | ok | 5230 | - | load 1.4→1.7, io 0.0→0.0 |
| chrome | d2-browser-folder | ok | 4649 | - | load 1.7→1.7, io 0.0→0.0 |
| chrome | d3-settings-all-tabs | ok | 4761 | - | load 1.7→3.5, io 0.0→0.0 |
| chrome | d4-playlist-middle-drag | ok | 4888 | - | load 3.5→3.4, io 0.0→0.0 |
| chrome | d5-mixer-front | ok | 4867 | - | load 3.4→3.6, io 0.0→0.1 |
| chrome | d6-playlist-close | ok | 4973 | - | load 3.6→3.4, io 0.1→0.0 |
| chrome | d7-main-close | ok (intended exit) | 4950 | - | load 3.4→3.2, io 0.0→0.0 |
| chrome | d8-file-menu-100 | ok | 4644 | 100/100 | load 3.2→3.4, io 0.0→0.0 |
| chrome | d9-keys | ok | 4025 | - | load 3.4→5.0, io 0.0→0.0 |
| chrome | d10-pianoroll-f12 | ok | 5657 | - | load 5.0→5.7, io 0.0→0.0 |
| chrome | d11-menu-hover-escape | ok | 5250 | - | load 5.7→7.8, io 0.0→0.0 |
| chrome | add-menu-simple-fade | ok | 5252 | - | load 8.1→7.0, io 0.0→0.0 |
| chrome | picker-cost | ok | 4630 | - | load 6.8→5.2, io 0.0→0.0 |
| chrome | picker-f8-fade | ok | 4546 | - | load 7.0→6.8, io 0.0→0.0 |
| chrome | plus-more-plugins | ok | 5902 | - | load 7.8→8.1, io 0.0→0.0 |


The scenes ran as two parallel jobs (Chrome and Firefox), so the first idle values of this table are reference values (the load is next to each).
### 5.2 alpha7
result ok; load 3.0→3.2, io 0.0→0.0; windows visible at the end: 1; windows larger than 1800 px wide:
- `14 TFruityLoopsMainForm 'FL Studio 2026' @0,0 1920x1080 z0`
- expected after alpha7: the main window @0,0 1920x1080 (previous df0512b0: @-5,-6 1930x1092). Screenshots: `main-1920x1080.png, scenario-alpha7-look.png`
### 5.3 Export
result ok; load 5.2→3.0, io 0.0→0.0; seconds 319
- WAV `serina1-serum2.wav`: 22863828 B, sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`; expected 22863828 B / `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`: MATCH

## 6 G9-a の test bundle の Export（参考。製品の bundle ではない。gpu1 の port 4969、非計時）

**事実（先頭）**: 実エンジン（mt3 の test bundle `fl-engine-test-g9a-28f6f631` = Balance を実モジュールにした物。製品の bundle ではなく、R2 に無い）を実 Chrome 156 で動かした 1 本。page は gpu1 の port 4969（main の app）、assets は `lift/out-gpu2/assets-g9a`。Export の scenario は ci1 の `scn-export.json` に engine log の保存（`saveLog`）を足した `scn-export-g9a.json`。この機械、headless、模擬入力。依頼者の機械は 0 件。開始前: 他の Chrome/Firefox/Wine/clang/wasm-opt/node の Export は 0 本、`vmstat` の最後の 4 サンプルの si/so は 0、`wait-quiet` の swap の入出力 0/0（Mem available 16.9 GB、Swap 使用 3.0 GB）、load 0.52、I/O 圧 0。first idle は測っていない。

| 判定 | 結果 |
|---|---|
| (1) 完走 | **完走**（scenario ok〔no stop〕、310 s。Start から WAV まで約 74 s〔file の時刻 04:57:10 → 04:58:24〕。同じ手順の P + V7 の page + E13 の run は 319 s、約 80 s〔load 5.2→3.0〕） |
| (2) `fle: unsupported` / abort | `fle: unsupported` は **0 行**（engine log 318 行）。`abort kind: (not a runtime message)` の 1 行は、アプリ起動前の `== stop: returned from entry` の標準の要約に含まれる文字列（function entries 0、`a4: init_count=0 of ?`）で abort ではない。E13 の engine log は保存していないので、同じ形かの比較は未実施 |
| (3) WAV | サイズ **22,863,828 B（基準と同じ）**、フレーム数 **5,715,911（基準と同じ）**、ヘッダ 44 byte は同一。**sha256 `94c9366687076a410f48c6fbe1485e8e22f6b528592fcc5ceba2d665d1554090` は基準 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` と違う** |
| (4) g9-diag | **2 行**: `LoadLibraryW "C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Effects\Fruity Balance\Fruity Balance_x64.dll" -> 0x6800000`、`GetProcAddress(0x6800000, "CreatePlugInstance") -> 0x6abdde0` |

### 基準（P + V7 の page + E13 の本番 bundle、同じ手順、sha `381cdb5a…f979`、`out/export/export.wav`）との WAV の差（numpy、再計算ではなく今回の比較）
- 差のあるフレーム: **640,995 / 5,715,911（11.21%）**。最初の差は **25.79 s**（フレーム 1,137,368）、最後は 119.78 s。**0〜20 s、90〜100 s、120〜129 s はビット一致**。
- 10 s 区間別の差の RMS（差のあるフレームの割合）: 20〜30 s 4,350（9.7%）、30〜40 s 6,802（22.9%）、40〜50 s 4,511（15.3%）、50〜60 s 3,928（18.9%）、60〜70 s 3,637（17.8%）、70〜80 s 3,566（18.2%）、80〜90 s 1,618（3.5%）、100〜110 s 5,584（15.9%）、110〜120 s 6,730（23.2%）。
- 全体: 差の RMS 3,987（L 4,189・R 3,773）、信号の RMS 20,413 に対し −14.2 dB。最大差 65,534（|差| > 32,768 の標本が 31,585 個 = 符号が反転した標本を含む）。ピークは両方 32,767。差のあるフレームの |g9a| / |基準| の平均は 0.998（レベルはほぼ同じ）、相関は L 0.842・R 0.873。
- **読み（未確認）**: Balance の実モジュールが呼ばれて出力が変わった（読み込みの g9-diag 2 行と sha の変化が根拠）。ただし変化が**大きい**（−14.2 dB、符号の反転を含む）うえレベルが変わらず相関が下がるので、単純な利得・パンの変化ではなく、波形の中身（遅れ・位相・クリップ付近の挙動など）が変わった可能性がある。**正しい変化かどうかを判定する参照が無い**（Wine の FL は Balance を含む全プラグインを読むので、Balance だけの差は取れない）。遅れか内容の変化かの切り分けは未実施。
- **未確認**: この変化が Balance の GUI/処理の正しさを示すか、Balance が参照プロジェクトのどのトラックに入っているか、E13 の本番で同じ run が 381cdb5a…f979 を再現する条件の再確認（今回の基準は 03:51 の P + V7 page の run の WAV）。

置き場所: `lift/out-git/ci1-results/0277/e14/g9a-export/`（`export.json`、`report.json/md`、`engine-log.log`、画像）。WAV 本体は `lift/out-ci1/kit/out/export-g9a/export.wav`（22.9 MB）。

### 追記（決定 0314d の作業中。alpha1 の解析〔0310 §6、PM が取り込み済み 8c4b72a6〕との突き合わせ）
上の『読み（未確認）』のうち「波形の中身（遅れ・位相・クリップ付近の挙動など）が変わった可能性」は、alpha1 の解析で次のように説明された（私の測定ではなく alpha1 の numpy の実測と FLP の読み。sha256 は alpha1 と PM も `sha256sum` で確認）: 差のあるフレーム 640,995 のうち 626,143 は Balance の自動化 clip 256 個（Insert 1）の内側、14,852 は各 clip の終端から 0〜137 フレーム（最大 3.11 ms）後の尾で、clip の無い区間の差は 0。基準の WAV は 30.83% のフレームに ±32767 の標本があり、符号の反転（最大差 65,534）は 16 ビットの整数化の clip の効果と読める。**よって WAV どうしの rms・最大差・相関は Balance の利得の正しさの尺度にならない**（私の『遅れ・位相』の可能性は取り下げ、利得の自動化 + clip で説明できる）。変化が正しいかは Wine の参照（Balance のみ有効の Export）との照合で判定する（未実施）。

## 7 e2e の棚卸し
`E2E-INVENTORY.md`（CI が自動で走らせる物、kit に無い物、最終 page を E13 の bundle で通す e2e の手順、P の経路で engine-desk 系の読み出しが通るかの読み）。**e2e は未実施**。

## 8 自己申告の逸脱
1. **自動開始の誤り（直前の担当、E13 の pure）**: 『gpu1 の連鎖が終わったら自動で開始する』待機を PM の go なしに張った（E13 の docs の逸脱 4 に記録済み。23:05 に張り、23:07 に止めた）。以後、計時・Chrome の開始は PM の info だけで、今回の E14 の first idle と pure は PM の『go』を受けてから始めた。
2. **`bash` という名前のツールを呼んだ**（E13 の docs の逸脱 6 の後にもう 1 回）。実行されたのは `echo test` だけ、変更なし。
3. **preflight が rc 2 で『FAILED: no WebGL in Firefox』と出た**（P の page は beacon に `desk.mode` を出さず、基準の `desk.mode = gl` と不一致）。Firefox は WebGL ありだった（`webglRenderer` = llvmpipe）。連鎖は止めずに Firefox の 6 組を走らせた。事後に `beacon.py` と `report.py` を、『`desk.mode` が None で `webglRenderer` が非 null かつ engine が `present D` の run を WebGL の run とみなす』ように直した（kit のツール。`verify-bundle.sh` は編集していない）。
4. **kit の run の入口 `wait-quiet.mjs` を変更**（PM の 04:16 の規則: swap の入出力 0 と MemAvailable の記録を足した）。走行中の run が無い時に編集した。
5. **G9-a の test bundle の Export の『読み（未確認）』の取り下げ**: 04:59 に『単純な利得・パンではなく波形の中身が変わった可能性』と書いたが、alpha1 の解析（0310 §6）で、差は Balance の自動化 clip の内側とその終端の尾だけ、符号反転は 16 ビット clip の効果と読めるとされた。`g9a-export/RESULTS-g9a-export.md` に取り下げの追記をした。
6. **使い終わった preview server を止めた**（自分が起動した 5052 = E12、5062 = P + V4）。残りは 5053・5061・5063。
7. **Firefox の 15 シーン・File ×100・Export は E14 では未実施**（時間と mt3 の build との重なりを避けたため。gpu1 の機能確認〔Firefox 158 + P + V7 の first idle 4.3 s ほか〕はある）。


## PM の判断と取り込み（2026-11-07 06:36、docs のみ。製品の差分は push していない）

ci1 の E14 の実ブラウザ確認（first idle、pure、非計時の確認、e2e の棚卸し）を取り込んだ。生の `*-audio.json`（12 本、約 3.5 MB）・PNG・`*.log` は取り込まない（`*.log` は `.txt` に改名）。**実エンジン・実 Chrome 156 / Firefox 158（WebGL あり）・この機械・headless・模擬入力。依頼者の機械は 0 件。**

PM の確認: `RESULTS-E14-first-idle.md`・`RESULTS-E14-pure.md` を自分で読み、first idle（Chrome 10 組: P + V7 の中央値 4,570 ms 対 E13 の 4,591 ms、組ごとの差の中央値 −54 ms、7/10 が負、p 0.38／Firefox 6 組: 4,107 ms 対 4,206 ms、差の中央値 −136 ms、p 0.63）と pure（Chrome 6 組・12 run、全て数えた run: 仮想 s/実 s 0.656 → 0.912、present/s 13.6 → 29.4、lagCuts/s 18.2 → 3.8、いずれも p 0.002。書き込み率 0.912 → 0.937 と 60 s の underrun 7.5 → 8.5 は区別できない。Worker スレッド全体の CPU は +44%〔51.0 → 73.5 s〕）が本文の表と一致することを確認した。**注意**: pure の A（E13）は 01:47 の E13 の pure（仮想 0.811、present 18.7/s、書き込み率 0.992）より遅く、基準の機械の状態が動いている（03:00 以降、原因未調査。VM のメモリが 20〜35 GB で変動）。同じ窓の A 対 B の差だけを使い、絶対値を以前の run と比べない。Firefox は WebGL が有り（llvmpipe）、本文の『Firefox は 2D』という表記は preflight の criteria（P の page が beacon に `desk.mode` を出さない）による誤読で、ci1 が `report.py` を直して訂正済み。

**判断**:
1. **E14 の最終の組は P + V7（`desk-worker.ts` のみ、+4/−1）**。V4 は採らない（決定 0311 の PM の追記）。
2. first idle の関門と pure の関門は満たした。**E14 はまだ push しない**。残る関門: (a) ci1 の e2e（`page-e14-PV7`、preview → dev → worker。mt3 の Filter の build が終わってから、手順は `E2E-INVENTORY.md`）、(b) Firefox 158 での p3 と pv7s の対（各 2 本、DISPLAY=:77。以前の単独 run で仮想 s/実 s 0.765・再生位置が約 11% 遅く原因未調査）、(c) PM の最終の再検証（検証用の木 `lift/out-pmchk-e14f/tree`: tsc 4 構成 rc 0、vitest 1,320 passed / 8 skipped / 0 failed）、(d) deploy の CI が走っていないこと。
3. CPU の代償（Worker スレッド +44%）は、P2 の入力の遅延と実時間性の改善と引き換えに受け入れるが、P3（音切れ）が改善した証拠にはならない（書き込み率・underrun は区別できない）ので、P3 の判定には使わない。
4. Firefox の 15 シーン・File ×100・Export は E14 では未実施。e2e の窓か別の静かな窓で足す。
5. G の判定は変えない（厳密な読みで G0、字義の読みで G4。G9 未到達、G10・G11 未着手）。依頼者の機械は 0 件。

