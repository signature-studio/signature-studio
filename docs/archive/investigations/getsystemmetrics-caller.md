# GetSystemMetrics の呼び出し元の調査と、仮想システムメトリクスの実装後の harness の再実行の記録（決定 0057）

実行日: 2026-10-21。作業ディレクトリ `signature-studio-develop`、HEAD 257990b。DLL は `binaries/win64/FLEngine_x64.dll`。決定は `docs/decisions/0057-virtual-system-metrics.md`。「実測」はコマンドの出力、「読み」は `lift/objdump-text.txt` の逆アセンブルの静的な読み、「推測」は未検証。成果物は `lift/out-sm/`（gitignore 済み。`TMPDIR=lift/out-sm/tmp`、`EM_CACHE=lift/out-sm/emcache`、`CARGO_TARGET_DIR=lift/out-sm/target`）。補助スクリプトは `lift/out-sm/work/`（`sites.py` = 呼び出し点と直前の `ecx`、`table.py` = 関数と SM 名への対応、`reach.py` = 静的な到達範囲、`mkmut.py`／`mkmut2.py` = 変異、`runmut.sh`）。

## 1. 停止点の確認（実測）

停止は `fle: unsupported instruction at 0x424ee0`。0x424ee0 は `ff 25 b2 bf 54 01`（`jmp [rip+0x154bfb2]`）で、IAT スロットは 0x1970e98、pefile の import 表で `user32.dll!GetSystemMetrics`（実測）。呼び出しスタック（native と wasm で同じ）: `f_424ee0` ← `f_620860`（guest-return 0x620872）← `f_6209a0` ← `f_620640` ← `f_620cd0` ← `f_623840` ← `f_4120c0`（0x412129）← `f_412160` ← `f_41d8f0` ← エントリ。関数実行数 572、入口数 13,230、深さ 10、unsupported_total 547（開始時点の基準）。

## 2. 全呼び出し点（読み。静的に追える範囲）

- `objdump` のテキストで `0x424ee0` を指す命令: **`call rel32` が 54 か所、`jmp`／条件ジャンプは 0**（実測: `grep`）。スロット 0x1970e98 を直接読む命令は thunk 自身だけ。`# 0x424ee0` のような他の参照（`lea` など）はテキストに無い。DLL のファイルに 0x424ee0 の 4／8 バイト定数も無い（実測）。関数ポインタ経由の呼び出しは、この調査では**追えない**（間接呼び出しの先は未確認）。
- 54 か所のうち、直前の `mov ecx,imm32`（または `xor ecx,ecx`）で index が読めるのは **50 か所、動的が 4 か所**。静的な index の種類は **36**。
- 動的の 4 か所:
  - f_858f00 の 0x858f35、0x858f5d: `movzx ecx,di`（第 3 引数 `r8w`）。f_858f00 を呼ぶ直接の呼び出しは 0x858fe1（`r8w = 0x15`）と 0x859007（`r8w = 0x14`）の 2 か所（読み）。つまり実際は **0x15（SM_CXHSCROLL）と 0x14（SM_CYVSCROLL）**。ほかの呼び出し元（間接）は追えていない。
  - f_859470 の 0x859495、f_859610 の 0x859635: `mov ecx,[0x14082e0 + b*4]`（f_859470）、`[0x14082e8 + b*4]`（f_859610）。b は `[rbx+0x20]` のバイト。表の中身（実測: ファイルの dword）は 0x15、0x2、0x3、0x14、0x200、0x2、0x10、0x4、0x80、0x100、0x1、0x20、0x8、0x40、0x0、0x1（0x14082e0 から）。b の取りうる範囲と、`[rbx+0x20]` の書き手は**追えていない**。b = 0、1 なら f_859470 は 0x15（SM_CXHSCROLL）、0x2（SM_CXVSCROLL）、f_859610 は 0x3（SM_CYHSCROLL）、0x14（SM_CYVSCROLL）。b が 2 以上だと f_859470 は 0x3、0x14、0x200（SM_* の番号ではない）…になり、**そのとき GetSystemMetrics は名前の無い番号で停止する**（実装の規則）。

### 2.1 呼び出し点の一覧（関数ごと。SM 名は Learn の表）

| 関数 | 呼び出し点: index | 戻り値の使われ方（読める範囲） |
|---|---|---|
| f_454090 | 0x454794: 89 SM_SERVERR2 | `test eax,eax; jne`。0 のとき 1 つの文字列、非 0 のとき別の文字列を選ぶ（OS の説明を作る経路の一部と読める。推測） |
| f_5c5500 | 0x5c55a9: 11 SM_CXICON、0x5c55bc: 12 SM_CYICON | 出力引数 `[rcx]`、`[rcx+4]` に保存（2 値がともに 0 のときだけ） |
| f_5d0010、f_5d0050 | 0x5d0040、0x5d0080: 12 SM_CYICON | 戻り値を返す |
| f_6197b0 | 0x61990c、0x61991b: 20 SM_CYVSCROLL、0x619932: 21 SM_CXHSCROLL | レイアウトの計算（読んでいない） |
| **f_620860** | **0x62086d: 19 SM_MOUSEPRESENT** | `test eax,eax; setne al; mov [rbx+0x10],al`（1 バイトのブール値） |
| **f_620880** | **0x62088d: 75（0x4b）SM_MOUSEWHEELPRESENT** | `setne al; mov [rbx+0x28],al`。真のとき `SystemParametersInfoW(0x68, 0, &[rbx+0x14], 0)`（thunk 0x4255e0。次の停止点） |
| f_631780 | 0x631847、0x63187c: 6 SM_CYBORDER | `imul`、`(x+3)/4 + 4*値`（余白の計算） |
| f_6369d0、f_636cd0 | 0x6369f6: 21 SM_CXHSCROLL、0x636edf: 10 SM_CXHTHUMB | レイアウト |
| f_700e10 | 0x700ecd: 3 SM_CYHSCROLL、0x700efa: 21 SM_CXHSCROLL | レイアウト |
| f_806910 | 0x806a33、0x806a44: 32 SM_CXSIZEFRAME | レイアウト |
| f_80e360 | 0x80e377: 94 SM_DIGITIZER、0x80e3a4: 86 SM_TABLETPC、0x80e3b9: 0x2003 SM_CONVERTIBLESLATEMODE、0x80e3ce: 0x2004 SM_SYSTEMDOCKED | グローバル 0x15f6ec8〜0x15f6ecc にブール値で保存。`SM_DIGITIZER` は bit 0（`test eax,1`）と bit 2/3（`test eax,0xc`）を別々に保存。**`SM_CONVERTIBLESLATEMODE` は `sete`（値が 0 のときに 1 を保存）**で、Learn は「0 は Slate Mode」と書く（デスクトップ PC には適用されないとも書く）。値の決め方で挙動が変わるので、この決定では答えない |
| f_80e3f0 | 0x80e506: 95 SM_MAXIMUMTOUCHES | タッチの有無（読んでいない） |
| f_815050、f_86bd40 | 0x8150bb: 13 SM_CXCURSOR、0x8150cb、0x86bd5c: 14 SM_CYCURSOR | カーソルの大きさ |
| f_81c2a0 | 0x81c318: 68 SM_CXDRAG、0x81c325: 69 SM_CYDRAG | 引数 `edi` が負で -1 でないとき、ドラッグ閾値として使う（`edi >= 0` ならその値、`-1` なら別の計算） |
| f_858f00、f_859470、f_859610 | 動的（上の §2） | スクロールバーの幅・高さ（0x15、0x14、0x2、0x3） |
| f_85acb0 | 0x85ad17: 49 SM_CXSMICON、0x85ad2f: 50 SM_CYSMICON | 小アイコンの大きさ |
| f_8621c0 | 0x86221e: 15 SM_CYMENU | メニューの高さ |
| f_865700、f_865720 | 0x865709: 1 SM_CYSCREEN、0x865726: 0 SM_CXSCREEN | 1 つの値を返すだけの包み |
| f_865730、f_865750、f_865770、f_865790 | 0x865739: 77、0x865759: 76、0x865779: 79、0x865799: 78（SM_Y/X/CY/CXVIRTUALSCREEN） | 同上 |
| f_8657d0 | 0x8657e7: 80 SM_CMONITORS | `[[rcx+0xbc]+0x10]` が 0 のときだけ呼び、非 0 ならその値を返す（上書きの値） |
| f_866760 | 0x866830: 61 SM_CXMAXIMIZED | レイアウト |
| f_867e10、f_869820 | 0x867efc、0x8698e0: 0 SM_CXSCREEN、0x867f08、0x8698ec: 1 SM_CYSCREEN | 画面の大きさ |
| f_a0ef50 | 0xa0efab、0xa0efde: 2 SM_CXVSCROLL | レイアウト |
| f_a15980 | 0xa159cd: 7 SM_CXFIXEDFRAME、0xa159da: 8 SM_CYFIXEDFRAME、0xa15a03: 45 SM_CXEDGE、0xa15a10: 46 SM_CYEDGE | 枠の厚さ |
| f_a253e0 | 0xa2569f: 3 SM_CYHSCROLL | レイアウト |

「レイアウト」と書いた箇所は、戻り値を寸法の計算に使うと読んだだけで、関数の目的の全体は読んでいない（推測を含む）。**ダブルクリック矩形（SM_CXDOUBLECLK／SM_CYDOUBLECLK、36／37）の呼び出しは無い**（ドラッグ閾値の 68／69 はある）。SM_REMOTESESSION、SM_SHUTTINGDOWN、SM_REMOTECONTROL、SM_CMOUSEBUTTONS、SM_SWAPBUTTON、SM_CYCAPTION、SM_CXBORDER も静的な index には出ない（動的の 4 か所の b が表のほかの値を取れば別。§2）。

静的な index の 36 種: 0、1、2、3、6、7、8、10、11、12、13、14、15、19、20、21、32、45、46、49、50、61、68、69、75、76、77、78、79、80、86、89、94、95、0x2003、0x2004（実測: `lift/out-sm/work/table.txt`）。

## 3. 次の停止点までの初期化経路（読み）

停止点の呼び出しスタック（§1）の途中の読み:

1. **f_620cd0**（0x620cd0）は `f_620640(rcx = [0x602b18], dl = 1)` を 0x620dcd で呼び、結果を 0x15f3208 に保存する（0x620dd2）。
2. **f_620640** はオブジェクトを作る（`rcx` 非 0 のとき `dl` を引数に別の関数で確保。読み）。`[obj+8] = 1`、`[obj+0xc] = 5` の後に、**OS のバージョンの問い**をする: `f_44acd0() == 2`（プラットフォーム ID。`[0x15ec7e8]`。決定 0014 の `dwPlatformId = 2`）かつ `f_44acf0() >= 4`（`[0x15ec584]` を返す。バージョンの番号と読めるが、0x15ec584 の書き手は追っていない。**推測**）なら `[obj+0x11] = 1`。ほかの組み合わせでは 0 か 1 になる条件を並べた式（0x62068b〜0x6206cd）。**この環境では `[obj+0x11] != 0`**（実測に基づく: f_6209a0 は `[obj+0x11] == 0` のとき f_620880 を呼ばず f_6208c0 へ進むが、実行は f_620880 に着いた。停止時のスタックが f_620880 の中にある）。
3. 続けて `f_6209a0(obj, 0)` を呼ぶ（0x6206da）。**f_6209a0** は `edx` が 0 のとき（`edx-1 == -1`）、まず **f_620860(obj)** を呼び（0x6209bc）、`[obj+0x11] == 0` なら **f_6208c0**（旧式のホイール対応）、そうでなければ **f_620880(obj)** を呼ぶ（0x6209d4）。`edx == 0x68` のとき（0x6209db）は、`[obj+0x28] != 0` なら、`[obj+0x11] != 0` で `SystemParametersInfoW(0x68, 0, &[obj+0x14], 0)`、そうでなければ `SendMessageW(...)`（設定変更のたびにスクロール行数を取り直す形と読める。推測）。
4. **f_620860**: `GetSystemMetrics(19 = SM_MOUSEPRESENT)` の結果を `[obj+0x10]` にブール値で保存する。**この値の読み手は追っていない**。
5. **f_620880**: `GetSystemMetrics(0x4b = 75 = SM_MOUSEWHEELPRESENT)` の結果を `[obj+0x28]` にブール値で保存し、真のとき **`SystemParametersInfoW(0x68, 0, &[obj+0x14], 0)`**（thunk 0x4255e0 は pefile の IAT で `user32.dll!SystemParametersInfoW`。実測）を呼び、結果の「スクロール行数」を `[obj+0x14]` に受ける。0x68 は Learn の SystemParametersInfo ページで **SPI_GETWHEELSCROLLLINES = 0x0068**（「Retrieves the number of lines to scroll when the vertical mouse wheel is moved. The *pvParam* parameter must point to a UINT variable … The default value is 3」。公式。2026-10-21 取得）。**これが今回実装後の次の停止点**。
6. **f_6208c0**（`[obj+0x11] == 0` の経路。この環境では通らないと読める）は `f_425810` を呼び、`FindWindowW(L"MouseZ", L"Magellan MSWHEEL")`、`RegisterWindowMessageW(L"MSWHEEL_ROLLMSG")`、`RegisterWindowMessageW(L"MSH_WHEELSUPPORT_MSG")`、`RegisterWindowMessageW(L"MSH_SCROLL_LINES_MSG")`、`SendMessageW` を使う（文字列は実測: ファイルの UTF-16。thunk は pefile の IAT）。**旧式（Windows 9x／NT 3.51 向け）のホイール対応の経路と読める**（推測）。

用途の分類（質問への答え）: マウスの有無は **f_620860 で `[obj+0x10]` に保存**（読み手は未追跡）。ホイールの有無は **f_620880 で `[obj+0x28]` に保存し、真のとき SPI_GETWHEELSCROLLLINES を呼ぶ**。ドラッグ閾値は f_81c2a0 の SM_CXDRAG／SM_CYDRAG。スクロールバー幅は f_6197b0、f_6369d0、f_700e10、f_a0ef50、f_a253e0 などの SM_CXVSCROLL、SM_CYHSCROLL、SM_CXHSCROLL、SM_CYVSCROLL。フレーム幅は f_806910（SM_CXSIZEFRAME）、f_a15980（SM_CXFIXEDFRAME ほか）。

## 4. 静的な到達範囲（読み。下限）

`lift/out-tid/out` の生成 C の直接呼び出し（`f_xxxxxx();`）だけを辿った閉包。`reach.py`。

| 根 | 閉包（直接／アドレス定数込み） | 間接呼び出しを含む関数 | 閉包にある GetSystemMetrics の使い手 |
|---|---|---|---|
| f_623840 | 610／869 | 73／123 | 直接: f_5c5500、f_620860、f_620880、f_867e10。アドレス定数込みで f_8657d0、f_869820 が加わる |

使い手は 35 関数（実測: 生成 C に `f_424ee0();` を持つ関数）。閉包にある 6 関数の index は 11、12、19、75、0、1、80 で、**この決定で全部答えがある**（決定 0057 の 14 index に含まれる）。間接呼び出し（`fle_dispatch(...)`）の先は追えていないので、閉包は下限。つまり GetSystemMetrics 自体の次の停止は、この閉包の範囲では出ないと読めるが、間接呼び出しの先は分からない。

## 5. 一次資料（Microsoft Learn。2026-10-21 取得）

| ページ | 確認した内容 |
|---|---|
| [GetSystemMetrics](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getsystemmetrics)（`updated_at` 2025-07-01、`ms.date` 2018-12-05） | **このページは表で各 SM_* の名前と十進の数値を併記する**（例: SM_MOUSEPRESENT 19、SM_MOUSEWHEELPRESENT 75、SM_CMONITORS 80、SM_SERVERR2 89、SM_REMOTESESSION 0x1000、SM_SHUTTINGDOWN 0x2000、SM_CONVERTIBLESLATEMODE 0x2003、SM_SYSTEMDOCKED 0x2004）。依頼文の「数値を書かないことが多い」は、このページには当たらない。**0x4b = 75 = SM_MOUSEWHEELPRESENT は公式（ページの表）で確認**。SM_MOUSEPRESENT: 「Nonzero if a mouse is installed; otherwise, 0. This value is rarely zero, because of support for virtual mice and because some systems detect the presence of the port instead of the presence of a mouse」。SM_MOUSEWHEELPRESENT: 「Nonzero if a mouse with a vertical scroll wheel is installed; otherwise 0」。SM_CXSCREEN／SM_CYSCREEN: 「the same value obtained by calling GetDeviceCaps as follows: GetDeviceCaps(hdcPrimaryMonitor, HORZRES／VERTRES)」。SM_CMONITORS: 「The number of display monitors on a desktop」（Remarks: 見える画面だけ数える）。SM_*VIRTUALSCREEN: 「the bounding rectangle of all display monitors」。SM_SERVERR2: 「The build number if the system is Windows Server 2003 R2; otherwise, 0」。戻り値: 成功で要求した値、失敗で 0 で「GetLastError does not provide extended error information」。「System metrics can vary from display to display」。「This API is not DPI aware」。**既定の寸法（スクロールバー、枠、アイコンなど）の値はページに無い**。最小の対応: Windows 2000。 |
| [The Virtual Screen](https://learn.microsoft.com/en-us/windows/win32/gdi/the-virtual-screen)（`updated_at` 2025-03-11） | 「The bounding rectangle of all the monitors is the virtual screen」。「The primary monitor contains the origin (0,0)」。 |
| [About Icons](https://learn.microsoft.com/en-us/windows/win32/menurc/about-icons)（`updated_at` 2026-09-21） | 「System icon sizes」の表: **SM_CXSMICON／SM_CYSMICON は 96 DPI で 16x16、SM_CXICON／SM_CYICON は 96 DPI で 32x32**（150% で 24 と 48、200% で 32 と 64）。「scale with the display DPI」。このページは **Windows 10 22H2 と限定していない**（DPI だけで書く）。 |
| [SystemParametersInfoW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow)（`updated_at` 2025-07-01） | SPI_GETWHEELSCROLLLINES = 0x0068（既定値 3）、SPI_GETWHEELSCROLLCHARS = 0x006C（既定値 3）、SPI_SETDOUBLECLKWIDTH／HEIGHT は GetSystemMetrics の SM_CXDOUBLECLK／SM_CYDOUBLECLK で取り出す、SPI_GETMOUSE = 0x0003（しきい値 2 つと加速）。表の全文は読んでいない（長いページの前半だけ）。 |

非公式の照合（参考。値の根拠にはしない）: mingw-w64（`/usr/share/mingw-w64/include/winuser.h`、Debian／Ubuntu の `mingw-w64-common 13.0.0-2ubuntu1`）の `SM_*` の定義と、Learn の表の 91 名の数値を比べた。**87 名が一致、不一致は 0、4 名（SM_CXFIXEDFRAME、SM_CYFIXEDFRAME、SM_CXSIZEFRAME、SM_CYSIZEFRAME）はその名前の定義が無かった**（別名で定義されているためと読める。確認していない）。実装が使う 14 個の数値は Learn の表で確認済みで、mingw の値は使っていない。

## 6. 実装（実測）

- `lift/runtime/fle_sysmetrics.h`、`fle_sysmetrics.c`（新規。`lift/out-sm/work/` で完成（clang `-Wall -Wextra -Werror` と emcc `-Werror` の両方で警告なし）させてから 1 回で `lift/runtime/` に置いた。置いた後にアイコンの 4 index を足すため `lift/runtime/` のファイルを編集ツールで直し、再度 clang と emcc で警告なしを確認した）。14 index に答え、残りのゲストが使う 22 index は名前付きで停止し、それ以外は番号で停止する。
- `lift/crates/fe-lift/src/imports_table.rs`: 末尾に 1 行（コメント付きで 2 行）追記（編集の直前に読み直した）。`lib.rs` は変更なし。
- `lift/crates/fe-lift/tests/gdc.rs`: `GetSystemMetrics` を「答えない」API の一覧から外した（コメント付き。0053 のテストは「GetSystemMetrics に行が無い」と主張していたので、この決定で正しくなくなった）。
- `lift/crates/fe-lift/tests/sysmetrics.rs`（新規、17 件）。

## 7. テスト（実測）

- `cargo test -p fe-lift --test sysmetrics`: **17 件すべて ok**（`lift/out-sm/test-sm.txt` は最初の実行（実装が 10 index の時点）。最終の件数は下の全体の実行に含まれる）。内訳: 行と宣言、Learn の数値、決めた値、Learn の表に重複が無いこと、14 の答え、0053 との整合（SM_CXSCREEN と GetDeviceCaps(HORZRES) の一致ほか）、`ecx` の下位 DWORD、状態を持たない・last error に触れない・戻り番地を壊さない、ゲストが使う index の名前付き停止（22 個）、それ以外の番号での停止（70 個余り）、環境状態の index（REMOTESESSION ほか）が答えられないこと、ソースがホストを読まないこと、ヘッダが plain C であること、生成器が thunk を応答に置き換えること、DLL 名が一致の一部であること、実 DLL のバイト列（54 か所の呼び出し点と直前の命令）、静的な index の全部（36 種）が「答える」か「名前付きで停止」のどちらかに入ること。
- 変異テスト（`lift/out-sm/work/mkmut2.py` が `lift/out-sm/mut/m01`〜`m22` に runtime の複製を作り、`FLE_SM_SRC_DIR` で指す。`runmut.sh`。`lift/out-sm/mut/summary.txt`）: **22 変異すべてがテストの失敗で検出された（生き残り 0）**。失敗件数は 1〜6 件。内訳: m01 SM_CXSCREEN が 1919（4 件）、m02 マウスなし（4）、m03 ホイールなし（3）、m04 モニタ 2（4）、m05 SM_CXVIRTUALSCREEN が 0（3）、m06 SM_CXVSCROLL を 17 で答える（3）、m07 未使用 index が黙って 0（2）、m08 `rcx` の全 64 ビットを見る（1）、m09 仮想画面の原点が 1（4）、m10 SM_SERVERR2 が 1（3）、m11 SM_CYSCREEN が 1080 の直書き（1）、m12 SM_CXSIZEFRAME を名前付き停止の表から外す（2）、m13 last error を 0 にする（1）、m14 SM_MOUSEPRESENT の番号が 18（6）、m15 SM_REMOTESESSION に答える（2）、m16 名前の取り違え（1）、m17 SM_CMONITORS が停止する（3）、m18 SM_CXSCREEN が高さ（4）、m19 SM_CXICON が 31（3）、m20 SM_CYSMICON が 32（3）、m21 SM_CXSMICON が SM_CXICON の値（2）、m22 SM_CYICON を落とす（2）。**最初の作り方の誤り**: m17 の最初の版は `(uint32_t)FLE_GDC_HORZRES > 1920 ? 2 : 1` で、結果が元と同じ（常に 1）になる同値変異だった。意味のある変異（`break` で落とす）に作り直して再実行した結果が上の数字。ほかに、m19 と m20 の最初の版はパターンの末尾の空白が実際と合わず生成に失敗した（アサーション）ので、作り直した（`mkmut2.py`）。コンパイルエラーで落ちた変異は 0（`grep "clang failed"` が空）。
- `cargo test --workspace --no-fail-fast`（`lift/out-sm/test-ws.txt`、shell のタイムアウト 3,600,000 ms、フォアグラウンド、cargo の終了コード 0）: awk の集計は `test result` 行 **75**、**731 passed、0 failed、0 ignored**。直前の基準は 74 行、714 件。差は +1 行、+17 件（新規 `sysmetrics.rs` の 17 件。`gdc.rs` は件数が変わらない）。

## 8. harness の再実行（実測）

生成 C は変わった（thunk が応答になり、応答表が 1 行増えた）ので `lift/out-sm/out` に新規生成した（`OUT=lift/out-sm bash lift/harness/wasm/build.sh gen split`）。`functions 59297`、`bad_nodes 0`、`unsupported_total` **547 → 546**（`jmp_imp:GetSystemMetrics 1`）。runtime 37 ファイル、生成 C の 17 パーツ、harness 3 ファイルがすべて `errors=0 warnings=0`（`build-wasm.txt` の 57 行）。ただし `fle_proc.c` に **既存の** 警告 1 件（`fle_thread_prio` の `-Wunused-but-set-global`。`out-tid` のログにも同じ警告がある）。`fle_sysmetrics.c` は 0 件。アイコンの 4 index を足した後は、`fle_sysmetrics.c` のオブジェクトを退避（`mv`）して runtime と link だけやり直し（`build-wasm2.txt`）、native は `build_native.sh` で同じ生成 C から再リンクした（`build-native2.txt`）。

| | native | wasm（node） |
|---|---|---|
| 停止 | `fle: unsupported instruction at 0x4255e0`（SIGABRT、`abort kind: fle_unsupported`） | 同じ |
| 関数実行数（distinct） | **574**（前 572） | 574 |
| 入口数（calls） | **13,233**（前 13,230） | 13,233 |
| 呼び出し深さ | 10 | 10 |
| unsupported_total | **546**（前 547） | 546 |

- `diff <(grep -v '^calling' run-native.txt) <(grep -v '^calling' run-node.txt)`: **差なし**。全体の diff は `calling entry … rsp=` の 1 行だけ（native `0x70121affffc0`、wasm `0x11c4f060`）。各 2 回実行（`run-native.txt`、`run-native2.txt`、`run-node.txt`、`run-node2.txt`）し、native 同士、wasm 同士が（rsp の行を除いて）一致。
- 0x4255e0 は `user32!SystemParametersInfoW` の thunk（実測: IAT）。呼び出しスタック（native と wasm で同じ）: `f_4255e0`（guest-return 0x6208b1）← `f_620880` ← `f_6209a0`（0x6209d9）← `f_620640` ← `f_620cd0` ← `f_623840` ← `f_4120c0` ← `f_412160` ← `f_41d8f0` ← エントリ。つまり **§3 の 5 の読みどおり**、`GetSystemMetrics(19)` → `GetSystemMetrics(75)` を通り、`SystemParametersInfoW(0x68, ...)` の 0x6208ac の呼び出しで止まる（`rcx = 0x68` かは gdb で確かめていない。guest-return が 0x6208b1 で、0x6208ac の `call` の直後であることと、`[rbx+0x28]` が真で 0x62089e 以降に入ったことは整合する）。
- 増分（関数 +2、入口 +3）は読みと整合する: 新しい関数は f_620880 と f_4255e0（f_424ee0 は前の停止点で数え済み）。入口は f_620880、その中の f_424ee0 の 2 回目、f_4255e0。`[obj+0x28]` の値（1）は gdb で読んでいない。

## 9. 未確認、限界

- 実 Windows との比較はしていない。GetSystemMetrics が実機で返す値は見ていない。
- 間接呼び出しの先（§2、§4）、動的な index の 4 か所のうち f_859470／f_859610 の b の範囲、`[obj+0x10]`（マウスの有無）の読み手、`[obj+0x14]`（スクロール行数）の読み手は追えていない。
- SM_CXVSCROLL、SM_CYHSCROLL など、ゲストが使う 22 の index の値は決めていない（一次資料に値が無い。決定 0057）。実行がそれらに届いたときに止まる。
- Windows 10 22H2 の標準値だと断言できるのは、アイコンの 4 値だけ（About Icons ページ。ただしページは OS を限定せず DPI だけで書く）。
- 0x4b と SM_MOUSEWHEELPRESENT の対応、14 index の数値は Learn の表で確認した。mingw の値との照合は参考。

## 10. 触ったファイルと制約

- 変更: `lift/crates/fe-lift/src/imports_table.rs`、`lift/crates/fe-lift/tests/gdc.rs`、`docs/decisions/0055-msctf-loadlibrary.md`（追記 1 行）、`docs/investigations/msctf-caller.md`（追記 2 行）。
- 新規: `lift/runtime/fle_sysmetrics.h`、`fle_sysmetrics.c`、`lift/crates/fe-lift/tests/sysmetrics.rs`、`docs/decisions/0057-virtual-system-metrics.md`、本書、`lift/out-sm/`（gitignore 済み）。
- `rm`、既存ファイルの `>` 上書き、git の変更操作、`/tmp`、コマンド文字列の `sleep`/`timeout`/`nohup` は使っていない。権限拒否はなし。`lift/iat-unsup-closure.txt` には触っていない。emcc は `JOBS=2` 以下。成果物の置き換えは `mv` で `lift/out-sm/old1`、`old2`、`log-old` に退避した（削除していない）。
