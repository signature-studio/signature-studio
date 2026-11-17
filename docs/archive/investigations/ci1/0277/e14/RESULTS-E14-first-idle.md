# E14 の first idle — A = E13 の page（5053、d0447902）対 B = P + V7 の page（5063、page-e14-PV7）、bundle は両方 E13 の本番 `fl-engine-ci-0363b050`（ci1、2026-11-07 05:54〜06:04 machine date）

**事実（先頭）**: 実エンジン・実ブラウザ（Chrome 156 / Firefox 158〔Xvfb :77、WebGL あり〕）、この機械、headless、模擬入力。依頼者の機械は 0 件。単独・冷間・各 run 新しいプロファイル・A と B を交互（奇数組は A→B、偶数組は B→A）。各 run の入口は `wait-quiet.mjs`（load ≤8・I/O 圧 0・swap の入出力 0 を待つ。待ちは各 run の `wait.txt` と `steps.log` の行に MemAvailable・swap とともに残る）。開始前: ps で他の Chrome/Firefox/Wine/clang/wasm-opt/node の Export は 0 本、`vmstat` の si/so は 0。**全 16 組（Chrome 10・Firefox 6）が数えた run**（開始 load は Chrome 0.70〜2.52、Firefox 1.01〜1.79、I/O 圧は全て 0.00、他の Chrome は全 run で 0 本〔`chrome-count` の観測〕、MemAvailable 14.8〜23.6 GB、swap 使用 2.88〜2.92 GB・入出力 0）。

## Chrome 10 組（A = E13、B = P + V7。first idle ms）

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

## Firefox 6 組（A = E13、B = P + V7。first idle ms）

| 組 | A | B | B − A | 開始 load（A / B） | 開始 I/O 圧 avg10（A / B） | 終了 load（A / B） | webglRenderer |
|---|---|---|---|---|---|---|---|
| 1 | 3941 | 4115 | +174 | 1.03 / 1.01 | 0.00 / 0.00 | 1.10 / 1.57 | llvmpipe, or similar |
| 2 | 3887 | 4393 | +506 | 1.37 / 1.44 | 0.00 / 0.00 | 1.47 / 1.37 | llvmpipe, or similar |
| 3 | 4208 | 4103 | -105 | 1.35 / 1.22 | 0.00 / 0.00 | 1.32 / 1.33 | llvmpipe, or similar |
| 4 | 4271 | 4002 | -269 | 1.35 / 1.33 | 0.00 / 0.00 | 1.60 / 1.35 | llvmpipe, or similar |
| 5 | 4205 | 4039 | -166 | 1.60 / 1.60 | 0.00 / 0.00 | 1.65 / 1.95 | llvmpipe, or similar |
| 6 | 4764 | 4111 | -653 | 1.46 / 1.79 | 0.00 / 0.00 | 1.54 / 1.73 | llvmpipe, or similar |

**Firefox の結果**: 中央値 A 4206 ms（3887〜4764）対 B 4107 ms（4002〜4393）、**組ごとの差（B − A）の中央値 -136 ms、平均 -85.5 ± 161.2（SE）、4/6 組が負**（符号反転の厳密検定、両側 p = 0.625）。基準（E13 比 +100 ms 以内）を満たす。

## 注意（`desk.mode` と preflight）
- B の page（P）は beacon の `desk.mode` を出さない（`None`。A は `gl`）。`verify-bundle.sh preflight` は『Firefox の `webglRenderer` が非 null かつ `desk.mode = gl`』を要求するので、**B の Firefox の preflight は rc 2 で『FAILED: no WebGL in Firefox』と出たが、これは `desk.mode` の欄が P の page で空になるための基準の不一致で、WebGL が無いのではない**（`webglRenderer` は `llvmpipe, or similar`、A も B も同じ）。連鎖は止めずに Firefox の 6 組を走らせた。`report` の『同じ mode どうしの比較』の表は A（gl）と B（None）が別の行になって差が出ないので、本書の対の表を使う。
- B の run で P の Worker が作られたことは、netlog に `desk-worker-gHF_7GvZ.js` の取得があること（A の run には無い）で見た。P が WebGL で提示している間の `present` の率などは first idle の run では見ていない（pure で見る）。
- 外れ値: 無い（Chrome の最大 A 4,933〔組 6〕・B 4,660、Firefox の最大 A 4,764〔組 6〕・B 4,393）。外れ値が出たときの追加 6 組と段階時刻は不要だった。
- 未確認: 依頼者の機械、warm（同じプロファイルの 2 回目）の first idle、I/O が混む窓。

置き場所: `0277/e14/data-first-idle/`（`idle-chrome.tsv`、`idle-firefox.tsv`、`preflight.txt`）。run ごとの生データは `lift/out-ci1/kit/out/idle-*`。
