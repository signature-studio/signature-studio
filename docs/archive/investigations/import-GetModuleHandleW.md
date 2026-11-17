# import-GetModuleHandleW: 仕様の出典、読んだ箇所、確認できなかった点

作業者（M1）の調査。取得日 2026-10-20。判断は `docs/decisions/0015-virtual-modules.md`。

## 対象

- IAT VA 0x1971368、kernel32.dll!GetModuleHandleW（`lift/out-imports/imports.txt` 306 行）。thunk は 0x407060（`jmp *0x1971368`）。
- 呼び出し箇所: f_4099c0 の 0x4099e4（`call 0x407060`）。rcx = 0x409ae8。0x409ae8 のバイト列は `6b 00 65 00 72 00 6e 00 65 00 6c 00 33 00 32 00 2e 00 64 00 6c 00 6c 00 00 00`（objdump -s の読み）= UTF-16LE の `kernel32.dll`。

## 仕様の出典

- URL: https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulehandlew （ms.date 2024-12-09、updated_at 2025-07-01）

読んだ箇所と要約:

- Syntax: `HMODULE GetModuleHandleW([in, optional] LPCWSTR lpModuleName);`
- 概要: 指定モジュールのハンドルを返す。モジュールは呼び出しプロセスにロードされていなければならない。競合を避けるには GetModuleHandleEx を使う。
- lpModuleName:
  - ロード済みのモジュール名（.dll または .exe）。
  - 拡張子を省くと、既定のライブラリ拡張子 .dll が付く。
  - 末尾の `.` は「拡張子なし」を表す。
  - パスは不要。パスを指定するときは `\` を使う（`/` ではない）。
  - 名前は、プロセスのアドレス空間にある現在のモジュール名と、大文字小文字を区別せず比較する。
  - NULL なら、プロセスを作成したファイル（.exe）のハンドルを返す。
  - LOAD_LIBRARY_AS_DATAFILE でロードしたモジュールのハンドルは取れない。
- Return value: 成功でモジュールのハンドル。失敗で NULL。拡張エラーは GetLastError。
- Remarks:
  - ハンドルはグローバルでも継承可能でもない。複製できず、他のプロセスでは使えない。
  - パスなしで、同じ基底名と拡張子のモジュールが複数あると、どれが返るかは予測できない。
  - 参照カウントを増やさない。ただしこのハンドルを FreeLibrary に渡すと減る。渡してはならない。
  - マルチスレッドでは、ハンドルが後で有効である保証はない。

## 確認できなかった点（ページに書かれていない）

- 失敗時の具体的なエラーコード。ページは GetLastError を呼ぶとだけ書く。ERROR_MOD_NOT_FOUND（126）は System Error Codes（0-499）ページ（https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--0-499- ）に「The specified module could not be found.」とあるが、このページが GetModuleHandleW の失敗で 126 を返すとは書いていない。126 は PM の決定（0015）。
- 空文字列の扱い。
- 拡張子の補完の細部（`a.b` のようにドットが途中にある名前は拡張子ありとして扱う、は読みであって明記はない）。
- 非 ASCII 文字の大文字小文字の比較規則。
- 名前の長さの上限。
- 成功時に last error を触るか。
- パスつきの名前と、ロード済みモジュールの完全パスの比較の詳細。

## 実装での扱い（`lift/runtime/fle_modules.c` の `fle_imp_GetModuleHandleW`）

- NULL: 0x400000（リフトした DLL のベース。PM の決定）。
- 名前は ASCII の英字を小文字に畳んで比較する。0x7F を超える文字を含む名前は表のどの名前とも等しくない（仮定）。
- 末尾 `.` は取り除いて拡張子を足さない。ドットが無ければ `.dll` を足す。空文字列は `.dll` になり、見つからない（読みからの推論）。
- パス（`\`、`/`、`:`）を含む名前は abort（仮想モジュールにパスがない。推測で答えない）。256 文字を超える名前も abort。
- 見つからない: NULL と last error 126。成功時は last error を変えない。
- 参照カウントは持たない（仮想モジュールは数えない）。

## 実行での確認

- `lift/out-modules/run-modules-896.txt`: entry 46（f_407060、guest-return 0x4099e9）は abort せず返った。
- テスト: `lift/crates/fe-lift/tests/modules.rs` の `get_module_handle_name_rules`（24 行の名前の規則）、`every_pe_import_dll_is_a_module`（PE の import 表の 24 DLL を、PE の綴りと大文字で引く）。Windows 本体との照合はしていない（ホストが Linux）。
