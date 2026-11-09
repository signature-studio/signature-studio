# 仮想ファイル・レジストリ・環境の導入と計測実行

作業者による実行記録。推測と実測を分けて書く。判断は `docs/decisions/0018-virtual-fs-reg-env.md`、API ごとの仕様は `docs/investigations/import-vfs-spec.md`。前回の記録は `docs/investigations/modules-run.md`（停止点 0x407120、kernel32!SetThreadLocale）。

## 1. 実装（実測）

- 新規: `lift/runtime/fle_env.h/.c`、`fle_vfs.h/.c`、`fle_reg.h/.c`。3 ファイルとも `clang -Wall -Wextra` で警告 0（`fle_reg.c` は一度 `L""` を UTF-16 の空文字列に使う誤りを直した。`wchar_t` は 4 バイト）。
- 追記: `lift/crates/fe-lift/src/imports_table.rs` に 42 行（ファイル 25、レジストリ 12、環境 5）。他のサブエージェントの行は触っていない。追加後の表は 83 行（`grep -c '^    ("'`）。重複する行は `uniq -d` で 0。他のサブエージェントの行数の内訳（ロケール 15、スレッド・メモリ 15 など）は数えていないので、上の「15 行」は取り消す: 83 − 13（元からの行）− 42（本作業）= 28 行が他の追加分。
- 新規テスト: `lift/crates/fe-lift/tests/vfs.rs`（22 件）、`reg.rs`（10 件）、`env.rs`（7 件）、共通部 `tests/fsx_common/mod.rs`。

## 2. テスト（実測）

- 3 ファイルを単独で実行: **39 件全件通過**（`lift/out-vfs/test-fsx-3.txt`、EXIT=0）。
  - 1 回目（`test-fsx-1.txt`）: コンパイルエラー（共通部の `ExitStatusExt::signal` の書き方）。直した。
  - 2 回目（`test-fsx-2.txt`）: 4 件失敗（env 1、reg 1、vfs 2）。4 件とも、実装ではなく**期待値の数え間違い**だった: `C:\Data\file.txt` の長さと NUL（16＋1＝17）、`GetFullPathNameW` の長さ（`C:\a\y.txt` は 10 文字）、`GetModuleFileNameW` のパスの長さ（57 文字。依頼文に長さは書かれていない）、`RegEnumValueW` の ERROR_MORE_DATA で返るサイズ（添字 0 は作成順で 8 バイトの値）。期待値を直して 3 回目で全件通過。
  - 期待値は、ページの規則を Rust のリテラルと式で書いた（C のコードを読み戻していない）。ページが番号を書かない箇所（import-vfs-spec.md で「推測」）は、テストの中に「inference」とコメントした。
  - abort のテスト: 3 ファイルで計 88 ケース（vfs 38、reg 34、env 16。ソースの `cases` の項目を数えた）（`undefined_input_stops_the_run`）。全て SIGABRT、`fle:` のメッセージ、戻り値を返さないことを確認した。
  - ホストの隔離: `runtime_never_uses_host_file_environment_or_registry_apis` が `fle_vfs.c`、`fle_reg.c`、`fle_env.c` に `fopen`、`getenv`、`opendir`、`<unistd.h>` などがないことを確かめる。`environment_starts_empty_and_hides_the_host` は、テストプロセスに PATH があることを確かめた上で、仮想環境から `PATH` が見えない（0 と 203）ことを確かめる。`host_paths_do_not_exist_in_the_virtual_volume` は `/etc/hostname` などが存在しない（3 と 2）ことを確かめる。
- `cargo test --offline --workspace --no-fail-fast`: 下の §7 に結果を書く。

## 3. 生成、clang、リンク（実測）

- 生成器: `lift/out-vfs/tool`（`out-modules/tool` の複製。新規）。ビルド EXIT=0（`build-tool.txt`）。seed は `lift/out-vfs/seeds-896.txt`（896 件。`out-modules` の複製）。
- 生成: EXIT=0（`gen-vfs-896.txt`、`gen-vfs-896.err` は空）。出力 `lift/out-vfs/out/after-vfs-896.c`（42,524,093 バイト）。
  - functions 8427、closure_instructions 497367（前回と同じ）。c_lines 548358。
  - 今回の 42 行の thunk が全て出た（`jmp_imp:CreateFileW 1`、`RegOpenKeyExW 2`、`GetModuleFileNameW 2`、`GetEnvironmentVariableW 2` など。他のサブエージェントの行も含む）。
  - `unsupported_total` 563（前回 `out-modules` は 667）。`Jmp: indirect through IAT slot (not lifted)` は 326（前回 420）。この減少は 3 者の変更（本作業、ロケール、スレッド・メモリ）の合計で、本作業の分だけは分けていない。本作業の 42 個は閉包で 52 件の jmp にあたる（import-vfs-spec.md）。
- clang（-O0）とリンク: `lift/out-vfs/build.sh`（`out-modules/build.sh` の複製。新規）。`EXTRA_RT="fle_int_more fle_sse_more fle_locale fle_sys fle_x87ctl fle_vfs fle_reg fle_env"` で EXIT=0（`build-896.txt`）。`lift/out-vfs/bin/flengine-after-vfs-896`。`fle_locale`、`fle_sys`、`fle_x87ctl` は他のサブエージェントのファイル（読み取りのみ。リンクに必要）。

## 4. 実行（実測）

コマンド: `./bin/flengine-after-vfs-896 ../../binaries/win64/FLEngine_x64.dll`（TMPDIR は `lift/out-vfs/tmp`）。出力 `lift/out-vfs/run-vfs-896.txt`、`run-vfs-896.err`。EXIT=134（SIGABRT）。

- 停止点 VA: 0x419419（f_419400 の中。生成物上は `call QWORD PTR [rip+0x11d1ad1]` の 1 命令）。関数 f_419400 の入口は 0x419400。
- abort の種類: `fle_dispatch_miss`（`fle: indirect call to 0 is not a known function entry`）。`fle_unsupported` ではない（今までの停止はすべて `fle_unsupported`）。
- 関数実行数（distinct）: 96。関数の呼び出し（entries）: 149。呼び出し深さ: 14。

```
#0 0x419400 f_419400 guest-return=0x419613
#1 0x419560 f_419560 guest-return=0x41a2d2
#2 0x41a1a0 f_41a1a0 guest-return=0x41a440
#3 0x41a3d0 f_41a3d0 guest-return=0x418bb4
#4 0x418b80 f_418b80 guest-return=0x418c0a
#5 0x418be0 f_418be0 guest-return=0x41c87a
#6 0x41c820 f_41c820 guest-return=0x4123b0
#7 0x4123a0 f_4123a0 guest-return=0x412491
#8 0x4123c0 f_4123c0 guest-return=0x42d890
#9 0x42d870 f_42d870 guest-return=0x412129
（深さ 14 のうち、トレースが出したのは 10 段。#10 以降の欄は出力されていない）
```

### 前回との比較

| 項目 | 前回（modules-run.md） | 今回 |
|---|---|---|
| 停止点 | 0x407120 f_407120（kernel32!SetThreadLocale） | 0x419419 f_419400 内の `call [0x15eaef0]`（GetProcAddress が NULL を返した関数ポインタ） |
| abort の種類 | fle_unsupported | fle_dispatch_miss |
| 関数実行数（distinct） | 35 | 96 |
| 関数の呼び出し | 51 | 149 |
| 呼び出し深さ | 6 | 14 |

- 前回との差は、本作業だけの効果ではない。同じ生成器と runtime に、ロケール系（SetThreadLocale など）、スレッド同期・時間・メモリ系の追加が入っている。本作業の効果として言えるのは、次の「実際に呼ばれた」ことだけ（entry の並びから。`lift/out-vfs/run-vfs-896.txt` の `callee=` を、`out-usage/iat-jmp-unsupported-all-v3.tsv` の thunk アドレスで API 名に引いた）。
  - `GetCommandLineW` 1、`GetStartupInfoW` 1、`GetModuleFileNameW` 2、`RegOpenKeyExW` 6 回が通過した（他の API の呼び出しも通過したが、本作業の対象外なので省く）。
  - `RegOpenKeyExW` の 6 回は、f_419560 の周辺（0x419a33〜0x419b11）の呼び出し。空のレジストリなので、2（ERROR_FILE_NOT_FOUND）を返したはず（推測。戻り値は表示していない）。呼び出し元は 0x419a38 の `test eax,eax; je 0x419b1e` で、失敗側に分岐したと読める（命令の読みであり、トレースではない）。
- ファイル系（CreateFileW、FindFirstFileW など）、`GetEnvironmentVariableW` は、この実行ではまだ呼ばれていない（entry に出ていない）。したがって**ファイル系の実装は、実行では未検証**で、テスト（39 件）でのみ検証している。

## 5. 次の停止点

- VA: **0x419419**（f_419400 の中）。命令: `call QWORD PTR [rip+0x11d1ad1]  # 0x15eaef0`（`ff 15 d1 1a 1d 01`）。objdump で確認した。
- IAT の名前: **なし**（IAT スロットではない）。0x15eaef0 は .data のグローバル変数で、0x418ea7（f_418e10 内）が `GetProcAddress(GetModuleHandleW(L"kernel32.dll"), "GetThreadUILanguage")` の戻り値を書く。呼ばれる名前は、バイト列から読んだ（0x418f0a: `GetThreadUILanguage`）。同じ関数が 0x15eaee0 と 0x15eaee8 にも `GetThreadPreferredUILanguages`、`SetThreadPreferredUILanguages` を書く（0x418ece、0x418eec）。
- 未対応の種類: **GetProcAddress の応答表にない API が NULL になり、ゲストが NULL の確認なしに呼ぶ**。f_418e10 は、`GetVersion` の下位バイトが 6 以上（0x418e2e〜0x418e39。仮想 OS は 10.0 なので成立）のときに 3 つの名前を引き、NULL を確認せずに保存する（0x418e63、0x418e85、0x418ea7）。f_419400 の 0x419419 は、その 1 つを無条件に呼ぶ。
- 呼び出し元: f_419400 は f_419560 の 0x41960e（guest-return=0x419613）から呼ばれる。
- 方針の見立て（推測）: 決定 0015 は「応答表にない API は NULL」。今回のように、実機の Vista 以降に必ずある API（GetThreadUILanguage）を NULL にすると、ゲストは落ちる。ロケール系（`fle_locale.c`）の担当に `GetThreadUILanguage` がない。応答表に `GetThreadUILanguage` を足す（ロケール担当の仕事）か、0015 の方針（OS のバージョンが 6 以上なら存在するはずの API は応答表に入れる）を見直す必要がある。**本作業の担当外。実装していない。**
- 同じ種類（呼び出し先が NULL）の停止は、残りの 2 つ（`GetThreadPreferredUILanguages`、`SetThreadPreferredUILanguages`）でも起き得る（呼び出し元は未調査）。

## 6. 残りの作業量（推測を含む）

- 本作業の担当のうち未実装は 4 個（CreateFileMappingW、MapViewOfFile、UnmapViewOfFile、SetDllDirectoryW）。うちメモリマップ 3 個は設計が要る（呼び出し元は計 14 関数のべ）。
- 未対応の IAT jmp は 326 か所残っている（担当外のものが大半。user32 133 slot、gdi32 54 slot などは未着手）。
- ファイル系・環境系は実行で未到達なので、停止点が進むにつれて、引数（パス、キー名）の実態で abort が出る可能性がある。出たら、abort の種類（上の「仕様が確認できずに abort」）と引数から判断する。

## 7. ワークスペース全体のテスト

`cargo test --offline --workspace --no-fail-fast`: **EXIT=0、204 件通過、失敗 0**（`lift/out-vfs/test-workspace-1.txt` の `EXIT=` 行と `test result` 行の合計で確認）。件数には他のサブエージェントのテスト（locale、sys など）と本作業の 39 件を含む。起動時の前回の件数（129）との差は、3 者の追加分の合計で、本作業の分は 39 件。

## 8. 実測として言えないこと

- Windows 本体との一致は、ホストが Linux のため未確認。
- 本作業の API がゲストにとって「正しい」かどうかは、ゲストの実行が先へ進んでからでないと分からない（ファイル系は未到達）。
- 停止点の前後で `RegOpenKeyExW` が実際に返した値は、戻り値を記録していないので未確認。

## 9. 残置物

- `lift/out-vfs/test-fsx-1.txt`、`test-fsx-2.txt`: テストの 1 回目（コンパイルエラー）、2 回目（期待値の誤り）のログ。
- `lift/out-vfs/target`、`target-tool`: cargo のビルド出力。`lift/out-vfs/test/`: テストが生成した C と実行ファイル（`vfs-*`、`reg-*`、`env-*`）。`lift/out-vfs/obj/`: 警告確認のためのオブジェクト。削除禁止のため残す。
- `lift/out-vfs/spec/`、`fetch_spec.py`、`condense.py`、`scope.py`、`scope.tsv`: 仕様ページの取得と集計。
