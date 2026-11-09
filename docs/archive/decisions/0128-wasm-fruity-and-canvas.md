# 0128: FRUITY 有効の 6 モジュール構成を wasm で動かす(node と headless Chrome)。窓の描画面(lid 13)を canvas に出す(G4 の最初の達成の判定)。idle で JS に制御を返す最小実装(G3)。すべて仮の決定

状態: **仮の決定(すべての選択)**。依頼は PM(決定 0128 の番号は PM の割り当て。pin1 が 0127 を使う)。測定・実装・文書は作業者が行った。作業は HEAD 9574cb5 の `git archive`(`lift/`、`docs/`)で始め、clean な写しを `clean/` に置いた。差分は `wasmfruity1.diff`(`diff -ruN clean src`、14 ファイル、約 100 KB、clean な写しに `patch -p1` で当たる: dry-run 終了 0、当てた木 = 作業木)。**実機 Windows との照合は 0 件。Firefox・Safari・headed の Chrome・GPU・HiDPI は未測定。音声・文字描画なし。**

区分の凡例: **実測**＝このセッションのコマンドの出力を見た事実。**読み**＝コードや出力からの解釈(確かめていない)。**未確認**。**出典なしの選択**＝ページにも実測にも根拠が無く、作業者が選んだ値(`S-…`、撤回条件つき)。

**肯定的に見せる点は限られる。** (1) wasm の入口数は native と**一致しない**(2 少ない。原因は特定済みだが、これは「ゲストが未初期化のスタックを読む」という発見であり、wasm の側に直す点があるのではない)。(2) G3 は**半分**: idle で JS に制御が返り、実入力がキューに入って窓手続きが走るところまで実測したが、窓手続きが runtime の壁に当たり「1 周」には届いていない。(3) 最初の Chrome のフル実行は**結果なし**だった(自分の Worker の不具合)。(4) 描画面の画素が native と一致したのは**同じ生成 C から作った同じ runtime の画素モデル**の結果であり、「Windows の GDI と同じ」ではない(決定 0126 の `S-` 選択がそのまま入っている)。

## 先に結論

| 項目 | 内容 |
|---|---|
| wasm(node)の FRUITY 有効は native と一致したか | **停止・関数数・スレッド・面の画素は一致、入口数は 2 ずれる。** 停止 = `fle: MessageLoop: idle: the first thread waits for input with an empty queue and no other thread can run`(exit 134 = abort 経路)、新規の関数 **5,515**、スレッド **10**(状態・待ちの表は native と同一、17 行)。入口: CreateFruityInstance の開始から native **134,880,081** / wasm **134,880,079**。G2(主窓が可視): native 34,866,773 / wasm 34,866,771。最初の非背景画素: native 134,983,755 / wasm 134,983,753。停止時のプログラム開始からの入口 135,889,417(wasm。native は +2 = 135,889,419)。node の完走 8 回(node1、nodeF1、nodeF2、nodeFinal、JSPI スタックを 450/475/500/700 kB に変えた 4 回)すべてで同じ数(134,880,079 入口、5,515 関数)。Chrome の完走 4 回も面の SHA は同じ。wasm は 1 回あたり 5:22〜5:31(native は a4-multi 全体 4:06、呼び出し 184.5 秒) |
| lid 13 の面の SHA-256 は PNG と一致したか | **一致(停止時と最初の画素の両方)。** 最初の画素: `7ecc7f73f0058e19bde94312c46f91a20c508f2083e485149cdde1e78e557ebf`(非 0 画素 201,380)、停止時: `a8963a48155ddc6b621bc00a7a9b53ef1d231a96693af1fe0d92de119c57f8b4`(非 0 画素 1,917,820、1930×1092)。**同じ値を C(`fruity_surface.c` の SHA-256)と node(`crypto`)の独立 2 計算で得た。** 期待値は決定 0126 の 2 つの PNG を present1 の PNG 復号(A = 255)で RGBA にして再計算(依頼文の値と同じ)。lid 115(119×192、非 0 画素 22,848)は PNG が無く比べていない(node と Chrome で同値 `324914776f11…`) |
| headless Chrome の結果 | **idle 停止まで到達した(3 回の完走 + 最終ビルドの 1 回 = 4/4)。** Chrome 156.0.8078.47(V8 15.6.75.19)、Worker 上、`--headless=new --disable-gpu`、隔離コンテキスト。エンジンの実行(main の開始から exit)316.3〜318.1 秒(runner の wall 318〜320 秒)。メモリ: 線形メモリ 671,088,640 B から **memory.grow 2 回**(+2,667 ページ → 845,873,152 B、+1,552 ページ → **947,585,024 B** = 14,459 ページ。上限 2 GiB(`--max-memory 0x80000000`)の 44%)。renderer のピーク RSS 1,432〜1,739 MB(4 回。ピークの時刻は wasm のコンパイル時)、プロセス木 2,757〜3,072 MB。wasm は 465,880,569 B(loopback のダウンロード 1.36〜1.45 秒、コンパイル 0.25〜0.45 秒)。ページの 50 ms heartbeat の最大間隔 57.9〜72.9 ms(ページは止まらない) |
| JSPI の深さ | 影のスタック(lifted の呼び出し)の最大 **185 段**(a4 経路は wasm の 68 フレーム。単位が違う)。JSPI スタックの下限(node 24、`--wasm-stack-switching-stack-size`、実 FRUITY 経路、各 1 回): **150・200・300・400・425 kB は `RangeError: Maximum call stack size exceeded`(7〜11 秒、CreateFruityInstance の中、G2 の前)、450・475・500・700 kB と既定 984 kB は完走。** Chrome も 400 kB で同じ失敗。a4 経路の 150 kB より大きい(下限は 425 kB を超え 450 kB 以下)。10 本のうちどのスレッドが最も深いかは**未測定** |
| canvas の読み戻し SHA-256 は一致したか(G4 の最初の達成の判定) | **一致。「G4 到達(native と同一画素、headless Chrome)」と書いてよい。** 最初の画素と停止時、方式 A(Worker の OffscreenCanvas に `putImageData`、同じ Worker が `getImageData` で読み戻し)と方式 B(RGBA を ArrayBuffer で page へ転送、page が `putImageData` して `getImageData`)の両方: **PNG の RGBA SHA-256 と各 3/3 一致**、これを Chrome の完走 4 回で(chrome1、chrome-g3、chrome-g3click、chrome-final)すべて。差の画素数 0。**根拠の出どころ**: chrome1・chrome-g3・chrome-final はランナーの `compare`(`expect=` つき)の値。chrome-g3click は `expect=` を渡し忘れたので JSON に `compare` が無く、記録された SHA を事後に同じ期待値と照合した(first・idle・stop の 3 時点で module・canvas A・canvas B がすべて一致。idle の面の SHA は停止時と同じ)。方式 C は作っていない。条件: GPU なし、1 マシン、Linux、DPR 1、`--force-color-profile=srgb` |
| G3 | **半分。** idle でエンジンの JSPI スタックが中断し Worker のイベントループが回る → page の実マウス入力(CDP `Input.dispatchMouseEvent`)が C の `fle_thread_post` でキューに入る(ack r=1)→ 第 1 スレッドが起き窓手続きが走る(43〜51 入口)→ **runtime の壁で停止**(WM_MOUSEMOVE: `comctl32!_TrackMouseEvent`、WM_LBUTTONDOWN: `DefWindowProcW` 0x201)。**WaitMessage → 入力 → 再び WaitMessage の 1 周は未達。** 現在の(idle 停止の)ビルドでは、エンジンの実行中(316 秒)に Worker のイベントループは一度も回らない(page から送った ping 3 つは エンジンの終了後に配信された) |
| 止まった条件 | (a) 目的 1〜3 は完了。目的 4 は (c)/(d) 手前: 1 周には**偽らない実装を超える壁**(runtime に TrackMouseEvent と DefWindowProc の応答を足す = 別レーンの壁)が要る。8 時間の上限には達していない(約 4.5 時間で文書化に入った) |

## 手順と構成(実測)

| 項目 | 内容 |
|---|---|
| 入力 | FLEngine `b2c64da4…`、dsp `497dd4f7…`、Blend2D `66634d24…`、QFC `afc5a0ed…`、freetype `6ea6c106…`、ILTools `20409dcb…`、フォント 2 ファイル、`GM_CCs.txt`(`af33bd90…`)。**すべて残っていて SHA が一致(再抽出は不要)** |
| 構成 | 6 モジュール(dsp は種 `dsp-ippv2-2000000-seeds-jt.txt`、Blend2D は seeds964 + `blend2d-no-jit-sites.txt`、FreeType c2、QFC と ILTools はフック)+ `FLE_SEEDS`(8 thunk)+ レジストリの種 2 つ + 認証の claims + VFS の種(フォント 2 つと GM CCs)+ `FLE_CALL_FRUITY=1`。入口は `lift/out-wf1/tools/fruity-config.sh`(決定 0126 の `run6.sh` + `final-a4.sh fruity` + `measure.sh` と同じ列。pin1 の `a4-fruity.sh on` と同じ列) |
| native の再現 | `a4-multi.sh`(無改造)で独立 1 回: 4:06.73、停止 `MessageLoop: idle`、**134,880,081 入口、5,515 関数、native `4ccd92946a5cefd0f96737dc43537accc51a61c590a55287e10297de809c963a`**(決定 0126 と一致。ツリーは 9574cb5 のまま) |
| wasm のビルド | 同じ分割 C(native のビルドの `src/` と `v-…/src/`)を `build_nmod_wasm.sh THREADS=1 JSPI=1 SJLJ=wasm VAR=force FRUITY=1` で: `rt` 5 秒、`units` 217 秒(最大 RSS 8.84 GB。dsp は 1 本ずつ)、`o1` 1,408 秒(最大 RSS 2.9 GB。locals 50,000 超の 4 単位 → 63 関数を -O1 の別単位 + 4 つの -rest)、`link` 3〜7 秒。**生成 C の locals 超過は 4 単位・0 件が残る(`over 50000: 0` の行 67/67)**。wasm 465,880,569 B、SHA-256 `38f1e9ec7d309b83c7b58912b682bc216d9c5084f692a7fa65c5bd826e40e0de`。**node 用と web 用のリンクの `.wasm` は `cmp` でバイト一致**(グルーだけ違う)。同じ `.o` からの再リンクもバイト一致。**OUT 名(`-g` のパス埋め込み)は wasm に入っていない**(`strings` で `out-wf1` の出現 0) |
| ビルドの規模 | 1 回の通しビルド(rt + units + o1 + link)約 27 分。`build_nmod_wasm.sh` の `FRUITY=1` は `main_two.c` を `-DFLE_FRUITY=1` で別のオブジェクトにし、`fruity.c`、`fruity_surface.c`、`fruity_idle.c`、`trace_fruity.c`、`ra0.c`(`emscripten_return_address` を wasm の関数に)を足す。`jspi_hooks.c` は入れない(同じ入口フックを `--wrap` するため) |
| 隔離 | Chrome はすべて: 新しい browser context、`--host-resolver-rules`、`--no-proxy-server`、`--disable-ipv6`、net-log は `Default`。**TCP の非ループバック接続 0**。UDP: `connect()` to `[2001:4860:4860::8888]:443` が実行ごとに 2〜9 回、**送信 0 バイト**(決定 0124 と同じ状況。基準「UDP の送信パケット 0」は満たすが、「UDP の接続 0」は満たさない。`NETLOG_TOLERATE_IPV6_PROBE=1` で許容して実行) |

## native と wasm の食い違い(重要な結果。原因まで特定)

**観測**: 入口数が native より 2 少ない(上の表)。G2・初画素の入口も同じく −2。DllMain までの入口数(1,009,338)と最初の G2 の証拠(7,059,458)は一致。

**二分(実測)**: (1) 計装(`-DFLE_WF1_TRACE`: lifted 入口の VA の FNV-1a ハッシュ。測定のバイナリには入っていない)で native と wasm の 2^20 入口ごとのチェックポイントを比べた: 最初の 32 個(33,554,432 入口)は一致、33 個目(34,603,008)が不一致。(2) その範囲 105 万入口を全部出して比べた: **33,702,791 番目(`f_79bec0`)までは同一、native はその次にもう一度 `f_79bec0` に入る**(wasm は入らない)。全体で差は 2 か所だけ: 33,702,792 番目と 33,838,152 番目(どちらも native が `f_79bec0` にもう 1 回入る。残りの 100 万入口は同一)。(3) `f_79bec0` を除く 134,880,032 入口の VA の列のハッシュ(2^22 ごと 32 個 + 最終): **native と wasm で完全一致**(`429e9134be00ecdc`)。`f_79bec0` の入口: native 49 回、wasm 47 回。

**原因(読み + 実測)**: `f_79c0f0`(レイアウト関数)は `f_79bec0` を 0x79c1b0 で呼び、その後 `cmp dword [rbp+0x5c], 0; jle 0x79c1c3` で、**[rbp+0x5c] が 0 より大きいときだけ** 0x79c1be でもう一度呼ぶ。`f_79c0f0` は [rbp+0x5c] を呼ぶ前に書かない(`f_79bec0` が書くのは [obj+0x53c] ≠ 0 の経路だけで、そこは 0)。つまり**ゲストは未初期化のスタックの 4 バイトを読んでいる**。診断の出力(入口 33,702,791): native の [rbp+0x58] = 0x369e6760、**[rbp+0x5c] = 0x7539**(= 64 ビット値 0x7539369e6760、前の呼び出しが同じスタック slot に残したホストポインタ)、wasm の [rbp+0x58] = 0x12f26760、**[rbp+0x5c] = 0**(32 ビットのポインタ)。ポインタと obj の距離は両方で 0x44ab0 で同じ(同じ古いポインタで、幅だけが違う)。native のホストポインタの上位半は 0x7539(≠ 0)、wasm32 のポインタの上位半は 0。
**因果の確認(実測、診断のみ)**: その 2 つの呼び出しだけ [rbp+0x5c] に 0x7539 を書くと(`FLE_WF1_POKE=2`)、入口数 134,880,081、`f_79bec0` 49 回、G2 34,866,773、初画素 134,983,755 が **native と完全一致**。全呼び出しに書くと(`FLE_WF1_POKE=1`、43 回)+40(負の結果として残す)。**この書き込みは診断で、直しではなく、結果の表には使っていない。**
**意味**: ゲストの挙動が古いスタックの内容に依存する。Windows 実機で何になるかは**未測定**(読み: 典型的なユーザーモードのヒープの 64 ビットポインタの上位半は 0 でないことが多い)。native と wasm のどちらも「Windows の挙動」とは言えない。画素・関数数・スレッド・停止は影響を受けなかった(2 入口の差のみ)。

**その他の差(実測、全出力の比較)**: DllMain までの 316 行で違うのは 2 行だけ(`entry 3 … guest-return=`: native 0、wasm はスタックが 0 クリアされないので古い値。上の「limits」の行)。スレッド表 17 行は同一、面の要約行は同一、窓の一覧は runtime の `why[6000]` バッファの切れ目だけが違う(ポインタの桁数が違うため。ハーネスの人工物)。

## 面の表の C API(`fruity_surface.c`、CONTRACT §2〜3 の最小実装)

`fle_raster_wrote` をリンカの `--wrap` で包み、呼び出しの**後**に表を更新する(画素を書いてから公開)。runtime(`fle_raster.c` を含む)は**編集していない**(公開関数 `fle_raster_surface_peek` などで読む)。node から検査(`surface_probe_fruity.cjs`、実測): 表と C の関数が一致(`fle_surface_count` 1 / 2、`fle_surface_info(i)` が表の欄と一致、範囲外 -1、未知の lid の `take` は -1、2 回目の `take` は空の矩形)。lid 13: 1930×1092、stride 7720、画素のオフセット 857,698,424、hwnd `7f990034`、位置 (-5, -6)、flags 3(生きている・可視)。lid 115: 119×192、位置 (900, 444)。
**ダーティ矩形**: 初画素で取った矩形は 201,380 個の非 0 画素を全部含む(外 0)。停止時の矩形 [6, 17, 1924, 1086] は、初画素の面から停止時の面までに変わった 1,718,856 画素を全部含む(外 0)。**最初に私は「矩形の外に 57,218 の非 0 画素がある = 一部が `fle_raster_wrote` を通らずに書かれた」と書いたが、誤り**(プローブ自身が初画素で矩形を取って空にしていたため。57,218 画素は初画素の面から変わっていない)。撤回して wall-log に残した。「全部の書き込みが `fle_raster_wrote` を通る」ことの証明ではない(`target_wrote` の呼び出し 12 か所を読み、すべて `fle_raster_wrote` に続くことは確かめたが、grep であって証明ではない)。present1 の CONTRACT の「窓表との hook(x、y、可視、hwnd、style)」は `fle_win_get` / `fle_win_dc_query` を呼ぶ形で実装した。`form_key` は 0(未実装。ゲストの `SetPropW` のポインタが再作成をまたぐという読みは未確認)。窓の再作成(lid が変わる)は**この実行では起きていない**。

## Chrome で見つけた不備(自分の Worker の不具合。結果として残す)

最初の Chrome のフル実行(13:18〜14:08、timeout 3,000 秒)は**何も残さなかった**。原因は 2 つ: (1) Web 用の emscripten のグルーは `Module.quit` を読まない(`quit_` は `throw toThrow`。node の枝だけが `Module.quit` を使う)ので、`run_node_*.cjs` の流儀の `quit:` でエンジンの終了を知ろうとした Worker は通知を受けない。(2) idle 停止の abort はスレッド 9(第 1 スレッドではない)で起きるため、`__wrap_abort` → report → `_exit(134)` が非第 1 スレッドの JSPI スタックで `ExitStatus` になる。`fle_thread_lib.js` はそれを「スレッド内の exit」として黙って扱い、**第 1 スレッドは WaitMessage で中断したまま**で `Module()` の promise は決まらない(決定 0124 の thread lib の冒頭コメントに書かれている形)。PM の観測(ページ側 renderer の CPU 5:46 のあと 45 分ほぼ 0)と合う。直し: `onExit` から `finish()`(`setTimeout 0`)、ランナーは `window.__live` を 2 秒ごとに `progress.jsonl` へ書き、Worker を CDP で attach、`quit` の 60 秒後に停止判定。短い実行(`FLE_FRUITY_MAX_CALLS=3000000`、10 秒)で終了経路を確認してから、フル実行を 1 回だけ。**同じ形は将来の本物のメッセージループ(第 1 スレッド以外の abort/exit)で再び起きる。**

## G3(idle で JS に制御を返す): 設計と測定

**現状の測定**: idle 停止(決定 0122 の条件 (b))は `schedule()` で「第 1 スレッドが入力待ち・キューが空・送信メッセージなし・他に走れるスレッドなし」のときに止まる。idle 停止を切った native(`FLE_FRUITY_NO_IDLE_STOP=1`)は **586,745 入口先**(n = 135,466,826、新規の関数 5,600)で `indirect call to 0x33d6e70 is not a known function entry`(lift の穴)に当たる = 決定 0126 の診断と同じ数。**仮想時計を進めて続行する道は、今は G3 にならない。** Chrome の現在のビルドでは、エンジンの実行中に Worker のイベントループは回らない(page が 5.0・15.0・60.0 秒に送った ping 3 つが、Worker に届いたのはエンジンの終了後 318,135 ms で、Worker の 10 ms タイマーは main の開始前の 138 回だけ)。**JSPI は協調スレッドの切り替え(microtask)にしか使われておらず、イベントループへは戻らない。**

**実装した形(最小)**: (1) `runtime/fle_thread.c` に弱参照の `fle_thread_idle_hook` と `schedule()` の 6 行(上の MERGE-NOTES)。(2) `harness/fruity_idle.c`: フックは面を kind 3(idle)で報告し、async の JS import `fle_idle_wait`(`WebAssembly.Suspending`。`fle_idle_lib.js`)で**呼んだスレッドの JSPI スタックを中断**する。(3) Worker の `Module.fleIdle` が page に `idle` を送る。page の**実入力**(pointermove、pointerdown)を Worker が C の export `fle_host_post_input(lid, msg, wParam, lParam)` へ渡す。許可リスト: WM_MOUSEMOVE、ボタンの down/up/dblclk、WM_MOUSEWHEEL、WM_KEYDOWN/UP。**WM_TIMER・WM_PAINT(runtime が仮想時計と更新領域から作る)と WM_QUIT は受け付けない**(決定 0122 の「偽らない」方針)。(4) 成功したら `fle_thread_post(owner, hwnd, msg, wp, lp)`(ゲストの `PostMessageW` と同じ関数)で窓の所有スレッドのキューへ入れ、idle の Promise を解決する。**idle 中は仮想時計を進めない**(`S-idle-clock-frozen`)。

**実測**(`chrome-g3`、`chrome-g3click`。`FLE_FRUITY_NO_IDLE_STOP=1` で idle 停止を切り、フックを呼ぶ): idle のイベント(Worker 時間 316.6〜317.9 秒、入口 135,889,417)で面の SHA は停止時と同じ。**実マウス入力 1 つ**(CDP `mouseMoved` / `mousePressed`、ブラウザの入力経路 → page の pointer イベント → `postMessage` → C の `fle_host_post_input`)→ ack **r = 1** → 第 1 スレッドが起き、**窓手続きが走る**。WM_MOUSEMOVE は 43 入口で `fle: unsupported instruction at 0x592470` = `comctl32.dll!_TrackMouseEvent` の import thunk(lift に無い)。WM_LBUTTONDOWN は 51 入口で `fle: DefWindowProcW: message 0x201: the default processing is not answered`。どちらも画素は変わらない。**この 1 本ずつで「ブラウザの入力 → エンジンのキュー → ゲストの窓手続き」が通ることは実測、「エンジンが応答して再び idle になる」は未達。**
**足していない理由**: `_TrackMouseEvent`(Learn: 「マウスポインタがウィンドウを離れる、または一定時間ホバーしたときにメッセージを post する」。`TME_LEAVE` は「マウスが窓の上にないなら離脱通知を直ちに生成」)は WM_MOUSELEAVE/HOVER を runtime が作る応答で、ブラウザのポインタに結ぶ設計が要る。`DefWindowProcW` の 0x201 も同様。いずれも新しい壁(決定 0126 の (g) の枠)で、出典なしの選択を伴う。
**ブラウザでの扱い(設計案、PM の判断)**: (A) 現在の実装 = idle で中断・実入力で再開。(B) idle の間にも時間が進む必要がある(ゲストの 16 ms の Sleep と 600 秒の SetTimer が期限になり、WM_TIMER が作られる)場合は、仮想時計を実時間に結ぶ(`performance.now` の経過分だけ進める。**設計のみ。作っていない**)。ただし idle 停止を切った native が 586,745 入口先で lift の穴に当たるので、(B) は先に `0x33d6e70` の穴を直す必要がある。(C) rAF/タイマーで JS 側から `WM_PAINT` の契機を作る必要は無い(runtime が更新領域から作る)。

## 出典なしの選択(S-…)と、読み

| 名前 | 値 | 理由 | 撤回条件 |
|---|---|---|---|
| `S-idle-clock-frozen` | idle の間(`fle_thread_idle_hook` の中断中)は仮想時計を進めない | idle 停止を切ってクロックを進めると lift の穴(0x33d6e70)に当たる(native で実測)。Learn にこの場合の規則は無い | 仮想時計を実時間に結ぶ設計(B)を実装するとき |
| `S-idle-hook-after-stop-flag` | idle のフックは idle 停止が OFF のとき(`FLE_FRUITY_NO_IDLE_STOP=1`)だけ呼ぶ | 既定(停止)の測定を変えないため | ブラウザ用の既定を「フック」にすると決めたとき |
| `S-input-allowlist` | `fle_host_post_input` が受けるメッセージ: WM_MOUSEMOVE、WM_{L,R,M}BUTTON{DOWN,UP,DBLCLK}、WM_MOUSEWHEEL、WM_KEYDOWN/UP(番号は winuser.h = 非公式) | 決定 0122 の「偽らない」: ブラウザの入力にある物だけ。WM_TIMER/WM_PAINT/WM_QUIT は不可 | 入力モデル(座標系・キー状態・フォーカス)を決めるとき |
| `S-input-surface-coords` | page の pointer の座標は canvas A の `offsetX/Y`(面のピクセル、DPR 1)を lParam の x、y にする。lid 13 の窓のクライアント座標 = 面の座標とみなす | 窓の位置 (-5, -6) と非クライアント領域 0(決定 0126 の `S-adjustrect-nc-zero`)を考えない | 窓のクライアント原点を出典つきの値にするとき |
| `S-wasm-stack-regions` | (決定 0124 の `S-…` を踏襲: 非先頭スレッドのシャドウスタック 1 MiB) | 0124 | 0124 の改訂。実エンジンの 9 本が 1 MiB で足りたことは `fle_thread_wasm_hiwater` で**測っていない** |
| `S-new-surface-all-dirty` | 面のバッファが新しい(大きさ・画素の位置が変わった)ときは矩形を全面にする | 窓の再作成の面は全部書き直しと見る | 窓の再作成の実測が別の扱いを要求するとき |
| `S-progress-2^22` | wasm のハーネスは 2^22 入口ごとに `Module.fleProgress` を呼ぶ | ランナーの停止検出のため(32 回の呼び出し) | 不要になったとき |
| (読み)`wf1-stale-stack-upper-half` | 上の食い違いの原因の読み(診断で因果を確認したが、f_79c0f0 を書いたコンパイラの意図は未確認) | | Windows 実機の測定 |

## 意外なこと・限界(測れなかったこと)

- **Chrome の実行時間**: エンジンの実行 316〜318 秒で、native の呼び出し(184.5 秒)の 1.7 倍、node の wasm(呼び出し 320〜326 秒)とほぼ同じ。1 秒あたり約 0.43 M 入口(native 0.73 M)。`-finstrument-functions` のフックが 1.35 億回のうち全部を数える(測定用)。**本番のビルドではない**。最適化(`-O1` 以上、フック無し)の wasm は作っていない。
- 画面の面は**1 回の idle 停止の時点**の画素。窓の再描画・差分更新(ダーティ矩形での部分描画)を Chrome で通した実測は無い(プローブは node の 1 回)。方式 C(SAB)と複数窓(ポップアップ)・DPR・実機の vsync・Firefox・Safari は**未測定**。
- 面のダーティ矩形を使った「差分だけ canvas に描く」経路は Chrome で動かしていない(present1 の `Presenter` は別ハーネスで実測済み)。ここでは全面を毎回変換した(1930×1092、約 2.3 ms の JS ループ)。
- node の JSPI の既定スタック 984 kB に対し必要量は 425〜450 kB: **Chrome の既定は node と同じ 984 kB**(`--js-flags=--help` で確認済み、決定 0124)だが、450 kB の Chrome 実行はしていない。
- `fle_thread_wasm_hiwater`(シャドウスタックの使用)は測っていない。10 スレッドの JSPI スタックが全部 984 kB を取る場合のメモリは、RSS に出ていない(触れていないページは入らない)。
- 全体テストは走らせていない(指示)。個別: `wall_fr9t` 20、`wall_fr8` 45、`raster` 10 が通る。新しいテスト(`fle_host_post_input` の許可リスト、`fruity_surface.c` の表)は**書いていない**。変異検査もしていない。
- `build_nmod_wasm.sh` の `FRUITY=1` なしの経路(FRUITY 無効の wasm)を**再ビルドして a4 を走らせていない**(コマンド文字列としては不変、`bash -n` 通過)。決定 0124 の `THREADS=1` の経路のオブジェクト(`fle_thread.o`)が cmp 一致であることだけ確かめた。
- Windows 実機との照合 0 件。native と wasm の食い違い(未初期化スタック)が実機でどうなるかは未測定。

## PM の判断点

1. **G4 の記録**: 「G4 到達(native と同一画素、headless Chrome、GPU なし、1 マシン)」。ただし画素は決定 0126 の画素モデル(`S-` 選択)の結果で、実機の GDI との一致ではない。文字は描かれていない(決定 0126 のまま)。
2. **native と wasm の食い違いの扱い**: ゲストが未初期化のスタックを読む(`f_79c0f0` → `f_79bec0`)。(a) 放置(2 入口。画素・関数数・スレッドは同じ)、(b) 両ホストで同じになるようスタックの初期値を決める(出典なしの選択。実機の値は未測定)。推奨: (a) で記録のみ。実機の値が分かるまで偽の値を入れない。
3. **G3 の続き**: 1 周に必要なのは `comctl32!_TrackMouseEvent`、`DefWindowProcW` の WM_LBUTTONDOWN/WM_MOUSEMOVE の既定処理などの新しい壁。それを (g) の枠で別レーンにするか。idle の仮想時計(設計 B)は先に `0x33d6e70` の lift の穴(586,745 入口先)が要る。
4. **Worker の終了検出**: abort/exit が第 1 スレッド以外で起きる形は、将来の本物のメッセージループでも起きる。`fle_thread_lib.js`(決定 0124)に「非第 1 スレッドの exit で全体を終わらせる」経路を入れるか(今はハーネス側の `onExit` で補った)。
5. **JSPI スタックの下限(425 kB 超)**: Chrome の既定 984 kB との余裕は約 2.2 倍。深い呼び出し(全 10 スレッドの最大)を測るか。
6. **実行時間**: 316〜318 秒は計測用のビルド。本番に近づけるなら `-finstrument-functions` を外した wasm と、`-O1`/`-O2` の別の測定が要る。

## 規則違反・不備の自己申告

- **1 時間ごとの報告**: 13:25 と 14:25 の定時報告を送らなかった(PM の観測の連絡を 14:08 に受けた)。送ったのは 12:25、14:35、15:30。
- **末尾 `&`**: native/wasm の比較コマンド 1 回で、native の実行を `... ) &` でバックグラウンドにした(その 1 つのシェルコマンドの中。`cd` も壊れ、wasm 側が即失敗。native 側は最後まで走り、wasm 側は前景で取り直した)。wall-log に記録。
- **走っているジョブのスクリプトの編集**: `o1` 段の実行中に `build_nmod_wasm.sh` のコメントを書き換えた。bash は走りながらファイルを読むので、段の終了後に構文エラー(rc 2)を出した。段の成果物は完成(67/67、locals 超過 0)で、結果に影響しない(wall-log 02)。
- **`tail --pid=… -f` を待ちの代わりに 1 回**(uutils の tail が `--pid` を受けず即失敗。待ちのループではない)。
- **書いて撤回した結論**: ダーティ矩形の外の非 0 画素の件(上)。誤りの理由を wall-log と本文に残した。
- **ランナーの停止の待ちが長すぎた**: `stall` を 600 秒にして、`quit` 後の判定が遅れ、最初のフル実行(14:18〜14:34)の 15 分が余計だった(`quit` の 60 秒後判定を足した)。
- `/tmp/opencode/wf1` の使用量は終始 0〜数十 MB(net-log と使い捨てプロファイルを実行ごとに削除)。作業ディレクトリ `lift/out-wf1` は一時 8.6 GB(15 GiB 以内)、掃除後に 300 MB 以内にする。
- git の変更系操作なし。他の `out-*` は読み取りのみ(`out-pin1` の A.patch を読み、`a4-multi.sh`・`a4-fruity.sh` は編集していない)。

## 限界(要約)

実機 Windows 0 件。Chrome は headless・GPU なし・1 マシン(Linux、Chrome 156)。画面の更新は idle 停止の時点の 1 回(差分の更新・複数窓・再作成・DPR・入力後の再描画は未測定)。G3 は 1 周に届かず(壁 2 つ)。FRUITY 無効の wasm・全体テスト・新しいテストなし。
