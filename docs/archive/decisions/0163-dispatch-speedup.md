# 0163: `fle_dispatch`(ゲストの間接呼び出しの宛先 VA → 関数)の探索を、結果を変えずに速くする — 二分探索の前に直接写像キャッシュ(runtime の `fle_dispatch.h` だけ)。定常の主ウィンドウの CPU が native -O2 で −22%、wasm -Oz で −19%(中央値)。起動(P1)にはほぼ効かない。すべて仮の決定

状態: **仮の決定**。依頼は PM(決定 0151 hle1 の「次の候補の 1 番」)。実装・測定・文書は作業者(`disp1`)。作業は HEAD **d33d2cd** の `git archive` から始め、差分 `disp1.diff` は最新の HEAD **5c77e44**(決定 0162・0157 を含む)の `git archive` の木に `patch -p1 --dry-run` が当たることを確認した(`logs/patch-dry-run.txt`)。`lift/runtime/fle_dispatch.h` は d33d2cd から 5c77e44 まで変わっていない。**触る既存ファイルは `lift/runtime/fle_dispatch.h` の 1 つだけ。fe-lift・生成 C・`lift/crates`・他の runtime ファイル・ビルドスクリプトは 0 行。**

区分の凡例は決定 0121〜0152 と同じ。**実測**＝作業者のコマンドの出力を見た事実(2026-11-03 JST。PM は下の数字を再測定していない)。**読み**＝コード・文書を読んだ解釈で、走らせていない。**出典なしの選択**＝根拠が無く作業者が選んだ値(`S-disp1-…`、§9 に撤回条件つき)。**未確認**＝確かめていない。

## 0. 先に書くこと(進捗の誠実性、AGENTS.md / `docs/policies/progress-integrity.md` に従う)

**(1) 実エンジンが実行された事実(実測)**: lift した実 FLEngine_x64.dll と同梱 DLL 5 つ(dsp_ippv2・Blend2D・QFC・FreeType・ILTools)の生成 C(FLEngine の SHA-256 `6d39d04f…`、HEAD の fe-lift、FRUITY on、SEH on)を、旧実装と新実装のそれぞれで native -O2(clang)と wasm -Oz の全単位にコンパイルして実際に実行した。実行したのは `cmp-short.g3`(最初の idle → 収束 → Enter → 閉じるアニメ。16 スナップ)と `steady-main.g3`(最初の idle → 3 仮想秒 → Enter → 3 仮想秒 → 主ウィンドウ 60 仮想秒)。宛先分布は、旧実装のビルドに計数プローブ(`fle_dispatch_hook` 経由)を足して `steady-main.g3` を 3 回実行して取った。**所定のプロジェクト(`serina1-serum2.zip`)は開いていない**(アプリに FLP を開く経路が無い。`docs/milestones/performance-targets.md` §3)。

**(2) 代役・再生・推測が含まれる範囲**: エンジンの数字(§3 の分布、§5 の同一性、§6 の速さ)に代役・再生・事前生成物は無い。**§4 の「リプレイ微ベンチ」だけは別物**: 実エンジンの走行から録った宛先の列(定常 30M 件・起動 10.7M 件)を、同じ表に対する各探索方式に流す微測定で、エンジン全体の速さではない(エンジンのキャッシュ圧が無いので、旧の二分探索の実エンジンでの費用より小さく出る)。**差分試験**(§5)は表の VA の集合に対する関数の比較で、エンジンの実行ではない(生成 C の表の VA は実の統合テーブルから取り、各行に別々の stub 関数を付けた)。

**(3) HLE**: 無し(旧も新も。決定 0162 の dsp の HLE は入っていない。HEAD d33d2cd の基準)。

## 1. 結果(要約)

| 項目(すべて HLE なし、同じ生成 C) | 旧(二分探索) | 新(キャッシュ) | 差 |
|---|---|---|---|
| native -O2: 主ウィンドウ 60 仮想秒の CPU 秒(9 回交互の中央値、load 7〜10) | 3.19 s | **2.49 s** | **−22%**(18.8 → 24.1 仮想秒/CPU 秒) |
| native -O2: 最初の idle までの CPU 秒(同じ 9 回) | 3.48 s | 3.27 s | −6%(ラン間の幅 3.1〜3.7 s。別の 5 回の組〔最初のヘッダ版〕では 3.51 → 3.52 で差なし。**雑音と区別しきれない**) |
| native SIGPROF 500 Hz: 定常の `fle_dispatch` の自己割合 | 24.5% / 25.2% / 26.8% | 0.0〜0.2%(インライン化されて呼び元に移った。**新経路の費用が 0 の意味ではない**) | CPU 秒の差が実質 |
| wasm -Oz(node 主スレッド CPU、6 回交互の中央値): 主ウィンドウ 60 仮想秒 | 20.16 s | **16.35 s** | **-19%**(2.98 → 3.67 仮想秒/CPU 秒。1 フレーム 8.33 ms あたりのエンジンの CPU 2.80 → 2.27 ms) |
| wasm -Oz: 最初の idle までの主スレッド CPU | 21.30 s | 21.02 s | -1%(ラン間の幅の内) |
| wasm -Oz の大きさ | 320,882,760 B | 320,891,437 B | +8,677 B(+0.003%) |
| native -O2 の text | 314,166,573 B | 324,198,301 B | +10.0 MB(+3.2%。インライン化) |
| 差分試験(旧 vs 新、実の統合テーブル 112,801 行) | - | 54,560,245 件ずつ(native -O2、wasm -O2)ほか | 不一致 **0** |
| 16 スナップ(`cmp-short.g3`) | 84 行(native)/ 82 行(wasm -Oz) | 同一 | バイト一致(hle1 の基準 84 行とも一致) |
| `test.sh full`(FLE_TEST_NOCACHE=1、142 executables を実際に実行) | (旧は実行していない。PM が示した基準 145 results・2,090 passed・0 failed) | **145・2,090・0**(SKIPPED 1 行は基準と同じ) | 基準と同一 |
| FRUITY 無効 a4 | (旧は実行していない。基準 942/942・895/895・functions 5,310・entries 1,009,731) | 同じ値 | 基準と同一 |

読み: **P2(定常の UI)に効く**: 定常の主スレッドの 1/4 を占めていた `fle_dispatch` が消え、native で −22%、wasm -Oz で -19%。ただし wasm -Oz で 1 フレーム 8.33 ms の仮想時間に使うエンジンの CPU は 2.3 ms(旧 2.8 ms)で、描画・提示・操作の遅延を含まない。残りの定常の支配項は VCL の木の歩き(SIGPROF の新: f_410010 22〜25%、f_863f60 12〜16%、f_6113f0・f_50aa70 各 8〜9%、f_863f20 4〜6%、f_5287c0・f_40fe90 5〜7%・4〜5%。決定 0151 §Q のとおり)で、次の標的は InheritsFrom 系の HLE と木の歩き全体。**P1(起動)にはほぼ効かない**(native −6% と 0% の二組、wasm -1%。起動の `fle_dispatch` は自己 4〜5% しかなかった)。**P3(音)・所定のプロジェクトは未測定。**

## 2. 現状の読み(実測と読み)

- `fle_dispatch(va)`(`lift/runtime/fle_dispatch.h`)は、ゲストの `call [reg]` / `jmp [reg]` の宛先 VA から関数 `fle_dispatch_table[]`(VA で整列した `{va, fn}` の表)を二分探索し、見つかれば進捗カウンタ(決定 0147)を 1 増やし、hook(決定 0097)があれば hook に渡し、無ければ関数を呼ぶ。見つからなければ `fle_dispatch_miss`(`fle_win_try_thunk` を試し、だめなら名前付きで停止)。呼ぶ前に landing(決定 0100)を調べる。呼び出し点は、fe-lift の `dispatch.rs`(`call`)・`tailjmp.rs`(`jmp`)・`hook.rs`・runtime の `fle_callback.c`(`fle_call_guest`)・`fle_win.c`・`fle_seh.h`。**生成 C の `fle_dispatch(` は FLEngine 44,180、m1〜m5 が 3,759 / 2,275 / 2,322 / 773 / 5,299、計 58,608 個**(grep)。
- 表は 1 つ(`gen_two.sh` の merge が FLEngine と 5 モジュールを統合した `common.c`)で、**112,801 行**(VA の範囲 0x403579〜0x7ffb00000030。611 行は API 擬似 VA の領域 0x7ff9…、16 の倍数の VA が 100,683 行 = 89.3%)。表は `const`(実行時に書き換える所は grep で見つからない)。
- 旧実装は `static inline` だが、native -O2 では 1 翻訳単位に 1 本の `static` 関数 `fle_dispatch`(nm の `t`、0x121 B)になる(呼び出し点にはインライン化されない。nm と sed で確認)。17 段の二分探索の命令は約 200、17 本の依存した読み出しを 1.8 MB の表に対して行う。
- 決定 0151 の測定(定常の主ウィンドウの主スレッドの自己時間の 29%、起動の主スレッドの標本の 5.7〜6%)は、この作業の旧ビルドでも再現した: SIGPROF 500 Hz(`tools/disp1/disp_prof.c`)で `fle_dispatch` の自己割合は**定常 24.5% / 25.2% / 26.8%(全スレッド)、起動 4.0% / 4.6% / 3.9%**(旧 3 回)。
- 協調スレッド(決定 0147・0122): ucontext(native)/ JSPI(wasm)で、切り替えは待機/yield の API と `fle_progress_slice()` の中だけ(`fle_thread.c`、`fle_thread_wasm.c` を読んだ。**読み**)。キャッシュへの書き込み(`fn`、次に `va`)の途中で切り替わる箇所は無い。同時に走る実スレッドは無い。

## 3. ゲストの間接呼び出しの宛先分布(実測)

旧ビルドの native -O2 に `tools/disp1/disp_stat.c` を足して(`fle_dispatch_hook` で `(va, fn)` を受け、呼び出し点を `[R[4]]` の戻り番地で数える。`jmp` の末尾呼びでは近似)、`steady-main.g3` を 3 回実行した。phase は仮想時計(ms)3601776 / 3607800 / 3667770 で切った。表は `tables/dispatch-distribution.tsv`・`dispatch-top-targets.tsv`・`cache-miss-simulation.tsv`。

| phase | 呼び出し | 異なる宛先 | 異なる呼び出し点 | 前回と同じ宛先(単段 IC) | 見つからなかった VA |
|---|---|---|---|---|---|
| 0 起動(最初の idle まで) | 10,772,479 | 3,763 | 4,618 | 89.50%(2 段 96.33%) | 331 |
| 1 ウィザード + Enter + 閉じるアニメ | 7,131,136 | 843 | 1,811 | 99.17% | 6,184 |
| 2 主ウィンドウ 60 仮想秒 | 79,687,680 | **84** | **103** | 99.91% | 61,490(0.08%) |

- 定常は宛先が 84 個で、上位 3 つ(f_5d76b0・f_528f20・f_5287c0)が 97.6%。VCL の木の歩き(決定 0151 §Q)に一致する。
- 見つからなかった VA は、MakeObjectInstance の thunk(ウィンドウ手続き)と COM のメソッドスロット(`fle_win_try_thunk` / `fle_com_try_slot`)。定常で 1 秒に約 1,000 回。
- 直接写像キャッシュの **simulated miss 率**(旧ビルドの走行中にプローブが数えた。キャッシュの中身は phase をまたいで保つ。phase 0 が最も不利): 4096 ライン(2^12)で `(va>>2)` 0.613%、`(va>>4)` 0.077%、畳み込み 0.326%、乗算ハッシュ 0.280%、2-way 4096 ラインの `(va>>2)` 0.224%。phase 2 は 4096 ラインのどの索引でも 0.011% 以下(`va>>4` は 0.002%)。宛先が 3,763 個なので下限(初回 miss)は 0.035%。
- 1 呼び出し点あたりの宛先は定常で 99.9% が単一(単段 inline cache の命中)だが、起動では 89.5% に落ちる(多態の呼び出し点が多い)。

## 4. 設計の比較と選択

測るものは「VA → 関数ポインタ」の探索だけ(進捗カウンタ・hook・landing は全案で同じ)。命令数は clang -O2 の x86-64 のリストから数えた(`tables/designs-instructions.tsv`、`tools/disp1/bench/designs.c`)。

| 案 | 命中経路の x86-64 命令数 | 追加のメモリ | 作るとき | 読み |
|---|---|---|---|---|
| 旧: 二分探索 | 約 200(17 段×11〜12 + 14) | 無し(1.8 MB の表を読む) | - | 定常の主スレッドの自己 25〜29% |
| **(a) 直接写像キャッシュ(採用)** | 12 | 64 KB(bss) | 遅延(miss で埋める) | 定常の命中 99.99%、起動の命中 99.92%(simulated) |
| (a2) 2-way | 12〜17 | 64 KB | 遅延 | 起動の simulated miss 率は `va>>4` で 0.042%(1 方向版は 0.077%)。差は 0.035 ポイントで、命令が増える割に得が小さい |
| (b) 完全な開番地ハッシュ(行数の 2 倍) | 16(衝突でループ) | 4 MB | 起動時に 112,801 回の挿入 | 命中率 100% だが、(a) より約 1.4 倍遅い(リプレイ) |
| (c) 64 B ブロックの 2 段表 + 短い走査 | 17 + 走査 | 6.4 MB | 起動時 | 約 2 倍遅い(リプレイ)。擬似 VA の領域は別扱い |
| (d) 呼び出し点ごとの inline cache | 8 | 58,608 点×16 B = 0.94 MB | 生成 C の変更が要る | 定常 99.9%、起動 89.5%。fe-lift の変更が要るので不採用 |

リプレイ微ベンチ(`tools/disp1/bench/bench.c`。実エンジンの走行から録った宛先の列を、同じ表に対する各方式に流す。スレッド CPU、5 回の最小。load は 5〜16 なので ns の絶対値は参考。ループと参照の読み 0.30 ns〔wasm 0.7〜0.8 ns〕を含む)。全表は `tables/replay-bench.tsv`:

| 方式 | native -O2 定常 | native -O2 起動 | wasm -O2 定常 | wasm -Oz 定常 | wasm -Oz 起動 |
|---|---|---|---|---|---|
| 旧: 二分探索 | 7.97 ns | 9.17 ns | 44.17 ns | 28.65 ns | 29.71 ns |
| 直接写像 4096、`va>>4`(採用) | 0.51 | 0.59 | 1.48 | 1.49 | 1.62 |
| 直接写像 4096、`va>>2` | 0.53 | 0.72 | 1.51 | 1.56 | 1.95 |
| 直接写像 4096、畳み込み | 0.78 | 0.92 | 1.78 | 1.83 | 2.05 |
| 直接写像 4096、乗算 | 0.75 | 0.97 | 1.60 | 1.77 | 1.93 |
| 2-way 4096 | 0.52 | 0.72 | 1.57 | 1.75 | 1.98 |
| (b) 完全ハッシュ | 0.85 | 1.12 | 2.16 | 2.24 | 2.70 |
| (c) 2 段表 | 0.97 | 1.35 | 3.15 | 3.21 | 3.66 |

**選択(出典なしの選択を含む。§9)**: (a)。4096 ライン×16 B = 64 KB、索引は `(va >> 4) & 4095`(= バイトオフセット `va & 0xFFF0`。x86-64 は `and` 1 つ、wasm は `i32.and` 1 つ)。`fn` が NULL の行は写さない。命中の条件は `e->va == va && e->fn != 0`(全ゼロの初期状態が `va == 0` に当たらないように。表に VA 0 の行があっても、その行は NULL でなければ写される)。miss は out-of-line の `fle_dispatch_find`(旧の二分探索そのまま)で、見つかった行を写す。**見つからない VA に否定キャッシュは付けない**(定常で 0.08%、キャッシュが `{va, NULL}` を持てば結果は同じだが、要らない)。`FLE_DISPATCH_BSEARCH` を定義すると旧実装がそのまま入る。`FLE_DISPATCH_CACHE_BITS` で大きさを変えられる(既定 12)。

安全性: キャッシュは const の表の行の写しなので、中身が古くなることがない(表を書き換える所が無い)。全協調スレッドで 1 つを共有し、ロックは無い。切り替えは書き込み(`fn`→`va`)の途中では起きない(§2)。**実スレッド(同時に走る)を足すなら 1 本ごとに 1 つ要る。** 実スレッドでは、読み手が `va` を見てから `fn` を読む間に書き手が別の行を書くと、誤った `fn` を読む(順序の問題)。

## 5. 同一性(実測)

「結果が同一」＝同じ VA に対して、同じ関数、または同じ miss(`fle_dispatch_miss` に同じ VA が渡る)。加えて進捗カウンタの増分(見つかったときだけ 1)・slice 関数の呼び出し・hook が受ける `(va, fn)`・hook 無しの呼び出しも同一。

**(a) 差分試験**(`tools/disp1/difftest/`): 旧(`-DFLE_DISPATCH_BSEARCH`)と新を同じプログラムに入れ、各 VA を両方に通し、独立な `bsearch` の結果とも比べる。表は実の統合テーブルの VA 112,801 行(各行に別々の stub)。入力: 全入口(昇順・降順・再)、入口 ±1〜64 の全部、入口の 64 ビットそれぞれの反転(キャッシュの同じラインに落ちる別 VA)、0・1・UINT64_MAX・2^32・表の両端の前後、入口の乱択を 3 回(競合による追い出し)、64 入口の hot set を乱択の割り込み付きで 2×10^6、**一様な 64 ビット乱数 10^7、画像範囲の乱数 10^7、入口 ± 3 の乱数 10^7**。

| 構成 | 件数 | 不一致 |
|---|---|---|
| native -O2(clang)、10^7 | 54,560,245(見つかった 4,809,479 / 見つからない 49,750,766 / hook 無し 6,820,030) | **0** |
| wasm -O2(emcc、node)、10^7 | 54,560,245 | **0** |
| native -O0・-O1・-O3・gcc -O2、wasm -Oz(3×10^6) | 各 33,560,245 | **0** |
| キャッシュ 2 / 256 / 65,536 ライン(-O2) | 各 33,560,245 | **0** |
| エッジ表(VA 0 の行・`fn==NULL` の行・UINT64_MAX の行・高い擬似 VA。native -O2、10^5) | 2,301,245 | **0** |

キャッシュの後ろに番兵のライン 8 本を置き、試験の終わりに変わっていないことも確かめる(範囲外書き込みの検出)。**この表の全構成は最終版のヘッダでやり直した結果**(`logs/difftest-real-O2-final.txt`・`difftest-edge-O2-final.txt`・`difftest-configs-final.txt`)。最初のヘッダ版〔末尾を共有する前〕の結果も同じ件数・不一致 0 だった(`logs/difftest-configs.txt`・`difftest-wasm-O2.txt`)。

**(b) 実エンジンの 16 スナップ**(`cmp-short.g3`): native -O2 の新は旧と 84 行でバイト一致、かつ hle1 の基準 `native-base-84lines.txt` ともバイト一致(面 SHA: idle `a970cb0a…`、収束 `4fa213b0…`、Enter 後 `dd416725…` を含む)。wasm -Oz(node)の新は旧と 82 行でバイト一致で、native 新の 84 行から `g3-layered` の 2 行(node には無い)を除いた物と差 0。wasm -O2 の 16 スナップは未実施(§10)。 ログ: `logs/native-O2-new-cmp-short-84lines.txt`・`native-O2-base-…`・`wasm-Oz-new-cmp-short-82lines.txt`・`wasm-Oz-base-…`。

**(c) 変異検査**(`tools/disp1/difftest/mutants.sh`。ヘッダの複製に sed で 1 か所ずつ壊し、差分試験が不一致または異常終了を出すか): 16 件のうち **14 件を検出**(タグを下位 32 ビットだけで比較 / `fn≠0` の検査の削除 / 埋める関数を 1 つ前の行にする / 埋めるタグを `va|1` にする / キャッシュの添字の上限が 1 つ多い〔番兵で検出〕/ miss を判定しない〔異常終了〕/ hit で進捗を飛ばす / hit で hook を飛ばす / miss を握りつぶす / 命中条件の `&&` を `||` にする / 二分探索が行 0 を飛ばす / 上限を 2 減らす / 見つかる前に埋める / 埋める関数を 1 つ後の行にする)。**2 件は等価変異で検出されない**: 二分探索の `hi = count-1`(`lo` の下限の探索なので結果が変わらない)、NULL 関数の行も埋める(命中条件が `fn≠0` なので結果が変わらない)。1 件(添字の上限)は、最初の差分試験(番兵なし)では**見逃し**、番兵を足して検出した。ASan は `libclang_rt.asan` が無く使えなかった。表: `tables/mutants.tsv`、ログ: `logs/mutants*.txt`。

**(d) 既存テスト**: `bash lift/harness/test.sh full`: **results=145 passed=2090 failed=0 SKIPPED-lines=1**(基準と同じ。`FLE_TEST_NOCACHE=1`、142 executables を実際に実行、`logs/fulltest3-nocache.txt`)。**最初の実行は 1 件失敗した**(`fe-lift/wall_fr20::the_progress_counter_is_in_the_one_inline_function_that_every_indirect_call_ends_in`。ヘッダに `++fle_progress_count` が 2 つあったため。テストは変えず、ヘッダの形を直した。§11、`logs/fulltest-run1-before-restructure.txt`)。。FRUITY 無効 a4: `OUT=… bash lift/harness/a4-fruity.sh off`(gen 13 s、build 201 s): init_count 942/942、run_rank 895/895、functions 5,310、entries 1,009,731(基準と同じ。`logs/a4-off.txt`)。。

## 6. 速さ(CPU 秒。壁時計は使っていない)

測り方: `tools/hle1/run-native-hle1.sh` / `wasm-ab.sh` の `steady-main.g3`。native は `/proc` の CPU(user+sys、1 OS スレッド)、wasm は node の `process.threadCpuUsage()`(主スレッド)。旧と新を交互に回し、中央値。「主ウィンドウ 60 仮想秒」は e03 → e63 の間の CPU、「最初の idle」は wiz0 までの CPU。

### 6.1 native -O2(clang)、`steady-main.g3`、旧と新を交互に 9 回(`tools/disp1/ab_native.sh`・`ab_summary.py`。load 7〜10、`logs/ab-native-2.txt`)

| | 最初の idle までの CPU 秒 | 主ウィンドウ 60 仮想秒の CPU 秒 |
|---|---|---|
| 旧(中央値) | 3.48 [3.35 3.17 3.24 3.58 3.70 3.48 3.36 5.45 3.58] | 3.19 [3.24 2.89 3.19 3.28 3.40 3.15 3.10 5.24 3.05] |
| 新(中央値) | 3.27 [3.27 3.10 3.26 3.26 3.44 3.29 3.75 4.75 3.25] | 2.49 [2.42 2.33 2.59 2.49 2.58 2.50 4.47 2.38 2.34] |

(各組に 1 つずつ外れ値〔旧 5.45・5.24、新 4.75・4.47〕があり、同じ時期に他の担当の負荷が重なった回と読む。中央値は影響されない。)最初のヘッダ版(`va>>2` でなく、`fle_dispatch` の末尾を共有する前の形。5 回、load 6〜12、`logs/ab-native-1.txt`)では 主ウィンドウ 3.44 → 2.72 s(−21%)、最初の idle 3.51 → 3.52 s。

SIGPROF(500 Hz、`disp_prof.c`、`logs/prof-ab.txt`・`prof-new2.txt`)の定常の標本数(1 走行の主ウィンドウ 60 仮想秒): 旧 1,742 / 1,633 / 1,598、新 1,247 / 1,198(最終版)・1,315 / 1,323(最初のヘッダ版)。`fle_dispatch` の自己割合: 旧 起動 4.0 / 4.6 / 3.9%、ウィザード〜Enter 2.7 / 3.4 / 3.1%、定常 24.5 / 25.2 / 26.8% → 新 いずれも 0.0〜0.3%(`fle_dispatch_find` のみ。`fle_dispatch` はインライン化されて呼び元に入った)。表: `tables/prof-share.tsv`。

### 6.2 wasm -Oz(node 24、`process.threadCpuUsage()` の主スレッド CPU)、`steady-main.g3`、旧と新を交互に 6 回(`wasm-ab.sh`、`wasm_agg.py`。load 3〜8、`logs/wasm-ab-1.txt`・`wasm-ab-2.txt`)

| | 最初の idle までの CPU 秒 | 主ウィンドウ 60 仮想秒の CPU 秒 |
|---|---|---|
| 旧(中央値) | 21.30 [21.9 21.5 21.1 20.9 20.6 27.3] | 20.16 [20.2 22.1 19.3 19.2 20.2 20.1] |
| 新(中央値) | 21.02 [22.7 21.0 20.9 20.4 21.4 21.0] | 16.35 [16.0 17.2 17.2 16.0 16.4 16.3] |

(新の wasm は、最初のヘッダ版〔`wb-new`〕と最終版〔`wb-new2`〕でバイト一致。6 回はその 2 つの組を合わせた。)

### 6.3 リプレイ微ベンチ

§4 の表。エンジン全体の速さではない。

## 7. 大きさ

| | 旧 | 新 | 差 |
|---|---|---|---|
| wasm -Oz 全体(`flengine6.wasm`) | 320,882,760 B | 320,891,437 B | +8,677 B(+0.003%) |
| native -O2 の実行ファイル | 320,040,392 B | 330,074,952 B | +10,034,560 B(+3.1%)。text 314,166,573 → 324,198,301(+3.2%)、bss 17,719,964 → 24,273,628(+6.55 MB) |
| `fle_dispatch`(翻訳単位あたりの関数。nm、fle-00.o) | 0x121 B | 0xb1 B(+ `fle_dispatch_find` 0x8b B + `fle_dispatch_miss` 0x3c B) | 関数 1 つは小さくなり、-O2 の native ではインライン化される(text の増加) |
| 1 関数の wasm 命令数(`emcc -O2 -S`、1 関数全体) | 138 | 75 | - |

- **native の bss +6.55 MB は欠陥**: キャッシュの配列 64 KB を、各翻訳単位が weak で持ち、ELF のリンカは weak の重複した定義の `.bss` の領域を落とさない(`-fdata-sections` が無いので 1 つの `.bss` に入る)。使われるのは 1 つだけ(触れられない領域なので RSS には効かないはず)が、仮想メモリ 6.5 MB を余分に取る。wasm-ld は重複した weak の定義を落とす(ファイルは +8.7 KB のみ)。直すなら `__attribute__((common))` か、1 つの .c への定義(S-disp1-5 の撤回条件)。**この修正は測定のあとの変更になるので入れていない。**
- `maxrss`(`/usr/bin/time`)は、native の新で旧より 5〜29 MB 多い値が出た(9 回のうち 7 回で +26〜29 MB、2 回で +5 MB)。原因は調べていない(text が +10 MB・実行時の割り当ての揺れ・上の bss のどれかは未確認)。
- 呼び出し点の個数は変わらない(58,608)。生成 C は 1 バイトも変えていない。

## 8. 再現

```
R=<repo root>; W=$R/lift/out-disp1; mkdir -p $W/src; git -C $R archive d33d2cd lift docs tools app | tar -x -C $W/src        # 基準の木(旧)
cp -r $W/src/docs/investigations/hle1/tools $W/tools; grep -rl out-hle1 $W/tools | xargs sed -i 's#out-hle1#out-disp1#g'  # hle1 の測定の道具(env.sh の W)
cp -r $W/src $W/work; (cd $W/work && patch -p1 < <この決定の disp1.diff>)                                       # 新の木
. $W/tools/env.sh; (cd $W/src/lift && cargo build --release --offline -p fe-pe -p fe-lift)                        # CARGO_TARGET_DIR=$W/target
SEH=1 bash $W/tools/gen-final.sh $W/gen base ; bash $W/tools/resplit.sh $W/gen $W/wsrc 10000000                    # 生成 C(旧新共通。FLEngine の SHA 6d39d04f…)
# native -O2: 旧と新(RTDIR だけ違う)
WSRC=$W/wsrc OBJ=$W/obj-base OPT=O2 JOBS=4 RTDIR=$W/src/lift/runtime bash $W/tools/native-compile.sh     # 約 20〜38 分
WSRC=$W/wsrc OBJ=$W/obj-new2 OPT=O2 JOBS=4 RTDIR=$W/work/lift/runtime bash $W/tools/native-compile.sh
LIFT=$W/src/lift OBJ=$W/obj-base  REALMOD_C=$W/gen/v-base/native/realmod_table_gen.c PTOOLS=$W/tools/probe P2=0 BIN_NAME=two-native-base  bash $W/tools/build_native_perf3.sh
LIFT=$W/work/lift OBJ=$W/obj-new2 REALMOD_C=$W/gen/v-base/native/realmod_table_gen.c PTOOLS=$W/tools/probe P2=0 BIN_NAME=two-native-new2 RTDIR=$W/work/lift/runtime bash $W/tools/build_native_perf3.sh
bash <disp1>/tools/ab_native.sh 9 runs/ab base=$W/obj-base/two-native-base new=$W/obj-new2/two-native-new2; python3 <disp1>/tools/ab_summary.py runs/ab base new
# 宛先分布: clang -c disp_stat.c; link_variant.sh $W/obj-base two-native-stat $W/obj-base/disp_stat.o;  DISP_PHASES=3601776,3607800,3667770 DISP_TRACE_FILE=... を ENVX に渡して steady-main.g3 を走らせる
# SIGPROF: link_variant.sh $W/obj-base two-native-prof disp_prof.o (obj-new2 も); ENVX="DISP_PROF_FILE=... DISP_PROF_HZ=500"; prof_report.py <binary> <samples> 3601776,3607800
# wasm -Oz: SRCTREE=$W/src WB=$W/wb-base NAT=gen WASM_OPT=Oz JOBS=4 bash $W/tools/wasm.sh rt harness units link   (新は SRCTREE=$W/work WB=$W/wb-new2)
#           SCRIPT=$W/tools/scripts/steady-main.g3 bash $W/tools/hle1/wasm-ab.sh 3 wab base:wb-base new:wb-new2 ; python3 <disp1>/tools/wasm_summary.py wab base,new
# 差分試験: python3 difftest/gen_table.py $W/gen/v-base/src/common.c table_real.c ; RT=$W/work/lift/runtime OUTD=$W/dt bash difftest/build.sh real-O2 ; $W/dt/real-O2 10000000
#           mutants: RT=... OUTD=... TABLE_OBJ=<table.o> bash difftest/mutants.sh
# テスト: FLE_INPUT_ROOT=$R bash $W/work/lift/harness/test-env.sh --link ; (cd $W/work && FLE_TEST_NOCACHE=1 FLE_INPUT_ROOT=$R bash lift/harness/test.sh full --jobs 2)
# a4: OUT=$W/a4off INPUT_LIFT=$R/lift FEBIN=$W/target/release JOBS=2 bash $W/work/lift/harness/a4-fruity.sh off
```


## 9. 出典なしの選択と撤回条件

| ID | 選択 | 根拠 | 撤回条件 |
|---|---|---|---|
| S-disp1-1 | キャッシュ 4096 ライン(64 KB) | `steady-main.g3` の 1 本の走行での simulated miss 率(§3)。所定のプロジェクトでの走行は未測定 | 所定のプロジェクトで miss 率が 1% を超える |
| S-disp1-2 | 索引 = `va & 0xFFF0`(ビット 4〜15) | 16 の倍数の VA が行の 89.3%、simulated miss 率が `va>>2`・畳み込み・乗算より低い(起動 0.077%)。16 バイト未満の間隔の入口は同じラインを奪い合う | 別のモジュールの構成(入口の整列が違う)で miss 率が上がる |
| S-disp1-3 | 否定キャッシュ無し | 見つからない VA は定常で 0.08% | 所定のプロジェクトで thunk / COM スロットの呼び出しが 1% を超える |
| S-disp1-4 | 全協調スレッドで 1 つを共有、ロック無し | 協調スレッドは書き込みの途中で切り替わらない(読み) | 実スレッドの導入 |
| S-disp1-5 | キャッシュの定義をヘッダの weak にする(`fle_progress_count` と同じ型) | 別の .c を足すと、`fle_*.c` を数個だけリンクする既存のテストが未定義参照になる | ヘッダの weak が wasm-ld / 将来の linker で重複エラーになる |
| S-disp1-6 | `fle_dispatch` のインライン化はコンパイラ任せ(noinline にしない) | native -O2 で定常 −21%(§6)。text +3.2% は wasm の大きさ(S1)に関係しない | wasm -O2 / 将来の直接生成でインライン化が大きさを悪化させる |

## 10. 確認できなかったこと

- **所定のプロジェクト(`serina1-serum2.zip`)での宛先分布・miss 率・速さ**: 未測定(アプリに FLP を開く経路が無い)。§3 の分布は `steady-main.g3`(ウィザードを出して Enter、空の主ウィンドウ)のもの。プロジェクトを開くと宛先が増え、4096 ラインの miss 率が上がる可能性がある(撤回条件 S-disp1-1)。
- **ブラウザ(Chrome)での測定**: 無い。wasm の数字は node(V8、`process.threadCpuUsage()`)の主スレッド CPU。
- **起動(P1)への効き**: native の最初の idle までの CPU 秒は旧と新で差が読み取れなかった(§6。ラン間の幅が ±0.3 s で、見込みの効果 0.1 s 前後は雑音に埋もれる)。wasm も −1% で幅の内。起動のうち `fle_dispatch` の割合は SIGPROF で 4.0〜4.6% と測れているが、新での削減が CPU 秒の差として確認できていない。
- **wasm -O2 の全体ビルド・A/B**: 未実施(時間。全 92 単位の wasm -O2 は約 25 分かかる見込みで、基準だけ 26 単位まで作って止めた)。wasm -O2 で行ったのは差分試験(54,560,245 件、不一致 0)とリプレイ微ベンチだけ。-O2 では小さくなった `fle_dispatch` がインライン化され、wasm の大きさが増える可能性がある(native -O2 は text +3.2%)。
- **`fle_dispatch` を noinline にした版**(native text の増加を避ける代わりに速さを削る): 未測定。
- **HLE 入り(決定 0162)・`-DFLE_LOWREGS`・`--swap-entries`・`--outline-shared` との組み合わせの全ビルド**: 未実施。ヘッダが `-DFLE_LOWREGS`・`-Wall -Wextra -Werror` で 1 ファイルとして通ることだけ確認した。
- **実スレッドが同時に走る場合の安全性**: 設計の対象外(§4)。
- **呼び出し点の数え方**: `jmp` の末尾呼びは `[R[4]]` が呼び元の戻り番地で、呼び出し点が近似(`tail-like` の欄は常に 0 で、判別していない)。単段 inline cache の命中率は近似。
- **SIGPROF の割合**は全スレッド(native は 1 OS スレッド)。主スレッドだけの割合は集計していない。新ではインライン化により `fle_dispatch` の割合が呼び元に移るので、新の `fle_dispatch` 0% は新経路の費用ゼロを意味しない。CPU 秒の差が実質の効果。
- **120 fps(8.33 ms/フレーム)との比較**: wasm -Oz の主ウィンドウの定常で、エンジンの計算だけの CPU が 1 フレームの仮想時間あたり約 2.9 ms(新)。描画・合成・提示・操作の遅延は含まない(未測定)。
- **perf が使えない**(`perf_event_paranoid=4`)。割合は自前の SIGPROF サンプラ(`disp_prof.c`、500 Hz)。
- **コンパイラの決定論**: 第1版のヘッダでコンパイルした native 92 単位と最終版(`fle_dispatch` の末尾を旧新で共有する形に直した版)でコンパイルした 92 単位を比べると、141 個の .o(単位 92 + runtime 60 のうち)がバイト一致、11 個(fle-03・06・08・11・13・16・18、m1-14・m1-35、m2-01、m3-01)は命令の並びだけが違う(text の大きさは同じ)。最終版のビルドで 16 スナップ・A/B をやり直した。


## 11. 作業者の自己申告

- **第1報が約 7 分遅れた**(期限 07:52、送信 約 08:00)。ベースの native 全 92 単位のコンパイルが load 14〜20 で約 38 分かかった。
- 存在しないツール名の呼び出しが 3 回(`bash_placeholder` 系。エラーのみで副作用なし)。拒否された `sleep` が 1 回。
- **待ち方**: コンパイル待ちに `tail --pid=<xargs の PID> -f /dev/null` で最大 25 分ブロックした(08:06〜08:31)。通知を待てばよかったが、その間に他の作業を進められなかった。while ループ・sleep・pkill -f・nohup は使っていない(`pkill -P <PID>` を 1 回: 自分が起動した wasm -O2 ビルドの子プロセスを止めるため)。
- **同時プロセスが 4 本を超えた時間がある**: コンパイル 3〜4 本に、単発の clang/emcc(designs.c、差分試験のビルド、SIGPROF 走行 25 秒を数回、リプレイ微ベンチ)が重なって 5 本になった。後半はテスト 2 本・wasm 2 本・a4 の JOBS=2 で 6 本前後になった時間がある。
- **途中でやり直した**: (1) 07:59 に、新ビルドを作り始めて 5 分で kill し、ハッシュ関数を `va>>2` から `va>>4` に変えて全部作り直した(obj-new を消した)。(2) 08:48 に、最初の `test.sh full` が `wall_fr20::the_progress_counter_is_in_the_one_inline_function_that_every_indirect_call_ends_in` で**失敗した**(ヘッダに `++fle_progress_count` が 2 つ〔新と BSEARCH〕あり、このテストは 1 つを要求する)。テストは弱めず、`fle_dispatch` を「ランドing → 探索(新: キャッシュ + 二分探索 / BSEARCH: 二分探索)→ 進捗 → hook → 呼び出し」の 1 本にして末尾を共有する形に直した。その後に native 92 単位・wasm -Oz 92 単位をもう一度全部作り、16 スナップ・A/B・テストをやり直した。最初の失敗したログは `logs/fulltest-run1-before-restructure.txt`。
- **報告の訂正**: 第1報の「58,608 の呼び出し点に二分探索が展開される」は誤り(旧は翻訳単位ごとに 1 本の static 関数)。第2報の変異検査「15 件のうち 13 件」は誤り(16 件のうち 14 件検出、2 件等価)。第2報で M07〜M09 を検出としたのはヘッダの旧形の sed で、最終版のヘッダでは sed が当たらず(M09 は旧側の呼び出しに当たっていた)、最終版の sed に直して 16 件を全部やり直した。
- `test-env.sh --link` で、作業用の木 `lift/out-disp1/work/` にだけ入力ファイルへのシンボリックリンクを作った(リポジトリには作っていない)。`disp1.diff` は変更したファイルだけから作ったので、リンクは入っていない。
- 壁時計は、load が 8 以下のときだけ参考にした。表の速さは CPU 秒。
- wasm -O2 のビルド(基準側)を 26/92 単位まで進めて止めた(時間のため)。

