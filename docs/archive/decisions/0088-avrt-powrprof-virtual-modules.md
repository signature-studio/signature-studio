# 0088: avrt.dll と powrprof.dll を仮想モジュール（N = 31、32）にする（決定 0015、0055、0081 の延長。仮の決定）

状態: PM の決定（2026-10-24）を、作業者（サブエージェント）が記録した。**仮の決定**（実機の Windows 10 22H2 で未確認。撤回条件は下）。**実装した**（決定の節は実装の前に保存し、「実装した場所」「テスト」「検証」「解釈が分かれた点」は実装と検証の後に書いた）。事実の出典は `docs/investigations/windows-dll-names-virtual-or-absent.md`（以下「調査」。節番号はその文書のもの）。この記録は調査に無い主張を足さない（調査に無い数値は、この作業で実測したと明記する）。決定 0015（仮想モジュール表。「必要が出たら末尾に足す」）、決定 0029（`fle_load_absent[]` の 9 名。「足す場合は PM の決定が先」）、決定 0055（msctf.dll を仮想モジュールにした先例。N = 26）、決定 0081（api-ms の 4 名を N = 27〜30 に足した先例。GetProcAddress は NULL と 127）に従う。0015、0029、0055 の本文は書き換えず、各 1 行の注記を足した（0083 が 0018 に付けた注記に倣う）。

## 決定（PM。内容を変えない）

| # | 項目 | 決定 |
|---|---|---|
| 1 | `avrt.dll` | 仮想モジュール表 `fle_modules[]` の **N = 31**（位置 index 30）に足す。`LoadLibraryW` は成功し、仮想ハンドル `0x7FF800000000 + 0x10000 × 31` = `0x7FF8001F0000` を返す。ハンドルの割り付け規則は既存どおり（決定 0015、0055、0081: 末尾に足す、並べ替えない） |
| 2 | `powrprof.dll` | 同じく **N = 32**（index 31）。ハンドル `0x7FF800000000 + 0x10000 × 32` = `0x7FF800200000` |
| 3 | `GetProcAddress` | 既存の規則どおり。応答表の `dll` 欄がそのモジュール名の行が無い名前は、**NULL と ERROR_PROC_NOT_FOUND（127）**。**応答表に行は足さない**。調査 §3.1 が挙げる avrt の 11 名と PowrProf の 2 名は、すべて NULL と 127 になる |
| 4 | FLEngine の生成 C | 変わらない見込み（PM）。基準の SHA-256 は `43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1`。変わったら止めて報告する（実測は下の「検証」） |
| 5 | 決めないもの | 下の「範囲外」 |
| 6 | 状態 | 仮の決定。撤回条件は下 |

## 背景（事実。調査の節番号）

- 現状は、2 モジュール構成で DllMain(ATTACH) が戻った後、実行は unit 145 の `f_65c9f0` で `LoadLibraryW("avrt")` に届き、`fle_load.c` の名前付き停止になる（`docs/investigations/two-module-after-0087.md` §3。実測）。調査 §1 の 1 は、12 名のうち DllMain の後に実行が届くのは avrt と PowrProf.dll の 2 名で、同じ関数 `f_65c9f0` が dwmapi、shcore と並べて引く、と書く（読み（静的）。順序は前回の仮の値つき gdb 測定の引用）。決定 0086 は、この 2 名の停止を「unit 149 より前に現れる別件」として扱わなかった。
- ゲストの呼び出し（調査 §3.1 の 1、2。読み）: `avrt` は 0x65ccd1 の `LoadLibraryW(L"avrt")`（`f_65c9f0`）で、結果をセル 0x15f4288 に保存し、直後の 0x65ccdd で NULL を検査する。非 NULL なら 11 名（`AvSetMmMaxThreadCharacteristicsW`、`AvSetMmThreadCharacteristicsW`、`AvSetMmThreadPriority`、`AvQuerySystemResponsiveness`、`AvRevertMmThreadCharacteristics`、`AvRtCreateThreadOrderingGroup`、`AvRtCreateThreadOrderingGroupEx`、`AvRtDeleteThreadOrderingGroup`、`AvRtJoinThreadOrderingGroup`、`AvRtLeaveThreadOrderingGroup`、`AvRtWaitOnThreadOrderingGroup`）を `GetProcAddress` で引く。`PowrProf.dll` は 0x65cf38 の `LoadLibraryW(L"PowrProf.dll")` で、結果をセル 0x15f42a8 に保存し、0x65cf44 で NULL を検査する。非 NULL なら 2 名（`PowerGetActiveScheme`、`PowerSetActiveScheme`）を引く。
- Windows 10 にあるかの導出（調査 §3.2 の 1、2。実測（Learn 取得）＋導出）: avrt は `req.target-min-winverclnt: Windows Vista [desktop apps only]`、`req.dll: Avrt.dll`（3 ページ）。powrprof は Vista、`req.dll: PowrProf.dll`（2 ページ）。**ページは Windows 10 とは書かない**。Windows 10 にあると**導出**したもので、決定 0055 の msctf.dll と同じ扱い。avrt の残り 8 名のページは取得していない（調査 §3.2 の 1、§3.2 末尾の「未取得」）。
- 応答表の行は 0（調査 §3.2。実測）: 12 名の DLL 名は `imports_table.rs` の `dll` 欄に 1 件も無く、`fle_imp_*` も無い。仮想にして行を足さなければ、全 `GetProcAddress` が NULL と 127（調査 §1 の 5）。
- 仮想にした場合と absent にした場合（調査 §4.1、§4.2、§6.2。読み）: どちらも、avrt の 11 スロットと PowrProf の 2 スロットは 0 のまま。違うのは、ハンドルのセルが非 0（仮想）か 0（absent）かだけ（§4.1「どちらでも同じ状態」）。どちらも「進む」。

## 理由（PM。調査の事実だけを引く）

- **Windows 10 にあると導出できる**: avrt と powrprof は、Learn の Requirements 欄から Windows 10 にあると導出できる（上の背景。決定 0055 の (c) と同じ導出）。ページが Windows 10 にあると書いているわけではない。
- **absent は環境についての主張になる**: absent にすると「この環境にこの DLL は無い」と主張することになり、決定 0055 の (c)（環境の定義 0014、Windows 10 22H2 と矛盾しない）を一次資料で確認できない（決定 0081 の (b) の却下理由と同じ向き）。
- **ゲストは NULL でも続行する形に読める**（読み。**実測ではない**）: avrt の使い手 4 関数（`f_bb0920`、`f_12b00f0` は呼ぶ前にスロットを検査、`f_bb1520`、`f_12b0190` は `[obj+0x48]` が非 0 のときだけ呼ぶ。調査 §4.1）、powrprof の使い手 `f_65c6c0` は 0x65c6ca でスロット 0 なら何もせず戻る（調査 §4.2）。
- **仮想にしても 0 番地を呼ぶ経路は、読んだ範囲で無い**（調査 §4.1、§4.2、§5）。ただし**avrt の 8 スロット（`AvSetMmMaxThreadCharacteristicsW`、`AvQuerySystemResponsiveness`、`AvRtCreateThreadOrderingGroup`、`AvRtCreateThreadOrderingGroupEx`、`AvRtDeleteThreadOrderingGroup`、`AvRtJoinThreadOrderingGroup`、`AvRtLeaveThreadOrderingGroup`、`AvRtWaitOnThreadOrderingGroup`）は読み手が見つからず、検査の有無は未確認**（調査 §4.1 末尾、§8 の 2。走査は rip 相対とデータセルだけで、基底＋オフセットで読む形は走査していない。見つからないことは「検査される」ことを意味しない）。

## 選択肢

| | 内容 | 結論 |
|---|---|---|
| (a) | 仮想モジュール表の N = 31、32 に足す。`LoadLibraryW` は成功。応答表の行が無い API は NULL と 127 | **採る** |
| (b) | `fle_load_absent[]` に足す（NULL と 126） | 採らない（理由は上の「absent は環境についての主張になる」） |
| (c) | 何もせず名前付き停止のまま | 採らない。現状は unit 145 の `LoadLibraryW "avrt"` で止まる（実測。`two-module-after-0087.md` §3） |

## 限界: 実機の再現ではない（PM の決定 3）

- 仮想モジュールにして `GetProcAddress` が NULL を返すと、実機の Windows 10（関数が存在し、`GetProcAddress` が非 NULL を返す）とは、**ゲストが違う経路（機能が無い経路）を通る**。これは決定 0081（api-ms）と同じ種類の限界である。「モジュールはあるが関数は無い」環境は実機と一致しない。
- absent も実機と一致しない（DLL があるのに無いと主張する）。
- **どちらも実機の再現ではなく、先へ進むための仮の選択**である。
- avrt と powrprof の関数を実装して、ゲストを実機の経路に合わせるのは**別の決定**である（この決定では行わない。応答表に行を足さない）。

## 範囲外（決めないもの。PM）

- **残りの 10 名**: `psapi`、`riched20`、`urlmon`、`ws2_32`、`mswsock`、`iphlpapi`、`wship6`、`fwpuclnt`、`idndl`、`normaliz`。表にも `fle_load_absent[]` にも足さない。名前付き停止のまま（`fle_load.c` の case 3）。
- **dwmapi と shcore**: 決定 0015、0029 のとおり `fle_load_absent[]`（NULL と 126）のまま変えない。`fle_load_absent[]` の 9 名は、この決定で追加も削除もしない。avrt と powrprof は 0015 の本文の 9 名にも `fle_load_absent[]` にも入っていなかった（確認した: 0015 の 22 行と `fle_load.c` の `fle_load_absent[]` は同じ 9 名。0029 は 54、69、71 行で avrt、powrprof を「止める」扱いと書く）ので、absent の名簿は変わらない。
  - 結果として、同じ関数 `f_65c9f0`（unit 145）の中で、dwmapi と shcore は absent、avrt と PowrProf は仮想モジュールになる（調査 §5 の最後の段落の順。調査 §9 の 2 も同じ形を指摘する。実行は通るが、環境の主張としては揃わない）。揃えるかは別の決定。
- **決定 0015、0029 と 0055 の基準の違い**: 解消しない。調査 §2.3 のとおり、**どの決定も相手の基準で書き直されていない**（0015/0029 は名簿方式、0055 は (a)(b)(c) の確認方式。0055 の (c) を dwmapi、shcore に当てると同じ導出で「Windows 10 にある」側に入る）。矛盾と呼ぶかは未決で、この決定は呼び分けない。
- avrt、powrprof の関数の実装、応答表の行（`fle_imp_*`）の追加。
- 実機の Windows 10 22H2 での確認。

## 撤回条件

- 実機の Windows 10 22H2 で `LoadLibraryW(L"avrt")`、`LoadLibraryW(L"powrprof")` が失敗すると確認されたとき（その場合は absent が実機と一致する）: 該当の名前を `fle_modules[]` から外し、`fle_load_absent[]` へ移すかを PM が判断する。表の途中の名前を外すと後ろのハンドルが変わる（決定 0015）ので、N = 32 を外すのは末尾なので他のハンドルは変わらないが、N = 31 だけを外すと N = 32 のハンドルが変わる。その扱いはその時に決める（決定 0081 の撤回条件と同じ形）。
- FL が実機で avrt、powrprof の関数を使う経路に入り、NULL の経路を通ると実機と違う可視の挙動になると確認されたとき。
- PM が環境の定義（決定 0014）を変えたとき。

## 実装した場所

- `lift/runtime/fle_modules.c`: `fle_modules[]` の末尾に `"avrt.dll"`（N = 31）、`"powrprof.dll"`（N = 32）の 2 行と、決定 0088 を指すコメント。0081 の実装（N = 27〜30）と同じ形で、名前の照合（大文字小文字を畳む、`.dll` が無ければ足す）、`LoadLibraryW`／`LoadLibraryA`／`LoadLibraryExW` のロード（参照 +1）、`GetModuleHandleW`、`GetProcAddress`（応答表に行が無いので NULL と 127）、`FreeLibrary`（参照数 1 で名前付き停止）は既存の処理のまま。`fle_modules.h`、`fle_load.c`、`fle_realmod.c`、`fle_locale.c`、`fle_reg.c`、`imports_table.rs` は変えていない。新しい .c、.h は作っていない。
- `fle_load_absent[]`（`fle_load.c`）は変えていない。`fle_load.c` の同配列の上のコメント（avrt、powrprof を「止まる」例に挙げる 50〜51 行）は、この決定で実態と食い違うが、本文の注記の範囲外（編集してよいファイルの指定に無い）として**書き換えていない**（報告に記載）。
- ハンドル: N = 31 は `0x7FF8001F0000`、N = 32 は `0x7FF800200000`。`LOAD_MODULES_MAX 64`（`fle_load.c`）に収まる（32 ≤ 64）。応答表の擬似 VA の領域 `0x7FF900000000` 以降とは重ならない（算術。調査 §7.3）。

## テスト

既存テストの固定数の更新（行番号は変更後のファイル。変更前後と理由は「テストの変更」）と、新規 5 件。`cargo test --workspace`: **95 バイナリ、1,130 件通過、0 失敗、0 無視**（基準 95 / 1,125 / 0 / 0。+5 は新規の 5 件。既存テストは増減していない）。

新規（`#[test]` 5 件）:

- `load.rs` `avrt_and_powrprof_load_by_name_and_get_module_handle_gives_the_same_handle`: `LoadLibraryW` の 8 通りの綴り（`avrt`、`avrt.dll`、`AVRT`、`Avrt.DLL`、`PowrProf.dll`、`powrprof`、`POWRPROF.DLL`、`PowrProf`）、`LoadLibraryA`、`LoadLibraryExW`（0x800 と 0）が N = 31／32 のハンドルを返し、last error は触れない（5 のまま）。`GetModuleHandleW` が同じハンドル。
- `load.rs` `avrt_and_powrprof_have_nothing_to_find_and_dwmapi_and_shcore_stay_absent`: `GetProcAddress`（`AvSetMmThreadCharacteristicsW`、`PowerGetActiveScheme`、取り違えた組）は NULL と 127。同じ本文の中で `dwmapi` と `SHCore.dll` は NULL と 126（absent のまま）。ハンドルは `fle_module_handle(30)`、`(31)` と一致。
- `load.rs` `avrt_and_powrprof_follow_the_free_library_rule_of_the_other_virtual_modules`: 参照数の初期値 1、LoadLibrary で +1、`FreeLibrary` で −1 と戻り値 1（last error 5 のまま）。参照数 1 の `FreeLibrary`（avrt、powrprof）と、LoadLibrary と FreeLibrary を釣り合わせた後の `FreeLibrary` は「the reference count would reach zero」で停止（決定 0082 D11 のとおり）。新しい表の外（N = 33）は「not a virtual module handle」で停止。
- `load.rs` `the_other_ten_windows_dlls_still_stop_by_name_after_decision_0088`: 範囲外の 10 名（`psapi`、`riched20`、`urlmon`、`ws2_32`、`mswsock`、`iphlpapi`、`wship6`、`fwpuclnt`、`idndl`、`normaliz`）が `module "<名前>.dll" is not a virtual module` で停止。
- `msctf.rs` `avrt_and_powrprof_are_modules_with_no_row_in_the_real_response_table`: 生成器の本物の応答表（`("kernel32.dll", "GetProcAddress")` ほかを import する合成 PE から作る。応答表に N = 31、32 の行が無いこと）をリンクしたドライバで、`LoadLibraryW` が N = 31、32 のハンドル、`GetModuleHandleW` が同じハンドル、調査 §3.1 の avrt の 11 名と PowrProf の 2 名がすべて「missing 127」、他の DLL の行の名前（`GetLastError`、`SetInputScope`）や取り違えた名前も「missing 127」、`dwmapi` と `shcore` は 0 と 126。

`modules.rs` の既存の 2 件（`get_module_handle_name_rules`、`module_handles_follow_the_formula_and_are_distinct`）には、新しい行を足した（件数は増えない）: `avrt`／`AVRT.DLL`／`Avrt.dll` が index 30、`powrprof`／`PowrProf.dll`／`POWRPROF` が index 31、近い名前（`avrt2.dll`、`avrt.dll.dll`、`powrprof2`）と範囲外の名前（`psapi`、`ws2_32.dll`）と absent の名前（`shcore.dll`、`dwmapi`）は見つからない（126）。表の 31、32 番目の名前が `avrt.dll`、`powrprof.dll`。

**変異検査**（この作業の自己検査。決定の根拠ではない）: ランタイムの作業コピー 2 通り（`avrt.dll` と `powrprof.dll` の順序の入れ替え、`powrprof.dll` の削除）に対して `msctf.rs` を実行し、どちらも少なくとも 1 件が落ちた（入れ替えでは新規の 1 件、削除では `msctf_is_the_26th_virtual_module` と新規の 1 件）。元の `runtime/` は変えていない。

### テストの変更（既存テストの行。行番号は変更後）

行番号は変更後のファイルのもの（`git diff -U0` の `@@` と `grep -n` で確認した）。

| ファイル:行 | 変更前 → 変更後 | 理由 |
|---|---|---|
| `modules.rs:214` | コメント `* 30` → `* 32` | 擬似 VA がハンドル領域より上、の説明。最後のハンドルが N = 32 になる |
| `modules.rs:217` | `0x7FF8_0000_0000 + 0x10000 * 30` → `… * 32` | 同上。応答の擬似 VA が最後のハンドル（N = 32）より上であることを検査 |
| `modules.rs:337`〜`351`（`get_module_handle_name_rules` の `rows` の末尾） | 15 行を足す（コメント 2 行と、avrt 3、powrprof 3、近い名前 3、範囲外 2（`psapi`、`ws2_32.dll`）、absent 2（`shcore.dll`、`dwmapi`）の 13 行） | 新規の名前の検査。既存の行は変えない（`#[test]` の件数は増えない） |
| `modules.rs:390`〜`392`（`module_handles_follow_the_formula_and_are_distinct`） | コメントに avrt、powrprof を足す。`count 30` → `count 32`、`lines[1..31]` → `lines[1..33]` | 表が 32 件になる |
| `modules.rs:398`〜`400`（同上） | `lines[31]` が nil、の 1 行 → `lines[31]`、`lines[32]` の名前（`avrt.dll`、`powrprof.dll`）の 2 行と、`lines[33]` が nil の 1 行 | 表の終わりが 2 つ後ろになる |
| `modules.rs:547`（`undefined_cases_abort` の `handle-past-table`） | `h(30)` → `h(32)` | このファイルの `h(i)` は **0 始まりの index**（`handle_of(index)` = `BASE + STEP × (index + 1)`）。`h(30)` は N = 31、つまり 30 件の表の外の最初のハンドルだった。32 件になった後の表の外の最初は index 32（N = 33）なので `h(32)`。**この行は変わらず「表の外のハンドルで abort する」を検査する**（テストが通ったことで確認。もし表の中なら abort せずテストが落ちる）。`load.rs`、`msctf.rs` の `h(n)` は 1 始まりの N で、`modules.rs` と読み方が違う |
| `msctf.rs:258` | `fle_module_name(30)` → `(32)` | 0 始まりの index。表の外が NULL の期待。表が 32 件になる |
| `msctf.rs:282`（変更前 269） | シナリオ名と呼び出し `handle-31`、`M(31)` → `handle-33`、`M(33)` | `M(n)` は N（1 始まり）。N = 31 は表の中になったので、表の外の最初（N = 33）を指す |
| `msctf.rs:355`（変更前 342） | `count 30` → `count 32` | 表が 32 件になる |
| `msctf.rs:455`（変更前 429） | `aborts("handle-31", …)` → `aborts("handle-33", …)` | 282 行と同じシナリオの呼び出し |
| `load.rs:446` | `fle_load_refcount(30)` → `(32)` | 0 始まりの index。表の外の参照数が 0 の期待。表が 32 件になる |
| `load.rs:525`〜`526`（コメントの追加）と、変更前 533〜534 の 2 行の削除 | `external_and_unnamed_modules_stop_by_name` の名前リストから `"avrt"`、`"PowrProf.dll"` を削除（14 名 → 12 名）。コメント 2 行を足す | この 2 名は仮想モジュールになり、停止しない。代わりに新規テストが成功を検査する |
| `event.rs:261`、`glob.rs:205`、`vol.rs:41`、`vfs.rs:33` | `mods_end = … * 30` → `* 32` | 他のトークン領域がモジュールのハンドルより上にある、という不等式。最後のハンドルが N = 32 になったので実態に合わせた（`* 30` のままでも不等式は成り立つ。算術） |

`*` の上限を 30 から 32 に合わせる更新（`modules.rs:214`、`217`、`event.rs`、`glob.rs`、`vol.rs`、`vfs.rs`）は、不等式の端を実態の最後のハンドルに合わせるもので、検査の強さは変わらない。`#[test]` の数を変えたのは新規の 5 件だけで、既存の `#[test]` は消していない。

## 検証

- `cargo test --workspace --no-fail-fast`（`FLENGINE_PE=binaries/win64/FLEngine_x64.dll`）: **95 バイナリ、1,130 件通過、0 失敗、0 無視**（基準 1,125 から +5）。出力は `lift/out-avrt88/test-ws.txt`。
- FLEngine の生成 C: `fe-lift lift ../binaries/win64/FLEngine_x64.dll out-avrt88/fle.c --pdata` の SHA-256 は **`43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1`**（基準と一致。215,707,512 B、`unsupported_total` 525）。2 モジュール構成の `gen_two.sh` も同じ値で検査を通った。**変わらなかった**（PM の決定 4 のとおり）。
- 単独構成（FLEngine だけ。新しい OUT で `fle_modules.o` を含む runtime を最初から作った）: `init_count 145 / 942`、functions 821、entries 15,685、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`。**基準と一致**（単独構成は avrt の手前の unit 144 で止まるので、この決定は効かない）。
- 2 モジュール構成の native 測定: `docs/investigations/two-module-after-0088.md`。要約: DllMain(ATTACH) は eax = 0x1 で戻り、`avrt`（`0x7ff8001f0000`）と `PowrProf.dll`（`0x7ff800200000`）の `LoadLibraryW` は成功し、avrt の 11 名と PowrProf の 2 名の `GetProcAddress` はすべて NULL。`dwmapi` と `SHCore.dll` は NULL（absent）。a4 は `init_count` 146 → **150 / 942**、次の停止は `LoadLibraryW "blend2d.dll"`（unit 149、決定 0086 の対象）。

## 解釈が分かれた点

1. **`modules.rs` の `handle-past-table` の書き換え方**: 調査 §7.1 の表（`modules.rs:530`）は「`h(最終 N + 1)` へ要更新」と書く。`h` の読み方がファイルで違い、この行を「N + 1」と文字どおりに読むと `h(33)` になりうる。`modules.rs` の `h(i)` は 0 始まりの index（`BASE + STEP × (i + 1)`）なので、表の外の最初のハンドル（N = 33）は `h(32)` である（元の `h(30)` が N = 31、30 件の表の外の最初だったのと同じ形）。`h(32)` を採った。決め手は、`GetProcAddress` の範囲検査 `(h − BASE) / STEP > FLE_MODULE_N`（`fle_modules.c`）と、テストが通ったこと（`h(32)` が表の中なら abort せず、`undefined_cases_abort` は「abort を期待」で落ちる）。調査の記述の文字どおりではなく、元のテストの意図（「表の外の最初のハンドルで abort」）に合わせた。
2. **新規テストの置き場所**: PM の指定は「新規テスト」のみで、置き場所は指定が無い。`load.rs`（ロード、`GetModuleHandleW`、`GetProcAddress` の NULL と 127、`FreeLibrary`、範囲外の 10 名）と `msctf.rs`（生成器の本物の応答表をリンクしたドライバ）に置いた。`modules.rs` には新しい `#[test]` を足さず、既存の 2 件に行を足した。0081 の実装が `modules.rs` の既存の行を直し、`realmod.rs` に新規のテストを足したのとは置き場所が違う（`realmod.rs` は実在モジュールの機構のテストで、仮想モジュールの追加には使わなかった）。
3. **`fle_load.c` のコメント（50〜51 行）**: avrt、powrprof を「止まる」名前として挙げている。この決定で実態と食い違うが、PM が編集を許したファイルに入っていない（`fle_load.c` は読み取りのみ）ので書き換えていない。次の作業で直すかは未決。
4. **0015、0029、0055 の注記の位置**: PM の指示は「該当行に 1 行ずつ追記」。0015 は 22 行（遅延 import だけの 9 DLL）の直後、0029 は 71 行（avrt ほかに「存在しない」を返す案を採らなかった段落）の直後、0055 は 26 行（「`fle_load_absent[]`（決定 0029 の 9 名）は変えない」）の直後に足した（下の「注記」）。

## その他の限界

- ハンドルは実メモリの住所ではない（決定 0015）。ゲストが仮想ハンドルからヘッダを読むと SIGSEGV になる。調査 §8 の限界は、avrt と powrprof の経路がハンドルを `GetProcAddress`、`FreeLibrary`、NULL 検査にだけ使うと読む（読み）ので、この限界に当たる経路は見つけていない、と書く。
- **実機の Windows 10 22H2 では未確認**（ホストは Linux）。Learn が「Windows 10 22H2 に avrt.dll、powrprof.dll が存在する」と明記したページは無い（導出のみ。調査 §8 の 5）。
- ゲストの振る舞い（NULL でも続行する、0 番地を呼ぶ経路が無い）は**すべて読み**で、実行して確かめていない（調査 §8 の 1）。avrt の 8 スロットの検査は未確認（上の「理由」）。
- 終了側 `f_65c750` は avrt のハンドルを `cmp [0x15f4288],0; je` の後で `FreeLibrary` する（調査 §4.1）。仮想の場合、`FreeLibrary(仮想ハンドル)` は `fle_load.c` の参照数が `LoadLibraryW` で 2 になっているので 2 から 1 になり停止しない（読み。`FreeLibrary` に届くかは未確認）。決定 0082 D11（参照 0 になる `FreeLibrary` は名前付き停止）は変わらない。
- Windows 本体との照合はしていない。
