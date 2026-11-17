# 0106: ILTools_x64.dll を A4 の 6 モジュール構成（FLEngine + 実在 5）に載せ、unit 245 の `LoadLibraryW "iltools_x64.dll"` を ILTools の DllMain(ATTACH) の中まで通した（`RM_MAX` 4 → 8。壁 5 件の応答。ATTACH は未完了で、次の壁は GDI の DC／DIB の塊）

状態: **仮の決定**（番号 0106 は PM の仮の割り当て）。PM の指示（決定 0101 の判定構成に、決定 0104 で用意した ILTools を、FLEngine を載せたまま 5 個目の実在モジュールとして載せる作業）を、作業者（サブエージェント）が 2026-10-27 に調査・設計・実装・測定した。0046、0060、0082、0086、0100、0101、0104、0105 の本文は書き換えない。

**統合版（M2）**: 本書は I2 の作業者が書いた文書を、**統合した木**（HEAD 520064a の `git archive` に `wallI2.diff`（この決定）、続けて `wallR2.diff`（決定 0107）の順に当て、衝突を手で解いた木。差分は `lift/out-git/m2-results/resolved-I2R2.diff`）に合わせて、数値だけ直した版である（旧 → 新は `lift/out-git/m2-results/MERGE-NOTES.md`）。直した数値: `vcall_thunk_dll` の未対応 thunk 数（514 → 513）、FLEngine の生成 C の SHA-256 とバイト数と `unsupported_total`、`sysmetrics` の固定数、全体テストの結果。**測定（S0〜S6、対照）は HEAD + I2 の木での記録で、統合した木では再測定していない**（a4・census の実測は統合の作業の範囲外）。R2（決定 0107）の応答は、停止していた経路を答えに変えるだけなので、A4 の 6 モジュール構成が unit 245 の `LoadLibraryW` まで通った経路は動かないと**読む**（推論。測っていない）。S6 より先、特に ILTools の DllMain の中で R2 の応答（GetSystemMetrics の 86／95／0x2003／0x2004、`CompareStringW`、`SHGetFolderPathW`）に届くかは未測定。

> **これは A4 の達成ではない。** 6 モジュール構成でも FLEngine の **init_count は 246 / 942 のまま**（5 モジュール構成と同じ。unit 245 の `LoadLibraryW` が戻らないため）。動いたのは**停止点**（C スタックの先頭が FLEngine の `f_423990` から ILTools の中に移った）と、**ILTools 自身の InitUnits の進み**（**221 / 283 unit**。測定は gdb で ILTools の InitContext を読んだ。下の「測定」）である。この 2 つを混同しない。unit 245 の `LoadLibraryW` は成功も失敗もしていない（戻っていない）。

記入状況: 記入済み。測定は native のみ（emcc は使っていない。wasm は未確認）。各構成は `a4-multi.sh` の複製（下の「測定の道具」）でビルドし、**最終の 6 モジュール構成と 5 モジュールの対照は 2 回、途中の段階は多くが 1 回**（下の表の「回数」）。数値の出典は `lift/out-git/i2-results/`（テキストだけ）。統合した木の数値（全体テスト、生成 C）の出典は `lift/out-git/m2-results/`。

事実の出典（読んだもの）:

- 決定 0104（ILTools の基底 0x6000000、種 400 個、ATTACH の壁 7 件、8 件目 `0x605ddaa jmp [rax+0x10]` で停止）、0100（壁 L: `jmp [mem]` の tail dispatch と仮想呼び出し thunk の種）、0101（A4 の判定は実在モジュール構成）、0103（QFC／freetype の実在モジュール化）、0105（インライン フック）、0082（実在モジュールのロードの意味）、0054／0058／0057／0075／0009（GDI、SPI、システムメトリクス、legacy SSE、CPUID）。`docs/investigations/iltools-attach-measurement.md`、`bundled-dll-inventory.md`。
- **Microsoft Learn（2026-10-27 に取得して読んだ）**: [About Cursors](https://learn.microsoft.com/en-us/windows/win32/menurc/about-cursors)（`updated_at` 2026-09-21。「Cursor Sizes」の表）、[SystemParametersInfoW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow)（SPI_GETCURSORSHADOW 0x101A、SPI_SETCURSORSHADOW 0x101B の記述）、[CreateBitmap](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-createbitmap)（ページの末尾に「Last updated on 2022-12-06」）。
- 命令の仕様: Intel SDM の HADDPD（felixcloutier.com の転載。「No longer updated」の注記つき、2023-12 版）。**実測の oracle は実行ホスト（AMD Ryzen 9 9950X。CPUID の flags に `pni`）で、同じ機械語を実行して比較した**。
- 読み（逆アセンブル）: capstone と pefile による ILTools のリベース済み像（SHA-256 `1ca9b793…d17f04e`、`fe-pe rebase` の往復は決定 0104 で原本と一致）。gdb は実行中の native ハーネス（`R` のレジスタ配列とゲストのメモリを読むだけ。値は書いていない）。

## 背景（事実）

- HEAD `520064a`（決定 0105 まで）の 5 モジュール構成（FLEngine + dsp_ippv2 + Blend2D + QFC + freetype。実在 4）は **246 / 942** で、停止は unit 245 の `LoadLibraryW "iltools_x64.dll"`（C スタック `f_423990 ← f_44db40 ← f_6539e0 ← f_815e10 ← f_814e60 ← f_812e80 ← f_732f30 ← f_733090`）。**この作業の木（下の差分を当てたもの）で同じ構成を再測定した**（対照。functions 2,148、entries 94,011、call depth 17、2 回とも同一）。決定 0105 の記録（functions 2,148、entries 94,011）と一致する。
- 実在モジュールの上限 `RM_MAX` は 4 で、上の 5 モジュールのうち FLEngine 以外の 4 つで埋まっていた。ILTools は 5 つ目。
- ILTools は unit 245、280、659 で `LoadLibraryW` される（245 が最初。`loadlibrary-names-classification.md`）。

## 決定（作業者。仮）

| # | 項目 | 決定 |
|---|---|---|
| 1 | `RM_MAX` | **4 → 8**（`lift/runtime/fle_realmod.c`）。理由: 必要なのは 5 つ（この構成）で、同梱 DLL の名前は 14 あり（`bundled-dll-inventory.md`）、ILTools の `LoadLibrary` の対象になるのは dsp／Blend2D／QFC（すでに実在）と `ilvideo_x64`（手元に無い）など。**8 は事実ではなく選択**（配列は `rm_t` 約 40 バイト × 8 = 320 バイトの静的領域だけで、wasm の locals や静的な表の数は変わらない）。上げ直すときは、下の「洗い出した箇所」の一覧が同じ作業になる |
| 2 | 6 モジュール構成の測定用の値（**測定用。決定ではない**） | FLEngine（0x400000、SizeOfImage 0x18c0000）+ dsp_ippv2（0x2000000、種 452）+ Blend2D（0x4700000、`--pdata`、種 58）+ QFC（0x1d00000、`--pdata`、フック点 `qfc-hook-points.txt`）+ freetype（0x4b00000、`--pdata`、種 160）+ **ILTools（0x6000000、`--pdata`、種 400 個 `iltools-6000000-seeds400.txt`、フック点 `iltools-hook-points.txt`）**。FLEngine のフック点は `flengine-hook-points.txt`（0105）。表の順序（= 実在モジュールの index）は上の順。ILTools は index 4 |
| 3 | ILTools のインライン フック点（決定 0105 の機構の 3 つ目の適用。壁 W1） | `lift/harness/hooks/iltools-hook-points.txt`: **RVA 0x178d0、0x31e70、0x31e90**（ファイル自身の ImageBase 0x400000 では 0x4178d0、0x431e70、0x431e90。基底 0x6000000 では 0x60178d0、0x6031e70、0x6031e90）。置き換え先は 0x61b8f90、0x61b8e80、0x61b8ef0（基底 0x6000000）。先頭 5 バイトは FLEngine／QFC の 3 点と同じ（`55 56 53 c6 44`／`53 48 83 ec 20`／`55 48 83 ec 50`）。**生成器・runtime は変更なし**（決定 0105 のとおり） |
| 4 | `GetSystemMetrics(SM_CXCURSOR = 13)`、`(SM_CYCURSOR = 14)` = **32、32**（**仮**。壁 W2） | Learn「About Cursors」「Cursor Sizes」: 「The standard sizes at default cursor size setting」は表示倍率 < 150%（< 144 DPI）で 32x32。仮想画面は 96 DPI（決定 0053）。ページは OS を限定せず、「default cursor size setting」がこの環境で何かも書かない。ILTools の初期化（`CreateCursorFromFont`）が最初に 13 を問い、直後の命令が 14 を問うので、2 つを同時に答えた（14 は単独の停止としては測っていない） |
| 5 | `SystemParametersInfoW(SPI_GETCURSORSHADOW = 0x101A)` = **FALSE**（**仮。値に出典なし**。壁 W3） | Learn: GET と SET（0x101B）のどちらにも既定値の記述がない（「Determines whether the cursor has a shadow around it ... receives TRUE if the shadow is enabled, FALSE if it is disabled. This effect appears only if the system has a color depth of more than 256 colors」）。**作業者の選択**で FALSE（決定 0103 の SPI_GETFONTSMOOTHINGORIENTATION と同じ型の「出典のない選択」）。実機の既定値は確認していない。ゲストは 0 のときシャドウを描く分岐（`cmp [rbp+0x1c8], 0` の 0x625f854 の先）を通らない。**ゲストの引数の形（uiParam 0、pvParam 非 NULL、fWinIni 0）以外は名前付き停止**（他の GET と同じ）。0x101B は答えない |
| 6 | `haddpd`（SSE3、66 0F 7C /r）を lift（壁 W4） | `sse_fp_flags.rs` に `Haddpd => op2(i, "haddpd", "fle_mx_haddpd", P128)`、`fle_mxcsr_flags.h` に `fle_mx_haddpd`（下位 = d[63:0] + d[127:64]、上位 = s[63:0] + s[127:64]。各和は ADDPD の 1 レーンと同じ関数 `fle_mx_arith`、MXCSR のフラグは 1 回で反映）。**他の SSE3（HADDPS、HSUB*、LDDQU、MOVDDUP、ADDSUB*、MOVSHDUP、MOVSLDUP）は実装しない**。**CPUID は変えない**（葉 1 の ECX = 0 のまま。決定 0009、0075）。観測: CPUID が SSE3 を名乗らないのに、dsp_ippv2 の `ippsSum_32f` の経路が `haddpd` に届いた（下の「解釈が分かれた点」の 2） |
| 7 | `gdi32!CreateBitmap`（**仮**。壁 W5） | `fle_gdi.c`（`fle_imp_CreateBitmap`）、種類 `FLE_GDI_KIND_BITMAP` を表に足す。答えるのはゲストの形: **nPlanes 1、nBitCount 1、lpBits NULL** で、nWidth と nHeight が負でなく、計算サイズ cj = (((w·planes·bpp + 15) >> 4) << 1)·h が 2^31 未満。**幅または高さが 0 のときは、ページの「1-by-1 pixel, monochrome bitmap」を作る**（実測: ゲストの最初の呼び出しは 0 と 0）。表が持つのは幅・高さ・planes・bpp だけで**画素は持たない**（ページ: lpBits が NULL なら内容は未定義）。ハンドルは新しいスロット（パレット、フォントと同じ列。選択）。`DeleteObject` は bitmap を削除できる（DC への選択は存在しないので「選択中」にならない）。`GetObjectW` は bitmap で名前付き停止。負の値、他の形式、非 NULL の lpBits、cj ≥ 2^31 は名前付き停止。応答表の行を**末尾に 1 行**足した（`imports_table.rs`。R2（決定 0107）の `shfolder.dll!SHGetFolderPathW` の行も末尾に足すので、統合では **I2 の `CreateBitmap` が先、R2 の shfolder が後**。疑似 VA は `CreateBitmap` が 0x7ff900001370、shfolder が 0x7ff900001380。既存の行の疑似 VA は動かない（決定 0015）） |
| 8 | 直さなかった壁（W6） | `unsupported instruction at 0x601d6d0` = **`gdi32!CreateCompatibleDC` の IAT thunk**（ILTools の `f_6130190` → `f_61311b0` → `f_61374a0`。VCL の `TBitmap.Canvas` の `Draw`）。この先は、読み（`py/reach_imports.py`、`f_625ee30` から直接 call だけを 8 段辿った上限）で **行の無い import が約 20**（CreateCompatibleDC、CreateCompatibleBitmap、CreateDIBSection、SelectObject、GetDIBits、GetDIBColorTable、SetDIBColorTable、SelectPalette、RealizePalette、BitBlt、PatBlt、FillRect、SetTextColor、SetBkColor、DeleteDC、CreateBrushIndirect、GetSysColor、GetPaletteEntries、CreateHalftonePalette、CreateIconIndirect、FormatMessageW）。DC、DIB（画素のメモリ）、ブリットの**モデルの設計**が要る塊で、1 件ずつの応答では扱えないので、設計せずに止めた |
| 9 | 作らないもの | 新しい .c／.h（runtime にも生成器にも作っていない）。CPUID の SSE3 の主張。GDI の DC・画素・ブリットのモデル。ILTools の種 400 個の見直し。census の実行（PM が統合後に行う）。`imports_table.rs` の途中への挿入（末尾追記のみ） |

## 測定

**この節の測定は HEAD + I2 の木で行った記録**（R2 を含まない）。統合した木では測っていない。

構成は上の表 2 の 6 モジュール。基底はどれも重ならない（算術。各 SizeOfImage は PE ヘッダから読んだ）:

| モジュール | 範囲 | 次との隙間 |
|---|---|---|
| FLEngine | 0x400000〜0x1cc0000（0x18c0000） | 0x40000 |
| QFC | 0x1d00000〜0x1f0e000（0x20e000） | 0xf2000 |
| dsp_ippv2 | 0x2000000〜0x461f000（0x261f000） | 0xe1000 |
| Blend2D | 0x4700000〜0x4a00000（0x300000） | 0x100000 |
| freetype | 0x4b00000〜0x4b9a000（0x9a000） | 0x1466000 |
| ILTools | 0x6000000〜0x6394000（0x394000） | — |

実測（`a4-multi.sh` の複製 `a4-multi-trim.sh`。ビルドは 1 構成 約 1 分 + lift 約 15〜65 秒）。FLEngine の **init_count は全段階で 246 / 942**（last unit index 245、proc 0x733090、started 209／completed 208／in_progress 1／895）:

| 段階 | 変更 | 停止（メッセージの要点） | functions | entries | call depth | 回数 |
|---|---|---|--:|--:|--:|---|
| S0 | HEAD ＋ `RM_MAX` 8（ILTools の註釈なし） | `VirtualProtect: address is not in a VirtualAlloc region`。**ILTools の中**（`f_601d510` ← `f_61bc3a0` ← `f_61bc220` ← `f_61bd9e0` ← … ← DllMain `f_62b35f0`）。**決定 0104 の 8 件目（`0x605ddaa`）は通った** | 2,966 | 115,333 | 26 | 2 回、同一 |
| S1 | + ILTools のフック点（W1） | `GetSystemMetrics: index 13 (SM_CXCURSOR) is used by the guest but no value is decided (decision 0057)` | 3,276 | 133,190 | 29 | 1 回 |
| S2 | + SM 13／14（W2） | `SystemParametersInfoW: uiAction 0x101a (SPI_GETCURSORSHADOW) is used by the guest but no value is decided (decision 0058)` | 3,276 | 133,333 | 29 | 1 回（runtime だけの変更なので S1 のビルドの runtime を作り直して再リンク。コンソールの記録のみ） |
| S3 | + SPI 0x101A（W3） | `unsupported instruction at 0x3a949d1`（dsp_ippv2 の `haddpd xmm4, xmm4`。`FontCache_AcquireFont` ← … ← `ippsSum_32f` ← `f_2cb45a0` ← `f_2db08e0` ← `f_3a94900`） | 3,338 | 133,694 | 38 | 1 回（再リンク。コンソールの記録のみ） |
| S4 | + HADDPD（W4。fe-lift の変更なのでフルビルド） | `unsupported instruction at 0x601d6a0`（`gdi32!CreateBitmap` の IAT thunk） | 3,344 | 133,777 | 30 | 1 回 |
| S5 | + `CreateBitmap` の行（ゼロの規則なし） | `CreateBitmap: nWidth or nHeight is not above zero ...`（引数は w = h = 0。gdb で `rcx = rdx = 0`、`r8 = r9 = 1`、`[rsp+0x28] = 0` を読んだ） | 3,344 | 133,777 | 30 | 1 回 |
| **S6（最終）** | + ゼロ → 1×1（W5） | **`fle: unsupported instruction at 0x601d6d0`**（`gdi32!CreateCompatibleDC` の IAT thunk。abort kind `fle_unsupported`） | **3,373** | **133,960** | **33** | **2 回、`calling entry … rsp=` の 1 行を除いて同一**（stderr も同一） |
| 対照 | 5 モジュール（ILTools なし）、同じ木 | `LoadLibraryW: module "iltools_x64.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)` | 2,148 | 94,011 | 17 | 2 回、同一 |

**最終（S6）の停止メッセージ全文**: `fle: unsupported instruction at 0x601d6d0`。

**最終の C スタック**（`gdb` の `bt` から `f_*` のフレームだけ。29 フレーム。ハーネスの出力は先頭の 10 フレームまで）: `f_601d6d0 ← f_61374a0 ← f_61311b0 ← f_6130190 ← f_625ee30 ← f_625ef30 ← f_6262b60（export CreateCursorFromFont）← f_625ed40 ← f_625cfc0 ← f_622aa30 ← f_622ab90 ← f_600da60 ← f_600db00 ← f_60186d0 ← f_62b3610 ← f_62b35f0（ILTools の DllMain）← f_423990 ← f_44db40 ← f_6539e0 ← f_815e10 ← f_814e60 ← f_812e80 ← f_732f30 ← f_733090 ← f_4120c0 ← f_412160 ← f_41d8f0 ← f_1376840 ← f_1376820`。

**ILTools 自身の InitUnits（実測、S6 の停止時に gdb で読んだ）**: ILTools の InitContext は基底 0x6000000 の像の 0x6306c90（ファイルの VA 0x706c90 の位置。`f_40da60` が `[0x706c90]` の表と `[0x706c98]` の InitCount を使う読みに基づく）で、**header 0x62b3680（`fe-pe pe-init-table` の header 0x6b3680 と対応）、unit_count 283、InitCount = 221**。止まっている unit は index 220（proc 0x82ab90 = 基底 0x6000000 では 0x622ab90。runnable の 188 番目 / 231）。**ILTools の InitUnits は 221 / 283 まで進んでいる**（FLEngine の init_count とは別の数。A4 の判定には使わない）。

**ILTools の DllMain(ATTACH) の中で起きたローダー呼び出し（実測、gdb でブレークポイントの引数を読んだ。`e-6mod-final/loadlib-trace.txt`）**:

- `LoadLibraryW "dsp_ippv2_x64.dll"`、`"Blend2D.dll"`、`"QuickFontCache_x64.dll"` が ILTools の DllMain の中から呼ばれ、**既にロード済みの実在モジュール 3 つに参照が増えて戻った**（入れ子のロード。DllMain は再び入らない）。
- ILTools の DllMain の中の `GetProcAddress` は 522 回（ILTools のロード以降）: dsp_ippv2（0x2000000）に 423、QFC（0x1d00000）に 5（`FontCache_AcquireFont`、`FontCache_CleanUp`、`FontCache_GetStats`、`AcquireSharedThreadBank`、`AcquireSharedAnimTimer`）、Blend2D（0x4700000）に 1（`AggCreateInstance`）、残りは仮想モジュール。
- ILTools の `FreeLibrary` は S6 の停止までに呼ばれていない。

**ILTools のフック（実測、S6、gdb のブレークポイントの計数）**: `fle_hook_target` が合計 141 回（FLEngine、QFC、ILTools のフック点の入口の合計。モジュール別には数えていない）、`VirtualProtect` が 12 回（QFC の 6 回 + ILTools の 6 回。3 点 × 準備と有効化。すべて成功）、**置き換え先 `f_61b8f90`（フック点 0x60178d0 の置き換え）が 6 回実行された**。残りの置き換え先 `f_61b8e80`、`f_61b8ef0` は 0 回。決定 0105 の記録は「置き換え先は A4 の経路で一度も実行されていない」だったので、**ゲストのフック層の置き換え先が実行されたのはこれが初めて**。停止も誤った結果も出なかったが、**置き換え先の結果が正しいか（ゲストの意図した動作か）は確認していない**。

## 壁の表

| # | 停止 | 原因 | 応答 | 区分 |
|--:|---|---|---|---|
| W0 | `0x605ddaa jmp [rax+0x10]`（決定 0104 の 8 件目） | Delphi のインターフェース thunk | **この作業では何もしていない**。6 モジュール構成で通った（S0。実測）。通った理由は、決定 0100 の `jmp [mem]` の tail dispatch と仮想呼び出し thunk の規則だと**読む**（0100 の決定 3 の形そのもの）。0100 を外したビルドとの比較はしていない | 実測（通過）、原因は読み |
| W1 | `VirtualProtect: address is not in a VirtualAlloc region`（S0） | ILTools の unit（`f_61bd9e0`）が決定 0105 と同じ Delphi のフック クラスで 3 つの関数の先頭 5 バイトを書き換える | フック点 3 つを註釈（表 3） | 停止は実測。フック点の特定は読み（`lea r8`／`lea r9` の 3 組、`f_61bc220` の呼び出し元が 3 か所だけであること（`E8`／`E9` rel32 の線形走査）、DIR64 の再配置にアドレスが無いこと）。実行後の動作は実測（上） |
| W2 | `GetSystemMetrics: index 13 (SM_CXCURSOR)` | ILTools 自身の初期化が export の `CreateCursorFromFont`（`f_6262b60`、引数は文字 'C'＝0x43、0xc8、-1。gdb で読んだ）を呼び、`f_625ef30` が 13 と 14 を問う | 32、32（表 4。**仮**） | 停止は実測。値は Learn の表から導出（OS の限定なし） |
| W3 | `SystemParametersInfoW: uiAction 0x101a` | 同じ関数が `SPI_GETCURSORSHADOW` を問う（BOOL を `[rbp+0x1c8]` に受ける） | FALSE（表 5。**仮、出典なし**） | 停止は実測。値は選択 |
| W4 | `unsupported instruction at 0x3a949d1` | dsp_ippv2 の `ippsSum_32f` の経路の `haddpd`（SSE3）。lift が未対応 | HADDPD を lift（表 6） | 停止は実測。意味は SDM（転載）とホストの CPU で確認（テスト） |
| W5 | `unsupported instruction at 0x601d6a0` → `CreateBitmap: nWidth or nHeight is not above zero` | `f_625ee30`（ビットマップから cursor を作る関数）の `CreateBitmap(w, h, 1, 1, NULL)`。**w = h = 0**（理由は下の「解釈が分かれた点」の 1） | 行を足し、ゼロを 1×1 にする（表 7。**仮**） | 停止は実測。ゼロの扱いは Learn |
| W6 | `unsupported instruction at 0x601d6d0` | `gdi32!CreateCompatibleDC` | **直さない**（表 8） | 実測（停止）、先の API の一覧は読み（上限） |

## 範囲外

- W6 より先（GDI の DC／DIB／ブリットの設計、`CreateIconIndirect` ほか、ILTools の InitUnits の 221 より先の 62 unit、ATTACH の完了、unit 280／659 での 2 回目以降の `LoadLibraryW`／`GetProcAddress("CreateCursorFromFont")`）。
- `FreeLibrary` と DllMain(DETACH)（決定 0082 D11 のまま）。ILTools の `FreeLibrary` は測定した経路に出ていない。
- census（PM が統合後に行う）。wasm（`emcc` は使っていない）。実機 Windows との照合（0 件）。
- ILTools の種 400 個（決定 0104 P2）を、決定 0100 の thunk の種で置き換えられるかの確認。
- 空のグリフ範囲（解釈が分かれた点の 1）の原因の調査。

## 撤回条件（仮の決定）

| # | 仮の決定 | 撤回条件 |
|---|---|---|
| P1 | `RM_MAX` = 8（表 1） | 9 個以上の実在モジュールが要るとき（定数と下の一覧の箇所を更新）。選択であって事実ではない |
| P2 | ILTools のフック点 3 つ（表 3） | ILTools が他の関数にも `VirtualProtect` でフックを書くと分かったとき（実行が名前付きで知らせる）。ILTools の版が変わったとき。置き換え先の動作が誤りと分かったとき。決定 0105 の「旧保護の値は仮」も引き継ぐ |
| P3 | SM_CXCURSOR／SM_CYCURSOR = 32、32（表 4） | 実機の Windows 10 22H2 の値が示されて食い違ったとき。仮想画面の DPI（0053）が 144 以上に変わったとき。ページの表が Windows 10 に当てはまらないと分かったとき |
| P4 | SPI_GETCURSORSHADOW = FALSE（表 5） | 一次資料（既定値の記述）が見つかったとき。ゲストが TRUE の経路（シャドウの描画）を要するとき。実機の値が示されたとき |
| P5 | HADDPD の NaN の演算順（表 6、下の 2） | Intel の機械での結果が示されて食い違ったとき。CPUID が SSE3 を名乗ることになったとき |
| P6 | CreateBitmap の表（画素を持たない）と、ゼロ → 1×1（表 7） | 画素を読む API（`GetBitmapBits`、`SelectObject`＋`BitBlt`）が実装されるとき（画素のモデルが要る）。ゼロ幅の原因が環境の欠落と分かったとき（その欠落を直す方が先で、この規則は残してよいが、ゼロに届かなくなる） |

## 洗い出した箇所（`RM_MAX` の引き上げ）

`grep -rn "RM_MAX\|MAIN_REAL_MAX\|at most 4\|max 4\|4 real"` の全件と、実在モジュールの個数に結びつく配列を読んだ結果（ファイルは `lift/` の下）:

| 箇所 | 内容 | 扱い |
|---|---|---|
| `runtime/fle_realmod.c` | `#define RM_MAX 4u`（配列 `mods[RM_MAX]`、`table_check` の個数検査、停止メッセージの「4 real modules」） | **8 に変更**（メッセージも） |
| `runtime/fle_realmod.h` | 表の規則のコメント「at most 4 modules」 | コメントを更新 |
| `runtime/fle_loadimage.h` | カーソルのハンドル `BASE + 0x1000 × (index + 1) + 4 × (id − 32762 + 1)`、コメント「(at most 4 modules)」 | 計算: 最大 index 7 で `BASE + 0x8018`。領域 1 MiB の内側。コメントを更新（式は変更なし） |
| `harness/census/main_two.c` | `MAIN_REAL_MAX = 4`（コメントは「RM_MAX」と同じと書く）。5 個目の実在モジュールのファイルは**ハーネスが runtime より先に拒否していた** | **16 に変更**（`harness/two/main_two.c` と同じ方針: runtime の名前付き停止に任せる） |
| `harness/census/census.c` | `MAX_IMAGES 8`。`census_add_image` は超えた像を**黙って捨てる**（FLEngine + 実在 8 = 9 が要る） | **9 に変更** |
| `harness/two/main_two.c` | `MAIN_REAL_MAX = 16`、コメントの「RM_MAX (4)」 | コメントだけ「(8)」に更新 |
| `harness/a4-multi.sh` | コメントの「RM_MAX」（数字なし）。モジュール数の固定なし | コメントに「8 since decision 0106」 |
| `crates/fe-lift/tests/realmod.rs` | `a_table_that_breaks_the_rules_stops_by_name` の `five-modules`（5 エントリ → 「larger than the 4 real modules」） | **`nine-modules`（9 エントリ → 「larger than the 8 real modules」）に更新**。**新規** `a_table_of_eight_modules_passes_the_table_check` |
| 変更なし（確認のみ） | `runtime/fle_vmap.c` の `IH_MAX_MODS 8`（フックを登録する像の数。この構成では FLEngine、QFC、ILTools の 3）。`fle_realmod.c` の他の関数は `mods[idx]` だけを使い、index に依存する分岐なし。`fle_res.c` は `fle_realmod_base_of(k)` を 0 が返るまで回す。`gen_two.sh`、`build_two_native.sh`、`merge_tables_n.py`、`two-native-run.sh` に個数の固定なし。`build_two_wasm.sh` と `realmod_table.c` は 1 モジュール用（wasm は未ビルド） | — |

**5 個目の実在モジュールの経路（読みと実測）**: `fle_realmod_load` は index で `mods[]` を引くだけで、ロード順は表の順ではなく**最初の `LoadLibrary` の順**（実測: ILTools の DllMain の中から dsp（index 0）、Blend2D（1）、QFC（2）を `LoadLibraryW` し、参照が増えて戻った）。DllMain(ATTACH) は `fle_call_guest` で入る（実測: ILTools の DllMain が実行され、InitUnits を 221 unit まで進めた）。参照カウント（ST_LOADING 中の同名ロードは 1 つ増やして DllMain に入らない、ST_LOADED は 1 つ増やす）と `FreeLibrary`（0 になる呼び出しは名前付き停止）は index に依存しない（読み）。**`FreeLibrary` と DllMain(DETACH) は ILTools では実行されていない**（未確認）。

## 実装した場所

差分は `lift/out-git/i2-results/wallI2.diff`（HEAD の `git archive` との `diff -ruN`。**24 ファイル**（I2 の原文は 23 と書いたが、`wallI2.diff` の `+++` 行は 24）。`patch -p1` で HEAD の木に当たることを確認した）。**統合した 1 本（I2 + R2 + 統合の修正）は `lift/out-git/m2-results/resolved-I2R2.diff`**:

- runtime: `fle_realmod.c`、`fle_realmod.h`、`fle_loadimage.h`（`RM_MAX`）、`fle_sysmetrics.c`／`.h`（SM 13／14）、`fle_spi.c`／`.h`（0x101A）、`fle_gdi.c`／`.h`（`CreateBitmap`、種類 bitmap、`DeleteObject`、`GetObjectW` の停止、検査用 `fle_gdi_bitmap_live_count`／`fle_gdi_bitmap_info`）、`fle_mxcsr_flags.h`（`fle_mx_haddpd`）。**新しい .c／.h は作っていない**。
- 生成器: `crates/fe-lift/src/sse_fp_flags.rs`（`Haddpd` の 1 行とコメント）、`imports_table.rs`（末尾に `gdi32!CreateBitmap` の 1 行。統合では、この後ろに R2 の `shfolder.dll!SHGetFolderPathW` の 1 行）。
- harness: `harness/hooks/iltools-hook-points.txt`（新規）、`harness/census/main_two.c`、`harness/census/census.c`、`harness/two/main_two.c`、`harness/a4-multi.sh`（コメントと、既定の `EXPECT_FLE_SHA` を **`a5832f3b38a24b87c22834d504935176f2318f3dba1ac8db804ecba0074671d2`** に変更。理由: `CreateBitmap` の行で FLEngine の C が変わった（215,747,179 → 215,747,384 バイト、`unsupported_total` 515 → 514。FLEngine と QFC は `CreateBitmap` を import する）。`a5832f3b…` は `a4-multi.sh` が比べる種類の値（`fe-lift lift FLEngine_x64.dll OUT.c --pdata` が OUT.c に書いたファイルの SHA-256）で、**HEAD + I2 の木での値**である。**統合した木（I2 + R2）では `794d26cb38f6aad5c92390f8fe1bcaa740d7c57f28840382cd4d56898cb0bcdd`（215,747,616 バイト、`unsupported_total` 513）で、`a4-multi.sh` の既定値はこれに更新した**）。
- テスト: `tests/realmod.rs`、`sysmetrics.rs`、`spi.rs`、`gdi.rs`、`mxcsr_flags.rs`、`hook_points.rs`、`vcall_thunk_dll.rs`（期待値 515 → 514。統合では R2 の shfolder の thunk も答えになり **513**）。統合では `sysmetrics.rs` の `ANSWERED`／`UNDECIDED` が R2 の 4 index と合わさり 21／15 になる（下のテスト）。

## テスト

I2 の作業（HEAD + I2 の木）で実行したのは次の 18 バイナリ（`cargo test --offline -p fe-lift --test <名前>`。`CARGO_TARGET_DIR=/tmp/opencode/i2/target`、`CARGO_PROFILE_DEV_DEBUG=0`、`CARGO_INCREMENTAL=0`）。**I2 の作業では全体テスト（`cargo test --workspace`）は走らせていない**（統合 M2 で行った。表の後の「統合した木の全体テスト」）:

| バイナリ | 結果 | 変更 |
|---|---|---|
| `realmod` | 16 件通過 | `five-modules` → `nine-modules`、新規 1 件（8 エントリの表が通る） |
| `sysmetrics` | 17 件通過 | `ANSWERED` に (13, 32)、(14, 32)、`UNDECIDED` から 13、14 を除く。定数と番号のテスト |
| `spi` | 33 件通過 | `LISTED` から 0x101A を除く。新規 1 件（0x101A の値と引数の形の停止）。`fle_st(` の数を 12 → 13 |
| `gdi` | 55 件通過 | 新規 6 件（ハンドル、ゼロ → 1×1、引数の下位 DWORD と cj の上限、`DeleteObject`、非対応の形の停止、`GetObjectW` の停止）。`ROWS` に `CreateBitmap`。スレッド検査に 1 行 |
| `mxcsr_flags` | 11 件通過 | 新規 1 件 `haddpd_matches_host`（4 形式 × 4,000 件 = **16,000 件**をホストの CPU と bit 単位で比較。XMM、GPR、触れてはいけないレジスタとメモリ、MXCSR。RC、FTZ、DAZ、sticky フラグはランダム。0 件の不一致。うち 12,059 件が新しいフラグを立てた） |
| `hook_points` | 17 件通過 | 新規 1 件（ILTools の註釈ファイルと実 DLL。`ILTOOLS_DLL` が無ければ skip） |
| `vcall_thunk_dll` | 2 件通過 | 515 → 514（HEAD + I2 の木。統合した木では 513） |
| `imports_table`、`wall_r`（31）、`census`（9）、`modules`（16）、`deferred`（15）、`wasm_port`（6）、`wall_s`（15）、`wall_q`（5）、`wall_q_lift`（3）、`vex`（13）、`cpuid`（2） | すべて通過 | 変更なし（影響の確認のため実行） |

### 統合した木の全体テスト（M2）

HEAD 520064a に I2、R2 の順に当てた木（`resolved-I2R2.diff`。fe-decode `jumptab` の期待値 2 か所の更新を含む）を dev プロファイルでコンパイルし（警告なし）、RAM 上でテストした。`cargo test --workspace` は RAM（tmpfs の quota）に収まらないので、パッケージとテスト binary のグループに分け、各グループの結果を保存した（`lift/out-git/m2-results/logs/`）。**合算（グループ分けの合算。1 回の通しではない）: 112 test result、1,428 passed、0 failed、0 ignored**（fe-pe 83、fe-decode 140、fe-lift 1,205。doc テスト 3 つを含む）。HEAD の 1,401 に対して +27 は、I2 の新規 10 件（realmod 1、spi 1、gdi 6、mxcsr_flags 1、hook_points 1）と R2 の新規 17 件（wall_r2）に一致する。上の変更したテストは、統合した木でも同じ件数で通った（realmod 16、sysmetrics 17、spi 33、gdi 55、mxcsr_flags 11、hook_points 17、vcall_thunk_dll 2）。`sysmetrics` の固定数は `ANSWERED` 21、`UNDECIDED` 15（合計 36 = ゲストが使う index の数。I2 の 13、14 と R2 の 86、95、0x2003、0x2004 が答えになった分）。`FLENGINE_PE`、`FLENGINE_DLL`、`ILTOOLS_DLL` を実ファイルに向けて実行したので、変更したテスト（`hook_points` の ILTools の検査を含む）は skip されていない（`--nocapture` で確認）。

**変異検査**: HADDPD の runtime の関数に 4 つ（第 1 の和の演算順を入れ替える、上位の結果を s でなく d から作る、フラグを反映しない、第 1 の和を既定の MXCSR（DAZ／FTZ なし）で計算する）、生成器の写像に 2 つ（ADDPD の関数に差し替える、メモリ オペランドを 8 バイトで読む）を入れ、**6 つとも `haddpd_matches_host` が失敗**した（元に戻して `cmp` で一致を確認）。`CreateBitmap`、SM、SPI、`RM_MAX` の変更には変異検査をしていない。

## 検証

- **生成 C**: FLEngine の註釈なしの C のファイル SHA-256 は、HEAD `8789c1a5…`（215,747,179 バイト、`unsupported_total` 515）→ **HEAD + I2 の木 `a5832f3b…74671d2`（215,747,384 バイト、514。差は `CreateBitmap` の thunk 1 つ）**→ **統合した木（I2 + R2）`794d26cb38f6aad5c92390f8fe1bcaa740d7c57f28840382cd4d56898cb0bcdd`（215,747,616 バイト、513）**。統合した木の C を HEAD の C と関数単位で比べると（59,380 関数のハッシュ表）、本体が変わった関数は `f_423f30`（`CreateBitmap` の thunk。この決定）と `f_4262e0`（R2 の shfolder の thunk）の 2 つだけで、ほかは同一。そのほかの差は `fle_imp_CreateBitmap` の宣言 1 行、`fle_imp_SHGetFolderPathW` の宣言 1 行、`fle_dispatch_table` と `fle_resp_table` の 2 行ずつ、2 つの個数（59,690 → 59,692、310 → 312）。HEAD の C は同じ手順で 8789c1a5… を再現した。統合した木の FLEngine の註釈あり（`--hook-points lift/harness/hooks/flengine-hook-points.txt`）の C は `6732af99fc6c7b9e7de384e8baffc9eeb4031bbe20ee7886d6c2baf48ed57028`（215,748,463 バイト。HEAD は `fc84a6d5…`、215,748,026 バイト）。註釈ありの最終ビルドの SHA-256 は `0628498f346cc2db91d21d9d5a151b19253126e3214969826217a102f7578cd2`（`e-6mod-final/log/`）。HADDPD の追加前後の比較で、FLEngine の C（註釈あり）の「流れの SHA」は HEAD の `5624ac31…` と一致した（FLEngine に `haddpd` は無い）。dsp_ippv2（`--seeds`）の C は 266,650,070 → 266,647,268 バイト、`unsupported_total` 391,252 → 390,454（**798 か所**の `haddpd` が lift された）。
- **ビルド**: `fe-lift`／`fe-pe` は `--release`（`CARGO_PROFILE_RELEASE_DEBUG=0`）。依頼の「dev」ではなく release にしたのは、FLEngine の lift（約 250 万命令）に dev ビルドが遅いため（dev ビルドでの所要は測っていない）。警告なし。C は `clang -fsyntax-only -Wall -Wextra` と、実際のビルド（`-Wall -Wextra`）で警告 0。統合した木は dev プロファイルでコンパイルした（警告 0。FLEngine の lift は約 20 秒）。
- **差分の適用**: `patch -p1 --dry-run` と実適用（HEAD の `git archive` のコピーに）。適用後の 23 ファイルがワークツリーと `cmp` で一致。統合では I2 → R2 の順に当て、R2 が 3 ファイル（`imports_table.rs`、`tests/sysmetrics.rs`、`tests/vcall_thunk_dll.rs`）で拒否ハンクを出したので手で統合した（`MERGE-NOTES.md`）。`resolved-I2R2.diff` は別の clean な archive に `patch -p1` で当たり（dry-run と実適用。ずれ・拒否なし）、適用後の木はテストした木と `diff -r` で一致した。

## 解釈が分かれた点

1. **幅 0 × 高さ 0 の CreateBitmap**: ゲストは glyph 'C'（0x43）、サイズ 0xc8 の cursor を作る関数で、文字列は長さ 1 の "C"（gdb で `len field 1`、`4300…` を読んだ）。テキスト測定（ILTools の `call [rbx+0x18]` の先は QFC の `f_1e82290`、vtable は 0x1e7e9b0。gdb で呼び出しを確認した）の結果の矩形が**ゼロ**になり、キャンバスの大きさが 0 × 0 になった。**なぜ 0 かは確認していない**（QFC の測定が空の矩形を返す。フォントのデータが無いこと、または QFC／freetype の経路の未実装を疑うが、読んでいない）。実機の Windows でゼロになるとは考えにくい。したがって **W5 の応答（ドキュメントどおりの 1×1）は API として正しくても、ゲストは実機とは違う状態で先へ進んでいる可能性がある**。この状態で W6 以降を直すと、**劣化した状態に対する応答を積む**ことになる。未決: (a) このまま W6 に進む、(b) 先に測定の矩形がゼロになる原因を調べる。**この作業は (b) をしていない**。
2. **HADDPD**: (a) SDM の疑似コードは `DEST[63:0] := SRC1[127:64] + SRC1[63:0]`（高位が先）だが、**ホストの CPU（AMD Zen 5）の結果は低位が先の演算順に一致した**（変異検査: 高位を先にすると 16,000 件のどれかで不一致）。2 つの被加数がどちらも QNaN で中身が違うときだけ結果が変わる。**Intel の機械では確かめていない**。この作業は実測（ホスト）に従った（MXCSR 系のテスト全体がホストを oracle にしているのと同じ方法）。(b) 決定 0075 は SSE3 を「CPUID が名乗らないので実装しない。CPUID に足すか実装するかは PM が決める」とした。今回 **CPUID は SSE3 を名乗らないまま、ゲストが `haddpd` に届いた**（観測）。IPP の CPU 振り分けが ECX を見て選んでいるとは限らないことを示す（なぜ SSE3 の実装が選ばれたかは調べていない）。HADDPD 1 つだけを実装し、残りの SSE3 には触れなかった。
3. **答える値の根拠の強さ**: SM 13／14 は Learn の表からの導出、SPI 0x101A は出典なしの選択。決定 0057 の「出典のない値を事実として書かない」に沿って、後者は「仮の決定、出典なし、作業者の選択」と書いた。
4. **フック点を ILTools に足すこと**: 決定 0105 の設計は「註釈ファイルを用意する」なので、機構は変えていない。ILTools の 3 点が、FLEngine と QFC の 3 点と同じ Delphi の RTL 関数の ILTools 自身の複製だという**読み**に基づく（先頭 5 バイトの一致が根拠）。
5. **途中の段階の測定が 1 回、再リンク（S2、S3）がコンソールの記録だけ**: 最終（S6）と対照は 2 回測り、`native.txt` が同一。S1／S4／S5 は 1 回（保存済み）。

## 限界

- **ILTools の DllMain(ATTACH) は未完了**（ILTools の InitUnits 221 / 283）。`LoadLibraryW "iltools_x64.dll"` は戻っていない。unit 245 の先も、unit 280／659 も、`GetProcAddress("CreateCursorFromFont")` の呼び出しも未測定。
- 上の「解釈が分かれた点」の 1: 空の測定矩形の原因が未確認。S6 より先の応答は、その状態に対する応答になる。
- 新しい応答はどれも**実機 Windows と照合していない**。SM 13／14 は Learn の表のページが Windows 10 22H2 に当てはまるかを書かない。SPI 0x101A は出典がない。
- HADDPD の oracle は AMD Zen 5 の 1 台だけ。Intel の演算順は未確認。
- 置き換え先の関数（ゲストのフック層）が実行されて停止しなかったことだけを確認した。結果の正しさ（ゲストが意図した文字列クラスの横取りが働いているか）は確認していない。
- wasm は未ビルド（emcc を使っていない）。`RM_MAX` の引き上げが wasm の実行で問題を起こさないことは、静的な読み（静的配列 320 バイト、locals や表の数の変化なし）だけ。
- I2 の作業では `cargo test --workspace` を走らせず、上の 18 バイナリだけを走らせた。統合 M2 で全体テストを走らせた（上。グループ分けの合算。1 回の通しではない）。統合した木での a4・census の実測はしていない。
- 種 400 個（決定 0104 P2）と、決定 0100 の thunk の種の重なりは調べていない。
- 途中の段階 S2、S3 は再リンクのビルドで、生成コードは S1 のものを使った（runtime だけの変更なので正確）が、フルビルドではない。

## 測定の道具と成果物

- `/tmp/opencode/i2/`（RAM。**削除していないもの**: `src/`（HEAD の `git archive` に差分を当てたもの。約 12 MB と、テストが作った `out-*`）、`target/`（fe-lift／fe-pe の release と、テストの dev ビルド。約 110 MB）、`res/`、`py/`、`learn/`、`head/`（HEAD の archive）、スクリプト）。測定の道具: `bin/clang`（`-g` を外し、コンパイルした分割ソースを消す。h2 の同名の道具の複製）、`a4-multi-trim.sh`（リポジトリの `a4-multi.sh` の複製。差は `HERE` の固定と、gen の後に生成 C を消す 2 行だけ）、`run6.sh`／`run5.sh`（モジュール引数）、`iter.sh`、`lean.sh`／`lean-run.sh`（runtime だけの変更の再リンク）、`rerun.sh`（2 回目の実行）、`py/gdbR.py`（gdb で `R` を読む）。**リポジトリのスクリプトは `a4-multi.sh` のコメントと既定の SHA 以外を変えていない**。
- 結果（`lift/out-git/i2-results/`）: `a-6mod-head-rm8/`（S0。2 回）、`c-6mod-hookpts-sm-spi-hadd/`（S4）、`d-6mod-createbitmap-wall-createcompatibledc/`（S6 と同じ木の 1 回目）、`e-6mod-final/`（S6 の 2 回と gdb の記録: ILTools の InitCount、ローダー呼び出しの trace、フックの計数、unit 表）、`f-5mod-control/`（対照。2 回）、`stage-logs/`（各ビルドの標準出力）、`wallI2.diff`、本書、`README-0106-line.txt`。
