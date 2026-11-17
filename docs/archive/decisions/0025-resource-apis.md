# 0025: リソース API（FindResourceW、LoadResource、LockResource、SizeofResource、LoadStringW）

状態: 作業者（サブエージェント）が決定。依頼者（PM）の判断は含まない。

実装は `lift/runtime/fle_res.c`、`fle_res.h`（新規）。表への追記は `lift/crates/fe-lift/src/imports_table.rs` の 5 行（`kernel32.dll` の 4 件と `user32.dll!LoadStringW`）。`lib.rs`（fe-lift）の変更はなし。PE の資源ディレクトリの読み取りは `lift/crates/fe-pe/src/lib.rs` に追加した（`Pe::resources`、`resource_data`、`strings`、`parse_string_block`）。テストは `lift/crates/fe-pe/tests/resources.rs`（9 件）と `lift/crates/fe-lift/tests/res.rs`（19 件）。

## 資源の実体

- 対象は **FLEngine_x64.dll 自身の PE リソース**。ホストの実ファイルは見ない。harness（`harness/image.c`）が DLL を ImageBase 0x400000 に等位置でマップしており、資源ディレクトリ（データディレクトリ 2、RVA 0x16b1000、サイズ 0x20e400）はそのメモリ上にある。runtime はそのメモリのヘッダから資源ディレクトリを辿る（`fle_res.c` の `img_open`）。ファイルは開かない。
- DLL の資源（fe-pe と独立の Python パーサ `lift/out-a2/work/dump_rsrc.py` で測定）: 327 リーフ。型ごと（RT_CURSOR 1: 60、RT_BITMAP 2: 25、RT_ICON 3: 13、RT_DIALOG 5: 1、RT_STRING 6: 40、RT_RCDATA 10: 128、RT_GROUP_CURSOR 12: 57、RT_GROUP_ICON 14: 1、RT_VERSION 16: 1、RT_MANIFEST 24: 1）。どの (型, 名前) にも言語リーフは 1 つだけ。RT_STRING は 40 ブロック、使われている文字列は 625 個（ID 64896〜65535 の範囲）、最長は ID 65114 の 230 文字。
- `hInstance` / `hModule`: 0x400000（`FLE_EXE_HANDLE`、決定 0015。`GetModuleHandleW(NULL)` と同じ値）と NULL を DLL 自身とみなす。`GetModuleHandleW` の仮想モジュール表（決定 0015）の他のモジュール（`kernel32.dll` など）の handle、およびどのモジュールの handle でもない値は **abort**（モジュール名を出す）。これらのモジュールの資源は無いので、「ない」と答える根拠が仕様にない。

## 仕様の出典（Microsoft Learn、取得 2026-10-20）

- LoadStringW: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-loadstringw
- FindResourceW: https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-findresourcew
- FindResourceExW（言語の記述、参照のみ）: https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-findresourceexw
- LoadResource: https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadresource
- LockResource: https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-lockresource
- SizeofResource: https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-sizeofresource
- Resource Types（RT_STRING の定義と「String Table Resources」）: https://learn.microsoft.com/en-us/windows/win32/menurc/resource-types
- STRINGTABLE resource: https://learn.microsoft.com/en-us/windows/win32/menurc/stringtable-resource
- PE Format（「The .rsrc Section」のディレクトリ、エントリ、データエントリ、ディレクトリ文字列）: https://learn.microsoft.com/en-us/windows/win32/debug/pe-format
- System Error Codes（1700-3999、エラー 1812〜1815 の名前と文面）: https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--1700-3999-
- 非公式の出典は使っていない。

### 仕様で決まること（ページの文面）

- **ABI**: `LoadStringW(HINSTANCE hInstance, UINT uID, LPWSTR lpBuffer, int cchBufferMax)`。x64 の引数は rcx、edx、r8、r9d。依頼文の「LoadStringW(hInstance, uID, lpBuffer, cchBufferMax) に見える」はページと一致した。`FindResourceW(HMODULE hModule, LPCWSTR lpName, LPCWSTR lpType)`（**名前が型より先**。FindResourceExW は型が先）、`LoadResource(HMODULE, HRSRC)`、`LockResource(HGLOBAL)`、`SizeofResource(HMODULE, HRSRC)`。
- **LoadStringW の戻り値と動作**（LoadStringW ページ）: `cchBufferMax` が 0 でなければ、バッファに文字列をコピーして NUL で終端する。切り詰めるときも NUL で終端する。戻り値は NUL を除いた、コピーした文字数。`cchBufferMax` が 0 なら `lpBuffer` に文字列そのものへの読み取り専用ポインタが入る（8 バイト）。戻り値はその文字列の文字数。資源内の文字列は NUL 終端とは限らないので長さで判断する。文字列が無いときは 0。
- **ブロック形式**（Resource Types の String Table Resources）: 文字列 ID X はブロック番号 `(X / 16) + 1`（整数除算）の資源（型 RT_STRING = 6、名前がブロック番号）の中の `X % 16` 番目。ブロックは 16 個の「WORD の長さ（UTF-16 単位）+ その単位数」で、終端の NUL は無い。使われていない ID は長さ 0。STRINGTABLE ページでは 16 個ずつ、ID の下位 4 ビットだけが違うものが同じブロック。
- **FindResourceW**: 名前と型は `MAKEINTRESOURCE(ID)`（ポインタの上位が 0）または文字列。`#` で始まる文字列は、残りが 10 進の整数 ID を表す（"#258" は 258）。hModule が NULL ならプロセスを作ったモジュールを探す。成功は HRSRC、失敗は NULL（`GetLastError` で拡張エラー）。
- **LoadResource**: HGLOBAL は後方互換の型で、実際の `HGLOBAL` ではない。`GlobalLock` などに渡してはいけない。ポインタは LockResource、サイズは SizeofResource で得る。hModule が NULL ならプロセスを作ったモジュール。
- **LockResource**: 「LoadResource の成功した戻り値以外を渡してはいけない」。戻り値は資源の先頭バイトへのポインタ。ポインタは、そのモジュールがアンロードされるまで有効。FindResource の HRSRC をそのまま渡してはいけない（「ランダムなデータを指す」）。
- **SizeofResource**: 戻り値は資源のバイト数。失敗は 0（`GetLastError`）。
- **PE の資源ディレクトリ**: 3 階層（型、名前、言語）。各表は 16 バイトのヘッダ（名前エントリ数、ID エントリ数が +12、+14）に 8 バイトのエントリが続き、名前エントリが先。エントリの最上位ビットが 1 ならサブディレクトリ（下位 31 ビットが資源ディレクトリの先頭からのオフセット）、0 ならデータエントリ。名前は WORD の長さに UTF-16 単位。データエントリは Data RVA、Size、Codepage、Reserved（各 4 バイト）。

## 決定（ページから決まらないもの）

1. **HRSRC の値**: ページは「資源の情報ブロックのハンドル」としか書かない。ここでは**マップされたイメージ内の 16 バイトのデータエントリの VA**にした。`LoadResource` と `SizeofResource` はこの値を、ディレクトリを全部辿って突き合わせて検証する（FindResource が返した値以外は abort）。
2. **HGLOBAL（LoadResource の戻り値）の値**: **資源データの先頭バイトの VA**（イメージ内）にした。`LockResource` は同じ値を返す（イメージはマップ済みで、ロックするものがない）。これは LockResource ページの「ポインタはアンロードまで有効」と、LoadResource ページの「GlobalLock などに渡してはいけない」に反しない。`LockResource` は、ディレクトリのいずれかの資源のデータ VA と一致する値だけを受け付け、それ以外（FindResource の HRSRC、NULL、データ内の途中のアドレス、でたらめな値）は abort。NULL を `LockResource` に渡す呼び出しのページ上の挙動は書かれていないので abort にした。
3. **FindResourceW の失敗コード**（ページは「`GetLastError` を呼べ」としか言わない）: System Error Codes（1700-3999）の文面から選んだ。資源ディレクトリがイメージに無い: `ERROR_RESOURCE_DATA_NOT_FOUND`（1812、「イメージファイルが資源セクションを含まない」）。型が無い: `ERROR_RESOURCE_TYPE_NOT_FOUND`（1813）。名前が無い: `ERROR_RESOURCE_NAME_NOT_FOUND`（1814）。名前の下に言語リーフが 1 つも無い: `ERROR_RESOURCE_LANG_NOT_FOUND`（1815）。**どの失敗にどのコードが付くかはページが書いていない**。名前の文面からの対応づけで、実 Windows では確認していない。成功時は last error を変えない（ページは変更すると書いていない）。
4. **文字列名の大文字小文字**: ページは書かない。ASCII の大文字小文字を同一視して比較した（`fle_w_fold`、他の runtime の仮想 FS やレジストリの名前比較と同じ規則）。DLL の名前付き資源には、大文字小文字だけが違う組はない（テストで確認）。ASCII 以外の文字は完全一致。
5. **`#` の名前**: `#` に 10 進数字だけが続く形を整数 ID にした（ページ）。ページが言わないものは abort: `#` だけ、数字以外が混じる（`#abc`、`#-1`）、65535 を超える（ページは数の上限を書かない。`MAKEINTRESOURCE` で表せるのが 16 ビットまでなので、それに合わせた決定）。先頭の 0 は 10 進として読む（`#001` は 1）。名前が 256 文字を超える文字列も abort。
6. **複数の言語リーフ**: ページ（FindResourceW）は、複数言語があるときにどれを返すかを書かない。FLEngine_x64.dll の RT_STRING には無い（全 40 ブロックが言語 0）。どれかの (型, 名前) に 2 つ以上のリーフがあれば abort。型が違えば別のリーフ（DLL には RT_ICON などで言語 0x409、0x2000 などがあるが、(型, 名前) あたりは 1 つ）。
7. **LoadStringW の `hInstance` が NULL**: ページは `hInstance` を `optional` と注記するが、NULL が何を指すかは書かない（説明は「アプリケーション自身のハンドルは `GetModuleHandle(NULL)`」）。ここでは abort にした。FindResourceW などの NULL（「プロセスを作ったモジュール」）とは違う扱いである。
8. **LoadStringW の失敗**: 文字列が無いとき 0 を返す（ページ）。バッファは書かない（ページは書き込むとは言わない）。ブロックが無い場合は FindResourceW と同じコード（1813 または 1814）。**使われていない ID（ブロックはあるが長さ 0）は 1814**（決定: ページに無く、「名前が無い」に最も近い）。実 Windows ではこの場合に `GetLastError` が 1814 になるかは確認していない。成功時は last error を変えない。
9. **LoadStringW の引数の範囲**: `cchBufferMax` が負、`uID` が 0xFFFF を超える（STRINGTABLE の ID は 16 ビットの符号なし整数）、`lpBuffer` が NULL（コピーでもポインタ返却でも必要）はページが定義しないので abort。`cchBufferMax` が 1 のときは NUL だけを書いて 0 を返す（ページ: 切り詰めて NUL 終端）。
10. **資源ディレクトリの形式エラー**（レベルの不一致、範囲外のオフセット、ブロックが 16 文字列より短い、長さがブロックを超える、データがイメージ外）は abort。DLL では起きない（テスト `every_leaf_is_found_and_its_bytes_match_fe_pe` と `load_string_matches_fe_pe_for_every_id` が全リーフ、全 ID で確認）。ブロック末尾のパディング（この DLL の 9 ブロックに 2 バイトある）は許す。

## 確認できていないこと

- **実 Windows での挙動**。ホストは Linux で、比較はしていない。決定 3、8 のエラーコードの対応づけ、決定 4 の大文字小文字、決定 6 の複数言語の選び方は、ページが書かないので実 Windows と違う可能性がある。
- `LoadStringW` が内部で `FindResourceExW` を使い、言語を現在のスレッド言語から選ぶ（ページは書かない）か、`MAKELANGID(LANG_NEUTRAL, SUBLANG_NEUTRAL)` の扱いがどうなるか。この DLL の RT_STRING は全て言語 0（中立）なので、結果は決まるが、他のリソースで言語を選ぶ規則は実装していない（決定 6 のとおり abort）。
- MUI ファイル（FindResourceW の hModule の説明「PE ファイルまたは付属の MUI ファイル」）は扱わない。
- 資源のアクセス保護、ロード後のアンロード（`FreeLibrary`）は扱わない。
- `FindResourceW` の `lpType` に文字列の型名（例 `"TEXTX"`）を渡す経路は、合成 PE のテストでパーサを確認した（fe-pe）が、runtime の C から名前の型を引くテストは DLL の RT_GROUP_ICON の `MAINICON`（名前側）のみ。型側の名前は `NOSUCHTYPE`（見つからない）だけを確認した。

## テスト

- `lift/crates/fe-pe/tests/resources.rs`: 9 件。合成 PE（3 階層、名前エントリ、言語 2 つ、パディング、サロゲート ペア、埋め込み NUL）でディレクトリのパースと RT_STRING のブロック形式、壊れた木（範囲外、レベルの不一致、フラグの不整合、切り詰め）が panic でなく Err になること。実 DLL（`FLENGINE_PE`、なければ `binaries/win64/FLEngine_x64.dll`。ファイルが無ければ SKIPPED で通る。`FLENGINE_PE` を指定してファイルが無ければ失敗）で 327 リーフ、625 文字列と、Python パーサで読んだ文字列。
- `lift/crates/fe-lift/tests/res.rs`: 19 件。生成器（5 つのサンクが runtime を呼ぶ。DLL が違えば別の API）、実 DLL を harness の `image_load` でマップして C から呼ぶ（LoadStringW の ID 64896 の文字列、切り詰め 5 通り、`cchBufferMax` 0 のポインタ、存在しない文字列、**全 65536 ID**を fe-pe の結果と比較、最長文字列、FindResourceW の名前と型の規則 19 通り、**327 リーフすべて**を Find/Load/Lock/Sizeof で引いて fe-pe のサイズ、データ RVA、FNV-1a と一致、abort する入力 29 通り（他モジュールの handle、でたらめな handle、NULL や偽の HRSRC・HGLOBAL、`#` の不正な形、長すぎる名前、`LoadStringW` の不正な引数。それぞれ SIGABRT と "fle:" のメッセージの内容を確認））。合成イメージ（資源ディレクトリ無し、空、言語リーフ無し、複数言語、データがイメージ外、レベル違い、数の超過、ブロックが短い、長さが超過）。
- 期待値は、Microsoft のページの文面と、fe-pe（Rust）、独立の Python パーサの測定から書いた。`fle_res.c` の出力から書いた値はない。
