# 0015: 仮想モジュール表（GetModuleHandleW、GetProcAddress）

状態: PM が決定（方針）。仕様の読みと実装は作業者（M1 作業者）。仕様の出典と読みは `docs/investigations/import-GetModuleHandleW.md`、`import-GetProcAddress.md`。実行記録は `docs/investigations/modules-run.md`。

## 決定（PM。変更しない）

- GetModuleHandleW と GetProcAddress は、仮想モジュール表で実装する。DLL のローダーは作らない。
- 既知のモジュール名（PE の import 表にある 24 DLL、ntdll.dll）は、DLL ごとに別の固定ハンドルを返す。名前は大文字小文字を区別せず比較する（仕様）。ハンドル値は `0x7FF800000000 + 0x10000 × N`。
- GetProcAddress は、応答表にある API は、その応答を呼ぶ擬似アドレスを返す。応答表にない API は NULL を返し、last error を ERROR_PROC_NOT_FOUND（127）にする。存在すると偽らない。
- 未知のモジュール名は NULL と ERROR_MOD_NOT_FOUND（126）。
- GetModuleHandleW(NULL) は、呼び出しプロセスの実行モジュールのハンドルを返す（仕様）。この環境にはリフトした DLL しかないので、DLL のベース 0x400000 を返す。
- 擬似アドレスは `fle_dispatch` から呼ばれる形にする。

## 作業者の判断（PM の決定の範囲内の細部）

### 1. N の割り当て

`lift/runtime/fle_modules.c` の `fle_modules[]` の並び順（1 始まり）が N。順序がハンドルを決めるので、追加は末尾に限り、並べ替えない（コメントに書いた）。

- N = 1〜24: PE の import 表（"normal" の 24 DLL）。小文字、拡張子つき。`advapi32.dll`、`api-ms-win-crt-string-l1-1-0.dll`、`comctl32.dll`、`comdlg32.dll`、`crypt32.dll`、`dsound.dll`、`gdi32.dll`、`imagehlp.dll`、`kernel32.dll`（N=9、0x7FF800090000）、`mpr.dll`、`msacm32.dll`、`oleacc.dll`、`ole32.dll`、`oleaut32.dll`、`opengl32.dll`、`shell32.dll`、`shfolder.dll`、`shlwapi.dll`、`user32.dll`（N=19）、`version.dll`、`wininet.dll`、`winmm.dll`、`winspool.drv`、`wintrust.dll`。
- N = 25: `ntdll.dll`。PE の import 表にはない。依頼が名前を挙げたので入れた（Windows のプロセスには常にある、という理解は推測。この作業では確認していない）。
- 遅延 import だけの DLL（dwmapi、imm32、msimg32、propsys、rpcrt4、shcore、uxtheme、windowscodecs、wtsapi32）は表に入れない。「PE の import 表にある 24 DLL」の「normal」を選んだ。これらの名前を GetModuleHandleW に渡す呼び出しが出たら、126 を返す（今はロードされていない扱い）。必要が出たら末尾に足す。
  - 注記（決定 0088、仮の決定）: `avrt.dll` と `powrprof.dll` は、この行の 9 DLL にも `fle_load_absent[]` にも入っていなかった（absent の名簿は変わらない）。0088 は 2 名を仮想モジュール表の末尾（N = 31、32）に足した。この 9 DLL（dwmapi、shcore を含む）は 0088 でも absent のまま変えていない。この行の基準（import 表で区切る）と決定 0055 の基準（(a)(b)(c)）の違いは解消していない（0088 の「範囲外」）。この行の本文は書き換えていない。
- テスト `every_pe_import_dll_is_a_module` が、実 DLL から読んだ 24 DLL が全て引けて、ハンドルが全て違うことを確かめる。

### 2. GetModuleHandleW(NULL) = 0x400000

実行ファイルは存在せず、FLEngine_x64.dll が ImageBase 0x400000（`objdump -p`）に載る。呼び出しプロセスを作ったファイルのハンドルは、通常の Windows では .exe のベースだが、この環境ではリフトした DLL のベースを返す。値は `FLE_EXE_HANDLE`（`fle_modules.h`）。このハンドルに対する GetProcAddress は abort（DLL 自身のエクスポートは未モデル化）。

### 3. 名前の規則（仕様のページから読める範囲）

- ASCII の英字を小文字に畳む。0x7F を超える文字を含む名前は一致しない（Windows の畳み込みは未モデル化）。
- 拡張子がなければ `.dll` を足す。末尾の `.` は「拡張子なし」で、取り除くだけ。
- パスを含む名前（`\`、`/`、`:`）は abort。256 文字を超える名前は abort。
- 成功時は last error に触れない（ページに書かれていない）。

### 4. 擬似アドレスの領域（IMP_VA_BASE という名前は存在しなかった）

依頼は「既存の import 用の擬似 VA 領域（IMP_VA_BASE など）」と書いた。リポジトリを検索したが、その名前の定数はない（`grep IMP_VA_BASE` は lift/、docs/ に 0 件）。既存の仕組みは、`lift/harness/image.c` が各 IAT スロットを「そのスロット自身の VA」で埋めること（`jmp [IAT]` の対応は、生成器が thunk を置き換える方式）だけで、擬似アドレスを呼ぶ仕組みはなかった。そこで次を作った。

- 応答表の行 k（0 始まり）の擬似 VA = `0x7FF900000000 + 0x10 × (k + 1)`（`imports_table.rs` の `RESP_VA_BASE`、`RESP_VA_STEP`）。
- 領域は、イメージ（0x400000〜0x1CC0000）より上、モジュールのハンドル（0x7FF800000000 + 0x10000 × N、N ≤ 25）より上。
- 各擬似 VA は `fle_dispatch_table` に、全関数エントリの後ろへ追加する。関数エントリは全て 0x7FF9... より小さいので、表は VA の昇順のままで、`fle_dispatch` の二分探索がそのまま使える（テスト `dispatch_table_stays_sorted_and_pseudo_vas_are_unique`）。
- 呼ぶ関数は、`jmp [IAT]` の thunk が呼ぶ関数と同じ `fle_imp_*`（戻り値は R[0]、ゲストの戻りアドレスはスタックに残る）。`call` 経由では `fle_dispatch` の呼び出し元（dispatch.rs の call）が戻りアドレスを積むので、`fle_imp_*` が `ret` を消費する規約と合う。

### 5. 応答表の管理方式（二重管理を避ける）

- **1 か所**: `lift/crates/fe-lift/src/imports_table.rs` の `TABLE`。thunk の置き換え（`lookup`）、`fle_resp_table[]`、擬似 VA、`fle_dispatch_table` への追加の全てが、この表から出る。
- 生成 C に出るのは `fle_resp_table[]`（`{ dll 小文字, API 名, 擬似 VA, 関数 }`）、`fle_resp_count`、`fle_imp_*` のプロトタイプ。runtime（`fle_modules.c`）は `fle_resp_table` を読むだけで、API 名を持たない。
- 出力は、閉包に GetProcAddress の thunk があるときだけ（`resp_wanted`）。無い出力は従来と同じ（`tables_absent_without_get_proc_address`）。runtime 側は `fle_resp_table` を weak 参照にしてあり、表なしでリンクしても全て「見つからない」になる（`get_proc_address_without_a_table_finds_nothing`）。
- 件数: 11 件 + 2 件 = 13 件。生成器の出力で確認した（`lift/out-modules/gen-modules-896.txt` と生成 C の `fle_resp_count = 13`）。GetModuleHandleW と GetProcAddress 自身も表に入るので、ゲストが `GetProcAddress(kernel32, "GetProcAddress")` を引くと擬似アドレスが返る。
- 表に行を足すときの注意: 行の順序が擬似 VA を決める。ゲストが保持した擬似アドレスは実行ごとの値で、保存されない前提なので、順序の変更は問題ないが、末尾に足す運用にした。
- 変更した既存ファイル（追記のみ）: `imports_table.rs`（2 行と 4 関数の追加）、`lib.rs`（`Stats.resp_entries` フィールドと、`dispatch::table` の呼び出しを `if resp { ... } else { ... }` に変えた 9 行）。`fle_imports.c`、`fle_imports.h` は変えていない（`fle_set_last_error` が既にあったため）。

### 6. 応答表にない API を NULL にする影響

GetProcAddress が NULL を返す API は、ゲストが「その API は無い」と判断する。実在する Windows の kernel32 は GetLogicalProcessorInformation などを持つ（推測。確認していない）ので、この環境は実機と違う。PM の決定どおり。ゲストが NULL を許す設計の場合（f_4099c0 は NULL なら 0 を返す）は先へ進むが、NULL を許さないゲストは別の経路に入る。応答表に行が増えると、NULL だった API が非 NULL に変わり、ゲストの経路が変わり得る。

### 7. 仕様で決まらず abort にしたもの

- GetProcAddress の序数指定（lpProcName の bit 16 以上が 0。NULL を含む）。仮想モジュールに序数がない。仕様の確認結果: 序数を渡せる、は書かれているが、64 ビットでの判定と、存在しない序数の結果は書かれていない。
- GetProcAddress の hModule が、仮想モジュールのハンドルでない値、NULL、DLL 自身のハンドル。
- GetModuleHandleW のパスつきの名前、256 文字超の名前。GetProcAddress の 256 文字超の名前。

## 限界

- ハンドルは、実メモリの住所ではない。ゲストがこの値からヘッダを読む（例: PE ヘッダの検査）と SIGSEGV になる。
- 擬似アドレスはコードとして実行できるものではなく、`fle_dispatch` が表で引くための値。ゲストがこの値を `jmp` や、`fle_dispatch` を通らない `call` で直接使うと、静的に解決された呼び出しや、解決できない間接 jmp（`indirect::jmp`）は `fle_unsupported`。
- GetModuleHandleW の参照カウント、LoadLibrary、FreeLibrary は未実装。
- Windows 本体との照合はしていない（ホストが Linux）。
