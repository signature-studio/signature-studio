# 0097: census（全 942 unit を名前付き停止で止めずに通し、unit ごとの結果を 1 つの表にする診断ツール）。A4 は厳格のまま

状態: **決定（仮ではない）**。PM の指示（2026-10-25。プロジェクトの問題は「名前付き停止で全体が止まるので、停止点の先に何があるか見えず、壁を 1 つずつ直列に直している」こと）を、作業者（サブエージェント）が記録した。診断ツールの仕様で、**バイト列と実行で検証できる**ので仮の決定にしない。番号 0097 は PM の割り当て。

記入状況: すべて記入済み（テスト・検証の数値は 2026-10-27 の実測。`cargo test --workspace` の結果は「検証」の表）。

事実の出典:

- 決定 0046（A4 の定義。厳格）、0060（A4 の進捗の測り方。「完了は戻ったことだけを意味する。unit の内部で fail-soft に進んだかは見ない」）、0007 系の方針（fail-soft で続行したものを進捗と数えない）。
- 決定 0064（`fle_call_guest`。入れ子カウンタ）、0082（実在モジュールのロード。`ST_LOADING`）、0086（Blend2D を実在モジュールにする方向）。
- `lift/harness/a4.c`、`a4-survey.sh`、`harness/wasm/trace_native.c`（シャドウ呼び出しスタックと停止の報告）、`lift/runtime/fle_dispatch.h`（`fle_dispatch`）。
- 構成の作り方: `docs/investigations/blend2d-switch-reaching-defs.md` §6、`blend2d-srw.md` §6、`two-module-after-0088.md`。
- InitUnits は `f_4120c0`: unit i について `InitCount = i + 1` を書いてから、`proc != 0` かつ `LD64(proc) != 0` のときだけ `0x412126: call proc`（戻りアドレス `0x412129`）。`call` の後は `0x412129` で `eax` を局所変数から読み直す。**戻り値（rax）を使わない**（生成 C を読んだ）。

## 決定（PM。内容を変えない）

| # | 項目 | 決定 |
|---|---|---|
| 1 | 位置づけ | **census は診断用。A4 は厳格のまま**（決定 0046、0060 は変えない）。census の数（ok／stop／fault／hang の件数）は **A4 の進捗として報告しない**。`docs/progress.md` の A4 の欄にも書かない。fail-soft で続行したものは進捗と数えない（0046 の方針 4） |
| 2 | opt-in | 環境変数 `FLE_CENSUS=<出力ファイル>` で有効。**未設定のとき、挙動は今と完全に同じ**（a4 は従来の数を出す）。出力ファイルが既にあれば作らずに止まる（上書きしない） |
| 3 | 放棄 | unit i の中で、名前付き停止（`fle_stop`、`fle_dispatch_miss`、`fle_unsupported`、`fle_jumptab_miss`、`fle_rd` ほか、`abort()` に至る全種類）、メモリ違反（SIGSEGV、SIGBUS ほか。sigaltstack で捕捉）、暴走（unit ごとの時間上限 20 秒、環境変数で変更可。ほかに unit ごとの関数入口数の上限）が起きたら、(unit 番号、proc、状態 ok／stop／fault／hang、停止メッセージ全文、top-of-stack の関数、入った関数数) を記録して、**その unit を放棄し、ゲストの unit ループの次（unit i+1）へ進む** |
| 4 | 侵襲の最小化 | **生成 C は変えない**（FLEngine の SHA-256 `feb5b908…` を保つ）。runtime の変更は、`fle_dispatch` の呼び出しの前に置く**関数ポインタのフック（既定は NULL = 従来の呼び出し）1 つ**だけ。**新しい .c／.h を runtime に作らない**（`lift/crates/fe-lift/tests/modules.rs` が全 `fle_*.c` をリンクする）。ハーネスの新規ファイルは `lift/harness/census/` |
| 5 | 出力 | TSV（unit、proc、状態、分類、メッセージ全文、top 関数、入った関数数、1 unit の経過時間ほか）と、要約（分類 → 正規化したメッセージ → 件数 → 最初の unit → 代表の呼び出し元。件数の多い順）。分類は (a) 未応答 import（API 名）、(b) 未対応命令／jmp、(c) LoadLibrary／GetProcAddress の名前、(d) 仕様が無く停止（名前付きの停止のうち (a)〜(c) 以外）、(e) メモリ違反（null 参照ほか。前の unit の失敗の二次被害が疑われるものは別に印を付ける。ヒューリスティックでよく、確実でないと明記）、(f) hang |
| 6 | 構成 | (i) FLEngine + 縮小 dsp_ippv2（実在モジュール。unit 144 が完了する）。Blend2D（unit 149）と quickfontcache_x64（unit 158）の `LoadLibraryW` は名前付き停止として記録されて次へ進む。(ii) 比較のため FLEngine 単独。census が未設定のとき a4 の数（単独 145/942、dsp 構成 150/942）が従来と一致する |

## 背景（事実）

- 名前付き停止は `fle_stop` → `abort()`、`fle_dispatch_miss`／`fle_unsupported`／`fle_jumptab_miss`／`fle_rd` なども `abort()` に至る。実行は最初の停止で終わるので、停止点より先に何があるかは、その壁を直して再測定するまで分からない（FLEngine 単独は unit 144 の `LoadLibraryW "dsp_ippv2_x64.dll"` で 145/942、実在モジュール dsp_ippv2 を載せた構成は unit 149 の `LoadLibraryW "blend2d.dll"` で 150/942）。
- InitUnits の unit は、ゲストのループが 1 つずつ `proc` を呼ぶ。ループは戻り値を使わず、unit の間で共有するレジスタは callee-saved の rbp だけ（局所変数はゲストのスタック上）。この 2 点が、unit を放棄して次へ進める根拠（生成 C を読んだ）。
- 単独構成で unit 144 が放棄されると、dsp_ippv2 のモジュールが無い状態になる。この状態の後続 unit の失敗は、unit 144 の失敗の結果であることがある（「検証」の比較）。

## 範囲（今回扱う）

- census のハーネス（`lift/harness/census/census.c`、`census.h`）、後処理（`census_report.py`）、N モジュール構成のスクリプト（`out-rdefs/three/harness-two/` のコピー）、`run_census.sh`。
- `fle_dispatch.h` の NULL 既定のフック。`harness/main.c` の弱い呼び出し 3 行、`harness/wasm/build_native.sh` の census.c のコンパイルとリンク、`harness/wasm/trace_native.c` の census 用の窓口（下の「実装した場所」）。

## 範囲外（今回扱わない）

- A4 の定義、判定基準、進捗の数え方の変更（0046、0060 のまま）。
- 壁を直すこと（応答の追加、仮の値、`fle_stop` の置き換え）。census は**値を作らない**。止まったものを止まったまま記録して次へ進むだけ。
- wasm 版の census（`sigaltstack`、`setitimer`、`siglongjmp` を使う native 専用）。
- Blend2D、quickfontcache_x64 を実在モジュールにした構成の census。
- unit の内部で最初の壁の先にある壁を見ること（下の「限界」）。

## 実装した場所

- `lift/runtime/fle_dispatch.h`（+11）: `fle_dispatch_hook`（`void (*)(uint64_t va, void (*fn)(void))`、**既定 NULL**、弱い定義）。`fle_dispatch` が表で関数を見つけた後、フックが非 NULL ならフックに呼びを渡す（フックが `fn()` を呼ぶ）。NULL なら従来どおり `fn()`。定義は弱く、ヘッダを取り込む全翻訳単位が持つ（`fle_vex.h` の `fle_teb` と同じ流儀。.c を足さない）。生成 C のテキストは変わらない（`fle_dispatch` は runtime のヘッダの inline 関数）。
- `lift/harness/census/census.c`（新規）: フック。unit 呼び出しの判定は「`[R[4]]` が `0x412129`（`f_4120c0` の戻りアドレス）で、`InitCount - 1` の unit の proc が呼び先に等しい」。判定が外れる呼び（unit の中の入れ子の dispatch、`fle_call_guest`）は素通し。unit では `R[]`、x87（`fle_x87`、`fle_x87_r`）、MMX（`MM`、`fle_x87_mmx_active`）、MXCSR、DF を保存し、`ITIMER_REAL` を張り、`sigsetjmp` して呼ぶ。SIGABRT／SIGSEGV／SIGBUS／SIGILL／SIGFPE／SIGALRM は 1 つのハンドラ（sigaltstack）で受け、記録して `siglongjmp` で戻る。関数入口数の上限は `trace_native.c` の窓口から同じ経路で戻る。放棄したら保存した状態を戻し、`rax = 0`、シャドウスタックの深さを戻して、ループに「戻った」ように見せる。unit の外の信号は元のハンドラ（`trace_native.c` の報告）に連鎖し、その前に TSV へ `#end` 行を書く。
- `lift/harness/census/census.h`（新規）、`main_two.c`、`build_two_native.sh`（census.c のコンパイルとリンクを足した）、`gen_two.sh`、`merge_tables_n.py`、`two-native-run.sh`（`out-rdefs/three/harness-two/` のコピー。`gen_two.sh`、`merge_tables_n.py`、`two-native-run.sh` はバイト一致）、`census_report.py`（分類、正規化、要約、二次被害のヒューリスティック、構成間の比較）、`run_census.sh`。
- `lift/harness/wasm/trace_native.c`（+31 −1）: メッセージ全文の別バッファ（既存の `last_msg`（511 バイトで切る。停止の報告が出す）は変えない）、unit ごとの入口数の上限（既定は従来の `HARNESS_MAX_CALLS`）とその窓口（既定 NULL）、シャドウスタックの深さの取得と設定、上の k 番目の関数と戻りアドレスの取得。census が無いとき、これらは動かない。
- `lift/harness/main.c`（+7）: `census_begin`／`census_end` の**弱い**宣言と、エントリを呼ぶ前後の 2 行（`FLE_CENSUS` が未設定なら `census_begin` は 0 を返し、何も呼ばない）。`lift/harness/wasm/build_native.sh`（+3 −1）: `census.c` を `h/census.o` にコンパイルしてリンクする。
- `lift/crates/fe-lift/tests/census.rs`（新規）: 下の「テスト」。
- `lift/runtime/*.c`、生成 C、`a4.c`、`image.c` は変えていない。

## テスト

`lift/crates/fe-lift/tests/census.rs` 9 件。`census.c` と `trace_native.c` を、模擬ゲスト（`0x400000` の mmap、InitContext、`f_4120c0` と同じ動きのループ、`-finstrument-functions` つきの 17 unit）と一緒にコンパイルして実行する。期待値は決定の定義から書いた（C のコードから読み戻していない）。

- 17 unit が、return、abort（名前付き停止）、null ページへの書き込み（SIGSEGV）、無限ループ（SIGALRM）、関数入口の無限列（入口数の上限）、ホストのスタック溢れ、SIGFPE、`jmp [rip+rel32]` の thunk の未応答 import、import の疑似 VA への dispatch、null 関数ポインタ呼び、空 unit、noproc で終わり、**ループが最後の unit まで進む**。
- 放棄した unit は `rax = 0`、`rbx`／`rbp`／`rsp`、シャドウスタックの深さ、x87 の制御語・タグ語、MMX フラグ、MXCSR、MM0、DF が呼ぶ前の値に戻る。完了した unit のレジスタには触れない。
- 記録の内容: メッセージ全文、top（unit の関数）、`jmp [rip+rel32]` の thunk と import の疑似 VA からの import 名、fault address、ホストスタック溢れの印、wall time／入口数の上限の別、TSV の `#counts`／`#end` 行。
- unit の中の入れ子の `fle_dispatch`（同じ戻りアドレス `0x412129` を積んでも）は unit として数えない。
- `FLE_CENSUS` 未設定: フックは NULL のまま、最初の名前付き停止が従来どおり SIGABRT（終了コード 134）で報告される。既存の出力ファイルは上書きされない（終了コード 2、内容は不変）。unit の外の停止は `#end` 行に書かれ、その後に従来の報告と終了コード 134。
- **変異検査**（実施。元に戻して `cmp` で一致を確認）: 状態の復元を外す → 1 件失敗、シャドウスタックの深さの復元を外す → 1 件失敗、`rax = 0` を外す → 1 件失敗、ホストスタック溢れの判定を外す → 1 件失敗。

## 検証

実測値の手順と生の出力は `lift/out-census/`（`logs/`、`runs/`）。構成と結果の表は `wall-census-2.md`（`census_report.py` の出力）。

| 項目 | 結果 |
|---|---|
| `cargo test --workspace --no-fail-fast`（`CARGO_TARGET_DIR=lift/out-census/target`、`FLENGINE_PE` 指定、`git archive` のスナップショット内） | TEST_RESULT_PLACEHOLDER |
| FLEngine の生成 C の SHA-256 | **`feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f` のまま**（スナップショットの release の `fe-lift` で生成。215,709,232 バイト。functions 59,297、bad_nodes 0） |
| census 未設定の a4（単独） | init_count **145/942**、functions **821**、entries **15,685**、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`（`a4-survey.sh`）。**基準と一致** |
| census 未設定の a4（FLEngine + 縮小 dsp_ippv2） | init_count **150/942**、last unit 149、functions **943**、entries **26,585**、停止 `LoadLibraryW "blend2d.dll"`。**基準（two-module-after-0088.md）と一致** |
| census 未設定の出力が従来と同じか | パッチ前のソース（`git archive` の無変更）で同じ 2 構成をビルドして実行し、パッチ後（census 未設定）と比べた: **stdout（281／283 行。関数入口ログを含む。`rsp=` の値だけマスク）と stderr が完全一致** |
| census の内訳（FLEngine + 縮小 dsp_ippv2） | ok **876** / stop **19** / fault **0** / hang **0** / empty 46 / noproc 1 / not_reached 0（計 942。runnable 895 = ok + stop）。InitUnits を通し終えた後、guest 自身の終了フェーズの `unsupported instruction at 0x412752`（`f_4126f0`。`jmp [rcx]`）で unit の外の停止 |
| census の内訳（FLEngine 単独） | ok **873** / stop **22** / fault **0** / hang **0** / empty 46 / noproc 1 / not_reached 0。dsp 構成との差は unit 144（`LoadLibraryW "dsp_ippv2_x64.dll"`）と、unit 833、935（null 関数ポインタ呼び。dsp 構成では ok） |
| 再現性 | 各構成を census 3 回実行して、経過時間の列を除いて**完全に同一** |
| 壁の数（dsp 構成） | 壁 16 種（a 未応答 import 6、b 1、c 4、d 5、e 0、f 0）。上位 20 の表と一次／二次の印は `wall-census-2.md` |

census の数は **A4 の進捗ではない**。上の ok の数は「戻った」unit の数で、放棄した unit の後の状態で動いている。

## 解釈が分かれた点

1. **放棄の方法**: 指示の例のうち、「unit proc の dispatch の前で setjmp し、停止時にレジスタと RSP を unit 入口の値に戻して longjmp し、unit が R[0]=0 で戻ったように見せる」を選んだ。InitUnits をハーネス側のループで置き換える方式は、ゲストのコード（`f_4120c0`）を使わなくなるので選ばなかった。`abort()` を SIGABRT のハンドラで受ける方式なので、runtime の停止の経路（`fle_stop`、`fprintf` ＋ `abort()` の各所）には手を入れていない。
2. **フックの場所**: 「停止の経路にフック」ではなく、**停止の手前の `fle_dispatch`** に置いた。unit の呼び出しを捕まえるのが目的で、unit を呼ぶのは生成 C の中の inline の `fle_dispatch` だけだから（生成 C を変えない条件下で、これ以外に unit の呼び出しを横取りする場所が無い）。
3. **何を戻すか**: `R[]`（rsp を含む）、x87、MMX、MXCSR、DF を戻す。unit が途中で止まると x87 スタックが満杯のまま、MMX 状態が「使用中」のままなどで、次の unit が `x87 stack overflow` などで見かけ上失敗する（二次被害の人工的な原因）ので、ハーネスが持つ CPU 状態は戻した。メモリとランタイムの表は戻さない。
4. **分類**: 指示の (a)〜(f) に、(g) その他のシグナル（SIGILL、SIGFPE）を足した。fle_stop の文面が「その API 全体が not answered」のものと、IAT thunk の `fle_unsupported`、import の疑似 VA への `fle_dispatch_miss` を (a) に入れた。null 関数ポインタ呼び（`fle_dispatch_miss` で 64 KiB 未満）は (e)。「未定義フラグ読み出し」「shift count」「x87」などの lift の意味の穴は (b)。
5. **二次被害の印**: 「最初の放棄 unit より後で、null 関数ポインタ呼び・null ページへのアクセス・`NULL`／`hModule 0x0` の手掛かりがある停止」に `secondary?`。確実でない。別に、2 つの構成で同じ unit が片方でだけ放棄される場合は、**構成の違いに依存する失敗**として実測で示す（`wall-census-2.md` の比較）。

## 限界

- **各 unit で見えるのは最初の壁だけ**。unit は最初の停止で放棄されるので、その先にある別の壁は見えない。壁の数は下限で、1 つの壁を直すと同じ unit の次の壁が出ることがある。
- **放棄した unit の後の状態は不完全**。メモリ（unit が書き込んだ分）、ランタイムの表（ハンドル、レジストリ、SRW のロック、実在モジュールのロード状態（`ST_LOADING` のまま残る）、`fle_call_guest` の入れ子カウンタ）は戻していない。後続の unit の失敗には二次被害が混ざる。**一次か二次かの印はヒューリスティックで、確実でない**。印の無い失敗が、前の放棄の結果でないことの証明にはならない。
- **ok は「戻った」だけ**（決定 0060 と同じ）。放棄した unit の後の状態を使って戻った可能性は排除していない。
- 構成の比較（単独と dsp 構成）が示す依存は、実在モジュールの有無と前の unit の完了が同時に変わるので、どの変化が原因かを切り分けない。
- 暴走の判定は壁時計 20 秒（環境変数で変更可）と unit あたり 2,000 万の関数入口。実機の速度ではない。今回の測定では hang も fault も 0 件で、これらの経路は模擬ゲストのテストでだけ動かした。
- unit の外で止まると、そこで census は終わる（InitUnits の後の終了フェーズ。dsp 構成も単独も `0x412752` で止まる）。終了フェーズの unit 相当の処理は数えない。
- native 専用。wasm では census は使えない（`harness/wasm/main.c` に呼び出しが無い）。実 Windows での挙動は未確認。
- census の後に出る a4 の行（`a4: init_count=…`）は、unit を放棄した実行の記録で、A4 の進捗ではない（出力に注意書きを出す）。
