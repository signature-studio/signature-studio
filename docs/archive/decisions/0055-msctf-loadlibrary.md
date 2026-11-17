# 0055: msctf.dll を仮想モジュールにし、LoadLibraryW("Msctf.dll") に応答する。SetInputScope は GetProcAddress で見つかるが、呼ばれたら名前付きで停止する

状態: 作業者（サブエージェント）が、PM の方針（「創作しない。ホストを見せない。環境の定義を既存の OS の定義と矛盾させない」）の範囲内で決定。**仮の決定**（理由は下の「仮の決定である点」）。依頼者（PM）の追加判断は含まない。実行記録と、ゲストの経路の読みは `docs/investigations/msctf-caller.md`。実装は `lift/runtime/fle_modules.c`（表の末尾に 1 行）、`lift/runtime/fle_msctf.h/.c`（新規）、`lift/crates/fe-lift/src/imports_table.rs`（応答表に 1 行）。テストは `lift/crates/fe-lift/tests/msctf.rs`（11 件）。

## 背景

harness は `fle: LoadLibraryW: module "msctf.dll" is not a virtual module and not a name this environment is defined to lack ...` で止まっていた。呼び出し元は f_5de9e0（`f_44db40(L"Msctf.dll", 0x8000)`）。ゲストは SetErrorMode(0x8000) を前後に挟んで LoadLibraryW を呼び、結果を検査せずに `[0x13d9ef8]` に保存する。NULL なら「Msctf なし」（0 を返すスタブ）、非 NULL なら「あり」（1 を返すスタブ）。非 NULL のときだけ、別の関数 f_5e0f30 が GetProcAddress で `SetInputScope`（1 つだけ）を引いて呼ぶ。NULL でも落ちない設計（`msctf-caller.md` §1）。

## 選択肢

PM の方針は、msctf.dll を決定 0029 の「この環境が欠いている DLL」の一覧（`fle_load_absent[]`）に加えてよいのは、(a)(b)(c) がすべて確認できたときだけ、としていた。

| | 内容 | (a) ゲストが NULL を許容 | (b) NULL の経路が自然 | (c) 環境の定義と矛盾しない |
|---|---|---|---|---|
| A | `fle_load_absent[]` に msctf.dll を足す（NULL + 126） | 確認（読み。§1.5） | 確認（「Msctf があるか」を 0/1 で返す関数があり、NULL は設計内） | **確認できない（矛盾する）** |
| B | msctf.dll を仮想モジュールにする。GetProcAddress で引く `SetInputScope` だけ応答表に行を作り、呼ばれたら名前付きで停止 | — | — | 矛盾しない |
| C | 何もせず名前付きで停止したまま報告 | — | — | — |

(c) の確認: 決定 0014 のエミュレート OS は Windows 10 22H2。msctf.dll の SetInputScope ページ（Learn）は要件を「Windows XP 以降」（DLL は Msctf.dll）と書き、TSF の概要ページは「Windows XP or later versions of the operating system」と書く。Windows 10 は XP 以降なので、**ページから導ける範囲で「Windows 10 には msctf.dll がある」**（ページが Windows 10 に msctf.dll があるとそのまま書いているわけではない。導いた）。A はこれと逆の主張（「この環境は msctf.dll を持たない」）になる。**A は採らない**。PM の方針の「矛盾する場合は欠いているとは定義せず、仮想モジュールとして応答させる」に従い B を採る。

C は、B の実装が小さく、ゲストの経路を創作せずに済む（下）ので採らない。

## 決定

1. **msctf.dll を仮想モジュール表の末尾（N = 26）に追加する**。ハンドルは決定 0015 の式 `0x7FF800000000 + 0x10000 × 26` = `0x7FF8001A0000`。並べ替えない（決定 0015）。名前の規則（大文字小文字を畳む、拡張子がなければ `.dll`、末尾の `.` は拡張子なし）は変えない。`LoadLibraryW("Msctf.dll")` と `GetModuleHandleW` は同じハンドルを返し、LoadLibrary は参照を 1 つ増やし、成功時は last error に触れない（決定 0029 のとおり）。起動時にロード済みの参照 1 から始まる（FreeLibrary の既存の規則。ゲストは終了処理で FreeLibrary を呼ぶ形があり、1 に戻るだけで 0 にはならない）。
2. **`fle_load_absent[]`（決定 0029 の 9 名）は変えない**。msctf.dll は加えない。「欠けている」名前に新しい行を足さない。
   - 注記（決定 0088、仮の決定）: 決定 0088 は、この決定の (c)（Learn の Requirements 欄から「Windows 10 にある」を導く）を avrt と powrprof に当てて、2 名を仮想モジュール表の末尾（N = 31、32）に足した。`fle_load_absent[]` の 9 名は 0088 でも変えていない。9 名のうち dwmapi と shcore に同じ (c) を当てると「Windows 10 にある」側に入るが、0088 はそれを解消していない（0015/0029 と 0055 の基準の違いは残る。0088 の「範囲外」）。この行の本文は書き換えていない。
3. **応答表に `("msctf.dll", "SetInputScope", fle_imp_SetInputScope)` の 1 行を足す**。GetProcAddress(msctf のハンドル, "SetInputScope") は擬似アドレスを返し、他の名前は決定 0015 のとおり NULL と ERROR_PROC_NOT_FOUND（127）。msctf.dll のほかの export には行を作らない（ゲストは引かない）。**`SetInputScope` の行は GetProcAddress がゲストの唯一の読み手に「ある」と答える**ためのもので、msctf.dll の export の一覧を主張するものではない。
4. **`fle_imp_SetInputScope` は呼ばれたら `fle_stop` で止まる**（"fle: SetInputScope: msctf.dll is a virtual module that exports it, but this runtime has no window table ... no HRESULT is invented"）。Learn のページは結果として S_OK と、別スレッドのウィンドウに対する E_INVALIDARG だけを書き、どちらもウィンドウを知らないと決まらない。この runtime はウィンドウ表を持たないので、HRESULT を創作しない。

## 理由（創作しないことの確認）

- ゲストの経路を作っていない: LoadLibraryW が非 NULL を返すと、ゲストは f_5dea70 で「あり（1）」を選び、f_5e0f30 が `SetInputScope` を引く。この経路はゲストの設計どおり（読み）で、runtime が値を作って分岐を決めたわけではない。
- 応答の内容は一次資料の範囲: 成功は「a handle to the module」（LoadLibraryW ページ）。ハンドルの値は決定 0015 の式（選択）。SetInputScope は「ある」（Learn のページに export として載り、DLL は Msctf.dll）。結果は作らず止める。
- ホストを見せない: どの応答もホストの DLL・ファイル・環境を読まない。ハンドルは固定値、SetInputScope は止まるだけ。

## 一次資料で確認できたこと（Microsoft Learn。2026-10-21 取得。`msctf-caller.md` §2）

- LoadLibraryW: 成功はモジュールのハンドル、失敗は NULL（GetLastError で拡張エラー）。
- SetErrorMode: 値 0x8000 = SEM_NOOPENFILEERRORBOX。戻り値は直前のモード。応答は決定 0020 で済み（`fle_proc.c`）。
- SetInputScope: `HRESULT SetInputScope(HWND, InputScope)`、DLL は Msctf.dll、最小の対応は Windows XP。戻り値は S_OK、別スレッドのウィンドウは E_INVALIDARG。
- TSF の概要ページ: 適用は「Windows XP or later」。
- System Error Codes: ERROR_MOD_NOT_FOUND = 126、ERROR_PROC_NOT_FOUND = 127。

## 一次資料で確認できないこと

- 「Windows 10 に msctf.dll が必ずある」ことを書いたページ。上のとおり XP 以降の要件から**導いた**。
- LoadLibraryW が存在しない DLL に対して付ける GetLastError が 126 であること。LoadLibraryW のページは失敗時のコードを書かない（126 は決定 0015/0029 の PM の選択）。msctf.dll は存在する扱いなので、この決定では 126 を使わない。
- msctf.dll が export する名前の一覧（`SetInputScope` 以外は何を export するか、この決定は主張しない）。
- ハンドルの値（`0x7FF8001A0000`）。決定 0015 の式による選択で、どのページにも値はない。ゲストが msctf のハンドルからヘッダを読むと SIGSEGV になる（決定 0015 の限界のまま）。
- ゲストが実際に SetInputScope を呼ぶ経路（f_620a80 → f_5e0f30）に実行が届くか。今回の停止点の先で、実測していない。

## 仮の決定である点

- 「Windows 10 に msctf.dll がある」は XP 以降の要件からの導出で、msctf.dll を「ある」とする根拠はそれだけ。
- ハンドルの値は選択。
- `SetInputScope` を「ある」とする応答表の行は、ゲストが引く名前だけに限る。msctf.dll の他の export にも広げるかは、ゲストが引く名前が増えたとき（GetProcAddress の呼び出しが別の名前を引いたとき）に決める。

## 撤回条件

- Windows 10 の msctf.dll の有無について、一次資料が逆のことを書いたとき、または PM がこの環境を「TSF を持たない」と定義し直したとき: msctf.dll を表から外し、`fle_load_absent[]` に移す（その場合、N = 26 の末尾を外すだけで、他のハンドルは変わらない。テスト `msctf_is_the_26th_virtual_module` と `the_nine_lacking_names_still_fail_with_126` を直す）。
- ゲストが SetInputScope を実際に呼ぶ経路に届いて、窓表（RegisterClassW、CreateWindowExW 系）が決まったとき: `fle_imp_SetInputScope` を、ページの規則（S_OK、別スレッドで E_INVALIDARG）で答える関数に置き換える。それまでは止まる。
- msctf.dll の他の export をゲストが引いたとき: その名前が来るたびに、その API の仕様を確認して行を足すか、止める。

## 限界

- ハンドルは実メモリの住所ではない（決定 0015）。
- LoadLibrary の DllMain の呼び出しや依存 DLL の読み込みはない（決定 0029）。
- GetProcAddress が返す擬似アドレスは `fle_dispatch` を通る呼び出しだけで使える（決定 0015）。
- Windows 本体との照合はしていない（ホストは Linux）。
- `[0x13d9ef8]` の値を gdb で読んでいない（実行は停止点まで。`msctf-caller.md` §6）。
- 入口数が native と wasm で一致しない（13,227 と 13,230）件は、この決定とは別の原因（GetCurrentThreadId の値の桁数。決定 0017/0045 の設計の帰結）。`msctf-caller.md` §6.1 に詳細。
  - **解消済み: 決定 0056**（GetCurrentThreadId を固定の仮想値 0x1001 に。以後、native と wasm の入口数はどちらも 13,230 で一致する。`docs/investigations/virtual-thread-id.md` §5）。上の記述は書いた時点のまま残している。
