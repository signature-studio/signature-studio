# 決定 0249(仮): UI Worker の CPU の内訳 — 参照プロジェクトの再生では Worker の 94% が忙しく、その 32% は C 経路の関数(+ それが呼ぶ補助 op と SSE/x87 エミュレーション)、UI 操作(Mixer ドラッグ・Settings)の 42〜47% は WebGL の push-frame(SwiftShader の費用)で、最初の 1 手は『直 wasm にできるのに set U に無い関数を足す』

状態: **仮の決定(すべての選択)**。作業者 `gpu1`(0249 担当。成果物名は gpu2)、2026-11-05 JST 15:47〜(UTC 06:47〜)。基準 origin/main **e840e4d4**(のち HEAD 4d70b8f4 = runtime b42fad36 の本番 bundle afd46e67 でも b を 1 回測った)。区分: **実測**=このセッションのコマンドの出力を見た事実、**読み**=コード・文書・測定の解釈、**未確認**。**出典なしの選択**=`S-gpu2-N`(§9)。**この機械(16 vCPU の Zen 5 VM、実 GPU なし〔SwiftShader〕、headless Chrome 156、load 5〜30 で他担当と共有)の値で、依頼者の機械(Firefox 159 / Windows 10 / GTX 980)ではない。入力は CDP の模擬で人の操作ではない。依頼者の機械の値は 0 件、実機 Windows の挙動は未確認。実装は 0 行(測定と案だけ)。**

## 結論(先頭の数行)

1. **測った bundle は本番そのもの**: origin/main e840e4d4 を CI の段スクリプトで冷間ビルドした `flengine6.opt.wasm`(sha256 3d0279d4…、141,795,291 B)は、本番の 8 モジュールの bundle `fl-engine-ci-623aba04` の wasm と **byte 同一**(`cmp`。glue 6ac38751eb38・dll pack 2f494419c1ad の hash も同じ)。本番の bundle は読み取り専用の wrangler(配信バケットの list/get だけ)で取った。
2. **Worker の忙しさ(Chrome 156、既定の入口、V8 CPU プロファイラ 200 µs)**: アイドル 14%(201 ms/s)、**参照プロジェクトの Song Mode 再生 94%(929 ms/s。mt3 の 92〜94% と一致)**、F8 ピッカー定常 21%(259 ms/s)、Mixer ヘッダードラッグ 48%(544 ms/s)、Settings タブ切替 52%(471 ms/s)。
3. **再生(b)の内訳(busy 42.2 s)**: 直 wasm(hybrid)48.6%(FLEngine 35.8%、elastique 6.7%、dsp 3.3%、QFC 2.1%)、**SSE/x87 エミュレーション 15.5%**、C 経路 13.2%、C 経路の補助 op(fle_ld/st/cc/op_*)7.1%、V8 native 5.3%、GDI/raster/合成 4.0%、他 6%。**UI スレッド(根 f_1375990…)が busy の 58.2%(24.6 s)、ミキサーを含む他のゲストスレッド(根 fle_wasm_thread_entry…)が 33.5%(14.1 s)**。後者の 33% が SSE/x87 エミュレーション。
4. **エミュレーション 15.5% と『C 経路 + 補助 op 20%』は同じ根**: エミュレーション 5.8 s の 92.7%(5.4 s)は C 経路の関数が呼んでいる。C 経路の関数が引き起こす費用(自己 + 補助 op + エミュレーション)は **b で 13.6 s = busy の 32.2%**(e 17.8%、d 8.4%、c 8.6%、a 9.2%)。
5. **理由別(なぜ C 経路に残るか)**: Elastique 5.30 s(12.6%)、**直にできるのに set U に無い 2.90 s(6.9%)**、x87 2.61 s(6.2%)、Cvtps2dq/Cvtpd2dq 1.07 s(2.5%)、他。**2.90 s のうち 2.66 s は直 wasm の本体が 20,000 B 以上の関数**(決定 0239 が first idle を理由に外した 63 本の一部)で、**f_e95750(73.8 KB)だけで 1.77 s**。最初の 1 手は set U への追加(§7)。実験: 4 関数(f_e95750・f_ddf630・f_e7e470・f_e9d8a0)を足した版は wasm +139,096 B(+0.098%)、Worker CPU は本番に対し 3 往復で −2.2%(**標準偏差の内**)。
6. **d/e の『(program)』42〜47% は WebGL の push-frame**(Chrome の trace で Worker の `GetResourceForPushFrame` が e の 16.3 s のうち 5.9 s。SwiftShader の費用で、ゲストの費用ではない。実 GPU の値は 0 件)。
7. **f_863f60(FLEngine の UI、b で自己 3.4 s)は FL 自身の 60 Hz(再生中)の UI 更新の再帰関数**。部分木に runtime の API 呼び出しは無い(lift-fidelity の (a)(b) のどちらでもなく、生成コードの量と質の費用)。

## 冒頭の三点(正直さ)

1. **何を実行したか(実測の範囲)**: 実エンジン(`FLEngine_x64.dll` と同梱 DLL 7 個を x86-64 → C → wasm にした物。8 モジュール、出荷構成 F2xH7)の origin/main e840e4d4 を **CI の段スクリプトで冷間ビルド**した(extract 1 s・felift 7 s・lift 34 s・b2d 25 s・objects 452 s・link 1,013 s〔load 5〜17〕)。比較用に、同じ入力に `-g`(wasm-opt・wasm-merge)・`emcc --profiling-funcs`・`fe-wasmgen --names`(測定用の木だけ)を足した**名前つき版**を作った(コードは同じで関数の並びだけ違う。§1)。隔離 headless Chrome 156 を CDP で動かし、エンジンの Worker に V8 の CPU プロファイラを付けて 5 場面(a アイドル 20 s、b 参照プロジェクト再生 45 s、c ピッカー 20 s、d Mixer ドラッグ 12 s、e Settings タブ 21 クリック)を測り、本番の nameless bundle でも b を 2 回(623aba04、afd46e67)測った。参照プロジェクトは `~/ダウンロード/serina1-serum2.zip`(読み取り専用、コミットしない)をページのファイル管理窓に入れて File>Open。代役・モック・画像の再生・期待値の貼り付けは 0。
2. **結論**: 上の『結論』。
3. **未確認**: 依頼者の機械・実 GPU での値(特に push-frame と GL の費用)。Firefox の内訳(プロファイラは Chrome だけ)。プロファイラ込みの絶対 ms(プロファイラの分は分けていない。b で本番の nameless を同じ条件で測ると busy 91.7% 対 名前つき版 93.6%)。set U への追加の効果(3 往復では雑音の内)。起動 16 スナップ・a4 の一致(追加版では未実行)。first idle への影響(Chrome の追加版では測っていない)。ピッカー定常が ci1 の 450〜530 ms/s より低い(259 ms/s)理由(0245 の差か、プロファイラ込みの差かは分けていない)。f_863f60 が何の文字列を処理するか。

## 1. 方法と、名前つき版が本番と同じコードであることの確認

- **プロファイラ**: 動作中の Worker にはブラウザ側から後付けでアタッチできない(`Target.attachToTarget` した Worker は `Runtime.evaluate` にも 20 s 応答しない)。ページ作成時に `Target.setAutoAttach`(waitForDebuggerOnStart、flatten)で Worker を生成時に捕まえ、`Profiler.enable` → `Runtime.runIfWaitingForDebugger`。場面の窓だけ `Profiler.setSamplingInterval(200 µs)` → `start` / `stop`。自己時間 = 次のサンプルまでの時間(timeDeltas)。`tools/sdriveg2.mjs`(gpu1 の sdriveg に `profStart/profStop/cpuStart/cpuStop/traceStart/traceStop` を足した物)。
- **名前**: 本番の wasm に name section は無く(section 1〜13 に custom 0 が無い)、V8 は `wasm-function[N]` としか出さない。CI の `wasm-opt -Oz` と `wasm-merge` は名前を書かないが読む。そこで同じ入力に **`emcc --profiling-funcs`**(link。unoptimised の wasm から名前を外すと CI の link と byte 同一)、**`wasm-opt -g`**、**`wasm-merge -g`**、**`fe-wasmgen --names`**(直 wasm の `f_<VA>` 名。測定用の木だけ)を足して作った(`tools/mk-names-wasm.sh` の流れ、実際は `ciPN2`)。link map は(パス文字列を除き)CI のと同一。**名前を外した結果と本番の違い**: 全 section のサイズが同じ(code 139,193,192 B)、関数 104,509 本、本体サイズの多重集合が同じ、ただし type/function/code section の中身が違う。Binaryen の ReorderFunctions は同数の関数を名前で並べるので、名前の有無で並びが変わる(=コードは同じで関数の並びだけ違う、byte 同一ではない)。**本番の nameless で b を測った粗い比と合う**(busy 91.7% 対 93.6%、busy の中の wasm 93.4% 対 93.6%、(program) 5.3% 対 5.3%、JS 1.2% 対 1.0%、GC 0.1% 対 0.1%)。
- **層別**: 名前 → 層は `tools/prof-analyze.py` の規則(link map 由来の `funcmap`〔関数 → オブジェクト = 10.7 万関数〕、`idxmap`〔wasm index → 名前。名前の無い直 wasm の関数は export 名 `f_<VA>` で引く〕、DLL の VA 範囲)。直 wasm か C 経路かは『名前が link map にあるか』で決める(C 経路の `f_<VA>` は link map にある。hybrid で直にした物は C の単位から外されている)。
- **スレッドの分け**: UI とミキサーは 1 本の Worker の協調スレッドで、プロファイルの木の根が違う(UI は `f_1375990 > f_1375950 > f_86a660`、他のゲストスレッドは `fle_wasm_thread_entry > fle_call_guest > f_412ae0`)。
- **場面**: 全部 viewport 2260x1116(ゲスト 1920x1080)、既定の入口(クエリなし)、wizard を閉じた直後から 8 s 待ってから測る。b は Problems ダイアログを Escape(「Copy」の popup を閉じる)+ Ok の click で閉じ、10 s 待ってから Space、45 s。d は 20 歩の往復を 12 s、e は F10 の 7 タブ × 3 周。

## 2. 場面ごとの内訳(`tables/scene-layers.md`、各場面の上位 30 は `tables/prof-{a..e}.md`)

| 層(Worker の busy に対する自己時間) | a アイドル | b 再生 45 s | c ピッカー | d ドラッグ | e タブ切替 |
|---|---|---|---|---|---|
| 窓(s) / busy(s, %) | 20.1 / 2.7 (14%) | 45.1 / 42.2 (94%) | 20.0 / 4.1 (21%) | 13.5 / 6.5 (48%) | 16.3 / 8.6 (52%) |
| Worker スレッド CPU(/proc、プロファイラ込み。s, ms/s) | 4.0 (201) | 41.9 (929) | 5.2 (259) | 7.3 (544) | 7.7 (471) |
| ゲスト: 直 wasm(hybrid) | 54.1% | 48.6% | 54.3% | 24.3% | 25.1% |
| runtime: SSE/x87 エミュレーション(fle_mx_*、round_pack、fle_x87_*) | 3.7% | **15.5%** | 3.6% | 2.5% | 4.8% |
| ゲスト: lifted C 経路 | 6.1% | 13.2% | 5.1% | 5.3% | 8.8% |
| lift の補助 op(fle_ld/st/cc/op_*/cas_*) | 1.4% | 7.1% | 0.8% | 1.7% | 5.1% |
| runtime: GDI / raster / 合成 | 3.3% | 4.0% | 16.8% | 3.8% | 2.8% |
| V8 native「(program)」 | 7.5% | 5.3% | 2.5% | **46.5%** | **41.9%** |
| JS(glue・ページ)+ JS 組込み(postMessage・setTimeout・GL 呼び出し) | 9.5% | 0.9% | 7.4% | 5.8% | 2.8% |
| runtime: threads/audio・windows/messages・dispatch | 8.1% | 1.3% | 5.3% | 7.3% | 2.7% |
| libc / emscripten | 2.6% | 1.9% | 1.8% | 1.7% | 2.0% |
| Blend2D upstream(C++)+ bl::* | 0.2% | 1.4% | 0.2% | 1.3% | 2.7% |
| JSPI 切替 | 1.8% | 0.2% | 1.0% | 0.3% | 0.3% |
| GC | 0.6% | 0.1% | 0.2% | 0.2% | 0.1% |

- b の busy 93.6%、Worker CPU 929 ms/s は mt3 の『UI Worker 92〜94% 忙しい』と一致。ピッカー定常 259 ms/s は ci1 の 5c4b6902 での 450〜530 ms/s より低い(本ビルドは 0245〔同一画素の書き直しを合成器に知らせない〕以降。プロファイラ込みの差か 0245 の効果かは分けていない)。
- 層の定義の注意: 『(program)』は V8 が JS/wasm フレームを持たない native の時間(コンパイル・GC 以外・push-frame など)。busy に含めた(S-gpu2-4)。

### 2.1 b(再生)の UI スレッド対他のゲストスレッド

| スタックの根 | ms | busy の割合 | 内訳(上位) |
|---|---|---|---|
| UI: `f_1375990 > f_1375950 > f_86a660` | 24,560 | **58.2%**(45 s の 55%) | 直 wasm 67%、lifted C 10%、SSE/x87 7%、GDI/raster 5% |
| 他のゲストスレッド(ミキサーを含む): `fle_wasm_thread_entry > fle_call_guest > f_412ae0` | 14,147 | **33.5%**(45 s の 31%) | **SSE/x87 33%**、直 wasm 28%、lifted C 21%、補助 op ほか |
| (program) | 2,241 | 5.3% | |
| 他(JS・GL・GC) | 約 1,300 | 3% | |

### 2.2 上位の関数(b。自己時間。`tables/prof-b.md` に 30 本)

`f_863f60`(FLEngine、直 wasm、UI)3,421 ms(busy の 8.1%)、`(program)` 2,241、`fle_ld`(C 経路の補助)1,427、`f_5287c0` 1,101、`f_6becc0`(c 類)1,091、`fle_r_blit` 1,042、`f_b2f3650`(elastique、C 経路)843、`f_6bed60` 811、`f_e95750`(FLEngine、C 経路)765、`f_b34aca0`(elastique、直)638 ほか。SSE/x87 エミュレーションの内訳(`tables/emulation-b.txt`): mulps 1,047、addps 717、cvtps2dq 418、subps 365、mulss 360、round_pack 346、maxps 295、mulsd 264、addsd 250 ms ほか。

## 3. 『(program)』= WebGL の push-frame(d/e)

- d の『(program)』3,036 ms のうち **10 ms 以上の連続する塊が 93 回で 2,733 ms(90%)、e では 135 回で 3,390 ms(95%)**(1 回 25〜35 ms。`tables/program-runs-{d,e}.txt`)。直前のフレームは idle・texSubImage2D・postMessage・clear・present。e の窓では 2 秒ごとの (program) が 80〜800 ms で、最初の周だけでなく 3 周とも続く(`tables/slices-e.txt`)= 初回呼び出しのコンパイルではない。
- Chrome の trace(CDP Tracing、`toplevel`・`devtools.timeline`・`v8`・`blink`・`cc`・`gpu`・`viz`。`tools/scn-trace-e.json`)で、DedicatedWorker スレッドの `WebGLRenderingContextBase::GetResourceForPushFrame` が **e の 16.3 s のうち 5.9 s**、その中が `CopyRenderingResultsFromDrawingBufferToResource`(5.9 s)。= OffscreenCanvas(WebGL)のフレームを compositor に渡す drawing buffer のコピーと同期を、Worker のタスク末尾で実行している。この機械は SwiftShader(ソフトウェアの GL)なので CPU で行う。
- **読み**: 実 GPU では GPU 上のコピーになり、Worker の費用は小さいはず。**実 GPU の値は 0 件**。ゲストの費用ではない。依頼者の機械での費用は、配備済みのビーコン(`desk.drawMs` ほか)で読める。
- `--no-wasm-lazy-compilation` を付けても e の (program) は 41.9% → 36.7% で、first idle も変わらなかった(フラグが効いていたかは確認していない)。

## 4. f_863f60(FLEngine の UI、b の自己 3.4 s = busy の 8.1%、アイドルでも 20 ms/s)

- **呼び出しの鎖**(木と census〔決定 0231 の E-all〕): FL のメッセージループ `f_86a660`(18 入口/18 場面)→ `f_86b440`(アイドル処理。入口 62,391 回)→ `f_86b300`(同回数。322 命令/入口)→ `f_863c20` → `f_863b50` → `f_864030` → **`f_863f60`(自分自身を再帰)**。`f_863f60` は入口 46.0 M 回/18 場面、1 入口 約 390 命令、1 入口あたり `f_863f20` を約 14 回、`f_6113f0` と `f_40fe90` を約 21 回呼ぶ。部下の一番は `f_5287c0`(入口 652 M 回、29 命令/入口)。
- **x86 の読み**: Delphi の UnicodeString(`[rcx-4]` が長さ)の WideChar を 1 文字ずつ調べ、`cx<=0xff` なら `bt` で文字集合に照会し、含まれれば `_UStrDelete` を呼ぶループ。コントロールの文字列を取り出して整形し戻す関数に読める(何の文字列かは未確認)。
- **頻度**: プロファイルのスタックから `f_86b300` の部分木は b で 5.7 s(126 ms/s)、1 回 平均 1.73 ms、**約 61 回/s(= 60 Hz のタイマー)**、アイドルでは 56 ms/s(アイドルの busy 135 ms/s の 42%)。
- **切り分け(lift-fidelity)**: 部分木の中に runtime の API 呼び出し(GDI・窓・タイマー)は無く、呼ぶのはゲスト関数と `fle_progress_slice` だけ。回数 61/s は 16 ms 周期の要求と矛盾しない(要求値そのものは API 記録で未確認)。= **(a)(余計な処理)でも (b)(API の誤った答え)でもなく、FL 自身の処理量 × 生成コード(直 wasm)の質**。runtime では下がらず、生成器の質(1 x86 命令あたりの wasm 命令数)でしか下がらない。

## 5. 削減案(実装は次の担当)

b の busy 42.2 s に対する割合。『削れる見積り』は『直 wasm にすると費用が 1/3』(S-gpu2-1)で出した。根拠: 同じプロファイルで実行された x86 命令数(決定 0231 の ref1-perf の命令数を重みに)あたりの費用が C 経路(補助 + エミュレーション込み)対 直 wasm = **5.2 : 1**(220 関数 13.5 s 対 981 関数 20.8 s。相対尺度。C 経路に残る関数は難しい命令を含むので偏りあり)。大きな C 経路の関数 4 本(f_e95750・f_ddf630・f_e7e470・f_e9d8a0)は 2.3〜6.6 ns/命令で、直 wasm の平均 0.3 ns/命令の 8〜22 倍。決定 0231 の A/B(c 類 37 関数を直にして再生 −28%)とも整合。

| 理由 | b の費用 | 割合 | 削れる見積り | 要る変更 |
|---|---|---|---|---|
| Elastique(opaque な呼び出し・呼び出し由来のフラグ・Rsqrtss・x87 状態) | 5.30 s | 12.6% | −3.5 s(−8.4%) | fe-lift / fe-wasmgen の改良(大) |
| **直にできる(分類 a/b/c、報告 ok)が set U に無い** | **2.90 s** | **6.9%** | **−1.9 s(−4.6%)** | **リストに足すだけ**(下記) |
| x87(Fld・F*・制御語が C 経路の状態) | 2.61 s | 6.2% | −1.7 s(−4.1%) | x87 の wasm 化(大) |
| Cvtps2dq / Cvtpd2dq(fe-wasmgen が未対応) | 1.07 s | 2.5% | −0.7 s(−1.7%) | fe-wasmgen に命令を足す(中。`Cvttps2dq`・`Cvtdq2ps`・`Cvttpd2dq` は `mx_packed2` で対応済み) |
| Jecxz / Cpuid / 32 bit アドレス / サイズ上限 / 他 | 約 1.7 s | 4% | 小 | 個別 |
| Blend2D の RSP 継続入口(17 関数) | 0.34 s(**e では 0.75 s = 8.8%**、d 0.13 s) | 0.8% | e で −0.5 s | fe-wasmgen の継続入口(中〜大) |

(`tables/c-path-by-reason.txt`、関数ごとの表は `tables/c-path-candidates.md`。SSE/x87 エミュレーションは C 経路の費用に含まれる〔別の層として足さない〕。)

### 5.1 最初の 1 手: set U に足す

- 『直にできるのに set U に無い』C 経路の関数 196 本(b+e+d の合計 3.27 s): **本体 ≥20,000 B が 2.66 s**(決定 0239 が first idle を理由に外した 63 本の一部)、mt4 stage A が LOCK を理由に除いた物 0.36 s(stage B のアトミック lowering を待つ)、その他 0.24 s。census(決定 0231 の E 分類)では `f_e95750` は b、報告は ok・73,827 B・LOCK なし。
- 上位: `f_e95750` 1,774 ms(73.8 KB)、`f_ddf630` 143(67.0 KB)、`f_e7e470` 128(111.6 KB)、`f_e9d8a0` 103(23.6 KB)、`f_12c2b80` 77(90.3 KB)、`f_de2280` 67(34.6 KB)、`f_47589d0` 38、`f_6c1490` 35、`f_79e1d0` 32、`f_9ce490` 31。
- **費用**: S1(サイズ): 4 関数で wasm +139,096 B(141,795,291 → 141,934,387 B、+0.098%)。first idle: V8 の Liftoff の同期コンパイルを node 24 で測ると 80 MB/s(5.29 MB の fle の直モジュールで 66 ms)、足す 276 KB で約 3.5 ms。Chrome での first idle の実測は 0251 で行う。
- **実験**: 上の 4 関数だけ足した bundle(`ciE`、2 単位だけ再コンパイル)を、本番と byte 同一の bundle と交互に b 場面で 3 往復(Worker スレッドの CPU、プロファイラなし、45 s): 本番 41.84 / 40.76 / 38.62 s(平均 40.41、標準偏差 1.62)、足した版 37.56 / 41.38 / 39.66 s(平均 39.53、1.92)。**差 −0.88 s(−2.2%)は標準偏差の内**(load 8〜24)。期待値(2.15 s × 2/3 = −1.4 s)と矛盾しないが『効いた』とは言えない。決定 0251 で往復を増やす。

### 5.2 次点: 見積りと設計(実装は次の担当)

- **Cvtps2dq / Cvtpd2dq**(fe-wasmgen、1.07 s 〔b〕、e 0.18 s、c 0.10 s): 5 関数(f_38e6d90〔dsp〕・f_6a74a0・f_6c7ae0・f_6c8910〔FLEngine〕、Cvtpd2dq は f_48b2de0〔Blend2D〕)。`mx_packed2` の機構(MXCSR の窓のガード + RC 依存の丸め + 不定値 0x80000000 + IE/PE フラグ)に命令を足す。見積り −0.7 s(b −1.7%)。
- **x87**(2.61 s): `f_67a160`(Fld、1.54 s)、`f_310fb40`(dsp、x87 制御語が C 経路の状態、0.76 s)、`f_32e9c00`(0.12 s)。80 bit の精度制御語(PC)の扱いを持つ f64 への落とし込みが要る(大)。見積り −1.7 s(−4.1%)。
- **Elastique**(5.30 s): `f_b2f3650`(2.54 s、『calls a function that is opaque』)、`f_b1ce5e0`(0.72 s、呼び出し由来のフラグ)、`f_b034096`・`f_b04a921`・`f_b301c10`〔Rsqrtss〕ほか。fe-lift が呼び出し先の効果を知らない(opaque)ためで、呼び出しのモデルの改良が要る(大)。見積り −3.5 s(−8.4%)。

## 6. メインスレッドに GL を持たせる案(読み。着手しない)

ゲストの面と scene はエンジンの共有メモリ(SAB)にあるので、メインスレッドが同じ SAB を読んで texSubImage2D と描画を行えば、ピクセルは postMessage を通らない(Worker → main は scene の世代の数 B)。Worker は GL の提出とフレームのコピーを持たなくなる(d/e で最大 busy の 40〜50%、ただし実 GPU では小さいはず)。問題: ① Worker が面を書いている最中にメインが読むと tearing(世代の seqlock か更新矩形ごとの double buffer = 小さなコピー)、② メインの 5 MB のアップロードが長いタスクになる、③ 現行は Worker で 1 フレームを作ってから 1 回出す整合性がある。実 GPU で push-frame が小さいなら得は小さい。**依頼者の GTX 980 のビーコン(`desk.drawMs`、`engineRun`)を読んでから**(PM の決定)。

## 7. 本番の新 bundle との突き合わせ

- 本番 `fl-engine-ci-623aba04`(e840e4d4)と、その後の `fl-engine-ci-afd46e67`(HEAD b42fad36 = alpha3。wasm c56d85421032、141,803,497 B = +8,206 B)で b を 1 回ずつ測った(nameless、粗い比): busy 91.7% 対 90.5%、busy の中の wasm 93.4% 対 90.9%、(program) 5.3% 対 7.6%、JS 1.2% 対 1.4%、GC 0.1% 対 0.1%、Worker CPU 40.12 s 対 40.44 s(45.08 s)。差は run 間の幅の内。alpha3(GDI の更新領域)は再生(b)に効かないと読む。

## 8. 未確認

依頼者の機械・実 GPU(特に push-frame・GL の費用)。Firefox の内訳。set U への追加の効果(3 往復では雑音の内)と、追加版の起動 16 スナップ(84 行)・参照 55 行・87 行・a4 の一致、first idle への影響。f_863f60 が処理する文字列の正体、FL の 16 ms 周期の要求(API 記録)。ピッカー定常が ci1 より低い理由。プロファイラの観測効果(サンプリング 5000 回/s のシグナルを受ける Worker スレッドの分)。名前つき版の関数の並びが本番と違うことの性能への影響(粗い比では 2% pt 以内)。

## 9. 出典のない選択(S-gpu2-N)

| 番号 | 選択 | 理由 | 代替 |
|---|---|---|---|
| S-gpu2-1 | 『直 wasm にすると費用が 1/3』で削減を見積る | 測った比 5.2 : 1(平均)・8〜22 倍(大きい 4 関数)の安全側 | 5 倍(−4.1 pt 余計に削れる) |
| S-gpu2-2 | プロファイラのサンプル間隔 200 µs | 場面の長さ(13〜45 s)で 6 万〜15 万サンプル。既定の 1 ms より細かい | 1 ms |
| S-gpu2-3 | 場面の時間(b 45 s、c 20 s、d 12 s、e 21 クリック、a 20 s)と、場面の前の待ち(8 s) | 起動直後の V8 の tier-up と Basic の起動処理の山を避ける | 長く/短く |
| S-gpu2-4 | busy = (idle) 以外(『(program)』を含む) | V8 の native の時間も Worker が忙しい時間 | (program) を除く(d/e の busy が半分になる) |
| S-gpu2-5 | 層の分類規則(`tools/prof-analyze.py`):名前が link map にあれば C 経路、`fle_mx_*`/`round_pack`/`fle_x87_*` は SSE/x87 エミュレーション、`fle_ld/st/cc/op_*/cas_*` は補助 op | PM の層の指定に合わせた | 別の線引き |
| S-gpu2-6 | viewport 2260x1116(ゲスト 1920x1080) | 前の担当の測定と同じ画面 | 他の画面 |
| S-gpu2-7 | A/B は 3 往復(load 8〜24) | 時間(1 往復 9 分) | 6 往復以上(0251) |
| S-gpu2-8 | 名前つき版の関数の並びが本番と違うことを許す | `-g` なしの CI の出力を名前つきにする方法が無い(Binaryen が並びを名前で決める) | 本番の index を本体の hash で名前に引き直す(約 1 時間の追加) |

## 10. 規則の点検と自己申告

- 余計な処理の追加: 0(測定の道具だけ。製品の木・PM の木に書いていない。`fe-wasmgen --names` と `build-engine-bundle.sh` の `-g` は自分の複製の木 `lift/out-gpu2/src` だけ)。オプション: 追加 0。依頼者への操作・資格情報・決定の依頼: 0(配信バケットの読み取りは wrangler の既存ログインの list/get だけ。書き込み 0)。CI に実ブラウザを置いていない。第三者サービス 0。
- **自己申告**: ① 実行中の CI の bundle 段のスクリプト(`build-engine-bundle.sh`)を測定用に書き換え、実行中の段が構文エラーで失敗した(自分の木の bundle 段だけ。PM の木は触っていない。以後 snapshot から走らせる)。② 実験用の木 `srcE` を src の複製で作り、測定用に編集したスクリプト(-g と --names)が入って `fe-wasmgen` が usage で落ちた(2 回。元の版に戻して再実行)。③ `tools/mk-inc.sh` の段が古く(`hybstrip` はもう無い)2 回失敗。④ CDP で動作中の Worker に後付けでアタッチして 5 分待った。⑤ A/B は共有の load の下で 3 往復だけ。⑥ 前の担当(0240)の `( cmd & )` の違反は今回は無い。

## 11. 再現手順

`tools/` に一式(`README.md`)。要点: `bash tools/run-ci-build.sh <git archive の木> <ciwork> F2xH7` で冷間ビルド、`tools/mk-names-wasm.sh` の流れ(`ciPN`/`ciPN2` の作り方は README)、`python3 tools/mk-funcmap.py` / `mk-idxmap.py` / `pe-ranges.py` で表、`tools/mk-assets.sh NAME BUNDLE` + `tools/preview.sh NAME PORT` で配信、`tools/run-scenes.sh TAG PORT a b c d e` で 5 場面、`python3 tools/prof-analyze.py PROFILE --top 30 --split roots --owners --incl`、`tools/mk-scene-table.py`、`tools/prof-cands.py … --reasons`、`tools/run-ab.sh ROUNDS TAGA PORTA TAGB PORTB`。

## PM の決定と取り込み(2026-11-05 18:35)

- 文書・表・道具だけの取り込み(製品の木への diff は 0 行)。400 KB 超の `funcmap.tsv.gz`・`idxmap.tsv.gz` は置かず、`tools/README` の作り方で再生成する。PM は gpu1 の測定を再現していない(この機械、Chrome 156、実 GPU なし、依頼者の機械は 0 件)。
- 採る読み: 再生中の UI Worker は 94% 忙しく、C 経路の関数が引き起こす費用は busy の 32.2%(うち SSE/x87 エミュレーションの 92.7% は C 経路から)。最初の 1 手は「直 wasm にできるのに set U に無い関数」の追加(f_e95750 ほか。実験の 4 関数で wasm +139,096 B、Worker CPU は −0.88 s で run 間のばらつきの内)。次の担当は決定 0251(gpu1、`lift/out-git/gpu3-results/`)。
- alpha3(b42fad36)は再生(b)には効かない読み(gpu1 の本番 afd46e67 での b の測定: Worker CPU 40.12 s 対 40.44 s は run 間の幅の内)と矛盾しない。
- d/e の「(program)」42〜47% = WebGL の push-frame(SwiftShader の費用)は、依頼者の GTX 980 の beacon(`desk.drawMs`/`engineRun`)を読むまで手を付けない(依頼者には頼まない)。
