# ライブラリロード系（A6）と ImageNtHeader（A12）の追加後の実行（896 seed の閉包）

日付: 2026-10-20。決定は `docs/decisions/0029-apis-a6-a12.md`。出力はすべて `lift/out-a6/`（`tmp/` は TMPDIR、`work/` は新規 runtime の作業場と計測の道具）。比較の相手は `docs/investigations/a2-resources-run.md`（`out-a2`。停止点 0x410320、`unsupported_total` 380）と `sse-shuf-run.md`（385）。

**この計測は、作業ツリーに他の担当の未コミットの変更が混ざった状態で行った**（git の stash、checkout は禁止なので、自分の変更だけを HEAD b464d45 に重ねた状態は作れていない）。混在の内容は §5。

## 1. 実装した API と、止めた入力

- 実装: kernel32 の LoadLibraryA、LoadLibraryW、LoadLibraryExW、FreeLibrary、SetDllDirectoryW（5 API）と imagehlp の ImageNtHeader（1 API）。計 **6 API**。閉包の thunk は 8 本（FreeLibrary 3、LoadLibraryA 1、LoadLibraryW 1、LoadLibraryExW 1、SetDllDirectoryW 1、ImageNtHeader 1。`gen-a6-896.txt` の `handled` の行）。
- 未実装として残した API: なし（§5.1 の A6、A12 の一覧は全部入れた。GetModuleFileNameW は決定 0018 で実装済みなので対象外）。
- 応答の方針（決定 0029）: 決定 0015 の仮想モジュール表にある名前は仮想ハンドルと参照 +1。同じ決定が「ロードされていない」とした 9 DLL（dwmapi、imm32、msimg32、propsys、rpcrt4、shcore、uxtheme、windowscodecs、wtsapi32）は NULL + 126。**それ以外は `fle_stop`（名前付き abort）**。外部のプラグイン、道具の DLL と、決定が名指ししない Windows DLL（avrt、powrprof、psapi、riched20、urlmon ほか）には、ハンドルも失敗も作らない。
- 止める入力の主なもの: NULL・空・パス付き・256 文字超の名前、LoadLibraryExW の hFile ≠ NULL と未モデル化のフラグ、参照 1 の FreeLibrary、FreeLibrary の NULL・0x400000・表にないハンドル、ImageNtHeader の 0x400000 以外の Base。
- 公式で決まらず、決定 0029 の判断に回したもの（要点）: 126 を返す名前の範囲（0015 の 9 DLL に限る）、参照カウントの初期値 1（FreeLibrary のページ）と 0 になる呼び出しの扱い、SetDllDirectoryW の検索結果への影響なし、ImageNtHeader の NULL 返却を作らないこと。

## 2. ゲストが名指しする DLL の実測

- 呼び出し 46 か所（LoadLibraryW 44、LoadLibraryA 1、LoadLibraryExW 1）の第 1 引数を、逆アセンブルと DLL のバイト列から実測（`out-a6/work/callsites2.py` → `callsites.tsv`）。リテラルで読める 37 か所の内訳: SHFolder.dll 21、kernel32.dll 2、shell32.dll 2、Shell32 1、User32.dll 1、wininet.dll 1、advapi32 1（以上は表にある）、dwmapi 1、SHCore.dll 1（9 DLL。126）、avrt、PowrProf.dll、PSAPI.dll、RICHED20.DLL、URLMON.DLL、WebView2Loader.dll 各 1（止める）。残り 9 か所は直前の `mov rcx,rax` で、第 1 引数の中身は確定していない。
- 全文字列の走査（`ext_modules.py` → `ext_runs.tsv`、164 行）: 外部 DLL の名前（dsp_ippv2.dll、stemextract.dll、declipper.dll、beat_tracker.dll、WavpackDLL_x64.dll ほか）が DLL 内にあることは確認した。依頼文が例に挙げた `ILTools_x64.dll` は、1 つの文字列としては見つからなかった（`ILTools` と `_x64.dll` が別々にある。組み立てる形は推測）。これらが LoadLibrary のどの呼び出しに渡るかは、この作業では特定していない。

## 3. テスト

- 新規 `lift/crates/fe-lift/tests/load.rs`: **18 件、すべて通過**（`test-load-2.txt`）。内訳: 生成器 3（thunk が runtime を呼ぶ、同名の別 DLL は別 API、応答表に行がある）、LoadLibraryW/A/ExW の戻り値と last error 5 件（名前の規則、126 になる 9 DLL、ANSI 版、ExW のフラグ、参照カウントと FreeLibrary）、止まる入力 4 群（FreeLibrary、LoadLibrary の未定義入力 23 通り、外部・未名指しの 14 名とメッセージ中の名前、SetDllDirectoryW の長すぎるパス）、SetDllDirectoryW の格納・置き換え・NULL による復帰 2 件、ImageNtHeader 3 件（実 DLL を `harness/image.c` で 0x400000 に載せ、返り値が Base + e_lfanew で、指す先が `PE\0\0`、Machine 0x8664、Magic 0x20b。合成イメージの e_lfanew 0x40、0x80、0xFFF。ヘッダ異常 5 通りと Base 5 通りの停止）、`GetProcAddress` → `fle_dispatch` で LoadLibraryW に届く 1 件。応答表の行数は固定していない（`>=` も使っていない。行の有無だけを見る）。
- 期待値の出所: Microsoft Learn のページ（決定 0029 の URL）と決定 0015 のハンドル式。実 DLL のテストは e_lfanew を Rust 側でファイルから読む。`fle_load.c` を読み返して期待値にした箇所はない。
- 途中の失敗 1 件: `set_dll_directory_stores_replaces_and_restores` が、テストの期待値の誤り（"D:\Other\x" の 4 文字目の文字を 0x5c と書いたが、正しくは `d[3]` = 'O' = 0x4f。`d[2]` が `\`）で 1 回失敗。テスト側を直して通過（runtime は変えていない）。
- 作業場でのテスト: `FLE_A6_DIR` なしでも、`runtime/` に無ければ `out-a6/work/` を使う。`.h`、`.c` は work で `clang -Wall -Wextra -Werror` を通してから、`lift/runtime/` に **ヘッダ 2 つ、.c 2 つの順で 1 回**置いた（`cmp` で work と同一）。
- ワークスペース全体（`cargo test --offline --workspace --no-fail-fast`、TMPDIR は `out-a6/tmp`）:
  - 1 回目（`test-ws.txt`）: 900000 ms の上限で打ち切られた（39 バイナリが終わった時点。失敗なし。同時に他の担当も同じ種類の実行をしており、CPU を競合した。**原因は推測**。各バイナリは clang を何十回も起動する）。`test-ws-timedout-partial.txt` は同じ内容の複製。
  - 2 回目（`test-ws2.txt`、上限 3600000 ms）: 54 バイナリ、488 件 passed、**1 件 failed**: `fe-lift/tests/glob.rs` の `rows_are_in_the_table_and_declared_in_the_header`（`GlobalAlloc` の行が `imports_table.rs` に 0 件）。A3 の担当の行が、このテストの実行より後に追記されたため（`imports_table.rs` に今は `GlobalAlloc` の行があり、そのテストだけ再実行して通過、`test-glob-recheck.txt`）。私の変更が原因ではない（これは推測ではなく、再実行で確認）。
  - 3 回目（`test-ws3.txt`、`test-ws3.exit` は EXIT=0）: **55 バイナリ、502 件 passed、0 failed、0 ignored**（`test result` 行の集計）。`load` は 18 件、`modules` は全 `fle_*.c` のリンクを含めて通過。
  - `test-ws.txt` は 1 回目の打ち切りの記録で、最終結果は `test-ws3.txt` と `test-ws3.exit`（`test-ws.txt` を上書きしないため別名にした）。
  - 件数の差（sse-shuf-run の 320 件 → 502 件）の大半は、他の担当が同時に追加したテストと推測する（内訳は確認していない）。私の分は 18 件。

## 4. 生成、clang、リンク、実行

- 生成器: `out-a6/tool`（`out-a2/tool` の複製）、`out-a6/Cargo.toml`（cargo は workspace 外のパスを拒否するので前任と同じ方法）。seed は `out-a6/seeds-896.txt`（896 件、`out-a2` の複製）。
- 計測 1（応答表に私の 6 行が入り、A2 が入っている状態。`gen-a6-896.txt`）: functions 8427、closure_instructions 497367、emitted_instructions 497527、bad_nodes 0、**`unsupported_total` 380 → 332**（`out-a2` との比較）。`fle_resp_count` は 122、`fle_dispatch_count` は 8549。
  - 減った 48 件のうち、**私の分は −8**: `Jmp: indirect through IAT slot (not lifted)` が 299 → 291。`handled` の差に出た新しい行は `jmp_imp:FreeLibrary` 3、`ImageNtHeader` 1、`LoadLibraryA` 1、`LoadLibraryExW` 1、`LoadLibraryW` 1、`SetDllDirectoryW` 1（計 8）。残りの −40 は、同時に追加された MMX（`mmx_*` の行、`unsupported` の `Movd`、`Punpcklbw` などが消えた）で、私の作業ではない。`unsupported` の種類は 21 → 9。
- 計測 2（最後に再生成。他の担当の行が表に入った後。`imports_table.rs` に GlobalAlloc（A3）と MapViewOfFile、VirtualQuery（A13）の行があることは確認した。A9、A10 や VEX の分は、`runtime/` と `tests/` に対応するファイルがあることからの推測で、表の行は個別に確認していない。`gen-a6b-896.txt`）: **`unsupported_total` 266**、種類は 3（`Jmp: indirect through IAT slot` 245、`Jmp: indirect target unresolved` 20、`Xgetbv` 1）。`fle_resp_count` は 167。私の 6 API の `handled` は計測 1 と同じ（8 thunk）。この 266 は他の担当の追加を含み、私の分は −8 のまま。
- clang -O0 とリンク: `out-a6/build.sh`（`out-a2/build.sh` の複製、出力先の名前だけ変更）。計測 1 は `EXTRA_RT="fle_int_more fle_sse_more fle_locale fle_sys fle_x87ctl fle_vfs fle_reg fle_env fle_uilang fle_proc fle_time fle_uipref fle_res fle_load fle_imagehlp"`、計測 2 は `runtime/` の全 `fle_*.c`（固定の 7 本を除く。`extra-rt-b.txt`）。どちらも EXIT=0。
- 実行（`./bin/flengine-after-a6-896`、`flengine-after-a6b-896`。`run-a6-896.*`、`run-a6b-896.*`）:

| 項目 | out-a2（A2 後） | 計測 1 | 計測 2 |
|---|---|---|---|
| 停止点 | 0x410320（f_410320。間接呼び出しの先 0x40f960） | 同じ | 同じ |
| abort の種類 | fle_dispatch_miss | 同じ | 同じ |
| 関数実行数（distinct） | 125 | 125 | 125 |
| 関数エントリ数（calls） | 658 | 658 | 658 |
| 呼び出し深さ | 8 | 8 | 8 |

  `run-a6-896.txt` は `run-a2-896.txt` と、2 行目の `rsp` の値の違いを除いて同一（`diff <(sed 2d ...) <(sed 2d ...)` で確認）。

## 5. 実行の停止点について（事実と限界）

- **停止点は変わらなかった**。私の 6 API は、実行の停止点（0x410320、0x40f960 への間接呼び出しが `fle_dispatch_miss`）の手前では呼ばれていない。根拠: 実行の統計（関数実行数 125、エントリ数 658、深さ 8、停止点）が `out-a2` と一致する。ただし、トレースの入口ログは「658 件のうち先頭 256 件」だけで、残りの 402 件は個別に見ていない。256 件の中には、10 本のローダ thunk（0x423990、0x4239a0、0x407090、0x41cf90、0x407020、0x41cf10、0x421920、0x4233d0、0x423b40、0x4262a0）の入口は 0 件（`grep` で確認）。したがって「実行で踏まれて動作を確かめた」ことは言えない。**実装の検証の根拠は §3 の単体テストだけ**。
- 停止点 0x40f960 は A6、A12 の外。今回の変更では動かせない。
- 混在の内容（作業ツリー、`git status` の読み取り）: 計測の時点で、私の変更（`imports_table.rs` に 6 行、`runtime/fle_load.*`、`fle_imagehlp.*`、`tests/load.rs`、`docs/decisions/0029-*.md`、この文書）のほかに、他の担当による未コミットの変更が作業ツリーにあった。例: `fe-lift/src/lib.rs`、`fe-decode`、`fe-pe` の変更、`runtime/fle_imports.c`、`fle_sys.*`、`fle_vfs.*`、`fle_sse.h` の変更、新規の `runtime/fle_glob.*`、`fle_vol.*`、`fle_vmap.*`、`fle_winmm.*`、`fle_user_nowin.*`、`fle_res.*`、`fle_vex.h`、`fle_mmx.h`、`src/mmx.rs`、`src/vex.rs`、`imports_table.rs` の他の行（A2、A3、A9、A10、A13 の応答行）。これらを私は読んでいない部分が多く、内容の正しさは確認していない。
- そのため、`unsupported_total` が「私の変更だけを HEAD に重ねた場合に何になるか」は測れていない。**算術で言えるのは、私の寄与が −8**（HEAD からの 385 に対して、他の変更がなければ 377 になる、という計算。HEAD で実測したものではない）。

## 6. 作業中に拒否・失敗した操作

- 権限の拒否: 0 件。外部への通信は Microsoft Learn の取得のみ（公式ページ 7 つ。PE Format は 2037 行のうち先頭 594 行のみ表示され、残りは読んでいない。この作業で使った事実は先頭の部分（e_lfanew、署名、COFF ヘッダ、オプションヘッダ）で足りる）。
- 失敗: §3 の 3 件（テストの期待値の誤り 1、ワークスペースの 1 回目のタイムアウト、他の担当の行の追記と重なった `glob.rs` の 1 回）。
- 削除（rm）、`>` による既存ファイルの上書き、git 操作、/tmp の使用、sleep/timeout のコマンド文字列: なし。`cp -n`（既存を上書きしない）で `runtime/` に置いた。`test-ws.txt` は 1 回目のリダイレクトで作ったファイルで、上書きしていない（2 回目以降は別名）。
- 道具の注意: 決定の `.md` を書いた直後の確認で、自分の文章に誤りが 2 つあった（「SHCore.dll は 9 DLL にある」の行の誤記、動的な呼び出しの件数の混乱）。いずれも、同じ新規ファイル内で編集して直した。また、書いた時点で「A13 は表にない」としていた記述が、その後 A13 の行が追記されて古くなったので、編集で直した（`MapViewOfFile` の thunk 0x423a10 が `jmp [0x1971410]` で、そこが MapViewOfFile であることは `imports.txt` で確認した）。

## 7. 触ったファイル

- 新規（`lift/runtime/`）: `fle_load.h`、`fle_load.c`、`fle_imagehlp.h`、`fle_imagehlp.c`。
- 新規（テスト）: `lift/crates/fe-lift/tests/load.rs`。
- 編集（追記のみ）: `lift/crates/fe-lift/src/imports_table.rs`（`TABLE` に 6 行とコメント 1 行。編集の直前に読み直し、その時点の末尾は `LoadStringW` の行だった）。`lib.rs` は触っていない。
- 新規（文書）: `docs/decisions/0029-apis-a6-a12.md`、この文書。
- 新規（`lift/out-a6/`）: `Cargo.toml`、`build.sh`、`tool/`、`seeds-896.txt`、`work/`（`fle_load.*`、`fle_imagehlp.*` の作業版、`callsites2.py`、`callsites.py`、`ext_modules.py`、`callsites.tsv`、`callsites.err`、`ext_runs.tsv`、`obj/`）、`out/`（生成 C 2 本）、`bin/`、`*.o`、`gen-*.txt`、`run-*.txt`、`build-*.txt`、`extra-rt-b.txt`、`test-load-*.txt`、`test-ws*.txt`、`test-glob-recheck.txt`、`test/`、`tmp/`、`target-tool/`。
