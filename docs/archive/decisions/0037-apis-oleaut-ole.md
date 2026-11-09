# 0037: oleaut32 の BSTR／VARIANT／SafeArray／GetErrorInfo と ole32 の初期化・割り当て・CLSID 文字列の応答（W4）

状態: 作業者（M1、サブエージェント）が決定。実装は `lift/runtime/fle_oleaut.h/.c`、`fle_ole.h/.c`、共有ヘッダ `fle_com.h`（HRESULT 値）、`fle_ptrmap.h`（生存ポインタ表）。表への追記は `lift/crates/fe-lift/src/imports_table.rs` の 28 行。テストは `lift/crates/fe-lift/tests/oleaut_ole.rs`（40 件）。実行記録は `docs/investigations/apis-oleaut-ole-run.md`。対象は `docs/investigations/unsupported-inventory-pdata.md` §6 W4。

## 出典

公式（Microsoft Learn。2026-10-20 に本文を取得して読んだ）:

- BSTR: [BSTR](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/automat/bstr)、[String Manipulation Functions](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/automat/string-manipulation-functions)（「NULL ポインタは BSTR の有効値で、慣習としてゼロ文字の BSTR と同じに扱う」の注記は、`?redirectedfrom=MSDN` 付きの版の検索結果に出た。素の URL の取得本文には無かった）、[SysAllocStringLen](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-sysallocstringlen)、[SysFreeString](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-sysfreestring)、[SysReAllocStringLen](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-sysreallocstringlen)、[SysStringLen](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-sysstringlen)
- VARIANT: [VARENUM](https://learn.microsoft.com/en-us/windows/win32/api/wtypes/ne-wtypes-varenum)、[VARIANT](https://learn.microsoft.com/en-us/windows/win32/api/oaidl/ns-oaidl-variant)、[VariantInit](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-variantinit)、[VariantClear](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-variantclear)、[VariantCopy](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-variantcopy)、[VariantCopyInd](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-variantcopyind)、[VariantChangeType](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-variantchangetype)
- SafeArray: [SAFEARRAY](https://learn.microsoft.com/en-us/windows/win32/api/oaidl/ns-oaidl-safearray)、[SAFEARRAYBOUND](https://learn.microsoft.com/en-us/windows/win32/api/oaidl/ns-oaidl-safearraybound)、[SafeArrayCreate](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-safearraycreate)、[GetLBound](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-safearraygetlbound)、[GetUBound](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-safearraygetubound)、[AccessData](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-safearrayaccessdata)、[UnaccessData](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-safearrayunaccessdata)、[GetElement](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-safearraygetelement)、[PutElement](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-safearrayputelement)、[PtrOfIndex](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-safearrayptrofindex)、[GetElemsize](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-safearraygetelemsize)、[SafeArrayDestroy](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-safearraydestroy)、[SafeArrayCopy](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-safearraycopy)、[SafeArrayLock](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-safearraylock)、[SafeArrayGetVartype](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-safearraygetvartype)、[Array Manipulation Functions](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/automat/array-manipulation-functions)
- nDim が 1 始まりであること: 関数ページは書かない。Learn の例の [ADSI Example Code for Reading Attributes](https://learn.microsoft.com/en-us/windows/win32/adsi/example-code-for-reading-attributes)（`SafeArrayGetLBound(psa, 1, ..)`、コメント「One-dimensional array」）と [Mobile Broadband API Best Practices](https://learn.microsoft.com/en-us/windows/win32/mbn/mobile-broadband-best-practices) の 2 つの例が 1 次元配列に 1 を渡す。これは例からの**推論**（ページの明文ではない）。
- 型のサイズ: [CY](https://learn.microsoft.com/en-us/windows/win32/api/wtypes/ns-wtypes-cy-r1)（8 バイト整数）、[MS-OAUT IDL](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-oaut/b37f57b1-ac27-4bdf-9cf0-5b8ecccb65af)（`typedef LONG SCODE`）、VARENUM（DECIMAL は 16 バイト、VT_BOOL は VARIANT ページで 16 ビット）。
- ole32: [CoInitialize](https://learn.microsoft.com/en-us/windows/win32/api/objbase/nf-objbase-coinitialize)、[CoUninitialize](https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-couninitialize)、[OleInitialize](https://learn.microsoft.com/en-us/windows/win32/api/ole2/nf-ole2-oleinitialize)、[OleUninitialize](https://learn.microsoft.com/en-us/windows/win32/api/ole2/nf-ole2-oleuninitialize)、[CoTaskMemAlloc](https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-cotaskmemalloc)、[CoTaskMemFree](https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-cotaskmemfree)、[StringFromCLSID](https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-stringfromclsid)、[StringFromGUID2](https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-stringfromguid2)、[CLSIDFromString](https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-clsidfromstring)、[ProgIDFromCLSID](https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-progidfromclsid)、[IsEqualGUID](https://learn.microsoft.com/en-us/windows/win32/api/guiddef/nf-guiddef-isequalguid)、[GUID](https://learn.microsoft.com/en-us/windows/win32/api/guiddef/ns-guiddef-guid)、[CLSID key](https://learn.microsoft.com/en-us/windows/win32/com/clsid-key-hklm)、[ProgID key](https://learn.microsoft.com/en-us/windows/win32/com/-progid--key)、[HKEY_CLASSES_ROOT](https://learn.microsoft.com/en-us/windows/win32/sysinfo/hkey-classes-root-key)
- GetErrorInfo: [GetErrorInfo](https://learn.microsoft.com/en-us/windows/win32/api/oleauto/nf-oleauto-geterrorinfo)
- HRESULT の値（`fle_com.h`、テストは同じ値を独立にリテラルで持つ）: [Common HRESULT Values](https://learn.microsoft.com/en-us/windows/win32/seccrypto/common-hresult-values)（S_OK 0、E_UNEXPECTED 0x8000FFFF、E_INVALIDARG 0x80070057、E_OUTOFMEMORY 0x8007000E）、[Error Handling in COM](https://learn.microsoft.com/en-us/windows/win32/learnwin32/error-handling-in-com)（S_FALSE 0x1）、[COM Error Codes (Dispatch)](https://learn.microsoft.com/en-us/windows/win32/com/com-error-codes-2)（DISP_E_TYPEMISMATCH 0x80020005、BADVARTYPE 0x80020008、OVERFLOW 0x8002000A、BADINDEX 0x8002000B、ARRAYISLOCKED 0x8002000D）、[COM Error Codes (RPC)](https://learn.microsoft.com/en-us/windows/win32/com/com-error-codes-3)（RPC_E_CHANGED_MODE 0x80010106）、[COM Error Codes (Generic)](https://learn.microsoft.com/en-us/windows/win32/com/com-error-codes-1)（CO_E_CLASSSTRING 0x800401F3、REGDB_E_CLASSNOTREG 0x80040154）。

**非公式の出典は使っていない。** 検索結果に Eric Lippert のブログ、Stack Overflow、古い MSDN の転載が出たが、実装の根拠にはしていない。

## 閉包の実際の呼び出し（実測、`lift/objdump-text.txt`）

- IAT 経由の thunk は oleaut32 で 20、ole32 で 31（IsEqualGUID 3 thunk、SysFreeString は oleaut32 で 2 thunk）、計 51。対象 28 API は **31 thunk**（oleaut32 19、ole32 12）に当たる。残る 20 thunk は COM オブジェクトの作成や OLE のクリップボード、ドラッグなど（下の「未実装」）。
- `SafeArrayCreate` の呼び出しは 2 か所（0x45b259、0x4653d7）。**どちらも cDims（`edx`）は関数内の変数（0x45b252 は `[rbp+0x34]`、0x4653cf は `mov edx,ecx` の `ecx`）で、定数 1 ではない**。型は 0x45b259 が定数 0xc（VT_VARIANT）、0x4653d7 が変数。したがって実行時に 2 次元以上の配列が作られうる（Delphi の可変長配列の生成に読める。**推測**）。この実装は cDims が 1 以外なら止めるので、その入力が届いた時点で止まり、複数次元の設計判断が要る（下の「確認できなかったこと」2）。当初このファイルに「cDims は 1」と書いたのは誤りで、逆アセンブルを読み直して訂正した。
- `VariantChangeType` を呼ぶのは 0x4572c0 のラッパ 1 つ。ラッパは第 3 引数（`r8d`）が **0x400 でなければ E_NOTIMPL（0x80004001）を返す**（これはラッパ自身の lcid 引数への検査で、`VariantChangeType` の `wFlags` ではない。当初「wFlags = 0x400」と書いたのは誤りで訂正した）。0x400 のときは第 4 引数（`r9d`）を `wFlags`（`r8d`）に、スタックの WORD を `vt`（`r9`）にして `VariantChangeType` を呼び、結果が `DISP_E_TYPEMISMATCH` かつ元の vt に VT_ARRAY（0x2000）が立っていれば `DISP_E_BADVARTYPE` に置き換える。**wFlags の実際の値は呼び出し元次第で、この読みでは分からない**。呼び出しの形は無印の 4 引数版。
- これらの読みは逆アセンブルだけで、**実行では確かめていない**（実行の停止点はこれらより手前にある。run 記録 §5）。

## 決定

### アロケータと割り当てのアドレス

BSTR、SafeArray、CoTaskMem のブロックはすべてホストの `calloc` ブロック（16 バイト境界）。`fle_glob.c` の GlobalAlloc（固定）と同じ流儀で、**生きているポインタを `fle_ptrmap.h` の開番地ハッシュに記録**し、記録に無いポインタの解放・操作は `fle_stop`（名前付き abort）にする。理由: ページは二重解放や他のアロケータの値の扱いを決めない。`fle_sys.c` の `HeapAlloc` と同じく、仮想ヒープのハンドル領域（`FLE_PROCESS_HEAP`）とは別物で、ゲストにはポインタだけが見える。BSTR の接頭辞 4 バイトのため、BSTR のポインタは 16 バイト境界から 4 ずれる（ページに整列の規定は無い）。

### BSTR（`fle_oleaut.c`）

1. **レイアウト**（BSTR ページ）: ポインタは先頭文字、直前 4 バイトにバイト数（終端を含まない）、末尾に NUL の WCHAR。
2. **SysAllocStringLen**: ui 文字をコピー、NUL を付ける（ページ）。埋め込み NUL も NUL なしの入力も可（ページ）。ui = 0 は空の BSTR を確保する（「ui + 1 文字を確保」）。**strIn が NULL のときの中身はページに無く、ゼロにした（設定）**。確保失敗は NULL（ページ）。上限 `FLE_BSTR_MAX_CHARS`（0x4000000 文字、**仮想機械の上限**）を超える要求は NULL。
3. **SysFreeString**: NULL は何もしない（ページ）。生きている BSTR でない値は止める。
4. **SysReAllocStringLen**: 新しい BSTR を作って内容をコピーし、古い BSTR を解放し、`*pbstr` を差し替えて TRUE（ページ）。**psz が NULL なら長さ len の未初期化（ここではゼロ）**（ページ）。**pbstr が NULL は止める**（ページ: アクセス違反）。`*pbstr` が NULL は「NULL は有効な BSTR」の慣習に従い、何も解放しない。**ページは `*pbstr` が NULL のときを書かないので、これは慣習からの推論**。コピーは解放の前に行うため、psz が古い文字列の内側でも安全。古い文字列の終端を越えて読む長さは止める。確保失敗は FALSE で `*pbstr` は変えない（ページは書かない。推論）。

### VARIANT（`fle_oleaut.c`）

レイアウトは VARIANT ページの構造体を自然境界で置いた x64 の 24 バイト（vt は 0、共用体は 8 から 16 バイト）。**この 24 バイトは構造体の定義からの推論。**

1. **VARTYPE の分類**（VARENUM の VARIANT 列）: VARIANT に使える型は VT_BOOL VT_BSTR VT_CY VT_DATE VT_DECIMAL VT_DISPATCH VT_EMPTY VT_ERROR VT_I1 VT_I2 VT_I4 VT_INT VT_NULL VT_R4 VT_R8 VT_RECORD VT_UI1 VT_UI2 VT_UI4 VT_UINT VT_UNKNOWN VT_VARIANT と、VT_ARRAY、VT_BYREF。`VT_VECTOR`、`VT_RESERVED` が立つもの、表に無い値は **DISP_E_BADVARTYPE**。次は表が決めないので止める: **VT_I8／VT_UI8**（VARENUM の VARIANT 列に無いが VARIANT 構造体に llVal／ullVal がある）、**VT_VARIANT（BYREF なし）**、**VT_BYREF | VT_EMPTY／VT_NULL**。
2. **VariantInit**: `vt` を VT_EMPTY にするだけ（ページ）。他のバイトは触らない。NULL は止める。
3. **VariantClear**: 先に中身を解放し、vt を VT_EMPTY にする（ページ）。VT_BSTR は文字列を解放。VT_ARRAY は配列を解放し、ロックされていれば **DISP_E_ARRAYISLOCKED**（vt は変えない）。VT_BYREF は何も所有しない（ページは VT_DISPATCH／VT_UNKNOWN の BYREF について明記。**他の BYREF 型への拡張は、VT_BYREF が「ローカル用の void ポインタ」というページの記述からの推論**）。不正な vt は **DISP_E_BADVARTYPE**。NULL は **E_INVALIDARG**（ページが「引数が不正」を挙げる）。
4. **VariantCopy**: dest を VariantClear と同様に解放してから src の完全な複製を作る（ページ）。VT_BSTR は文字列を複製、VT_ARRAY は配列全体を複製（ページ）。BYREF は参照を変えずにコピー（ページ）。順序は **src の型を先に検査（不正なら dest は無変更）→ dest を解放（失敗したらその HRESULT を返しコピーしない）→ コピー**（ページに順序の規定は無く、これは選択）。dest = src はページが書かないので止める。NULL は E_INVALIDARG。**NULL の BSTR は NULL のままコピーする**（「NULL はゼロ文字の BSTR と同じに扱う」慣習からの推論）。
5. **VariantCopyInd**: 値のコピーで、BYREF を外す（`VT_BYREF | VT_I2` は値渡しの `VT_I2`。ページの例）。`VT_BYREF | VT_VARIANT` は中の VARIANT をコピーし、それが BYREF ならさらに外す（ページ）。中の VARIANT が `VT_VARIANT | VT_BYREF` は VARIANT ページが禁じるので止める。ポインタが NULL の BYREF はページが書かないので止める。値渡し側で使わないバイトはゼロ（設定）。
6. **VariantChangeType**: ページは変換を例でしか書かず（数値と文字列、BOOL は先に VT_I2、オブジェクトの Value プロパティ）、丸めの規則も表も、wFlags の数値（ページはフラグ名だけ）も無い。**実装したのは、ページが決めている範囲のみ**: 成功時に結果の vt が要求した型（ページ）、`dest == src` はその場で変換（ページ）、BYREF の元は参照先の値（ページ）、不正な vt は DISP_E_BADVARTYPE、NULL は E_INVALIDARG、そして**同じ型への変換（値は変えない。推論）**。wFlags が 0 以外、型が違う変換、BYREF の元で型が違う変換、配列、BYREF や配列への変換は止める。**閉包のラッパの呼び出し元が実際にどの型へ変換するかは未調査**（`VariantChangeType` の呼び出し元は 1 か所のラッパだけだが、そのラッパの呼び出し元は読んでいない）。型が違う変換が実際に使われるなら、この関数は最初に止まる候補になる。

### SAFEARRAY（`fle_oleaut.c`）

レイアウトは SAFEARRAY ページの構造体（cDims 2、fFeatures 2、cbElements 4、cLocks 4、pvData は 16、rgsabound は 24。SAFEARRAYBOUND は cElements 4 と lLbound 4）。

1. **SafeArrayCreate**: 描画子とデータを確保し、**データはゼロ（ページは「初期化する」。値は推論）**。NULL を返すもの: VT_EMPTY／VT_NULL／VT_ARRAY／VT_BYREF 付き（ページ）、`rgsabound` が NULL（推論）、上限 1 GB を超える要求（仮想機械の上限）。要素の大きさ: I1 UI1 は 1、I2 UI2 BOOL は 2、I4 UI4 INT UINT R4 ERROR は 4、R8 CY DATE は 8、DECIMAL は 16、BSTR UNKNOWN DISPATCH は 8（ポインタ）、VARIANT は 24。fFeatures は **FADF_BSTR／UNKNOWN／DISPATCH／VARIANT を対応する要素型、その他は FADF_HAVEVARTYPE（設定。ページは各フラグの意味だけで、SafeArrayCreate がどれを立てるかは書かない）**。
2. **次元は 1 だけ実装**する。ページが複数次元の順序について一貫しないため（SAFEARRAY ページは左端の境界が `rgsabound[0]`、SafeArrayGetElement ページは右端の添字が `rgIndices[0]`、配列の関数の概説は列優先と書く）。cDims が 1 以外、cElements が 0（ページが書かない）、VT_RECORD、VT_I8／VT_UI8、表に無い型は止める。**閉包の SafeArrayCreate 呼び出しは cDims が変数なので（上の「閉包の実際の呼び出し」）、2 次元以上の入力は実際に届きうる。そのときは止まる**（誤った添字順で配列を作るより止める方を選んだ）。
3. **GetLBound／GetUBound**: 第 1 次元は nDim = 1（上記の例からの推論）。nDim が 0 か cDims 超過は **DISP_E_BADINDEX**。上限は lLbound + cElements − 1 で、LONG に収まらなければ **DISP_E_OVERFLOW**（ページ）。NULL は **E_INVALIDARG**。
4. **AccessData／UnaccessData**: ロック数を増減（ページ）。ロック数が最大で Access は **E_UNEXPECTED**（ページ「ロックできない」）。ロック数 0 の Unaccess は **E_UNEXPECTED**（ページ「アンロックできない」。SAFEARRAY ページも不整合なロック数が UnLock の E_UNEXPECTED になると書く）。NULL は E_INVALIDARG。Access で得たポインタは Unaccess の後も使えるまま（ページは無効になると書くが、無効化はしない）。
5. **GetElement／PutElement／PtrOfIndex**: 範囲外は **DISP_E_BADINDEX**、NULL は **E_INVALIDARG**（ページ）。文字列は新しい BSTR にコピー、VARIANT は VariantCopy 相当、既存の要素は正しくクリア（ページ）。PutElement の VT_BSTR／UNKNOWN／DISPATCH の pv はポインタそのもの（ページ）。**NULL の pv は BSTR とインタフェースでは NULL 値、それ以外の型では E_INVALIDARG（推論）**。ロック中でも PutElement はできる（ページの注）。
6. **GetElemsize**: cbElements（ページ）。NULL はページが書かないので止める。
7. **SafeArrayDestroy 相当**（VariantClear の VT_ARRAY が使う）: ロック中は DISP_E_ARRAYISLOCKED、BSTR 要素は SysFreeString、VARIANT 要素は VariantClear（ページ）、オブジェクト要素は Release が要る（呼び返し）ので非 NULL なら止める。**SafeArrayDestroy／SafeArrayCopy は IAT に無い**ので関数としては出さず、内部の処理だけ。

### GetErrorInfo（`fle_oleaut.c`）

「同じ論理スレッドで最後に SetErrorInfo されたエラー情報、無ければ S_FALSE」（ページ）。仮想環境には SetErrorInfo を呼ぶ COM オブジェクトが無い（閉包は SetErrorInfo を import しない）ので、常に **S_FALSE**。`*pperrinfo` には NULL を書く（設定。ページは [out] の値を書かない）。dwReserved が 0 以外、pperrinfo が NULL は止める（ページ: 予約、0 でなければならない）。

### COM の初期化状態（`fle_ole.c`）

1. **スレッドモデルは常に STA（決定）**。CoInitialize ページは「STA を指定する」、OleInitialize ページも STA。ゲストは単一スレッドで、閉包は `CoInitializeEx` を import しない。したがって MTA に初期化された後の CoInitialize が返す **RPC_E_CHANGED_MODE（0x80010106）は発生させない**（値は `fle_com.h` に公式の値で置いたが使わない）。
2. **初期化は S_OK、2 回目以降は S_FALSE**（CoInitialize ページ、OleInitialize ページ）。S_FALSE の呼び出しも CoUninitialize／OleUninitialize と対になる（ページ）。内部は「対のない成功呼び出し」の数 `com_depth` と、そのうち OleInitialize の数 `ole_depth`。
3. CoUninitialize は `com_depth` を 1 減らす。OleUninitialize は `ole_depth` と `com_depth` を 1 ずつ減らす（ページ: 内部で CoUninitialize を呼ぶ）。**対のない呼び出し、OleInitialize の分を CoUninitialize で外す呼び出し、CoInitialize の分を OleUninitialize で外す呼び出しは、ページが結果を書かないので止める**。pvReserved が NULL でないと止める（ページ「NULL でなければならない」が、結果は書かない）。
4. **ページは「最初に初期化した呼び出しは最後に解放される」ことと DllMain から呼ばないことを書く**が、いずれも単一スレッドの仮想環境では検査しない。

### CoTaskMem（`fle_ole.c`）

CoTaskMemAlloc: cb バイトのブロック（ページ）、内容は未定義とページが書くのでゼロ（設定）、cb = 0 は長さ 0 の有効なポインタ（ページ）、確保失敗は NULL（ページ）。上限 1 GB は仮想機械の上限（超えると NULL）。CoTaskMemFree: NULL は何もしない（ページ）、`CoTaskMemAlloc` が返した生きているブロックでない値は止める（`StringFromCLSID` の文字列もこのブロック）。

### GUID の文字列（`fle_ole.c`）

1. **StringFromCLSID**: 波括弧つきの `{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}`（38 文字、ページ）で、`CoTaskMemAlloc` したブロック（ページ）に書く。**16 進の大文字小文字はページに無く、GUID ページの例が大文字なので大文字（設定）**。NULL の引数はページが書かないので止める。
2. **CLSIDFromString**: `{` で始まる 38 文字の形（ページ）を CLSID にする（Data1／Data2／Data3 はリトルエンディアン。GUID ページの構造体）。**入力は大文字小文字の両方を受け付ける（推論）**。NULL は GUID_NULL（ページ）。形が違えば **CO_E_CLASSSTRING**（ページ）で、出力は書かない（設定）。pclsid が NULL は E_INVALIDARG（ページが標準値として E_INVALIDARG を挙げる）。**`{` で始まらない文字列は ProgID の検索で、ProgID の解決は実装していないので止める**（ページは ProgID を渡せると書く）。
3. **ProgIDFromCLSID**: レジストリの内容で決まる（ページ）。仮想レジストリ（`fle_reg.c`、決定 0018）は起動時に空で、HKCR は HKLM／HKCU の統合表示ではない（HKCR ページは統合と書く）。`CLSID\{...}`（HKCR）、`SOFTWARE\Classes\CLSID\{...}`（HKLM）、`Software\Classes\CLSID\{...}`（HKCU）のいずれのキーも無ければ **REGDB_E_CLASSNOTREG**（ページ）。ゲストがそのキーを作っていた場合、ProgID 値の読み方と統合の優先順位をページが決めないので止める。出力は書かない（設定）。
4. **IsEqualGUID**: 16 バイトを比べて 1／0（ページ: TRUE／FALSE）。NULL は止める。

### スレッド

`fle_glob.c` と同じく最初の呼び出しのスレッドを記録し、別スレッドからの呼び出しは止める。

## 呼び返しが要る入力（名前付き abort にしたもの）

`fle_stop` のメッセージは API 名と理由を出す。種類は次のとおり。

1. VT_UNKNOWN／VT_DISPATCH で**ポインタが非 NULL**: VariantClear（Release）、VariantCopy（AddRef）、VariantCopyInd（参照先のオブジェクト）。NULL のポインタは何も呼ばないので通す。
2. VT_UNKNOWN／VT_DISPATCH 要素の配列で**要素が非 NULL**: SafeArrayPutElement（AddRef と既存の Release）、SafeArrayGetElement（AddRef）、配列の解放と複製。
3. VT_RECORD（IRecordInfo）: VARIANT と SafeArrayCreate の両方。
4. VariantChangeType の「オブジェクトの Value プロパティ」（UNKNOWN／DISPATCH から他の型）は「型が違う変換」として止まる。

## 実装しなかったもの

- IAT に実在するが対象外にした API: oleaut32 の **GetActiveObject**（COM のクラス表に当たる。W11）、ole32 の **CoCreateInstance／CoGetClassObject／CoGetInterfaceAndReleaseStream／CoMarshalInterThreadInterfaceInStream／CreateDataAdviseHolder／CreateILockBytesOnHGlobal／CreateStreamOnHGlobal／DoDragDrop／IsAccelerator／OleDraw／OleFlushClipboard／OleGetClipboard／OleIsCurrentClipboard／OleSetClipboard／OleSetMenuDescriptor／RegisterDragDrop／ReleaseStgMedium／RevokeDragDrop／StgCreateDocfileOnILockBytes**（いずれも W10／W11 の COM とウィンドウの実体が要る）、遅延 import の **PropVariantClear**（`jmp rax` の前提 W1 が別担当）。
- IAT に無い API（実在の一覧を正とし、関数としては出していない）: **SafeArrayDestroy、SafeArrayCopy、SafeArrayLock、SafeArrayUnlock、SysAllocString、SysStringLen** など。依頼の一覧にあったのではなく、仕様の確認のために読んだもの。配列の解放と複製の処理だけは VariantClear／VariantCopy／SafeArray の内部で使う。**依頼の「SafeArray* 9」は IAT の実在 9 個（Create、GetLBound、GetUBound、AccessData、UnaccessData、GetElement、PutElement、PtrOfIndex、GetElemsize）に一致した。**
- 上の「VariantChangeType」の数値・文字列・日付などの実際の型変換と、wFlags の数値（VARIANT_NOVALUEPROP などはページに数値が無い）。

## 確認できなかったこと（実装せず、止めるか設定とした）

1. VariantChangeType の変換表、丸め、wFlags の数値、オブジェクトの Value プロパティ（ページは例だけ）。
2. 複数次元の SafeArray の境界と添字の順序（ページ間で書き方が違う）。
3. SysAllocStringLen の strIn が NULL のときの中身、SysReAllocStringLen の `*pbstr` が NULL のとき。
4. SafeArrayCreate が立てる fFeatures、cElements が 0 の配列、cDims が 0 の配列。
5. VT_I8／VT_UI8 を VARIANT と SafeArray に使えるか（VARENUM の表と VARIANT の構造体が食い違う）。
6. CoUninitialize／OleUninitialize の対のない呼び出しの結果、pvReserved が NULL でないときの結果。
7. ProgIDFromCLSID の ProgID 値の読み方と、HKCR の統合表示の優先順位。仮想レジストリが空のときだけ答える。
8. StringFromCLSID の 16 進の大文字小文字、CLSIDFromString が受け付ける大文字小文字。
9. VARIANT／SAFEARRAY の x64 の大きさ（構造体定義からの推論で、実機の `sizeof` は確かめていない）。

## 代替案

- 閉包にある VariantChangeType の呼び出しは LOCALE_USER_DEFAULT の無印版なので、「ホストの数値・文字列変換（`strtol`、`snprintf`）で VT_I4 と VT_BSTR を相互変換する」案があった。**却下**: ロケール依存でページに書式規則が無く、ホストの実物を見せることになる。
- `CoInitialize` の 2 回目に S_OK を返し続ける案（状態なし）。**却下**: ページが S_FALSE と対の呼び出しを明記する。
- BSTR を `fle_vmap.c` の仮想マップに置く案。**却下**: ポインタが 4 ずれる接頭辞つきの小さなブロックで、`GlobalAlloc` と同じホストの `calloc` で足りる。
- CoTaskMem と BSTR を 1 つのアロケータ表にする案。**却下**: BSTR の SysFreeString に CoTaskMem のポインタ（逆も）が渡る誤りを検出するため、別々に持つ。

## 限界

- 照合はページと、テスト内の独立のモデル（HRESULT、VARIANT／SAFEARRAY の構造体のリテラル）だけ。Windows 実機との比較はしていない。
- 閉包の呼び出し箇所の読みは逆アセンブルだけで、実行では確認していない。`SysFreeString` などは all-pdata の実行でまだ届いていない。
- 呼び返しが要る入力は止めるので、COM オブジェクトを持つ VARIANT が実際に現れたら、その時点で呼び返し機構の設計が要る。
