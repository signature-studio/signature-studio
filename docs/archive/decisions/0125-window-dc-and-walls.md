# 0125: 窓の DC の状態モデル（GetDC／GetWindowDC／GetDCEx／ReleaseDC／BeginPaint）、hdc=0 の属性呼び出し、Blend2D の JIT 無効、`LoadCursorW` の名前引きと `CreateFruityInstance` の壁 52〜55。停止条件 (d)（壁 55 `FillRect`）で停止。すべて仮の決定

状態: **仮の決定（すべての応答と選択）**。依頼は PM（決定 0125 の内容は PM の仮の決定。番号 0125 は PM の割り当て）。測定・実装・テスト・文書は作業者が行った。作業は HEAD 4ff4f33 の `git archive`（`src/`）で始め、clean な写しを `clean/` に置いた。作業の途中でリポジトリの HEAD は 7149a62（検証用サブエージェントの文書だけ。ソースの変更なし）まで進んだ。差分 `fruity10.diff` は 4ff4f33 と 7149a62 の両方の clean な写しに `patch -p1` で当たる（dry-run、オフセットなし、拒否なし。「検証」）。

**肯定的に見せる点は限られる。** 壁 52〜55 の 4 壁を通し、`CreateFruityInstance` は 8,651,367 入口から **11,445,924 入口**まで進んだ（新規の関数 2,671 → 3,049）。しかし **(a)（`CreateFruityInstance` が戻る）にも (b)（空キューのメッセージ待ち）にも到達していない**。止まったのは停止条件 **(d)**（最初の新しい描画）で、**G2（主窓 "FL Studio" の可視化）は未達成**（主窓は visible 0 のまま、`ShowWindow`／`SetWindowPos(SWP_SHOWWINDOW)` は 1 回も呼ばれずに壁 55 に着いた）。実機 Windows との照合は 0 件。wasm・ブラウザは未測定。

区分の凡例は決定 0123 と同じ。**実測**＝コマンドの出力を見た事実。**読み**＝逆アセンブルやコードを読んだ上での解釈。**Learn**＝Microsoft Learn のページ（取得日 2026-10-31。ページと SHA-256 は `learn/index.tsv`、21 行）。**出典なしの選択**＝ページにも実測にも根拠が無く、作業者または PM が選んだ値（名前は `S-…`。撤回条件つき）。**非公式**＝Microsoft 以外の出典。**未確認**。

## 先に結論

| 項目 | 内容 |
|---|---|
| 全体テスト | `cargo test --workspace --no-fail-fast --offline` を**有効な 1 回の通し**（2026-10-31T12:16:07Z〜13:01:53Z、45 分 46 秒、終了 0）で **127 test result・1,839 passed・0 failed・ignored 0**。0123 の担当の木（基準: 125・1,799）との差は **+2 バイナリ・+40 件**で、内訳を実測した: `tests/wall_fr10.rs`（新規、20 件）と `tests/wall_fr9t.rs`（決定 0124 の 20 件。0123 の担当の木には無い）の 2 バイナリだけで、他のすべてのバイナリの件数は同じ（バイナリごとの突き合わせ）。中断した実行は 0。**全体テストが走っている間はセッションを終えていない** |
| FRUITY 無効の a4（独立 2 回） | 942/942、895/895、`returned from entry`、終了 0、functions **5,308**、entries **1,009,189**（基準と同じ）、native `d12809ae80ffcaa2f4070f34c32833902bf978840adfd831dea0a84a90577f63`（2 回とも同一） |
| FRUITY 有効の最終構成（独立 2 回） | 停止 `fle: FillRect: filling a rectangle with a brush (hbr 0x7fba1578): the bitmap of hDC is a device-dependent bitmap … it keeps no pixels (decision 0106)`（thunk `f_424b20`）= **停止条件 (d)**、**11,445,924 入口**、新規の関数 **3,049**、終了 134、スレッド **5**（0x1001 実行中、0x1002 `done`、0x1003 `waiting`、0x1004 `ready (not started)`、0x1005 `suspended`）、native `20d6442bb5728346ba3907f05842831b7f826e0d30fca2e463d5cd36c2f579ab`（2 回とも同一） |
| G2（主窓の可視） | **未達成**。主窓 0x7f990034 "FL Studio"（-5,-6,1930×1092）は visible 0、`ShowWindow` 未呼出のまま壁 55 に着いた。可視の最上位窓は 16×16 のヘルパー 0x7f990048（入口 7,059,458。0123 と同じ。PM は G2 に数えない） |
| 直した壁 | **4 壁**（52、53、54。壁 55 は止めた）。壁 52＝hdc=0 の属性呼び出し、壁 53＝Blend2D の JIT 組み立て失敗、壁 54＝`LoadCursorW(hInstance, 名前)`。**窓の DC は壁ではなく、PM の項目 1 に従った実装**（ゲストは壁 55 までに窓の DC を `BeginPaint` の 1 回しか使わない） |
| 出典なしの選択 | 新規 **18 項目**（下の表。撤回条件つき） |
| 生成 C | FLEngine 59,380 関数で変わった本体は **2**（`f_424c60` GetDCEx、`f_424f30` GetWindowDC。どちらも import thunk）。ILTools（m5）で 2（`f_601e180`、`f_601e3b0`。同じ 2 API の thunk）。dsp／Blend2D／QFC／FreeType は 0。関数の追加・削除は 0（`logs/gencmp-*.txt`） |
| 変異検査 | **12 件**（M1〜M11 と M4b。木のコピー `mut/`、別の `CARGO_TARGET_DIR`）。**12 件とも検出**。M4 は最初の実行が `-Werror` のビルドエラーで落ちたので数えず、M4b で作り直した（自己申告） |
| 時間 | 開始 09:25Z、全体テストの終了 13:01Z（約 3 時間 36 分）。上限 8 時間の約半分 |

## 背景

- 前段（決定 0123）: 壁 22〜51 の 30 壁を直し、壁 52 `gdi32!SetWindowOrgEx(hdc=0)` を直さずに停止条件 (g)（30 壁）で止めた。
- PM の決定 0125（仮、決定 0123 を引き継ぐ）: 出典のない値で止まらない。壁 52 の原因を特定し、DC の取得・解放を窓の DC として**状態だけ**でモデル化し、最初の実際の描画呼び出し（(d)）で止まる。止まる条件は (a)〜(e) と (g)（40 壁または 8 時間）。窓の DC は描画面を top-level 窓単位で持つ（`detach-window.md` R10）。PM の追加の判断（作業中に確認）: (d) の読みは「宛先が窓の DC である最初の描画呼び出し」と「決定 0111／0113 の DIB セクションの画素操作で足りない、新しい画素モデルが要る最初の描画」。既に答えているメモリ DC の DIB セクション宛ての `PatBlt`／`StretchDIBits` は答えたまま進めてよい。壁 52 の hdc=0 は戻り値 0・last error 6 で通す（last error 6 は出典なしの選択）。スレッド数の基準は 4（指示の 3 は誤り）。native SHA は `fle_thread_wasm.o` を外して再リンクしたとき `aba87edb…`（決定 0124 の受け入れ）。
- 土台の再現（実測。`logs/a4/base-off`、`base-on`。OUT は削除済み）: FRUITY 無効 942/942、895/895、5,308、1,009,189、native `b96f754b…`（`fle_thread_wasm.o` を外して同じ `.o` を再リンクすると `aba87edb…` に一致）。FRUITY 有効は壁 52（8,651,367 入口、新規の関数 2,671、スレッド 4）。検証用サブエージェント（HEAD 7149a62 の文書）の値と同じ。

## 壁の表

入口数と新規の関数は、各壁を通した後の測定（`logs/a4/w*/`。FRUITY 有効）。

| 壁 | 停止の内容 | 原因・規則の出典 | 種類 | 通した後の入口／新規の関数 |
|---|---|---|---|---|
| 52 | `gdi32!SetWindowOrgEx(hdc = 0)` | `f_820440`（`TCanvas` の原点の保存と復帰）が `SetWindowOrgEx(0, -x, -y, &prev)` と `SetWindowOrgEx(0, prev.x, prev.y, NULL)` を呼び、**戻り値を使わない**。DC が 0 なのは `f_6b9120`（TBitmap の画像初期化）が `[img+0x496]`（DIB なし）= 1 のとき `CreateCompatibleDC` も `CreateDIBSection` も呼ばない設計だから。**VCL の DC 生成漏れではない**（読み＋gdb の実測。下） | 出典なし（S-hdc-null） | 8,671,836／3,068 |
| 53 | Blend2D の JIT 組み立て失敗（`OutputDebugStringA` の `call [IAT]`。`bl::Pipeline::JIT assembling error: InvalidInstruction: pmullw xmm@255 ; pmullw s.uc0`） | Blend2D の JIT（AsmJit）が実行時に機械語を作ろうとして失敗している。決定 0120（PM の仮の決定）が対象とする状況そのもの。FRUITY 有効の構成に注釈 `blend2d-no-jit-sites.txt` を付ける（`fe-lift`／`fe-decode` の変更なし。`a4-multi.sh` の既存の第 7 欄） | 決定 0120 の適用（S-b2d-nojit） | 11,444,792／2,997 |
| 54 | `user32!LoadCursorW(hInstance = 0x400000, L"VT_HEADERSPLIT")` | Learn: hInstance が非 NULL なら名前または ordinal でそのモジュールのカーソル資源を引く。ゲストの Delphi `Screen` が自前のカーソルを名前で読む（`f_9f2870`） | Learn＋出典なし（S-loadcursor-handle、S-loadcursor-missing） | 11,445,924／3,049（壁 55 の停止点） |
| **55** | **`user32!FillRect`（宛先: メモリ DC の 9×9 カラーのデバイス依存ビットマップ）。止めた** | 決定 0106 のデバイス依存ビットマップは画素を持たない。新しい画素モデルが要る = **停止条件 (d)** | (d) | — |

### 壁 52 の原因（実測と読み）

1. **gdb での計数**（`tools/gdbtrace.sh`、`logs/gdbtrace-gdi-base-on.txt`、`gdbtrace-draw-base-on.txt`）: 壁 52 までに `CreateCompatibleDC` 165、`DeleteDC` 165、`CreateDIBSection` 55、`CreateBitmap` 55、`SelectObject` 660、`GetDC` 123（**すべて hWnd=NULL**）、`ReleaseDC` 123、`BeginPaint` 1（hwnd 0x7f990048、唯一の窓つきの DC）、`GetWindowDC`／`GetDCEx` 0、`PatBlt` 55、`StretchDIBits` 55（すべてメモリ DC の DIB セクション宛て）。
2. **呼び元の鎖**（gdb と逆アセンブル）: `f_820440` ← `f_7f00c0`（幅 22・高さ 99 の TBitmap 相当）← `f_127f550`。`f_7f00c0` は `f_6ce790`（引数 rcx=0x6b8330、edx=1、r8=0x16、r9=0x63）を呼び、`f_6b9120` が画像を初期化する。`f_6b93b0` は `[img+0x495]`==0 かつ `[img+0x496]`==0 のときだけ `CreateCompatibleDC(0)` を呼んで `[img+0x424]` に入れる。このとき `[img+0x496]`==1（呼び元がスタックで渡す `noDIB`）なので **`[img+0x424]` は 0 のまま**。`f_820440` は 0 の DC に対して `SetWindowOrgEx` を呼び、その結果レジスタを読まない。
3. したがって「`TCanvas.Handle` が 0 になる理由」は、**FL（VCL）自身が DC を持たない TBitmap の canvas に対して `SetWindowOrgEx` を呼び、失敗しても気にしない**ことで、実 Windows でも同じ呼び出しが起きるはずと読む（実機では未観測）。Learn の `SetWindowOrgEx`（`learn/SetWindowOrgEx.txt`）: 「If the function fails, the return value is zero.」。**NULL hdc を失敗とも書かず、GetLastError も書かない**。→ 失敗（0）は読み、last error 6 は出典なしの選択。
4. この結果、**窓の DC のモデルは壁 52 を通すために必要ではなかった**。PM の項目 1 に従い、窓の DC は別に実装した（下）。

### 壁 55 の停止（記録。PM の指示どおり (d) として止めた）

- 停止: `fle: FillRect: … (hbr 0x7fba1578): the bitmap of hDC is a device-dependent bitmap (or the 1-by-1 placeholder): it keeps no pixels (decision 0106) (decision 0111)`、thunk `f_424b20`、入口 11,445,924、新規の関数 3,049、終了 134。ゲストの呼び元の鎖 `f_5cb210` ← `f_5cc340` ← `f_5cf6b0` ← `f_5ceb80` ← `f_a067c0` ← …（`logs/stop-d/final-native.txt` の shadow call stack）。
- 引数と DC の属性（`logs/stop-d/dc-attrs.txt`、gdb で停止時に実測）: hdc 0x7fba156c（メモリ DC。直前の鎖は `GetDC(NULL)`→`CreateCompatibleDC`→`CreateCompatibleBitmap(9,9)`→`SelectObject`→`CreateBrushIndirect`）、矩形 (0,0,9,9)、hbr 0x7fba1578（BS_SOLID、色 0xffffff）。選択中のビットマップ 0x7fba1574 は **カラー（32 bpp、1 プレーン）のデバイス依存ビットマップで画素を持たない**。背景モード OPAQUE、ROP2 13、MM_TEXT、ストレッチ BLACKONWHITE、位置・各原点 (0,0)、文字色 0、背景色 0xffffff、4 つの既定オブジェクト。
- 窓表（`logs/stop-d/final-native.txt` の `windows:`、38 窓）: 可視の最上位窓は 0x7f990048（16×16）だけ。**主窓は visible 0**。直前 30 入口（import thunk）は `FillRect`、`LeaveCriticalSection`、`CreateBrushIndirect`、`EnterCriticalSection`、`SelectObject`、`CreateCompatibleBitmap`、`CreateCompatibleDC`、`GetDC`、…（`final-native.txt` の「last 48 entries into import thunks」）。
- **宛先は窓の DC ではない**。窓の DC が宛先の描画呼び出しには到達していない（ゲストは壁 55 までに窓の DC を `BeginPaint` の 1 回しか取らず、そこには描いていない）。窓の DC を宛先とする最初の描画呼び出しで止まる実装は入れた（下、テストで固定）。

## 窓の DC のモデル（状態だけ。ピクセルへの描画は書かない）

規則の出典は Learn（`GetDC`、`GetWindowDC`、`GetDCEx`、`ReleaseDC`、`Window-Class-Styles`、`Display-Device-Context-Defaults`）。**`Device-Context-Types` のページは 4 種（display、printer、memory、information）だけで、common／class／private の区別は書かない**。区別は `GetDC` のページが書く（「common, class, or private DC depending on the class style」）。

| 属性 | 実装（`fle_gdc.c`、`fle_win.c`、`fle_class.c`） | Learn の根拠／選択 |
|---|---|---|
| 種類 | `FLE_GDC_K_SCREEN`（hWnd NULL）、`COMMON`（クライアント領域の common DC）、`OWN`（CS_OWNDC の private DC）、`CLASS`（CS_CLASSDC）、`WINDOW`（`GetWindowDC` と DCX_WINDOW の窓全体） | `GetDC`「common, class, or private DC depending on the class style」 |
| common DC | 呼ぶたびに新しいハンドル、`ReleaseDC` で解放、属性は新しい `GetDC` で既定に戻る（`fle_gdi.c` が解放時に属性を捨てる） | `GetDC`「for common DCs, GetDC assigns default attributes … each time」、Display Device Context Defaults「Changes … are not permanent」 |
| CS_OWNDC | 窓ごとに 1 つ。同じハンドル。属性は保つ。`ReleaseDC` は 1 を返して DC は残る | Window Class Styles「a unique device context for each window」。`GetDC`「Class and private DCs do not have to be released」。**`ReleaseDC` の戻りは S-release-owndc** |
| CS_CLASSDC | クラスごとに 1 つ（atom で引く）。窓が違っても同じハンドル | Window Class Styles「one device context to be shared by all windows in the class」 |
| `GetWindowDC` | 常に common の窓全体の DC。ハンドルは毎回新しい | `GetWindowDC`「the entire window」「ReleaseDC must be called」。クラススタイルは書かれない（**S-windc-common**） |
| `GetDCEx` | DCX_WINDOW は窓全体の common DC、DCX_CACHE は OWNDC／CLASSDC を無視した common DC、それ以外はクライアントの DC。DCX_INTERSECTRGN／DCX_EXCLUDERGN は**クリップ領域が未モデルなので名前付き停止**。DCX_NORESETATTRS は無視（Learn「This flag is ignored」） | `GetDCEx`。数値は windows-sys（非公式、`logs/dcx-constants.txt`）。**S-gdcex-bits、S-gdcex-window、S-gdcex-parentdc** |
| 描画面（R10） | **top-level 窓ごと**。記録の `root_lid` = 最上位の祖先の**論理 id**（HWND でなく。窓の再作成=R5 で孤児にならない）。子窓は祖先の面、オーナーされた窓は自分の面、デスクトップは自分 | detach-window.md R10、R5。**面は画素を持たない** |
| 破棄 | `DestroyWindow` は CS_OWNDC の private DC を捨てる。common DC は残り、ゲストが `ReleaseDC` で解放する。class DC は残る | **S-owndc-life、S-common-after-destroy**（Learn は書かない） |
| `ReleaseDC` | 窓が違えば 0（DC は残る）、別のスレッドなら 0、解放済みなら 0、NULL の DC なら 0、画面 DC に非 NULL の hWnd は従来どおり停止 | `ReleaseDC`「If the DC was not released, the return value is zero」「must be called from the same thread」。**S-release-mismatch、S-release-thread** |
| `BeginPaint`／`EndPaint` | 窓自身の DC（クラススタイルに従う）。0123 の `GetDC(NULL)`（S-paintdc）から変更 | `BeginPaint`「a handle to a display device context for the specified window」 |
| 無効な hWnd | `GetDC`／`GetWindowDC`／`GetDCEx` は NULL と last error 1400（0123 の S-err と同じ選択） | 各ページ「If the function fails, the return value is NULL」 |
| 描画 | **ピクセルを書かない**。窓の DC を宛先とする最初の `BitBlt`／`PatBlt`／`FillRect`／`StretchDIBits`／`SetDIBitsToDevice` は `pixel_dc` が窓の DC と見分けて名前付きで停止（窓、種類、面の lid、「stop condition (d)」を含む） | PM の項目 1・停止条件 (d) |

## 出典なしの選択の一覧（撤回条件つき）

撤回条件は、特記がなければ「**実機 Windows での測定が別の値を示したとき**」。

| # | 場所 | 選択（値） | 理由 |
|---|---|---|---|
| S-hdc-null | `fle_gdi.c`（`attr_dc`） | 14 の属性呼び出し（`MoveToEx`、`GetCurrentPositionEx`、`SetBkMode`、`SetROP2`、`SetMapMode`、`Set/GetStretchBltMode`、`Set/GetWindowOrgEx`、`SetViewportOrgEx`、`LPtoDP`、`Set/GetBrushOrgEx`、`SaveDC`、`RestoreDC`）に hdc=NULL を渡すと戻り値 0、last error 6（`ERROR_INVALID_HANDLE`）。POINT には書かない | ページは NULL を失敗と書かず GetLastError も書かない。ゲストは結果を読まない。NULL 以外の非 DC は従来どおり停止。撤回条件: 実機で別の戻り・エラーが測られたとき、またはゲストが結果を読む呼び出しが見つかったとき |
| S-b2d-nojit | `harness/hooks/blend2d-no-jit-sites.txt`（`tools/run6.sh` の `SITES_B2D`） | FRUITY 有効の構成は Blend2D を JIT 無効で使う（決定 0120）。FRUITY 無効の基準の生成 C は注釈なしのまま | 決定 0120（PM）。撤回条件: 決定 0120 の撤回条件（画素が実機と異なる、など） |
| S-loadcursor-handle | `fle_res.c`、`fle_loadimage.h` | `LoadCursorW(hInstance, 名前)` のハンドル = `FLE_LI_MODULE_HANDLE_BASE + FLE_LI_REALMOD_STRIDE*k + 4*(0x100 + p)`（p は RT_GROUP_CURSOR の名前ディレクトリ内の位置、k=0 は lifted DLL、実モジュールは index+1）。ordinal 32762〜32767 は `LoadImageW` と同じハンドル | ページは値を書かない。同じ名前は同じハンドル（「retrieves the handle to the existing resource」） |
| S-loadcursor-missing | `fle_res.c` | 名前が無ければ 0、last error 1814（`ERROR_RESOURCE_NAME_NOT_FOUND`。`FindResourceW` の既存の選択と同じ）。32762〜32767 以外の ordinal は名前付き停止 | ページは失敗の値だけ書く |
| S-windc-common | `fle_gdc.c` | `GetWindowDC` は常に common の DC（新しいハンドル）で、クラススタイルは見ない | ページがクラススタイルに触れない |
| S-owndc-life | `fle_gdc.c`、`fle_win.c` | CS_OWNDC の private DC は窓の破棄とともに消える | ページは書かない（private DC は窓のもの、と読む） |
| S-common-after-destroy | `fle_gdc.c` | 窓の破棄後も、未解放の common DC は残り、ゲストの `ReleaseDC` で解放される | ページは書かない |
| S-release-mismatch | `fle_gdc.c` | `ReleaseDC(hWnd, hDC)` の hWnd が DC を取った窓と違えば 0（DC は残る） | ページは「the window whose DC is to be released」とだけ書く |
| S-release-thread | `fle_gdc.c` | 別のゲストスレッドで取った DC の `ReleaseDC` は 0（DC は残る） | ページは「must be called from the same thread」と書き、違反の結果は書かない |
| S-release-owndc | `fle_gdc.c` | private／class DC への `ReleaseDC` は 1 を返し、DC は残る | ページは「do not have to be released」と書き、呼んだ結果は書かない |
| S-gdcex-bits | `fle_gdc.c`、`fle_gdc.h` | `GetDCEx` のフラグの数値（DCX_WINDOW 1、DCX_CACHE 2、DCX_NORESETATTRS 4、DCX_CLIPCHILDREN 8、DCX_CLIPSIBLINGS 0x10、DCX_PARENTCLIP 0x20、DCX_EXCLUDERGN 0x40、DCX_INTERSECTRGN 0x80、DCX_LOCKWINDOWUPDATE 0x400）。ページに無いビット（ゲストは 0x10001 などを渡す）は記録だけで影響しない | Learn は名前だけ（数値は windows-sys の docs.rs、**非公式**、`logs/dcx-constants.txt`）。撤回条件: Learn が数値を載せたとき、または実機で未知のビットが影響すると測られたとき |
| S-gdcex-window | `fle_gdc.c` | DCX_WINDOW は DCX_CACHE の有無にかかわらず common の窓全体の DC（private／class の DC はクライアント領域のもの） | ページは両者の優先を書かない |
| S-gdcex-parentdc | `fle_gdc.c` | CS_PARENTDC のクラスの窓は common DC として扱う（Window Class Styles「receives a regular device context from the system's cache」） | `GetDCEx` のページは「returns a handle to a DC that belongs to the window's class if CS_CLASSDC, CS_OWNDC or CS_PARENTDC」と書き、**ページ同士が食い違う**。撤回条件: 実機で CS_PARENTDC の窓が親の DC を返すと測られたとき |
| S-paintdc（0123 から変更） | `fle_win.c` | `BeginPaint` の DC は窓自身の DC（クラススタイルに従う）。0123 は `GetDC(NULL)` だった | `BeginPaint`「for the specified window」 |
| S-err（0123 の流用） | `fle_gdc.c` | 無効な hWnd は NULL と 1400 | 0123 と同じ |
| S-surface | `fle_gdc.c`、`fle_win.c` | 面の記録は最上位の祖先の論理 id。オーナーされた窓は自分の面 | detach-window.md R10、R5。ページは面を語らない（ブラウザ側の制約） |
| S-default-surface-size | `fle_win.c` | デスクトップの DC の大きさは仮想ディスプレイ（1920×1080） | 決定 0053 の値 |
| S-shared-cursor-setcursor | `fle_gdi.c`、`fle_cursor.c` | `LoadCursorW(hInstance, 名前)` が返した共有ハンドルは `SetCursor` と `DestroyCursor`（1 を返す。決定 0113 の選択）で受ける | Learn `SetCursor`「loaded by either the LoadCursor or the LoadImage function」。`DestroyCursor` の戻りは決定 0113 の選択のまま |

件数: 上の表は **18 項目**（S-hdc-null、S-b2d-nojit、S-loadcursor-handle、S-loadcursor-missing、S-windc-common、S-owndc-life、S-common-after-destroy、S-release-mismatch、S-release-thread、S-release-owndc、S-gdcex-bits、S-gdcex-window、S-gdcex-parentdc、S-paintdc、S-err、S-surface、S-default-surface-size、S-shared-cursor-setcursor）。**S-gdcex-parentdc は Learn の 2 つのページが食い違う**ので未決。

## 既存のテストの更新（旧 → 新と理由。通すために弱めたものは無い）

| ファイル | テスト | 旧 | 新 | 理由 |
|---|---|---|---|---|
| `wall_fr4.rs` | 「9 行は最後の 9 行」 | `LATER_ROWS = 9 + 15 + 73` | `9 + 15 + 73 + 2` | 末尾に 2 行追加 |
| `wall_fr5.rs` | 同様 | `8 + 15 + 73` | `8 + 15 + 73 + 2` | 同上 |
| `wall_fr8.rs` | 「15 行は最後の 15 行」 | `n = rows.len() - 73` | `n = rows.len() - 73 - 2` | 同上 |
| `wall_fr9.rs` | 「73 行は最後の 73 行」 | `n = rows.len()`、`469 + 73` | `n = rows.len() - 2`（`469 + 73` は不変） | 同上 |
| `vcall_thunk_dll.rs` | FLEngine の未対応 thunk | 281 | 279（−2） | `GetWindowDC` 0x424f30 と `GetDCEx` 0x424c60 の thunk が答えを持った（生成 C の比較で測った） |
| `gdc.rs` | 「GetDCEx／GetWindowDC は行でない」 | 行が無いことを検査 | 行がちょうど 1 つあることを検査 | 決定 0125 が行を足した |
| `gdi_bulk.rs` | `attr_dc_refusals` | NULL を「not a DC of this runtime」で停止 | NULL を除く（`wall_fr10.rs` が失敗を検査） | S-hdc-null |
| `loadimage.rs` | 「資源の検査は最後のエラーに触れない」 | 関数 `fle_res_cursor_group_ok` の本体を検査 | 本体が移った static の補助関数 `cursor_group_ok_key` を検査 | 本体を補助関数に移した。検査の中身（last error に触れない、`img_open`・`find_leaf` を使う）は同じ |

## 実装した場所

- 新規: `lift/crates/fe-lift/tests/wall_fr10.rs`（**20 件**）、この文書。
- 変更: `runtime/fle_gdc.c`／`.h`（DC の記録、`GetWindowDC`、`GetDCEx`、窓つきの `GetDC`／`ReleaseDC`、`fle_gdc_dc_info`、`fle_gdc_window_destroyed`）、`fle_win.c`／`.h`（`fle_win_dc_query`、破棄の鎖の呼び出し、`BeginPaint`／`EndPaint` が窓の DC）、`fle_class.c`／`.h`（`fle_class_style_of`）、`fle_gdi.c`（`attr_dc`、窓の DC を見分ける `pixel_dc`、`SetCursor`）、`fle_cursor.c`、`fle_res.c`／`.h`（`fle_cursor_load_named`）、`fle_loadimage.h`（ハンドルの形）、`crates/fe-lift/src/imports_table.rs`（**末尾に 2 行**。既存の 542 行は不変、擬似 VA は動かない）、既存のテスト 8 ファイル。差分は **21 ファイル**。
- 変えていない: `fe-lift`／`fe-decode`／`fe-pe` の本体、`fle_thread.c`、`fle_thread_wasm.c`、既存の応答表の行と擬似 VA、`docs/README.md`。**ハーネスの `harness/` は変えていない**（`SITES_B2D` は `tools/run6.sh`＝`lift/` の外の作業用スクリプトの既存の変数）。

## テスト

新規 `tests/wall_fr10.rs`（**20 件**）。期待値は Learn の文言と決定の選択から書き、C のコードから読み戻していない。実ランタイムをリンクし、ゲストの窓プロシージャを C のスタブで置いた駆動プログラム、およびゲストの実 DLL を `image.c` で載せた駆動プログラムで検査する。

- 生成器（2）: 2 行が末尾に決めた順で並び擬似 VA が動かない（469 + 73 + 2 = 544 行）、各 thunk は自分の DLL でだけ答える。
- 壁 52（3）: 14 + 1 の属性呼び出しが hdc=NULL で 0・last error 6・POINT に書かない、NULL 以外の非 DC は停止のまま、ゲストの呼び出しの形（`SetWindowOrgEx(0, -5, -6, &prev)` が 0 を返し `prev` に書かない）。
- 窓の DC（9）: common DC（毎回新しいハンドル・解放・属性が既定に戻る）、private と class の DC、面（子は最上位の祖先、オーナーされた窓は自分）、`GetDC`／`ReleaseDC` の失敗値（無効な窓、窓違い、NULL の DC）、`GetWindowDC` と `GetDCEx` のフラグ、`GetDCEx` のクリップ領域フラグの停止、`BeginPaint`／`EndPaint` の窓の DC、窓の破棄後の common DC。
- 壁 54（3）: 実 FLEngine の RT_GROUP_CURSOR を名前で引く（同じ名前は同じハンドル、大小文字無視、無い名前は 0 と 1814、32649 は停止）、NULL の hInstance と他のモジュールの扱い、`SetCursor`／`DestroyCursor` が共有ハンドルを受ける。
- 壁 55 と (d)（3）: 窓の DC への最初の `PatBlt`／`FillRect` が窓・種類・面の lid を含めて停止、メモリ DC のデバイス依存ビットマップへの `FillRect` は従来の停止、画面 DC は従来の停止。
- ソース検査（1）: `fle_gdc.c` はホストの状態を読まず画素を書かない、各選択の名前が現れる。

## 検証（実測）

- **全体テスト**（`logs/full/`、`start.txt`／`end.txt`／`rc.txt`／`test-result-lines.txt`）: `CARGO_TARGET_DIR=lift/out-fruity10/target-full`（変異検査の `target-mut` とは別）。環境変数は `tools/env.sh`（`FLENGINE_PE`／`FLENGINE_DLL`／`ILTOOLS_DLL`／`BLEND2D_PE`（リベース済み）／`DSP_IPPV2_PE`（原本の un-rebased dsp）／`DSP_IPPV2_PE_2000000`／`FREETYPE_PE`／`FL64_EXE`）。ログの `test result` 行を自分で集計: **127 result・1,839 passed・0 failed・ignored 0**（`test result: FAILED` 0、終了 0、終了時に残った `cargo test` のプロセス 0）。**有効な通しは 1 回**。差の内訳（バイナリごとの突き合わせ）は「先に結論」。
- **FRUITY 無効の a4**（`logs/a4/FOFF1`、`FOFF2`。別の OUT、同じ最終の木）: 942/942、895/895、5,308、1,009,189、`returned from entry`、終了 0、native `d12809ae…77f63`（同一）。基準（clean の木、`base-off`）は `b96f754b…` で、差は生成 C の import thunk 2 本と runtime の変更による。**`fle_thread_wasm.o` を外した再リンクの SHA（`aba87edb…` 系）の再確認は、この最終の木では行っていない**（runtime が変わったので `aba87edb…` とは一致しない）。
- **FRUITY 有効の最終構成**（`logs/a4/FON1`、`FON2`）: 上の表。2 回とも同一の native SHA と同一の停止。1 回の所要は約 1 分 6 秒、最大 RSS 約 2.7 GB。functions 5,332、entries 1,009,338（a4 の部分）。
- **生成 C の比較**（`logs/gencmp-flengine.txt`、`gencmp-modules.txt`）: 旧 `fe-lift`（clean の木）と新 `fe-lift`（ワークツリー。SHA `4e4b2b10…`。旧は `36c06573…`）で、同じ入力・同じ引数から FLEngine と m1〜m5 を生成し、`void f_XXXX(void)` の本体ごとに SHA-256 を比べた。変わった本体は FLEngine 2、m5（ILTools）2、他 0。**注意**: m1〜m5 の比較は FRUITY 無効の基準の種と c1 で行った（FRUITY 有効の Blend2D 964 の種と c2 の構成では比べていない）。
- **変異検査**（`logs/mutations.txt`、`tools/mutate.py`。M1 hdc=NULL が 1 を返す、M2 last error を 5 に、M3 private DC が毎回新しいハンドル、M4b class DC を窓ごとに、M5 common DC を解放しない、M6 `ReleaseDC` が窓を見ない、M7 DCX_CACHE が OWNDC を上書きしない、M8 面を窓自身に、M9 `LoadCursorW` の名前引きを無効に、M10 破棄が private DC を残す、M11 すべてのクラスを CS_OWNDC 扱い）: **12 件とも検出**（変異ごとの失敗したテスト名つき）。最後に `diff -rq` で `mut/` と `src/` の `lift/` が同じ（`out-wallFR10`、`target` を除く）ことを確認した。
- **diff の dry-run**（4ff4f33 と 7149a62 の clean な `git archive` で `patch -p1 --dry-run`）: どちらも終了 0、21 ファイルとも当たる（オフセット・拒否・fuzz なし）。実際に当てた木の `lift/` はワークツリー `src/lift/` と `diff -rq` で差異 0（`docs/` は検証用サブエージェントの `verify-0122-0123-0124` が HEAD 側にだけあるため、`docs/` は差異がそれだけ）。

### 規則違反と誤りの自己申告

- **M4 の最初の実行はビルドエラーで落ちただけだった**（`-Werror`: 未使用引数）。検出として数えず M4b で作り直した。
- **`gdb` のレジスタ変数 `$sp` を上書きして自分のトレースを壊した**（`tools/gdbfn.sh` の初版。実プロセスの rsp を書き換えた）。変数名を変えて作り直した。測定値には使っていない。
- `tmp` の置き場: 最初に `src/lift/runtime` から `../../tmp` を相対パスで指定し、`src/tmp` を作りかけた（出力ファイルは作られずエラー）。`src/tmp` が空で残っていないことを確認して削除した。
- 全体テストと各テストが `src/lift/out-*` と `src/lift/target/insn-int-test/*.c` を木の中に作る（既存のテストの動作）。差分に混ざったので、`diff` の前に削除した（私の変更ではない。差分の最初の版には `lift/target/insn-int-test/*.c` の 5 ファイルが入っていたので作り直した）。
- 作業中に `cut`／`grep` の出力が大きく、端末に長い出力を出した場面がある。

## 止まった理由

停止条件 **(d)**（最初の実際の描画呼び出し）。ただし、**窓の DC を宛先とする描画ではなく**、メモリ DC の 9×9 カラーのデバイス依存ビットマップへの `FillRect` で止まった。判定（作業中）: これは新しい画素モデル（デバイス依存ビットマップに画素領域を持たせる）が要る描画で、この作業では作らない。(a)・(b)・(c)・(e)・(g) には当たっていない（壁 4 件は、40 壁にも 8 時間にも遠い）。

## 未決の点(PM)

1. **壁 55 の次の方針（PM が作業中に推奨済み）**: (A) デバイス依存ビットマップを 32 bpp の画素領域にする、(B) 描画 API をすべて別レーンの名前付き停止にする、(C) 描画を無視して成功を返す（偽りなので不採用）。推奨は (A) を描画レーンで。
2. **S-gdcex-parentdc**: Learn の `GetDCEx` と `Window-Class-Styles` が CS_PARENTDC で食い違う。common DC として扱った。
3. **S-hdc-null の last error 6**: 出典なしの選択。
4. **S-b2d-nojit**: FRUITY 有効の構成は Blend2D を JIT 無効で使う（決定 0120 の注釈を付けた）。FRUITY 無効の基準は注釈なし。構成によって Blend2D の振る舞いが違う。
5. **G2 は未達成**: 主窓は壁 55 まで一度も `ShowWindow` されない。壁 55 より先に進むのは描画レーンの後になる。
6. **native SHA**: FRUITY 無効の最終の native は `d12809ae…`。基準 `b96f754b…`（clean）、`aba87edb…`（`fle_thread_wasm.o` を外した 0123 の値）とは別。次の基準にするかは未決。

## 撤回条件

- 各 `S-…` の表の撤回条件（既定: 実機測定）。
- 窓の DC のモデル（状態だけ）: PM が描画面の持ち方（top-level 窓ごと）を別の方針にしたとき（R10 の撤回）。

## 限界

- 実機 Windows との照合は 0 件。wasm・ブラウザは未測定（`fle_gdc.c` の変更はネイティブだけで測った）。
- (a) にも (b) にも到達していない。G2 は未達成。
- ゲストは壁 55 までに窓の DC を `BeginPaint` の 1 回しか使わず、`GetWindowDC`／`GetDCEx` を 1 回も呼ばない。したがって**窓の DC のモデルの大半（private／class／窓全体／フラグ）は、ゲストの実際の呼び出しに対して測っていない**。テストは Learn の文言と選択だけから書いた。
- クリップ領域（`DCX_*RGN`）、可視領域、窓全体の DC の原点（非クライアント領域）は未モデル。
- 描画しない。メモリ DC のデバイス依存ビットマップは画素を持たない（決定 0106）。
- スレッド数 5 の内訳（0x1005 `suspended`）の意味は調べていない。
- 時間と RAM: 6 モジュールのビルドが約 1 分、最大 RSS 約 2.7 GB。全体テストは約 46 分。
