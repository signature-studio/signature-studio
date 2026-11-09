# 0070: 仮想ウィンドウクラス表（RegisterClassW、GetClassInfoW）。ゲスト自身のクラスだけを答え、last error には触れない（仮の決定）

状態: **仮の決定**。PM の規則（調査 → 条件 i〜iv を満たすときだけ実装）に従い、作業者（サブエージェント）が実装した。「一次」は Microsoft Learn（2026-10-22 取得）、「実測」は DLL のバイト列・gdb・コマンド出力、「読み」は `objdump` の静的な読み、「仮」は根拠となるページが無い選択。調査は `docs/investigations/getclassinfo-caller.md`。実装は `lift/runtime/fle_class.c`、`fle_class.h`（新規）、`lift/crates/fe-lift/src/imports_table.rs`（2 行の追記）。テストは `lift/crates/fe-lift/tests/class.rs`。

## 背景

決定 0069 で LoadIconW が NULL を返すと、harness は `GetClassInfoW`（thunk 0x424ba0、`f_52a7f0` の 0x52a820、rcx = 0x400000、rdx = 0x52a7c8 = L"TPUtilWindow"）で止まった。`f_52a7f0` は Delphi の標準の形（調査 §3）: 「GetClassInfoW で探し、未登録なら RegisterClassW で登録し、CreateWindowExW で作る」。登録も問い合わせもウィンドウプロシージャを呼ばない（調査 §4）。

## 決定

### RegisterClassW（`ATOM f(const WNDCLASSW *rcx)`）

次の形だけを答える。`lpfnWndProc` は非 NULL、`hInstance` は 0x400000（`FLE_EXE_HANDLE`）、`style` に CS_GLOBALCLASS（0x4000）なし、`cbClsExtra` と `cbWndExtra` は 0〜40、`lpszClassName` は文字列（IS_INTRESOURCE でない: 上位 48 ビットが非 0）で、1〜256 ユニット、ASCII（0x7F 以下）、このモジュールでまだ登録されていない名前。

- 戻り値: **アトム 0xC000 + n**（n = これまでに登録した数。0、1、2…）。再利用しない。0xFFFF の次の登録は停止（"used up"）。
- 保存: 72 バイトの WNDCLASSW を**そのまま**複製する（`fle_ld` で 8 バイトずつ。ポインタのフィールドは辿らない）。名前は別に 256 ユニットまで複製する。`lpfnWndProc` は**数として保存するだけで呼ばない**。
- **last error には触れない**。戻り番地（`[rsp]`）と R[0] 以外のレジスタは変えない。
- 上の形以外はすべて名前付きの停止（`fle_stop`）: NULL の lpWndClass、NULL の lpfnWndProc、hInstance が 0x400000 でない、CS_GLOBALCLASS、extra が範囲外（負の値を含む）、名前が NULL、atom、空、257 ユニット以上、ASCII でない、既に登録された名前、**大文字小文字だけが違う登録済みの名前**、アトムの枯渇、2 番目のスレッドからの呼び出し。

### GetClassInfoW（`BOOL f(HINSTANCE rcx, LPCWSTR rdx, LPWNDCLASSW r8)`）

hInstance が 0x400000 で、lpWndClass が非 NULL、lpClassName が上の規則の文字列のときだけ答える。

- 名前が RegisterClassW で登録済み（hInstance も一致、**ユニット単位で完全一致**）→ 保存した 72 バイトを `lpWndClass` へ書き、**1**。
- 登録されておらず、名前が**ゲストの静的な 6 か所の名前**のどれか（TPUtilWindow、TTimerWindow、TLightweightLayeredControl、TApplication、VTPanningWindow、TILRemoteWindow。綴りは DLL のとおり）→ **0**（失敗。`lpWndClass` は書かない。last error に触れない）。
- それ以外はすべて停止: hInstance が NULL（システムクラスの問い合わせ）、0x400000 以外、名前がどれにも当たらない（EDIT、BUTTON などのシステムクラス、RICHEDIT20W、動的な名前を含む）、大文字小文字だけが違う登録済みの名前、2 番目のスレッド、lpWndClass が NULL、名前が atom、空、257 ユニット以上、ASCII でない。

### 範囲の外（実装しない。条件 iv）

UnregisterClassW（thunk 0x425670）、GetClassNameW（thunk 0x424bb0）、GetClassInfoExW、RegisterClassExW、CreateWindowExW は答えない（ゲストが後者 2 つは輸入しない）。次の停止点は CreateWindowExW（調査 §7）。（**0071 で追記**: CreateWindowExW は決定 0071 で、隠しツールウィンドウの形だけ答えるようになった。他は答えないまま。）

## 選択肢

- A. 停止のまま（規則どおり）。**不採用**: ゲストの標準の形で、クラス表だけの小さな設計で進める。
- B. 登録だけ答え、GetClassInfoW は常に 0 を返す。**不採用**: 同じクラスを登録後に問い合わせる形（ゲストの後続）で嘘になる。表を持つほうが小さい。
- C. GetClassInfoW の未登録を「どの名前でも 0」にする。**不採用**: EDIT、BUTTON などのシステムクラスは実機では 1 を返す（Learn: 「To retrieve information about classes defined by the system … set this parameter to NULL」）。名前が不明な形で 0 を返すと嘘になるので、ゲストの 6 つの名前に限る。
- D. 仮想クラス表（採用）。

## 値と出典の区別

| 項目 | 内容 | 出典 | 状態 |
|---|---|---|---|
| 失敗時の戻り値が 0（GetClassInfoW と RegisterClassW） | 両ページ「If the function fails, the return value is zero」 | **一次** | 確認済み |
| 成功時の GetClassInfoW は非 0、RegisterClassW はアトム | 両ページ | **一次** | 確認済み |
| RegisterClassW のアトムは「そのクラスを一意に識別する」 | RegisterClassW ページ | **一次** | 確認済み（値は書かれない） |
| 文字列アトムの範囲は 0xC000〜0xFFFF | About Atom Tables | **一次** | 確認済み |
| 最初のアトムが 0xC000、次々に +1 | ページに書かれない | — | **仮**（範囲の下限から取った選択） |
| WNDCLASSW が 72 バイト | フィールドの型と x64 の整列から導いた | **導出**（Learn に sizeof は無い） | 確認済み（レイアウトは WNDCLASSW ページのフィールド順と型） |
| 名前の最大 256、超えると RegisterClass が失敗 | WNDCLASSW ページ | **一次** | 確認済み（失敗の GetLastError の値は無いので停止） |
| extra は 40 バイトまで | About Window Classes「fails if … greater than 40 bytes」 | **一次** | 確認済み（失敗の値は無いので停止） |
| ウィンドウプロシージャは登録で呼ばれない | CreateWindow 以降が呼ぶ（About Window Classes、WNDCLASSW「You must use the CallWindowProc function to call」） | **一次**（文書の記述）と読み | 確認済み |
| 同じ名前の再登録の結果 | ページに書かれない | — | **未確認**（停止） |
| 名前の大文字小文字の扱い | ページに書かれない | — | **未確認**（完全一致のみ。大文字小文字だけが違う名前は停止） |
| 失敗時の GetLastError の値 | 両ページに書かれない（エラーコード表に ERROR_CLASS_ALREADY_EXISTS 1410、ERROR_CLASS_DOES_NOT_EXIST 1411 はあるが、両ページは返すと書かない） | — | **未確認**（last error に触れない: **仮**） |
| ゲストの GetClassInfoW の名前と hInstance | f_52a7f0 ほか 6 か所 | **実測**（DLL のバイト列。テストで固定） | 確認済み |
| ゲストが Get の 0 の後に GetLastError を読まない | f_52a7f0 の 0x52a7f0〜0x52a8f4 は 5 つの thunk（Get、Unregister、Register、CreateWindowExW、SetWindowLongPtrW）しか呼ばない。gdb でも GetClassInfoW の後に GetLastError / SetLastError は無い | **読み** + **実測** | 確認済み（限界あり） |

## 採用条件（PM の i〜iv）の確認

- (i) 「Get で未登録を確認し、Register で登録する」だけでウィンドウプロシージャを今呼ぶ必要がない: **満たす**（調査 §3、§4）。
- (ii) 失敗時の戻り値（0）は Learn にあり、GetLastError の値は無いので「last error に触れない」で済む: **満たす**。f_52a7f0 は Get の 0 の後に GetLastError を読まない（読みと gdb）。成功の経路（RegisterClassW の非 0）で GetLastError を読む箇所はない。ただし RegisterClassW の戻り 0 を検査する 2 か所（0x867eb8、0x6119f9）の失敗の報告は、実装が 0 を返さない（停止する）ので今は入らない。
- (iii) 設計が小さく、システムクラスに GetClassInfoW を向ける形は停止: **満たす**（調査 §5。f_611550 の 4 か所は EDIT、COMBOBOX、BUTTON、RICHEDIT20W に向くので、実装では名前が 6 つのうちのどれでもなく停止する）。
- (iv) 追加する API は 2 つまで: **満たす**。ゲストの RegisterClassW 8 か所の形は全部確認した（調査 §2.2）。UnregisterClassW、GetClassNameW は実装していない。

## 決定論

表は呼び出しの純関数で、ホストの実クラス、実ウィンドウ、モジュール、時計を読まない。単一スレッド（最初の呼び出しのスレッドだけ。2 番目のスレッドは停止。0054 と同じ `fle_host_tid()`）。テスト `source_uses_no_host_...` が、ホストの API と `fle_call_guest`、`fle_set_last_error` を使わないことと、`long` を使わないことを固定している。native と wasm（node）は rsp= の行を除いて一致した。

## 撤回条件

- 実 Windows の GetClassInfoW / RegisterClassW の失敗時の GetLastError が分かり、ゲストがそれを読むと分かったとき（その値を返す。last error に触れない仮の決定を改める）。
- Learn が失敗時の GetLastError の値、同じ名前の再登録の結果、クラス名の大文字小文字の扱い、最初のアトムの値を書いたとき（その値にする）。
- ゲストが NULL の hInstance（MainInstance = 0）または MainInstance 以外の値でゲスト自身のクラスを問い合わせると分かったとき、またはシステムクラスに GetClassInfoW を向ける経路に届いたとき（停止の形。クラス表の拡張か別の決定）。
- ゲストが GetClassInfoW の 0 の後で GetLastError を読むと分かったとき（停止に戻す）。
- CreateWindowExW を実装するとき（クラス表から wndproc を引く連携が要る。別の決定）。**0071 で追記**: 決定 0071 が実装した。連携は `fle_class_lookup`（`fle_class.c` に追加。同じ名前の規則で、登録済みならアトムと lpfnWndProc の数を返し、ゲストの静的な 6 名で未登録なら 0、他は停止）。本文の「範囲の外」節の「CreateWindowExW は答えない」は 0071 で答えるようになった（隠しツールウィンドウの形だけ）。UnregisterClassW、GetClassNameW は答えないまま。

## 確認できていないこと

- 実 Windows の失敗時の GetLastError の値（上）。
- ゲストの動的な名前（f_611550 の 4 か所、f_6118a0 の 15 の呼び出し元）の全部。到達すれば停止に出る。
- 停止点（CreateWindowExW）より先で、同じ名前の 2 度目の登録や UnregisterClassW（0x6d84b3 などの終了処理）に届くか。届けば停止に出る。
