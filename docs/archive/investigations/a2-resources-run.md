# リソース API（A2）の追加後の実行（896 seed の閉包）

日付: 2026-10-20。決定は `docs/decisions/0025-resource-apis.md`。出力はすべて `lift/out-a2/`（`tmp/` は TMPDIR）。前回の記録は `docs/investigations/int-more2-run.md`（停止点 0x407030、`unsupported_total` 502）。

## 1. 対象と実装

- 対象（`remaining-unsupported.md` §5.1 の A2）: kernel32 の FindResourceW、LoadResource、LockResource、SizeofResource と user32 の LoadStringW。**5 件**。
- 新規: `lift/runtime/fle_res.c`、`fle_res.h`、`lift/crates/fe-pe/tests/resources.rs`、`lift/crates/fe-lift/tests/res.rs`、`docs/decisions/0025-resource-apis.md`、この文書。
- 変更: `lift/crates/fe-pe/src/lib.rs`（`.rsrc` ディレクトリのパーサを追加。`dir::RESOURCE`、`ResourceId`、`ResourceLeaf`、`StringEntry`、`Pe::resources`、`Pe::resource_data`、`Pe::strings`、`parse_string_block`、`RT_STRING`。既存のコードは編集していない）、`lift/crates/fe-lift/src/imports_table.rs`（`TABLE` の末尾に 5 行を追記。編集の直前に読み直し、末尾は `GetDateFormatW` だった）。`fe-lift/src/lib.rs` は触っていない。
- `fle_res.h`、`fle_res.c` は `lift/out-a2/work/` で作り、`FLE_RES_DIR` でテストの include とソースの場所を work に向けて単体テストを通してから、`lift/runtime/` に **ヘッダ、.c の順で 1 回**置いた（`cmp` で work と同一であることを確認）。
- API の ABI と動作は Microsoft Learn のページで確認した（決定 0025 に URL と文面）。依頼文の LoadStringW の引数の読みはページと一致した。FindResourceW は名前が型より先。

## 2. テスト

- `cargo test --offline -p fe-pe`: `real_dll` 9 件、`resources` 9 件、`synthetic` 19 件（他の test バイナリは 0 件）、すべて通過。
- `cargo test --offline -p fe-lift --test res`: **19 件**、すべて通過（`FLE_RES_DIR` なし、`lift/runtime/` のファイルで）。`imports_table`、`modules` も通過（後者は全 `fle_*.c` をリンクするので、`fle_res.c` が他の runtime とリンクできることの確認）。
- 実 DLL を使うテストは、`FLENGINE_PE`（なければ `binaries/win64/FLEngine_x64.dll`）が無ければ SKIPPED で通る（fe-pe の `real_dll.rs` と同じ流儀）。今回の実行では DLL があり、すべて実行された。
- テストの中身の要点: 全 65536 の文字列 ID を `LoadStringW` で引き、fe-pe の 625 文字列と一致（残りは 0 を返し、バッファを書かず、エラー 1814）。327 リーフすべてを Find/Load/Lock/Sizeof で引き、fe-pe のサイズ、データ RVA、FNV-1a と一致。abort する入力 29 通りと合成イメージの異常を確認。
- 1 回目の実行で失敗が出た（修正して再実行済み）: ①テストの C の `A()` マクロが複合リテラルのカンマで引数を割った → 可変引数マクロに変更。②テストの期待値の誤り（ID 65535 は未使用ではなかった。DLL は ID 65535 に "File access denied" を持つ。未使用の ID 64898 に変更）。③短いブロックのテストが ID 5 を引いていて、ブロック 1 が存在せず abort に至らなかった → ID 15 に変更。④リーフ表の行を追記する前は `the_five_thunks_call_the_runtime` が当然失敗（表への追記後に通過）。runtime のコードを直した失敗は無い。
- `cargo test --offline --workspace --no-fail-fast`（`test-ws.txt`、`test-ws.exit` は EXIT=0）: **46 バイナリ、339 件 passed、0 failed、0 ignored**（`test result` 行の集計）。今回追加した分は 28 件（fe-pe 9、fe-lift 19）。前回（int-more2-run.md）の 266 件との差 +73 のうち、28 件が私の分で、残りは同時に作業していた他の担当のテスト（私は確認していない）。

## 3. 生成、clang、リンク

- 生成器: `out-a2/tool`（`out-int2/tool` の複製、`Cargo.toml` の説明の `out-uilang` を `out-a2` に置換）を `out-a2/target-tool` にビルド（`build-tool.txt`、EXIT=0）。seed は `out-a2/seeds-896.txt`（896 件、`out-int2` の複製）。
- 生成: EXIT=0（`gen-a2-896.txt`、`.err` は空）。`out/after-a2-896.c`（42,537,358 バイト）。functions 8427、closure_instructions 497367、emitted_instructions 497527（前回と同じ）、c_lines 548457（前回 548442）。`fle_resp_count` は 116、`fle_dispatch_count` は 8543。
- 5 つの API は `jmp_imp:FindResourceW`、`LoadResource`、`LoadStringW`、`LockResource`、`SizeofResource` が各 1 件（閉包に各 1 か所のサンク）。
- **`unsupported_total`: 502 → 380**（−122）。`Jmp: indirect through IAT slot (not lifted)` は 304 → 299（**−5、これが A2 の分**）。残りの −117 は、同時に作業していた他の担当の作業（ワークツリーの `sse_int2.rs`、`sse_shuf.rs`、`mmx.rs` の追加。`fle_sse.h` の変更。`Packssdw`、`Shufps`、`Psllw`、`Pshuflw` などの SSE/MMX の件数が減っている。`gen-a2-896.txt` と `../out-int2/gen-int2-896.txt` の「unsupported」の一覧の diff）で、私の作業の結果ではない。`unsupported` の種類は 39 → 21。
- clang -O0 とリンク: `out-a2/build.sh`（`out-int2/build.sh` の複製）、`EXTRA_RT="fle_int_more fle_sse_more fle_locale fle_sys fle_x87ctl fle_vfs fle_reg fle_env fle_uilang fle_proc fle_time fle_uipref fle_res"`（int-more2-run.md §4 の指定に `fle_res` を足した）、EXIT=0（`build-896.txt`）。`bin/flengine-after-a2-896`。

## 4. 実行

コマンド: `./bin/flengine-after-a2-896 ../../binaries/win64/FLEngine_x64.dll`（`out-a2/` で、TMPDIR は `out-a2/tmp`）。出力 `run-a2-896.txt`、`.err`、`.exit`（EXIT=134、SIGABRT）。

| 項目 | 前回（int-more2-run.md） | 今回 |
|---|---|---|
| 停止点 | 0x407030（f_407030、`jmp [IAT]` = LoadStringW） | **0x410320**（f_410320、間接呼び出しの先 **0x40f960**） |
| abort の種類 | fle_unsupported | fle_dispatch_miss（`fle: indirect call to 0x40f960 is not a known function entry`） |
| 関数実行数（distinct） | 116 | 125 |
| 関数エントリ数（calls） | 340 | 658 |
| 呼び出し深さ | 9 | 8 |

- **0x407030 は通過した**。根拠（gdb、`gdb-count.txt`）: `fle_imp_LoadStringW` が **26 回**呼ばれ、26 回とも 4096 文字のバッファ、`hInstance` = 0x400000、uID は 65456〜65519 の範囲（`FindResourceW`、`LoadResource` の呼び出しは 0 回）。26 個の uID はすべて DLL に実在する（Python パーサ）。1 回目（uID 65462）はバッファの先頭が "Method called on disposed object"（`gdb-ls2.cmds` の出力で先頭 12 文字を確認、戻り値 32 は Python パーサの長さ 32 と一致）。
- 呼び出し元（f_41c820）は LoadStringW の戻りの後に先へ進み、その後 f_410320 に至った。

## 5. 次の停止点

- VA **0x410320**（f_410320。命令は `call QWORD PTR [rax-0x30]`、0x41032e）。実行時の `rcx` = 0x42f698。そのオブジェクトの「vtable」の [rcx-0x30] = 0x42f668 の値が **0x40f960**（ログの `dispatch target: 0x40f960`、gdb で `x/8gx` により確認）。
- スタック（実行ログ）: `#0 0x410320`、`#1 0x448ee0`、`#2 0x44a3e0`、`#3 0x44a570`、`#4 0x4120c0`、`#5 0x412160`、`#6 0x41d8f0`、`#7 0x1376820`（エントリ）。
- 種類: **API でも x86 命令でもなく、間接呼び出しの先が関数の入口として生成されていない**（`fle_dispatch_table` に 0x40f960 が無い。`grep` で確認。同じ表から 0x410060、0x410090、0x449380、0x449150、0x40f9b0〜0x40f9d0 も無い）。0x40f960 の中身は `push rbx; sub rsp,0x20; mov rbx,rcx; movsxd rcx,DWORD PTR [rbx-0x80]; call 0x409410; ...`（`objdump-text.txt` 21363 行）で、DLL 内のコードである。seed の一覧（`seeds-896.txt`）にも 0x40f960 は無い。
- 推測（確認していない）: 0x42f668 付近は C++ の vtable（0x410060、0x410090 などが隣に並ぶ）で、その vtable の要素が seed に入っておらず、静的な閉包の間接分岐の解決（fe-decode の dataflow）でも拾えていない。DLL 内には 0x40f960 を要素に持つ 8 バイトの値が .text 内に多数ある（Python で数えた）ので、同じ形の vtable が多数あり、seed の追加は 1 件では済まない可能性が高い。今回は範囲外なので実装していない。

## 6. 残り

- A2 以外は手を付けていない。LoadStringW 以外の資源の使い方（LoadIconW、LoadCursorW、LoadBitmapW、FormatMessageW、LoadLibrary 系、FreeLibrary）は A2 の範囲外で、未実装。
- 実 Windows での挙動は未確認（決定 0025 の「確認できていないこと」）。

## 7. 失敗・拒否された操作

- 拒否はなし。
- 失敗: §2 の 1 回目のテスト失敗（テストコードと期待値の誤り。runtime のコードの失敗ではない）。
- 外部への通信は、Microsoft Learn のページの取得（`learn.microsoft.com`）のみ。
- 触ったファイル（ワークツリー、`lift/out-a2/` の生成物は除く）: `lift/crates/fe-pe/src/lib.rs`（M）、`lift/crates/fe-pe/tests/resources.rs`（新規）、`lift/crates/fe-lift/src/imports_table.rs`（M、5 行追記）、`lift/crates/fe-lift/tests/res.rs`（新規）、`lift/runtime/fle_res.c`、`lift/runtime/fle_res.h`（新規）、`docs/decisions/0025-resource-apis.md`、`docs/investigations/a2-resources-run.md`（新規）。`lift/out-a2/` 全体（work、tool、build.sh、seeds、out、bin、各ログ）は新規。
