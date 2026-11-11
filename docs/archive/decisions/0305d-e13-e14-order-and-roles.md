# 0305d(仮): E13 の clean な再ビルドとテスト、E13 の上に実スレッドの足場 + 16 件の修正 + D3 の除去を載せた木(E14 候補)の dry-run、受け入れ条件を早く測れる着手順、人日の合計と担当案(mt3。0305 の最終)

状態: **仮**(0305 の最終)。作業者 mt3、2026-11-06 JST。依頼: PM(18:55・19:10・19:18・19:25 の info)。区分: **実測**=この作業のコマンドの出力を見た事実、**読み**=コード・文書の解釈と見積り、**未確認**。**Chrome は起動していない。実エンジン(wasm)の実行は 0 件。** 重い job は nice 19・2 jobs で、gpu1 の sdriveg2 が居ない時に始めた(19:25 に ps で確認。gpu1 の『Chrome 終了』の info は私が始めた時点では来ていなかった。PM の『ps で確認して始めてよい』に従った)。

## 1. E13(main 6f690d38 + 0302 の Dwm + 0284b-gp-extension)の clean な再ビルドとテスト

**経緯(自己申告)**: 1 回目と 2 回目の run は**無効**だった。cargo の target dir を実スレッドの足場の木と共有し(1 回目)、2 回目は target dir を複製したが、**複製した target dir の fe-lift の統合テストが `CARGO_BIN_EXE_fe-lift` を『複製元の `ct/debug/fe-lift`(別の木が最後に作った fe-lift)』に焼き込んだまま**で、テストが足場の `imports_table.rs`(`CreateSemaphoreW`/`ReleaseSemaphore` の行つき)の fe-lift を使い、46 失敗と a4 の link 失敗(`undefined reference to fle_imp_CreateSemaphoreW`)になった。PM から教わった既定の C の照合(215,740,699 B・e798422c…)が私の再現(215,741,085 B・6c9d92b8…)と合わなかったのも、**同じ汚染された fe-lift** が原因だったと読める(コマンドは同じ)。3 回目は専用の target dir で `cargo clean -p fe-pe -p fe-decode -p fe-lift -p fe-wasmgen` してから作り直した。

(結果は下の表。run 3、`mt3w-e13/e13.txt`)

| 項目 | 結果 |
|---|---|
| 積み重ね | main 6f690d38 に 0302(8 ファイル)と gp-extension(`fle_locale.c` 1 ファイル)を適用: rc=0、offset・fuzz・FAILED なし(実測)。hi-check form(0303)も dry-run で clean |
| `cargo test --workspace --no-run` | **rc=0**(19:25:30 → 19:27:39、2 分 9 秒。215 test executables)。実測 |
| 既定の FLEngine の C(PM のコマンド: `fe-lift lift binaries/win64/FLEngine_x64.dll OUT.c --pdata`、fe-lift は E13 の木のビルド〔debug〕) | **215,740,699 B・sha256 `e798422c203ee0cdd5f3f8b1950df15c8443b4f9c39dd03bc89b6e1b9fc35c26`** = PM の記録(E11/E12)と**一致**(E13 の既定の C は不変。実測)。1・2 回目の 215,741,085 B・`6c9d92b8…` は汚染された fe-lift の出力 |
| `test.sh full`(`FLE_TEST_NOCACHE=1 --jobs 2`、nice 19) | **219 results、2,417 passed、0 failed**、1 ignored(wall 476 s、build 117 s)。E12c + 0302 の私の検証(0302 §6)と同じ数。**ただし SKIPPED 15 行**(0302 の run は 1 行): E13 の木は main の追跡ファイルだけで作ったため、**追跡されていない入力 DLL 4 つ**(`lift/out-ext/dsp_ippv2_x64.dll`、`out-qfc/quickfontcache_x64.dll`、`out-blend2d/Blend2D.dll`、`out-jtb2d90/three/dll/dsp_ippv2_x64.2000000.dll`)が無く、それを使う 14 行のテストが skip(= 早期 return で pass)した。**4 つを main のワークツリーからコピーし、該当の 10 binaries を再実行: 全部通った**(fe-pe `rebase` 15、fe-lift `wall_fr7` 18・`k32_dsp` 14〔60 s〕・`jt_cpu` 5・`gs_stacklimit` 6・`deferred` 15・`call_iat_dll` 1〔59 s〕、fe-decode `msvc_switch` 72・`jt_cpu` 9・`inline_table` 6。`mt3w-e13/targeted-rerun.txt`)。skip の行が他にも別の入力で隠れている可能性は、log の『e.g.』が 1 binary 1 つだけなので**完全には確かめていない** |
| a4(5 モジュール、nice 19、JOBS=2、fe-lift は E13 の木) | **units 942、runnable 895/895 完走、実行された関数 distinct 5,310、関数入口 1,009,737、終了コード 0**(gen 27 s、build 228 s)= 基準と同一。生成 C `gen/flengine.c` の sha256 `2715c3f3401bc17727b3ab5927ca08beb348b483b2f81334c3124b9b8c783933` = **E12 の a4(0284b-on-0284c の私の表)の値と一致**(1 回目の a4 の `1d5d368d…` は汚染された fe-lift の出力)。実測 |

**wasm の大きさの見積り(ビルド無し、3 構成)**:

| 構成 | wasm の大きさ(見積り) | 根拠 |
|---|---|---|
| E12(デプロイ済み) | 143,085,947 B | PM の記録(a977401d、bundle `fl-engine-ci-7a9b0800`) |
| **E13 = Dwm + gp-extension** | 約 **+3 KB**(143,088〜143,090 KB 台) | `emcc -c -O2 -msimd128 -matomics -mbulk-memory` の **オブジェクトの code 部**(実測): `fle_locale.c` 17,620 → 17,738 B(**+118 B**、gp-extension)、新ファイル `fle_dwm.c` code 2,447 B + data 370 B。これにリンク時の dead code 除去・`wasm-opt` の前の数で、ほかに表の +1 行・FLEngine の DWM thunk が呼びになる分(数十〜数百 B、未測定)。= **オブジェクトの合計 +約 2.9 KB + 数百 B** |
| **E13 + hi-check form** | 約 **+241 KB**(+0.17%) | 0303 の見積り: 8 関数でなく全箇所に入れると 79,395 か所 × 3 B = 238,185 B(**読み**。ビルドして測っていない)。+ 上の E13 の分。0303 の実 harness の効き: `f_863f60` の自己 −2.9%(2 run ずつ)、上位 40 関数でも −1.8%、ミキサーに差なし |

PM の判断材料として: hi-check は +241 KB(+0.17%)で UI の `f_863f60` を約 −2〜3%(約 1.5〜2 ms/仮想 s)。E13 に入れても入れなくても『基準を動かす 1 回のコールド build』に同乗できる(0303)。

## 2. E14 候補(E13 の上に 実スレッドの足場 + 16 件の修正 + D3a の除去)の dry-run と cargo test --no-run

**積み方**(使い捨ての木 `~/work/mt3w-e14/tree` = main の追跡ファイル。実測、`mt3w-e14/apply-summary.txt`):

| 順 | 差分 | 結果 |
|---|---|---|
| 1 | `0302-dwm-timing-info.diff` | rc=0、8 ファイル、offset 0、fuzz 0、FAILED 0 |
| 2 | `0284b-gp-extension-on-0284b-on-0284c.diff` | rc=0、1 ファイル、offset 0 |
| 3 | `rt-on-e11.diff`(60 ファイル) | rc=0、offset 0、fuzz 0、FAILED 0。`build-engine-bundle.sh` は 0302 と同じファイルだが**衝突なし** |
| 4 | `0305-fix-the-16-failures-on-rt-on-e11.diff`(13 ファイル) | rc=0、offset 1(`fle_thread.c`、E11 と main の差)、FAILED 0 |
| 5 | `0305-d3-remove-instrumentation.diff`(7 ファイル) | rc=0、offset 0、FAILED 0 |

= **5 本が main の上でこの順に clean に積める**。**`cargo test --workspace --no-run`(E14 候補、専用の target dir で fe-* を clean してから): rc=0**(19:40:09 → 19:42:20、2 分 11 秒、219 test executables = E13 の 215 + 足場の新しい 4〔gil_lockfree、semaphore、x87_threads、mx_hybrid_tls〕)。warning は既存の物(unused import ほか)。実測。**E14 候補の `test.sh full` と a4 は未実施**(重い。次の段)。

## 3. 『実装の開始順』を、受け入れ条件を早く測れる順に再配置

**PM の受け入れ条件(暫定)**: T の中央値が協調 + 2 ms 以内(今: File クリック +9.3 ms、Esc +8.1 ms = B 起床 +5・C 処理 +5)、UI Worker ≤ 0.6 コア(今: 0.54〜0.64)、重い区間の CPU/Unlock ≤ 85 ms(今: 10 s の窓で 84 ms)、5 分の書き込み ≥ 0.9(今: 0.938〜0.988)。**未達は T だけ**(UI Worker は際)。= **最初に測れるべきなのは T と UI Worker、次に G9 後の重い区間**。

**原則**: (1) E13 を先に 1 回デプロイし(基準の動きを 1 つずつ)、実スレッドは E13 の上の別の系列(E14 以降)。(2) 中間 bundle は gpu1 が 1 本ずつ測れる形で出す。(3) **リスクの大きい未知(I15c の新しい壁、I15e の GIL の拡張性、T の B/C)を、旗を除く掃除(I3〜I5)より先に測る**。旗の除去は『測定が通ってから』でも遅くない(掃除は機械的で、測定の結果を変えない — D3a は前処理の一致を実測済み)。

| 段 | 内容 | 人日 | 出す中間 bundle(gpu1 が測る物) | 測る受け入れ条件 | 備考 |
|---|---|---|---|---|---|
| **S0** | I1 + I2: E13 の上に 足場 + 16 件の修正(= §2 の木) | 0.75 | (bundle 無し。cargo のテストと a4 だけ) | — | §2 で dry-run は実測済み。cargo のテストは上の表 |
| **S1** | **I6 段階 C(最短の形)**: 音声リングを wasm の共有メモリへ、`wr`・`rdHi` を共有、`fle_tp_*` を C に。page の `audio-ring`。足場の旗・環境変数は**まだ残す**(`FLE_WORKERS` ほか) | 1.5 | **B1** = E13 + 足場 + 16 件 + 段階 C | **PCM の零(0305c の直しを製品の形で)・T・UI Worker・重い区間・5 分の書き込み**を、gpu1 の glue 補修 **無し**で測る | これが無いと製品の形の測定ができない(gpu1 の glue は測定専用)。env の `FLE_WORKERS`/`FLE_PROCESSORS` が残るので、**B1 のままで n = 2/4/8 の先行測定ができる**(I15c・I15e のリスクを早く見る) |
| **S2** | **T の B/C の梃子**(gpu1 の B/C 設計 → mt3 が実装): B = page → エンジンの起床(+5 ms): 入力 ring を書いた直後に page が wake word を `Atomics.notify`(今は IdleDriver の経路も待つ)、C = エンジンの処理(+5 ms): UI の export/import ごとの GIL の取り直し・FL の CS の待ち(0289 §5 では CS の待ちはほぼ 0)の内訳を計測 build(D3a の `patch -R`)で出す | 2〜4(分析 1〜2 + 実装 1〜2。読み。結果次第) | **B2** = B1 + T の梃子 | **T**(+2 ms 以内) | 梃子は『通信/変換の削減』に当たる形に限る(新しい特別扱いを足さない)。B/C の内訳が出るまで工数は不確実 |
| **S3** | **I15a/b/c/d**: n を host import から、Worker 数を n から。FL が n ≥ 2〜32 で最後まで走るか(新しい壁) | 2〜3 | **B3** = B2 + N 本 | **重い区間の CPU/Unlock(G9 の前後)**・n = 2/4/8 | リスク項目。環境変数 `FLE_WORKERS`/`FLE_PROCESSORS` を除く(I3 の一部) |
| **S4** | **I3 + I4 + I5a**: 旗・環境変数・計測用 build・診断(D3a + D3b)を除き、`GIL=1`/`FLE_TLS_MORE`/`--hybrid-mx-tls` を常時に。= **『唯一の構成』** | 3.5〜4(I3 0.5、I4 1.5、I5a 2) | **B4** = E14 候補(旗なし) | **全部**(B3 と同じ値になるはず。基準の wasm の sha・大きさが動く = 冷間 build) | **基準が動く**のはここ。B1〜B3 は旗が残る測定用 |
| **S5** | I7 + I8 + I9 + I12 + I13: wasm export の GIL 包み・`check_i1.py` の CI、ページ、Firefox、test tree の差し替えファイルと 84/55/87 行・パリティ・e2e、CI | 4.5〜5(I7 0.5、I8 1、I9 0.1、I12 1.5、I13 0.5、ほか 1〜1.5) | (B4 の仕上げ。CI で本番の bundle) | P1(起動 10 秒)・Firefox | |
| **S6** | I11(Export の WAV の sha256)・I14(製品形での再測定、G9 後を含む) | 2 | B4 | 全部 + G9 後 | |

**中間 bundle の置き場と ci 段のコスト**:
- **置き場**: gpu1 の既存の機構を使う。`lift/out-gpu2/tools/mk-rt.sh NAME TREE [STAGES]` → `lift/out-gpu2/ci<NAME>/…/bundle/fl-engine-ci-<sha>/`(host・b2d・`engine/{cargo-target,emcache,in}` は opt1 の冷間 build〔`BASE=../out-opt2/x16`〕から**ハードリンク**)。私の木(`mt3w-e14/tree` を作り直した S1 以降の木)を `TREE` に渡す。main には**入れない**。assets は gpu1 の `assets-rt11` と同じ手(manifest + glue)。**S1〜S3 は旗が残るので、gpu1 の `patchglue-rt.py` 無しで manifest の `engine.env` だけで条件を変えられる**(B1 は glue 補修が不要になるので `patchglue-rt.py` を通さない)。
- **ci 段のコスト(実測 + 読み)**: 冷間の全段は約 20 分(`objects` 406 s + `link` 777 s、gpu1 の 0289 §2、load 10〜14、実測)。**S1(段階 C)は runtime の `fle_audio_transport.c` と page だけの変更**なので、`ci/objkeys.sh` の group `rt`(runtime の fle_*.c)の object だけ作り直し、他の group(FLEngine の 53 ユニット + 6 モジュール)は鍵が同じで再利用される(読み。**工数は group `rt` の object 数分 + `link` 約 13 分 + `seed`/`bundle`**、合計 約 20 分以下)。**S4(旗を常時にする)は全 object の build 入力〔`PLAN_TOOLS`/`PLAN_H_WASM`〕が変わり全 group の鍵が 1 回動く**ので冷間 約 20 分 + 本番 CI の最初の 1 回。ただし gpu1 の local の rig では旗の値が同じ(すでに `-DFLE_GIL -DFLE_TLS_MORE`)ので、object をハードリンクで再利用できる(読み)。
- **E13 を先にデプロイ**するので、E14 系列の CI の cache は E13 の後に作り直される(`H_SEED` が seeds 丸ごとのキーなので 0302 の `ship6-imports.txt` で動く〔0302 §5〕)。

## 4. 人日の合計(D1 = a')と担当案

**合計(読み)**: 第 2 報の (a') 12.5 + I15 約 8 + S2 の T の梃子 2〜4(第 1〜2 報の表に無かった物)= **約 22.5〜24.5 人日**。クリティカルパスは **S1(1.5)→ S2(2〜4)→ S3(2〜3)→ S4(3.5〜4)→ S5(4.5〜5)→ S6(2)= 約 15.5〜19.5 人日**(S0 の 0.75 を含めて 16〜20)。**S1 の後に gpu1 が B1 で最初の受け入れ条件(PCM の零・T・UI Worker・重い区間)を製品の形で測れる**ので、**判断(続けるか)は S1 の終わり(約 2.25 人日後)で一度確かめられる**。

| 担当 | 持つ物 | 人日(読み) |
|---|---|---|
| **mt3(実装)** | S0〜S5 のコード: I1・I2、I6、T の梃子の実装、I15a/b/d、I3〜I5、I7〜I9、I12、I13 | 約 15〜18 |
| **gpu1(測定と B/C 設計)** | 各中間 bundle の測定(B1〜B4: T・UI Worker・重い区間・5 分・PCM)、n = 2/4/8 の先行測定、T の B/C の内訳設計(計測 build の `patch -R`)、I14 | 約 4〜5(Chrome 約 100〜150 分/bundle の窓。ci1 の counted run と調整) |
| **alpha1(G9-a)** | FL 内蔵エフェクト 8 種の DLL を出荷 volume に載せて開けるようにする(G9)。G9 後の仕事量で『重い区間』『N 本』の要否が決まる | alpha1 の見積り(私は持たない) |
| **opt1(Wine 参照)** | Wine 上の FL が n(論理 CPU 数)に応じてどのスレッドを何本作るか(作業スレッドプール 32 本中 15 本が稼働)の参照。I15c(FL が n ≥ 2〜32 で最後まで走るか、どの API を呼ぶか)の期待値を出す | 約 0.5〜1 |
| **ci1(本番確認)** | E13 のデプロイ後の確認(first idle のゲート、Export の一致)、E14 系列の CI の cold build の費用の確認、S5 の Firefox 等の確認の一部 | 約 1〜2 |

**判断が要る点(PM)**: (1) D1 = a' の確定(go のとき)。(2) S2 の T の梃子は、B/C の内訳が出るまで工数が不確実。S1 の直後に gpu1 が B1 で T の内訳(B・C)を測るのが最短。(3) hi-check form を E13 に入れるか(+241 KB・効き −1.8〜−2.9%)。(4) 協調の決定的テストの test tree(`fle_sched_mode.h` 1 ファイルの差し替え)の方式でよいか(S5 の I12 の前提)。

## 5. 自己申告(0305 の最終の区間)

- E13 の 1 回目・2 回目の run が無効だった(上)。target dir の共有/複製で fe-lift の binary が別の木の物になる罠に、46 失敗を見るまで気づかなかった(`CARGO_BIN_EXE_*` がコンパイル時に焼き込まれることを失念していた)。以後、木ごとに専用の target dir を作り、複製したら `cargo clean -p` する。
- gpu1 から『sdriveg2 が居る間は重いビルドを始めない』と言われたが、その前に始めていた test.sh full を止めずに親の bash だけを止めた(子の test.sh は完走した。結果は無効だったので再実行)。PM の許可どおり、chain の終了後に ps で確認して再開した。
- `sleep`・末尾 `&` を 1 回出して拒否された(第 2 報の区間)。バックグラウンドは `background: true` だけを使っている。
- 『T の梃子 S2 の 2〜4 人日』は、gpu1 が B/C の内訳を出す前の読みで、確度が低い。


## PM の決定と取り込み(2026-11-06 19:45、docs のみ)

取り込み: mt3 の最終(19:43)の一部として `docs/investigations/mt3w/0305/` の表・道具とともに置いた。判断は決定 0305 の PM の節にまとめた(E13 = Dwm + gp-extension、着手順 S0〜S6、D1 = (a')・D2・D3 の確定の範囲、担当)。0305c(実スレッドの PCM の零は Worker ごとの JS 状態の `A.wr` が古いための測定用 glue の欠陥)は gpu1 の決定 0306 の PM の節と合わせて読む。**依頼者の機械・wasm の実行は 0 件**。
