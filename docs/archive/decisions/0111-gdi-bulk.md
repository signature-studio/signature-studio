# 0111: ILTools の gdi32／user32 の非ピクセル系の API の一括 — DC の属性、DC スタック（SaveDC／RestoreDC）、ペンとソリッド ブラシ、パレット エントリ、アイコンとカーソルの状態、DIB セクション上の最小の画素操作（FillRect／PatBlt／BitBlt SRCCOPY）、名前付き停止 69 行（第 1 段階の設計と実装案に、第 2 段階の統合・コンパイル・全体テスト・変異検査・6 モジュール構成の測定を加えた版）

状態: **仮の決定**（番号 0111 は PM の仮の割り当て）。**第 2 段階（統合・コンパイル・テスト・測定。別の作業者、2026-10-27／28）を実施した版**: HEAD `0c5000f`（ブランチ main）に、PM の決定どおり**4 つの差分すべて**（主、extra-tests、getdevicecaps、stretchblt）と F2（決定 0110）を当て、RAM 上でコンパイルし、全体テストを失敗 0 にし（113 test result・**1,526 passed**・0 failed。グループ分けの合算）、変異検査 20 件を実行し（20 件検出）、FLEngine の生成 C の SHA と関数単位の差を測り、6 モジュール構成の a4 を配置なし／あり（決定 0110）で測定した（下の「第 2 段階の結果」）。第 1 段階の本文（設計、Learn の読み、規則の表、出典なしの選択、撤回条件）は、状態・件数・SHA・予測の記述以外は書き換えていない。0054、0066、0068、0098、0106、0108 の本文は書き換えない。差分は統合後の `resolved-F2G1.diff`（`patch -p1`）に含まれる。**実機 Windows との照合は 0 件。**

> **これは A4 の達成ではない。** 6 モジュール構成（測定用の基底と種）の FLEngine は決定 0108 の後 `gdi32!MoveToEx` の import thunk（0x601da30）で止まっている（init_count 246 / 942。実測は決定 0108 の第 2 段階）。**本決定の第 2 段階の測定では、停止は `gdi32!StretchDIBits`（名前付き停止、区分 C）に動いた。init_count は 246 / 942 のまま**（下の「第 2 段階の結果」）。この決定は、その先で**同じ種類の壁を 1 つずつ踏む**のをやめるために、ILTools が使う gdi32／user32 のうち**ピクセルを書かない API**を、実機の仕様（Microsoft Learn）から導ける範囲でまとめて実装し、導けないものは**あらかじめ名前付きの停止にする**設計である。**実機 Windows との照合は 0 件**で、応答はすべて仮。この文書の数は、「実測」と書いたものを除き、読み（静的な計算）である。

区分の凡例: **Learn** = Microsoft Learn に書かれていること（取得日 2026-10-27）。**読み** = 逆アセンブルや文の読みから導いたもので、実行して確かめていない。**選択** = 一次資料に無い作業者の選択（「出典なしの選択」）。**推測** = 推論。**未確認** = 確認していない。**UNOFFICIAL** = 数値を Windows SDK のヘッダ（mingw-w64 の `wingdi.h`／`winuser.h`。この環境にある）から取ったもの（Learn は名前だけを書く。決定 0053、0054 と同じ扱い）。

## 新しく分かったこと

### 1. Learn に「Display Device Context Defaults」がある（決定 0108 が見つけていなかった一次資料）

`https://learn.microsoft.com/en-us/windows/win32/gdi/display-device-context-defaults`（`updated_at` 2025-03-11）。表題は**表示 DC**の既定値で、取得できた表（原文のまま）: Background color「Background color setting from Control Panel (typically, white)」、Background mode「OPAQUE」、Bitmap「None」、Brush「WHITE_BRUSH」、Brush origin「(0,0)」、Clipping region「Entire window or client area with the update region clipped, as appropriate...」、Palette「DEFAULT_PALETTE」、Current pen position「(0,0)」、Device origin「Upper left corner of the window or the client area」、Drawing mode「R2_COPYPEN」、Font「SYSTEM_FONT」、Intercharacter spacing「0」、Mapping mode「MM_TEXT」、Pen「BLACK_PEN」、Polygon-fill mode「ALTERNATE」、Stretch mode「BLACKONWHITE」、Text color「Text color setting from Control Panel (typically, black)」、Viewport extent「(1,1)」、Viewport origin「(0,0)」、Window extent「(1,1)」、Window origin「(0,0)」。

`CreateCompatibleDC` のページは「all attributes are set to normal default values」と書く。**この決定は、その「normal default values」をこの表の値と読む**（読み。メモリ DC に固有の差は「Bitmap」＝ 1 × 1 のモノクロ（Learn の CreateCompatibleDC）と、クリップ領域と原点の欄だけ）。新しい属性の初期値（背景モード OPAQUE、描画モード R2_COPYPEN、マッピング モード MM_TEXT、ストレッチ モード BLACKONWHITE、現在位置・ブラシ原点・ウィンドウ原点・ビューポート原点 (0,0)）はこの表から取り、**選択ではなく読みの区分**にする。

**決定 0108 への影響（0108 は書き換えない）**: 0108 の D3 (b)（既定の 4 オブジェクトの正体）と (c)（初期色）、P1、P2 は「Learn に無い」としていたが、この表は**表示 DC について**ブラシ = WHITE_BRUSH、ペン = BLACK_PEN、フォント = SYSTEM_FONT、パレット = DEFAULT_PALETTE、背景色「typically, white」、文字色「typically, black」と書く。0108 の選択（黒の文字色、白の背景色）はこの表と矛盾しない。**既定ブラシの色**はこの決定の `PatBlt(PATCOPY)` が必要とする（既定ブラシのまま PATCOPY する経路）が、0108 の既定ブラシは隠しオブジェクトで色が無いので、この決定では**名前付きで止める**。0108 の D3 をこの表で正式化するかは未決（「未決の点」の 3）。

### 2. VCL の TCanvas の経路（読み。ILTools の逆アセンブル）

決定 0108 の停止点 `MoveToEx` の呼び出し元は `f_6130690`（`TCanvas.MoveTo` と読む: vtable +0x58 を呼んでから `MoveToEx(canvas.Handle, x, y, NULL)`）。その上は `f_6131150`（`TCanvas.SetHandle` と読む: 新しいハンドルを置いたあと vtable +0x38 の `SetPenPos` → vtable +0xc8 の `MoveTo`）。`TBitmapCanvas.CreateHandle`（`f_61374a0`）が `CreateCompatibleDC(NULL)` と選択のあと `Handle := dc` を呼ぶ。以降の `TCanvas.Draw`（`f_6130190`）→ `RequiredState`（`f_61311b0`）は、要る状態のビットごとに次を呼ぶ（読み）:

| `RequiredState` のビット | 関数 | 呼ぶ API |
|---|---|---|
| フォント | `f_6131260` | `SelectObject(Font.Handle)`、`SetTextColor(ColorToRGB(Font.Color))`（`ColorToRGB` が `GetSysColor`） |
| ペン | `f_61312a0` | `SelectObject(Pen.Handle)`（`f_612ed70` が `CreatePenIndirect` を呼ぶ: 0x612edef）、`SetROP2(表[Pen.Mode])` |
| ブラシ | `f_61312e0` | `UnrealizeObject(Brush.Handle)`、`SelectObject(Brush.Handle)`、`SetBkColor`、`SetBkMode(2 または 1)` |

その後の `graphic.Draw`（vtable）は、`f_6138cd0`／`f_6139290`（`TBitmap.Draw`／`StretchDraw` に近い形と読む）なら次の順に呼ぶ（読み。条件分岐は未確認）: `GetStretchBltMode`、**`GetDeviceCaps(キャンバスの DC, 12)` と `(…, 14)`**、`GetBrushOrgEx`、`SetStretchBltMode`、`SetBrushOrgEx`、`CreateCompatibleDC`、`SelectObject`、**`StretchBlt`**（0x613918e）、`DeleteDC`。

**次の壁の予測（第 1 段階の読み。第 2 段階の実測では外れた: 実際の壁は `GetDeviceCaps` でも `StretchBlt` でもなく `StretchDIBits`）**: この決定の主差分で `MoveToEx`、`SelectObject`、`SetROP2`、`UnrealizeObject`、`SetBkMode`、`GetStretchBltMode`、`SetBrushOrgEx` ほかは通る。続く壁は ① **`GetDeviceCaps` のメモリ DC**（既存の `fle_gdc.c` は画面 DC だけ。主差分では**触らない**。任意の差分 `g1-optional-getdevicecaps.diff` が答える）、② **`StretchBlt`**（主差分では名前付き停止。1:1 の SRCCOPY を BitBlt と同一視する任意の差分 `g1-optional-stretchblt.diff` がある）。この 2 つは**依頼の範囲（既存の関数の応答を変えない、StretchBlt は例外に入っていない）に照らして任意の差分にし、未決のまま残した**。

### 3. ILTools の静的な閉包（実測ではなく計算）

起点を DllMain（RVA 0x2b35f0）と初期化表の runnable 231 の proc とし、`.pdata` の関数範囲の中の `call rel32`／`jmp rel32`／`call [IAT]` を辿った（**L1**: 1,283 関数、import 159）。`lea reg, [rip+コード]` で番地を取られた関数も足した（**L2**: 2,144 関数、import 225）。**vtable 経由（データ中の関数ポインタ）は辿らない**ので、どちらも**下限**。実例: `MoveToEx` は L2 に入らない（vtable 経由）が実測では最初の壁だった。

| 区分 | 件数 | L1 に入る | L2 に入る |
|---|--:|--:|--:|
| A（答える）27 | 27 | 3（GetPaletteEntries、DestroyCursor、GetIconInfo） | 3 |
| C（名前付き停止）69 | 69 | 6（GetSystemPaletteEntries、CreateDIBitmap、GetBitmapBits、StretchBlt、GetTextMetricsW、CreateIcon） | 9（+ GetTextExtentPointW、DrawIcon、DrawTextW） |
| D（行を足さない）180 | 180 | 11 | 54（user32 48、comctl32 6） |

D のうち L2 に入る 54 は `fle_unsupported`（`unsupported instruction at 0x…`）の形で止まる。**停止の API 名は分類表 `gdi-bulk-classification.tsv` の `iltools_thunk_va_rebased`（基底 0x6000000）から引ける。** 名前付き停止の行を足すかは未決（「未決の点」の 4）。

## 対象 API の確定

ILTools の import 表（標準の import。遅延ロードは別）は gdi32 104、user32 199（遅延ロードの 13 を足すと 212）、comctl32 29。応答表（HEAD）に行が無いものは gdi32 **80**、user32 **167**（標準の 199 のうち）、comctl32 **29**。次のように分類した（全件は `gdi-bulk-classification.tsv`。列: dll、api、iltools_import、row_at_head、row_after_0111、class、subclass、closure_L1_depth、closure_L2、iltools_thunk_va_rebased、flengine_thunk_sites、qfc_imports_it、note）:

| 区分 | 内容 | gdi32 | user32 | comctl32 | 計 |
|---|---|--:|--:|--:|--:|
| 既存（変更なし） | 行があり、この決定が触らない | 22 | 31 | — | 53 |
| **B** | 行は決定 0108 にある。**本体を変える**（FillRect、PatBlt、BitBlt。DIB セクションへの最小の画素操作） | 2 | 1 | — | 3 |
| **A** | 状態の設定・照会で、Learn から結果が導ける。**答える** | 21 | 6 | — | 27 |
| **C** | ラスタライズ・グリフ・領域・メタファイル・システムのデータが要る。**行を足して名前付き停止** | 59 | 10 | — | 69 |
| **D** | 窓・メッセージ・メニュー・クリップボード・入力・スクロール バーなど（窓のモデルが無い）と comctl32 の ImageList／フラット スクロール バー。**行を足さない** | — | 151 | 29 | 180 |
| 遅延ロード | thunk を通らない（GetProcAddress が解決する） | — | 13 | — | 13 |

**新しい応答表の行 = A 27 + C 69 = 96**（B は既存の行）。ILTools 以外の同じ名前（DLL 名と関数名で引く）への影響: **FLEngine は 96 行すべてを import し、import thunk は 100**（ExtTextOutW、GetRandomRgn、GetTextExtentPoint32W、DrawTextW が 2 つずつ）。**生成 C で本体が変わる FLEngine の関数は 100**。ILTools の thunk は 97、QFC が import するのは 39（A 13、C 26）、dsp_ippv2／Blend2D／freetype は 0。`call [IAT]` 形（決定 0077）は FLEngine、ILTools、QFC とも 0。

**依頼文の解釈**: 「LineTo の現在位置」は、**現在位置の設定と照会（`MoveToEx`、`GetCurrentPositionEx`）を A**、**`LineTo` 自体は線を描くので C**と読んだ（「解釈が分かれた点」の 1）。

## 決定（作業者。仮）

| # | 項目 | 決定 | 区分 |
|---|---|---|---|
| D1 | 方針 | **実機の仕様（Learn）から導ける応答だけを実装し、導けないものは名前付きの停止にする**。新しい停止の書式は `fle: <API>: <理由> (decision 0111)`（`gdi32!` の接頭辞なし。既存の `fle_gdi.c` と同じ）。偽の成功で先へ進まない。値を創作する箇所は下の「出典なしの選択」に列挙する | PM の方針 |
| D2 | DC の属性 | 既存の `gdi_dc`（0108）に、背景モード、描画モード、マッピング モード、ストレッチ モード、現在位置、ウィンドウ原点、ビューポート原点、ブラシ原点、DC スタックを足す。**メモリ DC だけ**。画面 DC（`GetDC(NULL)`）には属性の状態が無いので、`SetTextColor` と同じく停止（`hdc is a DC of GetDC(NULL)`）。無効なハンドル、削除済みの DC、DC でないハンドルも停止（0108 の `need_mem_dc`） | 表の形は選択 |
| D3 | 初期値 | 背景モード OPAQUE（2）、描画モード R2_COPYPEN（13）、マッピング モード MM_TEXT（1）、ストレッチ モード BLACKONWHITE（1）、現在位置・ブラシ原点・ウィンドウ原点・ビューポート原点 (0,0)、DC スタック空。**Learn の Display Device Context Defaults を「normal default values」と読む**。数値は UNOFFICIAL | Learn（読み） |
| D4 | 現在位置 | `MoveToEx(hdc, x, y, lppt)`: 論理単位のまま保存。lppt が非 NULL なら**前の位置**を 8 バイトの POINT に書く。戻り 1。`GetCurrentPositionEx` は lppt が NULL なら停止（ページが定めない） | Learn |
| D5 | グラフィック モード | `SetBkMode`: 1（TRANSPARENT）と 2（OPAQUE）。`SetROP2`: 1〜16。`SetMapMode`: 1〜8。`SetStretchBltMode`: 1〜4（STRETCH_* は別名）。**それ以外は停止**（SetStretchBltMode のページは「ドライバ固有の追加モードがありうる」と書くので、無効値として 0 を返さない）。戻り値は前のモード。`GetStretchBltMode` は現在のモード。マッピング モードは**保存するだけ**で、延長（extent）は持たない: 変換を使う呼び出し（LPtoDP、GetClipBox、画素操作）は **MM_TEXT 以外で停止**（Learn は固定の 6 モードの延長を「システムが設定する」と書き、値を書かない） | Learn |
| D6 | 原点と LPtoDP | `SetWindowOrgEx`／`SetViewportOrgEx`（x, y, lppt）と `GetWindowOrgEx`。原点はマッピング モードと独立（Learn「Origins are specified in absolute units that the current mapping mode does not affect」）。`LPtoDP` は MM_TEXT で `Dx = Lx - WOx + VOx`（Learn の式 `Dx = ((Lx - WOx) * VEx / WEx) + VOx` と、MM_TEXT の延長 1 と 1）。c は正、変換後の座標が 27 ビット符号付き（−2^27 〜 2^27−1）に収まること（「exceed 27 bits」の読み）。外れたら**0 を返さず停止**（ページは結果を「undefined」と書く） | Learn（境界は読み） |
| D7 | ブラシ原点 | `SetBrushOrgEx`／`GetBrushOrgEx`: x, y は **0〜7 だけ**受ける（GetBrushOrgEx のページ「values between 0 and 7」）。範囲外は停止（SetBrushOrgEx のページは「ブラシの幅で剰余」と書き、幅を与えない） | Learn |
| D8 | SaveDC／RestoreDC | `SaveDC` は状態を DC ごとのスタックに積み、**積んだ後の深さ**を返す（最初は 1。ページは「identifies the saved state」としか書かない: **選択**）。`RestoreDC(n)`: n > 0 は n 番目の状態、n < 0 は上から \|n\| 番目（−1 = 直近）、n = 0 は停止、存在しない番号は 0（ページの失敗値）。復元した状態とその上の状態を捨てる。**保存・復元するもの**: 選択中の 5 つのオブジェクト（ビットマップ、ブラシ、ペン、フォント、パレット）、背景・描画・マッピング・ストレッチの 4 つのモード、ウィンドウ原点とビューポート原点（読み: マッピングの平行移動）。**テキスト色、背景色、現在位置、ブラシ原点は保存対象かページが書かない**ので、復元時に保存時の値と違えば停止。保存された**オブジェクトが削除済み**、または保存されたビットマップを**別の DC が持っている**ときも停止 | Learn（範囲は読み、番号は選択） |
| D9 | GdiFlush | 1 を返す（この runtime はバッチしない: 呼び出しは完了するか run を止める。ページ「If all functions in the current batch succeed, the return value is nonzero」） | Learn |
| D10 | GetClipBox | メモリ DC・MM_TEXT: クリップ領域が作れず（領域の呼び出しは停止）窓も重ならないので、可視領域 = 選択中のビットマップ。デバイス矩形 [0,w)×[0,h) を論理座標（Lx = Dx + WOx − VOx）に直して書き、戻り SIMPLEREGION（2）。lprect が NULL、他のマッピング モード、画面 DC は停止 | Learn（読み） |
| D11 | ペン | 新しい種類 `FLE_GDI_KIND_PEN = 12`。`CreatePenIndirect(LOGPEN)`: lopnStyle 0〜6（PS_SOLID〜PS_INSIDEFRAME、UNOFFICIAL）、lopnWidth.x ≥ 0（y は使わない）、色の上位バイト 0。それ以外は停止。`SelectObject`／`DeleteObject` は既存のブラシと同じ規則（選択中の削除は 0）。**ペンで線は引かない**（`LineTo` ほかは名前付き停止） | Learn |
| D12 | ソリッド ブラシと UnrealizeObject | `CreateSolidBrush(color)`: 上位バイト 0 のとき、0108 の `CreateBrushIndirect` と同じ種類の BS_SOLID を作る。`UnrealizeObject`: 生きているブラシ（Learn「does nothing, and the function returns TRUE」）と、`CreatePalette`／`CreateHalftonePalette` のパレット（この runtime は RealizePalette を答えないので、巻き戻す状態が無い）は 1。ストック オブジェクト（ページ「should not be used with stock objects」）、既定のオブジェクト、他の種類、無効なハンドルは停止 | Learn |
| D13 | GetPaletteEntries | `CreatePalette` のパレット（16 エントリ）と pPalEntries が非 NULL: `min(cEntries, 16 − iStart)` 個を 4 バイトずつ書き、その数を返す（iStart ≥ 16 は 0）。ページの NULL の文は取り違えている（「handle が NULL ならエントリ数」）ので、hpal または pPalEntries が NULL は停止。半色調パレット（「implementation detail」）、既定のパレット、他の種類は停止 | Learn |
| D14 | アイコンとカーソル | `DestroyIcon`／`DestroyCursor`: `CreateIconIndirect` が作った生きているものを削除して 1（ハンドルは再利用しない。種類は残し、2 回目は 0）。NULL と破壊済みは 0（無効なハンドル）。`DestroyCursor` は fIcon が FALSE のものだけ、`DestroyIcon` は両方。**共有のハンドル**（LoadIconW、LoadCursorW、LoadImageW）、`SetCursor` の現在のカーソル（ページ「must not be in use」）、他の種類は停止。`GetIconInfo`: fIcon が FALSE のカーソルだけ。fIcon、ホット スポット、そして**新しい 2 つのビットマップ**（マスク = モノクロ、hbmColor = 色ビットマップが渡されたときだけ仮想画面の COLOR 形式）。アイコン（ホット スポットが「中心」で数値が無い）は停止 | Learn（hbmColor の形式は読み） |
| D15 | カーソルの状態 | `SetCursor(h)`: h が NULL、LoadCursorW／LoadImageW の標準カーソル、`CreateIconIndirect` のハンドルのとき、現在のカーソルにして**前のカーソル**を返す（最初は NULL: 何も設定されていない世界）。他は停止。`GetCursor`: 現在のカーソル。`ShowCursor(b)`: カウントを ±1 して新しい値を返す。初期値 0（Learn「If a mouse is installed, the initial display count is 0」。`GetSystemMetrics(SM_MOUSEPRESENT)` は 1: 決定 0057） | Learn（初期カーソルは読み） |
| D16 | 画素の操作 | **FillRect**: メモリ DC・MM_TEXT・**24 または 32 ビットの DIB セクション**・ソリッド ブラシ（`CreateBrushIndirect`／`CreateSolidBrush`）または `COLOR_WINDOWTEXT + 1`（= 9。色は 0108 D11 の黒）。論理矩形 [left,right)×[top,bottom) を原点で変換し、**ビットマップの外の部分は書かない**。right ≤ left または bottom ≤ top は何も描かず 1（ページが定める半開区間が空）。**PatBlt**: 上と同じ DIB・MM_TEXT で、rop が PATCOPY（選択中のソリッド ブラシ。**既定のブラシ、中空、ストックのブラシは停止**）、BLACKNESS（0x000000）、WHITENESS（0xFFFFFF）。w, h が負は停止、0 は何も描かず 1。**BitBlt**: rop = SRCCOPY、ソースもデスティネーションもメモリ DC の DIB セクションで**同じ形式**（ビット数 8／16／24／32、圧縮、マスク、8 ビットは色表）、MM_TEXT、cx, cy は負なら停止・0 なら何もせず 1、ソース矩形はソースのビットマップの内側（外は停止: ページ「BitBlt only does clipping on the destination DC」）。画素のバイトをそのままコピー（32 ビットの未使用バイトも）。同じビットマップ内は memmove（行の順序を重なりの向きで選ぶ）。**書き込みはこの 2 か所（`dib_fill` と BitBlt の `memmove`）だけ**。DDB（画素を持たない）、他のビット数、クリップ領域（作れない）、他の rop、画面 DC はすべて名前付き停止（メッセージの先頭に 0108 と同じ引数の文字列） | Learn（バイト順は読み、未使用バイトは選択） |
| D17 | 名前付き停止 69 | 下の表。呼ぶと `fle: <API>: <理由> (decision 0111)` で止まる。**停止関数はスレッドを調べない**（状態に触れない）。新しい .c／.h は作らない（`fle_gdi.c`／`.h` に追記） | PM の方針 |
| D18 | 応答表 | **96 行を末尾に追記**（答え 27 → 名前付き停止 69 の順。順序は下）。既存の行の関数名・応答は変えない | 決定 0015 |
| D19 | 任意の差分 | ① `g1-optional-getdevicecaps.diff`（`fle_gdc.c` に弱い参照 1 本。メモリ DC の `GetDeviceCaps(12／14)` が選択中のビットマップのビット数とプレーン数を返す。他の index は停止。既存のテスト 1 行を変える）。② `g1-optional-stretchblt.diff`（`StretchBlt` の幅・高さが等しいときは BitBlt と同じ。BitBlt の本体を `blit()` に出す）。**どちらも主差分には入れていない**（既存の関数の応答を変える／例外に入っていない）。適用するかは未決 | 選択 |
| D20 | 作らないもの | 新しい .c／.h。`GetObjectW` の拡張（ブラシ、ペン）。`SetTextAlign`、`GetBkMode` ほかの照会（ILTools は import しない）。ペンでの描画、領域、クリップ、メタファイル。`CreateIcon`（ビット列を持たない）。D の 180 の行 | 範囲 |

### 出典の補足（Learn のページと取得できた文言）

URL は `https://learn.microsoft.com/en-us/windows/win32/api/` の下（wingdi／winuser）。`updated_at` は各ページの meta。原文は `lift/out-git/g1-results/learn/*.txt`（取得に使った道具は決定 0108 の `w6_fetch.sh`、`w6_html2txt.py`）。

- `wingdi/nf-wingdi-movetoex`（2024-02-22）「updates the current position to the specified point and optionally returns the previous position」「If this parameter is a NULL pointer, the previous position is not returned」。`getcurrentpositionex`「retrieves the current position in logical coordinates」。
- `setbkmode`（2025-05-01）「OPAQUE」「TRANSPARENT」「the return value specifies the previous background mode」、例の注釈「Reset the background mode to its default. SetBkMode(hdc, OPAQUE);」。`setrop2`（2024-02-22）16 の R2_ 名、「the return value specifies the previous mix mode」。`setmapmode` 8 の MM_ 名、「the return value identifies the previous mapping mode」。`setstretchbltmode`「the return value is the previous stretching mode」「Additional stretching modes might also be available depending on the capabilities of the device driver」。`getstretchbltmode`「the current stretching mode」。
- `setwindoworgex`「specifies which window point maps to the viewport origin (0,0)」「A pointer to a POINT structure that receives the previous origin of the window, in logical units. If lpPoint is NULL, this parameter is not used」。`setviewportorgex`「specifies which device point maps to the window origin (0,0)」。`getwindoworgex`。`lptodp`「The LPtoDP function fails if the logical coordinates exceed 32 bits, or if the converted device coordinates exceed 27 bits. In the case of such an overflow, the results for all the points are undefined」。
- `gdi/mapping-modes-and-translations`「Dx = ((Lx - WOx) * VEx / WEx) + VOx」「Origins are specified in absolute units that the current mapping mode does not affect」「When no translation is in effect (this is the default), page space in the MM_TEXT mapping mode is equivalent to physical device space」。`gdi/default-transformations`「MM_TEXT (the default page-space to device-space transformation)」「the viewport dimensions are 1-pixel by 1-pixel and the window dimensions are 1-page unit by 1-page unit」。
- `setbrushorgex`「The default brush origin coordinates are (0,0)」「If this value is greater than the brush width, its value is reduced using the modulus operator」。`getbrushorgex`「The brush origin is a set of coordinates with values between 0 and 7」。
- `savedc`「copying data describing selected objects and graphic modes (such as the bitmap, brush, palette, font, pen, region, drawing mode, and mapping mode) to a context stack」「the return value identifies the saved state」。`restoredc`「If this parameter is positive, nSavedDC represents a specific instance of the state to be restored. If this parameter is negative, nSavedDC represents an instance relative to the current state. For example, -1 restores the most recently saved state」「any states saved after the restored state was created are also destroyed」。`gdi/graphic-modes`（5 つの「graphic modes」: Background、Drawing、Mapping、Polygon-fill、Stretching）。
- `gdiflush`「If all functions in the current batch succeed, the return value is nonzero」。`getclipbox`「The visible area is defined by the current clipping region or clip path, as well as any overlapping windows」「SIMPLEREGION: Region is a single rectangle」。
- `unrealizeobject`「If hgdiobj is a brush, UnrealizeObject does nothing, and the function returns TRUE」「should not be used with stock objects」。`createsolidbrush`「creates a logical brush that has the specified solid color」。`createpenindirect`「creates a logical cosmetic pen that has the style, width, and color specified in a structure」。`ns-wingdi-logpen`（PS_SOLID、PS_DASH、PS_DOT、PS_DASHDOT、PS_DASHDOTDOT、PS_NULL、PS_INSIDEFRAME。「If the x member is NULL, then the pen is one pixel wide on raster devices」）。
- `getpaletteentries`「If the nEntries parameter specifies more entries than exist in the palette, the remaining members of the PALETTEENTRY structure are not altered」。
- `winuser/nf-winuser-destroyicon`（2025-07-01）「It is only necessary to call DestroyIcon for icons and cursors created with the following functions: CreateIconFromResourceEx (if called without the LR_SHARED flag), CreateIconIndirect, and CopyIcon」。`destroycursor`「Do not use this function to destroy a shared cursor」「The cursor must not be in use」。`geticoninfo`「GetIconInfo creates bitmaps for the hbmMask and hbmColor or members of ICONINFO. The calling application must manage these bitmaps and delete them with DeleteObject call when they are no longer necessary」。`setcursor`「If there was no previous cursor, the return value is NULL」「The cursor must have been created by either the CreateCursor or the CreateIconIndirect function, or loaded by either the LoadCursor or the LoadImage function」。`getcursor`「If there is no cursor, the return value is NULL」。`showcursor`「If a mouse is installed, the initial display count is 0」。
- `patblt`「PATCOPY: Copies the specified pattern into the destination bitmap」「BLACKNESS: Fills the destination rectangle using the color associated with index 0 in the physical palette. (This color is black for the default physical palette.)」「WHITENESS: ... index 1 ... (This color is white for the default physical palette.)」。`fillrect`「includes the left and top borders, but excludes the right and bottom borders」「GDI fills a rectangle up to, but not including, the right column and bottom row, regardless of the current mapping mode」。`bitblt`「SRCCOPY: Copies the source rectangle directly to the destination rectangle」「BitBlt only does clipping on the destination DC」「If the color formats of the source and destination device contexts do not match, the BitBlt function converts the source color format to match the destination format」。
- `gdi/display-device-context-defaults`（上の表）。`ns-wingdi-bitmapinfoheader`「if biHeight is positive, the bitmap is a bottom-up DIB with the origin at the lower left corner. If biHeight is negative, the bitmap is a top-down DIB with the origin at the upper left corner」。**24／32 ビットの画素のバイト順（青、緑、赤、32 ビットは未使用バイト）を直接書く文はこの版のページには無い**: `RGBTRIPLE`（rgbtBlue、rgbtGreen、rgbtRed）と `RGBQUAD` の並び、BI_BITFIELDS のマスク（「The 1 bits in each mask indicate the bits for that color within the pixel」。0x00FF0000／0x0000FF00／0x000000FF は赤が上位）からの**読み**。

## 1 呼び出しごとの表: Learn の文言 → 実装の規則（A 27、B 3、任意 2）

「停止」は `fle_stop`（`fle: <API>: <理由> (decision 0111)` と abort）。最後のエラーはどの呼び出しも変えない。画面 DC、NULL、削除済みの DC、DC でないハンドルは、DC を取る A のすべてで停止（`need_mem_dc`）。以下は引数がそれ以外の場合の規則。

| API（レジスタ） | Learn の文言 | 実装の規則 | 区分 |
|---|---|---|---|
| `MoveToEx`(rcx hdc, edx x, r8d y, r9 lppt) | 「updates the current position to the specified point and optionally returns the previous position」 | 位置を保存。lppt 非 NULL なら前の位置を書く。1 | Learn |
| `GetCurrentPositionEx`(hdc, rdx lppt) | 「retrieves the current position in logical coordinates」 | 位置を書く。1。lppt NULL は停止 | Learn |
| `SetBkMode`(hdc, edx mode) | 「OPAQUE」「TRANSPARENT」「previous background mode」 | 1 か 2 を保存し前の値を返す（初期 2）。他は停止 | Learn |
| `SetROP2`(hdc, edx rop2) | 16 の R2_ 名「previous mix mode」 | 1〜16。初期 13 | Learn（数値 UNOFFICIAL） |
| `SetMapMode`(hdc, edx iMode) | 8 の MM_ 名「previous mapping mode」 | 1〜8 を保存。初期 1。MM_TEXT 以外では変換を使う呼び出しが停止 | Learn |
| `SetStretchBltMode`／`GetStretchBltMode` | 「previous stretching mode」「Additional stretching modes might also be available」 | 1〜4。初期 1。他は停止 | Learn |
| `SetWindowOrgEx`／`SetViewportOrgEx`(hdc, x, y, lppt) | 「receives the previous origin ... If lpPoint is NULL, this parameter is not used」 | 保存し前の値を書く。1 | Learn |
| `GetWindowOrgEx`(hdc, rdx lppoint) | 「retrieves the x-coordinates and y-coordinates of the window origin」 | 書く。1。NULL は停止 | Learn |
| `LPtoDP`(hdc, rdx lppt, r8d c) | 「converts logical coordinates into device coordinates」「fail ... 27 bits」 | MM_TEXT: `L - WO + VO`。c ≤ 0、NULL、27 ビット超、他のモードは停止 | Learn（境界は読み） |
| `SetBrushOrgEx`／`GetBrushOrgEx` | 「values between 0 and 7」「The default brush origin coordinates are (0,0)」 | x, y が 0〜7。範囲外と NULL は停止 | Learn |
| `SaveDC`(hdc)／`RestoreDC`(hdc, edx n) | 「context stack」「identifies the saved state」「-1 restores the most recently saved state」 | D8 | Learn（番号は選択） |
| `GdiFlush`() | 「If all functions in the current batch succeed, the return value is nonzero」 | 1 | Learn |
| `GetClipBox`(hdc, rdx lprect) | 「tightest bounding rectangle ... visible area」 | D10。戻り 2 | Learn（読み） |
| `UnrealizeObject`(rcx h) | 「If hgdiobj is a brush, UnrealizeObject does nothing, and the function returns TRUE」 | 生きているブラシと作成したパレットは 1。他は停止 | Learn |
| `CreateSolidBrush`(ecx color) | 「creates a logical brush that has the specified solid color」 | 上位バイト 0 のとき新しい BS_SOLID。上位 32 ビットは無視（int でなく COLORREF の下位 DWORD） | Learn |
| `CreatePenIndirect`(rcx plpen) | 「creates a logical cosmetic pen」 | D11 | Learn |
| `GetPaletteEntries`(hpal, edx iStart, r8d cEntries, r9 pPalEntries) | 「the number of entries retrieved」「the remaining members ... are not altered」 | D13 | Learn |
| `DestroyIcon`／`DestroyCursor`(rcx h) | 「Do not use this function to destroy a shared cursor」「must not be in use」 | D14 | Learn |
| `GetIconInfo`(rcx h, rdx piconinfo) | 「creates bitmaps for the hbmMask and hbmColor」 | D14（ICONINFO: fIcon @0、xHotspot @4、yHotspot @8、hbmMask @16、hbmColor @24） | Learn |
| `SetCursor`(rcx)／`GetCursor`()／`ShowCursor`(ecx b) | 「If there was no previous cursor, the return value is NULL」「the initial display count is 0」 | D15 | Learn |
| `FillRect`(rcx hdc, rdx lprc, r8 hbr) | 「includes the left and top borders, but excludes the right and bottom borders」 | D16 | Learn |
| `PatBlt`(hdc, x, y, w, [5] h, [6] rop) | PATCOPY／BLACKNESS／WHITENESS | D16 | Learn（色は読み） |
| `BitBlt`(hdc, x, y, cx, [5] cy, [6] hdcSrc, [7] x1, [8] y1, [9] rop) | 「SRCCOPY: Copies the source rectangle directly to the destination rectangle」 | D16 | Learn |
| （任意）`GetDeviceCaps` のメモリ DC | 「BITSPIXEL: Number of adjacent color bits for each pixel」「PLANES: Number of color planes」＋メモリ DC のページ「an array large enough to store color information」 | 選択中のビットマップのビット数とプレーン数（読み）。他の index は停止 | 読み |
| （任意）`StretchBlt`(…, [10] hSrc, [11] rop) | 「stretching or compressing the bitmap to fit the dimensions of the destination rectangle, if necessary」 | 幅と高さが等しいとき BitBlt と同じ。他は停止 | 読み |

## 名前付き停止 69（区分 C。理由は 8 種類。メッセージの要点）

| 理由 | メッセージの要点 | gdi32 | user32 |
|---|---|---|---|
| 描画（ペンとブラシ） | `drawing with the selected pen and brush needs the rasterizer` | LineTo、Rectangle、RoundRect、Ellipse、Pie、Chord、Arc、ArcTo、AngleArc、Polyline、Polygon、PolyBezier、PolyBezierTo、ExtFloodFill、FrameRgn | FrameRect、DrawEdge、DrawFocusRect、DrawFrameControl、DrawIcon、DrawIconEx |
| 画素の転送・読み戻し | `this pixel transfer or read-back is not one of FillRect, PatBlt and BitBlt(SRCCOPY) ...` | GetPixel、SetPixel、SetPixelV、GetBitmapBits、SetDIBits、SetDIBitsToDevice、StretchDIBits、StretchBlt、MaskBlt、CreateDIBitmap | CreateIcon、LoadBitmapW |
| グリフ | `glyph rasterization needs the font files and the rasterizer` | ExtTextOutW | DrawTextW、DrawTextExW |
| 文字の計測 | `text metrics and extents need the glyph data of the font` | GetTextMetricsW、GetTextExtentPoint32W、GetTextExtentPointW | — |
| 領域とクリップ | `clip regions and region objects are not modelled: BitBlt, PatBlt and FillRect assume that no clip region exists` | SelectClipRgn、ExtSelectClipRgn、IntersectClipRect、ExcludeClipRect、GetClipRgn、RectVisible、GetRandomRgn、CreateRectRgn、CreateRectRgnIndirect、SetRectRgn、CombineRgn、GetRgnBox、OffsetRgn、EqualRgn、RectInRegion、GetRegionData | — |
| システム パレット | `the system palette and the colour matching of palettes are not on the pages` | GetNearestPaletteIndex、ResizePalette、GetSystemPaletteEntries | — |
| DC の原点 | `the final translation origin of a memory DC is not on the page` | GetDCOrgEx | — |
| メタファイル | `metafile data and playback are not modelled` | GetEnhMetaFileHeader、PlayEnhMetaFile、GetEnhMetaFileBits、GetEnhMetaFilePaletteEntries、GetWinMetaFileBits、GetEnhMetaFileDescriptionW、DeleteEnhMetaFile、SetEnhMetaFileBits、CopyEnhMetaFileW、SetWinMetaFileBits | — |
| 計 | | 59 | 10 |

（`StretchBlt` の 1:1 SRCCOPY を BitBlt と同一視する案は任意の差分（D19）。`GetClipBox` は領域を作らずに答える（D10）ので A に置いた。）

## 行を足さない 180（区分 D）

user32 の 151 と comctl32 の 29。**理由**: 窓、メッセージ、メニュー、クリップボード、キーボードとマウスの状態、キャレット、スクロール バー、フック、タイマーのモデルが無い（決定 0071 の窓は 1 つの形だけ）。ILTools の `CreateCursorFromFont` の経路（DllMain → 初期化 → Graphics ユニット）に直接は現れない（上の閉包の計算: L1 に入るのは 11 だけ）。comctl32 は `ImageList_Create` が行を持たず先に止まるので、Draw、Add、GetIcon には届かない。**L2 に入る 54 は、止まるときの形が `unsupported instruction at 0x…`（行が無い）で、API 名は分類表から引く。** 内訳（サブグループ。全 API 名は分類表の `subclass` 列）: 窓のモデル 46、ImageList／フラット スクロール バー 29、メニュー 21、メッセージ キュー 19、描画サイクルと窓の DC 14、キーボードとマウスの状態 11、フォーカス・アクティブ・キャプチャ 9、スクロール バー 8、クリップボード 8、フックとタイマー 5、キャレット 5、モニター 3、ダイアログと音 2。

L2 に入る 54: user32 の MessageBoxW、GetActiveWindow、GetCapture、GetFocus、SetActiveWindow、SetFocus、DeleteMenu、GetSystemMenu、PostMessageW、PostQuitMessage、SendMessageW、MonitorFromWindow、BeginPaint、EndPaint、InvalidateRect、ScrollWindow、EnableScrollBar、GetScrollInfo、GetScrollPos、GetScrollRange、SetScrollInfo、SetScrollPos、SetScrollRange、ShowScrollBar、EnableWindow、EnumChildWindows、EnumThreadWindows、EnumWindows、FindWindowW、GetClassLongPtrW、GetLastActivePopup、GetPropW、GetWindow、GetWindowLongPtrW、GetWindowRect、GetWindowTextW、GetWindowThreadProcessId、IsIconic、IsWindow、IsWindowEnabled、IsWindowVisible、RemovePropW、SetClassLongPtrW、SetPropW、SetWindowPos、ShowOwnedPopups、ShowWindow、UnregisterClassW（48）、comctl32 の FlatSB_GetScrollPos、FlatSB_SetScrollInfo、FlatSB_SetScrollPos、FlatSB_SetScrollProp、ImageList_Create、ImageList_SetBkColor（6）。

## 応答表の行（`imports_table.rs`。末尾に追記）

追記の前の行数は **331**（HEAD `3a1050a`。決定 0108 の 19 行まで）。以下の 96 行をこの順に末尾（`RealizePalette` の行の後ろ）に足す（行 k の擬似 VA は `0x7ff900000000 + 0x10 * (k + 1)`）。**k = 331〜426、擬似 VA は 0x7ff9000014c0〜0x7ff900001ab0**（算術）。生成 C の `fle_resp_count` は 331 → **427**、`fle_dispatch_count` は 59,711 → **59,807**（算術: 関数 59,380 + 427）。

答え 27: gdi32 の MoveToEx、GetCurrentPositionEx、SetBkMode、SetROP2、SetMapMode、SetStretchBltMode、GetStretchBltMode、SetWindowOrgEx、GetWindowOrgEx、SetViewportOrgEx、LPtoDP、SetBrushOrgEx、GetBrushOrgEx、SaveDC、RestoreDC、GdiFlush、GetClipBox、UnrealizeObject、CreateSolidBrush、CreatePenIndirect、GetPaletteEntries、user32 の DestroyIcon、DestroyCursor、GetIconInfo、SetCursor、GetCursor、ShowCursor。続いて名前付き停止 69（上の表の順: gdi32 の 59、user32 の 10。ヘッダ `fle_gdi.h` の宣言の順と同じ）。

## 第 2 段階の結果（実測。凡例: 実測＝この作業でコマンドの出力を見た事実／読み／未確認）

### 1. 統合とコンパイル

- 基準 HEAD `0c5000f`。F2 → G1 主 → extra-tests → getdevicecaps → stretchblt の順に `patch -p1 --dry-run` と適用: **すべてずれ・拒否・fuzz なし**（stretchblt の `tests/gdi.rs` のハンクだけ、getdevicecaps の後に当てたので 3 行の offset。統合差分 `resolved-F2G1.diff` では offset なし）。`imports_table.rs` は HEAD の末尾（`RealizePalette`）の直後に G1 の 96 行（コメント 2 行 + 96 行）が入った。getdevicecaps／stretchblt は応答表に行を足さない。
- `cargo build --offline`（dev）: **警告 0・エラー 0**。`fle_gdi.c`／`fle_gdc.c` を `clang -std=gnu11 -O0／-O1／-O2 -Wall -Wextra -Werror -fPIE`、`gcc -O1 -Wall -Wextra`: **警告 0**。テスト側の警告は HEAD にある 2 件だけ（`monitor.rs` の `INIT_HEX`、`sse_shuf.rs` の不要な `mut`）。
- **Rust／C の実装（`fle_gdi.c`、`fle_gdi.h`、`fle_gdc.c`、`imports_table.rs`）に直した誤りは 0**。応答の値は変えていない。

### 2. テスト（実測）

- **全体: 113 test result・1,526 passed・0 failed・0 ignored**（グループ分けの合算。1 回の通しではない）。基準（前回の統合 112・1,491）に対し **+35 = `tests/gdi_bulk.rs` 35 件**（主差分 34 + stretchblt 1）。`gdi.rs` は 96 件のまま（変更した 4 件を含め通過）。binary ごとの比較（前回の `per-binary.tsv` と今回）で、変わったのは `gdi_bulk` の追加だけ。84 → 85 の test binary は `tests/*.rs` と一致。`No space left`／`Disk quota exceeded` は全ログで 0 件。
- `tests/vcall_thunk_dll.rs` の 494 → **394** は**実測で確認**（生成 C の統計行 `unsupported_total 394`、テスト通過）。`fle_resp_count` 427、`fle_dispatch_count` 59,807（実測。算術どおり）。
- **初回の実行で `gdi_bulk` は 13 件が落ちた。原因はすべてテスト側の欠陥で、実装の欠陥ではなかった**（第 1 段階の「既知の弱点」②が当たった）。直したのは**テスト コードだけ**で、期待値の「値」は変えていない（3 は誤記の訂正）:
  1. **`stops()` が `setup` を親プロセスで実行していた**（12 件）。`fork` した子は別の tid を持つので、親が runtime の関数を呼んだ後に子が呼ぶと、単一スレッド規則（決定 0017）の `called from a second thread, not supported` で止まり、期待した理由の語が出ない。**直し: `setup` を子ごとに子の中で実行する**（`STOP("label", <setup> <stmts>)`）。ハンドルの割り当ては決定的なので、各子は親で実行した場合と同じ状態になる。
  2. **`reap()` が 1 回だけ `read` して読み端を閉じていた**（直し 1 の後に 2 件）。`fle_stop` の 1 行が長いと `fprintf` が分割して書き、子が閉じられた読み端に書いて `SIGPIPE`（13）で死ぬ（期待は `SIGABRT`（6））。**直し: EOF まで読む**（バッファを超えた分は捨てる）。
  3. `pat_blt_paints_with_the_brush_or_black_or_white` の期待値の**誤記**: WHITENESS の行を `5555555555ffff00ffffff00…`（8 バイト目までのずれ）と書いていた。決定（32 ビットの未使用バイトは FillRect／PatBlt が 0）どおりの `55555555ffffff00ffffff0055555555` に訂正（2 行）。実装の出力は決定どおりだった。
- 直した後、`gdi_bulk` 35 件は通った（トライアルで 1 回、全体テストの g06 で 1 回）。ログ: `logs/tests/trial1-gdi_bulk-before-fix.txt`（13 件の失敗）、`trial2-gdi_bulk-sigpipe.txt`（2 件）、`g06.txt`。
- 実行時間の注: `gdi_bulk` は 1 回約 200 秒（停止ケースごとに clang で 1 本ずつコンパイルする。他と並行すると 500 秒）。

### 3. 変異検査（20 件を実行。20 件検出。`logs/mut/`）

決定の「変異検査の案」(40 件) のうち 20 件（#1、7、8、9、10、12、14、17、18、21、22、26、28、29、32、34、35、36、37、39）を、`FLE_GDI_SRC_DIR` に変異させた `fle_gdi.c`／`.h` を置いて `--test gdi_bulk` で実行した（1 件ずつ変更 → 該当テストが落ちる → 元の木は変えない）。**20 件とも、案が「落ちるはず」と書いたテストで検出された**（#28 は 6 件、#29 は 7 件、#37 は 2 件が落ち、他は 1 件）。検出されなかったものは**無かった**。#21 の最初の変異（`count` を使わなくなる）は `-Wunused-variable -Werror` でコンパイルできず全テストが落ちたので**無効**として捨て、コンパイルできる変異（`n = (uint32_t)(n | 0u)`）に替えて 1 件の検出を確認した。残りの 20 件（#2〜6、11、13、15、16、19、20、23〜25、27、30、31、33、38、40）は**未実行**（#38 は Rust の応答表の入れ替え）。

### 4. FLEngine の生成 C（註釈なし。`fe-lift lift FLE OUT.c --pdata`）

| 項目 | HEAD（`94fe0c85…`） | 統合 |
|---|---|---|
| ファイルの SHA-256 | `94fe0c853537cefbcbc674b65e77c79464b8659a2c250b46dc762f6bf2ddc6b6`（HEAD で**再導出して一致**） | **`508b76fb15153de73e03dd9f7813b7f77cc0122b105a2f5d728b93737939263b`** |
| バイト数 | 215,751,676 | 215,771,656 |
| functions／bad_nodes／unsupported_total | 59,380／0／494 | 59,380／0／**394** |
| c_lines | 2,886,369 | 2,886,657 |
| `fle_resp_count`／`fle_dispatch_count` | 331／59,711 | **427／59,807** |
| 註釈あり（`flengine-hook-points.txt`）の SHA-256 | `ce205cfe…`（再導出して一致） | `6fd3a82e88c46e8f40ed5479a3298a225f71fa9ecc6a7eb4e7f41036874c0426` |

- **関数単位の比較（本体が変わった関数）: 100。すべて G1 の API の thunk**。100 関数の本体はすべて `L_xxxx: { /* 0xxxxxx Jmp */ { void fle_imp_<API>(void); fle_imp_<API>(); return; }}` の形（以前は `fle_unsupported`）。API 名は **96 種類で `rows-order.tsv` の 96 行と一致**（ExtTextOutW、GetRandomRgn、GetTextExtentPoint32W、DrawTextW が 2 関数ずつ）。ほかの変化は、プロトタイプ、`fle_dispatch_table`、`fle_resp_table` の節と 2 つの個数だけ。**G1 の API の thunk 以外の関数は 1 つも変わっていない**（予測どおり）。
- **他のモジュール**（a4 の引数と同じ種・註釈。関数単位）: dsp_ippv2 **0**、Blend2D **0**、freetype **0**、QFC **39**、ILTools **97**（予測どおり）。応答表の節（全モジュール共通）は増える。
- `harness/a4-multi.sh` の既定 `EXPECT_FLE_SHA`: `94fe0c85…` → **`508b76fb…263b`**（註釈なしの値。測定には註釈ありの `6fd3a82e…` を渡した）。

### 5. 6 モジュール構成の a4（native。註釈あり。測定用の基底と種）— 配置なし（M0）

(決定 0110 の第 2 段階の結果と同じ測定。配置ありは 0110 の文書を参照。)

```
a4: init_count=246 of 942, last_unit_proc=0x733090
a4: last_unit index=245 run_rank=209 of 895 proc=0x733090 state=in_progress
a4: runnable_started=209 completed=208 in_progress=1 of 895 runnable
a4: unit[init_count-1] proc=0x733090; units_outside_image=0
a4: entry_check started_but_not_entered=0 runnable_entered_beyond_init_count=0
a4: all_units_done=no functions_distinct=3411 function_entries=134085
```

| | W6 統合時（基準） | G1 統合（M0） |
|---|---|---|
| init_count | 246 / 942 | **246 / 942** |
| 停止 | `fle: unsupported instruction at 0x601da30`（`gdi32!MoveToEx`、未応答 import） | **`fle: StretchDIBits: this pixel transfer or read-back is not one of FillRect, PatBlt and BitBlt(SRCCOPY) between DIB sections: the conversions and the bytes GDI writes are not on the pages, and a device-dependent bitmap keeps no pixels (decision 0111)`** |
| 分類 | 未応答 import | **名前付き停止**（区分 C の「画素の転送・読み戻し」。ILTools の thunk 0x601dc90） |
| functions／entries／depth | 3,377／133,965／36 | 3,411／134,085／32 |
| 終了コード | 134 | 134（SIGABRT） |

- C スタック（gdb `bt` の `f_*`）: `fle_imp_StretchDIBits ← f_601dc90 ← f_61eb000 ← f_6130190 ← f_625ee30 ← f_625ef30 ← f_6262b60（CreateCursorFromFont）← f_625ed40 ← f_625cfc0 ← f_622aa30 ← f_622ab90 ← f_600da60 ← f_600db00 ← f_60186d0 ← f_62b3610 ← f_62b35f0（ILTools DllMain）← fle_realmod_load ← fle_imp_LoadLibraryW ← f_423990 ← f_44db40 ← f_6539e0 ← f_815e10 ← f_814e60 ← f_812e80 ← f_732f30 ← f_733090 ← f_4120c0 ← f_412160 ← f_41d8f0 ← f_1376840 ← f_1376820`（`MoveToEx` の停止のスタックと同じ道。`f_61eb000` が新しい）。
- **G1 の API の呼び出し実績（gdb。応答表の関数にブレークして呼び出し順に記録。`logs/gdb/m0-trace.tsv`、`m0-g1-calls.tsv`。停止までの全呼び出し）**。`CreateCursorFromFont` の後半（測定矩形 0 → `CreateBitmap(0, 0, 1, 1)` → `CreateCompatibleDC`）の続き。引数は rcx rdx r8 r9。

| # | API | 引数（読み） | 戻り値 | 区分 |
|--:|---|---|---|---|
| 453 | CreateBitmap | (0, 0, 1, 1, NULL) | 0x7fba00bc | 既存（0106） |
| 454 | CreateCompatibleDC | (NULL) | 0x7fba00d0 | 既存（0108） |
| 455 | **MoveToEx** | (dc, 0, 0, NULL) | 1 | **G1（A）** |
| 456 | SetBkColor | (dc, 0xffffff) | 0xffffff | 既存 |
| 457 | GetSysColor | (8) | 0 | 既存 |
| 458 | SetTextColor | (dc, 0) | 0 | 既存 |
| 459 | SelectObject | (dc, 0x7fba00d8) | 0x7fba00c0 | 既存 |
| 460 | GetSysColor | (8) | 0 | 既存 |
| 461 | SetTextColor | (dc, 0) | 0 | 既存 |
| 462 | **CreatePenIndirect** | (&LOGPEN) | 0x7fba00dc | **G1（A）** |
| 463 | SelectObject | (dc, pen) | 0x7fba00c0 | 既存（ペンの選択は G1 が足した） |
| 464 | **SetROP2** | (dc, 13) | 13 | **G1（A）** |
| 465 | CreateBrushIndirect | (&LOGBRUSH) | 0x7fba00e0 | 既存 |
| 466 | **UnrealizeObject** | (brush) | 1 | **G1（A）** |
| 467 | SelectObject | (dc, brush) | 0x7fba00c0 | 既存 |
| 468 | SetBkColor | (dc, 0xffffff) | 0xffffff | 既存 |
| 469 | **SetBkMode** | (dc, 2) | 2 | **G1（A）** |
| 470 | **SetStretchBltMode** | (dc, 3) | 1 | **G1（A）** |
| 471 | **GdiFlush** | () | 1 | **G1（A）** |
| 472 | **StretchDIBits** | (dc, 0, 0, 1, 1, 0, 0, 1, 1, lpBits=0x7ffff5d78600, lpbmi=0x7ffff5de6ec0, iUsage=0 (DIB_RGB_COLORS), rop=0xcc0020 (SRCCOPY)) | **停止（返らない）** | **G1（C）** |

  引数の 5 番目以降は停止の瞬間のスタックの 16 qword のダンプ（`STACKQ_at_stop`）から読んだ（上位 32 ビットの余りは無視）。G1 の A の 27 のうち**呼ばれたのは 7 種各 1 回**（MoveToEx、CreatePenIndirect、SetROP2、UnrealizeObject、SetBkMode、SetStretchBltMode、GdiFlush）。**呼ばれなかった**: GetCurrentPositionEx、SetMapMode、GetStretchBltMode、SetWindowOrgEx、GetWindowOrgEx、SetViewportOrgEx、LPtoDP、SetBrushOrgEx、GetBrushOrgEx、SaveDC、RestoreDC、GetClipBox、CreateSolidBrush、GetPaletteEntries、DestroyIcon、DestroyCursor、GetIconInfo、SetCursor、GetCursor、ShowCursor（この停止までの範囲）。メモリ DC の `GetDeviceCaps` と `StretchBlt` も**呼ばれなかった**（既存の画面 DC の `GetDeviceCaps` 8 回、`GetStockObject` 9 回、`GetObjectW` 10 回は停止の前に呼ばれた）。ペンは `CreatePenIndirect` → `SelectObject`、ブラシは `CreateBrushIndirect` → `UnrealizeObject` → `SelectObject` の順（VCL の `TCanvas` の経路。第 1 段階の読みのとおり）。
- **第 1 段階の予測との対照**: 「停止の前に `MoveToEx`、`SelectObject`、`SetROP2`、`UnrealizeObject`、`SetBkColor`、`SetBkMode`、`GetStretchBltMode`、`SetBrushOrgEx` が呼ばれ、続く壁が `GetDeviceCaps`（メモリ DC）または `StretchBlt`」 → 当たり 6（MoveToEx、SelectObject、SetROP2、UnrealizeObject、SetBkColor、SetBkMode）、**外れ 2**（`GetStretchBltMode`、`SetBrushOrgEx` は呼ばれず、代わりに `SetStretchBltMode` と `GdiFlush`）、**壁の予測は外れ**: 実際の壁は **`StretchDIBits`**（1×1、SRCCOPY、DIB_RGB_COLORS）。`TBitmap` の描画は、メモリ DC の `GetDeviceCaps`＋`StretchBlt` ではなく、`SetStretchBltMode`→`GdiFlush`→`StretchDIBits` の経路だった（読み。`f_61eb000` が呼び出し元）。
- 停止の `StretchDIBits` が転送先とする DC は `CreateCompatibleDC(NULL)` で作ったメモリ DC。ビットマップを選ぶ `SelectObject` はこの停止までに**確認していない**（選ばれていれば作成時のプレースホルダのまま＝画素を持たない DDB、と読む。停止の理由の「device-dependent bitmap keeps no pixels」と整合）。
- 再現性: 同じ native の再実行 2 回で結果は同じ（3 回）。**別ディレクトリで一からビルドして（gen 17 s、build 62 s）再測しても、a4 の 6 行・停止・functions／entries・depth・終了コードは同一**（native の SHA-256 先頭 `08f6bc15…` も同じ）。

### 6. 次の壁と PM に関わる点（第 2 段階で分かったこと）

1. **次の壁は `gdi32!StretchDIBits`**（名前付き停止）。1×1 の SRCCOPY、DIB_RGB_COLORS で、転送元は `lpbmi` と `lpBits`。DDB（画素を持たないプレースホルダ）への書き込みか、DIB セクションへの書き込みかで、決定 0108 の「DDB は画素を持たない」と 0111 の「画素は DIB セクションにだけ書く」の枠が問われる。**直していない**（新しい停止は記録するだけ）。
2. 6 モジュールの a4 の停止点は、**配置あり（決定 0110）では別の場所（freetype の `HeapReAlloc`）に先に当たる**（M1。`CreateCursorFromFont` の前半）。配置を採ると、この `StretchDIBits` の壁に着くのは `HeapReAlloc` の先になる。


## 範囲

- **答える**: メモリ DC の属性（位置、モード、原点、ブラシ原点）、DC スタック、ペンとソリッド ブラシの作成・選択・削除、パレット エントリの読み出し、アイコン／カーソルの破棄と情報とカーソルの状態、`GetClipBox`、DIB セクションへの FillRect／PatBlt／BitBlt(SRCCOPY)。
- **名前付きで止める**: 上の 69、および 0108 の 2 つ（GetDIBits、RealizePalette）。B の 3 つの引数が範囲外のとき。
- **範囲外**: `GetDeviceCaps` のメモリ DC（任意の差分）、`StretchBlt` の 1:1（任意の差分）、D の 180、`GetObjectW` のブラシとペン、領域・クリップ・メタファイル、ペンでの描画、画面 DC の属性、複数スレッド、wasm、実機との照合、次の壁より先の応答。

## 撤回条件（仮の決定）

| # | 仮の決定 | 撤回条件 |
|---|---|---|
| P1 | 新しい DC の既定値を Display Device Context Defaults から取る（D3） | Learn または実機が、メモリ DC の既定が違うと示したとき |
| P2 | SaveDC の番号 = 積んだ後の深さ（D8） | ゲストが番号の具体値に依存すると分かったとき（一次資料が値を与えたとき） |
| P3 | RestoreDC が色・位置・ブラシ原点を復元するか分からないので、違えば停止（D8） | 一次資料が復元の対象を列挙したとき。この停止が実際に起きたとき |
| P4 | 32 ビット DIB の未使用バイト = 0（FillRect、PatBlt）、BitBlt はコピー（D16） | PM の決定（既定は 0）または実機の確認 |
| P5 | 24／32 ビット DIB の画素 = 青、緑、赤（、未使用）（D16） | 一次資料が別の順を示したとき |
| P6 | BLACKNESS = 0x000000、WHITENESS = 0xFFFFFF（真色面）（D16） | 実機で違うと分かったとき |
| P7 | GetClipBox の可視領域 = 選択中のビットマップ（D10） | 領域の呼び出しを答えるとき |
| P8 | 初期のカーソルは無し、表示カウント 0（D15） | 実機の初期状態が分かったとき |
| P9 | GetIconInfo の hbmColor = 仮想画面の COLOR 形式（D14） | 一次資料が形式を書いたとき |
| P10 | 空の矩形は何も描かず 1（FillRect。D16） | 実機の戻り値が違うと分かったとき |
| P11 | 応答表の行の順序 | 統合の都合で並べ直すとき（擬似 VA が動く。決定 0015） |

## 実装した場所

差分は **`g1-gdi-bulk.diff`**（HEAD `3a1050a` の `lift/` の `git archive` に `patch -p1` で当たる。6 ファイル、3,149 行。HEAD の木で `patch -p1` を試し、適用後の木がワークツリーと `diff -r` で一致することを確認した）と、**`g1-gdi-bulk-extra-tests.diff`**（`tests/gdc.rs`、`tests/cursor.rs`、`tests/loadimage.rs` の「行が無い」リストの更新だけ。依頼が変更を許したパスの外なので別にした。3 ファイル、48 行）。

- `lift/runtime/fle_gdi.c`（`gdi_dc` と `gdi_dcsave`、`FLE_GDI_KIND_PEN` の選択・削除、`CreateCompatibleDC` の初期値 4 行、画素の補助関数（`dib_fill`、`dib_row`、`dev_rect`、`rect_clip`、`pixel_stop`、`pixel_dc`、`pixel_dib`、`pixel_brush_color`）、`BitBlt`／`PatBlt`／`FillRect` の本体の置き換え、新しい `fle_imp_*` 27 と名前付き停止 69（マクロ `GDI_STOP`）、検査関数 6）。`fle_gdi.h`（定数、宣言、検査関数）。**新しい .c／.h は作っていない。**
- `lift/crates/fe-lift/src/imports_table.rs`（末尾に 96 行とコメント 2 行）。
- `lift/crates/fe-lift/tests/gdi_bulk.rs`（新規。34 件）、`tests/gdi.rs`（「行が無い」リスト 3 か所と、0108 の source pin のテスト 1 か所）、`tests/vcall_thunk_dll.rs`（494 → 394）。
- **既存の関数の本体を変えた箇所と理由**: ① `fle_imp_CreateCompatibleDC`（新しい属性の初期値 4 行）、② `dc_destroy`（DC スタックの解放 1 行）、③ `fle_imp_SelectObject`（`case FLE_GDI_KIND_PEN` 1 行）、④ `fle_imp_DeleteObject`（ペンの種類 2 か所）、⑤ `fle_imp_BitBlt`／`PatBlt`／`FillRect`（PM が許可した例外。停止メッセージの先頭の引数の文字列は 0108 と同じ）。`GetObjectW`、`SelectPalette`、`CreateIconIndirect` ほかは変えていない。応答の値を変えた既存の API は、DIB セクションに対する FillRect／PatBlt／BitBlt だけ。

## テスト（作成して実行した。統合後 35 件が通った。初回は 13 件が落ち、テスト側の欠陥 3 つを直した。上の「第 2 段階の結果」の 2）

`tests/gdi_bulk.rs` **34 件**（主差分。任意の stretchblt の差分が 1 件足すので統合後は **35 件**）。C のテスト プログラムは 206 本（本体 23、停止のケース 180、生成 3）で、すべて `clang -fsyntax-only -Wall`（警告は定数変換 1 件のみ）を通した。停止の確認は `fork` で 1 本のプログラムの中に並べた（ケースごとに 1 本のコンパイルを避ける）。

| 区分 | 件数 | 検査すること |
|---|--:|---|
| 行・定数 | 2 | 96 行が決めた順で 1 回ずつ末尾にあり、宣言がある。older の行が前にある。行を足さない隣（CreatePen ほか 22 名）に行が無い。定数 |
| DC の属性 | 7 | 初期値、MoveToEx／GetCurrentPositionEx（前の位置、NULL、下位 DWORD）、4 つのモード（前の値、DC ごと）、モードの停止（無効値と画面 DC・NULL・削除済み・DC でない 4 通り × 5 API）、原点と LPtoDP（式、27 ビットの境界）、LPtoDP と原点の停止、ブラシ原点（0〜7 と停止） |
| DC スタック | 2 | SaveDC／RestoreDC（番号、相対、上の状態の破棄、DC ごとのスタック、選択数）、停止 7 通り |
| GetClipBox | 1 | プレースホルダ、DDB、原点、DIB、COLOR |
| オブジェクト | 4 | UnrealizeObject、CreateSolidBrush、CreatePenIndirect（7 つのスタイル、選択、削除）、GetPaletteEntries（クランプ、下位 DWORD） |
| アイコンとカーソル | 3 | GetIconInfo（新しいビットマップ 2 つ、モノクロ）、DestroyCursor／DestroyIcon、SetCursor／GetCursor／ShowCursor、停止 |
| 画素 | 8 | FillRect の 32 ビット、ボトムアップの 24 ビット、PatBlt の 3 つの rop、クリップと空の矩形、原点、システムカラー＋1 と BITFIELDS、BitBlt（コピー、クリップ、範囲外のソース）、重なり（memmove）、向きの違う DIB と 8 ビット、停止（PatBlt 19、FillRect 14、BitBlt 20） |
| メモリ DC の GetDeviceCaps | 1 | `fle_gdi_memdc_caps` を直接呼ぶ（任意の差分の下ごしらえ） |
| 名前付き停止・スレッド | 2 | 69 の停止（`fork`。名前、SIGABRT、理由の語、`(decision 0111)`）、27 の答えが第 2 スレッドで停止 |
| 生成器・ソース | 2 | 96 の thunk が runtime を呼ぶ・DLL 名と綴りが一致の一部、ソースの pin（書き込みは 2 か所、未使用バイト、`GDI_STOP` 70、弱い参照） |
| 計 | **34** | |

期待値は決定と Learn から書いた（C の出力を読み返していない）。数式で決まるもの（画素の並び、原点の式、スタックの深さ）は手で追った。**初回の実行で外れる可能性がある箇所**（第 2 段階で確認）: ① `dump` の行の並びとバイトの長さ、② `fork` した子の標準エラーの読み取り、③ `K` マクロの 12 引数配列、④ 生成器テストで `st.handled[...]` のキー。外れたら期待値ではなく C を疑う前に、この文書の規則と照合すること。

### 変異検査の案（40 件。第 2 段階で 20 件を実行し、20 件とも検出。上の「第 2 段階の結果」の 3）

| # | 変異 | 落ちるはずのテスト |
|--:|---|---|
| 1 | `SetBkMode` が新しい値を返す | `the_graphic_modes_return_the_previous_mode…` |
| 2 | 既定の背景モードを TRANSPARENT にする | `a_new_memory_dc_starts_with…`、modes |
| 3 | 既定の描画モードを 1 にする | 同上 |
| 4 | `SetMapMode` が 0 を受ける | `a_mode_the_pages_do_not_list…` |
| 5 | `SetStretchBltMode` が 5 を受ける | 同上 |
| 6 | `SetWindowOrgEx` がビューポートに書く | `the_window_and_viewport_origins…` |
| 7 | `LPtoDP` の符号を反転（`L + WO - VO`） | 同上 |
| 8 | `LPtoDP` が 2^27 を受ける | `lptodp_and_the_origin_calls_stop…` |
| 9 | ブラシ原点が 8 を受ける | `the_brush_origin_is_kept…` |
| 10 | `SaveDC` が 0 始まりの番号を返す | `save_dc_pushes_a_state…` |
| 11 | `RestoreDC(n > 0)` が 1 ずれる | 同上 |
| 12 | `RestoreDC` が上の状態を捨てない | 同上（triple） |
| 13 | `RestoreDC` がビットマップを戻さない | 同上（sel after） |
| 14 | `RestoreDC` が色の比較を省く | `restore_dc_stops_where…` |
| 15 | `restore_object` が `sel_add` を省く | `save_dc_pushes_a_state…` |
| 16 | `GetClipBox` が原点を引かない | `get_clip_box_gives…`（origins） |
| 17 | `UnrealizeObject` がストック ブラシを受ける | `unrealize_object_does_nothing…` |
| 18 | `CreateSolidBrush` が上位バイトを受ける | `a_solid_brush_gets…` |
| 19 | `CreatePenIndirect` がスタイル 7 を受ける | `a_pen_gets_a_gdi_handle…` |
| 20 | ペンの `DeleteObject` が `sel` を見ない | 同上 |
| 21 | `GetPaletteEntries` が個数をクランプしない | `palette_entries_are_copied…` |
| 22 | `DestroyCursor` が現在のカーソルを壊す | `icon_and_cursor_calls_stop…` |
| 23 | `DestroyIcon` が種類を消す（2 回目が停止になる） | `get_icon_info_makes_two_new_bitmaps…` |
| 24 | `GetIconInfo` が元のビットマップを返す | 同上 |
| 25 | `GetIconInfo` がアイコンを受ける | `icon_and_cursor_calls_stop…` |
| 26 | `ShowCursor` の初期値が −1 | `the_cursor_state_follows…` |
| 27 | `SetCursor` が任意のハンドルを受ける | 同上（停止） |
| 28 | `dib_fill` が未使用バイトに 0xFF を書く | `fill_rect_paints_the_half_open…` |
| 29 | `dib_row` の向きを逆にする | `a_bottom_up_dib_has_its_first…` |
| 30 | `rect_clip` が右下を 1 つ多く含める | `a_rectangle_is_clipped…` |
| 31 | `FillRect` が right／bottom を含める | `fill_rect_paints…` |
| 32 | WHITENESS が 0 を書く | `pat_blt_paints…` |
| 33 | `BitBlt` の行の向きを固定する | `bit_blt_inside_one_bitmap…` |
| 34 | `BitBlt` が形式の比較を省く | `fills_and_blits_stop…` |
| 35 | `PatBlt` が PATINVERT を受ける | 同上 |
| 36 | `dev_rect` の符号を反転 | `the_origins_of_the_dc_move…` |
| 37 | 名前付き停止の 1 つが `return` する | `the_sixty_nine_calls…` |
| 38 | 応答表の 2 行を入れ替える | `the_ninety_six_rows…` |
| 39 | 答えの 1 つで `fle_gdi_check_thread` を省く | `a_second_thread_stops_each…` |
| 40 | `GetPaletteEntries` が `start ≥ 16` で個数を返す | `palette_entries_are_copied…`（none） |

## 固定数テストの影響（式と根拠。第 2 段階で実測し、すべて式のとおりだった）

| 場所 | HEAD | この差分の後 | 式と根拠 |
|---|--:|--:|---|
| `tests/vcall_thunk_dll.rs`（`after_the_memory_form_jmp_only_the_import_thunks_remain_unsupported`）`unsupported_total` と `unsupported["Jmp: indirect through IAT slot (not lifted)"]` の 2 か所 | 494 | **394** | 494 − 100。100 = 96 行の FLEngine の import thunk（ExtTextOutW、GetRandomRgn、GetTextExtentPoint32W、DrawTextW が 2 つずつ）。FLEngine の `ff 25` の thunk を数える道具（0108 の `w6_fle_thunks.py` と同じ作り）で、HEAD の表 → 新しい表の「行の無い thunk」が **503 → 403**（差 100）。この道具の絶対値（503）は 0108 の測定（494）と 9 違う（0108 が「説明できない」とした差。参照されない 10 の thunk（DefMDIChildProcW ほか）が候補で、96 行のどれも含まない）ので、**差の 100 だけを使った**。`call [IAT]` 形は 0 |
| 応答表の行数（生成 C の `fle_resp_count`） | 331 | **427** | + 96。固定したテストは無い（`resp_count()` を使う `deferred.rs`、`modules.rs` は stats と比べる） |
| `fle_dispatch_count` | 59,711 | **59,807** | 関数 59,380 + 行 427。関数の数は変わらない（thunk の本体が変わるだけ） |
| 新しい行の擬似 VA | — | 0x7ff9000014c0〜0x7ff900001ab0 | `0x7ff900000000 + 0x10 * (k + 1)`、k = 331〜426。既存の行は動かない（末尾追記） |
| FLEngine の生成 C で本体が変わる関数 | — | **100** | 上の thunk。`jmp_imp:<API>` の handled が 96 キー（4 キーは 2） |
| ILTools の thunk | — | 97 | 96 行すべてを import。1 行が 2 つ |
| QFC の thunk | — | 39 | 答え 13、停止 26 |
| dsp_ippv2、Blend2D、freetype | — | 0 | import しない |
| `tests/gdi.rs` の「行が無い」リスト 3 か所 | — | 更新済み（差分に入れた） | GetTextMetricsW、GetSystemPaletteEntries、CreateSolidBrush、GetPaletteEntries、DestroyIcon、DestroyCursor、StretchBlt、ExtTextOutW、SetDIBits、StretchDIBits、SetBkMode に行ができた。新しい隣は CreatePen、GetCurrentObject、TextOutW、GetBkMode、GetSysColorBrush ほか |
| `tests/gdi.rs` の `the_source_of_decision_0108_pins…` | — | 更新済み | BitBlt、PatBlt、FillRect の本体は `pixel_stop(api` を含み、`fle_st(`、`memcpy`、`memset`、`->bits` を含まない（画素の書き込みは補助関数 `dib_fill` と BitBlt の `memmove` に限る） |
| `tests/gdi.rs` の 0108 の 12j（`bitblt_patblt_fillrect_getdibits_and_realizepalette_stop…`） | — | **不変（通るはず）** | 停止メッセージの先頭の引数の文字列（`rop 0xcc0020, 30 x 40 to (1, 2) from (3, 4)`、`rop 0xff0062, 30 x 40 at (1, 2)`、`hbr 0x1234`）を保った。画素を持たないプレースホルダの DC なので、新しい検査の「DDB」で止まる |
| `tests/gdc.rs`（224 行） | — | 更新（extra diff） | GetTextMetricsW、GetSystemPaletteEntries を「行が無い」リストから除く |
| `tests/cursor.rs`（268 行のリストと 634 行の「stay unsupported」） | — | 更新（extra diff） | SetCursor、GetCursor、DestroyCursor、ShowCursor を除き、634 行の SetCursor、DestroyCursor を GetCursorInfo、ClipCursor に替える |
| `tests/loadimage.rs`（327 行） | — | 更新（extra diff） | SetCursor、GetCursor、DestroyCursor、ShowCursor、GetIconInfo を除く |
| `harness/a4-multi.sh` の既定 `EXPECT_FLE_SHA`（FLEngine の C のファイル SHA-256、註釈なし） | `94fe0c853537cefbcbc674b65e77c79464b8659a2c250b46dc762f6bf2ddc6b6` | **`508b76fb15153de73e03dd9f7813b7f77cc0122b105a2f5d728b93737939263b`**（実測。更新済み） | 100 の thunk の本体が変わるため。第 2 段階が `fe-lift lift FLE OUT.c --pdata` で作り直して書き換える。a4 の測定が使う註釈ありの値も変わる |
| 他の 5 モジュールの生成 C | — | 変わる | 応答表の節（96 行）が全モジュールで増える。thunk の本体が変わるのは ILTools 97、QFC 39 |

任意の差分を当てる場合: `g1-optional-getdevicecaps.diff` は `tests/gdi.rs` の 1 か所（12a の `a_memory_dc_is_not_a_screen_dc_for_release_dc_and_get_device_caps`。`GetDeviceCaps(メモリ DC, 12)` が停止から 1 の返答に変わる）を更新済み。`g1-optional-stretchblt.diff` は `gdi_bulk.rs` の StretchBlt の語、`GDI_STOP` の数（70 → 69）、`gdi.rs` の pin のテストを更新済み。

## 検証（実施したこと。実施していないこと）

- **実施した（第 1 段階）**: (1) `clang -fsyntax-only -Wall -Wextra -Werror` を `fle_gdi.c`（-O0 と -O2）、`fle_gdc.c`、`fle_cursor.c` で。警告 0。(2) テストの C プログラム 206 本（optional の分を含めて 213 本）の `clang -fsyntax-only -Wall`。(3) `rustfmt --check --edition 2021` による構文の確認（`gdi_bulk.rs`、`gdi.rs`、`gdc.rs`、`cursor.rs`、`loadimage.rs`、`vcall_thunk_dll.rs`、`imports_table.rs`。構文エラー 0。整形の差は既存の行にもあり、見ていない）。(4) 既存の `tests/gdi.rs` 節 7 の禁止語・`long` の検査の Python での再現（0 件）。(5) HEAD の木への `patch -p1`（主差分と extra）と、ワークツリーとの `diff -r` の一致。任意の差分 2 つがワークツリーに当たることと、当てた `fle_gdi.c`／`fle_gdc.c` の構文。(6) Learn の取得と読み（約 100 ページ。新規 70 ページ前後）、ILTools の逆アセンブル、静的な閉包の計算（L1、L2）、FLEngine／ILTools／QFC の import と thunk の走査。
- **第 2 段階で実施**: `cargo build`、リンク、全体テスト（1,526 passed）、変異検査 20 件、FLEngine の生成 C の作り直しと SHA、他モジュールの関数単位の比較、6 モジュール構成の a4 と gdb。**まだ実施していない**: 変異検査の残り 20 件、census、実機 Windows との照合（0 件）、wasm のビルド。
- **既知の弱点**: ① `fork` を使うテストは、子が `abort` するときの標準エラーを読む。コアダンプの設定で時間がかかる可能性（`ulimit -c 0` を使うかは第 2 段階の判断）。② `tests/gdi_bulk.rs` の期待値は手で追った（第 2 段階の初回で外れた 13 件は、実装でも決定でもなく、テスト コードの欠陥 3 つだった）。③ 閉包の計算は vtable を辿らない下限。

## 出典なしの選択の一覧（凡例: 選択 = 出典なし、読み = 出典あり）

| # | 値 | 区分 | 備考 |
|--:|---|---|---|
| 1 | `SaveDC` の最初の戻り値 = 1（積んだ後の深さ） | **選択** | ページは「identifies the saved state」。ゲストは番号を `RestoreDC` に返すだけと読む |
| 2 | 32 ビット DIB の未使用バイト: `FillRect`／`PatBlt` は **0**、`BitBlt` は**コピー** | **選択**（PM の既定） | 実機の確認は未 |
| 3 | 24／32 ビット DIB の画素のバイト順 = 青、緑、赤（、未使用） | 読み | Learn に直接の文が無い（RGBTRIPLE／RGBQUAD の並びと BI_BITFIELDS のマスクから） |
| 4 | BLACKNESS = 0x000000、WHITENESS = 0xFFFFFF | 読み | 「index 0／1 in the physical palette（既定のパレットでは黒／白）」を真色面に当てた |
| 5 | 新しい DC の既定値（OPAQUE、R2_COPYPEN、MM_TEXT、BLACKONWHITE、(0,0) ほか） | 読み | Display Device Context Defaults を「normal default values」と読む |
| 6 | `SetCursor` の最初の「前のカーソル」= NULL、`GetCursor` の最初 = NULL | 読み | 何も設定されていない世界。ShowCursor の初期カウント 0 は Learn（マウスあり） |
| 7 | `LPtoDP` の 27 ビット境界 = [−2^27, 2^27) | 読み | 「exceed 27 bits」 |
| 8 | 空の矩形（FillRect）は何も描かず 1、PatBlt／BitBlt の幅 0 も 1 | 読み／選択 | FillRect は半開区間の定義から。PatBlt／BitBlt の 0 は選択 |
| 9 | `GetClipBox` の可視領域 = 選択中のビットマップ | 読み | メモリ DC のページ |
| 10 | `GetIconInfo` の hbmColor = 仮想画面の COLOR 形式（32 ビット、1 プレーン） | 読み | ページは形式を書かない |
| 11 | `RestoreDC` が復元するもの（オブジェクト 5 種、4 つのモード、2 つの原点） | 読み | 「selected objects and graphic modes」＋原点はマッピングの平行移動 |
| 12 | `RestoreDC` の範囲外の番号は 0 | 読み | ページの失敗値 |
| 13 | `UnrealizeObject`(パレット) = 1 | 読み | RealizePalette を答えないので巻き戻す状態が無い |
| 14 | `FillRect` の `hbr == 9`（COLOR_WINDOWTEXT + 1）= 黒 | 選択 | 0108 D11 の延長 |
| 15 | `DestroyCursor(NULL)` = 0、破棄済みのハンドル = 0 | 読み | 無効なハンドルは失敗（0108 の DeleteObject と同じ扱い） |
| 16 | 定数の数値（OPAQUE 2、R2_ 1〜16、MM_ 1〜8、BLACKONWHITE 1 ほか、PS_ 0〜6、SRCCOPY ほか） | UNOFFICIAL | mingw-w64 の wingdi.h |
| 17 | 名前付き停止 69 の分類（どれが「描画」「画素」か） | 選択 | 依頼の C の定義に照らした作業者の分類 |

## 解釈が分かれた点

1. **「LineTo の現在位置」**: 現在位置の設定・照会（MoveToEx、GetCurrentPositionEx）を A、LineTo は線を描くので C にした。LineTo が現在位置を進める効果だけを答える案もありうる（描画を省く: 偽の成功）ので採らない。
2. **Display Device Context Defaults をメモリ DC に当てる**（上の 1）。表は表示 DC のもの。
3. **D の定義**: 依頼の「DllMain の連鎖で到達しないと読める」を、静的な閉包（L1、L2）に入らない、と置いた。閉包は vtable を辿らない下限で、MoveToEx の実測が示すとおり**到達しないと言い切れない**。L2 に入る D の 54 は止まる形が `unsupported instruction` のまま。
4. **領域を C に置いた**（作らない案ではなく名前付き停止）: `BitBlt`／`PatBlt`／`FillRect` の「クリップ無し」を保つ条件（クリップ領域が作れない）を、停止の側で保証するため。
5. **RestoreDC の保守的な停止**（色・位置・ブラシ原点）: 実機は全属性を復元すると読むが、ページが書かないので止める。実際に起きたら、ゲストの使い方を見て復元側に倒す。
6. **`GetDeviceCaps` と `StretchBlt` を任意の差分にした**（上の 2）。
7. **`DestroyCursor` に共有ハンドルを渡す**: ページは「してはならない」と書き結果は書かない: 停止。
8. **`GetPaletteEntries` の NULL の文**: ページは hpal と pPalEntries を取り違えて書いている（「handle が NULL ならエントリ数」）。どちらの NULL も停止。
9. **ソースの範囲外の BitBlt**: ページは「clipping on the destination only」としか書かない。ソース矩形がソースのビットマップに収まらなければ停止。
10. **書式**: 依頼文の `fle: <API>: <理由>` に `(decision 0111)` を足した（0108 の画素の 3 つの停止は先頭の引数の文字列を保つ）。
11. **`GetTextMetricsW` を C にした**: 依頼の A の例に「GetTextMetrics」が挙がっていた。TEXTMETRIC の値（tmHeight、tmAscent、tmAveCharWidth ほか）は選択中のフォントのデータと GDI のフォント マッパー（丸めとヒンティング）で決まり、Learn は各メンバーの意味を書くだけで値を導く規則を書かない。仮想フォント（決定 0066 の "Segoe UI"）のファイルからメトリクスを計算する案はあるが、「実機の仕様から導ける範囲」に入らないと読んだ。`SetTextAlign` ほかの照会は ILTools が import しない（行を足さない）。

## 未決の点(PM)

1. **`g1-optional-getdevicecaps.diff` を当てるか**（**PM の決定: 当てる。第 2 段階で当てた**。ただし連鎖では**呼ばれなかった**ので効果は未観測）（読みでは `TBitmap.Draw` の経路が最初に踏む壁）。当てなければメモリ DC の `GetDeviceCaps` は 0108 の停止のまま（`not a live DC`）。
2. **`g1-optional-stretchblt.diff` を当てるか**（**PM の決定: 当てる。第 2 段階で当てた**。連鎖では `StretchBlt` は**呼ばれなかった**）（`StretchBlt` の 1:1 SRCCOPY を BitBlt と同一視する。依頼の例外は「BitBlt（SRCCOPY）」で StretchBlt を含まない）。当てなければ `StretchBlt` は名前付き停止。
3. **Display Device Context Defaults で 0108 の D3 を正式化するか**。正式化するなら、既定のブラシ = WHITE_BRUSH（白）、ペン = BLACK_PEN、フォント = SYSTEM_FONT、パレット = DEFAULT_PALETTE と置け、`PatBlt(PATCOPY)` が既定のブラシで通る。今回は触らない。
4. **L2 に入る D の 54 に名前付き停止の行を足すか**（`unsupported instruction at 0x…` の形を避けたいとき）。分類表の `closure_L2 = yes` の行で機械的に足せる。足すと FLEngine の thunk の数と固定数テストがさらに動く。
5. **32 ビットの未使用バイト**: 既定の 0（FillRect、PatBlt）と BitBlt のコピーでよいか。
6. **RestoreDC の保守的な停止**でよいか（色・位置・ブラシ原点が違えば停止）。
7. **`GetTextMetricsW` を A に上げるか**。L1 の深さ 3 で届く（読み）ので、unit 初期化の経路が実際に踏むかもしれない。上げるには、仮想フォントのメトリクスの出どころ（フォント ファイルの hhea／OS/2 から計算するなど）と丸めの規則を PM が決める必要がある。
8. **次の壁の予測**（第 1 段階の読み）: `GetDeviceCaps`（メモリ DC）→ `StretchBlt` → その先（`GetTextMetricsW` は L1 の深さ 3 で届く: フォントのメトリクスが要る）。**実測は外れ**: 次の壁は `StretchDIBits`（上の「第 2 段階の結果」の 5、6）。`StretchDIBits` を答えるか、名前付き停止のままにするかは未決（新しい項目）。

## 限界

- **第 2 段階で実行した**: コンパイル、リンク、テスト、変異検査 20 件、a4 の測定。**未実施**: 変異検査の残り 20 件、census、実機 Windows との照合。
- **連鎖で効いたのは 7 種**: 27 の答えのうち、6 モジュールの a4（配置なし）で停止までに呼ばれたのは 7 種（上の「第 2 段階の結果」の 5）。残り 20 は連鎖の中では**未実行**（単体テストと変異検査だけ）。実測の停止点は `StretchDIBits` で、第 1 段階の予測（`GetDeviceCaps`／`StretchBlt`）は外れた。
- **閉包は下限**（vtable を辿らない）。D の 180 のうち、実際に踏むものがあれば `unsupported instruction` で止まる。
- **画素の操作は最小**: DDB への書き込み、8 ビット以下・16 ビットへの塗り、クリップ、DIB の読み戻しは無い。
- **実機 Windows との照合は 0 件**。Learn の版（2024-02-22 ほか）が実機の挙動と一致するかは確認していない。
- 任意の差分は**主差分とは別に**テストした（構文だけ）。

## 測定の道具と成果物

`lift/out-git/g1-results/`: `g1-gdi-bulk.diff`（主）、`g1-gdi-bulk-extra-tests.diff`、`g1-optional-getdevicecaps.diff`、`g1-optional-stretchblt.diff`、`0111-gdi-bulk.md`（本書）、`README-0111-line.txt`、`MERGE-NOTES.md`、`gdi-bulk-classification.tsv`、`learn/`（新規に取得した Learn のページの本文。`w6_fetch.sh`／`w6_html2txt.py` の出力）、`tools/`（分類と閉包の計算のスクリプト）。
