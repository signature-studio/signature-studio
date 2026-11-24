# 段階 0: 重さの内訳と最適化の水準(Issue #1388)

実測: `bash tools/engine-cost/run.sh`(段: baseline・runs-oz・variants・runs-variants・contrast・aggregate)。製品の `-Oz` は `lift/crates/fe-build` の `fe-build engine` と `fe-build bundle` をそのまま使った。変種(O2・O3)は `tools/engine-cost/build-variant.mjs`。計測は `tools/engine-cost/measure.mjs`(Chrome の CDP。CPU プロファイルは 250 µs 間隔)。集計は `tools/engine-cost/aggregate.mjs`。nproc 4、loadavg は集計時点で 0.46 0.62 0.95(計測の run の値ではない)。

開始 09:45:02 UTC(`date -u`)。終了は 13:20 UTC ごろ。4 時間 30 分の目安(14:15)より前に終えた。

## 結論

- **判定: emscripten のグルーを替える価値は無い(0-7 の「価値がない」)。段階 1 は行わない。** 区分 A(emscripten のランタイム・グルー)は、場面 (iii) の忙しい時間の 0.2%(平均 25 ms / 12,388 ms)。境界の対照実験では、1 回あたりの差が 0.01〜0.09 ns(2 回の計測。どちらも run のばらつきの中)で、基準の 50 ns に遠く及ばない。
- **最適化の水準: 先に変える Issue は起票しない。** O2 は Oz より忙しい時間が 3.1% 少なく、O3 は 7.4% 少ない。どちらも基準の 20% に届かない。
- **重さの主因**(場面 (iii)、マウス 1200 Hz): 忙しい時間の 44.5% が F(ブラウザ内部の `(program)`。主にメインスレッド)、27.2% が E(アプリの JS とブラウザの組み込み関数)、B(lift した C)12.6%、D(runtime と libc の C)12.2%。場面 (ii)(再生中)では B が 49%。wasm の質(B・D)と、ページ側・ブラウザ側(E・F)が主因で、emscripten の区分 A は、どの場面でも 1% 未満。
- **受け入れの判定は通らない(正直に書く)**: 0.95 以上を求める classifiedShare が、18 run のうち 12 件で届かない。場面 (i) の 6 件は 0.92〜0.93、場面 (ii) の 6 件は 0.82〜0.86。場面 (iii) の 6 件は 0.966〜0.969 で満たす。原因は、名前が付かない wasm 関数。`lift/out-app/gen/wasm-func-names.tsv` に名前の無い行が 12,770 件あり、`engine.wasm` にも name セクションが無いため、どの区分にも入れられない(仕様どおり unclassified に入れた)。一回の run で確かめると、unclassified の 98〜99.7% がこれだった(場面 (i) 189/193 ms、場面 (ii) 1,172/1,177 ms、場面 (iii) 424/432 ms)。この関数の時間が A〜F のどこに入るかは、判定に効かない。A は、どの場面でも 1% 未満だから。

## 1. 場面と DPR ごとの区分

窓は 60 秒(場面 (i)・(iii) は t0+30 s 〜 t0+90 s。場面 (ii) は再生が始まってから 60 s)。忙しい時間と idle は、プロファイルした線(main、engine の Worker 2 本)の合計。値は n=1〜3 の平均。n ごとの値は `engine-cost-attribution.json` の `runs`。

場面 (iii)(マウス 1200 Hz を 60 秒。判定の場面):

| DPR | 忙しい時間 ms | A | B | C | D | E | F | unclassified | idle ms | classifiedShare |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1.00 | 12,388 | 25.3 (0.2%) | 1,557.0 (12.6%) | 7.4 (0.1%) | 1,517.2 (12.2%) | 3,371.4 (27.2%) | 5,506.8 (44.5%) | 402.8 (3.3%) | 167,716 | 0.967 |
| 1.325 | 12,202 | 19.5 (0.2%) | 1,506.0 (12.3%) | 4.9 (0.0%) | 1,482.5 (12.1%) | 3,407.2 (27.9%) | 5,385.4 (44.1%) | 396.5 (3.2%) | 167,918 | 0.967 |

場面 (i)(起動後・無操作。窓は t0+30 s 〜 t0+90 s):

| DPR | 忙しい時間 ms | A | B | C | D | E | F | unclassified | idle ms | classifiedShare |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1.00 | 2,862 | 20.0 (0.7%) | 810.7 (28.3%) | 7.4 (0.3%) | 654.9 (22.9%) | 516.2 (18.0%) | 629.7 (22.0%) | 222.8 (7.8%) | 177,215 | 0.922 |
| 1.325 | 2,754 | 22.8 (0.8%) | 744.6 (27.0%) | 5.6 (0.2%) | 656.5 (23.8%) | 502.7 (18.3%) | 606.8 (22.0%) | 214.6 (7.8%) | 177,315 | 0.923 |

場面 (ii)(Space で再生。窓は再生の開始から 60 秒。メトロノームは押していない。n=1 では `nonSilentQuanta` が 0 のままで、音は出ていない):

| DPR | 忙しい時間 ms | A | B | C | D | E | F | unclassified | idle ms | classifiedShare |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1.00 | 6,459 | 23.1 (0.4%) | 3,144.1 (48.7%) | 12.9 (0.2%) | 732.5 (11.3%) | 585.8 (9.1%) | 836.3 (12.9%) | 1,123.9 (17.4%) | 173,614 | 0.826 |
| 1.325 | 7,986 | 24.1 (0.3%) | 3,969.9 (49.7%) | 54.4 (0.7%) | 1,435.7 (18.0%) | 533.2 (6.7%) | 842.5 (10.6%) | 1,125.8 (14.1%) | 172,144 | 0.859 |

- 忙しい時間は、プロファイルした線の合計。一つの run の内訳は、main が約 6.1 s、engine の Worker が約 6.7 s、render の Worker が約 0.04 s(場面 (iii)、DPR 1.00、n=1)。したがって、60 秒の窓で 1 本の線が占める割合は、約 10% 前後。
- 区分 E には、V8 の境界フレーム `wasm-to-js` と、URL の無い組み込み関数(`postMessage`・`setTimeout`・`now` など)を入れた。一度保存した場面 (i) の計測では、engine の Worker の忙しい時間 1,923 ms のうち `wasm-to-js` は 37.6 ms(約 2%)。
- 区分 D には、runtime の C(`fle_*`・`poll_waiters`)と、C の関数のうち runtime でも Blend2D でも emscripten でもない物(libc の `memcpy`・`malloc` など)を入れた。libc を分けては数えていない。
- 区分 F は、ブラウザ内部の `(program)`・`(garbage collector)`・`(root)`。`(program)` の内訳は分けていない(未確認)。

## 2. 境界・JSPI・memory.grow・Atomics.wait

| 項目 | 値 | 理由 |
| --- | --- | --- |
| boundaryCallsPerSec | null | 呼び出し回数は、製品の import の内側でしか数えられない。製品を変えずには測れない |
| jspi(suspend・resume の回数と ms) | null | 同上。JSPI の包みの中で数えるしかない |
| memoryGrow(回数と ms) | null | 同上。`memory.grow` の呼び出しを数えるには製品の内側が要る |
| atomicsWaitMs | 0.0(全 run) | `Atomics.wait` と `emscripten_futex_wait` を葉に持つサンプルが無かった。Worker の待ちは `fle_js_wait`(EM_ASYNC_JS。区分 A)などの別の名前に入っている可能性がある(未確認) |
| preBootBytes(engine-idle まで、Network の encodedDataLength の合計) | 約 83.1 MB(Oz、6 run の平均) | 測定値。Worker の読み込みは attach より前に終わった分が見えない |

## 3. 対照実験(境界の費用)

`tools/engine-cost/contrast/contrast.mjs`。同じ C のループ(`hot(i)` を 1,000,000 回の `unsigned` 和)を、emscripten の包み(`EM_JS` の `hot`。`emcc -O2`、ES6 の glue 経由)と、包み無し(`clang --target=wasm32 -O2`、importObject に JS 関数を直接渡す)で、node 26 で計測した。1 回の run は 100 万呼び出し。ウォームアップ 1 回の後に 3 回計測し、中央値を取った。

| 計測 | emscripten の包み(ns/回) | 包み無し(ns/回) | 差(emscripten − 包み無し) | run のばらつき |
| --- | --- | --- | --- | --- |
| 1 回目(13:12) | 3.821(run: 3.795・4.421・3.821) | 3.727(run: 3.727・3.810・3.720) | +0.093 | 0.626 ns(中央値の約 16%) |
| 2 回目(13:14。JSON の値) | 3.834(run: 3.715・3.834・4.479) | 3.826(run: 3.826・3.904・3.738) | +0.008 | 0.764 ns(中央値の約 20%) |

- 二回の差は +0.093 ns と +0.008 ns で、符号も揃わない。どちらも、run のばらつきより小さく、この計測では決まらない。判定の基準(50 ns)には、どの振れを見ても届かない。ばらつきが大きいのは、この機械が静かでなかった可能性がある(loadavg は 2 回目の前後で 0.04〜0.8)。
- checksum は両方で 1784293664 で一致。glue の中の `hot` の式(`x+1`)を確かめた。
- 注意: 対照は、同期の import だけを測った。JSPI の suspend・resume(EM_ASYNC_JS: `fle_js_wait`・`jspi_suspend`)の費用は測っていない(未確認)。

## 4. 最適化の水準

場面 (iii)・DPR 1.00・n=3 が busyMs の比較の中心。underrunQuanta・elapsedMs・preBootBytes は場面 (i) と (iii) の 6 run の平均。

| 変種 | 忙しい時間 ms(場面 iii、平均) | 範囲 | 忙しい時間 ms(場面 i、平均) | underrunQuanta | elapsedMs(navigate→engine-idle) | wasm のバイト数(raw) | engine の br(brotli 11) | preBootBytes(測定) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Oz(製品) | 12,388 | 11,927〜12,836 | 2,862 | 0 | 8,134 | 165,619,829 | 17,889,995 | 83,149,478 |
| O2 | 12,003 | 11,669〜12,562 | 2,399 | 0 | 8,937 | 314,289,251 | 24,411,826 | 89,584,321 |
| O3 | 11,478 | 11,399〜11,548 | 2,236 | 0 | 8,577 | 291,256,904 | 22,965,961 | 88,240,154 |

- O2 は Oz より 3.1% 少ない、O3 は 7.4% 少ない(場面 (iii)、DPR 1.00)。O2 の値の範囲は Oz と重なる。O3 は Oz の範囲より下。どちらも 20% の基準には届かない。
- 変種の wasm は大きい(O2 は 1.9 倍、O3 は 1.8 倍の raw。br は +36%、+28%)。サイズ優先の `-Oz` は、この点で正しい。
- 変種の旗: `tools/engine-cost/flags/`(`O2-*`・`O3-*`)。変えたのは gen の C の `-Oz` と wasm-opt の `-Oz` だけ。runtime(`-O2`)・harness(`-O1`)・Blend2D の .o・libblend2d.a・exports.rsp・seed.bin はベースラインの物を使った(旗が変わらないので同じ物。`lift/out-app/variants/<name>/variant.json` の `reusedFromBaseline`)。
- 変種の gen の C のコンパイルは 925 s(O2)。wasm-opt は 166 s(O2)、978 s(O3)。

## 5. 判定の規則と結果(数字は変えていない)

| 規則(Issue 0-7) | 結果 |
| --- | --- |
| 価値がある: A ≥ 15%(場面 iii の平均)、または境界の差 ≥ 50 ns かつ ≥ 10 万回/秒 | A = 0.2%。境界の差 = 0.01〜0.09 ns(呼び出し回数は測れず null)。**満たさない** |
| 価値がない: 主因は B・C・D で、emscripten の置き換えは主因に効かない | A は 1% 未満。主因は B・D(場面 ii・iii)と E・F(場面 iii)。**当てはまる** |
| 先に最適化の水準を変える: O2 か O3 が Oz より忙しい時間を 20% 以上減らす | O2 −3.1%、O3 −7.4%。**満たさない** |

結論: 段階 1(zig cc と自前のグルーへの置き換え)は行わない(判定により、行わない)。最適化の水準を変える Issue も起票しない。

## 6. 測れなかった項目・制約

- **受け入れの classifiedShare(0.95)は 12 件で届かない**(§結論 の理由)。場面 (iii) の 6 件は満たす。
- **再試行**: 最初の試行で 2 件が 180 秒で engine-idle にならず失敗した(場面 i、DPR 1.325、n=3。場面 iii、DPR 1.325、n=2。どちらも state は running のまま)。同じ条件の再試行は両方とも成功した。原因は未確認。
- **変種の run**: 場面 (i) と (iii) の n=3 だけ。場面 (ii) は測っていない(仕様どおり)。
- **名前の無い wasm**: 表の unclassified の大半。名前を与える資料は無い(TSV と engine.wasm の name セクションのどちらにも無い)。
- **E と F の内訳**: これ以上は分けていない。`(program)` は、メインスレッドのブラウザ側の仕事。何かは未確認。
- **Atomics.wait**: 0.0 と測れた。ただし、待ちを数える名前は使われていない可能性が高い(未確認)。
- **emcc**: emsdk の emcc は、内部で別の言語のスクリプトを起動する(製品と同じツールチェーン)。計測の道具(`tools/engine-cost/`)は、どの段でも、そのような言語を使っていない。

## 7. 作業の記録

- 09:45:02 開始。10:12:20 baseline の `fe-build engine`(exit 0、24 分)。10:12:28〜10:24:13 `fe-build bundle`(12 分)。
- 10:31〜11:17 ベースラインの 18 run(起動の失敗 2 件は 11:14〜11:17 に再試行して成功)。
- 11:18:06〜11:56:46 O2 のビルド(38 分)。11:56:56〜12:49:39 O3 のビルド(53 分)。
- 12:50〜13:11 変種の 12 run。13:11〜13:12 対照実験(1 回目は照合の書式で失敗。修正して成功)。13:12 集計。
- 製品(`app/`・`lift/`・`ci/`・`.github/`・`inputs/`)は変えていない。製品の出力(`lift/out-app/`)は git で無視される。
- 作業ツリーに残っていた前回の作業のうち、この段階の範囲外の物を取り除いた: 誤って作られたアセンブリの出力 1 件(`./-`)と、段階 1 の `tools/zig-vs-emcc/`(index に入っていたが commit はされていなかった。段階 1 は判定により行わない)。

## 8. 数字の出典

- `pm-work/findings/engine-cost-attribution.json`(18 run の値、変種の値、対照実験、nproc・loadavg)。
- 生の run は `lift/out-app/engine-cost/runs/{Oz,O2,O3}/`(git で無視される。再集計は `node tools/engine-cost/aggregate.mjs`)。
- 対照実験: `lift/out-app/engine-cost/contrast.json`。
