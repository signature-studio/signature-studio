# プロセス・時刻の導入と計測実行（A1、A4）

作業者による実行記録。推測と実測を分けて書く。判断は `docs/decisions/0020-process-and-time.md`、仕様は `docs/investigations/import-proctime-spec.md`。前回の記録は `docs/investigations/uilang-run.md`（停止点 0x4193c4、`GetThreadPreferredUILanguages` が NULL）。

## 1. 実装（実測）

- 新規: `lift/runtime/fle_proc.h/.c`、`fle_time.h/.c`。作業用ディレクトリ `lift/out-proctime/work/` で `clang -std=gnu11 -O0 -Wall -Wextra` の警告 0 を確かめ、テスト（proc、time の各 -Werror つき）を通してから runtime/ に置いた（ヘッダを先、.c を後。`cmp` で作業用と同一を確認）。
- 追記: `imports_table.rs` に 26 行（A1 13、A4 9、追加 4: GetSystemTime、GetSystemTimeAsFileTime、GetTickCount64、SystemTimeToFileTime）。編集の直前に読み直し、他のサブエージェントの行（`GetThreadUILanguage`）の後に追記した。既存行は変更なし。表は 110 行、(DLL, API) の重複 0。
- 実装した API: A1 の 13 件（ExitProcess、TerminateProcess、RaiseException は「止める」実装）、A4 の 9 件のうち GetDateFormatW は一部のみ（下）、追加 4 件。
- 実装しなかった部分: GetDateFormatW の名前系（ddd、dddd、MMM、MMMM）、era（g、gg）、y と yyy、flags 付き、lpFormat NULL、他のロケール。ゲストの 2 か所の呼び出しは flags 4 と "gg" で、止まる形（仕様ノート §3.4）。SystemTimeToTzSpecificLocalTime の DST 規則（遷移日つきの構造体）も止める。

## 2. テスト（実測）

- `tests/proc.rs` 18 件、`tests/time.rs` 24 件。単独実行は、`out-proctime/test-pt-1.txt`（proc 18 通過、time 23 通過・1 失敗）、`test-pt-2.txt`（time 24 通過、EXIT=0）。
  - 1 回目の失敗 1 件は、**期待値の誤り**（`yyyy` に 301 年を入れた。SYSTEMTIME の年は 1601 以上なので、実装が正しく失敗を返した）。期待値を 1601 に直した。実装は変えていない。
- 期待値は Rust 側で独立に書いた。カレンダーは Hinnant の days_from_civil / civil_from_days（モデル自体も既知の日付で固定: 1601-01-01 = 0、1970-01-01 = 116444736000000000、2021-01-01 = 132539328000000000、曜日）。
  - FILETIME → SYSTEMTIME: 400 年周期の全 146,097 日（日の最後のミリ秒）と、境界値（うるう年の 1604、1700、1900、2000、2024、2100、2400 の 2/28、2/29、3/1、1601、2107、2108、9999、10000、30827 の年末）、乱数 3,000 件。往復の一致。
  - SYSTEMTIME → FILETIME: 実在しない日付（2/29 の非うるう年、4/31、月 0・13、日 0・32、年 1600・30828）と、時・分・秒・ミリ秒の境界の失敗。
  - DOS 日時: 日付 65,536 値 × 時刻 5 値（32 万件）の全数。FILETIME → DOS: 1979-12-25 〜 2108-01-07 の全日（始まりと終わり）。1980-01-01 と 2107-12-31 の境界。
  - 時計: 1 つの仮想時計をモデルで再現（読み出しで +10 tick、Sleep で +ms×10000、UTC = 起点 + tick）。同じプログラムを 2 回実行して同一。時刻の変化を待つループが 1000 回の読み出しで終わる。
  - タイムゾーン: 172 バイトの構造体を 1 バイトずつ照合（bias 0、"UTC"）。bias つきの変換（日・月・年・うるう日をまたぐ、1601 年より前で失敗）。
  - abort のテスト（止まること）: `assert_aborts` の表が proc 13 ケース、time 33 ケース（ソースの項目を数えた）。これに加えて、ExitProcess 4 値、TerminateProcess 2 形、RaiseException、SwitchToThread の上限、第 2 スレッドの個別テスト。いずれも SIGABRT、`fle:` のメッセージ、戻り値を返さない。
  - ソースが host の時計・環境・pid を使わないことの検査（コメントを除いた本文を検索）。
- ワークスペース全体: `cargo test --offline --workspace --no-fail-fast` は **EXIT=0、250 件通過、失敗 0**（`out-proctime/test-workspace-1.txt` の `EXIT=` と `test result` 行の合計）。起動時の 165 件との差は、他の担当の追加分と本作業の 42 件の合計（本作業の分は 42 件）。

## 3. 生成、clang、リンク（実測）

- 生成器: `lift/out-proctime/tool`（`out-vfs/tool` の複製）。ビルド EXIT=0（`build-tool.txt`）。seed は `out-proctime/seeds-896.txt`（896 件。`out-vfs` の複製）。
- 生成: EXIT=0（`gen-proctime-896.txt`、`.err` は空）。出力 `out-proctime/out/after-proctime-896.c`（42,530,409 バイト）。functions 8427、closure_instructions 497367（前回と同じ）。
  - `unsupported_total` **541**（`out-vfs` は 563）。`Jmp: indirect through IAT slot (not lifted)` は **304**（`out-vfs` は 326）。差の 22 は、A1 の 13 と A4 の 9 の thunk の数と一致する（各 1 か所）。`jmp_imp:` の行は 22 件すべてが生成物に出た。
- clang（-O0）とリンク: `out-proctime/build.sh`（`out-vfs/build.sh` の複製）。`EXTRA_RT="fle_int_more fle_sse_more fle_locale fle_sys fle_x87ctl fle_vfs fle_reg fle_env fle_uilang fle_proc fle_time"` で EXIT=0（`build-896.txt`）。`out-proctime/bin/flengine-after-proctime-896`。

## 4. 実行（実測）

`./bin/flengine-after-proctime-896 ../../binaries/win64/FLEngine_x64.dll`（TMPDIR は `out-proctime/tmp`）。出力 `run-proctime-896.txt`、`.err`、`.exit`。EXIT=134（SIGABRT）。

| 項目 | 前回（uilang-run.md） | 今回 |
|---|---|---|
| 停止点 | 0x4193c4（f_4193a0 内、`call [0x15eaee0]`） | **同じ** |
| abort の種類 | fle_dispatch_miss | fle_dispatch_miss（`indirect call to 0`） |
| 関数実行数（distinct） | 97 | 97 |
| 関数の呼び出し | 150 | 150 |
| 呼び出し深さ | 15 | 15 |

- `diff` で、停止の見出し（rsp を除く）と entry log（150 行の `callee=`）が前回と**完全に同一**。つまり**今回の 22 API は、この実行でまだ 1 回も呼ばれていない**（22 個の thunk アドレスを entry log と照合して 0 件）。停止点が進まなかったのは、実装の失敗ではなく、停止点が本作業の API より手前にあるため。
- したがって、**本作業の 22 API は、実行では未検証**で、テスト（42 件）でのみ検証している。ExitProcess の 0x412a0d、GetDateFormatW の 0x4431bd などの経路に実行が着くか、その引数が何かは、実行で確かめていない。

## 5. 次の停止点（実測と読み）

- 停止点: **0x4193c4**。命令: `call QWORD PTR [rip+0x11d1b16]  # 0x15eaee0`（`ff 15 16 1b 1d 01`）。引数（objdump）: ecx = 0x38、rdx = rbp+0x2c、r8 = 0、r9 = rbx。
- IAT の名前: **なし**（IAT スロットではない。`[0x15eaee0]` は 0x418e63 が書く `GetProcAddress(kernel32, "GetThreadPreferredUILanguages")` の戻り値。名前は 0x418ece のバイト列から読んだ）。
- 未対応の種類: **GetProcAddress の応答表にない API が NULL になり、ゲストが NULL の確認なしに呼ぶ**（`uilang-run.md` §9 と同じ）。続けて `SetThreadPreferredUILanguages`（`[0x15eaee8]`）も同じ状態。**本作業の担当外**（前回の記録のとおり）。実装していない。
- 前回の記録が「次の作業」とした GetThreadPreferredUILanguages と SetThreadPreferredUILanguages を応答表に足すのが、停止点を進める唯一の手である（推測。ここより先で何が止めるかは、足して実行するまで分からない）。

## 6. 残りの作業量（推測を含む）

- 本作業の担当（A1、A4）の未実装: GetDateFormatW の大部分と、DST 規則。実行がそこに来るまでは影響がない。
- 未対応の IAT jmp は 304 か所（user32 133 slot、gdi32 54 slot などが大半。`remaining-unsupported.md`）。
- 実行で最初に出会う未実装は、GetThreadPreferredUILanguages と SetThreadPreferredUILanguages（担当外）。そこを越えた先の停止点は未計測。

## 7. 実測として言えないこと

- 本作業の API がゲストにとって「正しい」かは、実行が進んでからでないと分からない（今回は 1 回も呼ばれていない）。
- GetCurrentThread の -2、GetCurrentProcessId の 0x1000、エラー番号 87（変換失敗）はページにない選択。
- Windows 実機との一致は未確認（ホストが Linux）。

## 8. 失敗・拒否された操作

- 権限の拒否は 0 件。
- 仕様ページの取得: 404 が 10 件（`fetch-spec*.txt` の `FAIL` の行の数。fileapi 版の DosDateTimeToFileTime、FileTimeToDosDateTime、FileTimeToSystemTime、winnls 版の GetDateFormatW、winnls 版の GetDateFormatEx、winnt 版の RaiseException、winbase 版の LocalFileTimeToFileTime など）。別の名前空間で取り直せたもの（DOS 日時は winbase、GetDateFormatW は datetimeapi、FileTimeToSystemTime は timezoneapi）と、取れなかったもの（仕様ノート §2）がある。
- テストの 1 回目の失敗（期待値の誤り、§2）。
- 途中で、`inform_shell` を呼ぶ前に誤ったツール名を 1 回呼んだ（実在しないため失敗。副作用なし）。

## 9. 残置物

- `lift/out-proctime/`: `spec/`（取得したページ）、`fetch_spec*.py`、`fetch-spec*.txt`、`test/`（テストが生成した C と実行ファイル）、`work/`（作業用のランタイムの複製と .o）、`target`、`target-tool`、`tool`、`out/after-proctime-896.c`（42 MB）、`bin/`、各 .o、`test-pt-1.txt`、`test-pt-2.txt`（1 回目は失敗の記録）、`test-workspace-1.txt`、`build.sh`、`seeds-896.txt`。削除禁止のため残す。
