# 0234i(mt3): 段階 B(本物のスレッドでの実行)の設計の確定と、export の表・GIL の外の共有状態の表

状態: 作業木の試作(旗つき足場)で測った事実と、そこから決めた設計。成果物の差分はまだ無い(規則 4: 未完成の物は取り込まない)。測定はこの機械(負荷 18〜30)・node 24 の worker_threads・最適化なしの wasm で、client の機械の値ではない。

## 1. 測ったこと(実エンジン)
- 構成: F2xH7 の非ハイブリッド NO_FS=0、共有メモリ、R/FL/X と YH/MM/x87/MXCSR/SEH/コールバック深さ/TEB・TLS ポインタ/dispatch キャッシュの TLS、GIL、cmpxchg/xadd/xchg[mem] の本物のアトミック、実時計、**1 スレッド 1 Worker**(CreateThread のたびに pthread を 1 本起動。最大 31)。
- cmp-short(起動 16 スナップ): アトミック版で 4/4 完走(約 20 秒)、大域インターロック版で 0/3、協調 + 旧 GIL の 1 Worker に 2 本以上載せた版は常に止まる。
- 参照プロジェクト 30 秒再生: 完走。同 5 分再生: **312 秒(実時計)を止まりなしで完走**(名前つきの停止 0、`late_frames` 0、ミキサーの Lock/Unlock が 452 → 2263 回)。再生区間のミキサーは 5.80 ブロック/秒(テンポ約 0.54。最適化なしの wasm・負荷 22〜27。**P3 の判定には使えない**)、起動後〜アイドルは 10.8 ブロック/秒(テンポ 1.0)。
- 協調モードの 84 行との比較: 実時計では時間の位相で変わる面(1930x1092 のオーバーレイ)があるため全行一致は成立しない。メインのダイアログ面(874x573)は 14/16 スナップで一致。

## 2. 確定した設計
1. **1 スレッド 1 Worker**。理由 = FastMM のスピンロック(ゲストの `lock cmpxchg byte [lock]` + pause の無限再試行、間接呼び出しが無いのでスライスが来ない)で、協調スケジューラは『ロックを持ったスレッドが同じ Worker の別スレッドに追い出されたまま』を作れる。実測: 1 Worker に 2 本以上 → 全 Worker が f_407e20 ← f_409440(FastMM の取得)で回り続けて進まない。Worker 数は FL のスレッド数に従う動的な形(CreateThread → 起動時に Worker を割り当てる)。JSPI のスタック切替・`pick_other`・優先度スケジューリングは使わなくなる(各 Worker は自分の 1 本だけを動かす)。
2. **ゲストの LOCK 付き命令・XCHG[mem] は本物のアトミック**(mt4 段階 A)。全体で 1 本の割り込み禁止にすると、FastMM のスピンが 1 つの語に群がって持ち主が進めない(実測 0/3 完走)。
3. **GIL は `fle_imp_*` の入口(`--wrap`)で取る**。第 1 Worker(ページのイベントループを持つ)は GIL を**非同期**(JSPI + `Atomics.waitAsync`)で待つ。第 1 Worker が同期で GIL を待つと、第 2 以降の Worker が第 1 Worker のイベントループを要する同期 proxy(NO_FS=0 の glue の stdio など)を GIL を持って待ったときにデッドロックする(実測)。**不変条件 I1: GIL を持つ Worker は、他の Worker の JS イベントループを同期で待たない**。出荷の NO_FS=1 の glue には pthread からの同期 proxy が無い(NO_FS=0 の node 駆動だけ、glue の copy で pthread の `fd_write` を局所にして回避)。I1 は『glue に `proxyToMainThread` を呼ぶ同期の syscall が無い』を CI の検査にして守る(未着手)。
4. 起床漏れの規則: 他の Worker のスレッドを READY にする箇所(`try_wake`、CreateThread、ResumeThread)は、その Worker の wake word を bump して notify する。idle の待ちは wake word の値を比べて返る(失われた起床が無い)。futex の mutex は『寝た後は常に 2 で取る』(これを破ると 3 秒の空白が出る。実測して直した)。

## 3. (iii) JS から呼ばれる export と GIL
JS(ページの Worker)が wasm の export を呼ぶ経路。第 1 Worker では**エンジンが fle_idle_wait で中断している間にだけ**呼ばれる(idle-driver.ts: Module.fleIdle の中から)。呼び出しの間 GIL は解放されている(idle フックが解放)ので、export は GIL を `fle_gil_enter`(同期取得)してよい: 他の Worker が GIL を持つ時間は runtime の関数 1 回分(µs〜ms)で、I1 により第 1 Worker のイベントループを待たない。
| export | 何をするか | 状態 | 扱い |
|---|---|---|---|
| fle_g3_mouse / key / display | 入力のキューへ積む(fle_input_host_*) | runtime の入力キュー(書く) | GIL を取る。積んだら入力待ちスレッドの Worker へ起床(`try_wake` が notify) |
| fle_g3_pending / fle_g3_ticks / fle_g3_clock_to | キューの長さ・時計の読み・時計の前進 | 時計は実時計(読むだけ。`clock_to` は実時計では何もしない) | ticks は GIL なしでよい(原子的に読む)。clock_to は不要になる |
| fle_g3_snapshot / probe_timer / timers | 面の hash・タイマーの記録(試験用) | 窓表・面(読む) | GIL を取る(試験用の入口。製品のコードに置かない方向) |
| fle_host_post_input | 窓へ WM_ メッセージを積む | スレッドのキュー(書く) | GIL を取る |
| fle_desk_compose / resync / describe | 窓を 1 枚の画像に合成 | 窓表・面(読む)、`need_full`(書く) | GIL を取る(合成は数 ms。その間は他の Worker の runtime 呼び出しが待つ: 測定が要る。ダブルバッファ化は第 3 段) |
| fle_av_poll、fle_filedlg_saved_poll、fle_app_open / open_state | ファイル選択・アプリのビュー | 小さな状態 | GIL を取る |
| fle_surface_table / count / info / take_dirty / scratch | 面の表の読み出し(Worker の表示) | 面の表(読む)、dirty(書く) | GIL を取る |
| fle_wasm_thread_entry | 新しい JSPI スタックの入口(JS の fle_thread_lib.js が起動) | — | 入口で GIL を取る(実装済み) |
実装済み(試作): fruity_idle.c の fle_g3_* は `fle_gil_enter / fle_gil_leave`(再入可: 同じ Worker が持っていれば取らない)。`fle_export_depth`(export の中)は GIL の待ちを同期にする(中断できる JSPI スタックが無い)。
未実装: 上の表の fle_desk_*、fle_surface_*、fle_av_poll、fle_app_*、fle_filedlg_*、fle_host_post_input。受付箱(export が GIL を待たずに積む)は、I1 を CI で守れるなら不要。守れない場合の代案として残す。

## 4. (iv) GIL の外で動く runtime(= 生成 C が `fle_imp_*` 以外で直接呼ぶ関数)の共有状態
方法: 生成 C のオブジェクト(`obj/mod`、`obj/fle`、2,318 個の `fle_imp_*` 以外)の未定義シンボルのうち、runtime に定義がある物。これらは `--wrap=fle_imp_*` の対象ではなく、GIL を取らない。
| シンボル(例) | 定義 | 状態 | 判断 |
|---|---|---|---|
| fle_x87_*(約 30 関数: arith、fld、fst、fcom、fxch、fsincos …) | fle_x87.c | x87 レジスタ・制御語(TLS)。**作業用の `static unsigned X`(その命令の例外ビット)と `keep_snan` は共有の static だった** | ~~スレッドごと。安全~~ **訂正(18:45): 安全でなかった。複数 Worker が同時に超越関数を走らせると、丸めの窓(`X = 0` で 2 回丸めて例外ビットまで比べる)が食い違い、FSINCOS が『rounding is not decided at 256 bits』で停止した(プロセッサ数 4 の実験)。`_Thread_local` にした(0234q §7)** |
| fle_mxcsr_load / store | fle_mxcsr.c | MXCSR(TLS) | 安全 |
| fle_cpuid、fle_xgetbv_stop | fle_cpuid.c | 定数表(読むだけ)の見込み | 要確認 |
| fle_tls_values | fle_vex.h | 最初のスレッドの TLS スロット表(静的。ほかのスレッドは T[i].tls) | 最初のスレッドだけが読む。安全 |
| fle_hook_register / fle_hook_target、fle_site_store32 | fle_vmap.c | 呼び出し先の上書き表(census・site override) | 起動時に登録、あとは読む。要確認(書き込みが走行中に起きるか) |
| fle_swap_ctx_*(約 17 関数) | harness/b2d1/b2d_swap.cpp(Blend2D の ctx に差し替えた描画) | BLContext(描画 context ごと)。アダプタの静的な設定・census | **危険**: 複数のスレッドが同時に別 context で描く場合のアダプタの静的状態と Blend2D のランタイムの共有状態(スレッドプール・フォント管理)。GIL の入口(wrap)を足す方針 |
| fle_hle_warp_bilin ほか fle_hle_* | fle_hle.c | 静的な統計 (g_wst 等)・モード変数 | **危険**: 統計と初期化の競合。GIL の入口を足す |
| fruity_host_slot0/1/2 | harness(偽のホストオブジェクトのメソッド) | 呼び出しの記録(静的) | 危険: GIL の入口を足す |
| fle_dispatch_miss → fle_win_try_thunk | fle_win.c | COM スロット(DirectSound) | 試作で GIL を取る(入口に追加済み) |
| fle_progress_slice | fle_thread.c | スライスの記録 | 試作で GIL を取る |
方針: wrap の対象を `fle_imp_*` に限らず『生成 C が直接呼ぶ runtime の関数』に広げる(データ駆動: 生成 C のオブジェクトの未定義シンボル ∩ runtime の定義)。許可リスト(GIL を取らない)= TLS だけを触る関数(x87、mxcsr)と定数表。これは `fle_imp_*` の自動生成(gen_wrap.py)の延長で、FL の名前は使わない。
その他の GIL の外の共有状態(ゲストコードが触る分): ゲストのヒープ・スタック・データ(共有メモリ。同期はゲスト自身の LOCK 命令と CS API)、`fle_progress_count / limit`(カウンタ。競合しても害は小さい: スライスの発火が遅れるだけ)、`fle_dispatch_hook`(定数)、アトミック化した RMW の interlock(整列していない/16 バイトだけ)。
「indirect call to 0」(参照プロジェクトの試行で 1 回): 大域インターロック版で起きた。アトミック版の 30 秒再生・5 分再生では 0 回。原因の確定は未了(競合ではなく、LOCK 命令の非アトミック性が原因だった可能性が高い: 読み)。

## 5. 未着手・残り
- ミキサーが専用 Worker で走ることの確認(CPU の内訳。Worker ごとのスレッド CPU 時間)、テンポ 1.0(静かな機械・最適化版・Chrome/Firefox の実ブラウザ)。
- ブラウザの駆動: ネストした Worker での pthread(emscripten の glue がそのまま使う)。Chrome 156 / Firefox 158 の確認は玩具(jn)まで。エンジン本体は未確認。
- audio ring の共有メモリ化(段階 C)。いまの node 試験は ring を使わない(外部の消費者が無い)。
- 旗の除去: FLE_GIL / FLE_TLS_MORE は `tools5/strip_ifdef.py` で畳める。環境変数 FLE_WORKERS は定数(1 スレッド 1 Worker)にする。診断(fle_dbg*、ノートのリング、DBG1、スライスごとの JS スタック)は成果物に入れない。

## 6. GIL の取り方の変更(17:30、0234q)で守られる不変条件と、GIL を取らない import
- GIL が守る状態の一覧(§3 の export の表と §4 の GIL の外の共有状態の表)は**変わらない**。変えたのは取り方だけ: 速い経路は `nwait` に関係なく CAS(割り込みを許す)、眠っている待ち手は順序に数えない、眠っている待ち手がいるときだけ起こす(`fle_gil_nsleep`)。
- 不変条件(`fle_gil_assert` と名前つきの停止で残す): (I-a) 持っている Worker が取らない、(I-b) 持っていない Worker が放さない、(I-c) 解放は `nsleep` を見て必ず起こす(眠りに入る前に `nsleep` を足し、もう一度ロック語を見る)、(I1) GIL を持つ Worker は他の Worker のイベントループを同期で待たない(`check_i1.py` は CI で守る)。
- I1 が保たれるので、第 1 Worker も GIL を**同期で**待てる(JSPI の `Atomics.waitAsync` は廃止。再開にイベントループ 1 周かかり、ページの仕事の分だけ 10〜100 ms 遅れた: 0234q)。この廃止は I1 が崩れると(第 2 以降の Worker が GIL を持ったまま第 1 Worker のループを同期で待つ)デッドロックを作るので、`check_i1.py` が落ちる変更は入れない。
- GIL を取らない import(`runtime/fle_gil_lockfree.inc`): runtime の共有状態を読みも書きもしない物だけ。表と理由は 0234q §4。
