# 0315d(仮): RT-B の木(tree-rtb)の native テスト `harness/test.sh full` の結果と、落ちた物の分類(mt3。PM の instruction 10:2x〜10:36、期限 11:15)

状態: **仮**。区分: **実測**=この作業のコマンドの出力を見た事実、**読み**、**未確認**。**実エンジン・Chrome・Wine の実行は 0**(native のテストだけ)。

## 0. 実際に実行した物

`harness/test.sh full --jobs 1`(nice 19、`CARGO_BUILD_JOBS=1`、`FLE_TEST_NOCACHE=1`)を、RT-B の木 `~/work/mt3w-s1/tree-rtb`(`tools/rtb-tree.sh tree` で作る)で、構成を変えながら 5 回。

| run | 時刻 | 木の構成の違い | 結果(実測) |
|---|---|---|---|
| 1 | 09:50〜10:0x | 最初の構成。72/219 で止めた(最終の構成を決める前だった) | 4〜8 件が落ちるのを見た時点 |
| 2 | 10:14〜10:28 | + alpha1 の callback-stack-limit-rtb(10:43 版でなく 10:0x の初版)+ diag-g9-fl-paint / -ulw + 領域・語の修正 + ci1 の 0323c は **無し** | **passed=2183 failed=253**、落ちた executable 41。SEGV が 16 executable 以上 |
| 3 | 10:30〜10:46 | callback の差分を外した(NOCB)+ ci1 0323c + 私のテストの更新 | **passed=2420 failed=16**(13 executable。SEGV 0) |
| 4 | 10:57〜11:12 | alpha1 の 10:5x の `-rtb-full` + tests、診断を FLE_TEST_GETENV の形に | **passed=2428 failed=9**(7 executable) |
| 5 | 11:15:39〜11:33:05 | 下記のテスト側の更新(ci1 の 0323d、alpha1 の run-values・scroll-model のテスト、私の `g9_hook`・fle_win.c 診断の `long` の書き換え)をすべて入れた最終の構成 | **results=224 passed=2437 failed=0**(rc=0。executables run=220) |

## 1. run 2 の 253 件の原因(初版の callback-stack-limit-rtb。alpha1 の差分の欠陥 = 実装の誤り)

1. **SIGSEGV(fle_callback.c:101)**: `fle_teb_cur ? fle_teb_cur : fle_teb` — 非 TLS_MORE の `fle_teb_cur` は weak な**変数**で、リンクされていないテストでは値を読もうとして address 0 を参照した。再現(実測): tests/callback.rs の DRIVER を取り出して clang -O0 -g で作る → `./drv nest` が SIGSEGV、gdb の bt が 101 行目。101 行目だけ `&fle_teb_cur != NULL && fle_teb_cur` に変えた一時のコピーでは rc=0 で正しい出力。
2. **guest stack の予約 64 KiB = テストの guest stack(64 KiB)**: 『guest stack is used up (rsp …, StackLimit …, reserve 65536)』で最初の callback で止まる(16 executable)。
alpha1 の 10:43 版は、(1) アドレスの検査、(2) 予約 = min(64 KiB, StackBase − StackLimit の 1/4)、(3) 決定 0064 のテスト 3 件の更新と新しいテスト 1 件を入れた(HEAD の複製で全 215 executable が通る、と alpha1 が実測)。**分類: (a) 我々が足した余計な処理の欠陥**(外部 API の契約の不足ではない)。

## 2. run 3(NOCB)の 16 件と、直した者・差分(分類つき)

| executable | 落ちた物 | 原因の差分 | 区分 | 直し |
|---|---|---|---|---|
| alpha5_erase_dc | 『0 を返すと BeginPaint がもう一度送る』 | ci1 の 0323(delayed erase、Wine 10.0) | 期待の更新漏れ | ci1 `0323c-test-expectations.diff` |
| win2_act | pin に `P3 WM_SETCURSOR hit=-2` の 3 行 | ci1 の 0323b | pin の更新漏れ | 同上 |
| wall_fr14 | disabled な窓に `SETCURSOR hit -2` の 3 行 | ci1 の 0323b | 期待の更新漏れ | ci1 `0323d-test-guards.diff` |
| wall_fr9 | `fle_win.c must not draw: FillRect`(+ GetDC/ReleaseDC の関数別の数) | ci1 の 0323(def_erase_background / erase_now_one) | 守りの更新漏れ | 同上 |
| window | `uintptr_t`、`fle_call_guest(` の数 8→9 | ci1 の 0323 / 0323b | 守りの更新漏れ | 同上 |
| window | `fle_win.c uses long`(fle_win.c:622 の診断) | alpha1 の diag-g9-fl(TEST-ONLY) | 診断の書き方 | mt3 `tools/rtb-g9hook-fmt.py`(PRIx64。出力は同じ) |
| callback | `fle_callback.c uses long`(g9_hook の診断) | mt3 の diag-g9-hook(TEST-ONLY) | 診断の書き方 | 同上 |
| callback | `must not use emscripten` | 第 1 形の FLE_DIAG_CB の診断 | 診断 | alpha1 の `-rtb-full`(第 2 形、emscripten なし) |
| time | `fle_time.c must not use getenv` | alpha1 の 0319-time-env-run-values(run 値 2 つを getenv で読む=意図) | 守りの更新漏れ | alpha1 `0319-run-values-and-scroll-model-tests.diff` |
| vfs | `fle_env.c mentions getenv`(FLE_ENV_DEFAULTS) | 同上 | 同上 | 同上 |
| vfs / wall_fr6 / wall_fr7 / wall_fr9 / window | `getenv`(FLE_DIAG_WIN / VFS_MISS の読み) | cand1 の木で診断を getenv に変えてある(wasm で生かすため) | 診断 | テストを走らせる木では FLE_TEST_GETENV の形に戻し(`rtb-tree.sh tree`)、build の直前に `rtb-tree.sh diag` で getenv に |
| wall_ia2 | 3 件、`undefined reference to fle_win_scroll` | alpha1 の scroll-model が旧い経路を置き換えた | テストの stub 不足 | alpha1 の同上(SCROLL_PRE の stub) |
| sweep1 | link: `fle_win_input_call_hooks`、`fle_win_input_hit_list` | ci1 の 0323b / 0324(fle_input.c の新しい参照) | 部分リンクのテスト | mt3 `0315-rtb-test-expectations.diff`(他の fle_win_input_* stub の前例どおり link stub) |
| dwm_timing | ship6 の行数 240 ≠ 210 | 0319 の 4 つの差分が ship6 の末尾に行を足した | pin の更新漏れ | 同上(dwmti は ship5 の次の最初の行、以降は後続の決定の行を許す) |
| wall_dec1 | `RM_MAX must stay 8` | mt3 の 0313(G9-c の段で RM_MAX 16) | 期待の更新漏れ(**私の更新漏れ**) | 同上(16u) |
| wall_fr9t | stderr の行数 8002 ≠ 4003 | alpha1 の diag-g9-fl が `fle-thread: wait` の行を trace に足した(cand1 の木から。HEAD 4003、cand1・rt1・rtb 8002 を実測) | 診断がテストの数に入る | 同上(wait の行を数えない) |

## 3. 前に見つけて直した物

- wall_fr16 の 14 件: alpha1 の 0319-scrolldc-model が fle_gdi.c で `fle_visrgn_subtract` を weak 宣言なしに呼んだ(他の visrgn 参照は decision 0142 で weak)。1 行の weak 宣言 `impl/0315-gdi-visrgn-subtract-weak.diff`。engine の link には fle_visrgn.c が入るので engine は変わらない(読み)。
- imagelist の領域: 宣言上 cursor / LI の領域と重なっていた → alpha1 が gdc と gdi の隙間 0x7FB90000(64 KiB)に移した(`0319-imagelist-drag.diff` 差し替え + `0319-imagelist-drag-tests.diff`)。
- num-cpu の文字列の "environment" が sysinfo.rs の `environ` の部分一致に当たった → "machine"(alpha1)。
## 4. 最終の run(run 5。実測)

`harness/test.sh full --jobs 1`(nice 19)を `tree-rtb`(= build の木 `tree-rtb-build` から『診断を getenv にする』1 工程を除いた物。`diff -r -x 'out-*' -x target` で差 0 を確認してから diag を当てた)で 11:15:39〜11:33:05、**rc=0、results=224 passed=2437 failed=0 ignored=1 SKIPPED-lines=15**(SKIPPED は入力の DLL・seed が無いテストの自己 skip。run 3・4 と同じ数)。
= RT-B の build の入力は『失敗 0』の木。build は `tree-rtb-build`(= 上 + `rtb-tree.sh diag`)で 11:18:42 に開始。

## 5. 教訓・自己申告

- 「runtime の差分を build の入力にする条件 = 該当の native のテストが通る」を、私は alpha1 の初版の callback の差分(dry-run と emcc の構文確認だけで『検証済み』)で守っていなかった。PM と alpha1 も同じ穴だった。以後、runtime の差分は該当のテストの実行を先に確かめる。
- 診断の第 1 形の再構成が、alpha1 の差分の更新で壊れる作りだった(原本を保存していなかった)。alpha1 の `-rtb-full`(自己完結)で解消。
- run 1・2 を最終構成を決める前に始めて無駄にした(約 25 分)。


## PM の追記(2026-11-07 11:28、docs のみ。実測は PM が WAV・json・ログを直接読んだ物だけ)

- **PM の検証の訂正**: 私は callback-stack-limit 系の差分を `patch --dry-run` と `emcc -fsyntax-only` だけで「検証済み」と扱っていた。runtime の差分は、該当の native テストが通ることを build の入力にする条件とする。alpha1 の修正版は alpha1 が HEAD のコピーで full 215 executable(passed 2417、failed 1 = dwm_timing の行数 pin、mt3 が更新済み)を実行した。PM は修正版の dry-run(HEAD の git archive と tree-rtb)だけを確認した。
- ci1 の `0331/0323d-test-guards.diff` は ci1 が実際の cargo で 5 targets(alpha5_erase_dc 1、wall_fr14 26、wall_fr9 30、win2_act 2、window 37 = 96 passed、0 failed)を回した。PM は fresh git archive HEAD への 0323 → 0323b → 0323c → 0323d の順の適用(FAILED 0)と tree-rtb への dry-run(FAILED 0)を確認した。
- mt3 は 7 target の cargo(callback 19/0、time 24/0、vfs 23/0、wall_ia2 21/0、wall_fr9 30/0、wall_fr14 26/0、window 37/0)で全て通ったと報告。RT-B の build は 2026-11-07 11:18:42 に始まった(callback の修正入り、nice 19、CI_JOBS=1、guard は MemAvailable < 7000 MB で kill)。最後の full は build と並走して 11:15:39 から実行中で、結果は mt3 が `rtb-native2.txt` に書く。
- **P3 の最初の測定(n=1、ci1。PM が audio-p3.json を直接読んだ)**: 参照プロジェクトの song 5 分、実 Chrome 156、headless。協調版 E13: underrunQuanta 1555(最初の約 30 s の 1521 quanta を含む)、engineGapFrames 266,130、lagCuts 4,137。実スレッド版 RT-A(N=8、page-rt1): underrunQuanta 0、engineGapFrames 46,381、lagCuts 0。RT-A の run には mt3 の build とテストが並走していた(終了時 load 11.6)。**再現は未確認、音楽の位置と T は未抽出。P3 は未達のまま**。first idle は E13 の median 5,312 ms に対し RT-A の median 7,813 ms(交互 n=5、全ペアで RT-A が遅い、差の median +2,515 ms)で、P1 の 10 s 以内は満たすが余裕は小さい。seed2b(+1.58 MB br)に first idle の差は見えない(RT-A median 8,116 ms 対 seed2b 7,809 ms、差の median −364 ms)。依頼者の機械は 0 件。
