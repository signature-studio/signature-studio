# 0116: `CreateFruityInstance` の壁 — 登録の種付け（`Shared data`）、`SHGetSpecialFolderLocation`／`SHGetPathFromIDListW`、`GetCPInfo(CP_UTF8)`。4 つ目の壁 `WinVerifyTrust` で停止（署名検査の判断が要る）。すべて仮の決定

状態: **仮の決定（すべての応答と選択）**。依頼は PM（2026-10-28。「壁を 1 つずつ『原因の特定 → 最小の実装 → 再測定』で連続して進める。最大 12 壁。止まる条件に当たったら止める」。番号 0116 は PM の仮の割り当て）。測定・実装・テスト・文書は作業者（サブエージェント）が行った。**HEAD f5e9552 の clean な `git archive` に差分 `fruity3.diff` を当てた木で、コンパイル（警告・エラーなし）、全体テスト（116 test result・1,594 passed・0 failed）、生成 C の関数単位の比較、変異検査 10 件（10 件検出）、6 モジュール構成の FRUITY 有効の測定（独立な 2 回のビルド）と無効のビルド 1 回まで実施した。実機 Windows との照合は 0 件。wasm は未確認（native だけ）。**

**測定用の基底・種・註釈（dsp 0x2000000、Blend2D 0x4700000、QFC 0x1d00000、freetype 0x4b00000、ILTools 0x6000000、フォント 2 ファイル）と、仮の応答を含む構成での値である。** 決定 0101 の表記では「A4 完走（仮の応答 N 件を含む）」の形の値で、達成の判定は PM が行う。

**肯定的に見せる点は少ない。** `CreateFruityInstance` は、エンジンの完全性の検査（自分の DLL の Authenticode 署名）の手前で、呼び出しの **35,096 関数入口目**（0.06 秒）まで進んで止まった。最初の停止点（4,693 入口目）から約 7.5 倍。署名の壁の先には、ウィンドウ（VCL の `TApplication`）、メッセージループ、音声、スレッド等が控えている（実験で 1 歩先だけ見た）。

区分の凡例: **実測**＝コマンドの出力を見た事実。**読み**＝逆アセンブルやコードを読んだ上での解釈。**Learn**＝Microsoft Learn のページに書かれていること（取得日 2026-10-28）。**一次**＝NSIS のソース（https://raw.githubusercontent.com/kichik/nsis/master/ 、取得日 2026-10-28）。**出典なしの選択**＝ページにも実測にも根拠が無く、作業者が選んだ値。**未確認**。

## 先に結論

| 項目 | 内容 |
|---|---|
| `CreateFruityInstance`（種あり） | **戻らない**。最後の停止点: `fle: unsupported instruction at 0x4256d0`（`wintrust.dll!WinVerifyTrust` の thunk `f_4256d0`。ゲスト側は `CreateFruityInstance` 自身の 0x1375bce から呼ぶ。戻り先 `0x1375bd3`）。**35,096 関数入口目**、新規の関数 312（種 = `reg-seed-nsis.tsv` + `reg-seed-app.tsv`。app だけなら 35,093／313）。`rax` なし、`instance` = 0 のまま、ハーネスの終了コード 134。独立な 2 回のビルドで同一（native `d361c921bd9fbfd7811473a9fe997940c986222823cedd6187a70fe3a0390723`） |
| 直した壁 | **3**（壁 1: `GetFileAttributesW("")`、壁 2: `SHGetSpecialFolderLocation`／`SHGetPathFromIDListW`、壁 3: `GetCPInfo(65001)`）。4 つ目（`WinVerifyTrust`）は停止条件 (b) で止めた |
| 前提の訂正（重要） | **インストーラの NSIS スクリプトは `Shared data` を書かない**（読むだけ。入口 402）。依頼は「インストーラが書く値」としたが、この値は**インストール後にアプリが書く**（かもしれない。インストーラ内のプラグイン `ILSetup.dll` が書く可能性は未確認で残る）。値は**出典なしの選択** |
| 実験（別枠。差分に入れない） | **B（`WinVerifyTrust` が `0x800B0100` を返す = 検査が通らない）: `CreateFruityInstance` が戻った**。`rax=0xfffffffe`（−2）、`instance=0`、35,101 入口、終了 0。FL64.exe ならここで「Verification error」を出す（読み）。**A（`WinVerifyTrust` = 0 と crypt32 の偽の応答 = 嘘）: 次の停止は `kernel32!RaiseException` 0xC06D007F（遅延ロードの `GetUserPreferredUILanguages`）、それを答えると `user32!GetActiveWindow`（38,238 入口。VCL の `TApplication` の初期化）** |
| 全体テスト | **116 test result・1,594 passed・0 failed**（基準 115・1,575 + 新規 19 件） |
| FRUITY 無効の既定ビルドの native | 基準 `9c324428…`（d7336ad 以降）→ **`8855bff412d29f16ad8e4914d76727375bbfb8a31d6d2038a5e1ac235ea2590e`**（変わった。理由は「検証」）。a4 は変わらない（942/942、895/895、functions 5,306、entries 1,007,942、終了 0） |

## 背景

- 前段（`docs/investigations/create-fruity-instance-measurement.md`）: DllMain/InitUnits の完走後に FL64.exe と同じ形で `CreateFruityInstance(0xC0, 0xEDE95A, 偽ホスト, &instance)` を呼ぶハーネス（コミット 0020f8e）の測定で、最初の壁は `fle: GetFileAttributesW: empty path is not defined`（4,693 入口）。設計書の予想（最初の停止は `WinVerifyTrust`）は外れていた。
- 方針（PM）: ふり（fail-soft）で進めない。仕様から導ける応答だけを実装し、導けないものは名前付き停止。`WinVerifyTrust` に黙って成功を返さない（判断点 J5）。応答表は末尾追記のみ。新しい `.c`／`.h` を runtime に作らない。既存の関数の本体・応答の値は変えない。
- 土台（実測）: HEAD f5e9552 の `git archive`、`CARGO_TARGET_DIR` を専用に。FRUITY 無効の基準を再現した（TAG `t2331d00e8ebb`、a4 942/942、895/895、functions 5,306、entries 1,007,942、native `9c324428…`）。FRUITY 有効の種なしで壁 1 を再現した（4,693 入口、native `8a54acc6…`）。

## 壁の表

各壁の記録は `wall-log.md`。

| # | 停止メッセージ | 原因（区分） | 修正（場所） | 根拠 | 出典なしの選択 | 測定（呼び出しの入口数／新規の関数） |
|---|---|---|---|---|---|---|
| 1 | `fle: GetFileAttributesW: empty path is not defined` | エンジンの `f_652e60` が HKCU `Software\Image-Line\Shared\Paths` の `Shared data` を開けず（3 回とも 2。仮想レジストリは空）、既定の組み立て `f_653130` が `DirectoryExists('')` を呼ぶ（読み + 実測。環境の主張） | 登録の種付けを新設（`harness/reg_seed.h`、`a4-multi.sh` の `REG_SEED`、`main_two.c` の `#ifdef FLE_FRUITY` の中の 1 呼び出し）。種 `reg-seed-app.tsv`（`Shared data` = `C:\Users\user\Documents\Image-Line\`）、`reg-seed-nsis.tsv`（インストーラ由来の 5 行） | NSIS のエントリ 402（ReadRegStr のみ）、`nsis-registry.md`。Learn（GetFileAttributesW）は空の名前に何も書かない | `Shared data` の**値**。形はエンジン自身の既定（`<CSIDL_PERSONAL>\Image-Line\`）、`C:\Users\user\Documents` は Learn の KNOWNFOLDERID の表（FOLDERID_Documents の既定）と決定 0107 のユーザー名 `user` | 4,693／157 → 6,806／180 |
| 2 | `fle: unsupported instruction at 0x593370`（`shell32.dll!SHGetSpecialFolderLocation`） | 未応答の import。Delphi の `f_63ab90` が `SHGetSpecialFolderLocation(NULL, 0x1a, &pidl)` → `SHGetPathFromIDListW` → `CoTaskMemFree`（実測 + 読み） | `fle_ole.c` に 2 関数、応答表の末尾に 2 行（`shell32.dll!SHGetSpecialFolderLocation`、`shell32.dll!SHGetPathFromIDListW`）。CSIDL_FONTS と CSIDL_APPDATA だけ。PIDL = 12 バイトの `CoTaskMemAlloc` ブロック | Learn: SHGetSpecialFolderLocation（hwnd は Reserved、ppidl は CoTaskMemFree で解放、S_OK）、SHGetPathFromIDListW（MAX_PATH のバッファ、TRUE／FALSE） | PIDL の中身（SHITEMID の形は文書化、abID の内容は選択）。E_OUTOFMEMORY の番号（到達しない） | 6,806／180 → 34,029／304 |
| 3 | `fle: GetCPInfo: code page other than the emulated ANSI and OEM code pages`（rcx = 0xfde9） | Delphi の `TEncoding` が `GetCPInfo(CP_UTF8)`（実測）。仕様に書かれていない値 | `fle_imp_GetCPInfo` に 65001 の分岐（MaxCharSize 4、DefaultChar `?`、LeadByte 0） | Learn: CPINFO（LeadByte は UTF-8 で与えられない、DefaultChar は通常 `?`、MaxCharSize は「2 より大きい値」） | **MaxCharSize = 4**（UTF-8 の最長の 4 バイト。RFC 3629。Microsoft のページにない） | 34,029／304 → 35,093／313（nsis 種も足すと 35,096／312） |
| 4 | `fle: unsupported instruction at 0x4256d0`（`wintrust.dll!WinVerifyTrust`） | エンジンが自分の DLL の Authenticode 署名を検査（`CreateFruityInstance` の 0x1375bce）。**停止条件 (b)**。応答は実装していない | （なし。下「止まった壁」） | 実測（ハーネスの引数解読）と読み | — | 35,096 で停止 |

壁 2 の途中の誤り（実測）: 最初は 2 関数を `fle_env.c` に書いたが、`fle_env.c` を `fle_ole.c` なしでリンクするテスト（`fsx_common`。vfs／reg／env）がリンクエラーになるので、`fle_ole.c`（PIDL が `CoTaskMemAlloc` のブロックなのでそちらが自然）に移した。

## 止まった壁（壁 4: `WinVerifyTrust`）の選択肢と推奨

**何が要求されているか（実測 + 読み）**: エンジンは `CreateFruityInstance` の中で、(1) `WinVerifyTrust(NULL, WINTRUST_ACTION_GENERIC_VERIFY_V2, {FILE: "C:\Program Files\Image-Line\FL Studio 21\FLEngine_x64.dll", StateAction VERIFY})` を呼び、戻り値が **0 でなければ** −2 で終わる。0 なら (2) `CryptQueryObject`（ファイルの埋め込み PKCS#7）→ `CryptMsgGetParam`（signer info）→ `CertFindCertificateInStore` → `CertGetNameStringW`（表示名）→ `L"Image Line NV"` との比較、(3) `StateAction CLOSE` の 2 回目、(4) `Flags` のビット 0 が 0 なら（FL64.exe はそう）同じことをホスト exe にも行う。ファイル `…\FLEngine_x64.dll` は**仮想ボリュームに無い**（種付けしたのはフォント 2 つだけ）。Learn（WinVerifyTrust）: 「信頼できるなら 0。0 以外を成功と見なしてはならない」「戻り値は信頼プロバイダー固有、または TRUST_E_SUBJECT_NOT_TRUSTED などのエラー」。

| 案 | 内容 | 結果（実測／読み） | 作業量 | 偽りか |
|---|---|---|---|---|
| A. 失敗を答える | ファイルが仮想ボリュームに無い（現状）から信頼できない、として 0 以外（実験は `TRUST_E_NOSIGNATURE` 0x800B0100） | **`CreateFruityInstance` が戻る: `rax=0xfffffffe`、35,101 入口**（実験 B。実測）。エンジンは動かない（ゲートが閉じる）。コード番号は出典なしの選択（エンジンは 0 かどうかしか見ない。FL64.exe は 4 つの個別の値を見る） | 小（`WinVerifyTrust` 1 関数 + 行 1 + テスト。50〜80 行） | 偽りではない（現状の環境の忠実な帰結） |
| B. 実際に検証する | エンジン DLL（22,227,016 B）を仮想ボリュームに置き（`FLE_VFS_SEED` は 64 MiB まで）、Authenticode を実装: PE のハッシュ（チェックサムと証明書表を除く）、PKCS#7 の解析、RSA 署名の検証、証明書チェーン（DigiCert Trusted Root G4 を信頼アンカーとする = **環境の主張**）、時刻（仮想時計 2021-01-01 と証明書の有効期間）、`CryptQueryObject`／`CryptMsgGetParam`／`Cert*` の実在のモデル。埋め込み証明書の subject は `CN=Image Line NV`（読み） | 動く。ただし信頼アンカー・失効・時刻の扱いは PM の主張が要る | 大（C で 1,500〜3,000 行。DER／PKCS#7／RSA／SHA の実装または JS の WebCrypto への委譲という設計判断を含む） | 偽りではない（ただし「どのルートを信頼するか」は主張） |
| C. 嘘の成功 | `WinVerifyTrust` = 0 と crypt32 の偽の signer 名 | 動く（実験 A。実測）。**PM の J5 の方針に反するので差分に入れていない** | 小（実験で約 70 行 + 8 行） | **偽り** |
| D. 検証の層を移す | ホスト（ブラウザ側）が配信する DLL のバイト列を**固定したハッシュ**（`b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d`）で検証し、その結果として `WinVerifyTrust` が 0 を返す、と**文書化された主張**にする | 動く。偽りか否かは「ホストが保証する」という主張の受け入れ次第（エンジンの検査より強い検査をホストが行う） | 小〜中（ハッシュの検査はハーネスに数十行。crypt32 の signer 名は DLL の実際の証明書から取り出す必要があり、その部分は B の一部） | 主張（PM の決定が要る） |

**推奨**: まず **A を差分に入れる**（偽りではなく、ゲートの失敗を再現可能にする。`CreateFruityInstance` が戻る回帰テストになる）。その上で、ブラウザ化の目的（エンジンを実際に動かす）には **D**（検証の層をホストへ移し、固定ハッシュで保証。主張として明記）か **B** を PM が選ぶ。C は推奨しない。A・B・D のどれも、PM が「どの時点の何を信頼するか」を決めてからでないと実装しない。

**範囲外**: 署名検査の実装（B の全部）、ホスト exe の検査（`Flags` ビット 0）、`FLE_FRUITY_*` の拡張。

### 実験（別枠。差分に入れていない。再現用のパッチは `experiment-wvt.diff`、**実験**と明記）

- 木のコピー `src-exp`（`fruity3.diff` の木 + `runtime/fle_exp.c` + 応答表の 9 行 + `fle_imports.h` の宣言）、環境変数 `FLE_EXP_WVT=fail|ok` で切り替え。
- **B（fail）**: `WinVerifyTrust` が `0x800B0100`。`logs/expB/native.txt`: `fruity: rax=0xfffffffe instance=0`、`CreateFruityInstance RETURNED (eax=-2) after 35101 lifted entries`、終了コード 0。
- **A（ok）**: `WinVerifyTrust` = 0、`CryptQueryObject`（偽のハンドル 2 つ）、`CryptMsgGetParam`（0x40 バイトの 0）、`CertFindCertificateInStore`（偽のコンテキスト）、`CertGetNameStringW`（`Image Line NV`）、`CertFreeCertificateContext`、`CertCloseStore`、`CryptMsgClose`。次の停止（実測）: `fle: RaiseException: exception code 0xc06d007f ...`（35,597 入口。遅延ロードの補助が `GetProcAddress(kernel32, "GetUserPreferredUILanguages")` で 0 を得た = `ERROR_PROC_NOT_FOUND` の VcppException）。これに en-US を答えると（実験コード 25 行）、次は **`user32!GetActiveWindow`**（38,238 入口。呼び出し元 `f_86c890` ← `f_86a920` ← `f_818d30` ← `f_13755b0` ← `CreateFruityInstance`。Delphi の `TApplication` の初期化の領域 0x86xxxx。読み）。ここからウィンドウ・メッセージ・メッセージループの領域（判断が要る壁）に入る。

## Learn／一次資料の取得記録（2026-10-28。curl で取得、タグを除いたテキスト）

| ページ | URL | 取得した文言（原文の抜粋） | 使った箇所 |
|---|---|---|---|
| SHGetSpecialFolderLocation | https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shgetspecialfolderlocation | "[in] hwnd ... Reserved." "[out] ppidl ... A PIDL specifying the folder's location relative to the root of the namespace (the desktop). It is the responsibility of the calling application to free the returned IDList by using CoTaskMemFree." "If this function succeeds, it returns S_OK. Otherwise, it returns an HRESULT error code." (Last updated 2024-02-22) | 壁 2 |
| SHGetPathFromIDListW | https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shgetpathfromidlistw | "Converts an item identifier list to a file system path." "[out] pszPath ... This buffer must be at least MAX_PATH characters in size." "Returns TRUE if successful; otherwise, FALSE." "If the location specified by the pidl parameter is not part of the file system, this function will fail." (Last updated 2024-11-20) | 壁 2 |
| GetFileAttributesW | https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfileattributesw | "If the function fails, the return value is INVALID_FILE_ATTRIBUTES. To get extended error information, call GetLastError." 空の名前についての記述は**ない** | 壁 1 の選択肢 (b) を採らない理由 |
| CPINFO | https://learn.microsoft.com/en-us/windows/win32/api/winnls/ns-winnls-cpinfo | "MaxCharSize: ... The length can be 1 for a single-byte character set (SBCS), 2 for a double-byte character set (DBCS), or a value larger than 2 for other character set types." "DefaultChar ... The default is usually the "?" character for the code page." "The lead byte information ... is not even provided for many multi-byte encodings, for example, UTF-8 and GB18030." | 壁 3 |
| GetCPInfo | https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-getcpinfo | "Returns 1 if successful, or 0 otherwise ... ERROR_INVALID_PARAMETER. Any of the parameter values was invalid." (Last updated 2024-02-22) | 壁 3 |
| WinVerifyTrust | https://learn.microsoft.com/en-us/windows/win32/api/wintrust/nf-wintrust-winverifytrust | "If the trust provider verifies that the subject is trusted for the specified action, the return value is zero. No other value besides zero should be considered a successful return." "The return value can be a trust-provider-specific value ..., or it can be one of the following error codes: TRUST_E_SUBJECT_NOT_TRUSTED, TRUST_E_PROVIDER_UNKNOWN, TRUST_E_ACTION_UNKNOWN, TRUST_E_SUBJECT_FORM_UNKNOWN." | 壁 4 |
| NSIS `fileform.h`、`exehead/util.c`、`Source/build.cpp` | https://raw.githubusercontent.com/kichik/nsis/master/Source/exehead/fileform.h など（kichik/nsis の master。インストーラの版と同じとは限らない） | 文字列の符号（`NS_LANG_CODE` 1、`NS_SHELL_CODE` 2、`NS_VAR_CODE` 3、`NS_SKIP_CODE` 4、`CODE_SHORT`）、シェル定数の `0x80`（レジストリ経由の ProgramFilesDir）と `0x40`（32 ビットビュー）、組み込み変数 0〜31 | `nsis-registry.md` |

## 実装した場所

- 新規: `lift/harness/reg_seed.h`、`lift/harness/seeds/reg-seed-app.tsv`、`lift/harness/seeds/reg-seed-nsis.tsv`、`lift/harness/seeds/reg_seed_selftest.c`、`lift/crates/fe-lift/tests/wall_fr3.rs`、`tools/nsis_script_dump.py`。
- 変更: `lift/runtime/fle_ole.c`／`.h`（2 関数を追加）、`lift/runtime/fle_locale.c`（`GetCPInfo` の分岐）、`lift/crates/fe-lift/src/imports_table.rs`（末尾に 2 行）、`lift/harness/a4-multi.sh`（`REG_SEED`）、`lift/harness/two/main_two.c`（`#ifdef FLE_FRUITY` の中）、`lift/crates/fe-lift/tests/vcall_thunk_dll.rs`（期待値 390 → 387）。
- 既存の関数の本体で変えたもの（指示により理由を記す）: `fle_imp_GetCPInfo`（壁 3。CP_UTF8 が停止 → 答え。他の分岐は不変）。それ以外の既存の関数は変えていない。
- 既定の挙動: `FLE_REG_SEED`／`REG_SEED` を設定しない実行は従来と同じ（実測: 種なしで壁 1 が同じ入口数 4,693／157 で再現、DllMain の functions／entries が 5,306／1,007,942）。`FLE_REG_SEED` を含むコードは **`-DFLE_FRUITY` のビルドにしか入らない**（`main_two.c` を `-E` で展開した出力で検査）。`a4-multi.sh` は `FLE_CALL_FRUITY=1` なしの `REG_SEED` を拒否し（終了 2）、呼び出し側の `FLE_REG_SEED` は無視する。

## FRUITY 無効の既定ビルドの native の SHA-256（関数単位の理由）

基準 `9c324428…`（d7336ad 以降）→ `8855bff412d29f16ad8e4914d76727375bbfb8a31d6d2038a5e1ac235ea2590e`（HEAD f5e9552 + この差分、FRUITY 無効、種なし。a4 は不変）。理由（実測）:

1. **FLEngine の生成 C**（註釈あり、SHA-256 `e047beeb…` → `040bc98867fb473944964dd346a2fe896cabc48f53c67a8fd0bd879ad31b6a42`）: 関数 59,380 個のうち**本体が変わったのは 3 個**: `f_593350`、`f_593360`（どちらも `shell32!SHGetPathFromIDListW` の thunk）、`f_593370`（`shell32!SHGetSpecialFolderLocation` の thunk）。`fle_unsupported(...)` が `fle_imp_*()` の呼び出しに変わった。`unsupported_total` 390 → 387。5 モジュールの生成 C は、結合前の fe-lift の出力を新旧の fe-lift で比べると（実測）、**dsp・Blend2D・QFC・freetype は関数の本体が 1 つも変わらない**（8,479／2,067／6,723／1,653 関数。表の行と個数の定数だけが増える）、**ILTools は 3 関数**（`f_611c440`、`f_611c450`、`f_611c460`。いずれも `SHGetPathFromIDListW`（2 つ）と `SHGetSpecialFolderLocation` の thunk で、`fle_unsupported` から `fle_imp_*()` に変わった（実測）。12,931 関数中）。`common.c`（結合した応答表）は行が 2 つ増えたので変わる。
2. **runtime のオブジェクト**: 50 個（`fle_*.c` 全部 + `image`、`a4`、`main_two`（`-DFLE_FRUITY` なし）、`trace_native`）を 2 つの木で同じフラグでコンパイルして比べると、違うのは **`fle_ole.o`（2 関数の追加）と `fle_locale.o`（`GetCPInfo` の分岐）の 2 つ**だけ。`main_two.o`、`a4.o`、`image.o`、`trace.o` は同一。
3. `common.c`: 応答表と dispatch 表に 2 行ずつ、個数の定数が変わる。

## テスト

新規 `tests/wall_fr3.rs`（19 件）と C の自己検査 `reg_seed_selftest.c`（69 項目。`wall_fr3` の 1 件が実行する）:

- 生成器: 新しい 2 行が末尾の 2 行で擬似 VA が詰まっていること、FLEngine の 2 つの thunk が shell32 の下でだけ答えられること、`WinVerifyTrust`／`crypt32` に行が無いこと（停止のまま）。
- ランタイム（全 `fle_*.c` + 合成 PE の生成 C）: `GetProcAddress` で見つかる、APPDATA／FONTS の PIDL が S_OK・12 バイトのブロック・パスに変換・`CoTaskMemFree` で解放、最終エラーに触れない、擬似 VA 経由、PIDL 2 つ、他の csidl・非 NULL の hwnd・NULL の ppidl・解放済み／外来／サイズ違いの pidl・NULL の pidl／pszPath の名前付き停止（文言）、`GetCPInfo(65001)`＝`4, '?', 0…`、1252 と 437 は従来のまま、932／65000／54936／1250 は停止、定数の同期（`fle_env.c` と `fle_ole.c`）。
- ハーネス: 自己検査（種の形式 22 種の拒否と「何も書かない」、型、DWORD の範囲、大文字小文字、2 つの一覧、R[] の復元、最終エラー 0）、`a4-multi.sh` の拒否（`FLE_CALL_FRUITY` なし／0、相対パス、読めないファイル、空白）、呼び出し側の `FLE_REG_SEED` の無視、`-DFLE_FRUITY` なしの `main_two.c` に種付けが入らないこと、種付けが vfs 種付けの後・guest の前、2 つの種ファイルの自己申告（`# NOT FROM THE INSTALLER`／`# FROM THE INSTALLER`、`Shared data` は app だけ）。
- 実 DLL（`FLEngine_x64.dll` があるとき）: `Shared data` と鍵の UTF-16 文字列、`f_63ab90` の呼び出し順、`GetCPInfo` の thunk、`WinVerifyTrust` の `test eax,eax; sete al` と −2 の書き込み（`0x1375eeb`）。
- 既存のテストの更新は 1 つ: `vcall_thunk_dll.rs`（FLEngine の未対応 thunk 数 390 → 387。理由は本書の「関数単位の理由」）。

## 検証（実測）

- 全体テスト: `cargo test --workspace --no-fail-fast --offline` を 1 回（46 分 24 秒）で **116 test result・1,594 passed・0 failed**（`logs/fulltest-final/`）。最初の通しでは `vcall_thunk_dll` の期待値（390）が 1 件落ちた（本決定の応答が thunk を 3 つ減らしたため。期待値の更新漏れ）ので 387 に直し、最終の通しを取り直した。
- 変異検査（木のコピー上で 1 件ずつ変更 → テストが落ちる → 元に戻す）10 件、**10 件検出**（`logs/mutation-final.txt`）: M1 `SHGetPathFromIDListW` が解放済みの pidl を受け入れる、M2 APPDATA のパスを `Local` に、M3 `GetCPInfo(65001)` の MaxCharSize 4 → 3、M4 REG_DWORD の範囲検査を緩める、M5 新しい 2 行の順序入れ替え、M6 `a4-multi.sh` が FRUITY なしの `REG_SEED` を受け入れる、M7 `main_two.c` が全ビルドで種付けを呼ぶ、M8 種付けが R[] を戻さない、M9 `SHGetSpecialFolderLocation` が任意の csidl を APPDATA として答える、M10 `vcall_thunk_dll.rs` の期待値を戻す。
- 測定（`logs/fin1`、`logs/fin2` = 独立な 2 回の一から のビルド、TAG `t7eb3fac2ab02`）: 同一の native SHA-256、同一の停止（`WinVerifyTrust`、35,096／312）、終了コード 134。ビルド時間 gen 16〜17 s + build 約 60 s（約 1:19）、最大 RSS 約 2.7 GB。実行 0.8 秒、RSS 約 204 MB。
- 種の組み合わせ（`logs/fin-noseed`、`logs/fin-apponly`、`logs/fin1`）: 種なし = 壁 1（4,693／157）、app のみ = 35,093／313、nsis + app = 35,096／312。

## 出典なしの選択の一覧

| # | 場所 | 選択 | 区分 |
|---|---|---|---|
| S1 | `reg-seed-app.tsv` | `HKCU\Software\Image-Line\Shared\Paths` の `Shared data` = `C:\Users\user\Documents\Image-Line\`（REG_SZ） | 出典なしの選択（値）。形はエンジンの既定（読み）、`Documents` は Learn の KNOWNFOLDERID、`user` は決定 0107 |
| S2 | `reg-seed-nsis.tsv` | `$INSTDIR` = `C:\Program Files\Image-Line\FL Studio 21`（`FL64.exe`、`FLEngine_x64.dll`、`Install path` の値） | 選択（既存の決定 0018／0104／0110／0112 の PM の値。スクリプトの実機での既定は `C:\Program Files (x86)\Image-Line\FL Studio 2026` のはず。読み） |
| S3 | `GetCPInfo(65001)` | MaxCharSize 4 | 出典なしの選択（RFC 3629 に由来。Microsoft のページにない） |
| S4 | PIDL | SHITEMID の abID の内容（csidl + `PIDL`）、E_OUTOFMEMORY の番号（`0x8007000E`。到達しない） | 出典なしの選択 |
| S5 | 種付け | DllMain の前に書く（実機ではレジストリは最初から存在する、という読み）。副作用: DllMain の functions／entries が +14／+156（5,320／1,008,098）、鍵の書き込み時刻が仮想クロックの 1 刻み分進み、鍵ハンドルが後ろに詰まる | 設計 |
| S6 | 実験 B | `0x800B0100`（TRUST_E_NOSIGNATURE） | 実験の値。出典なし（Learn の WinVerifyTrust の例のコードに現れる値の一つ） |
| S7 | `REG_SEED` の型 | REG_SZ／REG_EXPAND_SZ／REG_DWORD／REG_BINARY だけ。REG_MULTI_SZ、REG_QWORD は拒否 | 設計 |

## 範囲外

署名検査の実装・偽装、ウィンドウ・メッセージループ・音声・スレッド・COM の大きなモデル、`GetFileAttributesW("")` の応答（案 (b)。エラー番号が仕様にない）、`CSIDL_PERSONAL` の `SHGetFolderPathW`（種があるので到達しない。種なしで壁 1 を通した場合の次の壁と読んでいた。未実装）、wasm（native のみ）、ホスト exe の検査。

## 撤回条件

- 実機 Windows で `HKCU\...\Shared\Paths\Shared data` の既定や形式が違うと分かったとき → S1 を書き換える（`reg-seed-app.tsv` だけ）。
- `ILSetup.dll` の `StoreILPath`／`MigrateSettings` が `Shared data` を書くと分かったとき → S1 は「出典なし」ではなく「プラグインの読み」になる（値の形が変わりうる）。
- `GetCPInfo(65001)` の `MaxCharSize` が実機で 4 でないと分かったとき → S3 を書き換える。
- PM が壁 4 の案を決めたとき → 壁 4 は本決定から外れる（別決定）。

## 解釈が分かれた点

1. 依頼は「`Shared data` などインストーラが書く値は NSIS から読む」。読んだ結果、`Shared data` はインストーラが書かない。種は 2 つのファイル（インストーラ由来／そうでない）に分け、混ぜなかった。
2. 壁 1 の解法を、`GetFileAttributesW("")` への応答ではなく登録の種付けにした（上の壁 1 の選択肢）。
3. 種付けを DllMain の前に書く（`vfs_seed` と同じ位置）か、DllMain の後（`CreateFruityInstance` の直前）にするか。前を選んだ（実機ではレジストリが先に存在する）。代償は S5。
4. `WinVerifyTrust` の実験 B を「差分に入れる／入れない」: 入れない（案 A の採否は PM。ただし推奨は A）。
5. `fle_env.c` と `fle_ole.c` のどちらに書くか（壁 2 の途中の誤り）: `fle_ole.c`。

## PM の判断が要る点

1. **壁 4（署名検査）**: 案 A〜D のどれか（推奨: A を入れた上で D か B）。信頼アンカー・時刻・失効の主張を含む。
2. `Shared data` の値（S1）と、`ILSetup.dll` が書くかの確認（逆アセンブル）を誰がするか。
3. 種付けを DllMain の前に置く選択（S5）の可否。
4. 壁 4 の先（実験 A）は、ウィンドウ・メッセージ・メッセージループ（Delphi の `TApplication.Run`）に入る。**ブロッキングのメッセージループをどう分割するか**（設計書 J7）。
5. インストーラの `$INSTDIR` の既定が `C:\Program Files (x86)\Image-Line\FL Studio 2026`（読み）である点と、リポジトリの仮想パス `C:\Program Files\Image-Line\FL Studio 21`（PM の値）の食い違いをどうするか。

## 限界

- 実機 Windows との照合は 0 件。wasm・ブラウザは未測定（native のみ）。
- NSIS の読みは、命令のオペランドの形から番号を決めた（番号のずれの原因は未確認）。スクリプトの条件分岐は追っていない。`ILSetup.dll` の中身は読んでいない。
- 実験 A の先は 1 歩だけ。実験の応答（en-US、偽の署名者名）は嘘または仮のもので、結果は「そこで止まる」ことだけを示す。
- 前回の測定の「壁 2 以降の予想」（`SHGetFolderPathW(CSIDL_PERSONAL)`）は、種を入れたので通らなかった（種なしで壁 1 を別の方法で通した場合の予想で、未実測のまま）。
- 静的に数えた壁は、名前付きで止まる応答の壁を数えない（前回の指摘）。今回の 3 つの壁も静的な数え方には現れなかった。
- 時間と RAM: 1 回の 6 モジュールのビルド（gen + build）が約 1:19、RSS 約 2.7 GB。実行は 0.8 秒、RSS 約 204 MB。
