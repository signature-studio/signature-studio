# 文字が画面に出る経路(GDI / QFC+FreeType / Blend2D)と、idle までに文字が描かれなかった理由の候補

作成: 2026-11-01(HEAD 1d7173f、ブランチ main)。担当: 調査(静的分析と文書作成のみ)。**何も決めない**(決めるのは PM)。ビルド・実行・`runtime`/`crates`/`harness` の編集はしていない。リポジトリ内の新規ファイルはこの文書と `docs/investigations/text-path/` だけ。

## 0. 結論(5 行以内)

1. **主経路は GDI ではなく QFC(FontCache)+ 自前の字形ラスタライズの可能性が高い(読み、中程度)**。FLEngine の GDI 文字 API は 97 呼び出し点が実在するが、FRUITY 無効の記録(13,572 呼び出し)にも決定 0126 の FRUITY 有効の実行にも 1 回も現れない。QFC の `FontCache_AcquireFont` と、その先の FreeType `FT_Load_Glyph` は実行された。
2. **Blend2D は文字を描かない(読み、強い)**。Agg2D の 130 スロットにテキスト用は無く、Blend2D は kernel32 しか import しない。
3. **最有力の理由は「通常の UI 文字用のフォントが VFS に無い」(§4 の仮説 (ii))**。FLEngine の通常フォント(`segoeui`/`OpenSans-Regular`/`tahoma` ほか)の探索は記録で全部失敗し、QFC が読んだ記録のあるフォントは種の 2 つ(アイコン用)だけ。ただし 4 つの仮説のどれも実行では確認も反証もできていない。`FT_Render_Glyph` は、読みでは種の 2 フォントでは呼ばれない(QFC は scalable な TrueType を自前の輪郭ラスタライザで描く)ので、**`FT_Render_Glyph` の実行を測っても主経路の有無は決まらない**。
4. **仕事の範囲は (b) が本線、(c) が出る可能性が高い**。GDI の文字 API を実装する (a) は、今の読みでは主経路に要らない。通常フォントの供給(決定 0110 の延長)が先で、足すと freetype の未 lift 部分に当たる可能性がある。
5. 測定は未実施の部分が多い(§6)。次の最小の測定は、ゲスト関数の入口回数と `FontCache_AcquireFont` の引数・戻り値を数えること(§5)。

## 1. 凡例と手順

- **実測** = 既存の記録(ログ、PNG、決定の本文)を読み直した数値、またはこの作業で数えた数値のうち「バイナリの静的な数え上げ」。この作業で**プログラムは 1 つも実行していない**。したがって「実行された」と書くものはすべて既存の記録の再確認である。
- **読み** = 逆アセンブル(capstone)と文書の読み。実行していない。
- **未確認** = 確認していない、または記録が無い。
- 静的な到達: **T1** = 直接 `call`/`jmp rel` のみ。**T2** = T1 + `lea reg,[rip+関数の先頭]`。**T3** = T2 + Delphi クラスの仮想メソッド(クラスを生成する関数があれば、その VMT のスロット全部を到達とみなす)。T1・T2 は `docs/investigations/goal-ladder/` と同じ定義で、関数数(FLEngine の `.pdata` 57,816、`CreateFruityInstance` 0x1375990 の T1 閉包 6,591 関数、T2 8,953 関数)が goal-ladder の数と一致することを確認した。**T3 は上限**(インタフェース呼び出しとデータ上の関数ポインタは辺に無いので、その点では下限でもある)。**「届く」は「実行された」ではない**。
- 測定の基底: FLEngine 0x400000、QFC 0x1d00000、freetype 0x4b00000、Blend2D 0x4700000、ILTools 0x6000000(`tp_index.py` が再配置済みの VA で報告する)。入力の SHA-256 は FLEngine が `b2c64da4…496d`、QFC `afc5a0ed…36d2`、freetype `6ea6c106…7cb8`、Blend2D `66634d24…66c7`、ILTools `20409dcb…0f95` で、いずれも既存の抽出物と一致した。インストーラからの再抽出は要らなかった。
- 手順(すべて `docs/investigations/text-path/` のスクリプト、`PYTHONDONTWRITEBYTECODE=1`、作業場所 `/tmp/opencode/txt1/`):

| 段階 | スクリプト | 出力 |
|---|---|---|
| 5 つの PE の呼び出し点の索引 | `tp_index.py` | `/tmp/opencode/txt1/idx/*.pkl`(保存せず) |
| 呼び出し点・到達 | `tp_reach.py`、`tp_rta.py`(T3)、`tp_classify.py` | `callsites-gdi-text.tsv`、`class-text-reach.tsv`、`rta-t3-fle.txt` |
| Delphi クラスと仮想メソッド | `tp_vmt.py` | `classes-flengine-text.tsv`、`classes-qfc.tsv` |
| QFC が FreeType から引く関数 | `tp_ftslots.py` | `ft-slots.tsv` |
| 逆アセンブル・文字列 | `tp_dis.py`、`tp_strrefs.py` | (本文に引用) |
| フォント種の中身 | `tp_ttf.py` | `ilfont-info.txt` |
| 画面のアイコンとフォントの照合 | `tp_png.py`、`tp_glyphmatch.py` | `glyphmatch-ilglyphs.txt` |

## 2. 文字が画面に出る経路の特定(呼び出し点の表)

要約表は `paths-summary.tsv`。以下に主なものを書く。

### 2.1 (a) GDI の文字 API(FLEngine)

| API | 呼び出し点 | 呼び元関数 | T1 | T2 | T3(上限) | runtime の状態(HEAD) |
|---|---:|---:|---:|---:|---:|---|
| `ExtTextOutW` | 7 | 6 | 0 | 0 | 6 点・5 関数 | `GDI_STOP`(`fle_gdi.c` 3751) |
| `DrawTextW` | 23 | 17 | 4 | 5 | 11 点・6 関数 | `GDI_STOP`(3774) |
| `DrawTextExW` | 1 | 1 | 0 | 0 | 1 | `GDI_STOP`(3775) |
| `TextOutW` | 1 | 1 | 0 | 0 | 0 | 応答表に行なし |
| `GetTextExtentPoint32W` | 11 | 9 | 0 | 0 | 7 | `GDI_STOP` |
| `GetTextExtentPointW` | 2 | 2 | 2 | 2 | 2 | `GDI_STOP` |
| `GetTextMetricsW` | 10 | 9 | 4 | 4 | 5 | `GDI_STOP` |
| `CreateFontIndirectW` | 7 | 3 | 2 | 7 | 7 | 答える(決定 0066) |

(実測=静的な数え上げ。表は `callsites-gdi-text.tsv`。97 行。)

- **FLEngine の `ExtTextOutW` の 7 点**: `f_5c3970`・`f_5c3a80`(TCanvas 系の仮想メソッド、`TQuickControlCanvasEx`/`TCanvas`/`TBitmapCanvas`/`TControlCanvas` の同じスロット)、`f_7fd4d0`・`f_7fd520`・`f_7fd580`(`options = 2` を渡す薄いラッパー)、`f_9f0d10`(932 B、`GetTextMetricsW`→`GetTextAlign`→`ExtTextOutW` の文字描画ルーチン)。`DrawTextExW` は `f_5c3ba0`(同じ TCanvas のスロット)。(読み)
- **TCanvas の文字スロット**: `TCustomCanvas`(0x5b8ff0)の 53 スロットのうち 50〜52 は抽象スタブ(`f_40c820`)で、`TCanvas`・`TBitmapCanvas`・`TControlCanvas`・`TQuickControlCanvasEx` が同じ 3 つの実体(`f_5c3970`、`f_5c3ba0`、`f_5c3a80`)を持つ。Delphi の 64 ビット VMT では、メソッド呼び出しの変位は **(スロット番号 − 14) × 8**(QFC で `FT_Load_Glyph` を呼ぶ `f_1e75620`(スロット 27)が `call [vptr+0x68]` で呼ばれることと、FLEngine の TCanvas の読みの整合で確認した)。この規則で `call [reg+0x120/0x128/0x130]` を数えると **114/143/352 点**(`canvas-text-vcalls.tsv`)。**受け手のクラスは静的に決まらない**ので、これは TCanvas の文字呼び出しの数ではなく、同じ変位の仮想呼び出しの上限である(T3 の到達関数内は 23/97/168 点)。
- **到達**: `CreateFruityInstance` の T1/T2 閉包では `ExtTextOutW` に届かない。`TQuickControlCanvasEx` の生成点(T3 で `f_11b2ac0`→`f_ed7c00`→`f_85c6d0`→`f_5c3970` の経路が出る)を足すと届く。生成されているクラスは T3 で `TCanvas`、`TBitmap`、`TBitmapCanvas`、`TControlCanvas`、`TQuickControlCanvasEx`、`TQuickLabel`、`TNewCaption`、`TNumBar`、`TQuickStringGrid` など 820 クラス(1,890 のうち)。
- **実行の事実**(既存の記録の再確認):
  - FRUITY 無効の 6 モジュール完走のトレース `a4-six-module-trace`(13,572 呼び出し、171 種): `ExtTextOutW`・`DrawTextW`・`TextOutW`・`GetTextMetricsW`・`GetTextExtentPoint32W` の呼び出しは **0 回**。出るのは `CreateFontIndirectW` 11 回(FLEngine 5、ILTools 6)、`SetTextColor` 68 回(ILTools のみ)、`EnumFontFamiliesExW` 2 回。**`StretchDIBits`・`CreateDIBSection`・`MoveToEx`・`CreateBitmap` もそれぞれ 34 回で、呼び元はすべて ILTools**(`CreateCursorFromFont` の 34 回の繰り返しと読める。決定 0110)。つまりこのトレースの GDI 描画は ILTools のカーソル作成だけで、FLEngine のメイン UI の描画ではない。
  - FRUITY 有効(決定 0126): 文字 API は `GDI_STOP` のままで、**停止(b)までに文字描画を呼ぶ入口は 1 回も来ていない**(0126 本文)。呼んでいたなら `fle_stop` で止まっていたはずなので、この言い方は実測と整合する。
- 補足: 呼び出しが 0 回だったのは「文字を描く要求が無かった」ことを意味せず、「要求は GDI の文字 API に出なかった」ことだけを意味する。

### 2.2 (b) QFC の FontCache

QFC の export は 10 個(`exports.txt`)。FLEngine が引くのは 8 名で、そのうち文字に関わるのは 3 名。

| export | FLEngine の使用 | 読み | 到達(T1) | 実行 |
|---|---|---|---|---|
| `FontCache_AcquireFont` (0x57f7f0) | `f_6685d0`(0x668627 `call [[0x15e0b48]]`)の 1 点 | フォント名・サイズなどからフォントオブジェクトを返す | 到達(`f_6685d0` は `CreateFruityInstance`→…→`f_966550`→`f_668310`→`f_6685d0` など、経路長 6) | FRUITY 有効で実行済み(wall 50 のシャドウスタック: `f_7f1510`→`f_668760`→`f_6685d0`→`FontCache_AcquireFont`→…→`FT_Load_Glyph`) |
| `FontCache_GetStats` | `f_668980` の 1 点 | 統計 | 呼び元 `f_120b790` は静的な呼び元なし | 未確認 |
| `FontCache_CleanUp` | 読み手が見つからない | 終了側 | - | 未確認 |

- **FLEngine の QFC ラッパー**(`TQuickFont` 側):`f_6685d0`(13 点から呼ばれる)、`f_668670`(30 点)、`f_668760`(26 点)、`f_668850`(2 点)、`f_668310`(14 点)。`f_668670`/`f_668850` は QFC のフォントオブジェクトに `call [vptr+0x18]`(`TQuickInternalFont` のスロット 17 = `f_1e82290`、約 5.7 KB)を呼ぶ。`f_668310` などは `call [vptr+0x20]`(スロット 18 = `f_1e83fe0`、20 バイトの寸法のコピー)。`f_1e82290` は字形の寸法・配置を計算する大きな関数で、FreeType/AGG の呼び出しを下で行う。(読み)
- **QFC の文字クラス**(`classes-qfc.tsv`):`TQuickInternalFont`(0x1e7e940、インスタンス 0x8d8 バイト)、`TQuickGlyph`、`TQuickCachedFont`、`TAggFontEngine`、`TAggFontEngineFreetypeBase`、`TFontEngineFreetypeFast` と、AGG のラスタライザ・スキャンライン・レンダラの一式(`TAggRasterizerScanLineAA`、`TAggRendererScanLineAASolid`、`TAggOutlineAA` ほか)。**QFC 自身が AGG の輪郭ラスタライザを持つ**。(読み、強い)
- **FLEngine のフォント表**: `f_668d30`(2,525 行)が名前付きフォント(`P_SmallBold`、`P_Small`、`P_Tiny`、`P_Normal`、`P_NormalBold`、`P_Large`、`P_TitleBar`、`P_DigitWheel`、`P_Monospaced`、`P_ILGlyphs`、`P_WebSymbols`、`P_DescriptionLarge`、…)を作り、ファイル名の候補として `segoeui.ttf`、`OpenSans-Regular`、`tahoma`、`OpenSans-CondLight`、`consola`、`lucon`、`ILGlyphsEx.ilfont`、`WebSymbols-Regular.otf`、`PT_Sans-Web-Regular.ttf` を持つ(`tp_strrefs.py`)。実行の記録(`a4-six-module-trace` の `GetFileAttributesW`)では、これらの探索は **すべて失敗(0xffffffff)**。成功は種の 2 ファイル(ハーネスの配置)と ILTools の `ILGlyphsEx.ilfont` の確認だけ。

### 2.3 (c) Blend2D の Agg2D

- vtable は 0x48dbec0(基底 0x4700000)から連続する 130 個のコードポインタ。実体 78 + スタブ 52(`ret` 49、`xor eax,eax; ret` 3)。実体の役割は決定 0120 の表(`blend2d-nojit-slots.md` §4.1)で、図形(矩形・楕円・角丸・弧・星・多角形)、パス(`moveTo`/`lineTo`/曲線/`drawPath`)、色(`fillColor`/`lineColor`)、線の属性、変換、クリップ、グラデーション、`pointInPath`。**テキスト・フォント・画像・パターンのスロットは無い**。FLEngine が呼ぶ 45 種の変位(窓 200 命令で 445 点)もこの範囲。スタブのスロット 2・14・18・104 への呼び出しが 22 点あるが、これらは戻り値 0 / 何もしない。(読み、強い。ただし「スタブが将来のテキスト用でない」ことは構造上の読み)
- Blend2D.dll は **kernel32 しか import しない**(`pefile`)。OpenType の表のタグ(`glyf`、`cmap`、`GSUB` など)の文字列は `.rdata` に 1 つずつあるが、これは Blend2D ライブラリ自体のフォントパーサの定数が残っているだけで、`Agg2D` ラッパーの公開口(`AggCreateInstance` の 1 export)からテキスト描画に届く辺は見つかっていない。(読み。**未確認**: ライブラリ内のフォント関数がラッパーから呼ばれるかは追っていない。ただし 130 スロットにテキストが無いので、呼ぶ入口は無い。)

### 2.4 (d) FreeType

- FLEngine は `FT_*` の文字列も import も持たない(バイト検索で 0 件)。`freetype.dll` を引くのは QFC の `f_563000`(QFC の初期化 unit 129 から呼ばれる)で、14 名を UTF-16 で `GetProcAddress` する(`ft-slots.tsv`):`FT_Init_FreeType`、`FT_Done_FreeType`、`FT_Attach_File`、`FT_New_Memory_Face`、`FT_New_Face`、`FT_Done_Face`、`FT_Select_Charmap`、`FT_Set_Charmap`、`FT_Get_Char_Index`、`FT_Load_Glyph`、`FT_Render_Glyph`、`FT_Get_Kerning`、`FT_Set_Char_Size`、`FT_Set_Pixel_Sizes`。
- 各スロットは `.data` の「ポインタ変数」経由で使われる(`mov rax,[var]; call [rax]`)。使用点は 17 件で、`FT_Load_Glyph` と `FT_Render_Glyph`(2 点)は **どちらも `f_1e75620` 1 関数だけ**。`FT_Init_FreeType`/`FT_New_Memory_Face`/`FT_Get_Char_Index`/`FT_Get_Kerning`/`FT_Set_Pixel_Sizes` はそれぞれ別の関数(`f_1e74920`、`f_1e74f10`、`f_1e75570`/`f_1e755d0`、`f_1e76580`、`f_1e7cc30`)。`FT_New_Face`・`FT_Attach_File`・`FT_Set_Charmap`・`FT_Set_Char_Size` には使用点が無い。
- `f_1e75620` は `TFontEngineFreetypeFast`/`TAggFontEngineFreetypeBase`/`TAggFontEngineFreetypeInt32` の同じ仮想メソッド(スロット 27、`+0xd8`)。呼び元は `f_1e7d8b0`(`call [vptr+0x68]`、wall 50 のシャドウスタックの `f_1e7d8b0` と一致)。**`f_1e75620` は `FontCache_AcquireFont` を根にした T1〜T3 では届かない**(クラス `TFontEngineFreetypeFast` の生成関数 `f_1e7ce20` を呼ぶのは QFC の初期化 unit 144 の proc `f_1e7dcc0` で、AcquireFont からの辺に無い)。QFC の unit 144 を根に足すと届く(経路長 3)。つまり**エンジンは QFC の初期化で作られ、AcquireFont から後で仮想メソッドとして使われる**形である。(読み)
- **`FT_Render_Glyph` の使い方**(読み、強い): `f_1e75620` は `FT_Load_Glyph` の後、描画モードの byte(`[obj+0x4f]`)で分岐する。0(mono)と 1(gray)は `FT_Render_Glyph`(モード 0 は第 2 引数 2、1 は 0)を呼び、結果の `FT_Bitmap` を QFC の AGG のスキャンライン格納(`f_1e73980` 以下、`call [rbx+0x28/0x38/0x40/0x48]`)に入れる。2〜4 は `FT_Render_Glyph` を呼ばず、`FT_Outline` の点列を AGG の経路(`f_1e53f70` ほか)に渡す。モードは `f_1e74f10`(`FT_New_Memory_Face` の後)が決め、**唯一の呼び元 `f_1e7d840` が `r9b = 2` を渡す**。`r9b = 2` のとき、`f_1e62fa0`(`face_flags & 1`)が真ならモード 2、偽なら 1。`FT_FACE_FLAG_SCALABLE` が `face_flags` の bit 0 なので、**TrueType の outline フォント(種の `ILGlyphsEx.ilfont`/`ILCursors.ilfont` は glyf を持つ)ではモード 2、つまり `FT_Render_Glyph` は呼ばれない**。FreeType 側で `test byte ptr [reg+8], 1` が 17 件あり、`face_flags` が `+8`(Win64 の LLP64 で `FT_Long` が 4 バイト)であることとも矛盾しない。(読み。`face_flags` の位置は構造体定義ではなくコードの形からの推定。**未確認**。)

## 3. 実行の記録との突き合わせ(何が実行されたか)

| 項目 | 記録 | 区分 |
|---|---|---|
| GDI 文字 API の呼び出し | FRUITY 無効: 0 回(13,572 中)。FRUITY 有効: `GDI_STOP` に当たらず idle へ | 実測(既存) |
| `FontCache_AcquireFont` | FRUITY 有効で wall 50 の停止時に実行中(フォントファイル 0x19138 = 102,712 B を `CreateFileW`/`ReadFile`)。FRUITY 無効のトレースでは QFC が `ILCursors.ilfont`(0x1ed4 B)だけを読む | 実測(既存) |
| `FT_Load_Glyph`→`TT_RunIns` | wall 50〜52(入口 6,912,000→8,651,367 の約 174 万入口) | 実測(既存) |
| `FT_Render_Glyph`・`FT_Outline_Render`・`ft_smooth_render`(`f_4b15de0`・`f_4b15ef0`・`f_4b15fd0`) | **記録が 1 件も無い**(`lift/out-git/*/logs`、`docs/`、`lift/out-fruity11/logs` を VA と名前で検索) | **未確認** |
| 描画面への書き込み | lid 13 の面は 8 回の書き込みで 1,917,820/2,107,560 画素が非 0、2,759 色。左ブラウザ欄(6,150)-(218,1045)は 4 色・黒 99.9%(189,528/189,740 画素) | 実測(PNG を `tp_png.py` で数えた) |
| 文字 | 面に文字は 1 つも無い(PM と作業者の目視、0126)。この作業でも 4 領域の色数を数えたが、文字の有無は数えからは決まらない | 実測(目視は既存) |

- **ILGlyphsEx.ilfont の中身**(`tp_ttf.py`、`ilfont-info.txt`): "IL Glyphs"、648 グリフ、cmap 648 件のうち **591 件が私用領域(U+E000〜U+F8FF)**、ASCII 英字 51/52、数字 0/10、ヒンティング(`cvt `/`fpgm`/`prep`)あり。`ILCursors.ilfont`: 55 グリフ、私用領域 0、ASCII 英字 50/52。**どちらも UI の通常の文字(数字・ラベル)を出すためのフォントではない**。画面のトランスポートやブラウザタブのアイコンは `ILGlyphsEx` の私用領域グリフで描かれる可能性があるが、画面のアイコンとの照合(`tp_glyphmatch.py`、NCC)は最良 0.80〜0.99 で同点が複数あり、**どのグリフかは決まらない**(未確認)。
- **通常の UI 文字のフォントは VFS に無い**: 種は 2 ファイルだけ。`OpenSans-Regular.ttf`(217,360 B)ほかの 36 ファイルはインストーラにあるが種に入っていない(決定 0110)。FLEngine の探索(`f_668d30`、`f_668b50`)は `…\FL Studio 21\Artwork\Fonts\ILGlyphsEx.ilfont` と相対の `Shared\Artwork\Fonts\…` を見て **配置先(`…\Shared\…`)と合わず、見つからない**(0110、記録で 0x80 になるのは ILTools の探索だけ)。

## 4. なぜ idle までに文字が描かれなかったか(仮説と反証の材料)

どの仮説も**実行では確認も反証もされていない**。静的な読みと既存の記録で、材料を並べる。

### 仮説 (i) 文字は初回描画ではなく後の再描画で出る設計

- 材料(支持): 面への書き込みが 8 回で全体が埋まる(`calls that wrote 8`)ので、背景・コントロールのキャッシュをまとめて転送する設計に見える。決定 0126 は「(b)の後にゲストが文字を描くかは**未確認**」と書く。停止時に Sleep(16 ms 後)と SetTimer(600 s 後)が残る。
- 材料(反証できないが弱める): `TQuickLabel` のユーザ仮想スロットのうち 2 つが QFC のラッパーに届き(`class-text-reach.tsv`)、`TQuickLabel`・`TNewCaption`・`TNumBar`・`TQuickStringGrid` は T3 で生成される。つまりラベルの描画メソッドは起動中に呼ばれうる。
- 反証の材料: `TQuickLabel` の描画メソッド(`f_7f1720` と読めるもの。950 B、`f_6685d0` と `f_668760` から QFC に届く)を、**`fle_call_guest` で直接呼ぶか入口を数える**。

### 仮説 (ii) フォントの取得に失敗して文字をスキップする

- 材料(強い支持): FLEngine の通常フォント(`segoeui`、`OpenSans-Regular`、`tahoma`、`OpenSans-CondLight`、`consola`、`lucon`、`WebSymbols`、`PT_Sans`)の探索は記録で全部失敗する。実機の `C:\Windows\Fonts\segoeui.ttf` 等は VFS に無く、`OpenSans-Regular.ttf` は種に無い。`f_668d30` が失敗後にどのフォントを使うかは読めていない(`f_6685d0` は `FontCache_AcquireFont` に名前と `[obj+0x24]`〜のパラメータを渡すだけで、戻りの NULL の扱いは `f_6685d0` の戻りの `[rcx+8]` への格納まで)。
- 材料(強い支持の続き): wall 50 の停止時、FLEngine の `f_7f1510`→`f_668760`→`f_6685d0`→`FontCache_AcquireFont`→`FT_Load_Glyph` の鎖で QFC が読んだファイルは 0x19138 = **102,712 B** で、`ILGlyphsEx.ilfont`(102,712 B)と同じ大きさ(fruity9 wall-log。ファイル名は記録に出ていない=一致は大きさだけ)。**FLEngine の要求で QFC が作ったフォントが、記録のうえではアイコン用の 1 つだけ**であることになる。
- 材料(弱める): `FontCache_AcquireFont` は動き、字形の取得(`FT_Load_Glyph`→`TT_RunIns`、約 174 万入口)まで進んだので、**QFC のフォント機構自体は動く**。画面のアイコンが `ILGlyphsEx` の字形で QFC から描かれているなら、字形を面に合成する段も動いていることになり、その場合 (iii) は弱まる。アイコンが QFC の字形か、`RT_BITMAP`(25 個)/`RT_RCDATA`(128 個、1.79 MB)の画像かは、照合が曖昧で**決まらない**(§3)。
- 反証の材料: `FontCache_AcquireFont` の戻り値(NULL か否か)と、`f_1e82290`・`f_1e7cf80`(字形の取得)の入口回数を数える。

### 仮説 (iii) 文字のキャッシュを先にメモリ DC に描き、BitBlt で転送する

- 材料(支持): FLEngine は `TDIB`/`TDIBmp`/`TBitmap`/`TBitmapCanvas` を生成し、画素の合成を `.bss` の関数ポインタ 4 本(0x15f3ed0 ほか)で呼ぶ(goal-ladder E3)。描画面は BitBlt/StretchDIBits で埋まる(0126)。つまり「文字は DIB に先に合成され、DIB が転送される」形なら、転送元の DIB に文字が入っていなければ面に文字は出ない。
- 材料(支持): QFC 自身は **QFC の閉包から GDI 文字 API に届かない**(T3 でも `f_1e138e0` など 6 点は未到達)。QFC の文字は GDI を使わず、AGG のスキャンラインを呼び元に返す設計に読める。呼び元(FLEngine の TQuickFont 側)が DIB に合成する。
- 反証の材料: FLEngine 側で QFC のスキャンライン(`f_1e73980` の出力)を DIB に混ぜる関数(`TRenderText`、`TGlyphCache` など)を探す。**`TRenderText`(VMT 0x568130)と `TGlyphCache`(0x6e6d40)は、T3 の生成関数に入らない**(クラス参照を読む関数が到達集合に無い)。つまりこの 2 クラスは静的には起動中に生成されない。**訂正**: goal-ladder E7 が並べた `TQFHinting` と `TTextLayout` は、型情報の種別 byte が 3(列挙型)で、**クラスではない**(`TGlyphCache` だけが種別 7 = クラス。FLEngine の RTTI の読み)。E7 の「`TGlyphCache`、`TQFHinting` が FLEngine 自身にある=グリフのキャッシュとヒンティングを持つ」は、`TQFHinting` については根拠にならない。`TGlyphCache` はクラスだが、VMT のスロット数は 14(TObject の分だけ)で、ユーザ定義の仮想メソッドは無い。

### 仮説 (iv) フォントのサイズ・描画フラグ(Blend2D のテキスト関数が使われる)

- **反証(強い)**: Agg2D のスロットにテキストは無い。Blend2D は文字を描かない。この仮説は除ける(読み)。

### 総合(読み)

最有力は **(ii)**: 通常の UI フォントが VFS に無く、`P_*` フォントのうち記録のうえで QFC が作ったのはアイコン用の 1 つだけ。(i) は (ii) と両立し、(b) の後の再描画で文字が出るかは未確認のまま残る。(iii) は、アイコンが QFC の字形なら弱まり、そうでなければ残る。(iv) は除ける。(仮説。**測定が先**。)

## 5. ゴールに対する読み(決めない)

- **仕事の範囲**:
  - (a) GDI の文字 API を実装する: 今の読みでは主経路に要らない。FLEngine の GDI 文字 API の呼び出しは `CreateFruityInstance` の直接 call の閉包にほぼ無く、実行にも 0 回。TCanvas の文字スロットは T3 で届くので**完全には除けない**。入れるなら、`GetTextMetricsW`(10)・`GetTextExtentPoint32W`(11)の計測と `ExtTextOutW`(7)・`DrawTextW`(23)の描画が最小の範囲で、OS フォントの形状は再現できず実機と画素が一致しない(goal-ladder M-3)。
  - (b) QFC/FreeType/Blend2D の既存 lift が動けば足りる: QFC と FreeType は既に lift 済みで `FT_Load_Glyph` まで動いた。**runtime に足すものは小さい可能性が高い**。ただし通常フォントの供給(`OpenSans-Regular.ttf` など 36 ファイルの種と、FLEngine の探索先 `…\FL Studio 21\Artwork\Fonts\` の別置き)は環境の追加で、決定 0110 の延長になる。
  - (c) 新しい壁: 出る可能性が高い。フォントを足すと freetype の穴に当たる範囲が広がる(0110 の指摘)。QFC のモード 2 の経路は AGG(Delphi、lift 済み)で、`f_1e82290`(約 5.7 KB)と `f_1e73980`(580 行)が実行されたことは**記録に無い**ので、そこに壁がある可能性は読めない。
- **G4/G5 に対する優先度**: G4(空白でない)には文字は要らない(0126 の面は既に非空白)。**G5(「文字・アイコンを含めて描かれ、領域ごとに比較」)の必須条件**。X2(goal-ladder)の判断「GDI 主か FreeType 主か」は、読みでは **FreeType(QFC)主** に傾いたが、**画素一致は目標から外れる**(フォントが実機と同じでも、AGG のラスタライザと GDI/DirectWrite は一致しない。FL Studio 自身が QFC を持つので、**QFC 経路の画素は実機と一致しうる**。ここは実機のスクリーンショットが無いので未確認)。
- **次に作る測定(診断)と実装の最小の単位**:
  1. 測定 M-T1: ゲスト関数の入口回数を数える(gdb の `break f_…` と `calls` の読み、または `FLE_FRUITY_STOP_AT` の API 名版。**注意: `FLE_FRUITY_PEEK_AT` はリポジトリに存在しない**。既存の入口は `tools/gdbentry.sh`/`gdbfn.sh`(`out-fruity11/tools`、gdb でシャドウスタックと `calls` を読む))。対象: `f_6685d0`(FontCache_AcquireFont の呼び元)、`f_668670`・`f_668850`・`f_668760`・`f_668310`、`f_7f1720`(`TQuickLabel` の描画)、QFC の `f_1e82290`・`f_1e7cf80`・`f_1e75620`・`f_1e73980`、freetype の `f_4b15de0`(`FT_Render_Glyph`)。
  2. 測定 M-T2: `FontCache_AcquireFont` の戻り値と引数(フォント名・サイズ)を、`f_6685d0` の入口で記録する。
  3. 実装の最小単位: 測定が「フォントの取得に失敗」なら、種に `OpenSans-Regular.ttf` を 1 つ足して再実行(runtime の変更なし、`vfs-seed-fonts.tsv` の 1 行)。「字形は取れたが合成されない」なら、`TRenderText`/`TGlyphCache` の生成点(静的に届かない)を `fle_call_guest` で直接呼ぶ実験。
- **他の進行中の作業と触るファイルが重ならない範囲**: idle1(VFS 種と壁の連続修正)は `lift/harness/seeds/vfs-seed-fonts.tsv` と `lift/runtime/fle_vfs.c`/`fle_gdi.c` に触る。wf1(wasm + 入力 G3)は `lift/harness/web/` と `fle_thread.c`、pin1(再現性とテスト)は `a4-multi.sh` と `tests/wall_fr11.rs`。この作業の測定 M-T1/M-T2 は gdb とログだけで、これらのファイルに触れない。フォント種の追加は idle1 の領分なので、**idle1 の報告に「通常フォントの探索が全部失敗」を渡す**のが重ならない進め方。

## 6. 限界・未確認

- **実行していない**: この作業のすべての「届く」「呼ぶ」は静的な読み。`FT_Render_Glyph` の実行は記録が無く未確認。モード(`[obj+0x4f]`)の値、`FontCache_AcquireFont` の戻り値、種のどのフォントが作られたかも未確認。
- `face_flags` の位置(`+8`)と bit 0 が `FT_FACE_FLAG_SCALABLE` であることは、FreeType のコードの形(`test byte ptr [reg+8],1` が 17 件)からの推定で、構造体定義を読んだわけではない。
- T3 の上限は、仮想メソッドを「クラスが 1 つでも生成されれば全スロットが到達」とみなすので過大。インタフェース呼び出しとデータ上の関数ポインタは辺に無いので、その点では下限。画面のコントロールのクラスは窓クラス名から生成され、`CreateWindowExW` の経路を静的には辿れない(T3 は `lea` の VMT 参照だけ)。
- `f_7f1720`(`TQuickLabel` の描画メソッド)と読んだのは、クラスの VMT スロット(`TQuickLabel` の 18 番目の `f_7f14f0`→`f_7f1510`→`f_7f1720`)と `FontCache_AcquireFont` ラッパーへの呼び出しからの推定。何を描くかは読んでいない。
- 画面のアイコンと `ILGlyphsEx` の照合は曖昧(同点が複数。同じグリフ番号を共有する code point は 648 件中 2 組だけなので、同点は別名では説明できず、照合が弱い)。
- この作業の途中で `tp_vmt.py` が Delphi クラスを 529 個取りこぼしていた(クラス名の正規表現が貪欲だった)ことに気づき、直して全表を作り直した。本文の数値は直した後のもの。直す前の数値(T3 の 629 クラス)は使っていない。
- `docs/README.md` は編集していない(索引の 1 行は `docs/investigations/text-path/README-line.txt`)。
- 作業時間: 開始 2026-11-01 06:57Z。PM への途中報告は 2 回(07:48Z、08:48Z)。

## 7. 成果物

`docs/investigations/text-path/`: スクリプト 11 本(`tp_*.py`)、TSV 8 本、テキスト 3 本、`README-line.txt`。合計 196 KB(500 KB 以内)。
