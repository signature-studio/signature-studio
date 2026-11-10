# 決定 0092（SRW ロック 4 名と条件変数 3 名）の実装、テスト、統計、3 モジュールの測定

凡例: **実測**（この作業で実行して得た値、取得したページ）／**読み**（逆アセンブル、ソース、文書を読んだ結果。実行していない）／**未確認**／**意見**（事実ではない。別枠）。

この文書の状態: 実装・テスト・統計・3 モジュールの測定まで記入済み（下の「作業の状態」）。

- HEAD: `a64a41c`（ブランチ main）。ワークツリーに決定 0092 の未コミット変更（この作業のもの）。
- 決定: `docs/decisions/0092-srw-locks-and-condition-variables.md`（**仮の決定**）。調査（前提）: `docs/investigations/blend2d-remaining-imports.md`（コミット a64a41c）。
- 作業物: `lift/out-srw/`（`.gitignore` の `lift/out*/` で追跡されない）。`TMPDIR=lift/out-srw/tmp`、`CARGO_TARGET_DIR=lift/out-srw/target`。
- 3 モジュールの基底（`0x2000000`、`0x4700000`）と Blend2D の種 58 個は**測定用の値で、決定ではない**（決定 0086）。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | `cargo test --workspace` | **98 binaries / 1,195 passed / 0 failed / 0 ignored**（基準 97 / 1,178 / 0 / 0。**+1 binaries（新規 `srw.rs`）、+17 件**）。SKIPPED 0 件 | 実測（§3） |
| 2 | FLEngine の生成 C の SHA-256 | `eb9d8e7f…09cf` → **`feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f`**（215,707,705 B → 215,709,232 B）。**変わったのは応答表の 5 か所 32 行だけ。変わった関数は 0 個**。functions 59,297、bad_nodes 0、unsupported_total 525、register_jmp_sites 20、jumptabs 4、msvc_tables 0、msvc_rejected 0 は不変。debug と release の `fe-lift` で同一 | 実測（§4） |
| 3 | 単独構成の a4 | **145 / 942**、functions 821、entries 15,685、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`。**基準と一致**。2 回同一 | 実測（§5） |
| 4 | 3 モジュール構成の a4 | init_count **150 / 942**（不変）、last unit 149（proc `0x664260`）、functions_distinct **1,069**（前回 1,059）、function_entries **27,332**（前回 27,319）。**停止が `0x489cab9`（`AcquireSRWLockExclusive`）から `fle: indirect call to 0x48a20ff is not a known function entry` に動いた**（top of stack `f_48a2060`）。2 回同一 | 実測（§6） |
| 5 | 次の停止の分類 | **(v) その他**: MSVC の switch 表（`jmp r9`、`0x48a208b`）が決定 0090 の dominance で解決されず `fle_dispatch(R[9])` のまま、移動先 `0x48a20ff` が既知の関数入口でない。未応答の import ではない（(i) ではない）、TLS（ii）、JIT（iii）、ロケール（iv）でもない。**SRW・条件変数・スレッドの他の名前ではない**（調査 §9 の見込みと一致） | 実測（停止）、読み（原因） |
| 6 | DllMain(ATTACH) は戻ったか | **戻っていない**（`DLLMAIN-RETURN reason=1: out of scope (did not return normally)`。停止時の `eax = 0x48f32c8` は途中の値で DllMain の戻り値ではない）。LoadLibraryW は 0x4700000 を**返していない**（`LOADLIBRARY-RETURN` は dsp_ippv2 の `0x2000000` の 1 回だけ） | 実測（§6.3） |
| 7 | ATTACH 内の import | **183 回**（通し番号 1526〜1708。前回 178 回）。**1〜1703 は前回と名前・番号が同一**で、後ろに 5 つが加わっただけ: `AcquireSRWLockExclusive`、`ReleaseSRWLockExclusive`（1 対）、`GetSystemInfo`、`HeapAlloc`、`InitializeCriticalSection` | 実測（§6.3） |

## 2. 実装（決定 0092 の「実装した場所」と同じ。読み）

- `lift/runtime/fle_sys.c`: クリティカルセクションの節の直後に、SRW ロックと条件変数の節（`/* ---- Slim reader/writer locks and condition variables (decision 0092, provisional) ---- */`）を足した。
  - 状態は `fle_srw_table[65536]`（`fle_cs_hash` を共有。同時に 32,768 本まで。満杯は `STOP`）。**登録は取得中だけ**（未登録のアドレスは空のロック）。項は `shared == 0`（排他）か `shared == n`（共有 n 個）。
  - Acquire Exclusive: 未登録なら排他で登録。登録済み（排他でも共有でも）は名前付き停止。Acquire Shared: 未登録なら共有 1 で登録、共有なら +1、排他なら停止。Release Exclusive/Shared: 同じモードで登録されていれば（Shared は個数を 1 減らし 0 で）項を空ける。未登録、モードの食い違いは停止。
  - 空けた項は、次の項が EMPTY なら EMPTY（直前の DELETED の連なりも EMPTY に戻す）、そうでなければ DELETED にする（探索の連鎖を切らない）。
  - Wake、WakeAll: `fle_sys_check_thread` と NULL の検査だけで何もしない（`R[0]` は触らない）。
  - Sleep: 全入力で `STOP`。メッセージに `ConditionVariable`、`SRWLock`、`dwMilliseconds`、`Flags` を 10 進（アドレスは `%#llx`）で出し、ロックが未登録のとき（Learn が「undefined」と書く）、INFINITE、0、有限の 4 通りを文言で区別する。
  - ゲストの SRWLOCK／CONDITION_VARIABLE の 8 バイトは読まず書かない（`fle_ld`、`fle_st`、`fle_addr` なし）。
- `lift/runtime/fle_sys.h`: 7 関数の宣言（`fle_imp_SleepConditionVariableSRW` は `noreturn`）。
- `lift/crates/fe-lift/src/imports_table.rs`: `TABLE` の末尾（`GetModuleHandleA` の次）に 7 行。擬似 VA は `0x7ff900001060`〜`0x7ff9000010c0`（行の位置 262〜268）。既存の行の擬似 VA は 1 つも動かない。
- 新しい `.c`、`.h` は作っていない。

## 3. テスト（実測）

`cd lift && CARGO_TARGET_DIR=…/lift/out-srw/target FLENGINE_PE=…/binaries/win64/FLEngine_x64.dll cargo test --workspace --no-fail-fast > out-srw/test-ws.txt 2>&1`（終了コード 0）。`grep '^test result' | awk`: **98 binaries / 1,195 passed / 0 failed / 0 ignored**。SKIPPED 0 件。ログ `lift/out-srw/test-ws.txt`。**2 回実行した**: 1 回目（`test-ws-first.txt`。a4・3 モジュールの測定の前）と、すべての変更（Wake の変異検査の後に元へ戻した `fle_sys.c`、ドキュメント）を終えた最終の状態での 2 回目（`test-ws.txt`）。どちらも同じ数。2 回目の前に `fle_sys.c` が、テストと測定に使ったコピー（`out-srw/fle_sys.c.good`）と `cmp` で同一であることを確認した。

### 3.1 既存テストの書き換え（1 件。件数は増減しない）

| ファイル:行（変更後） | 変更前 → 変更後 | 理由 |
|---|---|---|
| `lift/crates/fe-lift/tests/modules.rs` の `get_module_handle_a_has_a_response_row_…`（旧名 `…_at_the_end_…`。関数名も変えた） | 「`GetModuleHandleA` の行が**応答表の最後**で、擬似 VA が `0x7ff900000000 + 0x10 * st.resp_entries`」 → 「`fle_resp_table` の行を順に読み（新しい補助関数 `resp_rows`）、**全行の擬似 VA が `0x7ff900000000 + 0x10 * (位置 + 1)`**、`GetModuleHandleA` の行が 1 つあり、その位置を n として擬似 VA が `0x7ff900000000 + 0x10 * n`、`GetModuleHandleW`（`0xc0`）と `GetProcAddress`（`0xd0`）の行が前のとおり、`GetModuleHandleW`、`GetProcAddress`、`GetModuleHandleExW` がそれより前にあること」 | 末尾に行を足すと「最後の行」の前提が崩れる（調査 §8.2）。「最後」を外し、新しい最後の行の位置も固定せず、**行の相対位置と既存の VA の不変**だけを確かめる形にした。検査の強さ: 旧は「最後の行の VA が n」と 2 行の VA。新は全行について「VA = base + 0x10 × (位置 + 1)」（これが「末尾に足せば既存の VA が動かない」の根拠そのもの）を確かめるので、既存の VA の不変については旧より強い |

**他に失敗した既存テストは無かった**（`IsProcessorFeaturePresent`、`TlsFree`、`WaitForSingleObjectEx` に関するテストは変えていない。今回それらの行は足していないので、調査 §8.2 の他の「失敗する見込み」は起きない）。`call_iat_dll.rs` の合計 138 は変わらず通った。

### 3.2 新規テスト（`lift/crates/fe-lift/tests/srw.rs`。17 件）

| テスト | 内容 |
|---|---|
| `the_jmp_iat_thunks_call_the_runtime_and_the_rows_follow_get_module_handle_a` | 生成器の `jmp [IAT]` が 7 名とも `fle_imp_*(); return;`、宣言あり、未応答 0。全行の擬似 VA = base + 0x10 × (位置 + 1)。`GetModuleHandleW`（`0xc0`）、`GetProcAddress`（`0xd0`）は前のまま。`GetModuleHandleA` が従来の位置（`0x7ff900001050` = 261 番目）のまま。7 行がその直後に決まった順で 1 回ずつ。`fle_dispatch_table`、`fle_dispatch_count` に載る |
| `the_call_iat_form_calls_the_runtime_with_the_call_shape` | `call [IAT]` の形が 7 名とも `call_imp:` で、戻り VA を積み → 呼び → pop する決定 0077 の形 |
| `an_exclusive_round_trip_and_the_first_use_of_an_unknown_address` | 空 → Acquire Exclusive → Release Exclusive の往復を 3 周。未登録アドレス（ゼロの 8 バイトと 0xAB の 8 バイト）の最初の Acquire が空として取れる。`R[0]` が変わらない（VOID）。8 バイトが変わらない |
| `shared_acquisitions_are_counted_and_released_one_by_one` | Shared を 3 個取って 3 回解放してから Exclusive が取れる。別のロックと独立。1,000 個の共有、1,000 本のロック |
| `acquire_stops_where_the_page_says_deadlock_or_where_no_one_could_release` | Exclusive 中の Exclusive／Shared、Shared 中の Exclusive（1 個、2 個、1 個解放して 1 個残り）が名前付き停止（SIGABRT、メッセージ） |
| `release_stops_when_the_lock_is_not_held_in_that_mode` | 持たない Release（未登録、解放済み、取得数より多い、別のロック）、モードの食い違い（Shared 中の Exclusive Release、Exclusive 中の Shared Release）が停止 |
| `a_null_pointer_stops_by_name` | NULL（6 名）が名前付き停止 |
| `another_thread_stops_the_run_for_every_name` | pthread の別スレッドからの呼び出しが 7 名とも `fle: <名前> called from tid …` で停止 |
| `the_table_limit_stops_and_released_entries_are_reused` | 同時に 32,768 本まで取れて、32,769 本目で `fle: more than 32768 SRW locks held at once`。全部を並べ替えた順で解放後、再利用できる。半分解放後、残りが見つかり、解放した側が空きになる |
| `released_entries_in_a_probe_run_do_not_hide_the_locks_behind_them` | **同じハッシュに当たる 8 個のアドレス**（テスト内でハッシュを再計算し、衝突することを確かめる）の連鎖で、真ん中の解放、最後の解放、再取得、共有の別順の解放を行い、連鎖の後ろのロックが見つかる。再取得の 2 重取得と持たない 2 重解放が停止 |
| `wake_and_wake_all_do_nothing_and_keep_r0` | Wake、WakeAll の `R[0]` が 153 のまま（VOID）。ロックを持っている間も、共有中も、ロックの状態を変えない。CONDITION_VARIABLE とロックの 8 バイトが変わらない |
| `sleep_condition_variable_stops_for_every_input_and_says_the_two_values` | INFINITE、有限（100、1、0xFFFFFFFE）、0、共有ロックと Flags 1、Flags の非解釈（`0x80000001` が 2147483649 と出る）、ロック未取得、解放後、ロック NULL、条件変数 NULL、Wake の後の Sleep が、すべて名前付き停止。メッセージに `dwMilliseconds` と `Flags` の値、ロック状態、理由（INFINITE／0／有限／ロック未取得）が出る。上位 32 ビットは無視される |
| `the_runtime_never_reads_or_writes_the_guests_lock_or_condition_variable` | ロックと CONDITION_VARIABLE を **PROT_NONE のページ**に置き、7 名を停止しない範囲で呼ぶ（読み書きしたら SIGSEGV = signal 11）。解放後、バイトが同じ |
| `the_new_code_touches_no_guest_memory_and_reads_nothing_from_the_host` | `fle_sys.c` の新しい節（コメントを除く）に `fle_ld`、`fle_st`、`fle_addr`、`memcpy`、`memset`、ポインタのキャスト、`getenv`、`clock_gettime`、`pthread`、`nanosleep` などの語が無い（禁止語の流儀。調査 §8.3） |
| `get_proc_address_finds_the_rows_and_dispatch_answers_like_the_direct_call` | `GetProcAddress(kernel32, <7 名>)` が `fle_resp_table` の擬似 VA を返す。大文字小文字が違う綴りは NULL と 127。`fle_dispatch(擬似 VA)` が直接呼びと同じ答え（VOID は 153）。擬似 VA で取り、直接関数で解放できる（同じ表）。擬似 VA 経由の Sleep が停止 |
| `the_static_lock_shapes_of_blend2d_run_and_never_touch_the_lock_bytes` | Blend2D の形を合成バイト列で再現: (a) 静的ロックに `call [IAT]` の Acquire と **`jmp [IAT]` の Release（tail jump）**、(b) `lea rbx,[this]; lea rcx,[rbx+0x30]` の静的オブジェクトの Acquire/Release（停止点 `f_18019caa0` の形）、(c) 共有の Acquire/Release、排他の Acquire、Wake、WakeAll、排他の Release（`jmp`）。**画像のデータページを PROT_NONE にして 2 周走らせる**（ロックのバイトを読み書きすれば SIGSEGV）。rsp、rbx、ゲストの戻りアドレスが保たれ、バイトが同じ |
| `a_lock_a_guest_function_leaves_held_stops_the_next_acquire` | 解放しない関数の 2 回目の呼びで、名前付き停止（`fle: AcquireSRWLockExclusive on a lock 0x140002700 that is already held exclusively`） |

### 3.3 変異検査（この作業の自己検査。決定の根拠ではない）

- Wake の本体に `R[0] = 0;` を足した作業コピー: 新規 17 件のうち **3 件が落ちた**（`wake_and_wake_all_do_nothing_and_keep_r0`、`the_runtime_never_reads_or_writes_the_guests_lock_or_condition_variable`、`get_proc_address_finds_the_rows_…`）。元に戻して 17 件通過（`cmp` で保存したコピーと同一）。
- 空けた項の再利用の処理（`fle_srw_release_entry`）を「常に DELETED にしない（探索の連鎖を切る）」に変えた作業コピー: `released_entries_in_a_probe_run_do_not_hide_the_locks_behind_them` が**落ちた**（`ReleaseSRWLockExclusive on a lock … that is not held`）。元に戻して通過。**この変異で `the_table_limit_…` は通った**（同じハッシュに当たらない並びでは差が出ないので、衝突させる専用のテストを足した）。

## 4. FLEngine の生成 C（実測）

`fe-lift lift ../binaries/win64/FLEngine_x64.dll out-srw/fle.c --pdata`（13.2 秒）。SHA-256 **`feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f`**。debug と release の `fe-lift` の出力は `cmp` で同一。`diff out-gmh/fle.c out-srw/fle.c`（`out-gmh/fle.c` の SHA は基準 `eb9d8e7f…09cf`）は **32 行**（`lift/out-srw/fle-diff.txt`）で、変わったのは応答表の次の 5 か所だけ:

| 場所 | 変更 |
|---|---|
| `fle_imp_*` のプロトタイプ | 7 行の追加（`void fle_imp_AcquireSRWLockExclusive(void);` ほか） |
| `fle_dispatch_table` | 末尾に 7 行（`0x7ff900001060`〜`0x7ff9000010c0`） |
| `fle_dispatch_count` | 59558 → 59565 |
| `fle_resp_table` | 末尾に 7 行 |
| `fle_resp_count` | 261 → 268 |

**変わった関数は 0 個**（この 5 か所を除くと、新旧の C はバイト単位で同一）。理由: FLEngine は SRW、条件変数を import していない（調査 §8.1。pefile）ので、`jmp [IAT]` の thunk も `call [IAT]` も変わらない。統計: functions 59,297、bad_nodes 0、unsupported_total 525（`Jmp: indirect through IAT slot (not lifted)` 524、`Jmp: indirect target unresolved` 1）、register_jmp_sites 20、jumptabs 4、msvc_tables 0、msvc_rejected 0。**基準の SHA を固定するテストは無い**（`grep` で `*.rs` に `eb9d8e7f`、`43985d62` が 0 件）。今後の基準は `feb5b9087f8d…`。

## 5. 単独構成の a4（実測。FLEngine だけ。`lift/out-srw/a4s/`）

`split_c.py out-srw/fle.c … 3700`（17 パート）→ `OUT=…/out-srw/a4s JOBS=2 bash harness/wasm/build_native.sh`（新しい OUT。runtime の .o 46 個を最初から作った。`build.err` 0 行）→ `OUT=… FEPE=… bash harness/a4-survey.sh` を 2 回（1 回目 18:01:35Z、2 回目 18:01:42Z。同じ秒の `units.tsv` との衝突を避けて空けた）:

```
A4 survey 20261024T180135Z   harness exit: 134
static : units=942 runnable=895 empty=46 noproc=1 (header 0x13768b0, units 0x13768e0)
stop   : fle: LoadLibraryW: module "dsp_ippv2_x64.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)
         top of stack: 0x423990 f_423990
depth  : init_count=145 / 942   last unit: index=144 run_rank=132/895 proc=0x65c540 in_progress
         started=132 completed=131 in_progress=1 of 895 runnable   all_units_done=no
         entry check: started_but_not_entered=0 runnable_entered_beyond_init_count=0
funcs  : executed (distinct)=821  entries=15685
```

**基準（145/942、functions 821、entries 15,685、`LoadLibraryW "dsp_ippv2_x64.dll"`）と一致**。2 回の出力は見出しの時刻と `logs` 行以外同一（`diff`）。なお、最初に `FEPE` を `out-srw/target/release/fe-pe` に指定したとき、その `fe-pe` はまだビルドされておらず（当時の `CARGO_TARGET_DIR` がシェルに設定されていなかったため、`cargo build` は `lift/target` に作っていた）a4-survey が `pe-init-table failed`（終了コード 2）で終わった。`CARGO_TARGET_DIR` を設定して `out-srw/target/release/fe-pe` をビルドし直し、同じ `survey1.txt` に書き直した（失敗の出力は上書きされて残っていない）。**その間違った `cargo build --release -p fe-pe` が `lift/target/release/`（`.gitignore` の `lift/**/target/`）に `fe-pe` を 1 つ作った**（作業物の規則では `lift/out-srw/target` のみを使う。`lift/target` は他の作業の target ではなく、リポジトリ既定の target。追跡されない。削除はしていない）。なお `lift/target/insn-int-test/` の更新は私が直接作ったものではなく、既存のテスト（`crates/fe-lift/src/insn_int.rs:394` が `CARGO_MANIFEST_DIR/../../target/insn-int-test` に固定で書く）が `cargo test --workspace` のたびに `CARGO_TARGET_DIR` の指定に関係なく更新するもの（この作業の実行でも更新された。既存の挙動）。

## 6. 3 モジュール構成の測定（測定のみ。gdb の仮の値なし、runtime の応答追加は今回の 7 名だけ。新しい停止は直していない）

### 6.1 手順（実測）

- 作業コピー（HEAD a64a41c + この作業の変更）の release の `fe-lift`、`fe-pe`（`lift/out-srw/target/release`）で、FLEngine（`--pdata`、§4 の `out-srw/fle.c` をコピー）、dsp_ippv2 縮小（`--seeds seeds-idx0-plus-initterm2.txt`）、Blend2D（`--pdata --seeds seeds58-4700000.txt`。種 58 個）の生成 C を作り直した（`lift/out-srw/three/`）。
- `lift/out-gmh/three/harness-two/` を `out-srw/three/harness-two/` にコピー（`diff -rq` で同一。スクリプトは変えていない）。リベース済み DLL と種を `out-gmh/three/dll/`、`out-gmh/three/seeds/` からコピーし、SHA-256 が前回の記録と一致: dsp_ippv2 `4fd25502…`、Blend2D `24e01ec9…`、dsp の種 `cf664439…`、Blend2D の種 `ca6439b6…`。
- `gen_two.sh rebase gen split merge`。`MODULES`、`TAG=b2d58` は前回（0091）と同じ。**`EXPECT_FLE_SHA=feb5b9087f8d…`**（新しい SHA を環境変数で渡した。スクリプトは変えていない）。`build_two_native.sh`（`JOBS=8`）。runtime のオブジェクト、FLEngine のオブジェクト、ハーネスは新しい OUT で最初から作った。two-native は 612,293,968 B（2 GB 未満）。ログ `lift/out-srw/three/gen_two.txt`、`build.txt`、`build.err`（0 行）。
- 表の統合: 59,565 + 8,298 + 2,334 → **69,661 行**（前回 69,654。複数の表に在る行 **268**（前回 261）、`f_<VA>` の衝突 0）。

### 6.2 生成 C の変化（前回の `out-gmh/three/v-b2d58/gen/` との `diff`。実測）

- **縮小 dsp_ippv2（`real.c`）**: 応答表の 5 か所（プロトタイプ 7、`fle_dispatch_table` 7、`fle_dispatch_count` 8291 → 8298、`fle_resp_table` 7、`fle_resp_count` 261 → 268）だけ。**関数の本体は変わらない**（dsp_ippv2 はこの 7 名を import していない）。functions 8,030、bad_nodes 0。
- **Blend2D（`blend2d.c`）**: 同じ応答表の 5 か所 + **65 サイト**の `fle_unsupported(UINT64_C(…))` が応答する形に変わった（呼び出し点 83 のうち、SRW・条件変数の 7 名 = 20 + 27 + 3 + 6 + 4 + 1 + 4 = **65**。残り 18 はこの作業の対象外の名前）。内訳: `call [IAT]` の形（`{ void fle_imp_X(void); R[4] -= 8; fle_st(R[4], 8, UINT64_C(next)); fle_imp_X(); R[4] += 8; }`）が Acquire Exclusive 20、Release Exclusive 24、Acquire Shared 3、Release Shared 6、Wake 4、WakeAll 1、Sleep 4 の 62 サイト、**`jmp [IAT]` の形（`{ void fle_imp_ReleaseSRWLockExclusive(void); fle_imp_ReleaseSRWLockExclusive(); return; }`）が Release Exclusive の 3 サイト**（`0x489bef4`、`0x489bf56`、`0x489d06d`。調査 §4.0 の `jmp` 3 か所と一致）。**調査 §4.0 の数（20、27、3、6、4、1、4）と全て一致**。他の行の変更は無い（`fle_unsupported` の削除 65 と、同じ番地の置換 65 の他は応答表の行だけ）。functions 2,066、bad_nodes 0 は不変。

### 6.3 a4 の行（2 回: 18:04:45Z、18:04:52Z。gdb なし。ログ `lift/out-srw/three/out/a4/`）

```
A4 survey 20261024T180445Z   dll: …/binaries/win64/FLEngine_x64.dll   harness exit: 134
static : units=942 runnable=895 empty=46 noproc=1 (header 0x13768b0, units 0x13768e0)
stop   : fle: indirect call to 0x48a20ff is not a known function entry
         top of stack: 0x48a2060 f_48a2060
depth  : init_count=150 / 942   last unit: index=149 run_rank=137/895 proc=0x664260 in_progress
         started=137 completed=136 in_progress=1 of 895 runnable   all_units_done=no
         entry check: started_but_not_entered=0 runnable_entered_beyond_init_count=0
funcs  : executed (distinct)=1069  entries=27332
```

- 2 回の a4 の出力は見出しの時刻と `logs` 行以外同一。stderr は `cmp` で一致、stdout は `rsp=` の値を除いて一致。（1 回目と 2 回目の間は 7 秒空けた。）
- **次の停止メッセージ全文**: **`fle: indirect call to 0x48a20ff is not a known function entry`**（`abort kind: fle_dispatch_miss`、`dispatch target: 0x48a20ff`、`call depth at stop: 20`）。stderr も同じ 1 行。
- **C のスタック**（先頭 10 行。`native.txt`）:

```
#0 0x48a2060 f_48a2060 guest-return=0x489da70
#1 0x489d880 f_489d880 guest-return=0x476ee62
#2 0x476ed70 f_476ed70 guest-return=0x489b08c
#3 0x4899d90 f_4899d90 guest-return=0x4701009
#4 0x4701000 f_4701000 guest-return=0x48a59eb
#5 0x48b5b90 f_48b5b90 guest-return=0x48a59eb
#6 0x48a59a8 f_48a59a8 guest-return=0x489e34e
#7 0x489e2b4 f_489e2b4 guest-return=0x489e4c4
#8 0x489e264 f_489e264 guest-return=0x489e4c4
#9 0x489e450 f_489e450 guest-return=0xcb000000
```

  （前回の停止（先頭が `f_489caa0`）の `#1` 以降（`f_4899d90` から）が今回の `#3` 以降と同一。**今回の `#0`〜`#2`（`f_48a2060` ← `f_489d880` ← `f_476ed70`）が加わった**。`f_476ed70` は `f_4899d90` の `0x489b087: call 0x476ed70`（`.xc` の初期化子の末尾の 5 つの直接 call の最後。調査 §4.1 の 3、§9 の 2 が「自身は行なしの API を呼ばない」と読んだもの）の先。）
- **`0x48a208b` は `jmp r9`（MSVC の switch）**。`f_48a2060`（原本 `0x1801a1060`）は `memcpy` 風の小サイズコピー（`rcx` = コピー先、`rdx` = コピー元、`r8` = サイズ）。入口で `lea r10,[ImageBase]`、`cmp r8,0xf; ja 0x48a2180`、`data16 nop` の詰め物、`mov r9d,[r10+r8*4+0x2fd050]; add r9,r10; jmp r9`。**停止時のサイズ `r8 = 3`**（gdb の停止時のレジスタ `r8=0x3`、`r9=0x48a20ff`）で、表（RVA `0x2fd050`、16 項。`r8` = 0〜15）の 3 番目が `0x48a20ff`（3 バイトのコピー: `movzx ecx,word [rdx]; movzx r8d,byte [rdx+2]; mov [rax],cx; mov [rax+2],r8b; ret`）。`f_476ed70` が `0x476ee5d: call 0x489d880`、`f_489d880` が `0x489da6b: call 0x48a2060`（`rcx = r15 + r13`、`rdx = r9`、`r8 = r12` = 3）。
- **生成器はこの `jmp r9` を解決していない**: `gen-blend2d.txt` の `msvc_rejected 0x48a208b: N not derived: no \`ja\` before the table idiom (only \`cdqe\` and \`lea B,[rip+d]\` may sit between)`（`cmp r8,0xf; ja` と表を引く `mov` の間に `data16 nop` の詰め物があるため、決定 0089/0090 の上限 N の導出が通らない）。Blend2D の `msvc_rejected` は 77 件（前回と同じ数。`0x48a208b` は前回から拒否の 1 つ）。決定 0090 の検証が解決したのは 3 件（`msvc_tables 3`）で、**この switch は解決されない形**。関数は `fle_dispatch(R[9]); return;` に生成され、`0x48a20ff` は既知の関数入口ではないので `fle_dispatch_miss` で止まる。（**これを直していない**。測定のみ。）
- **gdb（読み取りだけ。`lift/out-srw/gdb/ro_srw3.py`。`out-gmh/gdb/ro_stop3.py` の無変更のコピー。`return`、`set var`、`call`、値の書き込みなし。ログ `lift/out-srw/run/m3-ro1-imports.txt`、`m3-ro2-imports.txt`、`-gdb.log`、`-stdout.txt`、`-stderr.txt`）**: 2 回実行し、**import のログは `cmp` で全バイト一致**。
  - **Blend2D の DllMain(ATTACH) は戻ったか**: **戻っていない**（`DLLMAIN-RETURN reason=1: out of scope (did not return normally)`。スタックに `f_489e584` が残る。停止時の `rax = 0x48f32c8` は途中の値で、DllMain の戻り値ではない）。
  - **LoadLibraryW が 0x4700000 を返したか**: **返していない**（`LOADLIBRARY-RETURN` は 1 回だけで `R[0]=0x2000000`（dsp_ippv2）。Blend2D の `fle_realmod_load` は戻っていない）。
  - Blend2D の DllMain に入るまでの `fle_imp_*` は 1,525 回（前回と同じ）。
  - **ATTACH 内の import の列**: **183 回**（通し番号 1526〜1708。前回 178 回）。**1526〜1703 は前回と通し番号・名前が同一**（比較: 名前と番号の組が 1703 件全部一致）。後ろに 5 つが加わった:

| 通し番号 | import | 引数と戻り値（読み取り） |
|---|---|---|
| 1700〜1703 | 前回と同じ（`GetModuleHandleA`、`GetProcAddress` ×3） | |
| **1704** | **`AcquireSRWLockExclusive`**（今回実装） | rcx = `0x49f4800`（調査 §4.2 の静的な `this+0x30` = 原本 `0x1802f4800` のリベース後）、戻りアドレス `0x489cabf` |
| **1705** | **`ReleaseSRWLockExclusive`**（今回実装） | rcx = `0x49f4800`、戻りアドレス `0x489cb0c`（`f_489caa0` の Release の 1 つ。調査 §4.2 の読み「`[this+0x40] = 0x20000`、戻り値 0」の経路: `r8 = 0x20000` が 1705 の r8 に見える） |
| **1706** | `GetSystemInfo` | 戻りアドレス `0x470fc0f` |
| **1707** | `HeapAlloc` | rcx = `0x7ff700000000`（プロセスヒープ）、r8 = `0xa8` |
| **1708** | `InitializeCriticalSection` | rcx = `0x555566fc0b10`（`HeapAlloc` が返した領域）、戻りアドレス `0x470fc9c` |

  **調査 §4.2、§9 の読み（`f_18019caa0` の Acquire 1 + Release 1 だけが ATTACH で確実に実行される SRW）は実測と一致した**: 1704、1705 の 1 対が実行され、Acquire Exclusive も Release Exclusive も**停止せずに戻った**（SRW の他の 5 名は ATTACH のここまでで呼ばれていない）。続いて `.xc` の初期化子の直接 call の先（`f_476ed70` の経路）で `GetSystemInfo`、`HeapAlloc`、`InitializeCriticalSection` が呼ばれ、上の `memcpy` 風の関数で止まった。
  - 名前ごとの回数の前回との差: `AcquireSRWLockExclusive` +1、`ReleaseSRWLockExclusive` +1、`GetSystemInfo` +1（2 → 3。通し番号 1706）、`HeapAlloc` +1、`InitializeCriticalSection` +1。
  - **名前付き停止の import は無い**。CELLS（`[0x48f3280]`、`[0x48f3288]`、`[0x48f3290]`、`[0x48f31a0]`）は停止時もすべて 0（前回と同じ。`GetProcAddress` の 3 つの NULL とフラグ 0）。
- `Sleep`、`Wake`、`Shared` は ATTACH のここまでで 1 回も呼ばれていない（import の列に無い）。**調査 §4.1 の読み（「B のみ」「ATTACH の直線上では呼ばれない」）と矛盾しない**。ただし ATTACH はまだ終わっていない（停止）ので、この先で呼ばれるかは未確認。

### 6.4 停止点の分類

**(v) その他**: 未応答の import ではなく、**生成器が解決しない MSVC の switch 表**（`jmp r9`。`msvc_rejected: N not derived`）で、`fle_dispatch` が `0x48a20ff` を既知の入口でないとして止まる。これは `0x470a89e`（決定 0090）、`0x48a57f8`（`blend2d-seed58.md`）と同じ種類（B5/B3 の `jmp reg` の解決）で、(ii) TLS、(iii) JIT、(iv) ロケールではない。**SRW・条件変数・スレッドの他の名前ではない**（調査 §9 の見込みと一致）。unit 158（quickfontcache_x64.dll の LoadLibraryW）には届いていない。

- **DllMain が戻った場合に記録する GetProcAddress("AggCreateInstance") 以降は、DllMain が戻っていないので記録の対象外**（届いていない）。

## 7. 解釈が分かれた点、実装できなかった点

実装できなかった PM の決定の項目は無い。決定の内容は変えていない。詳細は決定 0092 の「解釈が分かれた点」。

## 8. 限界、未確認

- 実機の Windows での挙動は未確認（ホストは Linux）。決定 0092 の仮の点 (a)〜(e) は Learn が書かない動作を作っている。
- ヒープ上のロック（スレッドごとのオブジェクトの `+0x80`、他）の中身をゲストが読むかは未確認（調査 §4.2。静的な 2 本は読まないことを実測）。この実装は表にだけ状態を持つので、ゲストが 8 バイトを読む経路があれば食い違いうる。
- 3 モジュールの測定は、ATTACH の途中（1708 件目の import の後）までで、SRW の他の 5 名と条件変数は**ここまで 1 回も呼ばれていない**。SRW の Shared、Sleep、Wake が実際に呼ばれたときの動作（特に Sleep の名前付き停止）は、3 モジュールの実行では確かめていない（合成バイト列のテストだけ）。
- 3 モジュールの基底と種 58 個は測定用の値で、決定ではない。
- 停止した `jmp r9`（`0x48a208b`）の原因は、生成器の `msvc_rejected` の文言からの読み（`data16 nop` の詰め物が `ja` と表を引く `mov` の間にある）で、`ja` を探す処理の中身は読んでいない（fe-decode のソースは読み取りのみで未読）。

## 9. 作ったファイル

- 新規（追跡対象）: `docs/decisions/0092-srw-locks-and-condition-variables.md`、`docs/investigations/blend2d-srw.md`（この文書）、`lift/crates/fe-lift/tests/srw.rs`。
- 変更（追跡対象）: `lift/runtime/fle_sys.c`、`lift/runtime/fle_sys.h`、`lift/crates/fe-lift/src/imports_table.rs`（末尾に 7 行）、`lift/crates/fe-lift/tests/modules.rs`（1 テストの書き換え）、`docs/README.md`（0092 の行）、`docs/investigations/provisional-decisions.md`（0092 の行と注記）。
- 作業物（`lift/out-srw/`）: `test-ws.txt`、`fle.c`、`fle-diff.txt`、`fle-lift.txt`、`a4s/`（単独構成）、`three/`（3 モジュール）、`gdb/ro_srw3.py`、`run/`（gdb のログ）、`target/`、`tmp/`、`fle_sys.c.good`（変異検査の前に保存した `fle_sys.c` のコピー）。
