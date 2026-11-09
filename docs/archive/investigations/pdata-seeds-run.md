# all-pdata seed でのネイティブ harness の再生成と実行

日付: 2026-10-20。決定は `docs/decisions/0031-pdata-seeds.md`、pdata の実測は `docs/investigations/pdata-seeds.md`。出力はすべて `lift/out-pdata/`（`tmp/` は TMPDIR）。前回の記録は `docs/investigations/a2-resources-run.md`（停止点 0x410320、先 0x40f960）。

「実測」はツールの出力、「推測」は未検証の解釈。

## 1. 実装したもの

- `fe-pe`: `Pe::runtime_functions`、`unwind_info`、`function_table`、`function_starts`、`function_start_vas`、`dir::EXCEPTION`、`RuntimeFunction`、`UnwindInfo`、`FunctionEntry`（`lift/crates/fe-pe/src/lib.rs`に追記）。CLI `fe-pe pe-pdata-seeds`、`pe-pdata-info`。
- `fe-decode`: `pdata_seeds(pe)`、`analyze_with_pdata(pe)`。CLI `fe-decode closure FILE --pdata`。
- `fe-lift`: CLI `fe-lift lift FILE OUT.c [--pdata] [--seeds SEEDS.txt]`（引数なしは従来どおり）。
- テスト: `fe-pe/tests/pdata.rs` 18 件、`fe-decode/tests/pdata_seeds.rs` 3 件（新規）。
- 計測ツール `lift/out-pdata/tool`（`pdata-seeds closure | compare | linear | lift`）、`split_c.py`、`build.sh`、`build-parts.sh`、`trace.c`（`out-trace/trace.c` の `SEEN_SIZE` のみ変更した複製）、`seeds-pdata.txt`（57,816 行）、`seeds-896.txt`（`out-a2` の複製）。

## 2. テスト

- `cargo test --offline --workspace --no-fail-fast`（`out-pdata/test-ws.txt`、`test-ws.exit` は EXIT=0）: **49 バイナリ、374 件 passed、0 failed、0 ignored**（`test result` 行の集計）。前回（a2-resources-run.md）の 339 件との差 +35 のうち、私の分は 21 件（fe-pe 18、fe-decode 3）。残りは同時に作業していた他の担当のテスト（私は確認していない）。
- 作業ツリー更新後の再実行（`test-ws3.txt`、`test-ws3.exit` は EXIT=0）: **54 バイナリ、489 件 passed、0 failed、0 ignored**。件数が 374 → 489 に増えたのは、同時に作業していた他の担当のテストが加わったため（私の分は 21 件のまま）。途中の `test-ws2.txt` は、他の担当の `cargo test` 3 本が同時に走っていて 900 秒のタイムアウトで打ち切られた（29 バイナリまで。失敗なし。集計には使っていない）。
- 実 DLL の数字（57,816 件、2,523,194 命令など）は `FLENGINE_PE` なしの既定パスで検証された（DLL あり、SKIPPED なし）。
- 1 回目の `pdata.rs` で 1 件失敗（`multi_level_chain_resolves_to_the_primary`、`Malformed("unwind info address has the low bit set")`）: テストの合成データの誤り。UNWIND_INFO（0x2070、code 1 個 = 4 + 2×2 + 12 = 20 バイト）の親エントリ領域が、次に置いた UNWIND_INFO 0x2080 の先頭と重なっていて、0x2080 のバイトが親エントリで上書きされ、読んだ unwind アドレスの下位ビットが立った（推測: 失敗メッセージと配置からの解釈。バイト列は未ダンプ）。実装の誤りではない。テストの配置を 0x2090、0x20c0 に移して通過（`pdata.rs` 18 件）。

## 3. seed と閉包（実測）

| 項目 | 896 seed（現在の作業ツリー） | all-pdata |
|---|---|---|
| 関数 | 8,427 | **58,695** |
| 基本ブロック | 73,234 | 345,466 |
| 閉包の命令 | 497,367 | **2,523,194** |
| decode 失敗 | 0 | **0** |
| 生成の emitted_instructions | 497,527（c_lines 548,457） | 2,523,994（c_lines 2,876,532） |
| 生成 C のバイト数 | 42,537,790 | **215,643,338** |
| 表（`fle_dispatch_table`）の行数 | 8,543 | 58,811 |
| `unsupported_total` | **340**（9 種類） | **3,027**（54 種類） |
| `bad_nodes` | 0 | 0 |

- 閉包の包含、「pdata 開始でない既存の関数」（463 件、すべて `jmp [IAT]` のサンク）、誤デコード 0 件の確認は `pdata-seeds.md` §4〜§5。
- **誤デコード（decode 失敗）の増加: 0 件**（`decode_failures` 0、`out_of_text_flow` 0、`bad_nodes` 0）。期待どおり。原因の分類は不要。
- `unsupported_total` 340 → 3,027（**+2,687**）。内訳の増分（`gen-896.txt` と `gen-pdata.txt` の一覧の差、実測）:
  - x87（Fld 591、Fmul 413、Fstp 210、Fadd 192、Faddp 163、Fxch 136、Fst 111、Fcompp 68、Fsubp 64、Fsub 58、Fild 44、Fabs 27、Fmulp 26、Fdivp 24、Fistp 20、Fchs 10、Fldz 5、Fimul 5、Fldpi 4、Fld1 4 など）: F で始まる x87 の命令の合計 **2,193**（`F*: mnemonic outside minimal set` の行の合計）。ただし 896 のときの `Fninit` 等の行は含まない。
  - `Jmp: indirect through IAT slot (not lifted)` 299 → 654（**+355**）。
  - `Jmp: indirect target unresolved` 20 → 62（+42）。
  - SSE2（Mulpd +25、Addpd +13、Cvtdq2ps +9、Subpd +8、Rcpps +8、Pcmpgtd +8、Psubw +10 など）と Prefetcht0 +5、Bswap +5 など。
  - 閉包の増加（命令 +407%）に対し `unsupported_total` は +790%。
- 閉包のサイズ: 旧実装の実績（約 58,000 関数 / 250 万命令）と一致。

## 4. ビルドの所要時間とメモリ（実測、`/usr/bin/time -v`）

マシン: 16 コア、RAM 約 19 GB（`free` の total）、ディスク空き約 840 GB。

| 工程 | 所要 | 最大 RSS | 出力 |
|---|---|---|---|
| 閉包（`pdata-seeds closure`、all-pdata） | 0.94 秒 | 320 MB | — |
| C 生成（`pdata-seeds lift`、all-pdata） | 2.52 秒 | 320 MB | `out/after-pdata.c` 215,643,338 B |
| C 生成（`fe-lift lift ... --pdata`） | （未計測） | — | `out/cli-pdata.c`（上と `cmp` で同一） |
| clang -O0 -g -finstrument-functions、**1 つの翻訳単位** | 53.7 秒で失敗 | 6,053 MB（`ulimit -v` 6 GB の制限下） | なし（`LLVM ERROR: out of memory`、EXIT=1） |
| 参考: 896 seed（42 MB の C、1 つの翻訳単位）の `build.sh` 全体 | 21.7 秒 | 2,434 MB | `bin/flengine-after-896`（45 MB） |
| clang 16 分割（`split_c.py` 3,700 関数/ファイル、`build-parts.sh`、並列 4） + runtime + harness + リンク | **36.3 秒** | 1,204 MB（`time -v` の最大。並列の子の最大） | `parts/*.o` 合計 502 MB、`bin/flengine-after-pdata` 254,530,856 B |

- 1 つの翻訳単位は、**私が掛けた 6 GB の仮想メモリ制限の下で** LLVM が out of memory で失敗した。制限なしで通るかは試していない（RAM の total は 19 GB なので通る可能性はあるが、未確認。分割で現実的な時間（36 秒）で通ったので試していない）。
- 2 GB を超える中間ファイルは作っていない（最大の .o は 49 MB、最大の C は 216 MB）。`-O0` のまま（フラグは既存の `build.sh` と同じ）。
- 分割の仕組みは `lift/out-pdata/split_c.py`（生成 C を `void f_XXXX(void)` の関数境界で切る。1 ファイル目に表と全域の定義、2 ファイル目以降に `extern` 宣言を置く）。fe-lift には分割機能が無く、今回も足していない。

## 5. 実行（実測）

コマンド: `./bin/flengine-after-pdata ../../binaries/win64/FLEngine_x64.dll`（`out-pdata/` で）。

### 5.1 1 回目（`out-trace/trace.c` のまま）

- `== stop: limit: wall time 120s`、`EXIT=4`、関数実行数 0。実行時間 120.00 秒（`HARNESS_MAX_SECONDS`）。
- 原因（実測と読解）: `out-trace/trace.c` の `is_lifted` が、表の関数ポインタを固定長 16384 の開番地ハッシュ（`SEEN_SIZE`）に入れる。表が 58,811 件なので満杯になり、`hset_has` の探索が終わらない（空きスロットが無い）。最初の関数入口の `__cyg_profile_func_enter` で止まる。runtime や生成コードの問題ではない（推測を含む: ハッシュが満杯であることはコードから読んだ。実際の無限ループは gdb では確認していない）。
- 対処: `lift/out-pdata/trace.c`（複製）で `SEEN_SIZE` を 262144 にしただけ。`lift/harness/trace.c`、`lift/out-trace/trace.c` は触っていない。

### 5.2 2 回目（`SEEN_SIZE` 262144）

- **0x410320（f_410320）は通過した。0x40f960（f_40f960）も呼ばれた。** 根拠: gdb（`gdb-pass.cmds`、`gdb-pass.txt`）で `f_410320` と `f_40f960` にブレークポイントを置いて continue した結果、どちらも **2 回**ヒット。`fle_dispatch_miss` は発生していない（abort の種類は `fle_unsupported`）。
- 停止: SIGABRT、`fle: unsupported instruction at 0x41cf90`、`abort kind: fle_unsupported`。実行 0.01 秒、最大 RSS 30 MB。

| 項目 | 896 seed（今回の作業ツリーで再実行） | all-pdata |
|---|---|---|
| 停止点 | 0x410320（f_410320 の間接 call、先 0x40f960）`fle_dispatch_miss`（a2-resources-run.md と同じ） | **0x41cf90**（f_41cf90）`fle_unsupported` |
| 関数実行数（distinct） | 125 | **137** |
| 関数入口（calls） | 658 | **704** |
| 呼び出し深さ | 8 | 9 |
| `unsupported_total`（生成） | 340 | 3,027 |
| 閉包（関数 / 命令） | 8,427 / 497,367 | 58,695 / 2,523,194 |

- スタック（実行ログ、gdb の `bt` と一致）: `#0 0x41cf90`、`#1 0x41d9a0`、`#2 0x41cfc0`、`#3 0x4236f0`、`#4 0x454090`、`#5 0x4120c0`、`#6 0x412160`、`#7 0x41d8f0`、`#8 0x1376820`（エントリ）。gdb の `bt` では `#4 f_454090` が `fle_dispatch`（f_4120c0 内、表のループ 0x412126 の間接 call）から呼ばれている。

### 5.3 3 回目（作業ツリーの更新後。他の担当の LoadLibrary 系などが入った fe-lift と runtime で再生成）

5.2 の後、作業ツリーに他の担当の変更（`imports_table.rs` に LoadLibraryA/W/ExW、FreeLibrary ほか、`runtime/fle_load.c`、`fle_imagehlp.c`、`fle_user_nowin.c`、`fle_winmm.c` など）が入った。それを含めて all-pdata を再生成（`out/after-pdata-b.c`、`gen-pdata-b.txt`）、再分割、再ビルド（`EXTRA_RT` に `fle_load fle_imagehlp fle_user_nowin fle_winmm` を追加。生成 C が参照する `fle_imp_*` 144 個がすべて runtime のどれかに定義されていることを確認してからビルド）、再実行した。

- 生成: 関数 58,695、命令 2,523,194（不変）、`c_lines` 2,876,616、`c_bytes` 215,649,724、**`unsupported_total` 2,991**（54 種類）、2.51 秒、最大 RSS 320 MB。同じツリーでの 896 seed は `unsupported_total` **309**（9 種類、`gen-896-b.txt`、`c_lines` 548,541）。→ 差は +2,682。
- ビルド: 35.9 秒、最大 RSS 1,202 MB、EXIT=0（`build-pdata-b.txt`）。`bin/flengine-after-pdata-b`。
- 実行（`run-pdata-b.txt`、EXIT=134）: **停止点 0x41cfa0（f_41cfa0）**、`fle: RaiseException: exception code 0xc06d007f, flags 0, 1 argument(s) raised; no structured exception handling is modelled, stopped here`（`fle_proc.c` の `fle_imp_RaiseException` が `fle_stop` した。`abort kind` は「runtime message でない」）。関数実行数（distinct）**140**、関数入口 **708**、深さ 9。実行 0.01 秒。
- スタック: `#0 0x41cfa0`、`#1 0x41d9a0`、`#2 0x41cfc0`、`#3 0x4236f0`、`#4 0x454090`、`#5 0x4120c0`、`#6 0x412160`、`#7 0x41d8f0`、`#8 0x1376820`。5.2 の 0x41cf90（LoadLibraryA）は通過した（`LoadLibraryA name=kernel32.dll` が 1 回呼ばれた。gdb、`gdb-gpa.cmds`）。
- 0x41cfa0 の命令は `ff 25 c2 44 55 01`（`jmp [0x1971468]`）で、スロットは **kernel32.dll!RaiseException**。呼び出し元は 0x41da25、0x41db65（f_41d9a0 内の 2 か所。`call 0x41cfa0`。実行時は 0x41db6a に戻る側）。
- 原因（実測の範囲）: 停止の直前の `GetProcAddress` は `GetNativeSystemInfo`（hmod 0x7ff800090000、`LoadLibraryA("kernel32.dll")` の戻り）で、**戻り値が 0**（gdb で `finish` 後の RAX）。`fle_modules.c` の応答表に `GetNativeSystemInfo` が無く、NULL（ERROR_PROC_NOT_FOUND）を返している（`GetNativeSystemInfo` の文字列は `fle_modules.c` に 0 件）。その後に `RaiseException(0xc06d007f)` が呼ばれる。0xc06d007f は `ERROR_PROC_NOT_FOUND`（127 = 0x7f）を含む遅延ロードのヘルパの例外コード（facility 0x6d = `VcppException`、下位 16 bit が 127）。（0xc06d007f が遅延ロードの「プロシージャが見つからない」であることは、数値の分解（facility 0x6d、code 0x7f）に基づく推測。Microsoft Learn の該当ページは確認していない）。
- 同じ呼び出し元の先行する `GetProcAddress` は `GetLogicalProcessorInformation`、`GetThreadPreferredUILanguages`、`SetThreadPreferredUILanguages`、`GetThreadUILanguage`（hmod 同じ）。
- 範囲外のため実装していない。次の作業の候補（事実のみ）: `GetNativeSystemInfo` を `fle_modules.c` の応答表に加えるか、遅延ロードのヘルパ（0x41d9a0 の呼び出し元）の NULL 処理を確認する。

## 6. 次の停止点の原因（5.2 の時点。5.3 で更新済み）

- VA **0x41cf90**。命令は `ff 25 82 44 55 01`（`jmp QWORD PTR [rip+0x1554482]`、スロット VA 0x1971418、RVA 0x1571418）、生成 C は `L_41cf90: { /* 0x41cf90 Jmp */ fle_unsupported(...) }`（`Jmp: indirect through IAT slot (not lifted)`）。
- そのスロットのインポートは **kernel32.dll!LoadLibraryA**（fe-pe のインポートの再計算で確認。独立の Python で RVA 0x1571418 を引いた）。つまり、IAT 経由のサンク（LoadLibraryA）が表に無い。
- 呼び出し元の関数 f_41d9a0 → f_41cfc0 → f_4236f0 → f_454090。f_454090 は表 0x13768b0 のループ（0x412126）の要素として dispatch 経由で呼ばれた。
- 関連: 作業ツリーには `lift/runtime/fle_load.c`、`fle_load.h`（未追跡、他の担当の作業）がある。LoadLibrary 系の実装と見られるが、`imports_table.rs` の表への追記が私のこの生成の時点であったかは確認していない（私は読み取りのみで、この生成は現在の作業ツリーの fe-lift で行った）。範囲外なので実装していない。
- 次に出る未対応の規模（推測を含む）: `Jmp: indirect through IAT slot` 654 件、x87 の F* 命令 2,193 件、が all-pdata の閉包に入っている。LoadLibraryA の次にも、IAT 経由のサンクで止まる可能性が高い（推測。実際の到達順は未確認）。

## 7. 制約の遵守と残置物

- `rm`、`>` による既存ファイルの上書き、git 操作、`/tmp`、sleep/timeout は使っていない。TMPDIR は `lift/out-pdata/tmp`。権限拒否はなし。外部通信は Microsoft Learn（x64 exception handling）の取得 1 回。
- 既存ファイルの変更は編集ツールの局所変更のみ。`lib.rs`（fe-lift）、`imports_table.rs`、runtime は触っていない。
- 後始末できていないもの（削除禁止のため）: `lift/out-pdata/build-parts.sh.bak`（`sed -i.bak` が作ったバックアップ。内容は旧 `trace.c` パスの版）、`out/cli-pdata.c`（`out/after-pdata.c` と同一の 216 MB の重複）、`build-pdata.txt` と `build-pdata.err`（1 回目のビルドのログ）、`single-tu.*`（1 つの翻訳単位の失敗ログ）。`lift/out-pdata/` 全体で約 1.8 GB（`out` 676 MB、`parts` 502 MB、`bin` 286 MB、`target-tool` 25 MB など）。
- 触ったファイル（作業ツリー、`lift/out-pdata/` 配下の生成物は除く）: `lift/crates/fe-pe/src/lib.rs`（M、追記のみ）、`lift/crates/fe-pe/src/main.rs`（M）、`lift/crates/fe-pe/tests/pdata.rs`（新規）、`lift/crates/fe-decode/src/lib.rs`（M、追記）、`lift/crates/fe-decode/src/main.rs`（M）、`lift/crates/fe-decode/tests/pdata_seeds.rs`（新規）、`lift/crates/fe-lift/src/main.rs`（M）、`docs/decisions/0031-pdata-seeds.md`、`docs/investigations/pdata-seeds.md`、`docs/investigations/pdata-seeds-run.md`（新規）。`lift/out-pdata/` 全体は新規（`pdata_probe.py`、`split_c.py`、`build.sh`、`build-parts.sh`、`trace.c`、`tool/`、`seeds-*.txt`、`out/`、`parts/`、`bin/`、各ログ）。
- 5.3 で追加した生成物: `out/after-pdata-b.c`、`out/after-pdata-b-part-*.c`、`out/after-896-b.c`、`bin/flengine-after-pdata-b`、`gen-*-b.txt`、`run-pdata-b.*`、`gdb-*.cmds` と `gdb-*.txt`、`imps-b.txt`、`imp-defs.txt`。`lift/out-pdata/` 全体は約 2.3 GB に増えた（`du` で未再計測）。
