# 0303 (b)(d): 『Chrome の UI は node の 1.3〜2.0 倍遅い』は比較の取り方と node の tick の較正で説明がつく — 同じ窓・同じ仕事で比べると `f_863f60` は 1.07 倍、ミキサーは Chrome のほうが速い(0.6〜0.8 倍)、全体は 0.84〜1.12 倍(mt3)

## 0. 先頭の三点(実エンジンか代役か、実測か読みか)

1. **実エンジンの実行**: node 側はすべて**新規に実行**した実エンジン(E11 のテスト用 wasm `engine.abecc01c7b30` = 本番 `engine.49bbf63df894` と同じソース、node 24.19 = V8 13.6、`ref1-perf-short.g3` の仮想時計 G3)。**Chrome は 0 回起動した**(指示)。Chrome 側の値は**既存の cpuprofile**(opt1 の `ser-e16p`/`ser-e17p`、alpha1 の a14 の 6 本)の**再解析**で、新しい Chrome の実測ではない。代役・画像の再生・モックは**無い**。
2. **読み物・観察・推定の区別**: (i) V8 15.6 の既定値(`wasm_deopt` ほか)は **V8 のソース(github v8/v8 の 15.6-lkgr と 13.6-lkgr の `flag-definitions.h`)を読んだ値**で、稼働中の Chrome 156.0.8078.47 で確かめた値ではない。(ii) Chrome の wasm メモリの配置は、**他の agent が走らせている最中の Chrome の `/proc/PID/maps`・`smaps_rollup` を読んだ**(読むだけ。起動も操作もしていない)。(iii) 『オーバーヘッドを引いた値』『同じ仕事に直した全体比』は**測定値から組み立てた推定**(式を §4 に書いた)。
3. **未検証**: Chrome 15.6 の V8 の挙動の差(§5 の 9)、V8 sandbox(§5 の 8)、JSPI の差(§5 の 10)。これは Chrome でしか確かめられない。§6 に最小の Chrome 実行案を書いた。

## 1. 結論

1. **『Chrome の UI は node の 2.0 倍』は、比べ方の差で出た値**。0303 第一報の node 値(60 ms/仮想 s ほか)は**通し(起動 + 約 30 仮想 s の UI だけの区間 + 再生 13 仮想 s)の平均**で、Chrome の値は**再生中の窓**(曲位置 28.4〜46.3 s)。再生中の UI は、node でも UI だけの区間より重い(`f_863f60` は UI だけの区間で約 35、再生中で約 100 ms/仮想 s〔生〕。UI だけの区間のメインスレッド CPU は 0.19 s/仮想 s)。**同じ曲位置の窓を node で取り直す**(2 本の run の差: 28.4 s まで再生 → 46.3 s まで再生)と、`f_863f60` は node 99.3 対 Chrome 122.3 ms/仮想 s(1.23 倍)。
2. **node の `--prof` の tick は 1 ms ではない**(今回気づいた): 負荷のある machine(load 5〜11)では V8 の標本 thread が tick を落とし、**tick 数 ÷ メインスレッドの CPU 時間 = 0.83〜0.91**(窓ごとの実測 k = CPU ms ÷ tick = 1.10〜1.21)。窓の CPU 時間で較正すると node の値が 10〜20% 増え、`f_863f60` は **114.3(Chrome/node = 1.07)**、`f_5287c0` は **27.9(同 1.40。node の窓による揺れが大きく、30〜50 s の組で 1.12)**。= **UI の 2 大関数に、Chrome 固有の遅さと呼べる差は見えない**(1.07 は node の run 間の揺れ ±5〜7% の内、`f_5287c0` は 1.1〜1.4 で揺れが大きい)。
3. **ミキサーの大関数は Chrome のほうが速い**: 較正後の Chrome/node = `f_b34aca0` 0.64、`f_b34eaf0` 0.63、`f_b1ce5e0` 0.67、`f_b25ce20` 0.67、`f_b2f3650` 0.78、`f_b2f4490` 1.08、`f_b2f23c0` 1.05。**ミキサーの頂点(`f_b2abc20` の包含)は 41.0 対 57.4 ms/ブロック = 0.71**(`f_b284590` 0.71、`f_b2ea5b0` 0.73)。= 『Chrome が一様に遅い』ではなく、**UI は同等、ミキサーは Chrome が約 30% 安い**。
4. **(d) 全体比(同じ wasm・同じ仕事)は 0.84〜1.12 で、1.3 倍に届かない**(式と前提は §4)。上限 1.12 は『ミキサー以外の仕事が仮想時間に比例する』場合、下限 0.84 は『ブロックに比例する』場合。1.3 倍以上ではないので、**原因を種類別に分離する campaign は要らない**。
5. **原因の候補を node で当てた結果**(§5): 明示的な境界検査(`--wasm-enforce-bounds-checks`)は node で UI +18〜42%・全体 CPU +21% だが、Chrome の UI は +7% しか違わず、Chrome の wasm メモリは 12 GiB の guard 領域を持つ(= guard 領域方式が効いている形)ので**当たらない**。SMT の兄弟 thread を busy にしても node の UI は遅くならない(**当たらない**)。大ページ(THP)は両方 0(**差なし**)。cpuprofile の費用は Chrome で +2.0%(**主因でない**)。V8 の版の既定値の差(`wasm_deopt`・`wasm_inlining_call_indirect` が 13.6 で false、15.6 で true)は**ソースを読んだ限りの差**で、node 13.6 に同じ flag を足しても時間は動かない(§5 の 9)。『ミキサーが Chrome で 0.7 倍』の理由は**未特定**。
6. **Chrome の最小の追加実行案**(§6): 差が 1.3 倍未満なので必須ではない。『ミキサーが安い理由』を知りたいなら、E11 で `--js-flags="--no-wasm-deopt --no-wasm-inlining-call-indirect"` の 3 run と既定の追加 2 run(各約 5 分)。

## 2. 方法

- **Chrome(既存データ)**: `tools/chromeprof.py`(自己時間)、`tools/chrome-incl.py`(包含時間)、`tools/chrome-cat.py`(区分)、`tools/chrome-table.py`。窓 = 『play の印から +30〜+50 s』。窓の**仮想 s**は `audio-b.json` の pulses、**ブロック**は unlocks(4096 フレーム)から数える。UI 関数は ms/仮想 s、ミキサーは ms/ブロックに直す。cpuprofile の標本は 2 ms(alpha1 の a14 は 1 ms)で、時間は標本間隔(timeDeltas)の重み付き。窓 20.0 s のうち busy 17.07 s、idle 2.93 s。
- **node(新規)**: 同じ G3 を、`play28.g3`(28.4 s まで再生)と `play46.g3`(46.3 s まで再生)で別々に `--prof` で走らせ、**2 本の差**を取る(`tools/winnode.py`)。= 曲位置 28.4〜46.3 s(Chrome の E11 の窓と同じ範囲)の 17.837 仮想 s・192.0 ブロックの仕事。30〜50 s の組(`w30`/`w50`)は 2 反復。仮想時計の G3 なので『仕事』は決定的(間接呼びの数 `progress` が node と N1 で UI 区間 0.4% 差、第一報の確認)。
- **tick の較正**: 窓の node メインスレッドの CPU(harness が各 snap で書く thread CPU)÷ 窓の tick 数 = k(ms/tick)。『関数の ms = 関数の tick × k』。標本が落ちても関数ごとの**割合**は偏らない、という前提(§8)。表には生(1 tick = 1 ms)と較正後を併記した。
- **ミキサーの頂点の包含**: Chrome は呼び出し木の祖先から、node は `--prof` の tick のスタック(戻り番地)から `f_b2abc20` を含む標本を数えた(`tools/nodeincl.py`)。再帰は標本ごとに 1 回。

## 3. 比較表(窓: 曲位置 28.4〜46.3 s、17.837 仮想 s、192.0 ブロック。node は 2 本の差、`tables/0303b-ratio-table-matched-window.tsv`)

| 関数 | 単位 | Chrome E11 | node 生(1 tick = 1 ms) | node 較正後(× k = 1.151) | Chrome / node(較正後) | 同(生) | node の 30〜50 s 2 反復(較正後) | Chrome / node(30〜50 s 平均) |
|---|---|---|---|---|---|---|---|---|
| `f_863f60.body` | ms/仮想 s | 122.3 | 99.3 | 114.3 | **1.07** | 1.23 | 110.1 / 118.2 | 1.07 |
| `f_863b50.body` | ms/仮想 s | 6.2 | 9.6 | 11.1 | 0.56 | 0.65 | 8.1 / 9.7 | 0.70 |
| `f_5287c0` | ms/仮想 s | 39.0 | 24.2 | 27.9 | **1.40** | 1.61 | 34.1 / 35.7 | 1.12 |
| `f_b34aca0` | ms/ブロック | 6.08 | 8.22 | 9.46 | **0.64** | 0.74 | 9.6 / 10.2 | 0.62 |
| `f_b34eaf0` | ms/ブロック | 5.38 | 7.44 | 8.56 | **0.63** | 0.72 | 9.3 / 10.0 | 0.56 |
| `f_b2f3650.body` | ms/ブロック | 4.24 | 4.72 | 5.43 | 0.78 | 0.90 | 3.9 / 4.2 | 1.05 |
| `f_b1ce5e0.body` | ms/ブロック | 2.14 | 2.76 | 3.18 | 0.67 | 0.78 | 2.9 / 2.8 | 0.75 |
| `f_b2f4490.body` | ms/ブロック | 1.54 | 1.23 | 1.42 | 1.08 | 1.25 | 1.7 / 2.0 | 0.83 |
| `f_b25ce20` | ms/ブロック | 1.81 | 2.34 | 2.69 | 0.67 | 0.77 | 3.1 / 3.6 | 0.54 |
| `f_b2f23c0.body` | ms/ブロック | 1.08 | 0.90 | 1.04 | 1.05 | 1.20 | 1.1 / 1.2 | 0.93 |
| **ミキサー頂点 `f_b2abc20`(包含)** | ms/ブロック | 41.0(7,879 ms ÷ 192) | 49.9 | 57.4 | **0.71** | 0.82 | | |
| ミキサー `f_b284590`(包含) | ms/ブロック | 35.4 | 43.1 | 49.6 | 0.71 | 0.82 | | |
| ミキサー `f_b2ea5b0`(包含) | ms/ブロック | 37.6 | 44.9 | 51.7 | 0.73 | 0.84 | | |

- 窓 k: 一致窓 1.151、30〜50 s の 2 反復 1.104 / 1.206(= 同じ仕事の 2 run で CPU 時間が最大 20% 違った run もあるが、関数ごとの tick の割合は ±2〜3% で揃う)。
- Chrome の E10 の 7 本(`tables/0303b-chrome-runs.tsv`)の `f_863f60` は 111〜135 ms/仮想 s、`f_5287c0` は 31〜50、`f_b34aca0` は 4.7〜7.0 ms/ブロック。**Chrome の run 間の揺れは UI で ±10%、ミキサーで ±20%**(窓の曲位置が run ごとに違う分を含む)。
- ミキサーの ms/ブロックは**曲位置で大きく変わる**(E11 の `f_b34aca0` は曲の 0〜10 s で 2.1、30〜50 s の窓で 6.1)。だから**窓を揃えること**が必要で、通しの平均と窓の値を比べると 2 倍の差が出る。
- 窓の Liftoff の tick は 0 付近(tier-up の混入なし、`tables/0303b-node-window-difference-runs.txt`)。

## 4. (d) 同じ wasm・同じ仕事での全体比

**窓(192 ブロック + 曲位置 28.4〜46.3 s)の CPU**:

| | CPU(プロファイラ込み) | プロファイラの費用 | 仕事 |
|---|---|---|---|
| node メインスレッド | **20.334 s** | `--prof` は +5.5%(通し 5 run のメインスレッド CPU 平均 23.39 s 対 `--prof` の 2 run 24.68 s) | 17.837 仮想 s + 192.0 ブロック |
| Chrome Worker(busy) | **17.07 s**(窓 20.0 s − idle 2.93 s) | cpuprofile は +2.0%(Worker CPU 60 s: `p3-e17` 49.73 s 対 `ser-e17p` 50.72 s) | 11.537 仮想 s + 192.0 ブロック |

Chrome の窓の内訳(自己、`tables/0303b-chrome-e11-inclusive-and-categories.txt`): wasm 15.73 s(78.7%)、`(program)` 1.29 s(6.4%)、JS 0.04 s(0.2%)、GC 0.015 s、idle 2.93 s。**JS・GC は無視できる**(Win32 層の仕事も wasm の中)。

**ミキサーとそれ以外に分ける**(ミキサーの頂点 `f_b2abc20` の包含):

| | ミキサー頂点(包含) | それ以外 |
|---|---|---|
| node | 9,575 tick × 1.151 = **11.02 s**(54%) | 20.334 − 11.02 = **9.31 s**(17.837 仮想 s) |
| Chrome | **7.88 s**(46%) | 17.07 − 7.88 = **9.19 s**(11.537 仮想 s) |

**同じ仕事に直した Chrome の CPU**(ミキサーは 192 ブロックで同じ。『それ以外』は 2 通りの仮定):

- それ以外が**仮想時間に比例**(UI の tick は仮想時計の周期で動くので理屈に合う): Chrome = 7.88 + 9.19 × (17.837 ÷ 11.537) = **22.09 s** → node 20.33 s に対し **1.09 倍**(オーバーヘッドを引くと 21.66 s 対 19.27 s = **1.12 倍**)。
- それ以外が**ブロックに比例**: Chrome = **17.07 s** → **0.84 倍**(引くと 16.73 対 19.27 = **0.87 倍**)。

= **全体比は 0.84〜1.12(中央付近の 1.0 前後)**。1.3 倍以上という条件に当てはまらない。

**alpha1 の『node 27.6 s(40.9 仮想 s)』と Chrome の『Worker 49.7 s/60 s』は同じ仕事の比ではない**: node の通しは UI だけの区間約 30 仮想 s(0.19 CPU-s/仮想 s)+ 再生 13 仮想 s(140 ブロック)で、Chrome の 60 s は再生中だけ。Chrome は仮想時計が壁時計の 0.577 倍(v)で、同じ仮想 1 s あたりの音声ブロックが node より多い(窓: 192 ブロック ÷ 11.537 仮想 s = 16.6 ブロック/仮想 s。node 再生区間は 140 ブロック ÷ 13 仮想 s = 10.8)。= **仮想 s あたりの CPU を並べても比にならない**(第一報に書いた 1.47/2.0 の元もこれ)。node の 27.6 s は『プロセスの user 時間』(コンパイル thread・GC thread を含む。メインスレッドは 23.0〜23.8 s)。

**付記(私の問いの外だが P3 に関わる観察。意味は未確認)**: この E11 の窓で、ミキサーは 20 壁 s に 17.83 音声 s(0.89 倍速)しか作っていない(`audio-b.json` の unlocks から)。busy は 85%(idle 14.6%)。= 窓内でミキサーは実時間より遅い。idle が残るので CPU 不足だけでは説明できない(待ちの構造は 0299/0296 の範囲)。

## 5. 原因の種類別(node で当てられる物は当てた)

| # | 種類 | 検証 | 結果 | 判定 |
|---|---|---|---|---|
| 1 | cpuprofile の自己費用 | **実測**(Chrome 既存): Worker CPU 60 s で 49.73 対 50.72 s(+2.0%)。1 ms と 2 ms の run も一致 | +2% | 主因でない |
| 2 | 窓・正規化の取り違え(私の第一報) | **実測**(node の 2 本の差) | 通し平均 2.0 倍 → 同じ窓の生 1.23 倍 | **主因(説明済み)** |
| 3 | node の tick の落ち(較正) | **実測**: tick ÷ CPU = 0.83〜0.91(負荷下の unpinned の run。窓 k = 1.10〜1.21)、1.03〜1.04(cpu 12 に固定した run。k = 0.97) | 生 1.23 倍 → 較正後 1.07 倍 | **説明済み**(§1 の 2) |
| 4 | SMT の兄弟 thread が busy | **実測**(node、cpu 12 に固定、cpu 13 に Python の回転 / FMA 重負荷の spinner、通し 40.9 仮想 s): 較正後 `f_863f60` 67.7 / 67.2 / 62.2、`f_5287c0` 21.5 / 21.7 / 19.8、CPU 27.5 / 27.1 / 24.9 s(なし / 軽 / 重)。unpinned の基準 65.8 / 65.1 | **遅くならない**(重い兄弟のほうが少し速い、揺れの内) | 当たらない。Chrome 自身の SwiftShader の thread との競合は**再現していない**(Chrome 側の他 thread は各 3 s/60 s、計約 0.9 core。UI が 1.07 倍で説明すべき差も小さい) |
| 5 | 明示的な境界検査 | **実測**(node `--wasm-enforce-bounds-checks`、通し): 較正後 `f_863f60` 77.5(+18%)、`f_5287c0` 29.5(+42%)、`f_b34aca0` 9.78(+13%)、`f_b34eaf0` 9.21(+28%)、メインスレッド CPU 29.87 s(+21%)。**観察**(読み取り専用): Chrome 156 の renderer は wasm メモリを 1.09 GiB(commit)+ 10.91 GiB(PROT_NONE)= 12 GiB の guard 領域で持つ(node は 1.03 + 6.97 = 8 GiB) | 境界検査を明示にすると UI・ミキサーとも遅くなる。Chrome のミキサーは速い | 当たらない(guard 領域方式の形。UI の差 1.07 とも合わない) |
| 6 | 大ページ(THP) | **観察**: `enabled = madvise`。node の harness 実行中 `AnonHugePages` = 0 kB(Rss 1.27 GB)、Chrome の E11 の renderer も 0 kB(Rss 1.12 GB) | 両方 0 | 差なし |
| 7 | 共有メモリ | **読み**: 同じ engine-glue が `WebAssembly.Memory({initial: 10240 pages, maximum: 32768, shared: true})` を作る(node の harness も同じ glue) | 同じ | 差なし |
| 8 | V8 sandbox | **観察**: Chrome の wasm メモリは `[anon:v8-sandbox]` の中(64 GiB + 約 1.3 TiB の予約)。node にはない | 差はあるが flag で切れない(build 設定) | **未検証**。UI に遅さが見えないので優先度は低い |
| 9 | V8 の版(13.6 と 15.6) | **読み**(ソース): `wasm_deopt` false→**true**、`wasm_inlining_call_indirect` false→**true**、`wasm_deopts_per_function_limit` 50→10、`zero_unused_memory` false→true(`tables/0303b-v8-flag-defaults-13.6-vs-15.6.txt`)。**実測**(node 13.6 に `--wasm-deopt --wasm-inlining-call-indirect` を足した 2 run + 追跡 1 run): 較正後 `f_863f60` 65.9 / 68.5(基準 65.8 / 65.1)、`f_5287c0` 19.7 / 20.7(21.4 / 20.3)、`f_b34aca0` 8.41 / 8.86(8.69 / 8.56)、CPU 24.4 / 25.2 s(24.9 / 24.5)。追跡: flag は**効いている**(call_indirect の推測的 inline が 52 か所、wasm の deopt 2 回)が、**13.6 では範囲が狭い**(53 関数は旧式の例外処理を使うため deopt 無効、40 関数は FrameState が大きすぎて無効) | 13.6 では時間が動かない。**15.6 の挙動は不明** | **未検証**。『ミキサーが Chrome で約 0.7 倍』の最有力の候補だが、15.6 でしか確かめられない |
| 10 | JSPI | node は `--experimental-wasm-jspi`、Chrome も JSPI を使う。同じ glue。分離できるデータは無い | | 未検証。Chrome の JS+GC は窓の 0.3%、`(program)` は 6.4%(JSPI の停止・再開を含みうる) |
| 11 | tier-up(Liftoff の混入) | **実測**: 一致窓の Liftoff の tick 差は大関数で −31〜+8(1,500 tick 中) | 無視できる | 混入なし |

## 6. Chrome の最小の追加実行案(ci1/alpha1 と日程調整、必須ではない)

**差が 1.3 倍未満なので、原因を分離する必要は無い**。それでも『ミキサーが Chrome で 0.7 倍な理由』は P3(音切れ)の梃子の手掛かりになりうる(同じ仕組みが UI の呼び出しにも効くか)。確かめるなら:

- **対象**: E11(`engine.49bbf63df894`)、opt1 の rig(`ser-e17p` と同じ 60 s の再生、cpuprofile 2 ms)。
- **A 条件(V8 15.6 の既定)**: 既存の `ser-e17p` + **追加 2 run**(Chrome の run 間の揺れがミキサーで ±20% あるので、3 run 要る)。
- **B 条件**: `--js-flags="--no-wasm-deopt --no-wasm-inlining-call-indirect"`(node 13.6 の既定に揃える)を足した **3 run**。
- **読み方**: ミキサーの頂点(`f_b2abc20` の包含)が 41.0 ms/ブロック前後から node の 57 前後に上がれば、15.6 の call_indirect の推測的 inline が原因。UI の 2 大関数が動けば、UI の呼び出しにも効いている。どちらも動かなければ、V8 の版でなく sandbox など build 側の差。
- **規模**: 計 5 run × 約 5 分(`report.json` の `seconds` = 284〜286)、同時 1 run、machine load ≤ 8。出力は同じ形式(`ser-w0.cpuprofile`、`report.json`、`audio-b.json`)で、私の `tools/chrome-table.py`・`chrome-incl.py` にそのまま掛かる。
- **境界検査の A/B(`--js-flags=--wasm-enforce-bounds-checks`)は提案しない**: 観察(guard 領域 12 GiB)と node の結果(ミキサーも遅くなる)で足りる。

## 7. 方針との適合

- 今回は**解析のみ**で、FL の挙動・lowering・runtime に何も足していない(差分なし)。ページ・runtime の分岐を増やす提案も無い。
- §6 の flag は**測定用**で、本番の起動には入れない(入れる場合は『外部 API の契約の実装』でも『余計なロジックの除去』でも『通信/変換の削減』でもないので、別の決定文書を要する)。

## 8. 自己申告(間違い・逸脱)

1. **第一報の『Chrome の値は node の 1.3〜2.0 倍(一様)』は、私の比較の取り方の誤りで出た値**だった(node の通し平均 ÷ Chrome の再生窓)。今回、node で同じ窓を取り直して是正した(§1)。
2. **node の tick を 1 ms として扱った**(`--prof` の標本は負荷下で 10〜20% 落ちる)。窓の差の run を 2 本取って CPU 時間を比べ、0.86 tick/ms で気づいた。第一報の `f_863f60` 60.4 ms/仮想 s と『N1 の 0.40 倍』は、`--print-wasm-code` を付けた run(tick ÷ CPU = 0.985)で同じ走り方の比較なので影響は約 2% だが、**この文書の生の列は最終値でなく、較正後の列が最終値**。また node を cpu 12 に固定した run が『遅い』(70 対 60)と書いた途中の読みは、tick の落ちの差(tick ÷ CPU が 1.03 対 0.90)で、較正後は同じ(67.7 対 65.8)だった。
3. **較正の前提**: 落ちた標本が関数に対して偏らない、を仮定した(未検証。窓の組で k が 1.10〜1.21 と揺れ、`f_5287c0` の node 値が窓で 24.2 / 30.9 / 29.6 と ±13% 動くので、較正後の比は ±10% 程度の幅で読む)。
4. 一時の誤り: 他の agent が走らせている Chrome の `/proc` を**読んだ**(起動・操作なし、読み取りのみ)。集計の一行を雑に書いて **12 万字の出力を端末に出した**(再実行で是正)。`sleep` と末尾 `&` の命令が拒否された(この区間の前半)。
5. `~/work/mt3w-bundle93` を alpha1 の 16:58 の info を受けて消し、PM の『18:30 まで残す』の決定と行き違った(17:14 に再生成して通知済み)。以後、alpha1 の許可なく消さない。
6. 第一報の自己申告(body 索引の取り違え、合成の微小テストの過大評価)は据え置き。

## 9. 再現(`tools/`、`tables/`)

- Chrome: `chrome-table.py`(表)、`chromeprof.py`、`chrome-incl.py`、`chrome-chain.py`、`chrome-cat.py`(`maps/` の索引表を使う)。
- node: `run-node.sh`(`BUNDLE`/`HARNESS`/`SCRIPT`/`CPUS` を環境変数で)、`harness303.mjs`(各 snap で ring の write_frames と thread CPU を出す)、`g3/play28.g3`・`play30.g3`・`play46.g3`・`play50.g3`、`winnode.py`、`nodeincl.py`、`nodefuncs.py`、`flagtable.py`、`ratio-table2.py`(表 §3)。SMT: `spin.py`、`spinheavy.c`、`run-flags.sh`、`run-win2.sh`、`run-plain3.sh`。
- 表: `0303b-ratio-table-matched-window.tsv`(§3、最終)、`0303b-ratio-table-matched-window-RAW-superseded.tsv`(生のみの旧表)、`0303b-chrome-runs.tsv`、`0303b-node-window-difference-runs.txt`、`0303b-node-mixer-top-inclusive.txt`、`0303b-chrome-e11-inclusive-and-categories.txt`、`0303b-node-flag-and-smt-runs.tsv`、`0303b-node-plain-cpu.tsv`、`0303b-v8-flag-defaults-13.6-vs-15.6.txt`(+ 両版の `flag-definitions.h`)、`0303b-fl_trace1-wasm-inlining-counts.txt`、`0303b-address-space-thp-observation.txt`。


## PM の判断と取り込み(2026-11-06 18:12、docs のみ。製品の差分なし)

取り込み: mt3 の第一報(文書・表・道具)を `docs/investigations/mt3w/0303/` に置いた(V8 の `flag-definitions.h` の抜粋 2 本〔計 376 KB、V8 のソースのコピー〕は入れない。原本は `lift/out-git/mt3w-results/0303/tables/`)。node 側は新規に実行(E11 のテスト用 wasm、node 24.19 / V8 13.6、仮想時計 G3)。Chrome 側は既存の cpuprofile(opt1 の ser-e17p ほか)の再解析で、**Chrome は起動していない**。依頼者の機械は 0 件。

mt3 の自己是正を受理する: 0303 第 1 報の『Chrome は node の 1.3〜2.0 倍(関数に固有でない)』は、node の通し平均(UI だけの区間約 30 仮想 s を含む)と Chrome の再生窓(曲位置 28.4〜46.3 s)を比べた誤りで、さらに node の `--prof` の tick を 1 ms として扱った誤り(負荷下で 10〜20% 落ちる。tick÷CPU = 0.86 で較正)があった。

事実として受け取る(窓 = 曲位置 28.4〜46.3 s、17.837 仮想 s、192.0 ブロック。node は 2 run の差。較正後の列が最終値、較正の前提『落ちた標本が関数に偏らない』は未検証で ±10% の幅で読む):
1. UI: `f_863f60` は node 114.3 対 Chrome 122.3 ms/仮想 s で Chrome/node = 1.07、`f_5287c0` は 1.40(node の窓による揺れが大きく、30〜50 s の 2 反復では 1.12)。= Chrome 固有の遅さと呼べる差は無い。
2. **ミキサーは Chrome のほうが速い**(Chrome/node: `f_b34aca0` 0.64、`f_b34eaf0` 0.63、`f_b1ce5e0` 0.67、`f_b25ce20` 0.67、`f_b2f3650` 0.78、頂点 `f_b2abc20` の包含 41.0 対 57.4 ms/ブロック = 0.71)。
3. 同じ wasm・同じ仕事の全体比は 0.84〜1.12 倍で 1.3 倍に届かない。alpha1 0299 の『node 27.6 s/40.9 仮想 s』と Chrome の『Worker 49.7 s/60 s』は同じ仕事の比ではない(node は UI だけの区間込み、Chrome は再生中のみで仮想 s あたりのブロック数が多い)。
4. 原因の候補の仕分け: cpuprofile の費用 +2.0%(主因でない)、SMT 兄弟が busy でも遅くならない(node 実測)、明示的な境界検査は node で +13〜+42% だがミキサーも遅くなるので Chrome の『ミキサー速い』と合わない、大ページ・共有メモリは差なし、V8 の版(15.6 は `wasm_deopt` と `wasm_inlining_call_indirect` が既定で true、13.6 は false。13.6 に両 flag を足しても時間は動かない)が『ミキサーが Chrome で 0.7 倍』の最有力の候補だが未検証、V8 sandbox・JSPI は未検証。
5. 付記(P3 に関わる観察、意味は未確認): この E11 の窓でミキサーは 20 壁 s に 17.83 音声 s(0.89 倍速)しか作らず、Worker は idle 14.6%。CPU 不足だけでは説明できない(gpu1 に idle の中身の種類別を依頼済み)。

判断:
1. 『Chrome 固有の遅さ・特定の大関数の遅さ』という梃子は無いと結論する。0296 の L3 撤回(決定 0303 と 0296 の訂正)と合わせて、1 Worker の協調版で『必要な 23% の CPU 削減』を得る忠実な梃子は、present の Worker 外し(gpu1 の 0297: Worker の 18〜21%)と実スレッド化(UI 専用コア)に絞られる。
2. 種類別に分ける Chrome の追加 campaign(5 run)は**やらない**(1.3 倍未満)。V8 15.6 の call_indirect 推測 inline の効きは記録のみ。
3. 未確認: Chrome 15.6 の V8 の挙動、V8 sandbox、JSPI、較正の前提、他の agent の Chrome の /proc を読み取りだけした観察(mt3 の自己申告に記載)。
