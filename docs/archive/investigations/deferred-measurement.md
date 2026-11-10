# 決定 0096 の遅延モジュールの実装と測定: 遅延構成（診断）の a4 は 159/942、次の停止は shell32!SHGetFolderPathW（未応答の import）。この数は A4 の進捗ではない（測定のみ、決めない）

実行日: 2026-10-25（JST。a4 の測定とテスト、変異検査）と 2026-10-27（JST。前回の中断後の再開。3 モジュールの回帰、全体テストのやり直し）。基準の HEAD は `0ddcbf1`（`git archive 0ddcbf1` を `lift/out-deferred/src/` に展開。`lift/` はその後の HEAD `5002d98` と同一で、`diff -rq` で確認した）。ワークツリーの決定 0095 の未コミットの編集（`fe-decode/src/jumptab.rs` ほか）には**触れていない**（この作業の差分に入らない）。作業物は `lift/out-deferred/`（gitignore 対象）。決定は `docs/decisions/0096-deferred-modules.md`（仮の決定。この文書は何も決めない）。

**この文書の a4 の数（159/942）は「遅延構成（診断）」の数であり、A4 の進捗ではない**（決定 0096 の 6、決定 0046 は変えない）。遅延モジュールは `blend2d.dll` と `quickfontcache_x64.dll` の DllMain を実行せず、メモリ像も作らないので、実機と同じ挙動とは主張できない。M1 の A4 は、両 DLL を lift して実在モジュールにしてからの数で判定する。

## 0. 凡例

- **実測** = このセッションでコマンドを実行して得た値。ログは `lift/out-deferred/` の下。
- **読み** = 逆アセンブル、ソース、別の文書を読んだ結果。実行して確かめていない。
- **未確認** = そのとおり。
- 「遅延構成（診断）」= FLEngine + 縮小 dsp_ippv2（実在モジュール 1 つ、基底 0x2000000）+ 遅延モジュール 2 つ（runtime の N = 33、34）。gdb の仮の値なし、runtime の応答追加なし（遅延モジュール以外）。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | 実装 | `fle_modules.c` +44、`fle_modules.h` +16、`fle_realmod.c` +2 −1、`fle_realmod.h` +3 −2、`imports_table.rs` +15。**`fle_load.c` は無変更**（LoadLibrary 系は実在モジュール → 仮想モジュール表の順に引くので、表に 2 名を足すだけで足りた）。新しい `.c`、`.h` なし | 実測（`diff`） |
| 2 | テスト | 新規 `deferred.rs` 15 件。既存テストの固定数を 6 ファイル 13 行で更新（§3.2）。全体: **99 binaries / 1,243 passed / 0 failed / 0 ignored**（基準 98 / 1,228 / 0 / 0。+1 binary（`deferred.rs`）、+15 件。`SKIPPED` 0 件。既存のテストは増減していない） | 実測 |
| 3 | FLEngine の生成 C の SHA-256 | `feb5b908…` → **`170bd18fe2a365de47068e85ffe1c2e1a3f04f912db7e3335f40f6eecaecca43`**（215,709,232 → 215,712,039 B）。旧 C との `diff` は応答表の 5 か所（37 行: 追加 35、変更 2）だけで、**変わった関数 0**（§4） | 実測 |
| 4 | 単独構成の a4 | **145/942**、functions 821、entries 15,685、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`。基準と一致。2 回同一 | 実測 |
| 5 | **遅延構成（診断）の a4** | init_count **159/942**、last unit 158（proc `0x66d1a0`）、started 146 / completed 145、functions 978、entries 27,134。2 回同一 | 実測 |
| 6 | 次の停止 | `fle: unsupported instruction at 0x593450`（`f_593450` = `jmp [0x1971f98]` = `shell32.dll!SHGetFolderPathW` の thunk）。分類: **未応答の import**。遅延モジュールの呼び出しではない | 実測（スタックと import）と読み（引数の意味） |
| 7 | 遅延モジュールの通り方 | unit 149 で `LoadLibraryW("Blend2D.dll")` → `0x7ff800210000`、`GetProcAddress("AggCreateInstance")` → `0x7ff9000010d0`。unit 158 で `LoadLibraryW("QuickFontCache_x64.dll")` → `0x7ff800220000`、`GetProcAddress` 5 回（`FontCache_AcquireFont`、`FontCache_CleanUp`、`FontCache_GetStats`、`AcquireSharedThreadBank`、`AcquireSharedAnimTimer`）がすべて擬似 VA。**`fle_imp_deferred_*` は 1 回も呼ばれていない**（読み取りの gdb の breakpoint 0 回） | 実測 |
| 8 | 実在モジュール優先 | 3 モジュール（FLEngine + 縮小 dsp_ippv2 + Blend2D を実在モジュール、基底 0x4700000、種 58 個）を同じ runtime で動かすと **150/942、停止 `0x489f940`（`f_489f7d0`）**。決定 0094 の測定と同一（functions 1,069、entries 27,333 も同一）。実在の Blend2D が遅延モジュールに負けていない | 実測 |
| 9 | 変異検査 | 8 種の変異がすべて 1 件以上のテストを落とした。元に戻して `cmp` で一致を確認（§3.3） | 実測 |

## 2. 実装（実測。`diff -ruN` と、ソースを読んだ事実）

- `lift/runtime/fle_modules.c`: `fle_modules[]` の末尾に `"blend2d.dll"`（N = 33、index 32）、`"quickfontcache_x64.dll"`（N = 34、index 33）。`FLE_DEFERRED_FIRST` = 32、`FLE_DEFERRED_N` = 2、`fle_module_is_deferred(index)`。`get_module_handle`: 表に当たった名前が遅延モジュールで、同名の実在モジュールが登録されていれば表の答えを返さず実在モジュールの経路（`fle_realmod_module_handle`）へ進む。11 個の `fle_imp_deferred_*`（Blend2D 1、QFC 10。停止メッセージは §2.1）。`GetProcAddress` 本体は**変えていない**（応答表の `dll` 欄がモジュール名の行を引く既存の経路で、遅延モジュールの 11 行がそのまま当たる）。
- `lift/runtime/fle_modules.h`: `fle_module_is_deferred` と 11 個の宣言。
- `lift/runtime/fle_realmod.c`: `table_check` が「実在モジュールの名前が仮想モジュールの名前なら停止」とする所で、遅延モジュールの名前は除く（除かないと、実在の `blend2d.dll` を登録したビルドが「has the name of a virtual module」で停止する）。`fle_realmod.h`: その注記。
- `lift/crates/fe-lift/src/imports_table.rs`: `TABLE` の**末尾**に 11 行（`blend2d.dll!AggCreateInstance`、`quickfontcache_x64.dll!` の 10 名を PE の export ディレクトリの序数順に）。既存の擬似 VA は動かない（決定 0015）。新しい行の擬似 VA は `0x7ff9000010d0`〜`0x7ff900001170`（行 269〜279）。
- `fle_load.c`: **変更なし**。LoadLibraryW／A／ExW は `load_by_name` が (1) 実在モジュール、(2) `fle_modules[]` の順に引くので、実在モジュールが優先され、遅延モジュールは表に入れるだけで成功し、参照カウントも仮想モジュールと同じ（`refs[]` は `fle_module_count()` で初期化される。64 以内）。`FreeLibrary` も同じ（参照 1 で名前付き停止、決定 0082 D11）。

### 2.1 停止メッセージ（実測。テストで固定）

```
fle: blend2d.dll!AggCreateInstance: the module is installed but not lifted yet (deferred, decision 0096; decision 0086 lifts it later) RCX=0x1111 RDX=0x2222 R8=0x3333 R9=0x4444
```

`<module>` は `fle_modules[]` の綴り（小文字、拡張子つき）、`<export>` は export 名そのまま（大文字小文字を保つ）。RCX、RDX、R8、R9 は `R[1]`、`R[2]`、`R[8]`、`R[9]`。引数の書式（`RCX=0x…` を空白で並べる）は決定に無く、作業者の選択（解釈が分かれた点として決定文書に記載）。

### 2.2 export の名前（実測）

- Blend2D: `lift/out-blend2d/Blend2D.dll`（SHA-256 `66634d24…66c7`）の export ディレクトリを pefile 2024.8.26 で読んだ: `AggCreateInstance`（序数 1、RVA 0x59a60）の **1 名**。フォワーダなし。
- QFC: `lift/out-qfc/quickfontcache_x64.dll`（SHA-256 `afc5a0ed…36d2`）を同様に読んだ: **10 名**、フォワーダなし。序数 1 `dbkFCallWrapperAddr`（`.bss`。コードではなく変数）、2 `__dbk_fcall_wrapper`、3 `InitSharedPython`、4 `CreateVST3Instance`、5 `LoadSharedElastiqueDll`、6 `AcquireSharedAnimTimer`、7 `AcquireSharedThreadBank`、8 `FontCache_GetStats`、9 `FontCache_CleanUp`、10 `FontCache_AcquireFont`。`lift/out-qfc/exports.txt`（`fe-pe pe-exports`）および QFC 調査 §3.3 と一致。FLEngine が引く 8 名は、このうち `dbkFCallWrapperAddr` と `__dbk_fcall_wrapper` を除く（QFC 調査 §5.1）。
- テスト `the_rows_are_the_export_directories_of_the_real_dlls` が `fe_pe` で同じ export 名・序数を読み、応答表の行と照合する（DLL が無い環境では `SKIPPED` を出して通る。この作業の実行では `BLEND2D_ORIG_PE`、`QFC_PE` を与えて実際に照合した）。

## 3. テスト

### 3.1 新規 `lift/crates/fe-lift/tests/deferred.rs`（15 件）

1. 生成器: 応答表に 11 行が既存の 268 行の直後に入る（擬似 VA = 基底 + 0x10 × 位置。既存の行の VA が動かない）、export 名が実 DLL の export ディレクトリと一致。
2. 実行時（生成器の本物の応答表をリンクしたドライバ）: 表の末尾が N = 33、34（ハンドル `0x7ff800210000`、`0x7ff800220000`）、LoadLibraryW／A／ExW（綴り・大文字小文字・拡張子・末尾の `.`・フラグ）が成功してハンドルを返し last error が 5（呼ぶ前の値）のまま、**DllMain などゲストの呼び出しが 0 回**（`fle_call_guest` を `--wrap` して数える）、ハンドルの位置に何もマップされていない（`mincore`）、参照カウント（初期 1、LoadLibrary で +1、FreeLibrary で −1、1 で停止、表の外は別の停止）、`GetModuleHandleW／A`（仮想モジュールの規則）、`GetProcAddress` が 11 名すべてで擬似 VA（一意、応答領域内）、export にない名前（大文字小文字違い・近い名前・他モジュールの名前・空文字列・DllMain）は NULL と 127、**11 個すべての擬似 VA を `fle_dispatch` で呼ぶと名前付きで停止**（stderr 全文と引数 4 つ）、ゲストの手順（LoadLibraryW → GetProcAddress → ポインタを保存 → 後で呼ぶ）、近い名前（`blend2d2.dll`、32 ビット版 `quickfontcache.dll` ほか）は従来どおりの名前付き停止、序数・表の外・ずれたハンドルは仮想モジュールと同じ停止。
3. 実在モジュールが優先: 実在の `blend2d.dll` を登録したドライバ（偽のイメージを基底にマップし、DllMain は生成した関数）で、ロード前は GetModuleHandle が NULL と 126（遅延ハンドルではない）、ロードで DllMain が 1 回走って基底を返す、GetProcAddress が実イメージの export ディレクトリを読む（遅延モジュールの行ではない）、同時に QFC は遅延モジュールのまま（DllMain なし、停止メッセージにも引数）、FreeLibrary は実在モジュールの規則（`fle_realmod_free`）。

### 3.2 既存テストの固定数の更新（決定 0088、0092 と同じ流儀。理由はすべて「表が 32 件 → 34 件になった」）

| ファイル:行（変更前の行番号） | 変更前 → 変更後 | 理由 |
|---|---|---|
| `modules.rs:214`、`:217` | `* 32` → `* 34`（コメントと不等式） | 擬似 VA がハンドル領域より上、の説明。最後のハンドルが N = 34 になる |
| `modules.rs:610`〜`:620`（`module_handles_follow_the_formula_and_are_distinct`） | `count 32` → `count 34`、`lines[1..33]` → `[1..35]`、末尾の名前の検査に 33、34 番目（`blend2d.dll`、`quickfontcache_x64.dll`）を足し、表の終わりを `lines[33]` → `lines[35]` に | 表が 34 件になる |
| `modules.rs:767`（`undefined_cases_abort` の `handle-past-table`） | `h(32)` → `h(34)` | このファイルの `h(i)` は 0 始まりの index。表の外の最初は index 34（N = 35） |
| `msctf.rs:258` | `fle_module_name(32)` → `(34)` | 0 始まり。表の外が NULL の期待 |
| `msctf.rs:282`、`:455` | シナリオ名と `M(33)` → `handle-35`、`M(35)` | `M(n)` は N（1 始まり）。N = 33 は表の中になったので、表の外の最初（N = 35） |
| `msctf.rs:355` | `count 32` → `count 34` | 表が 34 件になる |
| `load.rs:446`、`:628` | `fle_load_refcount(32)` → `(34)` | 表の外の参照数 0 の期待 |
| `load.rs:657` | `fle_module_handle(32)` → `(34)` | 表の外の最初のハンドルで停止、の期待 |
| `event.rs:261`、`glob.rs:205`、`vol.rs:41`、`vfs.rs:33` | `mods_end = … * 32` → `* 34` | 他のトークン領域がモジュールのハンドルより上にある、という不等式の端を実態に合わせた（`* 32` のままでも成り立つ。算術） |

「応答表の行数」「擬似 VA の位置」「`GetModuleHandleA` が最後の行でない」を固定するテストは、決定 0092 のときに既に「総数を固定しない」形に直されていたので、今回は変更が要らなかった（`modules.rs` の `st.resp_entries >= 13`、`srw.rs` の位置検査など）。新規の `deferred.rs` は**行数の総数は固定せず**（`>= 279`）、既存の 268 行の位置と新しい 11 行の位置だけを固定する。他の決定が行を末尾に足しても影響しない。ただし**先に別の行が末尾に足された後でこの変更が入ると、`ROWS_BEFORE`（268）と遅延モジュールの擬似 VA が動く**ので、その場合は `deferred.rs` の `ROWS_BEFORE` を直す。

### 3.3 変異検査（実施。自己検査で、決定の根拠ではない。`mut/run-mut.sh N`。元に戻して `cmp` で一致を確認）

| # | 変異 | 結果 |
|---|---|---|
| 1 | GetModuleHandle: 実在モジュールがあっても遅延モジュールが勝つ | 1 件失敗 |
| 2 | 実在モジュールが遅延モジュールの名前を持つとテーブル検査が停止 | 4 件失敗 |
| 3 | 停止メッセージの R8 と R9 の値を入れ替える | 2 件失敗 |
| 4 | QFC の export の行を 1 つ欠く | 5 件失敗 |
| 5 | Blend2D の行が QFC の停止関数を呼ぶ | 2 件失敗 |
| 6 | 遅延モジュール 2 名の並びを入れ替える（N = 33、34 の入れ替え） | 11 件失敗 |
| 7 | 遅延モジュールの export の呼び出しが停止せず 0 を返す | 2 件失敗 |
| 8 | `fle_load.c` が遅延モジュールの名前を表で見つけない | 6 件失敗 |

（失敗件数は `mut/mut1.txt`〜`mut8.txt`。取り直しの記録: (1) 変異 3 は最初の実行で置換の文字列が一致せず適用されなかった（「15 passed」）ので、スクリプトに「適用されなかったら結果なし」の検査を足して取り直した。(2) 最初の一巡では元に戻すのに `cp -p`（更新時刻を保つ）を使ったため、Rust のライブラリ側の変異（4、5。`imports_table.rs`）の後の実行が、cargo の再ビルド判定（更新時刻）をすり抜けて古い変異つきのライブラリを使った（変異 6〜8 の最初の結果に変異 5 の失敗が混ざったことで気づいた）。`cp` に直して更新時刻を新しくし、6〜8 と 4、5 を取り直した。表は取り直し後の値。runtime の C はテストが実行のたびに `clang` でコンパイルするので、この問題の影響を受けない。(3) 変異検査は `deferred.rs` の最後の編集（総行数を固定しない形）より前に実施した。）

## 4. FLEngine の生成 C（実測）

`fe-lift lift FLEngine_x64.dll gen/fle-new.c --pdata`（release、`lift/out-deferred/target/release`）:

- SHA-256 **`170bd18fe2a365de47068e85ffe1c2e1a3f04f912db7e3335f40f6eecaecca43`**。旧 C は `lift/out-rdefs/fle.c`（SHA `feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f`。基準と一致）。215,709,232 → 215,712,039 B（+2,807）。
- `diff 旧 新`: ハンクは 5 つ（`59576a59577,59587`、`119143a119155,119165`、`119145c119167`、`119415a119438,119448`、`119417c119450`）。内容は (1) `fle_imp_deferred_*` のプロトタイプ 11 行、(2) `fle_dispatch_table` の末尾に 11 行、(3) `fle_dispatch_count` 59565 → 59576、(4) `fle_resp_table` の末尾に 11 行、(5) `fle_resp_count` 268 → 279。追加 35 行、変更 2 行。**関数の本体の差は 0**（`void f_…` の定義は 1 つも変わっていない）。
- 統計は変わらない: functions 59,297、bad_nodes 0、closure_instructions 2,528,030、`unsupported_total` 525（`gen/fle-new.stats.txt`）。
- 今後の基準の SHA は `170bd18f…`（この変更が入った後）。

## 5. 単独構成の a4（実測。`a4single/`）

runtime のオブジェクト（`fle_modules.o` を含む）を新しい OUT で最初から作り直し、生成 C は上の 17 分割。a4-survey を 2 回:

```
depth  : init_count=145 / 942   last unit: index=144 run_rank=132/895 proc=0x65c540 in_progress
         started=132 completed=131 in_progress=1 of 895 runnable   all_units_done=no
funcs  : executed (distinct)=821  entries=15685
stop   : fle: LoadLibraryW: module "dsp_ippv2_x64.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)
```

基準（145/942、functions 821、entries 15,685、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`）と**一致**。2 回の出力は時刻の行以外同一。単独構成は unit 144 で止まるので、遅延モジュールは効かない。リンクした runtime のオブジェクトが現在のソースのオブジェクトと**バイト一致**することを、同じ cwd・同じフラグで再コンパイルして `cmp` で確認した（`tmp/objcheck.sh`。`fle_modules.o`、`fle_realmod.o`、`fle_load.o`、`fle_env.o`、`fle_sys.o`、`fle_imports.o`）。

## 6. 遅延構成（診断）の a4（実測。`deferred-cfg/`）

### 6.1 手順

- 生成: `lift/out-deferred/target/release` の `fe-lift`（決定 0096 の変更つき）。FLEngine の C は上の `gen/fle-new.c`（SHA `170bd18f…`）。縮小 dsp_ippv2（基底 0x2000000、種 452 個）は `lift/out-rdefs/three/dll/` の読み取り専用のコピー（SHA-256 `4fd25502…`、種ファイルも同じ）。生成した C は functions 8,030、closure 3,038,921、bad_nodes 0（決定 0094 の測定と同じ値）。`gen_two.sh`、`build_two_native.sh`、`main_two.c` ほかのスクリプトは `lift/out-rdefs/three/harness-two/` の無変更のコピー（`diff -rq` で差なし）。`run-gen.sh`、`run-build.sh`、`run-a4.sh` は決定 0094 の `out-rdefs/three/run-*.sh` に倣って書いた。
- **Blend2D は実在モジュールにしない**（`realmod_table_gen.c` は `dsp_ippv2_x64.dll` の 1 件だけ）。two-native 571,262,944 B。ビルドの stderr 0 行。
- a4-survey を 2 回（`out-1/`、`out-2/`）。読み取りだけの gdb を 2 本（`gdb_ro_imports.py`: ローダー系の呼び出しとその戻り値、`fle_imp_deferred_*` の breakpoint。`gdb_ro_stop.py`: 停止時のゲストのレジスタ）。`return`、`set var`、`call`、値の書き込みは**使っていない**。

### 6.2 a4 の行（2 回。実測）

```
A4 survey 20261025T131031Z   dll: …/binaries/win64/FLEngine_x64.dll   harness exit: 134
static : units=942 runnable=895 empty=46 noproc=1 (header 0x13768b0, units 0x13768e0)
stop   : fle: unsupported instruction at 0x593450
         top of stack: 0x593450 f_593450
depth  : init_count=159 / 942   last unit: index=158 run_rank=146/895 proc=0x66d1a0 in_progress
         started=146 completed=145 in_progress=1 of 895 runnable   all_units_done=no
         entry check: started_but_not_entered=0 runnable_entered_beyond_init_count=0
funcs  : executed (distinct)=978  entries=27134
```

2 回目（`…T131038Z`）は時刻の行以外同一。**前回までの値（実在モジュール 1 つの構成、決定 0088 の測定: 150/942、停止 `LoadLibraryW "blend2d.dll"`）から 150 → 159**。

### 6.3 次の停止（実測と読み）

- **メッセージ全文**（stderr 1 行）: `fle: unsupported instruction at 0x593450`（`abort kind: fle_unsupported`、`call depth at stop: 9`）。
- **C のスタック**（ハーネスの報告）:

```
#0 0x593450 f_593450 guest-return=0x81bcc5
#1 0x81bca0 f_81bca0 guest-return=0x6690a0
#2 0x668d30 f_668d30 guest-return=0x66d08f
#3 0x66d050 f_66d050 guest-return=0x66d1be
#4 0x66d1a0 f_66d1a0 guest-return=0x412129
#5 0x4120c0 f_4120c0 guest-return=0x41229a
#6 0x412160 f_412160 guest-return=0x41d989
#7 0x41d8f0 f_41d8f0 guest-return=0x1376875
#8 0x1376820 f_1376820 guest-return=0xdead0000
```

- `f_593450` は `ff 25 42 eb 3d 01` = `jmp [rip+0x13deb42]`（IAT スロット `0x1971f98`）。PE の import 表（pefile）でそのスロットは **`shell32.dll!SHGetFolderPathW`**。応答表に行がないので生成器は thunk を「未対応」として生成し、実行時に `fle_unsupported`。
- 停止時のゲストのレジスタ（読み取りの gdb）: `rcx = 0`、`rdx = 0x14`、`r8 = 0`、`r9 = 0`、第 5 引数（`[rsp+0x28]`）= `0x7ffff7bff2d6`（呼び元 `f_81bca0` の `lea rax,[rbp+0x36]; mov [rsp+0x20],rax` で作られる、呼び元のスタック上のバッファの先頭。バッファの大きさは読んでいない）。つまり `SHGetFolderPathW(hwnd = NULL, csidl = 0x14, hToken = NULL, dwFlags = 0, pszPath = buf)`。`0x14` は mingw の `shlobj.h` の `CSIDL_FONTS`（`#define CSIDL_FONTS 0x0014`）。Learn の CSIDL のページ（取得した）は `CSIDL_FONTS` を「フォントを含む仮想フォルダ。典型的なパスは `C:\Windows\Fonts`」と書くが数値は載せない（数値は mingw のヘッダによる）。
- 呼び元: `f_668d30` は QFC 調査 §5.1 が「フォント設定の関数。`GetProcAddress`、`LoadLibrary` を呼ばない」と読んだ関数で、`f_66d050`（unit 158 の起動）から呼ばれる。`f_66d050` は `f_665bd0`（QFC のロードと 3 名の取得）、`f_66cfa0`（`AcquireSharedThreadBank`、`AcquireSharedAnimTimer` の取得）の後に `f_668d30` を呼ぶ（QFC 調査 §5.2。読み）。この実行でも、gdb のログの順序（§6.4）はそのとおり。
- **停止点の分類**: **未応答の import**（行なし import `shell32.dll!SHGetFolderPathW`）。未対応命令でも、遅延モジュールの呼び出しでも、レジストリ・ロケールでもない。**直していない**（範囲外。応答は `CSIDL_FONTS` の値を runtime が決める必要があり、`docs/investigations/remaining-unsupported.md` の A11（「仮想環境が何を主張するか」の判断の束。`SHGetFolderPathW` を挙げる）の類い）。

### 6.4 ローダー系の呼び出し（読み取りの gdb。`run/ro1-loader.txt`。実測）

unit 149 と 158 の部分（通し番号は gdb の breakpoint の通し番号）:

```
  525 fle_imp_LoadLibraryW  name='SHCore.dll'                         -> R[0]=0x0       （unit 145 の absent）
  526 fle_imp_LoadLibraryW  name='Blend2D.dll'                        -> R[0]=0x7ff800210000
  527 fle_imp_GetProcAddress rcx=0x7ff800210000 proc='AggCreateInstance'  -> R[0]=0x7ff9000010d0
  528 fle_imp_LoadLibraryW  name='QuickFontCache_x64.dll'             -> R[0]=0x7ff800220000
  529 fle_imp_GetProcAddress rcx=0x7ff800220000 proc='FontCache_AcquireFont'     -> R[0]=0x7ff900001170
  530 …                                              proc='FontCache_CleanUp'        -> R[0]=0x7ff900001160
  531 …                                              proc='FontCache_GetStats'       -> R[0]=0x7ff900001150
  532 …                                              proc='AcquireSharedThreadBank'  -> R[0]=0x7ff900001140
  533 …                                              proc='AcquireSharedAnimTimer'   -> R[0]=0x7ff900001130
```

- `fle_imp_LoadLibraryW` は 21 回。`fle_imp_deferred_*` の breakpoint は **0 回**（`grep` で確認。停止まで遅延モジュールの export は 1 つも呼ばれていない）。
- 保存先に入る値は、Blend2D の `0x15f45f0`（`AggCreateInstance`）、QFC の `0x15f4628`、`0x15f4630`、`0x15f4638`、`0x15f4648`、`0x15f4658` の各スロット（QFC 調査 §5.1 の表。保存先の値そのものは今回読んでいない: **未確認**）。ゲストはこれらの擬似 VA を呼ぶ前に停止した。

### 6.5 unit 150 以降に入ったことの意味（読み）

unit 149 の Blend2D の `LoadLibraryW` / `GetProcAddress` と、unit 153 の `RegCreateKeyExW`（決定 0085）、unit 158 の QFC の `LoadLibraryW` / `GetProcAddress` を通り、unit 158 の proc `f_66d1a0` の途中（フォント設定）で次の行なし import に届いた。これは「ゲストが両 DLL をロードしただけで、まだ呼んでいない」状態でのことで、描画経路などで呼ぶ段階（Blend2D の静的な読みで unit 614 以降、QFC の `FontCache_AcquireFont` の使い手は unit 245 以降）には届いていない（読み。QFC 調査 §5.3、Blend2D 調査 §4.2）。**測定が unit 245 に届いたとき、`FontCache_AcquireFont` の擬似 VA を呼ぶ停止が最初の遅延モジュールの停止として現れる可能性が、静的な読みにある**（未確認）。

## 7. 実在モジュール優先の回帰（3 モジュール。実測。`deferred-cfg/out-three-1`、`out-three-2`）

決定 0094 の 3 モジュール構成（FLEngine + 縮小 dsp_ippv2 基底 0x2000000 + **Blend2D を実在モジュール**、基底 0x4700000、種 58 個）を、**決定 0096 の runtime と生成器**で再現した（`run-gen3.sh`、`run-build3.sh`、`run-a4-three.sh`。スクリプトは §6.1 と同じ無変更のコピー。リベース済み DLL の SHA-256 は `24e01ec9…`（Blend2D）、`4fd25502…`（dsp_ippv2）で決定 0094 のものと一致）。`realmod_table_gen.c` は `dsp_ippv2_x64.dll` と `blend2d.dll` の 2 件。

```
stop   : fle: indirect call to 0x489f940 is not a known function entry
         top of stack: 0x489f7d0 f_489f7d0
depth  : init_count=150 / 942   last unit: index=149 run_rank=137/895 proc=0x664260 in_progress
         started=137 completed=136 in_progress=1 of 895 runnable   all_units_done=no
funcs  : executed (distinct)=1069  entries=27333
```

決定 0094 の検証の表の値（150/942、停止 `0x489f940`、functions 1,069、entries 27,333）と**一致**。2 回同一。実在の `blend2d.dll` は LoadLibraryW で実在モジュールとして扱われ（`fle_realmod_table` が `blend2d.dll` を持つ）、遅延モジュール（N = 33）には回らなかった。

## 8. 未確認、限界

- **遅延構成の数は A4 の進捗ではない**（上のとおり）。DllMain の副作用（Blend2D と QFC の初期化）が省かれる。ゲストが両 DLL のメモリを直接読む経路があれば、決定 0096 の撤回条件 (a)。それを調べていない。
- 停止したのは `SHGetFolderPathW` で、**遅延モジュールの export の呼び出しには 1 回も届いていない**。停止メッセージ（`fle: <module>!<export>: …`）の実際の発火は、単体テストと合成ドライバで確認しただけで、FLEngine の実行では未確認。
- unit 159 以降は未測定（次の停止を直していない）。
- 実機の Windows での挙動は未確認（ホストは Linux）。`LoadLibraryW` が実機で成功するか（撤回条件 (c)）、DllMain を実行しないことの影響は未確認。
- `GetModuleHandleW／A` は、遅延モジュールを LoadLibrary 前から「ある」と答える（仮想モジュールと同じ規則。実機では未ロードの DLL は見つからない）。
- `GetModuleFileNameW`（遅延モジュールのハンドル）は `fle_vfs.c` が「NULL か 0x400000 でなければ停止」とする既存の挙動のまま（名前付き停止。読み）。`LoadResource` 系は `fle_res.c` の `check_module` が「module blend2d.dll has no resources in this runtime」で停止する（読み）。どちらも今回の測定では届いていない。
- QFC の `dbkFCallWrapperAddr` は `.bss` の変数だが、決定の「export 全部」に従って行を作った。`GetProcAddress` が返す擬似 VA はコードではなく、呼ぶと停止し、読むと SEGV になる。FLEngine は引かない（QFC 調査 §1 の 9）。
- 母集団は 2 つの DLL に限る（同梱の他の DLL 11 名は扱っていない。決定 0086）。wasm、emcc は使っていない。
- 全体テストは、並行して他の作業の全体テストが走っていた間に 1 回タイムアウトした（`test-ws2.txt`: 99 本中 70 本目で打ち切り。他の作業のテストの abort による apport のコアダンプ処理が重なって遅くなっていたことを `ps` で見た）。取り直しは自分のシェルで `ulimit -c 0` としてコアダンプを出さない設定にした（テストの内容・結果には関係しない）。

## 9. 作ったファイル

- 追跡対象にするもの: `docs/decisions/0096-deferred-modules.md`、`docs/investigations/deferred-measurement.md`（この文書）、`docs/README.md` の 0096 の行（ワークツリーの README は編集していない）、`lift/out-deferred/patches/deferred.diff`（`lift/runtime/fle_modules.{c,h}`、`fle_realmod.{c,h}`、`lift/crates/fe-lift/src/imports_table.rs`、`lift/crates/fe-lift/tests/{deferred,modules,msctf,load,event,glob,vfs,vol}.rs`）。
- 作業物（`lift/out-deferred/`）: `src/`（スナップショット）、`base0/`（`git archive 0ddcbf1`）、`base/`（その後の HEAD の `git archive`）、`verify/`（`git archive 0ddcbf1` に `patches/deferred.diff` を `patch -p1` で当てた結果。`src/` と全ファイルが一致）、`patches/`（`deferred.diff` 1,188 行、`patch -p1 --dry-run` と適用のログ）、`gen/`（FLEngine の新旧 C の diff）、`a4single/`、`deferred-cfg/`（遅延構成と 3 モジュールの回帰。スクリプト、ログ、gdb）、`mut/`（変異検査）、`test-ws.txt`、`test-ws2.txt`（打ち切り）、`test-ws3.txt`、`tmp/`、`target/`。
