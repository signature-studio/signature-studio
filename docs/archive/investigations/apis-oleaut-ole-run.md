# oleaut32／ole32 の応答（W4）の実行記録

日付: 2026-10-20。決定は `docs/decisions/0037-apis-oleaut-ole.md`（出典の URL は同文書）。出力はすべて `lift/out-oleaut/`（`tmp/` は TMPDIR）。対象は `docs/investigations/unsupported-inventory-pdata.md` §6 W4。

「実測」はツールの出力、「推測」は未検証の解釈。

## 1. 実装したもの

- 新規 runtime: `lift/runtime/fle_com.h`、`fle_ptrmap.h`、`fle_oleaut.h`、`fle_ole.h`（ヘッダを先）、`fle_oleaut.c`、`fle_ole.c`（`.c` を後）。`lift/out-oleaut/work/` で作って単体テストし、`runtime/` には 1 回で置いた（`cmp` で同一を確認）。
- 表への追記: `lift/crates/fe-lift/src/imports_table.rs` の TABLE に **28 行**（oleaut32 18、ole32 10）。直前に読み直し、`SetProcessAffinityMask` の行の後ろ、`];` の前にだけ追記した。`lib.rs` は変更していない。
- テスト: `lift/crates/fe-lift/tests/oleaut_ole.rs`（新規、**40 件**）。

### 1.1 実装した API（28）

IAT に実在するものを正とした（`out-inv/jmp_thunks.tsv`）。

- oleaut32（18）: SysAllocStringLen、SysFreeString、SysReAllocStringLen、VariantInit、VariantClear、VariantCopy、VariantCopyInd、VariantChangeType、SafeArrayCreate、SafeArrayGetLBound、SafeArrayGetUBound、SafeArrayAccessData、SafeArrayUnaccessData、SafeArrayGetElement、SafeArrayPutElement、SafeArrayPtrOfIndex、SafeArrayGetElemsize、GetErrorInfo。
- ole32（10）: CoInitialize、CoUninitialize、OleInitialize、OleUninitialize、CoTaskMemAlloc、CoTaskMemFree、StringFromCLSID、ProgIDFromCLSID、CLSIDFromString、IsEqualGUID。

依頼の一覧と IAT の実在の差: **差は無かった**。依頼の 28 API（oleaut32 18、ole32 10）はすべて IAT に実在し、「SafeArray* 9」も IAT の実在 9 個と一致した。IAT に無い API（SafeArrayDestroy、SafeArrayCopy など）は依頼の一覧にも無く、関数として出していない。

### 1.2 abort にした入力の種類

「名前付き abort」= `fle_stop`（stderr に `fle: <API>: <理由>`、SIGABRT）。

呼び返し（ゲスト関数の呼び出し）が要る入力:

1. VT_UNKNOWN／VT_DISPATCH で**ポインタが非 NULL**: VariantClear、VariantCopy、VariantCopyInd。
2. VT_UNKNOWN／VT_DISPATCH の配列の**非 NULL 要素**: SafeArrayPutElement、SafeArrayGetElement、配列の解放と複製。
3. VT_RECORD（IRecordInfo）: VARIANT と SafeArrayCreate。

ページが決めない入力（呼び返しではない）:

- VT_I8／VT_UI8（VARIANT、SafeArray）、VT_VARIANT（BYREF なし）、VT_BYREF | VT_EMPTY／VT_NULL、VT_BYREF でポインタが NULL。
- VariantChangeType: wFlags が 0 以外、型が違う変換、配列、BYREF や配列への変換。
- SafeArrayCreate: cDims が 1 以外、cElements が 0、表に無い要素型。
- 生きていない BSTR／配列／CoTaskMem のポインタ（二重解放、ゲストが作った描画子、他のアロケータの値、BSTR でない文字列）、dest = src、NULL が結果を持たない引数（GetElemsize の psa、GUID 関数の引数など）、SysReAllocStringLen の pbstr が NULL、psz が古い BSTR の内側で終端を越えて読む場合。
- CoInitialize／OleInitialize の pvReserved が非 NULL、対のない CoUninitialize／OleUninitialize、OleInitialize の分を CoUninitialize で外す呼び出し。
- CLSIDFromString の ProgID 文字列、ProgIDFromCLSID でゲストがキーを作っていた場合、GetErrorInfo の dwReserved が非 0。

### 1.3 未実装として残した API とその理由

IAT に実在し、W4 の依頼の範囲外のもの（20 thunk、この表に無い）:

- oleaut32: **GetActiveObject**（実行中オブジェクト表。COM の実体、W11）。
- ole32 の **CoCreateInstance／CoGetClassObject**（クラス表と COM サーバ）、**CoGetInterfaceAndReleaseStream／CoMarshalInterThreadInterfaceInStream**（ストリームとアパートメント間のマーシャル）、**CreateDataAdviseHolder／CreateILockBytesOnHGlobal／CreateStreamOnHGlobal／StgCreateDocfileOnILockBytes**（COM オブジェクトを返す）、**DoDragDrop／RegisterDragDrop／RevokeDragDrop／OleSetClipboard／OleGetClipboard／OleFlushClipboard／OleIsCurrentClipboard／OleDraw／OleSetMenuDescriptor／IsAccelerator**（ウィンドウとクリップボードの実体、W10）、**ReleaseStgMedium**（STGMEDIUM の解放で IUnknown::Release を呼びうる）。
- 遅延 import の **PropVariantClear**（`jmp rax` 末尾。W1 が前提）。

## 2. 単体テスト（実測）

`cargo test --offline -p fe-lift --test oleaut_ole`（`test-unit3.txt`）: **40 件 passed、0 failed**。

内訳（テスト関数名の数。`oleaut_ole.rs` の `#[test]` を数えた）: 表と宣言 1、実 DLL の handled 1（all-pdata の seed を使う。DLL か seed が無ければスキップ）、BSTR 5、VARIANT 10（Init 1、Clear 3、Copy 2、CopyInd 2、ChangeType 2）、SafeArray 8、GetErrorInfo 2、COM 初期化 3、CoTaskMem 2、GUID 文字列 7（StringFromCLSID、CLSIDFromString、ProgID の abort、IsEqualGUID、NULL 引数の abort のほか）、ProgIDFromCLSID 2、ソース確認 1。合計 40。

- 期待値はページから書き、HRESULT は公式の 16 進リテラルをテスト側に別に持つ（`fle_com.h` から読み戻さない）。
- メモリ内容を確認するもの: BSTR の接頭辞・終端・埋め込み NUL・コピーが別物、SafeArray の `cDims`／`cbElements`／`cLocks`／`pvData`／境界、要素の配置、VARIANT の vt と値、GUID の 16 バイト。
- 二重解放と不正入力の abort は `assert_aborts`（SIGABRT、`fle:` のメッセージ、abort 後に出力が無いこと）で、テスト内のケースを数えて **約 69 ケース**確認した（正規表現による数え上げで、厳密な数ではない）。
- 作成中に見つかったテスト側の誤り 3 件（ホストのリテラルを `static const` にして書き込んだ、BSTR の接頭辞を数えず内側のポインタを 2 バイトずらした、所有する文字列の解放を数え忘れた）は、実装ではなくテストの期待値を直した。実装の挙動を合わせるために変えたものは無い。
- 追加の確認: ASan／UBSan／LeakSanitizer（gcc）で BSTR、配列、CoTaskMem、GUID の一連の呼び出しを実行し、エラーとリークは 0（`out-oleaut/san/smoke.c`。clang の ASan ランタイムは環境に無い）。

## 3. ワークスペース全体のテスト（実測）

`cargo test --offline --workspace --no-fail-fast`（`test-ws.txt`、`test-ws.exit` は EXIT=101）: **62 バイナリ、580 件 passed、2 件 failed、0 ignored**（`test result` 行の集計）。

失敗 2 件は私の変更ではない:

- `null0.rs::undefined_cases_stop_the_run`: `Text file busy`（os error 26）。並行して他の担当が同じ `out-null0/test` の実行ファイルを再ビルドしていた競合。単独で再実行（`test-null0-rerun.txt`）して **6 件 passed**（推測を含む: 競合と読んだ。再実行で通ったことは実測）。
- `x87ctl.rs::x87_arithmetic_stays_unsupported`: `lift_body(&[0xd9, 0x07])`（`fld dword [rdi]`）が `is_err()` を期待しているが、同時に作業中の x87 担当が `src/x87.rs`（未追跡）を足し、`lib.rs` に `x87::lift` の分岐を入れたため成功するようになった。**x87 担当の変更とそのテストの食い違い**で、W4 とは無関係（`x87ctl.rs` は私が触っていない）。

## 4. all-pdata での生成とビルド（実測、混在状態）

**他の担当（x87、sysinfo、indirect-jmp）が未コミットで作業ツリーに混在している**。比較は同じ作業ツリーで、TABLE の W4 の 28 行の有無だけを変えた 2 回の生成（`base-table.sha` の `imports_table.rs` 先頭 `c89be6e39aab` が TABLE 追記前、`after-table.sha` の `ae1aa9146cc1` が追記後）。他担当の未コミット分（`lib.rs`、`x87.rs`、`jumptab.rs`、`tailjmp.rs` ほか）は両方に同じく入っている。seed は `out-pdata/seeds-pdata.txt`（57,816 行）、ツールは `out-pdata/tool` の複製（`out-oleaut/tool`）。

| 項目 | W4 追記前（`gen-base.txt`） | W4 追記後（`gen-after.txt`） | 差 |
|---|---:|---:|---:|
| 関数 | 58,695 | 58,695 | 0 |
| `bad_nodes` | 0 | 0 | 0 |
| **`unsupported_total`** | **698** | **667** | **−31** |
| `Jmp: indirect through IAT slot (not lifted)`（IAT 経由 jmp 未解決） | **588** | **557** | **−31** |
| `Jmp: indirect target unresolved` | 1 | 1 | 0 |
| 未対応のニーモニック（種類） | 19 | 19 | 0 |
| `c_bytes` | 217,968,523 | 217,974,978 | +6,455 |

- 差の 31 は、TABLE に追記した 28 API が当たる thunk の数と**一致**（`out-inv/jmp_thunks.tsv` で oleaut32 の対象 19 thunk と ole32 の対象 12 thunk、計 31。IsEqualGUID は 3 thunk、SysFreeString は 2 thunk）。閉包の oleaut32 の thunk は 20、ole32 は 31 で、このうち対象外の 20 thunk（§1.3）が残る。
- IAT 経由 jmp 未解決の残り 557 は、`out-inv/jmp_thunks.tsv` から TABLE を引き直した数（557）と一致。
- 他の担当（x87、sysinfo、indirect-jmp）の分が両方に入っていることが、`unsupported_total` が `unsupported-inventory-pdata.md` §0 の 2,991（生成日は古い）と大きく違う理由（x87 の `F*` が lift されて 2,193 行が消えている。**推測**: `gen-base.txt` の一覧に x87 の `F*` が 2 種類（F2xm1、Fsincos）しか残らないことからの読み）。この表の差は W4 だけの効果。
- 生成: 2.75 秒、最大 RSS 320 MB（`gen-after.err`）。
- ビルド（`build-parts.sh` の複製、16 分割、`JOBS=8`）: 生成 C のコンパイルとすべての runtime（W4 の `fle_oleaut.c`、`fle_ole.c` を含む。`-Wall -Wextra`、警告なし）は通った。**リンクは x87 の runtime が `runtime/` に無いため失敗**（`fle_x87_*` の未解決 2,182 件のみ。W4 に関する未解決は 0）。`fle_x87.c` は x87 担当が `out-x87/work/` で作成中。**リンクだけ通すため、`out-oleaut/x87stub/fle_x87_stub.c`（最初の呼び出しで abort するリンク専用のスタブ、`runtime/` には置いていない）を使って手でリンク**した。このスタブは納品物ではない。

## 5. 実行（実測）

コマンド: `./bin/flengine-after-pdata ../../binaries/win64/FLEngine_x64.dll`（`out-oleaut/` で。W4 追記後）と、同じ手順の W4 追記前（`bin/flengine-base-pdata`）。

| 項目 | W4 追記前 | W4 追記後 |
|---|---|---|
| 停止 | `fle: VerifyVersionInfoW: dwOSVersionInfoSize is not sizeof(OSVERSIONINFOEXW) ...` | 同じ |
| 関数実行数（distinct） | 147 | 147 |
| 関数入口（calls） | 736 | 736 |
| 呼び出し深さ | 7 | 7 |

- **W4 の追加で停止点は動かなかった**。停止はどちらも `VerifyVersionInfoW`（`fle_osver.c`、決定 0012）の `dwOSVersionInfoSize` で、W4 の API には届いていない。スタックは `f_423cc0 ← f_454030 ← f_454090 ← f_4120c0 ← f_412160 ← f_41d8f0 ← エントリ`。
- したがって **W4 の 28 API は all-pdata の実行ではまだ 1 回も呼ばれていない**（`SysFreeString` などは停止点より先にある）。実行の上での検証は無く、確認は単体テストとコード読解のみ。
- 参考: `out-pdata/bin/flengine-after-pdata-b`（pdata-seeds-run.md §5.3）は `RaiseException(0xc06d007f)` で止まっていた。今回の作業ツリーでは別の担当の変更（GetNativeSystemInfo の delay-load、VerifyVersionInfoW への到達など）で停止点が進んでいる（**推測**: 停止点の文字列の違いからの読み）。

## 6. 触ったファイル

新規（リポジトリ内）:

- `lift/runtime/fle_com.h`、`lift/runtime/fle_ptrmap.h`、`lift/runtime/fle_oleaut.h`、`lift/runtime/fle_ole.h`、`lift/runtime/fle_oleaut.c`、`lift/runtime/fle_ole.c`
- `lift/crates/fe-lift/tests/oleaut_ole.rs`
- `docs/decisions/0037-apis-oleaut-ole.md`、`docs/investigations/apis-oleaut-ole-run.md`

変更（追記のみ）:

- `lift/crates/fe-lift/src/imports_table.rs`（TABLE に 28 行）

`lift/out-oleaut/`（すべて新規）: `work/`（ヘッダと `.c` の作業コピー）、`learn/`（取得した Learn の HTML）、`tool/`、`Cargo.toml`、`target-tool/`、`target-test/`、`out/`（`base-pdata.c`、`after-pdata.c` と分割、各 217 MB と分割）、`parts/`、`bin/`、`harness/`、`x87stub/`、`san/`、`test/`、`tmp/`、`chk/`、各ログ（`gen-*.txt`、`build-*.txt`、`run-*.txt`、`test-*.txt`、`test-ws.txt`、`test-ws.exit`）、複製（`build-parts.sh`、`split_c.py`、`trace.c`）。

## 7. 制約の遵守と残置物

- `rm`、`>` による既存ファイルの上書き、git 操作、`/tmp`、コマンド文字列中の sleep／timeout は使っていない。TMPDIR は `lift/out-oleaut/tmp`。権限拒否はなかった。外部通信は Microsoft Learn の取得と検索のみ。
- 既存ファイルの変更は `imports_table.rs`（追記のみ）の 1 つだけ。runtime の既存ファイル、`lib.rs`、`tests/apis_common/mod.rs`、他の担当のファイルは変更していない。
- 後始末できていないもの（削除禁止のため）: `lift/out-oleaut/` 全体は **約 4.1 GB**（`du`: `target-test` 1.8 GB、`parts` 1.0 GB、`out` 868 MB、`bin` 487 MB、`target-tool` 26 MB）。**個々のファイルは 2 GB 未満**（最大は 255 MB の実行ファイルと 217 MB の C）で、2 GB 級の中間ファイルは作っていない。`out-pdata` の大きなファイルは複製していない（C は新しく生成した）。`target-test` はワークスペース全体のテスト用のビルド出力。
