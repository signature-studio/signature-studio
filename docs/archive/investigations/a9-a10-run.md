# A9（user32 窓なし 12 API）と A10（winmm 10 API）の追加後の実行（896 seed の閉包）

日付: 2026-10-20。決定は `docs/decisions/0030-apis-a9-a10.md`。出力は `lift/out-a9/`（`tmp/` が TMPDIR）。前回の記録は `docs/investigations/sse-shuf-run.md`（`unsupported_total` 385、停止点 0x407030）。手順は同記録 §4、§5 と同じ（生成器の複製、`build.sh`、実行）。

**ワークツリーには他の担当の未コミットの変更が混在していた**（§0）。数値の変化は、この作業だけによるものではない。内訳は §3 に分けた。

## 0. 計測時のワークツリー

`git status --short` を計測の直前に `out-a9/git-status-at-measure.txt` に保存した。自分が触ったのは `imports_table.rs`、`runtime/fle_user_nowin.*`、`runtime/fle_winmm.*`、`tests/a9_a10.rs`、`docs/` の 2 文書、`out-a9/`。それ以外に次が混在していた（自分は触っていない）:

- 変更: `fe-decode/src/lib.rs`、`fe-decode/src/main.rs`、`fe-lift/src/main.rs`、`fe-pe/src/lib.rs`、`fe-pe/src/main.rs`、`runtime/fle_sys.c`、`fle_sys.h`、`fle_vfs.c`、`fle_vfs.h`（`imports_table.rs` には他の担当の行も入っている）。
- 新規: `fle_imagehlp.*`、`fle_load.*`（決定 0029）、`tests/glob.rs`、`tests/load.rs`、`fe-decode/tests/pdata_seeds.rs`、`fe-pe/tests/pdata.rs`。
- 作業中に `fle_res.*`、MMX（`mmx.rs`、`fle_mmx.h`）、決定 0025、0028、0029、0031 がワークツリーに現れた（計測時には `git status` の一覧に出ないものもある。コミットされた可能性。確認していない）。

## 1. 一覧と閉包内の実測（実装前）

入力: `lift/out-sse2/out/after-sse2-896.c`、`lift/objdump-text.txt`、`lift/out-imports/imports.txt`。

- 一覧（`remaining-unsupported.md` §5.1 の A9、A10）: user32 12 API、winmm 10 API。IAT スロットは 22 個すべて `imports.txt` にある。
- `.text` 全体で 22 API の thunk は 26 個。そのうち閉包（`after-sse2-896.c` に `Jmp` の `fle_unsupported` として出る）にあるのは **23 個**（user32 13: CharNextW が 2、winmm 10）。閉包の外の 3 個は 0x4247d0（CharLowerBuffW）、0x4247f0（CharNextW）、0x424840（CharUpperBuffW）。
- 各 API が閉包内で要求する引数と戻り値の使い方は、決定 0030「閉包内の呼び出しの実測」にまとめた。ここで確認できた要点: CharUpperW は 1 文字渡しがある（0x42d368）、IntersectRect は dst と src1 が同じ、midiOutGetNumDevs の 2 か所（0xba4648、0xba47fc）は台数 0 でも本体が 1 回実行され、**MIDI_MAPPER（-1）で midiOutGetDevCapsW を呼ぶ形に読める**。
- RegisterWindowMessageW / RegisterClipboardFormatW の引数は PE の `.data` から文字列を読んだ（`out-a9/names.py`、出力 `names-out.txt`）: メッセージ 12 か所（名前 11、読めない 1）、クリップボード形式 27 か所（名前 25、読めない 2）。

## 2. 実装

- 新規（置いた順: ヘッダ、`.c`、表、テスト）: `lift/runtime/fle_user_nowin.h`、`fle_winmm.h`、`fle_user_nowin.c`、`fle_winmm.c`（`out-a9/work/` で `-Wall -Wextra -Werror` のコンパイルと単体テストを済ませてから `runtime/` に 1 回で置いた。`cmp` で同一を確認）。
- 編集（追記のみ）: `lift/crates/fe-lift/src/imports_table.rs` に 22 行（直前に読み直した。他の担当の A6/A12 の行が入った後ろに追記した）。`lib.rs` は触っていない（API の追加は表だけで済む）。
- 実装した API: **22**（user32 12、winmm 10）。一覧の全部。`fle_winmm.c` は仮想時計（`fle_sys.c`）を変えず、`timeGetTime` も変更していない。
- 「止める」入力（`fle_stop`）を持つ API の数と理由は、決定 0030 の各項と「確認できなかったこと」に列挙した。

## 3. `unsupported_total` の変化（実測）

生成: `out-a9/tool`（`out-shuf/tool` の複製）、`out-a9/Cargo.toml`、seed `out-a9/seeds-896.txt`（896 件、複製）。`gen-a9-896.txt`、`.err` は空、EXIT=0。`out/after-a9-896.c`（42,544,021 バイト）。functions 8427、closure_instructions 497367、emitted 497527、bad_nodes 0（前回と同じ）、c_lines 548541（前回 548442）。

| 項目 | 前回（sse-shuf） | 今回 |
|---|---|---|
| `unsupported_total` | 385 | **309**（−76） |
| `unsupported` の種類 | 21 | 9 |
| `Jmp: indirect through IAT slot` | 304 | 268（−36） |
| MMX と Emms | 37 + 3 | 0（−40） |

内訳（`gen-shuf-896.txt` と `gen-a9-896.txt` の `handled` の差、および `unsup-before.txt`、`unsup-after.txt`、`unsup-removed.txt` の `comm` で確認。`unsup-added.txt` は 0 行 = 新しく未対応になった箇所はない）:

| 減った分 | 件数 | 由来 |
|---|---|---|
| **この作業（A9 + A10）** | **23** | 22 API の thunk 23 個。`unsup-removed.txt` に 23 個全部が含まれる。`handled` に 22 行（CharNextW は 2） |
| MMX（他の担当） | 40 | `mmx_*` の行 12 種（合計 40）が `handled` に現れた |
| リソース・ライブラリロード・ImageNtHeader（他の担当） | 13 | FindResourceW、LoadResource、LockResource、SizeofResource、LoadStringW（5）、LoadLibraryA/W/ExW（3）、FreeLibrary（3）、SetDllDirectoryW、ImageNtHeader（各 1） |
| 計 | 76 | 23 + 40 + 13 = 76。算術で一致 |

つまり、**この作業だけの寄与は 385 → 362 に相当する −23**。309 は他の担当の変更を含む混在状態の値。`Jmp: indirect through IAT slot` の −36 は 23（今回）+ 13（他）。

残る 309 の内訳: IAT スロット経由 268、解決不能 `Jmp` 20、AVX/VEX 18（Vmovups 12、Vmovaps 2、Vmovntps 2、Vzeroupper 2）、FS/GS 2、Xgetbv 1。268 + 20 + 18 + 2 + 1 = 309。

## 4. clang とリンク

`out-a9/build.sh`（`out-shuf/build.sh` の複製で、既定の `NAME` のみ `after-a9-896` に変更）、`EXTRA_RT` は `runtime/*.c` のうち標準の 7 つ以外すべて（`fle_env fle_imagehlp fle_int_more fle_load fle_locale fle_proc fle_reg fle_res fle_sse_more fle_sys fle_time fle_uilang fle_uipref fle_user_nowin fle_vfs fle_winmm fle_x87ctl`）。EXIT=0（`build-896.txt`）。リンクで未解決の記号はなく、追加した 22 関数がリンクされた。

## 5. 実行

コマンド: `./bin/flengine-after-a9-896 ../../binaries/win64/FLEngine_x64.dll`（`out-a9/` で）。出力 `run-a9-896.txt`、`.err`、`.exit`（EXIT=134、SIGABRT）。

| 項目 | 前回 | 今回 |
|---|---|---|
| 停止の種類 | `fle_unsupported` at 0x407030（`user32!LoadStringW` の thunk） | `fle: indirect call to 0x40f960 is not a known function entry`（`fle_dispatch_miss`） |
| 関数実行数（distinct） | 116 | 125 |
| 関数エントリ数（calls） | 340 | 658 |
| 呼び出し深さ | 9 | 8 |

- 停止点は変わったが、**この作業の 22 API が原因ではない**。前回の停止点 0x407030 は LoadStringW（A2、他の担当）で、これが応答されたため先へ進んだ。今回の停止は、間接呼び出しの飛び先 0x40f960 が関数エントリ表（`fle_dispatch_table`）にない（`fle_dispatch_miss`）という別の問題。停止時の呼び出し鎖の先頭は `f_410320`（guest-return 0x448f0c）で、誰がどの命令から呼んだかは調べていない。なお、逆アセンブルには 0x41c164 に `call 0x40f960`（直接呼び）があるが、生成 C にはこのアドレスも 0x40f960 も 1 件も出ない（grep で 0 件）ので、閉包の外の関数であり、今回の停止の原因ではない（この点は最初の版で取り違えたので直した）。
- 実行ログ（`run-a9-896.txt`）の先頭 256 エントリには、追加した 22 API の関数は 1 つも現れない。ログは 658 エントリのうち先頭 256 しか含まないので、**残り 402 エントリで呼ばれたかは分からない**。したがって、追加した API が実行で踏まれたかは確認できていない。検証の根拠は単体テストである。

## 6. テスト

- `cargo test --offline -p fe-lift --test a9_a10`: **27 件、すべて通過**（ワークスペース実行内の `Running tests/a9_a10.rs` の結果行で確認。`finished in 99.75s`）。内訳: A9 の応答 13（CharXxx 5、RECT 5（格子 4000 組を含む）、登録系 3）、A10 の応答 11、ジェネレータ 2（22 thunk がすべて表を通って runtime を呼ぶ、22 関数がすべて宣言・定義されリンクできる）、時計との整合 1。名前付き abort のケースはプロセスを分けて実行し、`fle:` の出力、SIGABRT、「returned」が出ないことを 1 件ずつ確認した。応答表の行数を固定値でテストしていない。
- 期待値はテスト内に書いた（MMRESULT の数値、ASCII の対応、矩形の独立モデル、0xC000 の割り当て）。ヘッダから読み戻していない。
- ワークスペース全体: `cargo test --offline --workspace --no-fail-fast`（`test-ws-2.txt`、`test-ws-2.exit` は EXIT=0）: **54 バイナリ、489 件 passed、0 failed、0 ignored**（`test result` 行の集計）。混在状態での値で、他の担当のテストを含む。
- **1 回目のワークスペース実行（`test-ws.txt`）は、`timeout` 900000 ms で完了しなかった**。出力は 28 バイナリ分の途中までで、`test-ws.exit` は作っていない。この実行の中で失敗したテストはない（失敗行は 0 件）が、完了していないので集計に使わない。遅れた原因は**推測**: `ps` で、別の `cargo test --workspace` と `fe-lift` のテストが他に 3 本以上並行して動いていた（他の担当の実行。自分のものではない）。確認していない。2 回目は timeout を 3600000 ms にして完了した。
- 実装中の失敗（すべてテスト側の誤りで、実装側の修正はなかった）: (1) `st.handled.get` の型の誤りでコンパイルに失敗（`.as_str()`）。(2) ジェネレータのテストの合成 PE で、呼び出し列が thunk の領域に重なり（22 個 × 5 バイトが 0x1040 を超える）、先頭の API が未対応になった（thunk の開始を 0x1100 に移した）。(3) 矩形の格子が偏っていた（交差ありが 57 / 4000。LCG の下位ビットと範囲の取り方の問題。splitmix64 と幅・高さの指定に変えて 493 / 4000 に）。(4) 0xC000 の使い切りのテストで、abort 前の標準出力がバッファに残り失われた（`fflush` を追加）。

## 7. 失敗・拒否された操作

- 権限の拒否: なし。
- 失敗: 上記（§6）。Microsoft Learn の `multimedia-error-codes` と `api/mmsyscom/` は 404 で、MMRESULT の数値は取れなかった（非公式の mingw-w64 ヘッダと windows-sys で補った。決定 0030）。`raw.githubusercontent.com` の windows-rs のパスも 404 だった。
- 外部への通信: Microsoft Learn（API ページ）、docs.rs（windows-sys の定数ページ）、raw.githubusercontent.com（Wine の `win.c`、windows-rs。どちらも根拠には使っていない）。
- 削除、既存ファイルの上書き（`>`）、git 操作、/tmp の使用: なし。`out-a9/tmp` を TMPDIR にした。`out-a9/build.sh` は `out-shuf/build.sh` を複製したあと、`sed -i` で既定の `NAME` を書き換えた（自分が新規に作ったファイルへの局所変更）。
- 未実行・未確認: 追加した 22 API が実行で呼ばれること（§5）。midiOutGetDevCapsW の MIDI_MAPPER の応答（決定 0030 で未決）。Windows 実機との比較。

## 8. 触ったファイル

- 新規: `lift/runtime/fle_user_nowin.h`、`fle_user_nowin.c`、`fle_winmm.h`、`fle_winmm.c`、`lift/crates/fe-lift/tests/a9_a10.rs`、`docs/decisions/0030-apis-a9-a10.md`、この文書、`lift/out-a9/` 以下（`work/`、`tool/`、`Cargo.toml`、`build.sh`、`names.py` と出力、`unsup-*.txt`、`gen-*`、`run-*`、`test-*`、`out/after-a9-896.c`、`bin/`、`target-tool/`、`test/`、`tmp/` ほか）。
- 編集（追記）: `lift/crates/fe-lift/src/imports_table.rs`（22 行）。
