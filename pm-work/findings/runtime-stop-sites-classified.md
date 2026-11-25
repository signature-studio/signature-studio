# 暫定の停止の分類と返り値(Issue #1418、段階 1)

`runtime-stop-sites.md` の (b) の行(1,241 行)を、`runtime-stop-sites-classified.tsv` に写し、`区分`・`返り値の規約`・`状態` を足した。(a) の 123 行と定義の 1 行は含めない。表の行の数は md の (b) の行の数と一致する(未分類 0)。

## 1. 内訳

| 区分 | 件数 |
| --- | --- |
| 失敗が正規の結果 | 699 |
| 判断保留 | 375 |
| 止めるべき | 167 |

| 返り値の規約 | 件数 |
| --- | --- |
| NONE(止めるべき・判断保留の行) | 542 |
| BOOL | 282 |
| COUNT | 145 |
| HANDLE | 97 |
| HRESULT | 70 |
| ERROR | 66 |
| POINTER | 21 |
| VOID | 18 |

| 状態 | 件数 |
| --- | --- |
| 残した(止めるべき・判断保留) | 542 |
| 未了(失敗が正規の結果。まだ直していない) | 694 |
| 直した | 5 |

## 2. 分類の規則(機械で当てた。境界は人が見る)

区分は、次の順で当てた。

1. 止めるべき: 行のファイルと関数が、止めるべき領域(スレッド・同期・メモリ・SEH・コールバックの戻り先・ロードの根幹・描画の根幹)のもの。`fle_thread*.c`・`fle_event.c`・`fle_glob.c`(メモリ)・`fle_vmap.c`(仮想メモリ)・`fle_seh.c`・`fle_callback.c`・`fle_desk.c`・`fle_raster.c`、`fle_load.c` の読み込みの中身(`LoadLibrary*`・`FreeLibrary` の API の行を除く)、`fle_realmod.c` の表と PE の読み(`table_check`・`read_headers`)、`fle_gdi.c` の画素と DC の中身(`need_mem_dc`・`screen_state`・`clip_reserve`・`slot_get`・`pixel_stop`・`get_bitmap`・`dib_color_table`・`set_dc_color`・`RealizePalette`・`ScrollDC`)、`fle_proc.c` の終了とスレッドの優先度、`fle_vfs.c` の実体の読み込み(`node_reserve`・`node_materialize`・`lazy_fetch`)、`fle_win.c` の `thunk_target`。
2. 判断保留: 止める条件の文が、Windows の答えそのものが分からないと言っている行(「the page does not define」「does not say」「gives no behaviour」「not decided」「no page」など。「names no error code」「names no HRESULT」は、失敗はあると書いているので除く)。加えて、`fle_proc.c` の `SetErrorMode`、`fle_timerq.c` の全部、`fle_dwm.c`・`fle_webview2.c`、`fle_win.c` のメッセージループ・フック・描画(`PeekMessageW`・`GetMessageW`・`BeginPaint`・`SetWindowsHookExW` など)、`fle_dsound.c`・`fle_filedlg.c` などの機能の未対応のうち、機能の範囲を決める必要があるもの。
3. それ以外は、失敗が正規の結果(ファイル・レジストリ・環境・シェル・版情報・ネットワーク・COM・クリップボード・デバイス・ウィンドウ・資源 など。利用者の操作で失敗がありうる)。

返り値の規約は、止めるべき・判断保留の行を NONE とし、それ以外は API の戻り値の型で当てた(関数名の規則。`SPECIAL` の表を含む)。型の表は Microsoft の宣言に合わせたが、人が一つずつ確かめてはいない。`—` の行(内部の関数)は、括弧の関数名の API の型で当てた。

## 3. 行の番号について(ずれ)

md の行の番号は #1411 の時点のもので、今の `lift/runtime` と一致しない行がある(例: `fle_reg.c` の `RegEnumValueW` の NULL は md が 900、今は 934)。TSV の `行` は md のまま写した。直した 5 行の、今の行は次のとおり。

| 直した行(md) | 今の行 | API |
| --- | --- | --- |
| fle_realmod.c 291 | fle_realmod.c 296(`fle_realmod_getproc` の未ロードの分岐)。判定は fle_modules.c 444 行(`fle_realmod_loaded`) | GetProcAddress(登録されていないモジュール) |
| fle_reg.c 640 | fle_reg.c 674 | RegSetValueExW(lpData が NULL、cbData が 0 以外) |
| fle_reg.c 900 | fle_reg.c 939 | RegEnumValueW(lpValueName か lpcchValueName が NULL) |
| fle_vfs.c 1947 | fle_vfs.c 2129 | GetFileVersionInfoSizeW(ディレクトリ部の無い名前) |
| fle_vfs.c 2000 | fle_vfs.c 2193 | GetFileVersionInfoW(ディレクトリ部の無い名前) |

また、md の `API` 列は、行の直前の `fle_imp_` 関数の名前を機械で付けたもので、行の関数と違うことがある(例: `fle_env.c` 578〜618 は `SHGetFolderPathW` の並びにあるが、コンピュータ名の関数 `GetComputerNameExW` の行)。区分の判断は、行の関数で行った。

md の表に無い停止: `fle_stop_` の別名で止める行(`fle_locale.c` の `fle_stop_outside_d`、CompareStringW の非 ASCII)は、md の `fle_stop(` の数え方に入っていない。#1395 の範囲なので、この段階では直さない(状態は表に無い)。

## 4. 直した 5 行(段階 1)

| family | 行 | Windows の返り値 | 資料 |
| --- | --- | --- | --- |
| GetProcAddress(登録されていないモジュールのハンドル) | fle_modules.c・fle_realmod.c | NULL、`GetLastError` = 126(ERROR_MOD_NOT_FOUND) | Wine の `dlls/kernel32/tests/module.c`(`testGetProcAddress_Wrong`。2026 年 1 月の版) |
| レジストリ: RegSetValueExW(lpData NULL・cbData ≠ 0) | fle_reg.c | 998(ERROR_NOACCESS) | Wine の `dlls/advapi32/tests/registry.c`(`RegSetValueExW` の `REG_SZ, NULL, 4`。2024 年 12 月の版) |
| レジストリ: RegEnumValueW(名前か個数が NULL) | fle_reg.c | 87(ERROR_INVALID_PARAMETER) | 同じ `registry.c`(`RegEnumValueW` の `no valueW parameter` の段) |
| 版情報: GetFileVersionInfoSizeW・GetFileVersionInfoW(ディレクトリ部の無い名前) | fle_vfs.c | 0 / FALSE と ERROR_FILE_NOT_FOUND(2)。空の名前は ERROR_BAD_PATHNAME(161) | Wine の `dlls/kernelbase/version.c`(`GetFileVersionInfoSizeExW`。2026 年 9 月の版)。Microsoft の GetFileVersionInfoSizeW の説明(`fle_vfs.c` の注記が引く版)は、失敗で 0 と書くだけで、コードは書いていない |

版情報の 2 行は、探索の順序(LoadLibrary と同じ検索)を持たない暫定の答え。#1415 が探索の順序を入れるまで、この答えを使う。

## 5. 確かめられなかった・見送った family(段階 2 以降。PM が Issue にする)

- レジストリ(残り): RegEnumKeyExW の NULL 名(Wine の試験が見つからなかった)、RegQueryValueExW の lpReserved、RegDeleteKeyW の空の名前と NULL(試験が見つからなかった)、RegSetValueExW の型の違い(REG_DWORD の cbData 違い など)。
- シェルのフォルダ: SHGetFolderPathW の未対応の CSIDL・hToken・dwFlags・pszPath NULL(Microsoft Learn の「SHGetFolderPathW function (shlobj_core.h)」(2025 年 7 月の更新)は HRESULT を返すと書くが、失敗のコードを書いていない。Wine の `dlls/shell32/tests/shlfolder.c` は、不正な CSIDL に E_INVALIDARG を置くが、未対応の正しい CSIDL の答えではない)。
- ロケール: CompareStringW(非 ASCII)は #1395 の範囲。`fle_locale.c` の `fle_stop_outside_d` で止める。
- 版情報の残り: 空でない名前の探索(#1415)、`lpData` NULL・`dwLen` 0 の GetFileVersionInfoW、仮想ファイルの版情報。
- ウィンドウ・GDI・DirectSound の多くは、失敗の返り値は分かるが、機能の範囲(何を答えるか)を決める必要があり、「未了」のまま。

## 6. 止めるべき・判断保留の行(542 行)

止めたままにした。止めるべき(167 行)は、メモリ・スレッド・同期・SEH・コールバック・ロード・描画の根幹の行。判断保留(375 行)は、PM が決める。
