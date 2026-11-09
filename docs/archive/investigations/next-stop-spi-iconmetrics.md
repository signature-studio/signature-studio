# 次の停止点: user32!SystemParametersInfoW(SPI_GETICONTITLELOGFONT = 0x1f) の調査と判断材料（実装していない）

実行日: 2026-10-22。作業ディレクトリ `signature-studio-develop`、HEAD 0060382 に決定 0064/0065 の変更を載せた状態。「一次」は Microsoft Learn（2026-10-22 取得）、「実測」はコマンドの出力、「読み」は `lift/objdump-text.txt` と生成 C の静的な読み、「推測」は未検証。成果物は `lift/out-cb/`（gitignore 済み）。

**実装していない。** 理由: 決定 0058 が 0x1f と 0x29 を「フォントとスクロールバー寸法をまとめて決める決定が要る」ものとして名前付き停止にしており、その決定は PM の判断（フォントの方針）を要する。

## 1. 停止点（実測。native と wasm が一致）

```
stop   : fle: SystemParametersInfoW: uiAction 0x1f (SPI_GETICONTITLELOGFONT) is used by the guest but no value is decided (decision 0058)
depth  : init_count=115 / 942   last unit: index=114 proc=0x623840 in_progress   started=111 completed=110
funcs  : executed (distinct)=602  entries=13541
```

スタック: f_4255d0（thunk、guest-return 0x866429）← f_8663b0（0x865379）← f_8651a0（0x620dea）← f_620cd0 ← f_623840 ← f_4120c0 ← f_412160 ← f_41d8f0。

## 2. f_8663b0 の読み（`lift/objdump-text.txt` の 0x8663b0〜0x86660e）

f_8663b0（引数 rcx = オブジェクト。`[obj+0xfc]`、`[obj+0x10c]`、`[obj+0x114]`、`[obj+0x11c]`、`[obj+0x104]` にフォント用のスロットを持つ）は、グローバル `[0x15f7190]` が非 NULL なら `f_86b240(…)` を前後で呼ぶ（0x8663fe、0x866601）。本体:

1. `SPI(0x1f, 0x5c, &lf0 = [rbp+0x22c], 0)`（0x866424）。`LOGFONTW` 1 つ（sizeof = 0x5c = 92）。
   - 戻り値が非 0（0x86642d）: `f_423fc0(&lf0)`（= `CreateFontIndirectW`、IAT スロット 0x19719d0）の結果 `hfont` を `f_5c1360([obj+0x104], hfont)` に渡す。
   - 戻り値が 0（0x866451）: `f_4242e0(13)`（= `GetStockObject(13)` = SYSTEM_FONT。決定 0054 が答える index）の結果を同じ `f_5c1360` に渡す。
2. `f_425a50()`（0x425a10 の内部の判定で 0x1f8 か 0x1f4 を返す。`GetVersionExW` の結果の major が 6 以上なら 0x1f8 = 504、そうでなければ 0x1f4 = 500。決定 0014 の仮想 OS は Windows 10 なので 0x1f8）を `cbSize` とし、`SPI(0x29, 0, &ncm = [rbp+0x34], 0)`（0x866493）。
   - 戻り値が非 0（0x8664a0〜0x866525）: 4 つの LOGFONT について `CreateFontIndirectW` → `f_5c1360`。lf1 = `[rbp+0x170]`（ncm + 0x13c = `lfStatusFont`）→ `[obj+0xfc]`、lf2 = `[rbp+0x114]`（ncm + 0xe0 = `lfMenuFont`）→ `[obj+0x10c]`、lf3 = `[rbp+0x1cc]`（ncm + 0x198 = `lfMessageFont`）→ `[obj+0x114]`、lf4 = `[rbp+0x4c]`（ncm + 0x18 = `lfCaptionFont`）→ `[obj+0x11c]`。**構造体の寸法（`iBorderWidth`〜`iCaptionHeight` = `[rbp+0x38..0x4b]`、`iSmCaption*` = `[rbp+0xa8..0xaf]`、`iMenuWidth/Height` = `[rbp+0x10c..0x113]`、`iPaddedBorderWidth` = `[rbp+0x228]`）と `lfSmCaptionFont`（`[rbp+0xb0..0x10b]`）は、この関数の 0x866498〜0x866525 の範囲では読まれない**（読み。ncm は `rbp+0x34` から 504 バイト。オフセットは Learn の構造体定義から計算: lfCaptionFont +0x18、lfSmCaptionFont +0x7c、lfMenuFont +0xe0、lfStatusFont +0x13c、lfMessageFont +0x198、iPaddedBorderWidth +0x1f4）。ncm を別の関数へ渡す形は見つけていない（`[rbp+0x34]` の `lea` は SPI の引数だけ）。
   - 戻り値が 0（0x866527）: `f_5c14d0([obj+0xfc], 8)` と、`GetStockObject(13)` の結果を `[obj+0x10c]`、`[obj+0x114]`、`[obj+0x11c]` に `f_5c1360` で渡す。
3. 最後に 3 つのスロット `[obj+0xfc]`、`[obj+0x10c]`、`[obj+0x114]` に `f_5c1030(slot, 0xff000017/07/08)` を呼ぶ（0x8665a6〜0x8665e2）。

**重要な読み**: 0x1f も 0x29 も、**失敗（0）を返すと、ゲストは `GetStockObject(13)`（決定 0054 が答える）で続行する経路を持つ**（0x866451、0x866527）。つまり、フォントの値を決めずに先へ進める選択肢がある（B 案）。ただしページは SPI の失敗条件（どの引数の形が失敗か）を書かないので、「失敗を返す」こと自体が決定 0058 の規則（ページが定義しない形は止める）の例外になる。

## 3. Learn で確認できること（一次。2026-10-22 取得）

- [NONCLIENTMETRICSW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-nonclientmetricsw)（`updated_at` 2024-11-20）: 構造体は `UINT cbSize; int iBorderWidth, iScrollWidth, iScrollHeight, iCaptionWidth, iCaptionHeight; LOGFONTW lfCaptionFont; int iSmCaptionWidth, iSmCaptionHeight; LOGFONTW lfSmCaptionFont; int iMenuWidth, iMenuHeight; LOGFONTW lfMenuFont, lfStatusFont, lfMessageFont; int iPaddedBorderWidth`。**sizeof**: LOGFONTW = 5×4 + 8×1 + 64 = 92（ページの LOGFONTW の定義から計算）。cbSize 4 + 5 int 20 + 92 + 2 int 8 + 92 + 2 int 8 + 92×3 = 500 + iPaddedBorderWidth 4 = **504 = 0x1f8**（「iPaddedBorderWidth の存在で 4 バイト大きい」という Remarks と合う）。500 = 0x1f4 = 古い OS 向けの cbSize。ゲストの 0x1f8 / 0x1f4 と一致（計算）。フィールドの意味: iBorderWidth「The thickness of the sizing border, in pixels. The default is 1 pixel.」、iPaddedBorderWidth「The default value is 4 pixels」。**他の寸法（iScrollWidth、iCaptionWidth、iMenuHeight など）と、どのフォントのフェイス名・高さ・太さについても既定値は書かれていない**。
- [LOGFONTW](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-logfontw)（`updated_at` 2025-08-15）: フィールド（lfHeight、lfWidth、lfEscapement、lfOrientation、lfWeight、lfItalic、lfUnderline、lfStrikeOut、lfCharSet、lfOutPrecision、lfClipPrecision、lfQuality、lfPitchAndFamily、lfFaceName[32]）。lfHeight は負値で文字の高さ。`lfHeight = -MulDiv(PointSize, GetDeviceCaps(hDC, LOGPIXELSY), 72)`（LOGPIXELSY は決定 0053 の 96）。lfWeight 400 = FW_NORMAL、700 = FW_BOLD。lfFaceName は 32 文字（NUL 含む）まで。**どのシステムフォントの値も書かれていない**。
- SystemParametersInfoW のページは決定 0058 が全文を読んだ（SPI_GETICONTITLELOGFONT = 0x001F と SPI_GETNONCLIENTMETRICS = 0x0029 の数値と型は一次。値は書かれていない）。

## 4. 決定 0057 が止めている 22 index と、整合させる値（`getsystemmetrics-caller.md` と 0057 の表）

ゲストは 0x29 の寸法を**読まない**が、同じ量を `GetSystemMetrics` では読む。0057 が止めている index のうち、NONCLIENTMETRICS のフィールドと**対応しうる**もの（対応は Learn の名前からの**推測**。ページが「SM_CXVSCROLL は iScrollWidth と同じ」とは書いていない）:

| NONCLIENTMETRICS のフィールド | 対応しうる SM_*（0057 が停止にしているもの） |
|---|---|
| iScrollWidth / iScrollHeight | SM_CXVSCROLL（2、20=SM_CYVSCROLL）、SM_CYHSCROLL（3、21=SM_CXHSCROLL）、SM_CXHTHUMB（10） |
| iBorderWidth、iPaddedBorderWidth | SM_CYBORDER（6）、SM_CXFIXEDFRAME / SM_CYFIXEDFRAME（7、8）、SM_CXSIZEFRAME（32）、SM_CXEDGE / SM_CYEDGE（45、46） |
| iMenuHeight | SM_CYMENU（15） |
| iCaptionWidth / iCaptionHeight | （0057 の 22 index に SM_CYCAPTION、SM_CXSIZE は無い。ゲストが使わない） |

つまり、フォントとスクロールバー寸法をまとめて決めるとき、**0x29 のうちゲストが実行で使う部分はフォント 4 つ（status、menu、message、caption）だけ**で、寸法は GetSystemMetrics の 16 index（上の 2、3、6、7、8、10、15、20、21、32、45、46 と、無関係の 13、14、61、68、69）が別途要る。0x29 に答える決定と、SM_* の 12 index を決める決定は、**同じ「ウィンドウのフレームとスクロールバーの寸法」の決定にするのが自然**（推測）。

## 5. 選択肢（判断は PM）

- **A. フォントと寸法をまとめて決める**: 仮想 LOGFONTW を 5 種（icon title、caption、status、menu、message）と、SM_* の寸法を、一つの表として決める。値はどれも Learn に書かれていない（出典のある数値は iBorderWidth = 1、iPaddedBorderWidth = 4 だけ）。LOGFONT の lfFaceName（例: "Segoe UI" は実 Windows 10 の既定だが、非公式の出典になる）、lfHeight = −12（9 pt、96 DPI。Windows 10 の既定だが非公式）、lfWeight = 400 は**仮の決定**になる。決定すれば、CreateFontIndirectW（5 回、`GetStockObject(13)` と同様の仮想 GDI オブジェクト。決定 0054 の拡張）と、`f_5c1360` の中身が次の停止点になる。
- **B. 0x1f と 0x29 の両方で FALSE（0）を返し、ゲストの `GetStockObject(13)` の経路に進ませる**: フォントの値を決めずに済む。ゲストの失敗経路は読みの上で完結している（§2。ただし `f_5c14d0([obj+0xfc], 8)` の中身は追っていない）。欠点: ページは SPI の失敗の条件を書かず、0058 の「ページが定義しない形は止める」規則の例外になる。実 Windows ではこの 2 つの SPI は失敗しないので、この経路は実環境では通らない（推測）。GetLastError の値も決める必要がある。
- **C. 停止のまま**（今の状態）。

## 6. その先の見通し（推測）

0x1f、0x29 を越えると、f_8663b0 の `f_5c1030`、そして `0x866640〜` 以降の同じ unit の続き（f_8651a0 の 0x865379 以降）。f_8651a0 の呼び出し元 f_620cd0 は unit 114（0x623840）の一部で、a4 の進み（115 / 942）は、この初期化がまだ続く間は変わらない。`GetStockObject(13)` は 0054 が答える。`CreateFontIndirectW`、`CreateHalftonePalette`（0x1971ba0。thunk 0x423fd0）は未実装。MonitorFromWindow（5 か所）、MonitorFromRect（2 か所）、MonitorFromPoint（1 か所）は、`callback-caller.md` §6 のとおり未到達。

## 7. 限界

- ゲストの `f_5c1360`、`f_5c14d0`、`f_5c1030`、`f_86b240` の中身は追っていない。
- 0x29 の `iScrollWidth` などをゲストが別の経路で読む可能性（間接呼び出しの先）は見ていない。
- NONCLIENTMETRICS の sizeof（504 / 500）は Learn の構造体定義からの計算で、ページには数値が無い（ゲストの 0x1f8 / 0x1f4 とは一致）。
