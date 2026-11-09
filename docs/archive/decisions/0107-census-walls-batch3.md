# 0107: census の壁 バッチ 3（ILTools 以外）— GetSystemMetrics の 4 index（unit 276）、CompareStringW の「最初の差で決める」（unit 283）、shfolder.dll!SHGetFolderPathW と CSIDL_APPDATA（unit 830、880）。すべて仮の決定

状態: **仮の決定（すべての応答）**。依頼は PM（2026-10-27。番号 0107 は PM の割り当て。census 0097 の壁表のうち ILTools 以外の担当）。決定は作業者（サブエージェント）が行い、書いた。**第 1 段階（設計と実装だけ。R2 の作業者）**: C は `clang -fsyntax-only -Wall -Wextra`（`-Werror` つきと gcc `-O2` でも）で構文を確かめただけで、コンパイル・リンク・実行・テスト（cargo）・census・a4 の測定は行わなかった。Rust のテストは `rustfmt --check` で構文を読ませただけ（型は通していない）。**統合 M2（HEAD 520064a に決定 0106 の差分、続けてこの差分の順に当てた木）で、コンパイルし（Rust の型の誤りなし、警告なし）、`wall_r2.rs` の 17 件を含む全体テストを実行して失敗 0 を確認した**（`lift/out-git/m2-results/`）。**census・a4 の測定は行っていない**。**実機 Windows との照合は無い**（ホストは Linux）。ホストの画面・ファイル・レジストリ・ロケール・ユーザー名・フォルダは一切読まない（0053、0057、0058、0065 と同じ方針）。

**これは A4 の進捗ではない。** census の数は診断で、放棄した unit は完了していない（0046、0060、0097、0101）。この決定で census の壁が動いても、それは仮の応答を含む構成での停止点の移動である。

区分の凡例: **実測** = 実行した出力（census の TSV、他の担当の gdb の記録）。**読み** = PE の逆アセンブルを読んだ結果で、実行して確かめていない部分。**Learn** = Microsoft Learn のページに書かれていること（取得日 2026-10-27）。**参考（非公式）** = Learn 以外。**推測** = 作業者の推論。**未確認** = 確認していない。

事実の出典:

- census の結果: `lift/out-git/h2-results/census-hooks/`（`top-walls-dsp.tsv`、`top-walls-single.tsv`、`report-dsp.tsv`、`report-single.tsv`、`report.md`。HEAD 520064a、決定 0105 まで）。**census の数は A4 の進捗ではない**。
- 決定 0057（GetSystemMetrics）、0098（バッチ 1。「8. SM_DIGITIZER」と「未決定」）、0099（バッチ 2。「census の分類」と W5 の前の記録）、0103（SHGetFolderPathW(CSIDL_FONTS)）、0072（CompareStringW の ASCII の枠）、0015（仮想モジュール。応答表は末尾に足す）、0018（仮想ファイルシステム・環境。初期状態は空）、0014（エミュレート OS = Windows 10）、0016（ロケール en-US）、0073・0101（同梱 DLL を「無い」と定義しない）。
- 他の担当の記録: `lift/out-wallS/docs-new/wall-s-measurement.md` §2.1（unit 830 の null 呼びの gdb 測定。**読むだけ**）。
- FLEngine_x64.dll（`binaries/win64/`、読み取りのみ）の逆アセンブル（`objdump -d -M intel`）とバイト列の走査（作業物は `lift/out-git/r2-results/tools/`。実行は `/tmp/opencode/r2/tools/` に置いたコピーで行った。PE の自前パーサ `xref.py`、UTF-16 文字列の探索 `strscan.py`、rip 相対参照の探索 `dref.py`、thunk の IAT 名 `thunkname.py`、呼び出し元 `callers.py`、CSIDL の走査 `csidl.py`、NULL 検査の走査 `nonull.py`、バイト列 `pat.py`）。
- Microsoft Learn（2026-10-27 に取得。各節に URL）。mingw-w64 のヘッダ（`/usr/share/mingw-w64/include/`。非公式）は CSIDL の番号だけに使った。

## 先に結論

| 壁 | unit | census の分類 | 原因（区分） | この決定 |
|---|---|---|---|---|
| W1 | 276 | d: `GetSystemMetrics` index 86 | `f_80e360` が SM_DIGITIZER の次に **86、0x2003、0x2004** を続けて問い（**読み**）、f_80e3f0 がその後で **95** を問う（**読み**）。census は最初の 86 で止まった（**実測**）。**0x2003、0x2004 は SPI の uiAction ではなく GetSystemMetrics の index**（`mov ecx, 0x2003; call 0x424ee0` = GetSystemMetrics の thunk。**読み**。依頼文の「SPI 0x2003、0x2004」は誤記。0098 自身は SM_CONVERTIBLESLATEMODE、SM_SYSTEMDOCKED と書いている） | 4 つとも答える: **86 → 0、95 → 0、0x2003 → 1、0x2004 → 0**。0x2003 と 0x2004 は一次資料が値を決めない**選択**（PM の判断の点 1）。SM_DIGITIZER = 0 の間、ゲストの分岐はこの 2 つの値に依存しない（**読み**） |
| W2 | 283 | d: `CompareStringW` 「ASCII の英数字以外」 | Delphi の `CompareText`（`f_4399e0`）が `CompareStringW(0x400, NORM_IGNORECASE, "Default", 7, "Segoe UI", 8)` を呼ぶ（**読み**。引数の組はコールバック `0x8650c0` の読みと 0098 の記録）。`fle_locale.c` は**両方の文字列の全ユニット**が ASCII の英数字であることを要求し、"Segoe UI" の空白で止まっていた。順序は位置 0 の 'D' と 'S' で決まる | **最初に違うユニットで決める**: その位置より前のユニットが ASCII の英数字（両側で等しい）で、その位置で両方が英字または両方が数字なら、その順序で答える。**その位置より後は見ない**。それ以外は停止のまま |
| W3 | 830、880 | e: null 関数ポインタ呼び（census は「二次の疑い」） | **一次**。`LoadLibraryW(L"SHFolder.dll")` → `GetProcAddress(h, "SHGetFolderPathW")` の結果を NULL 検査せずに `(0, CSIDL_APPDATA 0x1a, NULL, 0, buf)` で呼ぶ。応答表に `shfolder.dll!SHGetFolderPathW` の行が無く、GetProcAddress が NULL と 127 を返す（unit 830: **実測**（他の担当の gdb）と読み。unit 880: 読み） | 応答表に `shfolder.dll!SHGetFolderPathW` の**行を 1 つ足し**（`shell32.dll!SHGetFolderPathW` と**同じ関数**を共有）、その関数に **CSIDL_APPDATA (0x1a) = `C:\Users\user\AppData\Roaming`** を足す。ユーザー名 "user" は選択（PM の判断の点 3） |

census の上位の壁の残り（ILTools の `LoadLibraryW "iltools_x64.dll"`（unit 245、280、659）は別の担当）。単独構成の `LoadLibraryW "dsp_ippv2_x64.dll"`（unit 144）と null 呼びの unit 170、833、935 は本決定で扱わない（理由は「d」の節）。

## W1: unit 276 — `GetSystemMetrics` の 86、95、0x2003、0x2004

### 原因

| 項目 | 内容（区分） |
|---|---|
| census の記録 | unit 276（proc `0x80f980`）。停止: `fle: GetSystemMetrics: index 86 (SM_TABLETPC) is used by the guest but no value is decided (decision 0057)`。top `0x424ee0`（GetSystemMetrics の thunk）← `0x80e360` ← `0x80e3f0`、top_guest_return `0x80e3a9`（**実測**。`report-dsp.tsv`、`report-single.tsv`） |
| ゲストの関数 `f_80e360`（`0x80e360`〜`0x80e3e2`） | 次の順に GetSystemMetrics を呼び、結果を 5 つのバイトに入れる（**読み**）: (1) `ecx=0x5e`（94、SM_DIGITIZER。0098 が答える）→ `[0x15f6ec8] = (v & 1)`、`[0x15f6ec9] = (v & 0xc) != 0`。(2) `ecx=0x56`（**86**）→ `[0x15f6ecb] = (v != 0)`。呼び出しの戻りは `0x80e3a9`（census の値と一致）。(3) `ecx=0x2003` → `[0x15f6eca] = (v == 0)`（`sete al`）。(4) `ecx=0x2004` → `[0x15f6ecc] = (v != 0)`（`setne al`） |
| 呼び出し元 `f_80e3f0` | `GetModuleHandleW(L"user32")` と `GetProcAddress` で `GetTouchInputInfo`、`CloseTouchInputHandle`、`RegisterTouchWindow`、`UnregisterTouchWindow`、`IsTouchWindow` を引き（`RegisterTouchWindow` 以外のどれかが NULL なら `RegisterTouchWindow` の保存先 `[0x15f6ee8]` を 0 にする。応答表に user32 のこれらの行は無く、NULL になる）、`f_80e360` を呼び、**モード `[0x15f6ec0]` を決める**: `[ec8] かつ [eca]` → 2、そうでなく `[ec9] または [ecb]` → 1、それ以外 → 0。続けて `ecx=0x5f`（**95**、SM_MAXIMUMTOUCHES）を呼んで `[0x15f6ec4]` に入れ、**モードが 2 で値が 0 以下のときだけ** `[ec4] = 2` にする（**読み**） |
| 読み手 | `[ec8]`…`[ecc]` の読み手は `0x80e4b3`〜`0x80e4d8`（上のモードの計算）だけ。**`[ecc]`（SM_SYSTEMDOCKED）は書かれるだけで読まれない**。`.text` 全体の rip 相対参照を走査した（`0x401000`〜`0x13a44b0`。**読み**）。`[eca]`（0x2003）は `test [eca], al`（al = `[ec8]`）の 1 か所だけ |
| つまり | SM_DIGITIZER = 0（0098）なら `[ec8] = 0` で、`[eca]` がどちらの値でもモードは 2 にならない。0x2003 の値は、タッチ・デジタイザのビット 0（NID_INTEGRATED_TOUCH）を立てる決定が入ったときに初めて意味を持つ（**読み**） |
| 呼び出し点 | 86 を渡すのは `0x80e3a4` の 1 か所、95 は `0x80e506`、0x2003 は `0x80e3b9`、0x2004 は `0x80e3ce`（tests/sysmetrics.rs の `SITES` と一致）。他の関数はこれらの index を問わない（**読み**） |

### 仕様（Learn）

[GetSystemMetrics](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getsystemmetrics)（`updated_at` 2025-07-01）の表（2026-10-27 に再取得）:

- SM_TABLETPC 86: 「Nonzero if the current operating system is the Windows XP Tablet PC edition or if the current operating system is Windows Vista or Windows 7 and the Tablet PC Input service is started; otherwise, 0.」
- SM_MAXIMUMTOUCHES 95: 「Nonzero if there are digitizers in the system; otherwise, 0. SM_MAXIMUMTOUCHES returns the aggregate maximum of the maximum number of contacts supported by every digitizer in the system. If the system has only single-touch digitizers, the return value is 1. …」
- SM_CONVERTIBLESLATEMODE 0x2003: 「Reflects the state of the laptop or slate mode, 0 for Slate Mode and non-zero otherwise. … Note that this system metric doesn't apply to desktop PCs. In that case, use GetAutoRotationState.」
- SM_SYSTEMDOCKED 0x2004: 「Reflects the state of the docking mode, 0 for Undocked Mode and non-zero otherwise. …」
- 戻り値: 失敗は 0 で「GetLastError does not provide extended error information」（0057 のとおり、last error には触れない）。

Windows-hardware の Learn（OEM 向け）:

- [ConvertibleSlateMode](https://learn.microsoft.com/en-us/windows-hardware/customize/desktop/unattend/microsoft-windows-gpiobuttons-convertibleslatemode)（`updated_at` 2024-04-22）: 「For tablets or other devices that don't have a keyboard, set ConvertibleSlateMode to 0. For laptops or other devices that always have an accessible keyboard, set ConvertibleSlateMode to 1.」値: 「1 Laptop mode … 0 Tablet mode … This is the default value」。レジストリ値は `HKLM/System/CurrentControlSet/Control/PriorityControl/ConvertibleSlateMode`。
- [Recommended settings for better tablet experiences](https://learn.microsoft.com/en-us/windows-hardware/customize/desktop/settings-for-better-tablet-experiences)（`updated_at` 2024-04-22）: 「Set ConvertibleSlateMode value as 0 if the device is Tablet, Convertible or Detachable. Set it as 1 for other form factors like Desktop, Laptop, or Notebook.」

**ページは「GetSystemMetrics(0x2003) の値が OEM 設定 ConvertibleSlateMode の値」とは書かない**（この結び付きは読みで、上の 2 ページを根拠にした作業者の推論）。

### 決定（仮）

| index | 名前 | 値 | 根拠 | 出典の区分 |
|---|---|---|---|---|
| 86 | SM_TABLETPC | **0** | ページの 2 つの条件（XP Tablet PC edition、Vista／7 で Tablet PC Input service 起動）に、エミュレート OS（Windows 10。0014）はどちらも当たらず、ページの「otherwise, 0」。デジタイザが無い（SM_DIGITIZER = 0）環境とも整合 | Learn の文言からの導出。ただしページは Windows 10 を書かない（実機の Windows 10 の値は**未確認**） |
| 95 | SM_MAXIMUMTOUCHES | **0** | 「Nonzero if there are digitizers in the system; otherwise, 0」。この環境にデジタイザは無い（0098 の SM_DIGITIZER = 0） | Learn の文言と 0098 からの導出 |
| 0x2003 | SM_CONVERTIBLESLATEMODE | **1** | 「0 for Slate Mode and non-zero otherwise」。この環境はスレートでない（マウスあり、デジタイザなし）。OEM の Learn が Desktop には ConvertibleSlateMode = 1 を勧める。**ページが「デスクトップ PC には適用されない」と書くので、デスクトップの値は一次資料が決めない** | **選択**（上の 2 つの Learn ページからの推論。0 と 1 の選択は PM の判断の点 1） |
| 0x2004 | SM_SYSTEMDOCKED | **0** | 「0 for Undocked Mode and non-zero otherwise」。ドッキングは模擬しない。デスクトップ PC の値は一次資料が書かない | **選択**（PM の判断の点 1。ゲストは読まない） |

実装: `fle_sysmetrics.c` の `undecided[]` から 4 件を外し、`case` を 4 つ足す。名前は `fle_sysmetrics.h` の `FLE_SM_IDX_*` と `FLE_SM_*`。index は int なので rcx の下位 32 ビットだけを見る（従来どおり）。last error は触れない。

### 選択肢

- A. 採用（上）。
- B. 0x2003 を 0（スレート）にする: **不採用**。この環境をスレートモードと名乗る根拠が無い。ただし SM_DIGITIZER = 0 の間はゲストの分岐が同じなので、実装への影響は無い（PM の判断の点 1）。
- C. 86 を 1 にする: **不採用**（Tablet PC edition でも Vista／7 でもない。デジタイザも無い）。
- D. 4 つとも名前付き停止のまま: **不採用**。値は決められる（86、95）か、ゲストの分岐に影響しない選択で足りる（0x2003、0x2004）。0098 が「86 だけ決めても unit は ok にならない」としたのは、残りの 3 つが決まらなかったから。

### 撤回条件

(1) 実機 Windows 10 の `GetSystemMetrics(86/95/0x2003/0x2004)`（デスクトップ、タッチなし）の値が示され、食い違う。(2) タッチ・ペンのデジタイザを持つ環境にする決定が入る（SM_DIGITIZER を 0 以外に変えるとき、86、95、0x2003 は一緒に見直す。**0x2003 の値がゲストのモードを変える**。tests/wall_r2.rs の `the_guests_decision_after_these_answers_is_not_tablet_and_does_not_depend_on_the_slate_value` がこの依存を固定する）。(3) ゲストの別の読み手が `[0x15f6ecc]`（docked）を使うと分かった。(4) エミュレート OS（0014）の変更。

### テスト

`tests/wall_r2.rs`: `the_four_indexes_after_sm_digitizer_are_answered_and_the_last_error_is_left_alone`（4 index × 上位ビットが汚れた rcx、last error 5 のまま）、`the_index_numbers_and_values_are_the_learn_numbers_and_the_decided_values`（ヘッダの定数）、`the_guests_decision_after_these_answers_is_not_tablet_and_does_not_depend_on_the_slate_value`（`f_80e360` の 5 回の呼び出しを順に行い、ゲストの計算をテスト側で再現して、モードが 0 であること、0x2003 が 0 でも 1 でも同じであること、タッチのビットが立つと 0x2003 が効くことを示す）、`the_neighbouring_indexes_are_still_not_answered`。実 DLL（無ければ飛ばす）: `the_real_dll_f_80e360_stores_the_answers_as_the_decision_reads_them`（`sete`／`setne` と `test [eca], al` などのバイト列）。

## W2: unit 283 — `CompareStringW` は最初に違うユニットで決まる

### 原因

| 項目 | 内容（区分） |
|---|---|
| census の記録 | unit 283（proc `0x81e040`）。停止: `fle: CompareStringW: character other than an ASCII letter or digit in unequal strings (…)`。top `0x4231c0`（CompareStringW の thunk）← `0x4399e0` ← `0x8650c0`、top_guest_return `0x439a3d`（**実測**） |
| 呼び出し | `f_4399e0`（Delphi の `CompareText`）: `ecx = 0x400`（LOCALE_USER_DEFAULT）、`edx = 1`（NORM_IGNORECASE）、`r8` = 文字列 1、`r9d` = その長さ、`[rsp+0x20]` = 文字列 2、`[rsp+0x28]` = その長さ、呼び出し `0x439a38`（戻りが `0x439a3d` = census の値）、続けて `sub eax, 2`（**読み**。バイト列を tests/wall_r2.rs が確かめる） |
| 文字列 | コールバック `0x8650c0`（`EnumFontFamiliesExW` に渡される。0098）: `lfFaceName` を局所の文字列（"Segoe UI"、8 文字。0066 のシステムフォント）にし、リストの**最後の要素**と `f_4399e0` で比べる。リストは呼ぶ前に `0x866de8` で "Default"（7 文字。`0x866e98` の Delphi 文字列。**読み**）を 1 つ足してある。比べる順は (最後の要素 = "Default", 新しい名前 = "Segoe UI")。結果が 0 でなければ新しい名前をリストに足す（**読み**）。実行して引数の値を gdb で読んではいない（**未確認**） |
| 現在の実装 | `fle_locale.c` の `fle_imp_CompareStringW`（0072）: 等しければ 2、位置 0 の記号と英数字の組、そうでなければ**両方の文字列の全ユニット**が ASCII の英数字であることを要求し、満たさなければ停止。"Segoe UI" の位置 5 の空白で止まる（census のメッセージと整合） |
| 順序が決まる位置 | 位置 0 の 'D' と 'S'（NORM_IGNORECASE で 'd' と 's'）。**どちらも英字で、違う**。空白は 'S' より後ろ（**読み**） |

### 仕様（Learn）

- [Handling Sorting in Your Applications](https://learn.microsoft.com/en-us/windows/win32/intl/handling-sorting-in-your-applications)（`updated_at` 2025-03-11）: 「Usually the sorting functions evaluate strings character by character. However, many languages have multiple-character elements, such as the two-character pair "CH" in traditional Spanish.」また 0072 が使った word sort の文（記号は英数字の前、ハイフンとアポストロフィは別扱い）と、word sort の表（"billet, bills, bill's, cannot, cant, can't, con, coop, co-op"）。「When used in memcmp, the sort key produces the same order as … CompareString」。
- [CompareStringEx](https://learn.microsoft.com/en-us/windows/win32/api/stringapiset/nf-stringapiset-comparestringex)（CompareStringW は「See Remarks for CompareStringEx」）: 戻り値 CSTR_LESS_THAN 1、CSTR_EQUAL 2、CSTR_GREATER_THAN 3（「the value 2 can be subtracted」）。「NORM_IGNORECASE: Ignore case. For many scripts (notably Latin scripts), NORM_IGNORECASE coincides with LINGUISTIC_IGNORECASE. Note NORM_IGNORECASE ignores any **tertiary** distinction」。Remarks: 「…the locale does not support any linguistic compressions, as when traditional Spanish sorting treats "ch" as a single character.」
- 英語（米国）ロケールの ASCII の英字の順序と、英字の主な差が大文字小文字の差より強いことは、ページの表（billet < bills）と「tertiary」の文から読める（0087 と同じ読み）。**英語（米国）に複数文字の要素があるかをページは書かない**（既存の実装は、英数字だけの文字列でそれが無いと仮定している）。

### 決定（仮）

`fle_imp_CompareStringW` の最後の部分を次のようにする（それ以前の分岐 — 等しければ 2、位置 0 の記号と英数字（0072）— は変えない）:

1. 共通の長さ `m = min(n1, n2)` を位置 0 から調べる（ASCII の大文字を小文字に畳んだユニット `a`、`b`）。
2. `a == b`: そのユニットが ASCII の英字・数字でなければ**停止**（メッセージ: `character other than an ASCII letter or digit before the first difference`）。英数字なら次の位置へ。
3. `a != b`: 両方が英字、または両方が数字なら、**`a < b` で 1、そうでなければ 3 を返す**。**この位置より後ろのユニットは、どちらの文字列でも見ない**（空白・記号・非 ASCII を含んでよい）。
4. `a != b` で、どちらかが英数字でなければ**停止**（従来のメッセージ `character other than an ASCII letter or digit in unequal strings`）。英字と数字なら**停止**（`letter against digit`）。
5. 共通の長さで差が見つからなければ**停止**（`prefix or case-only difference`）。

**従来との関係**: 従来は「両方の文字列の全ユニットが英数字」を先に要求していた。新しい規則は要求を「最初の差の位置まで」に狭める。全部が英数字の文字列の答えは**変わらない**（tests/locale.rs の `compare_string`、`compare_string_symbol_before_alnum`、停止のテストを、規則の写しで机上確認した: `tools/cmpsim.py`、`cmpsim2.py`。C の実行ではない）。変わるのは、従来「英数字以外を含むので停止」だった組のうち、最初の差が英字どうしか数字どうしで、それより前が英数字のものだけ。

**停止のまま残るもの（理由）**:

- 最初の差より**前**に空白・記号・ハイフン・アポストロフィ・非 ASCII がある組（"a b" と "a c" など）。ページは空白の重みを書かず、ハイフンとアポストロフィは word sort で別扱いと書くだけ。**同じユニットが両側にあれば順序に効かない、という読みは自然だが、ページに書かれていない**ので広げない（PM の判断の点 4）。
- 最初の差の位置で、記号（0072 の位置 0 以外）、空白、ハイフン、アポストロフィ、非 ASCII のどれかが片方でもあるもの。英字と数字の組。
- 接頭辞（一方が他方の頭）、大文字小文字だけの差（flags 0）、空文字列と非空。
- flags が 0 と NORM_IGNORECASE 以外（**SORT_STRINGSORT 0x1000、NORM_IGNORENONSPACE を含む**）。ページは SORT_STRINGSORT でハイフンとアポストロフィを他の記号と同じに扱うと書くが、本決定は ASCII の英数字の順序にしか答えず、フラグを広げる根拠が無い。
- ロケールが emulated（0x409、0x400、0x800）以外（LOCALE_INVARIANT を含む）。
- **非 ASCII の文字**（大文字小文字の対応、正規化、複数文字の要素）。Unicode の重み表は Learn に無い。

### 選択肢

- A. 採用（上）。
- B. 新しく決めるのは位置 0 の差だけにする（最も狭い。unit 283 には足りる）: 次点。差が位置 0 にあれば「前のユニット」が無いので、読みの余地が最小。位置 1 以降の差は従来の「全ユニットが英数字」の規則のまま（従来の答えは変えない）。実装は、従来の 2 つの全走査の前に「位置 0 が英字どうし・数字どうしで違えば答える」を足すだけ。位置 1 以降の差で同じ壁が再び出る（"Segoe UI" と "Segoe Print" などは、前に空白があるので本決定でも停止する）。
- C. 最初の差より前の**同一の**ユニットは何でもよい（空白・記号・非 ASCII を含む）: 「文字ごとに評価する」の文と、同じユニットは同じ重みという前提だけから導けるが、(i) ハイフン・アポストロフィの word sort の扱いが前提に入る、(ii) 複数文字の要素が前の記号に絡まない、という読みが増える。**不採用**（PM の判断の点 4）。
- D. 近似（常に LESS、など）: **不採用**。戻り値は API の仕様として正しくなければならない（0072）。ゲストの鎖では結果は等しいか否かにしか使われない（`sub eax, 2` の後の `test`）が、他の呼び出し点は順序を使う（0072 の調査）。

### 撤回条件

(1) 実機 Windows の測定または Microsoft の別の文書が、最初の差が英字どうし・数字どうしの組で、本規則と逆の順序を示す（例: en-US に ASCII の複数文字の要素がある）。(2) Learn が「最初の差」の規則を別の形で書く。(3) 0072 の撤回条件（記号の位置、空白）が成り立った場合の規則の見直しの際、本決定の「前のユニットは英数字」の条件も同時に見直す。

### テスト

`tests/wall_r2.rs`: `the_pair_of_unit_283_is_decided_at_its_first_unit`（"Default" と "Segoe UI" の両順、lcid 0x400／0x409／0x800 × flags 1／0、長さ -1 と明示長）、`what_follows_the_first_difference_is_not_looked_at`（尾に空白・記号・数字・非 ASCII がある 10 組 × flags 0／1）、`the_old_decided_cases_keep_their_answers`、`what_is_not_decided_still_stops_by_name`（空白の前置、記号の差、非 ASCII の差、英字と数字、接頭辞、大文字小文字だけ、空文字列、SORT_STRINGSORT、NORM_IGNORENONSPACE、他ロケール）。実 DLL: `the_real_dll_compare_text_passes_user_default_locale_and_norm_ignorecase`。既存の `tests/locale.rs` は**変更なし**で通るはず（机上確認。R2 の作業では実行していない）。**統合 M2 で実行し、32 件が通った**（C の実行）。

## W3: unit 830、880 — `shfolder.dll!SHGetFolderPathW` と `CSIDL_APPDATA`

### 原因

| 項目 | 内容（区分） |
|---|---|
| census の記録 | unit 830（proc `0x10d50d0`）: 停止 `fle: indirect call to 0 is not a known function entry`、top `0x10c7cd0`、unit 880（proc `0x1263540`）: 同じメッセージ、top `0x125da90`（**実測**）。census は「e: null 呼び、二次の疑い」（「unit 245 の後」＋「null 呼び」の手掛かりだけの**ヒューリスティック**）。単独構成でも同じ unit が同じ関数で止まる |
| unit 830 の gdb 測定 | 他の担当（レーン S）の記録: `f_10c7cd0` の `0x10c8941`（`call [rbp+0x860]`）が 0 を呼ぶ。0 は `GetProcAddress(LoadLibraryW(L"SHFolder.dll"), "SHGetFolderPathW")` の戻り値。gdb で `fle_imp_GetProcAddress` の `hModule = 0x7ff800110000`（`fle_modules[16]` = `shfolder.dll`）と名前 `SHGetFolderPathW` を確認（**実測**。`lift/out-wallS/docs-new/wall-s-measurement.md` §2.1。**本作業では再測定していない**） |
| unit 880 | `f_125da90`（`0x125da90`〜）: `0x125dc0f` で `lea rcx, L"SHFolder.dll"`（`0x125e7f0`）、`0x125dc16` で LoadLibraryW（thunk `0x423990`）、`0x125dc29` で `lea rdx, L"SHGetFolderPathW"`（`0x125e80a`）、`0x125dc30` で `0x4235f0`（Delphi の `GetProcAddress(HMODULE, PWideChar)` のラッパー。ANSI 名にして kernel32 の GetProcAddress を呼ぶ）、結果を `[rbp+0x198]` に保存し、**NULL 検査なしで** `0x125dc55: call [rbp+0x198]`（`ecx=0`、`edx=0x1a`、`r8=0`、`r9=0`、5 番目 = 局所のバッファ `[rbp+0x1d8]`）。**読み**。unit 830 の `f_10c7cd0` も同じ並び（`0x10c88fb`〜`0x10c8941`。**読み**、上の gdb と整合） |
| 同じ型が 25 か所 | FLEngine に UTF-16 の "SHFolder.dll" が 24 個、"SHGetFolderPathW" が 24 個あり、`xor ecx,ecx; mov edx,0x1a; xor r8,r8; xor r9,r9` の呼び出し準備が**ファイル中に 25 か所**ある。25 か所すべてで、`LoadLibraryW`（thunk `0x423990`）の後に `0x4235f0`（GetProcAddress のラッパー）を呼び、その結果の保存と `call [rbp+…]` の間に `test`／`cmp`／条件分岐が**無く**（25 か所を命令列で走査: `tools/nonull.py`）、**CSIDL は 0x1a（CSIDL_APPDATA）**（25 か所の `mov edx` の値）。**読み**。tests/wall_r2.rs の実 DLL のテストが 24、24、25 を固定する。つまり census で今見えている 830 と 880 は、同じ型の複製のうちの 2 つで、他の複製を含む unit も、そこへ届けば同じ壁を持つ（**読み**。どの unit かは調べていない） |
| 呼び出し後のゲスト | バッファの文字を NUL まで（最大 0x2000 文字）1 つずつ文字列オブジェクトに足す。`FreeLibrary(h)`（thunk `0x4233d0`）。続けて `GetComputerNameExW(0, buf, &0x2000)`（thunk `0x423430`）、`RegOpenKeyExA(HKLM, "SOFTWARE\Microsoft\Cryptography", 0, KEY_READ, &h)`（`0x4230e0`）、成功したときだけ `RegQueryValueExW(h, L"MachineGuid", …)`（`0x423100`）、`RegCloseKey`（**読み**）。つまりこの関数は AppData のパス、コンピュータ名、MachineGuid を 1 つの文字列オブジェクトに集める（マシン識別子を作る関数、というのは用途の**推測**）。パスの内容に依存する分岐は、読んだ範囲（`0x125da90`〜`0x125dde4`）には無い。それ以降（文字列の後処理。`0xbf86f0` など）は**読んでいない**（**未確認**） |
| 戻り値 | ゲストは HRESULT を見ない（`call` の直後に `test eax, eax` が無い。**読み**）。失敗を返してもバッファ（呼ぶ前に先頭を 0 にしてある）が空になるだけで止まらないが、失敗を返すのは環境の主張になる（下の選択肢 B、C） |
| FLEngine 自身の import | `SHFolder.dll!SHGetFolderPathW` は FLEngine の**通常の import**でもある（IAT `0x1971820`、thunk `0x4262e0`、呼び出し元 3 か所 `0x5a0a1c`（CSIDL 5 = CSIDL_PERSONAL）、`0x5a0d1c`（0x1c = CSIDL_LOCAL_APPDATA）、`0x63b8f9`（動的な値）。**読み**）。この thunk は応答表に行が無いので `fle_unsupported`（`imports.txt`: 行 455）。`shell32.dll!SHGetFolderPathW`（IAT `0x1971f98`、thunk `0x593450`）は 0103 が答える |
| 原因の区分 | **一次**（応答の欠落）。census の「二次の疑い」は当たらない（0098、0099 の読みと同じ）。ただし 830 と 880 が**この呼びで止まる**ことは、830 は gdb（他の担当）、880 は読みで確かめた。880 の gdb は無い（**未確認**） |

### shfolder.dll の実機での挙動（一次資料）

| 主張 | 出典 | 区分 |
|---|---|---|
| SHFOLDER.DLL は SHGetFolderPath を Windows 95／98／NT 4.0 にも提供する再配布 DLL で、**「Windows 2000 includes this DLL as a protected system file and, as such, this DLL cannot be replaced on Windows 2000 or greater」** | Learn（アーカイブ）[Appendix A: Best Practices](https://learn.microsoft.com/en-us/previous-versions/ms954149(v=msdn.10))（Windows 2000 のアプリケーション仕様の A-2。取得済み） | Learn（アーカイブ。**Windows 2000 の記述**） |
| 「SHFolder.dll always calls the current platform's version of this function. If that fails, it tries to simulate the appropriate behavior. SHFolder.dll continues to be included for backward compatibility, but the function is now implemented in Shell32.dll」 | 旧 MSDN の SHGetFolderPath の Remarks。Wine-devel（2004）、Stack Overflow（2011）などが**転載**している（検索で取得）。**現行の Learn の SHGetFolderPathW のページ（取得）にはこの文がない**。旧ページ（`previous-versions` の `bb762181(v=vs.85)`）は 404 だった | **参考（非公式の転載）**。Learn で直接は**未確認** |
| 現行の SHGetFolderPathW: 「Deprecated」、DLL は「Shell32.dll (version 5.0 or later)」、「As of Windows Vista, this function is merely a wrapper for SHGetKnownFolderPath. The CSIDL value is translated to its associated KNOWNFOLDERID」。サポートする CSIDL の例に **CSIDL_APPDATA**、CSIDL_LOCAL_APPDATA、CSIDL_PERSONAL ほか | [SHGetFolderPathW](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shgetfolderpathw)（`updated_at` 2025-07-01） | Learn |
| **Windows 10 に shfolder.dll がある**こと | Learn のどのページにも書かれていない（**未確認**）。FLEngine は `SHFolder.dll!SHGetFolderPathW` を**通常の import 表**で読む（`imports.txt`: `normal .idata SHFolder.dll name SHGetFolderPathW`）。DLL が無い、またはその名前が export に無い環境ではプロセスの起動が失敗するので、この製品が Windows 10 で動くなら、DLL と export は存在する、という推論は成り立つ | **推測**（読み。決定 0015 が shfolder.dll を PE の import 表から仮想モジュール N = 17 にした根拠と同じ） |

結論: PM の前提「現行の Windows には存在する」は、**Learn では確認できず**、製品の import 表からの推論に基づく。応答の設計は、(i) 行を足して GetProcAddress と PE の import thunk の両方に同じ答えを返す、(ii) 実装は `shell32.dll!SHGetFolderPathW` と**共有**する（旧 MSDN の転載が言う「現行プラットフォームの実装を呼ぶ」の形。現行の Learn も SHGetKnownFolderPath の薄い wrapper と書く）。

### 仕様（Learn）

- [SHGetFolderPathW](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shgetfolderpathw): S_OK を返す。パスは MAX_PATH のバッファで「The returned path does not include a trailing backslash」。「Only real folders are valid. If a virtual folder is specified, this function fails」。SHGFP_TYPE_CURRENT／DEFAULT は、フォルダが移動・リダイレクトされうるときの区別（「If the folder has not been redirected, then SHGFP_TYPE_DEFAULT and SHGFP_TYPE_CURRENT retrieve the same path」）。hToken は「usually set to NULL」。
- [CSIDL](https://learn.microsoft.com/en-us/windows/win32/shell/csidl): 「CSIDL_APPDATA — FOLDERID_RoamingAppData — The file system directory that serves as a common repository for application-specific data. A typical path is C:\Documents and Settings\*username*\Application Data.」（Windows XP の典型例）。「CSIDL_PROFILE … A typical path is C:\Users\*username*」。CSIDL_FLAG_CREATE と CSIDL_FLAG_DONT_VERIFY の意味。**ページは数値を書かない**（0x1a は mingw-w64 の `shlobj.h:30`／`shfolder.h:28`。非公式。ゲストの `mov edx, 0x1a` と一致）。
- [KNOWNFOLDERID](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid)（取得）: 「FOLDERID_RoamingAppData … Default Path %APPDATA% (%USERPROFILE%\AppData\Roaming) … CSIDL Equivalent CSIDL_APPDATA」、「FOLDERID_Profile … Default Path %USERPROFILE% (%SystemDrive%\Users\%USERNAME%)」。

### 決定（仮）

1. 応答表の**末尾に 1 行**: `("shfolder.dll", "SHGetFolderPathW", "fle_imp_SHGetFolderPathW", "jmp_imp:SHFolder.SHGetFolderPathW")`。runtime の関数は `shell32.dll!SHGetFolderPathW` と**同じ** `fle_imp_SHGetFolderPathW`（疑似 VA は別。統計の名前は別にして、2 つの thunk を別々に数える）。shfolder.dll は既に仮想モジュール N = 17（決定 0015。モジュールの表は**変えない**）。`SHGetFolderPathA` など他の export には行を足さない（FLEngine は使わない）。
2. `fle_imp_SHGetFolderPathW` に **CSIDL_APPDATA (0x1a)** を足す: S_OK（0）と `C:\Users\user\AppData\Roaming`（29 文字。末尾に `\` なし。NUL を含めて書く）。hToken は NULL、dwFlags は 0 か 1（同じパス）、pszPath 非 NULL（0103 と同じ条件）。それ以外の csidl（CSIDL_FLAG_CREATE 0x8000 と CSIDL_FLAG_DONT_VERIFY 0x4000 を足した値を含む）は従来どおり名前付き停止（メッセージは `csidl 0x.. is not answered (only CSIDL_FONTS 0x14 and CSIDL_APPDATA 0x1a are)`。冒頭の `csidl 0x24 is not answered` は既存テストの部分一致が通る）。CSIDL_FONTS（0103）の答えは変えず、shfolder.dll の行からも答える。
3. パスの組み立て: `fle_env.h` に `FLE_VIRTUAL_USER_PROFILE_DIR "C:\\Users\\user"` を足し、`fle_env.c` が `\AppData\Roaming`（`FLE_APPDATA_SUBDIR`）を足す。**ユーザー名 "user" は選択**（出典なし。この環境に他のユーザー名はない。PM の判断の点 3）。
4. フォルダの**存在は確かめない**（仮想ボリュームにこのフォルダを作らない）。0103 の `C:\Windows\Fonts`、0018 の `GetWindowsDirectoryW`（`GetFileAttributesW("C:\Windows")` は 2 で失敗する）と同じ扱い。実機では存在しない場合に S_FALSE（旧ページの記述）になりうるが、この環境が「プロファイルのフォルダが無い」と主張するのは避ける（0073 の方針）。結果として、ゲストが `CreateDirectoryW(APPDATA + "\\Image-Line")` を呼べば 0018 の規則で ERROR_PATH_NOT_FOUND になる（親が仮想ボリュームに無い）。これが問題になる実行に届いたときは別の決定（PM の判断の点 5）。

### 選択肢

- A. 採用（行を足し、関数を共有、CSIDL_APPDATA に成功と仮想パス）。
- B. APPDATA に失敗（E_FAIL など）か S_FALSE（空のパス）を返す: **不採用**。ゲストは HRESULT を見ず空のパスで続けるので止まらないが、実機の Windows 10 では常に成功する API に「フォルダが無い」と答えることになる（0073、0103 の方針）。
- C. サービス アカウントのプロファイル（`C:\Windows\System32\config\systemprofile\AppData\Roaming` など）: 0018 の GetTempPathW が `C:\Windows\` になる（環境変数が空）ことと整合するが、出典（Learn）が無く、対話ユーザーの典型例（CSIDL の表、KNOWNFOLDERID）から離れる。**不採用**（PM の判断の点 3 に残す）。
- D. shfolder.dll の行を足さず、ゲストの `GetProcAddress` を NULL のまま（現状）: 止まる。または shfolder.dll を `fle_load_absent[]` に足す（`LoadLibraryW` が NULL を返し、ゲストは NULL で `GetProcAddress`、さらに NULL を呼ぶ）: **不採用**。FLEngine が通常の import で読む DLL を「無い」とは言えない（0015、0073）。
- E. 別の実装（shfolder 専用）: **不採用**。旧 MSDN の転載は shfolder が現行の実装を呼ぶと書き、現行の Learn も SHGetKnownFolderPath の wrapper と書く。2 つの入り口は同じ答えであるべき。

### 撤回条件

(1) 実機 Windows 10 の `SHGetFolderPathW(NULL, CSIDL_APPDATA, NULL, 0, buf)`（ユーザー名のある対話ユーザー）の値が示され、構造が `C:\Users\<name>\AppData\Roaming` でないと分かる。(2) PM がユーザー名を決める（環境の定義にユーザー名が入る）— そのとき `FLE_VIRTUAL_USER_PROFILE_DIR` の 1 か所を直し、`GetUserNameW` などと一緒に整合させる。(3) 仮想ボリュームにプロファイルのフォルダを作る決定が入る（初期状態が空でなくなる。0018）。(4) shfolder.dll が現行の Windows 10 に存在しない、または `SHGetFolderPathW` を export しないと分かる（FLEngine の import 表と矛盾する。行を外し、通常の import の thunk も外す）。(5) Learn が shfolder.dll の挙動を別に書く。

### テスト

`tests/wall_r2.rs`: 生成器 — `the_shfolder_row_follows_the_shell32_row_of_the_same_function`（行の順、VA は 0x10 刻み、shfolder の行は 1 つだけ、関数は 1 つで疑似 VA が 2 つ）、`both_sh_get_folder_path_w_thunks_of_flengine_are_answered_under_their_own_dll`（`SHFolder.dll`／`shell32.dll` の thunk が答えられ、`user32.dll` の同名は答えられない。統計の名前が別）。runtime — `shfolder_has_the_shget_folder_path_w_row_and_no_other`、`csidl_appdata_is_the_virtual_roaming_app_data_folder_through_either_dll`（両 DLL × SHGFP 0／1、CSIDL_FONTS も、バッファの前後）、`the_guests_machine_id_sequence_gets_a_non_null_function_and_the_appdata_path`（`LoadLibraryW(L"SHFolder.dll")` = `0x7ff800110000`、GetProcAddress が NULL でない、パス、`FreeLibrary` が成功）、`the_other_csidl_values_flags_tokens_and_a_null_buffer_stop_by_name`。実 DLL — `the_real_dll_has_the_two_thunks_and_25_call_setups_with_csidl_1a`（thunk の IAT スロット、文字列 24 と 24、呼び出し準備 25）。

## d: 上以外の壁

| 壁（census） | unit | 扱い |
|---|---|---|
| `LoadLibraryW "iltools_x64.dll"` | 245、280、659 | **別の担当**。触らない |
| `LoadLibraryW "dsp_ippv2_x64.dll"`（単独構成） | 144 | **扱わない**。単独構成の定義は dsp_ippv2_x64.dll を載せないこと（0101: 単独は下位指標）。応答する対象ではなく、「無い」と定義する（`fle_load_absent[]`）のは 0073、0086、0101 に反する |
| null 関数ポインタ呼び（単独構成だけ） | 170、833、935 | **扱わない**。dsp 構成では ok、単独では放棄（census の比較: `report.md`「測定した依存」）。読み: 833 の proc `f_10d79f0` は `sub [0x16bc6ec], 1; cmp [0x16bc6ec], -1; jne skip` の後で `mov rax, [0x15e0a48]; call [rax]`（`0x10d7a10`〜`0x10d7a17`）を呼び、170 の `f_675290`（カウンタ `[0x15f47f0]`）が呼ぶ `f_672600` も `mov rax, [0x15e0a48]; call [rax]`（`0x672644`〜`0x67264b`）を持つ。カウンタが -1 になる（0 のまま 1 回引かれた）ときだけ入る経路で、カウンタと `[0x15e0a48]` の先の設定は、放棄された前の unit が行うはず、という読みだが、**設定する unit は特定していない**（**未確認**）。935 は `0x13709d0`（Delphi のコンストラクタ風）で、同じ型かは調べていない |
| 830、880 の次の壁 | — | census は 1 unit 1 壁しか見せない。**読みの予測**（実測でない）: 上の「呼び出し後のゲスト」のとおり、次は **`kernel32!GetComputerNameExW`**（応答表に行が無い。`jmp [IAT]` の thunk `0x423430` は `fle_unsupported`）。その先は `RegOpenKeyExA(HKLM, "SOFTWARE\Microsoft\Cryptography")`（仮想レジストリは `HKLM\Software` だけ空のキーとして存在する（0085）ので、`Microsoft` が無く ERROR_FILE_NOT_FOUND のはずで、ゲストは MachineGuid の読みを飛ばす）。`GetComputerNameExW` は**環境の主張**（コンピュータ名）なので、本決定では答えない（0034 が同じ理由で未対応のまま。PM の判断の点 6） |

## 実装した場所

新しい `.c`、`.h` は作っていない（`tests/modules.rs` が全 `fle_*.c` をリンクするため）。

- `lift/runtime/fle_sysmetrics.h`／`.c`: index と値の定数、`case` 4 つ、`undecided[]` から 4 件を外す。
- `lift/runtime/fle_locale.c`: `fle_imp_CompareStringW` の最後のループとコメント（`fle_is_*` の補助は変えない）。
- `lift/runtime/fle_env.h`／`.c`: `FLE_VIRTUAL_USER_PROFILE_DIR`、`FLE_CSIDL_APPDATA`、`FLE_APPDATA_SUBDIR`、`fle_imp_SHGetFolderPathW` の分岐とコメント。
- `lift/crates/fe-lift/src/imports_table.rs`: `TABLE` の**末尾に 1 行**（コメント付き）。行数 310 → 311（統合では決定 0106 の `gdi32!CreateBitmap` の行が先、この行が後ろで 312 行。疑似 VA は `CreateBitmap` が 0x7ff900001370、この行が 0x7ff900001380）。既存の擬似 VA は動かない（0015）。
- テスト: 新規 `tests/wall_r2.rs`（17 件）。更新した既存テストは次の節。

## テスト

### 新規 `lift/crates/fe-lift/tests/wall_r2.rs`

生成器 2 件（W3）、runtime 12 件（W1 が 4、W2 が 4、W3 が 4）、実 DLL 3 件（W1、W2、W3 に 1 件ずつ。DLL が無ければ飛ばす）、合計 **17 件**（`#[test]` の個数。名前は各壁の「テスト」の節）。runtime のテストは、模擬 PE の生成 C と全 `fle_*.c` をリンクした 1 つのドライバ（`tests/wall_s.rs` と同じ形）で、GetProcAddress が返す疑似 VA を `fle_dispatch` で呼ぶ。期待値は決定と Learn から書き、C のコードから読み戻していない。

### 更新した既存テスト（固定数・停止の主張が、この決定で偽になったもの）

| ファイル | 行 | 内容 | 理由 |
|---|---|---|---|
| `tests/sysmetrics.rs` | `ANSWERED` | `[(u64, i32); 15]` → `[(u64, i32); 19]`（`(86, 0)`、`(95, 0)`、`(0x2003, 1)`、`(0x2004, 0)` を足す）。テスト名 `the_fourteen_answers` → `the_nineteen_answers`。**統合では決定 0106 の `(13, 32)`、`(14, 32)` も入り `[(u64, i32); 21]`、テスト名は `the_twenty_one_answers`** | 4 つの index に答える |
| `tests/sysmetrics.rs` | `UNDECIDED` | `[u64; 21]` → `[u64; 17]`（86、95、0x2003、0x2004 を外す）。**統合では決定 0106 の 13、14 も外れて `[u64; 15]`**（ANSWERED 21 + UNDECIDED 15 = 36 = ゲストが使う index の数） | 同上 |
| `tests/sysmetrics.rs` | 2 つのコメント | 「0x2003、0x2004 は UNDECIDED」→「答える（0107）」ほか | 事実の更新 |
| `tests/wall_r.rs` | `the_other_metrics_of_the_same_guest_function_are_still_undecided` | 名前を `…_are_answered_by_decision_0107` に変え、停止の検査を 4 つの値（0、0、1、0）と last error の不変の検査に置き換えた | 86、95、0x2003、0x2004 が停止しなくなる |
| `tests/vcall_thunk_dll.rs` | `after_the_memory_form_jmp_only_the_import_thunks_remain_unsupported` | `unsupported_total` と `unsupported["Jmp: indirect through IAT slot (not lifted)"]` の 515 → **514**（2 か所）。コメントに 0107 の分を足した。**統合では決定 0106 の `gdi32!CreateBitmap` の thunk（FLEngine の `0x423f30`）も答えになり 513**（生成 C の統計行 `unsupported_total` 513 と、テストが通ることで確認） | thunk `0x4262e0`（`SHFolder.dll!SHGetFolderPathW`）に答える行ができ、未対応の thunk が 1 つ減る。thunk は `--pdata` の閉包に入っている（`unsupported-inventory-pdata.md` 123 行: `shfolder.dll 1: SHGetFolderPathW(1/3,u=2)`。**読み**） |

**変更の要らない固定数**（`grep` で確かめた。変えていない）: 応答表の行数（`resp_entries` を動的に使う。310 という数はテストに無い）、`fle_modules[]` の数 35（モジュールを足さない）、`spi.rs` の `fle_st(` 件数（この決定は SPI に触れない。決定 0106 の 0x101A で 12 → 13 になるのは 0106 の側）、`imports_table.rs` の行の個数（`wall_q_lift.rs` ほかの「正確に 1 行」の検査は、行の全文（DLL 名を含む）で数える。新しい行は DLL 名が違うので 1 のまま）、`wall_q.rs` の `shgfp-csidl`（0x24）、`shgfp-flag-create`（0x8014）の停止（メッセージの冒頭が同じ）。`locale.rs` は変更なし（机上確認）。

## 検証

- **第 1 段階でやったこと**: `clang -fsyntax-only -std=gnu11 -O0 -Wall -Wextra -Werror -fPIE`（変更した 3 つの `.c`）と gcc `-O2 -Wall -Wextra -Werror` の構文検査 — 警告なし。テストのドライバ（`wall_r2.rs` の C の文字列）を取り出して `clang -fsyntax-only -Wall -Wextra` — 警告なし。`rustfmt --check` で `wall_r2.rs` の構文を確かめた（書式の差分は出るが、パースのエラーは無い）。`fle_imp_CompareStringW` の新しい分岐を Python に写して、新規・既存の期待値（`tools/cmpsim.py`、`cmpsim2.py`）を確かめた — 不一致 0（C の実行ではない）。
- **統合 M2 でやったこと（HEAD 520064a に決定 0106、この決定の順に当てた木。dev プロファイル）**: cargo のビルド（警告 0。R2 の Rust に型・構文の誤りはなく、直した箇所は無い）、全体テスト（グループ分けの合算 112 test result・**1,428 passed・0 failed**。`wall_r2.rs` の 17 件は初回の実行で全件通った。更新した 3 ファイル（`sysmetrics`、`wall_r`、`vcall_thunk_dll`）も通った。runtime の C のコンパイル・リンク・実行は、これらのテストの中で行われる）。`FLENGINE_PE`、`FLENGINE_DLL` を実ファイルに向けたので、実 DLL のテスト 3 件も skip されずに通った。
- **やっていないこと**: census の再測定、a4 の測定、変異検査（`wall_r2.rs` のテストが偽の実装を落とすかの検査）、wasm。
- **生成 C への影響（予測と実測）**: 予測の (1) `fle_dispatch_table` と `fle_resp_table` に 1 行ずつ（`fle_dispatch_count`、`fle_resp_count` が +1）、(2) 宣言 `void fle_imp_SHGetFolderPathW(void);` がもう 1 行、(3) **thunk `0x4262e0` の 1 行**（`fle_unsupported(UINT64_C(0x4262e0))` → `fle_imp_SHGetFolderPathW(); return;`）は、統合した木の FLEngine の C を HEAD の C と関数単位（59,380 関数のハッシュ表）で比べて**そのとおりだった**: 本体が変わった関数は `f_4262e0`（この決定。thunk は `--pdata` の閉包に入っていた）と `f_423f30`（決定 0106 の `CreateBitmap` の thunk）の 2 つだけで、ほかの 59,378 は同一。表と宣言の差は、決定 0106 の分と合わせて 2 行ずつ（個数 59,690 → 59,692、310 → 312）。FLEngine の C の SHA-256（`fe-lift lift FLEngine_x64.dll OUT.c --pdata` のファイル）: HEAD `8789c1a5…`（215,747,179 バイト、`unsupported_total` 515）→ **この決定だけを当てた木 `07e9a86c8252ec4e1b65ea72f4c26f559382d8a32ef75f0c5ce05a5b068b1591`（215,747,411 バイト、514。差は +232 バイト）** → 統合した木 `794d26cb38f6aad5c92390f8fe1bcaa740d7c57f28840382cd4d56898cb0bcdd`（215,747,616 バイト、513）。`harness/a4-multi.sh` の既定の `EXPECT_FLE_SHA` は統合した木の値に更新した。dsp_ippv2、Blend2D、QFC、freetype の C は測っていない（「応答表の 1 行と個数だけが変わるはず」という予測のまま）。
- **census への影響（予測。未測定）**: unit 276 は 4 つの index を通る（その先は未測定）。unit 283 は CompareStringW を通る（その先は未測定）。unit 830、880 は SHGetFolderPathW を通り、予測では次に `GetComputerNameExW` で止まる。これらの unit が ok になるかは**わからない**。

## PM の判断が要る点

| # | 点 | 実装した最小の案 | 他の選択肢 |
|---|---|---|---|
| 1 | SM_CONVERTIBLESLATEMODE (0x2003) と SM_SYSTEMDOCKED (0x2004) の値 | 0x2003 = 1（スレートでない）、0x2004 = 0（ドッキングなし） | 0x2003 = 0（OEM の既定値が 0 = tablet。デスクトップで 0 になるかもしれない、という実機の挙動は未確認）、0x2004 = 非 0。**SM_DIGITIZER = 0 の間、ゲストの分岐は変わらない**（読み）。タッチを入れる決定が入ったとき、0x2003 の値が分岐を変える |
| 2 | SM_TABLETPC = 0 | ページの文言（Windows 10 は条件に入らない）とデジタイザ無しから導いた | ページは Windows 10 を書かない。実機の値は未確認 |
| 3 | CSIDL_APPDATA のユーザー名と形 | `C:\Users\user\AppData\Roaming`（ユーザー名 "user" は選択） | "User" など別名、サービス アカウントのプロファイル（0018 の GetTempPathW が `C:\Windows\` になることと整合）、失敗・S_FALSE（不採用の理由は選択肢 B）。ユーザー名は `GetUserNameW`、`USERPROFILE` などの将来の決定と整合させる必要がある |
| 4 | CompareStringW の「最初の差」の範囲 | 最初の差の前が ASCII の英数字（同一）であることを要求し、後ろは見ない | B: 新しく決めるのは位置 0 の差だけ（従来の全走査を残して、位置 0 の判定を足す）。C: 最初の差の前は同一なら何でもよい（空白・記号を含む。導けるが読みが増える） |
| 5 | プロファイル・システムのフォルダを仮想ボリュームに作るか | 作らない（0103、0018 と同じ）。SHGetFolderPathW は存在を確かめず S_OK | 初期状態に `C:\Users\user\AppData\Roaming` などを作る（0018 の「初期状態は空」の変更）。ゲストが `CreateDirectoryW(APPDATA\…)` を呼ぶ実行に届いたときに必要になる |
| 6 | `GetComputerNameExW` ほか「環境の主張」の束（A11） | 答えない（本決定の外） | 830、880 の次の壁の予測。値（コンピュータ名）は PM の決定 |
| 7 | 他の CSIDL | 答えない（0x14 と 0x1a だけ）。FLEngine の SHFolder import の呼び出し元に 5（PERSONAL）、0x1c（LOCAL_APPDATA）、動的な値がある（読み） | 実行がそこへ届いたときに個別に決める |

## 解釈が分かれた点

1. **census の分類を信じるか**: 830、880 の null 呼びを「二次の疑い」とする census のヒューリスティックは当たらない（原因は応答の欠落）。本決定は 0098、0099 の読みと、他の担当の gdb を採った。
2. **shfolder.dll の行を足すこと**: 「無い」と定義せず（0073）、実在の import に応答する。Windows 10 での存在は Learn で確認できず、製品の import 表からの推論（上の表）。
3. **GetSystemMetrics の 0x2003**: 「デスクトップには適用されない」の一次資料の空白を、OEM 向けの Learn の推奨（Desktop には 1）で埋めた。この結び付き（OEM 設定と GetSystemMetrics の値）は書かれていない推論。
4. **CompareStringW**: ページの「文字ごとに評価する」を、最初の差が決める、と読んだ。複数文字の要素は英語（米国）に無い、という既存の仮定（英数字の文字列で既に使っている）を、より多くの入力に広げた。
5. **依頼文の「SPI 0x2003、0x2004」**: GetSystemMetrics の index（SM_*）であり、SystemParametersInfoW の uiAction ではない（逆アセンブルで確認）。

## 限界

- **実機の Windows と照合していない**。応答はすべて仮の決定。特に 0x2003 の値、0x2004 の値、ユーザー名 "user"、SM_TABLETPC の Windows 10 の値は出典のない選択か、ページが書かない値。
- R2 の作業者はコンパイル・テストを行っていない（第 1 段階。構文検査と机上の確認だけ）。統合 M2 でコンパイルと全体テストを行った（上）。**census・a4 の実測、変異検査は行っていない**。
- 各 unit で見えるのは最初の壁だけ。830、880 の次の壁は読みの予測。276、283 の先は何も分からない。
- unit 880 の null 呼びは gdb で確かめていない（830 は他の担当の gdb。880 は読み）。unit 283 の文字列は gdb で読んでいない（読み）。
- 同じ型の関数（SHFolder + APPDATA + マシン識別子）が FLEngine に 24 複製ある。census が今見ているのは 830 と 880 だけで、他の複製を持つ unit に届けば、同じ答えを返す（それらの unit で別の壁が出るかは分からない）。
- `CompareStringW` の新しい規則は、より多くの入力に答える。実機との照合が無いので、仮定（英語（米国）に ASCII の複数文字の要素が無い）が崩れれば、広い範囲で答えが変わる。
- 応答表の末尾は決定 0106 の `gdi32!CreateBitmap` の行と重なったが、統合では 0106 が先、この行が後ろ（312 行目）で解決した。
- wasm 版はビルドしていない。
