# 0117: `WinVerifyTrust` の方針 D — 検証を実行前のオフライン検査に移し、その結果の主張の表から固定の応答を返す（検査した範囲と、していない範囲を明記）。その先の壁 5（`GetUserPreferredUILanguages`）を直し、壁 6 で停止: エンジンの CPU 検査が `CPUID` の応答（決定 0009）で落ちる（PM の判断が要る環境の主張）。すべて仮の決定

状態: **仮の決定（すべての応答と選択）**。依頼は PM（2026-10-29。決定 0117 の方針 D は PM の仮の決定。番号 0117 は PM の仮の割り当て）。測定・実装・テスト・文書は作業者（サブエージェント）が行った。**HEAD 159a42e の clean な `git archive` に差分 `fruity4.diff` を当てた木（作業は HEAD acf69cf で始め、途中で HEAD が 159a42e に進んだ。差は docs の 1 本と `lift/harness/web/eventloop/*` の追加だけで、差分と重ならない。測定の 2 回と生成 C の比較は 159a42e の追加分を足す前の木で行い、全体テストは足した後の木で、変異検査は足す前の木のコピーで行った。追加分はビルドに入らないので、native の SHA は同じと読む）で、コンパイル（警告・エラーなし）、全体テスト（118 test result・1,632 passed・0 failed）、生成 C の関数単位の比較、変異検査 19 件（19 件検出。変更なしの対照は通る）、6 モジュール構成の FRUITY 有効の測定（独立な 2 回のビルド、同一）と無効のビルド 1 回まで実施した。実機 Windows との照合は 0 件。wasm・ブラウザは未測定（native のみ）。**

**測定用の基底・種・註釈（dsp 0x2000000、Blend2D 0x4700000、QFC 0x1d00000、freetype 0x4b00000、ILTools 0x6000000、フォント 2 ファイル、レジストリの種）と、仮の応答を含む構成での値である。** 決定 0101 の表記では「A4 完走（仮の応答 N 件を含む）」の形の値で、達成の判定は PM が行う。

**肯定的に見せる点は少ない。** 署名の壁（`WinVerifyTrust`）は越えた。しかし `CreateFruityInstance` は戻らず、**38,241 関数入口目で `user32!GetActiveWindow` に止まる**（決定 0116 の停止点 35,096 入口目から 約 1.09 倍）。この停止は「ウィンドウの領域に入った」ことを**意味しない**（前の決定の読みは誤りだった）: `GetActiveWindow` は**エラーのメッセージボックスの所有者を探す呼び出し**で、そのメッセージは `This computer's CPU won't run FL Studio` である。エンジンが `CPUID` の応答から作る CPU 機能の語（FPU・CX8・CMOV など）が 0 のため、自分で CPU 検査に落ちている。この応答は決定 0009 の作業者の判断（`CPUID` の leaf 1 EDX = SSE と SSE2 の 2 ビットだけ）で、**環境の主張が分岐を決める壁**（停止条件 (b)）である。

区分の凡例: **実測**＝コマンドの出力を見た事実。**読み**＝逆アセンブルやコードを読んだ上での解釈。**Learn**＝Microsoft Learn のページに書かれていること（取得日 2026-10-29 JST。タグを除いたテキスト）。**一次(ABI)**＝x86-64 psABI のソース（Microsoft ではない）。**出典なしの選択**＝ページにも実測にも根拠が無く、作業者が選んだ値。**未確認**。

## 先に結論

| 項目 | 内容 |
|---|---|
| `CreateFruityInstance`（種あり・主張の表あり） | **戻らない**。最後の停止点: `fle: unsupported instruction at 0x424b60`（`user32.dll!GetActiveWindow` の thunk `f_424b60`。呼び出し元 `f_86c890` → `f_86a920` → `f_818d30`（メッセージボックスの補助）→ `f_13755b0`（CreateFruityInstance の事前検査。戻り先 0x1375694）→ `CreateFruityInstance`）。**38,241 関数入口目**、新規の関数 348（決定 0116 の停止点 35,096／312 から +3,145 入口／+36 関数）。`rax` なし、`instance` = 0 のまま、終了コード 134。独立な 2 回のビルドで同一（native `15ab0a8e27b24a7439398cb8ca28a1dba3ddd58bab08e60c2ab8d8ffe77d8845`） |
| 直した壁 | **2**（壁 4: `WinVerifyTrust` と crypt32 の 7 関数、壁 5: `GetUserPreferredUILanguages`）。壁 6（CPU 検査）は停止条件 (b) で止めた。壁の通し番号は決定 0116 の続き（壁 1〜3 はそちら） |
| `WinVerifyTrust` の応答 | **0 を返すのは、「ハーネスが登録した主張の表」にそのパスがあり、行が成立（Authenticode ダイジェストの一致と署名値の検証が、実ファイルでオフラインに確認済み）のときだけ**。それ以外（表が空、パスが無い、行が不成立、要求が主張より多い）は名前付き停止（偽の成功にしない）。意味は「このファイルについて、ダイジェストと署名値の一致をオフラインで確認済み。**証明書チェーン・失効・時刻は未検証**」であり、実機の Windows が同じ結果を返すことを意味しない（下「検査した／していないもの」） |
| Authenticode 検査の結果（`tools/authenticode_check.py`） | `FLEngine_x64.dll`（SHA-256 `b2c64da4…9496d`）: **(a) ダイジェスト ok、(b) 署名 ok**、署名者 CN `Image Line NV`、発行者 CN `DigiCert Trusted G4 Code Signing RSA4096 SHA384 2021 CA1`、有効期間 2026-07-28〜2027-10-21、タイムスタンプ（RFC 3161）あり（2026-09-29T14:31:53Z）。`FL64.exe`（インストーラから取り出した。SHA-256 `4c839f24…c65`）: **(a) ok、(b) ok**、同じ署名者・発行者・有効期間、タイムスタンプ 2026-09-29T14:31:48Z。独立な照合（OpenSSL の証明書表示、`cryptography` の RSA 検証、`pefile` の見出しオフセット）と一致 |
| **重要な発見（時刻）** | 署名者証明書の有効期間の始まり（**2026-07-28**）は、この環境の仮想時計（`FLE_VFS_CLOCK_START` = **2021-01-01**、決定 0018）より後。実機の Windows の時計がこの値なら、証明書は「まだ有効でない」と見えうる（タイムスタンプがあれば別の扱いになりうる。Learn は `WTD_LIFETIME_SIGNING_FLAG` が無ければ「タイムスタンプ付きの署名は永久に有効」と書く。この呼び出しは dwProvFlags = 0）。実機で何が返るかは**未確認**。この環境の「0」は実機の結果の再現ではない |
| 「ホスト exe の検査」 | エンジンは `Flags` のビット 0 が 0 のとき、`GetModuleFileNameW(NULL)` のパスにも `WinVerifyTrust` を呼ぶ。**この環境では `GetModuleFileNameW(NULL)` が DLL のパス（決定 0018: NULL も 0x400000 も `FLE_VIRTUAL_DLL_PATH`）を返す**ので、2 回目の検査も `FLEngine_x64.dll` のパスに対して行われる（gdb で 4 回の呼び出しを確認。`logs/fin1/gdb-wintrust-paths.txt`）。表に載せた `FL64.exe` の行は**使われない**（解釈が分かれた点 1） |
| 壁 6 の原因（実測 + 読み） | `f_639a20`（CPU 情報の単位。InitUnits の中で走る）が `CPUID` leaf 1 の EDX を読んで機能語（`record+0x12`）を作る: EDX bit 0 → 1、bit 4 → 2、bit 8 → 4、bit 11 → 8、**bit 15（CMOV）→ 0x10**、bit 24 → 0x40 ほか。`f_13755b0` は `test [record+0x12], 0x10` で 0x10 が無いと上のエラーを出す。決定 0009 の `fle_cpuid.c` は EDX = bit 25（SSE）と bit 26（SSE2）だけを答える。**x86-64 の基準 CPU は CMOV・CX8・FPU・FXSR・MMX を持つ**（一次(ABI)）ので、この応答は x86-64 の CPU としても整合しない |
| 実験（別枠。差分に入れない） | **E1: EDX に CMOV だけ足す → a4（FRUITY 無効）は不変**（942/942、895/895、functions 5,306、entries 1,007,942、`returned from entry`、終了 0）。FRUITY 有効では CPU 検査を通り、**38,166 入口目で `RaiseException 0xc06d007e`（遅延ロードの `rpcrt4.dll` の `LoadLibraryA` 失敗）**。**E4: psABI の基準（FPU・CX8・CMOV・MMX・FXSR）を足す → a4 の functions 5,308（+2）、entries 1,007,944（+2）**（他の行は不変）。E2: `rpcrt4.dll` をモジュールにして `UuidCreate` に固定の GUID（嘘）→ 次は `LoadIconW(0x400000, "MAINICON")`（38,876 入口）。E3: それに共有の IDI_APPLICATION を答える（嘘）→ `GetIconInfo` で止まる（38,907 入口）。詳細は「実験」 |
| 全体テスト | `cargo test --workspace --no-fail-fast --offline` を **1 回の通し**（45 分 55 秒、2026-10-29T23:08:07Z〜23:54:02Z、終了 0）で **118 test result・1,632 passed・0 failed・ignored 0**（基準 116・1,594 + 新規 38 件 = 1,632。`logs/fulltest/`）。この前にサーバー再起動で中断した通し（92 test result まで、失敗 0）は数えない |
| FRUITY 無効の既定ビルドの native | 基準 `8855bff412d29f16ad8e4914d76727375bbfb8a31d6d2038a5e1ac235ea2590e`（決定 0116 以降）→ **`ab6c6be63c255daf24e884efaed1ac8f3e200e098191985a8d162e5f5f38f187`**（変わった。理由は「関数単位の理由」）。a4 は変わらない（942/942、895/895、functions 5,306、entries 1,007,942、`returned from entry`、終了 0） |

## 背景

- 前段（決定 0116、`docs/investigations/create-fruity-instance-measurement.md`、`create-fruity-instance-harness.md`、`host-engine-boundary.md`）: 6 モジュール構成の DllMain／InitUnits 完走（942/942）の後に `CreateFruityInstance(0xC0, 0xEDE95A, 偽ホスト, &instance)` を呼ぶハーネス（`FLE_CALL_FRUITY=1`／`FRUITY=1`／`-DFLE_FRUITY`）は、3 つの壁を直したあと 35,096 関数入口目の `wintrust.dll!WinVerifyTrust`（`unsupported instruction at 0x4256d0`、呼び元 0x1375bce、戻り先 0x1375bd3）で止まった。決定 0116 は案 A（失敗を答える）、B（実際に検証する）、C（嘘の成功）、D（検証の層を移す）を挙げ、PM の判断を求めた。
- PM の仮の決定（0117、変えない）: **案 D「検証を実行前のオフライン検査に移し、その結果として固定の応答を返す。ただし何を検査し何を検査していないかを明記する」**。案 C（無条件の「検証成功」）は採らない。(1) オフライン検査ツール `tools/authenticode_check.py`（標準ライブラリだけ）、(2) runtime の `WinVerifyTrust` は主張の表にあって行が成立のときに限り 0（表は FRUITY ビルド以外では空）、(3) `WinVerifyTrust` の後の署名者名の比較に使う crypt32 には表の署名者 CN を返す、(4) 表記と撤回条件。
- 方針（PM、前の決定から継続）: ふり（fail-soft）で進めない。仕様から導ける応答だけを実装し、導けないものは名前付き停止。応答表は末尾追記のみ（決定 0015）。新しい `.c`／`.h` を runtime に作らない。既存の関数の本体・応答の値は変えない。
- 土台（実測。HEAD acf69cf の `git archive`）: 専用の `CARGO_TARGET_DIR`。FRUITY 無効の基準を再現した（TAG `t2331d00e8ebb`、a4 942/942、895/895、functions 5,306、entries 1,007,942、`returned from entry`、終了 0、native `8855bff4…2590e`）。FRUITY 有効（種 = nsis + app）で、決定 0116 の記録どおり `WinVerifyTrust` の `unsupported instruction at 0x4256d0`（**35,096 関数入口目**、新規の関数 312、native `d361c921bd9fbfd7811473a9fe997940c986222823cedd6187a70fe3a0390723`）で停止することを再現した。

## 検査した／していないもの（`tools/authenticode_check.py`）

読むだけ（入力を書き換えない。`--tsv`、`--report` で名指しした 2 つのファイルだけを書く）。標準ライブラリだけ（DER／BER の読み取り、PE の見出し、Authenticode ダイジェスト、RSA PKCS#1 v1.5 の検証を自前で実装）。

**検査したもの**

| 記号 | 内容 |
|---|---|
| (a) ダイジェスト | PE の Authenticode ダイジェスト（チェックサムの 4 バイト、証明書テーブルのディレクトリエントリ 8 バイト、証明書テーブル本体を除き、セクションは `PointerToRawData` の昇順、最後に残りのデータ）が、埋め込み PKCS#7 の `SpcIndirectDataContent` のダイジェストと一致するか。アルゴリズムは記載のもの（SHA-1／SHA-256／SHA-384／SHA-512）。証明書テーブルがファイルの末尾に無いときは「処理しない」（ng） |
| (b) 署名 | `SignerInfo` の `issuerAndSerialNumber` が指す署名者証明書が埋め込み証明書にあるか。署名属性の `contentType` が `SpcIndirectDataContent`、`messageDigest` が `SpcIndirectDataContent` の内容オクテット（外側の SEQUENCE のタグと長さを除く）のハッシュに等しいか。署名属性を `SET OF` に直した DER のハッシュに対する RSA PKCS#1 v1.5 の署名が、署名者証明書の公開鍵で検証できるか（`pow(sig, e, n)` と DigestInfo の厳密な比較。パディングは 00 01 FF…FF 00 で FF が 8 個以上）。署名アルゴリズムの OID がハッシュを名指すときは `digestAlgorithm` と一致するか |
| (c) 報告するだけ | 署名者証明書の subject／issuer／有効期間／serial、タイムスタンプ副署名の有無（`countersignature`／`rfc3161`、読めれば時刻） |

**検査していないもの（すべての出力に `unverified = chain,revocation,time` と書く）**: 証明書チェーンのルートまでの信頼（署名者証明書自身の署名も検証しない）、失効、時刻の信頼（有効期間は印字するだけで時計と比べない。タイムスタンプの存在は報告するが、タイムスタンプ自身の署名も時刻も検証しない）。テスト `the_limits_are_real…` が「署名者証明書の 1 バイトを変えても (a)(b) は ok のまま」を固定している。ネストした署名（`1.3.6.1.4.1.311.2.4.1`）は読むが、行は 1 つめの署名のものだけ（実ファイルには無い）。RSA 以外の署名は (b) = ng（未対応）。

**独立な照合（実測。`authenticode-report.md`）**: (1) `openssl pkcs7 -print_certs -text` が同じ subject（`C=BE, L=Gent, O=Image Line NV, CN=Image Line NV`）、issuer、有効期間、serial `0afc3aad2fa7d5b5e9c267ed4a67eb12` を出す。(2) `pefile` で見出しのオフセットを取り、`hashlib` で別に書いたマスクで Authenticode ダイジェストを計算 → 署名のダイジェスト `16a77cf9…ddd5d0` と一致。(3) `cryptography` の `PKCS1v15` による RSA 検証が署名を受理し、署名属性を 1 ビット変えたものを `InvalidSignature` で拒否（FLEngine と FL64.exe の両方）。(2)(3) は DER の切り出しにツールの読み取りを使った（検証そのものは独立）。

## 決定（仮）

1. **表**: `lift/harness/seeds/authenticode-claims.tsv`（ツールが書く。手で編集しない）。13 列: 前半 11 列は依頼の列（file、file の SHA-256、authenticode ダイジェスト `<alg>:<hex>`、a ok/ng、b ok/ng、署名者 CN、発行者 CN、notBefore、notAfter、timestamp、unverified）、**後ろ 2 列は追加**（`signer_issuer_der` = 署名者証明書の issuer Name の DER、`signer_serial` = serial の符号なし big-endian）。crypt32 の `CMSG_SIGNER_INFO` と `CERT_INFO` が持つ値を、**同じ証明書から読んだ実値**で返すため（創作しない）。行は a と b の両方が ok で、署名者 CN・issuer・serial があるときだけ「成立」。
2. **登録**: ハーネスのヘッダ `lift/harness/authenticode_claims.h`（新規。`reg_seed.h` の流儀。`-DFLE_FRUITY` のビルドにだけ入る）が、環境変数 `FLE_AUTH_CLAIMS`（`a4-multi.sh` では `AUTH_CLAIMS=<絶対パスの 1 ファイル>`。FRUITY 有効のときだけ受け付け、呼び出し側の `FLE_AUTH_CLAIMS` は無視）の表を読み、各行を**ゲストのパス**（`FLE_VIRTUAL_DLL_PATH` のディレクトリ + file 列）で runtime に登録する（`fle_trust_claim_add`）。未設定なら何もせず、表は空のまま（従来と同じ挙動）。表の `unverified` が `chain,revocation,time` でなければ拒否する。**エンジンの DLL の行は、ハーネス自身が読み込む DLL の SHA-256 と突き合わせる（違えば exit 2）**。FL64.exe の行は突き合わせるファイルを持たないので「attested ではない」と表示する。
3. **runtime**（`runtime/fle_vfs.c`／`fle_vfs.h`。新しい `.c`／`.h` は作らない。応答表の末尾に 9 行）: `wintrust.dll!WinVerifyTrust`（`WINTRUST_ACTION_GENERIC_VERIFY_V2`、`WINTRUST_DATA` 88 バイト、`WTD_CHOICE_FILE`、`WINTRUST_FILE_INFO` 32 バイトからパスを読む。VERIFY は 0 を返して状態ハンドル（領域 `0x7FFA90000000`）を `hWVTStateData` に入れ、CLOSE は生きた状態ハンドルを解放して 0。IGNORE は 0 だけ。失効確認の要求、方針コールバック、SIP クライアント、署名設定、`hFile`、既知の subject、カタログ、URL 参照、`dwProvFlags` が 0／0x10 以外、は**主張より多い要求**なので名前付き停止）。`crypt32.dll`: `CryptQueryObject`（FILE、`PKCS7_SIGNED_EMBED` のみ、BINARY のみ）、`CryptMsgGetParam`（`CMSG_SIGNER_INFO_PARAM` のみ。136 バイトの構造 + issuer + serial。`dwVersion` = 1、Issuer、SerialNumber だけ実値で、ほかは 0）、`CertFindCertificateInStore`（`CERT_FIND_SUBJECT_CERT` のみ。issuer のバイト列と serial の数値を比べ、無ければ NULL と `CRYPT_E_NOT_FOUND`）、`CertGetNameStringW`（`CERT_NAME_SIMPLE_DISPLAY_TYPE` のみ。表の署名者 CN）、`CertFreeCertificateContext`、`CertCloseStore`（フラグ 0 のみ）、`CryptMsgClose`。**証明書コンテキスト（`CERT_CONTEXT`）の `pbCertEncoded`・`cbCertEncoded`・`pCertInfo` は 0**（表に証明書本体が無い）。そのほかの crypt32／wintrust の関数は答えない。
4. **`kernel32!GetUserPreferredUILanguages`**（`runtime/fle_uipref.c`。壁 5）: ユーザーの UI 言語 = 環境の 1 言語（en-US、決定 0021／0022 と同じ `fle_emulated_locale`）。名前（既定）または `MUI_LANGUAGE_ID`（`0409`）、リストは 2 つの NUL で終わる。バッファの規則は `GetThreadPreferredUILanguages` と同じ。
5. **壁 6 は直さない**（停止条件 (b)）。`fle_cpuid.c`（決定 0009）は変えない。
6. **表記**: 本決定は「仮の決定」。「`WinVerifyTrust` が 0 を返す」は『このファイルについて、ダイジェストと署名値の一致をオフラインで確認済み。証明書チェーン・失効・時刻は未検証』という意味であり、実機の Windows が同じ結果を返すことを意味しない。

## 壁の表

各壁の記録は `wall-log.md`。凡例の「種類」は 未応答 import／名前付き停止／lift の穴／環境の主張／メッセージループ／その他。

| # | 停止メッセージ | 原因（区分） | 種類 | 修正（場所） | 測定（呼び出しの入口数／新規の関数） |
|---|---|---|---|---|---|
| 4 | `fle: unsupported instruction at 0x4256d0`（`wintrust.dll!WinVerifyTrust`） | エンジンが自分の DLL の Authenticode 署名を検査（`CreateFruityInstance` の 0x1375bce）。0 でなければ −2 で戻る。0 なら `CryptQueryObject` → `CryptMsgGetParam`（2 回）→ `CertFindCertificateInStore` → `CertGetNameStringW`（2 回）→ `CertFreeCertificateContext` → `CertCloseStore`／`CryptMsgClose` で `Image Line NV` と比較し、さらに `Flags` のビット 0 が 0 のときホスト exe にも同じ（読み + 実測）。**停止条件 (a) に近いが、方針 D で扱う** | 未応答 import（PM の判断を含む主張の表で答える） | 上の「決定」の 1〜3（`fle_vfs.c`、`fle_vfs.h`、`imports_table.rs` の 8 行、ハーネス、ツール） | 35,096／312 → **35,600／330**（次の停止） |
| 5 | `fle: RaiseException: exception code 0xc06d007f, flags 0, 1 argument(s) raised; no structured exception handling is modelled, stopped here` | 遅延ロードの補助が `GetProcAddress(kernel32, "GetUserPreferredUILanguages")` で 0 を得て `ERROR_PROC_NOT_FOUND` の VcppException を上げる。呼び元は Delphi の補助 `f_9df190`（`MUI_LANGUAGE_NAME`、まず NULL バッファで長さ、次に取得、先頭の名前を使う）（読み + 実測） | 未応答 import（遅延ロード） | `fle_imp_GetUserPreferredUILanguages`（`fle_uipref.c`）、応答表の 9 行目 | 35,600／330 → **38,241／348** |
| 6 | `fle: unsupported instruction at 0x424b60`（`user32.dll!GetActiveWindow`） | **エンジンの CPU 検査の失敗**: `f_13755b0` が `record+0x12` の bit 0x10 を見て、無いので `This computer's CPU won't run %s`（`FL Studio`）を `f_818d30` で表示しようとし、補助が `GetActiveWindow` を呼ぶ。その先は `MessageBoxW`（モーダル）。機能語は `f_639a20` が `CPUID` から作る（実測 + 読み） | **環境の主張**（停止条件 (b)）。`GetActiveWindow` 自体は症状 | （なし。下「止まった壁」） | 38,241 で停止 |

壁の種類の件数（この作業の壁 4〜6）: 未応答 import 2（壁 4、5）、環境の主張 1（壁 6）、名前付き停止 0、lift の穴 0、メッセージループ 0、その他 0。決定 0116 の壁 1〜3（環境の主張 1、未応答 import 1、仕様に書かれていない値 1）と合わせると、壁 6 つ中 未応答 import 3、環境の主張 2、仕様に書かれていない値 1。壁 4 のうち crypt32 の 7 関数は**停止して見つけたのではなく**、`CreateFruityInstance` の逆アセンブルから先に読んで実装した（読んだ流れは測定で確かめた: 2 回の検査で各 1 組の呼び出し列が走り、署名者名の比較を通る。`logs/wall1/`）。

## 止まった壁（壁 6: CPU 検査）の選択肢と推奨

**何が起きているか（実測 + 読み）**: 停止の見かけは `user32!GetActiveWindow` だが、原因は CPU 検査である。

- 呼び出しの連鎖（shadow call stack、実測）: `GetActiveWindow` ← `f_86c890`（オーナーウィンドウを探す。`[rbx+0x1c0]` のフックが空なら `GetActiveWindow`、それも 0 なら別の窓）← `f_86a920` ← `f_818d30`（`ShowMessage` 相当。引数は文字列とフラグ 0x10 = `MB_ICONERROR`）← `f_13755b0`（`CreateFruityInstance` が最初に呼ぶ事前検査。戻り先 0x1375694）。停止時の引数はメッセージ `This computer's CPU won't run FL Studio`、題 `Error`、フラグ 0x10。
- `f_13755b0` の検査（読み、バイト列はテストで固定）: ① ホストの版 `[host+8] == 1`（偽ホストは 1）。② **`mov rax,[0x15e4c78]`（ポインタの枠 → 0x15f32e0）; `mov eax,[rax+0x12]; test eax,0x10; jne ok`** — 通らなければ `This computer's CPU won't run %s`。③ ほかの同一プロセスの検査（`[0x15e0fa8] > 0` で `Another instance of %s is open`）。
- 機能語の作り方（読み）: `f_639a20` が `f_639900`（`CPUID` を実行して EAX、EBX、ECX、EDX を `[rdi]` に書く）を leaf 1 で呼び、EDX を `[rbp+0x34]`、ECX を `[rbp+0x30]` に得て、`record+0x12` に `or` する: **EDX bit 0（FPU）→ 1、bit 4（TSC）→ 2、bit 8（CX8）→ 4、bit 11（SEP）→ 8（ファミリ・モデルで落とすことがある）、bit 15（CMOV）→ 0x10、bit 24（FXSR）→ 0x40**（さらに `0x6399d0` が真なら 0x20000）ほか。この単位は InitUnits の中で走る（gdb: `f_63a5e0` → `f_63a330` → `f_639a20`）ので、DllMain の後に `CPUID` の答えを変えても、すでに作られた機能語は変わらない。
- この環境の答え（決定 0009、`fle_cpuid.c`）: leaf 1 EDX = **bit 25（SSE）と bit 26（SSE2）だけ**、ECX = 0、ベンダ文字列なし、EAX = EBX = 0。したがって機能語は 0、`test eax,0x10` が落ちる。x86-64 の基準（一次(ABI): 「Any program can expect that an x86-64 processor implements the baseline features: CMOV, CX8, FPU, FXSR, MMX, OSFXSR, SCE, SSE, SSE2」。`gitlab.com/x86-psABIs/x86-64-ABI` の `low-level-sys-info.tex`、取得 2026-10-29）と食い違う: SSE2 を持つ CPU は CMOV などを持つ。

**選択肢**

| 案 | 内容 | 結果（実測／読み） | 作業量 | 注意 |
|---|---|---|---|---|
| A1. CMOV だけ足す | `FLE_CPUID_LEAF1_EDX` に bit 15 | **実験 E1: a4（FRUITY 無効）は不変**（942/942、895/895、functions 5,306、entries 1,007,942、終了 0）。FRUITY 有効では CPU 検査を通り、次は `RaiseException 0xc06d007e`（`rpcrt4.dll`、38,166 入口）。**`cpuid.rs` など `fle_cpuid.c` の値を固定する既存のテストが落ちる**（期待値の更新）。全モジュールの `CPUID` の答えが変わる（dsp_ippv2 の CPU 振り分けを含む） | 小（1 行 + 既存テストの期待値 + 決定 0009 の更新） | 「CMOV だけある CPU」は存在しない。エンジンの検査だけを通す最小の主張。**PM の主張** |
| A2. x86-64 の基準を足す（推奨） | bit 0（FPU）、8（CX8）、15（CMOV）、23（MMX）、24（FXSR）を足す。TSC（bit 4）は基準に無いので足さない | **実験 E4: a4（FRUITY 無効）の functions 5,308（+2）、entries 1,007,944（+2）**。ほかの行（942/942、895/895、`returned from entry`、終了 0）は不変。つまり**基準の数字（5,306／1,007,942）が変わる**ので、PM が新しい基準を承認する必要がある。+2 の原因は未特定（FXSR の枝の `0x6399d0` などと読むが未確認） | 小（1 行 + 期待値） | 出典は psABI の表（Microsoft ではない）。TSC・SEP などの扱いは別の判断 |
| A3. ハーネスだけが答えを変える | FRUITY ビルドのハーネスが InitUnits の前に `fle_cpuid` の上書きを入れる（環境変数）。既定のビルドは不変 | 未実測。既定のビルドの native と a4 は不変にできる。ただし「FRUITY のときだけ CPU が違う」ことになり、DllMain の実行（機能語を使う分岐）が既定と変わる | 中（ハーネス + runtime の上書きの口。新しい `.c` は作れないので `fle_cpuid.c` に口を足す） | 環境の主張を二重にする |
| B. CPU 検査を飛ばす | `f_13755b0` の検査をフックで飛ばす、`MessageBoxW` に OK を答える | 検討しない（エンジンの失敗を隠す。偽の成功の一種） | — | 推奨しない |
| C. `GetActiveWindow`／`MessageBoxW` を実装する | ウィンドウ・メッセージボックスのモデル | **無意味**: エラーのメッセージボックスを出して終わる（モーダル。メッセージループで待つ）。CPU 検査が通らない限りこの先に進まない | 大 | 推奨しない |

**推奨**: A2（x86-64 の基準）。理由: エンジンの検査と整合し、実在する CPU の主張として自然。ただし**基準の数字が +2 動く**ので、PM が (1) CPU の主張（A1／A2／別の機種像）、(2) 基準の更新（5,306→5,308、1,007,942→1,007,944）、(3) 決定 0009 の更新とその撤回条件を決める。A2 を入れた**あと**の壁は、実験 E1（CMOV のみ）で見えている範囲では、`rpcrt4.dll` の遅延ロード（モジュール追加 + `UuidCreate`＝乱数の主張）、`LoadIconW` の module のアイコン（`MAINICON`。エンジンの `.rsrc` の `RT_GROUP_ICON` にある。アイコン資源のモデル）、`GetIconInfo`（アイコンのモデル）で、**ウィンドウ・描画の領域に入る**（停止条件 (d) に近づく）。

## 実験（別枠。差分に入れない。再現用のパッチは `experiment-cpu.diff`）

木のコピー `src-exp`（`fruity4.diff` の木 + 下の変更）と `src-exp4`、専用の `target-exp`。native のみ。

| 実験 | 変更 | 結果（実測） |
|---|---|---|
| E1 | `FLE_CPUID_LEAF1_EDX` に bit 15（CMOV） | FRUITY 無効: a4 不変（`logs/expE1off/`）。FRUITY 有効: 38,166 入口、新規の関数 350、`RaiseException 0xc06d007e`（`logs/expE1/`）。呼び出しは `f_11b2450`（`GetTempPathW`、`GetCurrentProcessId` の後）が `LoadLibraryA("rpcrt4.dll")`（遅延ロード `UuidCreate`）で 0 を得る |
| E2 | E1 + モジュール表に `rpcrt4.dll` を追加（N = 36）+ `UuidCreate` に**固定の GUID**（嘘） | 38,876 入口、新規の関数 372、`fle: LoadIconW: hInstance is not NULL: icon resources of a module are not implemented`（`LoadIconW(0x400000, L"MAINICON")`、呼び元は `CreateFruityInstance` 自身の 0x13764eb）（`logs/expE2/`）。`MAINICON` はエンジンの `RT_GROUP_ICON` にある（pefile で確認） |
| E3 | E2 + `LoadIconW(0x400000, …)` に共有の IDI_APPLICATION のハンドルを答える（嘘） | 38,907 入口、新規の関数 373、`GetIconInfo: hIcon is not an icon … made by CreateIconIndirect`（`logs/expE3/`）。嘘の副作用で、実装しても同じ壁かは分からない |
| E4 | `FLE_CPUID_LEAF1_EDX` に bit 0、8、15、23、24 | FRUITY 無効: a4 の functions 5,308、entries 1,007,944（他は不変）（`logs/expE4off/`） |

E2、E3 の応答は**嘘**で、結果は「そこで止まる」ことだけを示す。

## 実装した場所

- 新規: `tools/authenticode_check.py`、`lift/harness/authenticode_claims.h`、`lift/harness/seeds/authenticode-claims.tsv`、`lift/harness/seeds/authenticode_claims_selftest.c`、`lift/crates/fe-lift/tests/authenticode_check.rs`、`lift/crates/fe-lift/tests/wall_fr4.rs`、`lift/crates/fe-lift/tests/auth117/mk_signed_pe.py`、`lift/crates/fe-lift/tests/auth117/mutate_real.py`。
- 変更: `lift/runtime/fle_vfs.c`／`.h`（WinVerifyTrust と crypt32 の 7 関数、主張の表）、`lift/runtime/fle_uipref.c`／`.h`（`GetUserPreferredUILanguages`）、`lift/crates/fe-lift/src/imports_table.rs`（応答表の末尾に 9 行）、`lift/harness/two/main_two.c`（`#ifdef FLE_FRUITY` の中の 1 呼び出しと 1 include）、`lift/harness/a4-multi.sh`（`AUTH_CLAIMS`）、既存のテストの更新 3 件（下）。
- 既存の関数の本体で変えたもの（指示により理由を記す）: **なし**。既存の応答の値も変えていない。
- 既存のテストの更新（期待値の更新漏れ。理由を記す）: `vcall_thunk_dll.rs`（FLEngine の未対応 thunk 数 387 → 379。`WinVerifyTrust` と crypt32 の 7 関数の thunk 8 個が答えられるようになった）、`wall_fr3.rs` の `wintrust_is_still_not_answered`（決定 0116 の「WinVerifyTrust に行が無い」を固定していた。`the_other_wintrust_and_crypt32_functions_are_still_not_answered` に置き換え、隣の関数が答えられないままであることを固定）と `the_two_shell32_rows_are_the_last_two_rows…`（「最後の 2 行」を固定していた。`…follow_the_0112_rows…` に変え、0112 の行の直後であることを固定）。
- 既定の挙動: `FLE_AUTH_CLAIMS`／`AUTH_CLAIMS` を設定しない実行は従来と同じ（表は空で、`WinVerifyTrust` は名前付きで止まる。実測: テスト `an_empty_table_stops_every_call_by_name…`）。`FLE_AUTH_CLAIMS` を含むコードは `-DFLE_FRUITY` のビルドにしか入らない（`main_two.c` を `-E` で展開した出力で検査）。`a4-multi.sh` は `FLE_CALL_FRUITY=1` なしの `AUTH_CLAIMS` を拒否し（終了 2）、呼び出し側の `FLE_AUTH_CLAIMS` は無視する。

## FRUITY 無効の既定ビルドの native の SHA-256（関数単位の理由）

基準 `8855bff412d29f16ad8e4914d76727375bbfb8a31d6d2038a5e1ac235ea2590e`（HEAD acf69cf 以降、FRUITY 無効、種なし）→ **`ab6c6be63c255daf24e884efaed1ac8f3e200e098191985a8d162e5f5f38f187`**（HEAD + この差分、FRUITY 無効）。a4 の数字は不変。理由（実測）:

1. **FLEngine の生成 C**（註釈あり、SHA-256 `040bc98867fb473944964dd346a2fe896cabc48f53c67a8fd0bd879ad31b6a42` → `660b4335ae6d023e3982e6597f968defd00a60b4b9e580a33b26a6b52461d484`）: 関数 59,380 個のうち**本体が変わったのは 8 個**: `f_4256d0`（`wintrust!WinVerifyTrust` の thunk）、`f_1375030`（CryptQueryObject）、`f_1375040`（CryptMsgGetParam）、`f_1375050`（CertCloseStore）、`f_1375060`（CryptMsgClose）、`f_1375070`（CertFindCertificateInStore）、`f_1375080`（CertGetNameStringW）、`f_1375090`（CertFreeCertificateContext）。`fle_unsupported(...)` が `fle_imp_*()` の呼び出しに変わった。`unsupported_total` 387 → 379。追加した thunk だけ（`logs/gencmp/function-compare.txt`）。
2. **5 モジュールの生成 C**（結合前の fe-lift の出力を新旧の fe-lift で比べる。実測）: **dsp（8,479 関数）・Blend2D（2,067）・QFC（6,723）・freetype（1,653）・ILTools（12,931）は、関数の本体が 1 つも変わらない**。変わるのは表の行と個数の定数だけ。
3. **runtime のオブジェクト**: 50 個（`fle_*.c` 全部 + `image`、`a4`、`main_two`（`-DFLE_FRUITY` なし）、`trace_native`）を 2 つの木で同じフラグでコンパイルして比べると、違うのは **`fle_vfs.o`（WinVerifyTrust と crypt32 の追加）と `fle_uipref.o`（`GetUserPreferredUILanguages` の追加）の 2 つ**だけ。`main_two.o`、`a4.o`、`image.o`、`trace.o` は同一（`logs/gencmp/runtime-objects.txt`）。
4. `common.c`: 応答表と dispatch 表に 9 行、個数の定数が変わる。応答表の行数 `fle_resp_count`: 436 → 445（結合した表では FLEngine の 436 + 追加の 9）。

## Learn／一次資料の取得記録（2026-10-29 JST。取得は `urllib` で、タグを除いたテキスト。原文は `learn/` に保存）

| ページ | URL | 取得した文言（原文の抜粋） | 使った箇所 |
|---|---|---|---|
| WinVerifyTrust | https://learn.microsoft.com/en-us/windows/win32/api/wintrust/nf-wintrust-winverifytrust | "If the trust provider verifies that the subject is trusted for the specified action, the return value is zero. No other value besides zero should be considered a successful return." "[in] hwnd ... Zero: The trust provider can use the interactive desktop to display its user interface." "pgActionID ... WINTRUST_ACTION_GENERIC_VERIFY_V2: Verify a file or object using the Authenticode policy provider." "The return value is a LONG, not an HRESULT" | 壁 4 |
| WINTRUST_DATA | https://learn.microsoft.com/en-us/windows/win32/api/wintrust/ns-wintrust-wintrust_data（最終更新 2023-02-14） | "WTD_STATEACTION_VERIFY: ... The hWVTStateData member will receive a handle to the state data. This handle must be freed by specifying the WTD_STATEACTION_CLOSE action in a subsequent call." "WTD_STATEACTION_CLOSE: Free the hWVTStateData member previously allocated with the WTD_STATEACTION_VERIFY action." "fdwRevocationChecks ... WTD_REVOKE_NONE 0 / WTD_REVOKE_WHOLECHAIN 1" "WTD_LIFETIME_SIGNING_FLAG 2048 (0x800): If this flag is not set, all time stamped signatures are considered valid forever. Setting this flag limits the valid lifetime of the signature to the lifetime of the signing certificate." "dwUIChoice ... WTD_UI_ALL 1, NONE 2, NOBAD 3, NOGOOD 4" "pwszURLReference: Reserved for future use. Set to NULL." | 壁 4（構造、状態、フラグ） |
| WINTRUST_FILE_INFO | https://learn.microsoft.com/en-us/windows/win32/api/wintrust/ns-wintrust-wintrust_file_info（最終更新 2024-02-22） | "pcwszFilePath: Full path and file name of the file to be verified. This parameter cannot be NULL." "hFile: Optional ... pgKnownSubject: Optional" | 壁 4 |
| CryptQueryObject | https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/nf-wincrypt-cryptqueryobject | "CERT_QUERY_OBJECT_FILE: The object is stored in a file." "This parameter is a pointer to a null-terminated Unicode string that contains the path and name of the file to query." "dwFlags: This parameter is reserved for future use and must be set to zero." "phCertStore ... When you have finished using the handle, free it by passing the handle to the CertCloseStore function." "phMsg ... CryptMsgClose function." 数値（`CERT_QUERY_CONTENT_PKCS7_SIGNED_EMBED` 10、`CERT_QUERY_FORMAT_BINARY` 1 など）はページに**ない**: ローカルの mingw-w64 の `wincrypt.h`（Microsoft のものではない）から | 壁 4 |
| CryptMsgGetParam | https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/nf-wincrypt-cryptmsggetparam | "CMSG_SIGNER_INFO_PARAM: pvData data type: pointer to a BYTE array to receive a CMSG_SIGNER_INFO structure. Returns information on a message signer. This includes the issuer and serial number of the signer's certificate ..." "pvData: ... This parameter can be NULL to set the size of this information for memory allocation purposes." "ERROR_MORE_DATA The specified buffer is not large enough to hold the returned data." | 壁 4 |
| CMSG_SIGNER_INFO | https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/ns-wincrypt-cmsg_signer_info（最終更新 2024-02-22） | メンバの並び（dwVersion、Issuer、SerialNumber、HashAlgorithm、HashEncryptionAlgorithm、EncryptedHash、AuthAttrs、UnauthAttrs）。バージョンの値は `CMSG_SIGNER_INFO_V1` = 1（mingw-w64 の `wincrypt.h`） | 壁 4 |
| CERT_INFO | https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/ns-wincrypt-cert_info（最終更新 2021-04-02） | "SerialNumber: ... The least significant byte is the zero byte of the pbData member of SerialNumber ... The most significant byte is the last byte of pbData. Leading 0x00 or 0xFF bytes are removed." | 壁 4（serial はリトルエンディアン） |
| CERT_CONTEXT | https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/ns-wincrypt-cert_context（最終更新 2021-04-02） | dwCertEncodingType、pbCertEncoded、cbCertEncoded、pCertInfo、hCertStore | 壁 4 |
| CertFindCertificateInStore | https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/nf-wincrypt-certfindcertificateinstore | "CERT_FIND_SUBJECT_CERT: Data type of pvFindPara: CERT_INFO structure. Searches for a certificate with both an issuer and a serial number that match the issuer and serial number in the CERT_INFO structure." "If ... a certificate ... is not found, the return value is NULL." "CRYPT_E_NOT_FOUND No certificate was found matching the search criteria." "A non-NULL CERT_CONTEXT ... must be freed by CertFreeCertificateContext" | 壁 4 |
| CertGetNameStringW | https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/nf-wincrypt-certgetnamestringw（最終更新 2023-02-09） | "CERT_NAME_SIMPLE_DISPLAY_TYPE 4: Iterates through the following list of name attributes and uses the Subject Name or the Subject Alternative Name extension for the first occurrence of: szOID_COMMON_NAME, szOID_ORGANIZATIONAL_UNIT_NAME, szOID_ORGANIZATION_NAME, or szOID_RSA_emailAddr." "Returns the number of characters converted, including the terminating zero character. If pszNameString is NULL or cchNameString is zero, returns the required size of the destination string (including the terminating NULL character)." | 壁 4 |
| CertFreeCertificateContext／CertCloseStore／CryptMsgClose | .../nf-wincrypt-certfreecertificatecontext（2024-02-22）、.../certclosestore（2021-10-13）、.../cryptmsgclose（2024-02-22） | "The function always returns nonzero." "dwFlags: Typically, this parameter uses the default value zero. The default is to close the store with memory remaining allocated for contexts that have not been freed." "If the function succeeds, the function returns nonzero (TRUE)." | 壁 4 |
| GetUserPreferredUILanguages | https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-getuserpreferreduilanguages（最終更新 2021-10-13） | "The flags are mutually exclusive, and the default is MUI_LANGUAGE_NAME." "... an ordered, null-delimited display language list ... This list ends with two null characters." "if this parameter is set to NULL and pcchLanguagesBuffer is set to 0, the function retrieves the required size of the language buffer in pcchLanguagesBuffer. The required size includes the two null characters." "ERROR_INSUFFICIENT_BUFFER. A supplied buffer size was not large enough, or it was incorrectly set to NULL." "en-US will be returned as "0409"" | 壁 5 |
| x86-64 psABI | https://gitlab.com/x86-psABIs/x86-64-ABI/-/raw/master/x86-64-ABI/low-level-sys-info.tex | "Any program can expect that an x86-64 processor implements the baseline features mentioned in table ... Most feature names correspond to CPUID bits" の表: CMOV、CX8、FPU、FXSR、MMX、OSFXSR、SCE、SSE、SSE2 | 壁 6 |

Learn が**書いていない**こと（出典なしの選択の根拠）: `WinVerifyTrust` の CLOSE の戻り値、CLOSE 後の `hWVTStateData` の値、`CryptMsgGetParam` が `CMSG_SIGNER_INFO` のどのメンバを埋めるか（実機は全部）、`CertGetNameStringW` のバッファが短いときの結果、`CertFindCertificateInStore` の `CRYPT_E_NOT_FOUND` の数値（`winerror.h` の 0x80092004）。

## 出典なしの選択の一覧

| # | 場所 | 選択 | 区分 |
|---|---|---|---|
| S1 | `WinVerifyTrust` の CLOSE | 戻り値 0。`hWVTStateData` は呼び出し側の値のまま（実機がゼロにするかは未確認） | 出典なしの選択 |
| S2 | 状態ハンドル／ストア／メッセージのハンドル | 領域 `0x7FFA90000000` + 4 × n の**トークン**（再利用しない）。決定 0024 が空けた 0x7FFA30000000／40 は使わない | 設計 |
| S3 | `CryptQueryObject` の出力値 | encoding 0x10001、content type 10、format 1（`wincrypt.h` の値。Learn にない）。ストアとメッセージのハンドルは別のトークン | 出典: ローカルの mingw-w64 `wincrypt.h`（Microsoft ではない）。未確認 |
| S4 | `CMSG_SIGNER_INFO` | 136 バイト + issuer（8 バイト境界に切り上げ）+ serial（同）。`dwVersion` = 1、Issuer、SerialNumber 以外は **0**（実機は属性やハッシュ値を持つ）。エンジンは先頭 0x28 バイトのうち +8..+0x28 しか読まない（読み） | 出典なしの選択（限界） |
| S5 | `CERT_CONTEXT` | 40 バイトのホストメモリ（確保して解放しない）。`dwCertEncodingType` と `hCertStore` だけ実値。`pbCertEncoded`、`cbCertEncoded`、`pCertInfo` は 0 | 出典なしの選択（限界） |
| S6 | 署名者名 | 表の署名者 CN（`CERT_NAME_SIMPLE_DISPLAY_TYPE` の最初の CN）。CN が無い行は成立しない | Learn の「最初の CN／OU／O／E」を CN のみで近似 |
| S7 | `GetUserPreferredUILanguages` | リストは 1 言語（`fle_emulated_locale` = en-US）。ユーザーの言語リストを 1 件にするのは環境の選択 | 出典なしの選択（値は決定 0021 の en-US） |
| S8 | 表のゲストのパス | `FLE_VIRTUAL_DLL_PATH` のディレクトリ（PM の値、決定 0018）+ file 列。FL64.exe は `C:\Program Files\Image-Line\FL Studio 21\FL64.exe`（インストーラ由来の `reg-seed-nsis.tsv` と同じ） | PM の値 + 設計 |
| S9 | 状態ハンドルの上限 | 生きた状態とすべての crypt32 オブジェクトの合計 64（トークンは再利用しないので総発行数）。超えたら停止 | 設計 |

## テスト

新規 `tests/authenticode_check.rs`（14 件。ツール）、`tests/wall_fr4.rs`（24 件。runtime、ハーネス、生成器、実 DLL）、C の自己検査 `harness/seeds/authenticode_claims_selftest.c`（44 項目。`wall_fr4` の 1 件が実行する）、補助 `tests/auth117/mk_signed_pe.py`（独立な署名ファイルの作成）、`mutate_real.py`:

- ツール（合成した署名ファイル。作成側は DER・Authenticode ハッシュ・RSA 署名を独立に実装）: 正しいファイルは (a)(b) とも ok でダイジェストは第三の方法（`sha256sum` の 3 区間）と一致、SHA-1 + 副署名、コードの 1 バイト改ざんで (a) だけ ng、署名値の 1 バイト改ざんで (b) だけ ng、`messageDigest`／`contentType`／署名者の不在／署名アルゴリズムの不一致でそれぞれ (b) ng と理由、署名なしは行が不成立で理由つき、PE でないファイル、コマンドラインの誤り（終了 2）、入力を書き換えず名指しのファイルだけを書く、列と見出し。
- ツール（実 DLL）: `FLEngine_x64.dll` の値（ダイジェスト、署名者、発行者、有効期間、serial）が独立な照合と一致、1 バイト改ざんで (a) または (b) ng、署名者証明書／タイムスタンプの 1 バイトを変えても ok のまま（限界の固定）、リポジトリの表が現在のツールの出力と一致（FL64.exe は環境変数 `FL64_EXE` があるとき全体が一致）、`FL64.exe` の ok／ok。
- runtime（全 `fle_*.c` + 合成 PE の生成 C）: 空の表で全呼び出しが名前付き停止、成立した行で 0 と状態の発行／解放、IGNORE、2 つの状態は別のハンドル、二重の CLOSE・外来の状態・状態付きの VERIFY の停止、パスの大文字小文字、表にないパスの停止（拡張子違い・相対・二重の区切り）、不成立の行の停止と理由、要求の異常 19 通り（GUID、cbStruct、コールバック、UI 選択、失効、カタログ、URL、`dwProvFlags`、FILE_INFO の各メンバ、署名設定、状態アクション 3／4）、crypt32 の一連の流れ（出力値、サイズ照会、小さいバッファの 234、書き込みと残りの 0、serial のリトルエンディアン、`CERT_INFO` へのコピー、見つからない issuer／serial と `CRYPT_E_NOT_FOUND`、余分な先頭 0 の serial、名前の長さ 14 と書き込み、解放とクローズ、ハンドルの生存数）、crypt32 の引数の異常（オブジェクト種別、フラグ、ハンドルの種類違い、二重の解放、`dwParamType`、`dwIndex`、検索の種類、名前の型）、主張の表の拒否規則、`GetUserPreferredUILanguages`（サイズ 7／6、内容、小さい／NULL のバッファの 122、`GetProcAddress`、疑似 VA、フラグの停止）。
- 生成器: 新しい 9 行が末尾の 9 行で、順序どおり、擬似 VA が詰まっていること、wintrust と crypt32 の thunk が自分の DLL の下でだけ答えられること。
- ハーネス: 自己検査（refusals 22 種で「何も登録しない」、受理、ゲストのパスの作り方、attested／NOT attested の表示、runtime が実際に 0 を返す、同じパスの再登録の拒否、成立しない行）、`a4-multi.sh` の拒否（FRUITY なし／0、相対パス、読めないファイル、空白、`:`）、呼び出し側の `FLE_AUTH_CLAIMS` の無視、`-DFLE_FRUITY` なしの `main_two.c` に登録が入らないこと、登録が reg seed の後・fruity precheck の前、runtime に新しい `.c`／`.h` がないこと、表を埋めるのが `authenticode_claims.h` だけ。
- 実 DLL（`FLEngine_x64.dll` があるとき）: 8 個の thunk が wintrust.dll／crypt32.dll の指定の関数（IAT を辿って確認）、呼び出しの引数のバイト列、`Image Line NV` の文字列、`GetUserPreferredUILanguages` の名前と `MUI_LANGUAGE_NAME`、**壁 6 のバイト列**（`test eax,0x10`、メッセージ、`GetActiveWindow` の thunk、`test rax,0x8000` → `or …,0x10`）、決定 0009 の `CPUID` が SSE|SSE2 だけで CMOV が無いこと（壁 6 の原因の固定。PM が CPU の主張を決めたら、決定とともに更新する）。
- 既存のテストの更新は 3 件（上「実装した場所」）。

## 検証（実測）

- **全体テスト**: 上のとおり 1 回の通しで 118 test result・1,632 passed・0 failed（`logs/fulltest/cargo-test.txt`、警告は既存のテスト `sse_shuf`、`monitor` のもので、新規・更新のテストからは 0）。木は HEAD 159a42e の archive + 差分。**他の `out-*` の入力の確認**: ワークツリーの `lift/out-*` のミラー 7,820 リンクは壊れたものが 0、リンク先 7,787 ファイル（`/dev` 配下 33 を除く）の更新時刻とサイズは通しの前後で同一（実リポジトリ側の `out-*` に書き込んでいない）。テストが固定パスで読む入力（`out-ext/dsp_ippv2_x64.dll`、`out-blend2d/Blend2D.dll`、`out-qfc/quickfontcache_x64.dll`、`out-seed58/s58/dll/Blend2D.4700000.dll`、`out-jtb2d90/three/dll/dsp_ippv2_x64.2000000.dll`、`out-wallQ/freetype/freetype_x64.4b00000.dll`、`out-ctype/tables/in-units.txt`、`out-modules/seeds-896.txt`）は全部存在する。失敗が 0 なので、別作業の削除（生成 C／H、cargo target、.o、ELF）に由来する失敗はなかった。実 `lift/` に存在しない `out-*` 名（`out-wallb`、`out-hook`、`out-call-iat` など 19 個）は、テストが `out-X/test` を作る出力先で、テスト自身が作る。
- **変異検査**（木のコピー `mut/` で 1 件ずつ変更 → 該当のテストだけを実行 → 落ちる → 元に戻す。`logs/mutation-run.txt`、`tools/mutate117.py`）19 件、**19 件検出**、対照（変更なし）は通る: M1 不成立の行に答える、M2 パスの比較が大文字小文字を区別、M3 表が空でなければ任意のパスを受理、M4 `CertGetNameStringW` の名前が違う、M5 serial をリトルエンディアンにしない、M6 CLOSE が状態を解放しない、M7 失効確認の要求に答える、M8 アクション GUID を見ない、M9 ツールがダイジェストの比較を飛ばす、M10 ツールが署名値を検証しない、M11 ツールがチェックサムを含めてハッシュ、M12 ハーネスが `unverified` の不足を受理、M13 ハーネスが DLL の SHA-256 を突き合わせない、M14 `MUI_LANGUAGE_ID` が主言語だけ、M15 応答表の 2 行の順序入れ替え、M16 `a4-multi.sh` が FRUITY なしの `AUTH_CLAIMS` を受理、M17 `main_two.c` が全ビルドで登録、M18 FLEngine の thunk 数の期待値を戻す、M19 `CMSG_SIGNER_INFO` の切り上げを 4 バイトに。
- **測定**（`logs/`。6 モジュール構成。ビルド gen 約 15 s + build 約 60 s、合計 1:01〜1:25、最大 RSS 約 2.7 GB。実行 0.06 秒の呼び出し + 約 1 秒のハーネス）:

| 構成 | native SHA-256 | 結果 |
|---|---|---|
| FRUITY 無効・差分なし（`base`、TAG `t2331d00e8ebb`） | `8855bff4…2590e` | a4 942/942、895/895、functions 5,306、entries 1,007,942、`returned from entry`、終了 0 |
| FRUITY 有効・種のみ・表なし（`wall0`） | `d361c921…0723` | `WinVerifyTrust` の `unsupported instruction at 0x4256d0`、35,096／312、134（決定 0116 と同一） |
| FRUITY 有効・表あり、壁 4 まで（`wall1`） | `23f9f66d…6570` | `RaiseException 0xc06d007f`、35,600／330 |
| FRUITY 有効・表あり・最終（`fin1`、`fin2`） | `15ab0a8e27b24a7439398cb8ca28a1dba3ddd58bab08e60c2ab8d8ffe77d8845` | `unsupported instruction at 0x424b60`（`GetActiveWindow`）、**38,241／348**、134。`fin2` は別の OUT・別の cargo target で一から（fe-lift の SHA-256 も同一）。`native.txt` の `fruity:` 部分（入口数、レジスタ、呼び出しの記録）が同一、生成 C 6 つの SHA-256 が同一 |
| FRUITY 無効・差分あり（`fin-off`、TAG `t2331d00e8ebb`） | `ab6c6be63c255daf24e884efaed1ac8f3e200e098191985a8d162e5f5f38f187` | a4 942/942、895/895、functions 5,306、entries 1,007,942、`returned from entry`、終了 0（`base` と同一の数字） |

- **生成 C の比較**（`logs/gencmp/`、`tools/genall.sh`、`tools/cmpmod.py`）: FLEngine の旧生成 C の SHA-256 は `040bc988…`（測定の `gen.txt` と一致）、5 モジュールも一致（再現）。新旧の関数単位の比較は「FRUITY 無効の既定ビルドの native の SHA-256」の節のとおり。runtime のオブジェクト 50 個のうち違うのは 2 個（`logs/gencmp/runtime-objects.txt`）。
- **独立な 2 回の FRUITY 有効のビルド**: 上の表の `fin1` と `fin2`。
- **gdb**（`logs/fin1/gdb-wintrust-paths.txt`）: 最終構成の native で `WinVerifyTrust` の 4 回の呼び出し（VERIFY・CLOSE・VERIFY・CLOSE）のパスは、すべて `C:\Program Files\Image-Line\FL Studio 21\FLEngine_x64.dll`。
- **`patch -p1 --dry-run`**: HEAD 159a42e の clean な archive で終了 0、全ファイルが当たる（`logs/patch-dryrun-159a42e.txt`）。当てた木の差分対象ファイルはワークツリーの `src/` と `cmp` で一致。

## 解釈が分かれた点

1. **ホスト exe の検査**: 依頼は「エンジンが同じ WinVerifyTrust で検査するホスト exe（FL64.exe）」と「実際にエンジンが WinVerifyTrust に渡すパスを gdb／トレースで特定して表のキーにする」。測定すると 2 回目の検査（`Flags` のビット 0 が 0 のとき）のパスは `GetModuleFileNameW(NULL)` の応答で、**この環境ではそれが DLL のパス**（決定 0018: NULL も 0x400000 も DLL のパス）。表のキーは実測どおり DLL のパスだけが使われ、FL64.exe の行は登録されるが使われない。`GetModuleFileNameW(NULL)` を FL64.exe にするのは既存の応答の値の変更（DllMain の `ParamStr(0)` などに及ぶ）なので**しなかった**。PM の判断（点 3）。
2. **表の追加の 2 列**（issuer の DER と serial）: 依頼の列に足した。値は同じ証明書から読んだ実値。足さなければ `CMSG_SIGNER_INFO` の Issuer／SerialNumber に創作の値を入れることになる。
3. **ダイジェストの形式**: `<alg>:<hex>`（例 `sha256:16a7…`）。依頼は「authenticode ダイジェスト」とだけ。
4. **主張より多い要求の扱い**: 失効確認や `dwProvFlags` の追加は、表が持たない検査の要求なので停止にした（0 を返すと「検査した」ことになる）。Learn にそれを禁じる文言はなく、作業者の安全側の選択。
5. **不成立の行も登録する**: 「表にあって不成立」の停止の理由を出せるため。依頼は「表に無い、または不成立なら名前付き停止」で、どちらも停止。メッセージだけが違う。
6. **DLL の SHA-256 の突き合わせ**: 依頼にない。表が別のファイルのものなら登録を拒否する（exit 2）ようにした。FL64.exe は突き合わせられない。
7. **壁 5 を直す方法**: 遅延ロードの `GetProcAddress` に行を足した（`GetUserPreferredUILanguages` の本体を実装）。決定 0099（shcore）や 0112（`LocaleNameToLCID`）と同じ形。
8. **壁 6 で止める判断**: 依頼の停止条件 (b)。`GetActiveWindow` を答えると次は `MessageBoxW`（モーダル）になり、メッセージを出して終わるだけで、CPU の主張が通らない限り意味がない。実験 E1〜E4 は「PM が決めるときの材料」で、差分に入れない。

## PM の判断が要る点

1. **CPU の主張**（壁 6）: 案 A1（CMOV だけ）、A2（x86-64 の基準。推奨）、A3（FRUITY のときだけ）、別の機種像。A2 は基準の数字を +2 動かす（functions 5,308、entries 1,007,944）。決定 0009 の更新と、`cpuid.rs` など固定しているテストの期待値の更新が伴う。
2. **仮想時計と署名者証明書の有効期間**: 時計 2021-01-01 は証明書の notBefore（2026-07-28）より前。この環境で「`WinVerifyTrust` = 0」は実機の結果の再現ではない。仮想時計を変えるか（既存の応答の値の変更）、これを限界として受け入れるか。
3. **`GetModuleFileNameW(NULL)` の値**: DLL のパス（現状）か FL64.exe か。後者なら、ホスト exe の検査は FL64.exe の行で行われ、表の 2 行がどちらも使われる。
4. **FL64.exe の取り扱い**: インストーラから取り出した 597,064 バイトの `FL64.exe`（`FL64 (scaled).exe` は別ファイルで SHA-256 が違う、署名者・有効期間は同じ）。どちらをホスト exe とするか。表は `FL64.exe` だけ。
5. **A2 を入れたあとの壁**（実験 E1 + E2 + E3 の範囲）: `rpcrt4.dll`（モジュールの追加 + `UuidCreate` の乱数＝環境の主張）、`LoadIconW(MAINICON)`、`GetIconInfo`（アイコン資源のモデル）、その先はウィンドウ・描画・メッセージループの領域。どこまでを範囲とするか。
6. 決定 0116 の PM の判断点のうち、`Shared data` の値（S1）、`ILSetup.dll` の確認、種付けの位置、`$INSTDIR` の食い違いは**この作業では扱わなかった**（未変更）。

## 撤回条件

- 実機の Windows、または PM の追加の主張で、署名者・発行者・有効期間・タイムスタンプの扱いが違うと分かったとき → 表の再生成（ツールの再実行）。
- **証明書チェーンの検証を実装する決定**（信頼アンカー・失効・時刻の扱いを決める）→ 本決定の「検証していない」は外れ、表の `unverified` と runtime の応答の意味を見直す。
- **エンジンの検査対象が変わったとき**（DLL のバージョンが変わる、`CreateFruityInstance` が別のファイルを検査する、署名者名が変わる）→ 表を作り直す（DLL の SHA-256 が合わなければハーネスが拒否する）。
- ハーネスが DLL を仮想ボリュームに置く決定（`FLE_VFS_SEED` の 64 MiB の枠を広げるなど）→ 案 B（実際の検証）を検討するときの前提が変わる。
- PM が `GetModuleFileNameW(NULL)` を FL64.exe にしたとき → 解釈が分かれた点 1 が解消し、FL64.exe の行が使われる（コードの変更は不要、テストの期待値のみ）。
- 壁 6: PM が CPU の主張を決めたとき → テスト `the_emulated_cpu_claims_sse_and_sse2_only…` と本決定の壁 6 の節を更新する。

## 限界

- 実機 Windows との照合は 0 件。wasm・ブラウザは未測定（native のみ）。
- 署名の検証は (a)(b) のみ。チェーン・失効・時刻は未検証。表の「0」は『ダイジェストと署名値の一致をオフラインで確認』であり、実機の `WinVerifyTrust` の結果ではない。
- `CMSG_SIGNER_INFO`、`CERT_CONTEXT` は一部のメンバだけ実値（S4、S5）。エンジンが読む範囲（読み）は満たすが、他の呼び出し元は想定しない。
- FL64.exe の行は、ハーネスに実体がないため DLL のように突き合わせられない。インストーラからの取り出し（`tools/extract_nsis2.py`、読み取り専用）の SHA-256 `4c839f24…` が行の根拠。
- 壁 6 の先は、嘘の応答を含む実験（E1〜E4）で 3 歩見ただけ。E1 の `rpcrt4.dll` の失敗がどの `CreateFruityInstance` の処理によるかは、`GetTempPathW` と `GetCurrentProcessId` の後という以上は読んでいない。
- `GetActiveWindow` が止まる理由（CPU 検査）は、機能語の作り方の読み（EDX のビットの対応）と、E1 で CMOV を足すと先へ進む実測の両方に基づく。FPU・TSC・CX8・SEP・FXSR のビットが後の処理にどう効くかは未追跡（E4 で a4 の functions が +2 になることは実測、原因は未特定）。
- 時間と RAM: 1 回の 6 モジュールのビルド（gen + build）が約 1:05〜1:25、最大 RSS 約 2.7 GB。実行は 0.06 秒の呼び出し + 約 1 秒のハーネス。全体テストは約 46 分（45 分 55 秒）。
