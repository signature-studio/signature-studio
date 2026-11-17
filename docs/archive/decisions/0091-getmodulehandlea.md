# 0091: kernel32!GetModuleHandleA を、既存の GetModuleHandleW と同じモジュール表・同じハンドル・同じ規則で実装する（ANSI の名前は ASCII に限って広げる。0x80 以上は名前付き停止）

状態: **決定（仮ではない）**。PM の決定（2026-10-24）を、作業者（サブエージェント）が記録した。Learn の `GetModuleHandleA` のページの規則と、既存の `GetModuleHandleW`（決定 0015）の表の**延長だけ**で、新しい値を作らないので仮の決定にしない（下の「仮の決定にしない理由と、その限界」）。決定の節と背景は実装の前に保存し、「実装した場所」「テスト」「検証」「解釈が分かれた点」「限界」は実装と検証の後に書いた（すべて記入済み）。

事実の出典（節番号は各文書のもの）:

- `docs/decisions/0090-msvc-switch-base-dominance.md`（コミット b0dc3ab）の「検証」の最後の行、「限界」の最後。3 モジュール（種 58 個あり）の停止が `fle: unsupported instruction at 0x4899fed` に移った。
- `docs/investigations/blend2d-switch-dominance.md` §6.2、§6.3（停止、C のスタック、import の列）、`blend2d-switch-tables.md` §6.3。
- `docs/investigations/blend2d-lift-feasibility.md` §3.3、§5.2、B8（行なし 15 名の 1 つ）。
- `docs/investigations/blend2d-getmodulehandlea.md`（この作業の調査。以下「調査」）§2（引数の測定）、§3（Learn のページ）。
- 決定 0015（仮想モジュール表、GetModuleHandleW の規則）、0081（API セット名の仮想モジュール N = 27〜30）、0082（実在モジュール。DllMain 実行中の扱いの保留）、0084、0016（ASCII の枠）、0086、0088。

## 決定（PM。内容を変えない）

| # | 項目 | 決定 |
|---|---|---|
| 1 | `GetModuleHandleA` | `kernel32.dll!GetModuleHandleA` を実装する。**既存の `GetModuleHandleW` と同じモジュール表（`fle_modules[]`）、同じハンドル（`0x7FF800000000 + 0x10000 × N`）、同じ規則**。ANSI の名前を、決定 0016/0084 の枠で**ASCII に限って** wide に広げて既存の処理に渡す（または同等の共通化）。名前の比較は `GetModuleHandleW` が今行っている規則どおり（ASCII の英字を小文字に畳む、拡張子が無ければ `.dll` を足す、末尾の `.` は拡張子なしで取り除く） |
| 2 | 非 ASCII | **0x80 以上のバイトが来たら名前付き停止**（`GetModuleHandleW` は 0x7F を超える文字を「一致しない」として 126 を返すが、A は停止する。ANSI のコードページでの解釈と畳み込みは Learn が書かない） |
| 3 | NULL | `lpModuleName == NULL` は `0x400000`（`FLE_EXE_HANDLE`。`GetModuleHandleW(NULL)` と同じ。不変の制約）。last error は触れない |
| 4 | 表に無い名前 | NULL と `ERROR_MOD_NOT_FOUND`（126）。`GetModuleHandleW` が今答えるものと同じ。`GetModuleHandleW` が名前付き停止にする名前（パスを含む名前、256 文字を超える名前）は、A でも名前付き停止 |
| 5 | 実在モジュール（`fle_realmod`） | **`GetModuleHandleW` と同じ扱い**。ロード済みなら基底、未ロードなら「見つからない」（NULL と 126）。**DllMain 実行中の実在モジュールは「未ロード（0 と 126）」**（決定 0082 で保留のまま。**新しい扱いを作らない**） |
| 6 | 応答表 | `imports_table.rs` に `GetModuleHandleA` の行を足す（kernel32。`GetModuleHandleW` の行の書式に倣う）。新しい `.c`、`.h` は作らない |
| 7 | 範囲 | 実装するのは `GetModuleHandleA` **1 つだけ**。他の行なし import（SRWLock、条件変数、`CreateThread` など）は触らない |
| 8 | 状態 | 仮の決定ではない（Learn の規則と既存の表の延長）。引数が既存の仮想モジュール表に無い名前で、停止も応答もできない場合は、名前付き停止のままにして報告する |

## 背景（事実。出典の節番号）

- **停止点**（`blend2d-switch-dominance.md` §6.2、§6.3。決定 0090 の検証）: 3 モジュール構成（FLEngine + 縮小 dsp_ippv2 基底 0x2000000 + Blend2D 基底 0x4700000。種 58 個あり。測定用の値で決定ではない）の native の停止は `fle: unsupported instruction at 0x4899fed`（top of stack `f_4899d90`）。a4: init_count 150/942、last unit 149（proc 0x664260）、functions_distinct 1,058、function_entries 27,317、ATTACH 内の import 174 回。DllMain は戻らず、LoadLibraryW は 0x4700000 を返さない。
- **引数の測定**（調査 §2。実測）: `0x4899fed` は `call qword ptr [rip+0x1d0d5]` で、IAT スロット `0x48b70c8` = `KERNEL32.dll!GetModuleHandleA`。直前の `0x4899fe6: lea rcx,[rip+0x435d3]` で `rcx = 0x48dd5c0`。**`lpModuleName` は NULL ではなく、ANSI 文字列 `api-ms-win-core-synch-l1-2-0.dll`**（読み取り専用 gdb で停止時の rcx の指す文字列を読み、ファイルのバイトから読んだ値と一致。すべて ASCII、小文字、拡張子 `.dll` つき、パス区切りなし、32 文字）。この名前は `GetModuleHandleW` の規則では決定 0081 の仮想モジュール N = 27（ハンドル `0x7FF8001B0000`）。
- **戻り値の使い方**（調査 §2.3。objdump の読み）: `f_4899d90` は戻り値を `rbx` に保存し、`test rax,rax; je` で **NULL 検査**、非 NULL なら `GetProcAddress(rbx, "WaitOnAddress")`、`"WakeByAddressSingle"`、`"WakeByAddressAll"` の 3 回に hModule として渡し、結果をセル `0x48f3280`、`0x48f3288`、`0x48f3290` に保存する。3 つとも非 NULL なら `[0x48f31a0] = 1`、1 つでも NULL なら 0。ハンドル自体は保存しない。**停止時のこれらのセルはすべて 0**（gdb 読み取り。実測）。
- **この呼び出しだけ**: Blend2D の `.text` に `0x48b70c8` を参照する `call`/`jmp` は `0x4899fed` の 1 か所だけ（objdump の全 `.text` を `grep`。実測）。FLEngine と dsp_ippv2 は `GetModuleHandleA` を import していない（pefile。実測）。
- **Learn のページ**（調査 §3。取得した。実測）: <https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulehandlea>。`lpModuleName` について: 「If the file name extension is omitted, the default library extension .dll is appended. The file name string can include a trailing point character (.) to indicate that the module name has no extension. The string does not have to specify a path. … The name is compared (case independently) to the names of modules currently mapped into the address space of the calling process.」、「If this parameter is NULL, GetModuleHandle returns a handle to the file used to create the calling process (.exe file).」。戻り値: 「If the function succeeds, the return value is a handle to the specified module. If the function fails, the return value is NULL. To get extended error information, call GetLastError.」。Remarks: 「returns a handle to a mapped module without incrementing its reference count.」
- **ページが書かないこと**（この決定でも「書かれていない」と明記する）: (1) 失敗時の `GetLastError` の**値**（`ERROR_MOD_NOT_FOUND` とは書いていない。126 は決定 0015 の PM の決定を `GetModuleHandleW` と共有する）、(2) 空文字列の扱い、(3) パスを含む名前の照合規則、(4) 非 ASCII の文字の畳み込みと ANSI のコードページでの解釈、(5) 成功時に last error を触るか、(6) 名前の最大長、(7) DllMain 実行中（ロード中）のモジュールが見つかるか、(8) `/` を区切りとして受けるか。**A と W で規則が違うとはページは書いていない**（同じページで、`lpModuleName` の型だけが `LPCSTR` と `LPCWSTR`）。
- **既存の `GetModuleHandleW`**（`lift/runtime/fle_modules.c`。決定 0015 §3 の規則）: 上の決定の表の 1、3、4、5 のとおり。0082 の保留（DllMain 実行中の実在モジュールは未ロード扱い）は `fle_realmod.c` の `fle_realmod_module_handle`（ST_LOADING なら 0）が実装している。

## 範囲外（今回扱わない）

- 他の行なし import（`AcquireSRWLockExclusive` ほか。調査 B8 の残り 14 名）。
- `GetModuleHandleExA`（`GetModuleHandleExW` は決定 0078 で名前付き停止のまま）。
- ANSI のコードページ（cp1252 など）での 0x80 以上のバイトの解釈（決定 0084 は MultiByteToWideChar に限る。この決定は使わず停止する）。
- 決定 0082（DllMain 中の扱い）、0015、0055 の本文は書き換えない。

## 実装した場所

- `lift/runtime/fle_modules.c`: `fle_imp_GetModuleHandleW` の本体を `static void get_module_handle(const char *api, int wide)` に移した。`fle_imp_GetModuleHandleW` は `get_module_handle("GetModuleHandleW", 1)`、新しい `fle_imp_GetModuleHandleA` は `get_module_handle("GetModuleHandleA", 0)` の 1 行ずつ。同じ表、同じハンドル、同じ照合、NULL は `0x400000`、表に無い名前は NULL と 126、実在モジュールは `fle_realmod_module_handle`（ロード済みなら基底、DllMain 実行中と未ロードは 0）、パスと 256 文字超は名前付き停止。違いは (1) 名前を 1 バイト単位で読む（ASCII は同じ単位に広がる）、(2) **A は 0x80 以上のバイトで名前付き停止**（`fle: GetModuleHandleA with a byte above 0x7F (0x..) is not supported (the page does not define the ANSI code page folding; decision 0091)`。W は従来どおり「一致しない」で 126）、(3) 停止メッセージの API 名が関数名になる（W の文字列は従来と同一）。
- `lift/runtime/fle_modules.h`: `fle_imp_GetModuleHandleA` の宣言。
- `lift/crates/fe-lift/src/imports_table.rs`: `TABLE` の**末尾**に `GetModuleHandleA` の 1 行（既存の行の擬似 VA は動かない。決定 0015 の「末尾に足す」運用）。他の行は触っていない。
- 新しい `.c`、`.h` は作っていない。決定 0082、0015、0055 の本文は書き換えていない。

## テスト

`cargo test --workspace`: **97 binaries / 1,178 passed / 0 failed / 0 ignored**（基準 97 / 1,172 / 0 / 0。+6 は新規）。既存テストの期待値の変更は無い（「`GetModuleHandleA` は行なし／停止」を表明していたテストは `lift/` に無かった。`grep GetModuleHandleA` が 0 件）。

新規 6 件:

- `modules.rs` `get_module_handle_a_answers_like_w_on_every_ascii_row`: W のテストの ASCII の行（40 件超）と NULL を A と W で呼び、答えが一致し期待値と一致（ハンドルまたは `0 126`。成功時は last error が 5 のまま。NULL は `0x400000`）。
- `modules.rs` `get_module_handle_a_name_rules`: A の 14 名（Blend2D の引数 `api-ms-win-core-synch-l1-2-0.dll`、大文字、拡張子なし、末尾の `.`、`kernel32.dll.`、空文字列、`.`、表に無い名前、absent の `dwmapi`、近い名前、`avrt`、`PowrProf.dll`）。
- `modules.rs` `get_module_handle_a_stops_where_the_page_is_silent_and_w_does_not_stop_on_a_high_unit`: 0x80 以上のバイトで名前付き停止（SIGABRT、stderr の文言）、パス、256 文字超の停止。同じ文字を UTF-16 の単位にした W は停止せず `0 126`。
- `modules.rs` `get_module_handle_a_has_a_response_row_at_the_end_and_is_found_by_get_proc_address`: 生成器の `jmp [IAT]` が `fle_imp_GetModuleHandleA(); return;`、応答表の最後の行（既存の行の VA は動かない）、`GetProcAddress(kernel32, "GetModuleHandleA")` が擬似 VA を返し、`fle_dispatch` で A の答えになる、綴りを変えると NULL と 127。
- `realmod.rs` `get_module_handle_a_names_that_need_no_load_answer_like_w`: 実在モジュールの合成 PE のホストから、NULL、`Kernel32`、API セット名、ロード前の実在モジュール、存在しない名前を A と W で呼び一致（実在モジュールの表あり／なし）。
- `realmod.rs` `get_module_handle_a_of_a_real_module_follows_w_before_during_and_after_the_load`: 実在モジュールの名前を、ロード前（NULL と 126）、**DllMain の実行中（決定 0082 の保留のまま。A も W も 0 と 126）**、ロード後（基底。大文字、拡張子なし、末尾の `.`）で A と W に問う。

既存テストの書き換え（件数は増減しない）: `modules.rs` の `get_module_handle_name_rules` の `rows` を関数 `name_rows()` に切り出した（行の内容は変えていない）。`realmod_s1/` の合成 PE にラッパとシナリオを追加した（`shared.h`、`guest_host.c`、`guest_mod2.c`。既存のモード 1、2、8 とシナリオは変えていない。ステージ 1 の 34 件の検査は通った）。

## 検証

実測値と手順は `docs/investigations/blend2d-getmodulehandlea.md`。作業物は `lift/out-gmh/`。

| 項目 | 結果 |
|---|---|
| `cargo test --workspace` | **97 binaries / 1,178 passed / 0 failed / 0 ignored**（上のとおり。`SKIPPED` 0 件） |
| **FLEngine の生成 C の SHA-256** | **変わった**: `43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1` → **`eb9d8e7fa74ae562742fb1999f6da9b3ca82f2884d1845b5b7319153e1b709cf`**（215,707,512 B → 215,707,705 B）。変わったのは応答表の 5 か所 7 行だけ（`fle_imp_GetModuleHandleA` のプロトタイプ 1 行、`fle_dispatch_table` の末尾 1 行、`fle_dispatch_count` 59557 → 59558、`fle_resp_table` の末尾 1 行、`fle_resp_count` 260 → 261）。**変わった関数は 0 個**（その 5 か所を除くと新旧の C はバイト単位で同一）。functions 59,297、bad_nodes 0、unsupported_total 525、register_jmp_sites 20、jumptabs 4、msvc_tables 0、msvc_rejected 0 は不変。原因は「応答表に行を足したこと」だけ（FLEngine の jmp thunk は `GetModuleHandleW` の 2 か所で、`GetModuleHandleA` を import していない。決定 0078 の前例と同じ形）。**基準の SHA を固定するテストは無い**ので、期待値の更新は無い。今後の基準は `eb9d8e7f…` |
| 単独構成の a4（新しい OUT で runtime の .o を最初から作り直した） | **145 / 942**、functions 821、entries 15,685、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`。基準と一致。2 回同一 |
| 3 モジュール（種 58 個あり、基底 0x4700000。測定のみ） | `GetModuleHandleA` は通り（`0x4899fed` の呼び出し 1 回、引数 `api-ms-win-core-synch-l1-2-0.dll`、戻り値 N = 27 のハンドル）、続く `GetProcAddress`（`WaitOnAddress`、`WakeByAddressSingle`、`WakeByAddressAll`）は 3 回とも NULL。**次の停止は `fle: unsupported instruction at 0x489cab9`**（top of stack `f_489caa0`。IAT スロット `0x48b7098` = `KERNEL32!AcquireSRWLockExclusive`。行なし import の 1 つ、B8）。a4: init_count 150/942（変わらず）、last unit 149（proc 0x664260）、functions_distinct 1,059（前回 1,058）、function_entries 27,319（前回 27,317）、ATTACH 内の import 178 回（前回 174）。DllMain は戻らず、LoadLibraryW は 0x4700000 を返さない。分類 (i) 未応答の import。2 回同一。unit 158 には届いていない |

## 仮の決定にしない理由と、その限界

- Learn のページ（2025-07-01 更新）が書く規則と、既存の `GetModuleHandleW`（決定 0015）の表・ハンドル・規則の**延長だけ**。新しい値を作っていない（ハンドルは `fle_modules[]` の位置から決まる既存の式、失敗は決定 0015 の NULL と 126）。引数 `api-ms-win-core-synch-l1-2-0.dll` は既存の仮想モジュール N = 27（決定 0081。これ自体は仮の決定）の名前で、停止も応答もできない名前ではないので、手順どおり「仮の決定にしない」。
- ただし、**この決定の前提に仮の決定 0081 がある**: `GetProcAddress` が 3 つとも NULL になるのは 0081 の「モジュールはあるが関数は無い」の仮の扱いによる。0081 の撤回条件が満たされると、この決定の結果（Blend2D の次の経路）も変わりうる。この決定の本文は 0081 の仮を引き継がない（GetModuleHandleA の規則そのものは 0081 に依存しない）。
- 表に無い実機の DLL 名は NULL と 126（決定 0015 から続く W と同じ扱い。実機では存在する DLL を「無い」と答える）。ここは**この決定で新しく作った扱いではなく**、W と同じにするという PM の決定のとおり。

## 解釈が分かれた点

詳細は調査 `docs/investigations/blend2d-getmodulehandlea.md` §8。決定の内容は変えていない。実装できなかった項目は無い。

1. **非 ASCII の A と W**: 決定の表の項目 1（比較は W と同じ）と 2（0x80 以上は停止）は、非 ASCII について食い違って見える。項目 2 を「ASCII の範囲の比較規則（畳み込み、`.dll`、末尾の `.`）は W と同じ。非 ASCII だけは A が停止し、W は従来どおり 126」と読んだ。W は変えていない。
2. **ページが書かない入力のうち W が停止しないもの**（空文字列、`.`）: W と同じ 126（停止しない）。「ページが書かない入力は名前付き停止にする」という一般則より、決定の表の項目 4（W と同じにする）を優先した。W が既に答えている入力に新しい扱いを作らないため。W が停止する入力（パス、256 文字超）は A も停止する。
3. **共通化の形**: W の本体を `static` 関数に移し、`wide` の 0／1 で呼ぶ形。ANSI を wide の一時配列に広げて W を呼ぶ形は、`R[1]` の差し替えと停止メッセージの名前の違いのため採らなかった。W の停止メッセージは従来と同一の文字列。
4. **DllMain 実行中の実在モジュール**: 新しい扱いを作らず、決定 0082 の保留（未ロード = 0 と 126）のまま。A と W が同じ値を返すことをテストで確認した。
5. **`gen_two.sh` の FLEngine の SHA 検査**: 既定は基準 `43985d62…` なので、`EXPECT_FLE_SHA` に新しい SHA を渡した（スクリプトは変えていない）。

## 限界

- **実機の Windows での挙動は未確認**（ホストは Linux）。失敗時の last error 126 は決定 0015 の PM の決定。ページは値を書かない。
- 非 ASCII の A の入力は名前付き停止で、ANSI コードページでの解釈は実装していない。Blend2D の `0x4899fed` の引数は ASCII なので、この停止は Blend2D の経路では起きない（実測）。
- `fle_modules[]` に無い名前は A も W も NULL と 126 で、実機では存在する DLL を「無い」と答えうる（決定 0015 からの仮定）。
- ロード中の実在モジュールを「未ロード」とする扱いは決定 0082 の保留で、実機の挙動は未確認。
- `GetModuleHandleExA`、他の行なし import（SRWLock 系ほか 14 名）は実装していない。3 モジュールの次の停止は `AcquireSRWLockExclusive` で、**直していない**。
- Blend2D の `GetModuleHandleA` は 1 か所だけ（`.text` の `grep`）。他の同梱 DLL は調べていない。
- 3 モジュールの測定の基底（`0x2000000`、`0x4700000`）と種 58 個は測定用の値で、決定ではない。`GetProcAddress` が NULL になるのは決定 0081 の仮の扱い。この結果で「Blend2D の ATTACH が WaitOnAddress を使わない経路に入る」とまでは言えない（`[0x48f31a0]` を読む側は追っていない）。
