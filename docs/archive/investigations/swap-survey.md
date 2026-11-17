# swap1: 同梱 DLL を lift せず、上流(OSS)や別の物で置き換えられるか(調査)

**これは調査であり、進捗(G 段階の到達)ではない。FL Studio は実行していない。** 実行したのは、(1) 公開 OSS(FreeType 2.4.10、Blend2D 0.10.6/0.11.0/0.11.1、AsmJit)を自分でビルドした小さなベンチ、(2) FL に同梱された Image-Line の実 DLL 3 本(`Blend2D.dll`、`freetype_x64.dll`、`dsp_ippv2_x64.dll`)を **Wine 10.0(Ubuntu 版)上で単体として**呼ぶ小さな Windows 用 exe(mingw-w64 でビルド)だけである。lift の本番ビルドは変更していない。代役・モック・事前生成物は使っていない。作業日 2026-11-03(04:34〜)、HEAD は作業中に 8577612 → 83006a9 に進んだ(私は git を変更していない)。

凡例: **【実測】** 私がこのセッションで実行して得た値(出力は `tables/` とツールの出力)。**【読み】** 逆アセンブルや文書を読んだ解釈で、実行で確かめていない。**【推測】** 根拠が薄い。**【未確認】** 調べていない、または決められない。**【引用】** 既存文書・PM の提示の転記(再測定していない)。公開 OSS のコードは引用していない。

---

## 0. 結論の要約

1. **5 つの同梱 DLL のうち、オープンソースで上流から作り直せるのは 2 つだけ**: `freetype_x64.dll`(= FreeType **2.4.10**、確度: 高)と `Blend2D.dll` のうち Blend2D 本体(= Blend2D **a7f9476 = 0.11.1**[2024-05-31 20:06 +0200]+ AsmJit 1.13、確度: 高。直前の 7e32ebf[0.11.0]とは、SRC_OUT + アンチエイリアスの画素で実 DLL が a7f9476 とだけ一致することで区別できた)。**Blend2D.dll には Image-Line 自身のコードである Agg2D ラッパー(130 スロットの vtable)が載っており、これは上流に無い。** `dsp_ippv2_x64.dll` は **Intel IPP 2021.7**(非公開)で OSS ではない。`ILTools_x64.dll` と `quickfontcache_x64.dll` は Image-Line の Delphi コードで OSS ではない(QFC は AggPas と FreeType の結合ユニットを内蔵する)。
2. **上流を同じ入力で実 DLL と比べると、ビット単位で一致した**【実測】。(a) FreeType 2.4.10(既定構成)を native -O2、wasm -O2、wasm -Oz でビルドし、実 `freetype_x64.dll` と比べた: 4 フォント(プロジェクトの `ILGlyphsEx.ilfont`/`ILCursors.ilfont`、DejaVuSans、Lato)× ロードフラグ 4 通り × 描画モード × 3 サイズで、ビットマップ、アウトライン点列とタグ、メトリクス、cmap 引き、カーニング、face の各フィールドのハッシュが **全 16 組合せ(ほか別の 27 組合せ)で一致**。(b) Blend2D 0.11.0/0.11.1 の BLContext に Agg2D のスロットを素直に写した描画は、実 `Blend2D.dll`(Wine、JIT 経路)と **ランダム 20,000 手(矩形/楕円/多角形塗り/線/折れ線ストローク/塗り+ストローク、1280×720)で全画素一致**(上流 JIT、DISABLE_JIT、no-JIT ビルド、wasm -Oz/-O2/-O2 -msimd128 の 6 通りすべて同一ハッシュ)。さらに **1,000,000 手で、100 手ごとの全画面ハッシュ 10,000 点がすべて一致**(実 DLL[Wine、JIT] = 上流の固定パイプライン native = 上流 wasm -O2 -msimd128。`tables/b2d-1M-differential.txt`)。lift 版の記録値(決定 0120 の 6 手順の FNV-1a 6 個)も、実 DLL を Wine で動かした値と一致した。
3. **削れる wasm の大きさ**【読み。サイズは実測の上流 wasm と PM 提示の size1 の値から】: Blend2D 44.0 MB → 上流の no-JIT 描画ライブラリは **約 1.0 MB**(-Oz、画像コーデック・フォント・パス・グラデーション込み)。freetype 2.9 MB → 上流 2.4.10 は **約 0.37 MB**(全モジュール込み)。両方で約 **45 MB(wasm 全体 約 322 MB の約 14%)** の削減。S1(ネイティブの 2 倍以内)の倍率で言えば 4.54 倍 → 約 3.9 倍。**S1 は届かない**(dsp 54% と FLEngine 22% が残る)。
4. **見込める速さ**: 上流 wasm の実測は、FreeType が native の 1.3〜1.75 倍、Blend2D(固定パイプライン)が native の 1.4 倍(-Oz)〜0.88 倍(-O2 -msimd128 の自動ベクタ化)。**lift した FreeType/Blend2D の同じ負荷の時間は未測定**(決定 0120 も「速度は測っていない」)。**P1/P2 への効きは小さい**: native の lift 実行のサンプルでは、起動 CPU のうち Blend2D は 2.8%、FreeType は 0.5%(perf3)で、P2 の主因は dsp の `ippiWarpAffineQuad_8u_C4R` など(サンプルの 68〜74%)。置き換えで P1 が縮むのは上限でも 1〜2 秒(37 秒中)と読む。
5. **最大の障害**: (a) **JIT 無しの Blend2D(固定パイプライン)は、合成演算 29 種のうち 4 種(SRC_OVER、SRC_COPY、DST_COPY、CLEAR)しか描けない**。残りの 25 種は `BL_ERROR_NOT_IMPLEMENTED` を返して何も描かない【実測+一次ソース】。実機の DLL は JIT でこれらを描く。FL が `blendMode`(Agg2D スロット 15)に渡す値は、静的な読み(下限)で SRC_OVER・SRC_COPY・**OVERLAY(グラデーションのストロークの 1 か所)**の 3 種。実行時の頻度は未測定。**この OVERLAY は、自前のテンプレート(約 35 行)を固定パイプラインに足すと、実 DLL(JIT)とビット同一になることを、同じ形[線形グラデーション + drawPath flag 4]の 30 シーンで実測した**(§5.6)。wasm では AsmJit が使えないので、これは**上流 wasm の置き換え、現状の lift(決定 0120)のどちらでも同じ限界**。上流に全演算の移植版は無く(issue #182)、足すなら 1 演算 約 20〜35 行のテンプレート(PLUS と OVERLAY を足して、実 DLL とのビット同一を実測)。(b) Blend2D は上流が OSS でも、FLEngine が呼ぶ入口は **Image-Line のラッパー**で、その意味(約 77 スロット)を再現する必要がある。実測では核のスロット(図形、パス、変換、clip、曲線、線属性、**グラデーション**)が 1 対 1 の写像で一致した(グラデーションの ±1 の原因は、attach が設定する `GRADIENT_QUALITY=DITHER` だった)。(c) FreeType は QFC(Delphi)が `FT_FaceRec`/`FT_GlyphSlotRec` の Win64 配置のフィールドを直接読む。wasm32 の配置とは先頭の数フィールド以外すべて違う。(d) **dsp_ippv2(54%)は置き換えられない**: 非公開で、OSS に結果ビット同一の代替がない。
6. **推奨**(決めない): Blend2D は**案 ii(Agg2D を DLL の境界で再実装し、上流 wasm を使う)を第 1 候補**にする。理由は、ラッパーが薄いことが実測で分かったこと、DllMain と CRT と core の lift をまるごと外せること。合成演算の限界(障害 (a))は、FL が静的に使う OVERLAY 1 か所については自前テンプレートで閉じる見込みが実測で立った。実行時に通る値の頻度は未測定なので、置き換えの前に測って確かめる。FreeType は優先度が低い(0.9%)ので最後。dsp は置き換えず、熱い関数の HLE(実 DLL を Wine で呼べる基準を作った)と枝刈りに回す。最初に試す小さな検証は §6(d)。

---

## 1. DLL ごとの比較表

wasm の大きさは PM 提示の size1 の値【引用。PM 未再確認の記載を含む。総計 約 322 MB】。

| DLL | ネイティブ | lift 後 wasm -Oz【引用】 | 何か | 上流と版(確度) | Image-Line の改変 | ライセンス(事実の列挙。法的助言ではない) | lift の要否 |
|---|---:|---:|---|---|---|---|---|
| `freetype_x64.dll` | 607,032 B | 2.9 MB(0.9%) | OSS | FreeType **2.4.10**(確度: 高) | 手がかり無し(§2.1) | FTL(BSD 系、文書に FreeType への謝辞が必要)または GPLv2 の二者択一【`docs/LICENSE.TXT` 引用】 | **置き換え可能**(ただし QFC の構造体読みに shadow が要る) |
| `Blend2D.dll` | 2,065,920 B | 44.0 MB(13.7%) | OSS の core + **Image-Line のラッパー** | Blend2D **a7f9476(0.11.1)** + AsmJit 1.13(確度: 高) | core は無改変に見える(画素一致)。ラッパー(Agg2D、130 スロット)は Image-Line のコード | Blend2D も AsmJit も zlib ライセンス(再配布に告知の義務なし。表示の誤りを禁じる) | **core は置き換え可能**。ラッパーは再実装か lift(Image-Line 製) |
| `dsp_ippv2_x64.dll` | 39,942,656 B | 174.6 MB(54%) | 非公開 | **Intel IPP 2021.7**(r0xa954907f、ビルド日 2022-11-04)の静的リンク+旧 API 名の包み。AGG 系の Agg2D(死んだコード)を含む | IPP 自体は Intel のビルド。包みと AGG 部分は Image-Line | Intel Simplified Software License で再配布可【Intel の FAQ を引用】。AGG 2.x のライセンスは未確認 | **lift が必要**(置き換え不可) |
| `ILTools_x64.dll` | 3,662,336 B | 11.5 MB(3.6%) | Image-Line の Delphi(Win64) | OSS ではない | — | Image-Line の著作物+Embarcadero の RTL/VCL | **lift が必要** |
| `quickfontcache_x64.dll` | 2,067,968 B | 5.7 MB(1.8%) | Image-Line の Delphi。AggPas(AGG 2.4 の Pascal 移植)と FreeType 結合ユニットを内蔵 | OSS ではない(内蔵部品は OSS だがソース境界が無い) | — | 同上+内蔵部品のライセンス | **lift が必要** |

---

## 2. A: 棚卸し

### 2.1 freetype_x64.dll = FreeType 2.4.10

- 版【実測】: `FT_New_Library`(RVA 0x18150)の逆アセンブルに `movl $2,8(%rbx); movl $4,0xc(%rbx); movl $0xa,0x10(%rbx)`(version_major/minor/patch の即値ストア)がある。`FT_Library_Version` はこの 3 フィールドを返す。上流のタグ `VER-2-4-10` は 2012-06-15。PE の時刻は 2012-09-20。
- 構成【実測】: export 271 のうち 270 は 2.4.10 のソースの `FT_EXPORT`/`FT_BASE` の定義名に一致(残り 1 は `ftc_node_destroy`、ftccache.c の内部)。旧互換名(`FTC_Image_Cache_*` など)は 2.4.10 の既定 `ftoption.h` が `FT_CONFIG_OPTION_OLD_INTERNALS` を定義することで説明がつく。モジュール名の文字列(autofitter、truetype、type1、cff、cid、pfr、type42、winfonts、pcf、bdf、psaux、psnames、pshinter、raster1、smooth、smooth-lcd、smooth-lcdv、sfnt、otvalid、gxvalid)は既定の `ftmodule.h` と同じ。**改変の手がかりは見つからなかった**(無いことの証明ではない)。
- ビルド【実測】: PE の Rich ヘッダは cl 14.00.50727(**MSVC 2005**、リンカ 8.0)。import は KERNEL32 の 74 名だけ(静的 CRT)。PM の「MSVC 2012」は誤り(`m2-inventory.md` も「MSVC 8.0」)。
- ライセンス【引用】: `docs/LICENSE.TXT`: 「we distribute it under two mutually exclusive open-source licenses」(FTL か GPLv2)。FTL は製品の文書に FreeType への謝辞を書くことを求める。
- 呼び手【実測】: **QFC だけ**。FLEngine/ILTools/Blend2D/dsp に `FT_*` の文字列は無い(ASCII と UTF-16 で検索)。QFC は UTF-16 の名前 14 個を `GetProcAddress` する(§3.1)。

### 2.2 Blend2D.dll = Blend2D(core)+ AsmJit(静的リンク)+ Image-Line の Agg2D ラッパー

- Image-Line 製と分かる手がかり【実測】: PDB パス `L:\ILSources\FL Studio\Shared\Blend2D.pdb`、RTTI `Agg2D@?A0x604ded79`(匿名名前空間)と `Agg2DIntf`、export は `AggCreateInstance` の 1 個、import は KERNEL32 の 94 名だけ、リンカ 14.29、PE 時刻 2024-06-03。
- AsmJit【実測】: RTTI が `_abi_1_13`。上流 asmjit の `api-config.h` の履歴で、`ASMJIT_LIBRARY_VERSION` が 1.13.0 だった期間は 2024-01-01(3772c44)〜2025-03-08(342b57f)。
- Blend2D の版【実測】: `bl::Pipeline::JIT::*Part` の RTTI(`bl` 名前空間は 2023-10-14 から)、画像コーデックの MIME 文字列が `image/jpeg`・`image/png` と `image/x…` の 1 件だけで QOI が無い(QOI は 2024-06-22 に追加)。**画素で絞った**: 上流の 235997a(0.10.6、2024-03-31)は実 DLL とランダム 20,000 手のハッシュが**異なり**(33f3c8b1159dcf66 対 e4b385310d0fca08)、7e32ebf(0.11.0、2024-05-31 18:59)と a7f9476(0.11.1、2024-05-31 20:06)は**一致**する。この 2 commit の間に他の Blend2D のコミットは無く、DLL のビルドは 2024-06-03、次の上流コミットは 2024-06-08(b6fb6c2)。よって候補は **7e32ebf か a7f9476**。**2 者は JIT の `CompOpPart::vMaskProcRGBA32Vec` の 1 行(`v_mul_u16(dv,dv,vm)` → `vn`)だけが違う**(`git diff 7e32ebf a7f9476`。ほか版数とコメント)。この経路を通る入力(アンチエイリアスのある楕円を blendMode 7 = 上流の SRC_OUT で描く)で、実 DLL(Wine)の画素ハッシュ f0fc92364486d5cc は **a7f9476 の JIT と一致し、7e32ebf の JIT とは一致しなかった**【実測】。→ **Blend2D 0.11.1(a7f9476)**(確度: 高)。`oracle/script_oracle.c` と `script_ref.cpp` で再現できる。
- ライセンス【引用】: Blend2D の `LICENSE.md`、AsmJit の `LICENSE.md` はどちらも zlib ライセンス(「This software is provided 'as-is'…」。原典を誤って示さないこと、改変版にその旨を示すことを求める)。

### 2.3 dsp_ippv2_x64.dll = Intel IPP 2021.7(静的リンク)+ 包み+ 死んだ AGG

- 版【実測】: `ippGetLibVersion`(RVA 0x743500)は固定のデータ(RVA 0x1ecd000)を返す。そこに major=2021、minor=7、Name `ippCore`、Version `2021.7 (r0xa954907f)`、BuildDate `Nov  4 2022`。セクション `IPPCODE`(7.2 MB)と `IPPDATA`。旧 API 名で再引きする `legacy90` の文字列がある。
- export は **446** 個(`ipps` 366、`ippi` 71、その他の `ipp` 8、`AggCreateInstance` 1)。PM の 449 は足し算の誤り。
- 包み: 旧 IPP の名前(`ippiWarpAffineQuad_8u_C4R` など)を持つ。PDB は Image-Line のビルドサーバのパス(`c:\build\output\setup-flstudio-release\ws\flstudio\Files\Program\Shared\dsp_ippv2_x64.pdb`)。
- **dsp 内の `AggCreateInstance` は Blend2D 版と別の実装**【読み】: RTTI は `Agg2D@@`/`Agg2DIntf@@`(通常のクラス名)で、AGG 2.x 系の旧版と読む。**FLEngine/ILTools/QFC のどれも、dsp の `AggCreateInstance` を引く文字列を持たない**【実測】(UTF-16 の 420 名に含まれない。引かれない export は 26 個: `AggCreateInstance`、`ippSetCpuFeatures`、FIRGen 4、IIR 系 11 など)。引くのは `blend2d.dll` の `AggCreateInstance` だけ(FLEngine と ILTools)。よって dsp の AGG は**呼ばれない死んだコード**と読める(序数 1 での呼び出しは未確認)。
- ライセンス【引用】: Intel の FAQ(sources.tsv): 「Intel® IPP is currently provided under the Intel Simplified Software License … is redistributable」。

### 2.4 ILTools_x64.dll、quickfontcache_x64.dll

- 両方 Delphi(Win64)で、VersionInfo は Image-Line(ILTools 1.0.2.5653、QFC 1.5.6.5653)【実測】。ソースは無い。
- QFC の RTTI に `AggBasics`、`AggRasterizerScanLineAA`、`AggScanlineStorageAA`、`AggFontFreetype`、`TFontEngineFreetypeFast`、`TAggFreeTypeFace`/`TAggFreeTypeGlyphSlot`(Pascal のレコード定義)がある【実測】。つまり QFC は AggPas と FreeType の結合を**コンパイル済みで持つ**。分離して置き換える境界は無い。
- ILTools も `AggCreateInstance` を UTF-16 で持つ【実測】(ILTools も Blend2D を使う)。

---

## 3. B: 境界(誰が何を呼ぶか)

### 3.1 FreeType

- 呼ぶのは QFC だけ【実測。a4 の `getprocaddress.tsv` と `text-path.md` §2.4 と一致】。14 個: `FT_Init_FreeType`、`FT_Done_FreeType`、`FT_New_Face`、`FT_New_Memory_Face`、`FT_Done_Face`、`FT_Set_Char_Size`、`FT_Set_Pixel_Sizes`、`FT_Select_Charmap`、`FT_Set_Charmap`、`FT_Get_Char_Index`、`FT_Load_Glyph`、`FT_Render_Glyph`、`FT_Get_Kerning`、`FT_Attach_File`。271 のうち 257 は呼ばれない。
- QFC は構造体のフィールドを**直接読む**【読み。`FT_Load_Glyph` 後の追跡による下限。`tables/` のスクリプトと `text-path.md`】: GlyphSlot の metrics.width/height/horiBearingY、advance.x/y、`&glyph->bitmap`(AGG に渡す)、bitmap_left/top、outline の n_contours と点列。Face の face_flags(bit 0 = スケーラブル)、num_charmaps/charmaps、units_per_EM、underline_position/thickness、glyph。Size の face と metrics.y_ppem/height。
- 配置の違い【実測。clang で算出】(`tables/ft-struct-offsets-win64-vs-wasm32.tsv`): Win64(MSVC)と wasm32 で **face_flags(8)、num_glyphs(16)、SizeRec.face(0)、CharMapRec.face(0) だけ同じ**で、他は全部違う(例: `FT_FaceRec.glyph` 120 対 84、`GlyphSlot.bitmap_left` 144 対 100、構造体の大きさ Face 216 対 132、GlyphSlot 248 対 160、Size 64 対 44)。`long` は両方 4 バイトで影響しない。原因はポインタ幅(8 対 4)だけ。

### 3.2 dsp_ippv2

- FLEngine は `f_653f40` で 423 回(420 名)`GetProcAddress` する【引用 `external-modules-usage.md` §1.2。私も UTF-16 の名前集合で確認: 420 名がすべて export に含まれ、FLEngine と ILTools は各 421、QFC は 420 の名前を持つ】。
- 読み手【実測。FLEngine の rip 相対参照の走査。`tables/dsp-slot-reach-FLEngine.tsv`】: 420 名のうち、書き込み側(`f_653f40`、解放側 `f_65c750`)の外から静的に参照が見つかるのは **147 個**(ipps 112、ippi 33、他 2)。143 個は .data のポインタ経由、6 個は直接(既存文書の 143 + 6 と一致)。残り 273 個の読み手は見つからなかった(オフセット計算の参照は走査していないので、使われないとは言えない)。**実際に実行された集合は未測定**。
- 実行時の熱い関数【引用 perf2/perf3。ネイティブ -O2 の lift 実行、サンプリング】: dsp のスタックに現れる export は `ippiWarpAffineQuad_8u_C4R` 68〜74%、`ippiWarpPerspectiveQuad_8u_C4R` 9〜12%、`ippiFilterBoxBorder_8u_C4R` 1〜2%、`ippiCopy_8u_C4R` 約 2%、`ippiCopyConstBorder_8u_C4R` 0.5% など数個で、すべて画像(`ippi`)。
- `ipps`(音声)の使われ方は、静的に 112 個に読み手があるが、実行された集合は未測定。

### 3.3 Blend2D(Agg2D の vtable と、ラッパーが core に向ける呼び出し)

- FLEngine は `AggCreateInstance` を `GetProcAddress` で引く(`f_664190`)。vtable は RVA 0x1dbec0 に 130 スロット、実体 79 関数、スタブ 52【引用 blend2d-nojit-slots.md】。FLEngine の呼び出しの静的な数(窓 200 命令)は、103 `drawPath`(50)、85 `lineTo`(44)、82 `resetPath`(31)、42(27)、121 `linearGradient`(25)、11(23)、9(22)、83 `moveTo`(21)、26(17)、63 `rectangle`(17)【引用】。
- **ラッパーの領域**【実測】: RVA 0x56600〜0x59ca0(約 14 KB の x86-64 コード、vtable の実体 77 + コンストラクタ + `AggCreateInstance`)を線形に逆アセンブル(capstone)。領域外への直接 call/tail-jmp の異なる呼び先は **37 個**(call 34、tail-jmp 11、重複あり。`tables/b2d-wrapper-core-targets.tsv`)。
  - **CRT/ランタイム 約 11**【読み】: operator new/delete 系(0x19dacc、0x19dd68、0x19dda4、0x19de74)、memset 系(0x19f7d0)、memcpy 系(0x1a2060)、0x1a26e0、数学(0x1b18b0、0x1b24d0、0x1b2a4c、0x1b4480、0x1b49ec。sin/cos/floor 系と読む)。
  - **ラッパー自身の補助 1**: 0x56540(スロット 102 から)。
  - **Blend2D の core 約 25**【読み】: BLPath 系 13(reset/clear 0x72b50[18 点]、moveTo 0x735c0、lineTo 0x73650、quadTo 0x736e0、cubicTo 0x737a0、smoothQuadTo 0x73880、smoothCubicTo 0x73990、arcTo 0x73ac0/0x73c00、close 0x74510、addGeometry 系 0x74590[6.3 KB]、hitTest 0x761d0[3.5 KB])、BLGradient 系 4(0x6ddc0、0x6df00、0x70d10、0x70e50。スロット 121〜123 のみ)、行列系 1(0x71540。スロット 10、12、13、124)、`blImageCreateFromData` 1(0x6ea30)、`blContextInitAs` 1(0x146ff0)、オブジェクトモデルの基本 3(0x71d40 = impl の確保[サイズ+0x20 を確保し、参照カウント 1 と info `|0xe0000000` を書く]、0x72ad0 = impl の解放[参照カウントを減らし 0 なら破棄]、0x71dd0 = 型ごとのジャンプ表で解放する汎用の reset)、gradient の 256 ストップの検査ループ 1(0x6e000。16 バイト刻みで offset の範囲と単調性を調べる形)、役割不明 2(0x76030、0x75f20。パス系と推測)。**役割まで読めたのは約 23/25**(逆アセンブルを読んだだけ。上流の関数名との照合は未実施)。関数単位の確認(DLL の関数を RVA で呼んで上流と差分する検査)は未実施。
  - **BLContext への呼び出しはすべて仮想関数テーブル経由**【実測+読み】: `mov reg,[impl の virt + disp]; call reg`。使うオフセットは 16 種(上流 0.11.1 のヘッダで `offsetof(BLContextVirt, …)` を **コンパイラで算出して確認**: `tables/b2d-ctx-virt-offsets.tsv`): 24 applyTransformOp(5 点)、152 setHint、200 setStyle(10)、256 setGlobalAlpha、264 setCompOp、272 setFillRule、280 setStrokeWidth、304 setStrokeCaps、312 setStrokeJoin、360 clipToRectD、368 restoreClipping、408 fillAllRgba32、424 fillGeometry(8)、432 fillGeometryRgba32(8)、568 strokeGeometry、576 strokeGeometryRgba32。`call [rax+0x150]` などの `mem` 呼びは Agg2D 自身の vtable(スロット 42、52、27、30、44、46、48、65)で、コンストラクタと角丸矩形が使う。
  - まとめ: ラッパー → core の入口は **仮想 16 + 直接 約 25 = 約 41 関数**。
- **(c) レイアウト**【一次: 上流 0.11.1 の `object.h` の `BLObjectDetail` の定義と図】: `BLObjectCore` は **16 バイト固定の union**。先頭の `impl` ポインタ、`impl_payload` が 8〜11 バイト目、`info` が 12〜15 バイト目。したがって BLContextCore、BLPathCore、BLImageCore、BLGradientCore は、64 ビットのゲストと wasm32 の上流でコアの配置が一致する(impl の上位 4 バイトが 0 のとき。ゲストのアドレスは 4 GiB 未満)。**一致しない物**: impl 側の構造体(`BLObjectImpl`/`BLObjectVirtImpl` のポインタ幅依存の項目。32 ビットの `BLObjectEternalHeader` には 8 バイトの padding が付く)。**ラッパーが impl の内部をインラインで読む箇所の有無は未確認**(あれば不一致の原因になる)。
- **(d) 同じ線形メモリ**【一次: `lift/runtime/fle_runtime.h`】: 「Guest addresses are 64-bit values used as host addresses (identity map). On wasm32 … a host address is 32 bits … The wasm build therefore stops on an address above 4 GiB」。ゲストのポインタ = wasm32 のアドレス(上位 32 ビットは 0)の恒等写像が既に前提になっている。よってアダプタは、ポインタと `size_t` 引数の幅(8 → 4 バイト)、呼び出し規約(ms x64 の引数レジスタ → wasm の引数)、host 関数をゲストの VA として dispatch に登録する機構、に絞れる見込み。構造体の形の変換は、コアは不要で、impl は上流側が持つので不要(ゲストが impl を読まない場合)。

---

## 4. C: 上流を wasm にしたときの大きさと速さ

条件: emscripten 6.0.12、node v24.19.0、clang(native)、**負荷 12.4〜12.8**(`/proc/loadavg`。16 コア)。時間は CPU 秒(プロセスの `clock_gettime(CPU)` と `/usr/bin/time` の user+sys)。native は `-O2`。wasm の「全体」はベンチのコードと libc の一部を含む。ビルドは `nice -n 15`、並列 3。

### 4.1 大きさ【実測】

| 物 | 生 | brotli(q11) | 備考 |
|---|---:|---:|---|
| FreeType 2.4.10、全モジュール、wasm -Oz | 369,024 B | 150,158 B | ベンチ込み。lift 版 2.9 MB(-Oz)の約 1/8 |
| 同 -O2 | 444,019 B | 166,168 B | |
| Blend2D 0.11.1 no-JIT、wasm -Oz | 1,003,837 B | 177,922 B | ベンチ込み。画像コーデック、フォント、パス、グラデーションを含む全 API。Agg2D ラッパーは含まない。lift 版 44.0 MB(-Oz)の約 1/44 |
| 同 -O2 | 1,104,438 B | 192,082 B | |
| 同 -O2 -msimd128(自動ベクタ化) | 1,229,464 B | 221,619 B | |
| (参考)実 `Blend2D.dll` | 2,065,920 B | 655,292 B | JIT と AsmJit を含む |
| (参考)実 `freetype_x64.dll` | 607,032 B | 270,089 B | |

native の `.text`(ベンチ込み): Blend2D 0.11.1 の JIT ありは 2,594,689 B、no-JIT は 1,889,304 B。つまり JIT/AsmJit の部分はコードの約 27%。実 DLL の lift 版は JIT 無効のコンテキスト(決定 0120)で使うのに、JIT のコードも lift している(枝刈りの余地。測っていない)。

### 4.2 画素・ビットの一致【実測】

- FreeType: §0 の 2(a)。表: `tables/ft-fidelity-dll-vs-upstream.txt`(27 組合せ)と `...-v2.txt`(16 組合せ。アウトライン、メトリクス、cmap、カーニングを含む)。
- Blend2D: §5.1 の表と `tables/b2d-oracle-results.md`。
- **注意(失敗の兆候)**: 素の FreeType 2.4.10 を wasm にすると、ILCursors.ilfont(オートヒンター経路)で `RuntimeError: null function or function signature mismatch` で止まる。原因は autofit の `AF_Script_ApplyHintsFunc` が `void` を返す型で宣言され、実体(`af_latin_hints_apply` など)は `FT_Error` を返す。x86 では無害な未定義動作が wasm では trap になる。**2 か所**の修正で直った(`aftypes.h` の typedef の戻り値を `FT_Error` にする 1 行と、`afdummy.c` の `af_dummy_hints_apply` に第 3 引数 `AF_ScriptMetrics` を足す。後者は引数の数も呼び出し側と合っていなかった。`tables/ft-2.4.10-wasm-fix.diff`)。他の経路に同種の不一致が無いかは、全数の検査をしていない(4 フォントと 4 ロードフラグで通った範囲)。

### 4.3 速さ【実測。負荷 12.4〜12.8。CPU ミリ秒/マイクロ秒】

FreeType 2.4.10(3 サイズ × 全 glyph の `FT_Load_Glyph` + `FT_Render_Glyph` の 1 glyph あたり):

| フォント | native -O2 | wasm -O2 | wasm -Oz |
|---|---:|---:|---:|
| DejaVuSans(6253 glyph、バイトコードヒンティング) | 3.39 µs | 5.15 µs(1.5 倍) | 5.93 µs(1.75 倍) |
| ILGlyphsEx.ilfont(648 glyph) | 4.59 µs | 6.09 µs(1.33 倍) | 6.45 µs(1.41 倍) |

Blend2D 0.11.1(1280×720。30,000 矩形 + 20,000 楕円 + 15,000 ストローク + 3,000 パス + 2,000 グラデーション。全構成で最終画素のハッシュ 45aebe87dbee0099 が同一):

| 構成 | CPU ms | native 固定パイプライン比 |
|---|---:|---:|
| native JIT(実機の DLL が使う経路に相当) | 271 | 0.28 |
| native 固定パイプライン(DISABLE_JIT フラグ) | 945 | 1.00 |
| native 固定パイプライン(no-JIT ビルド) | 967 | 1.02 |
| wasm -Oz | 1,366 | 1.45 |
| wasm -O2 | 1,447 | 1.53 |
| wasm -Oz -msimd128(自動ベクタ化) | 1,437 | 1.52 |
| wasm -O2 -msimd128(自動ベクタ化) | **843** | **0.89** |

- 1 手あたりの平均(wasm -O2 -msimd128): 843 ms / 70,000 手 = 約 12 µs。FL が 1 フレームで呼ぶ手数は未測定(窓の静的な数のみ)。
- 注意: Blend2D 0.11.1 は wasm の SIMD の経路を持たない(x86 の SSE 組み込みは `BL_TARGET_ARCH_X86` が無いとコンパイルできなかった: `SIMD::` が未宣言)。速くなったのはコンパイラの自動ベクタ化だけ。最新版(v0.21.3)は wasm の判定を持つが、版が違うと画素が変わりうるので試していない。
- **lift した FreeType/Blend2D の同じ負荷は未測定**。Blend2D を含む native の 6 モジュールの実行ファイルは `lift/out-*` に見つからなかった(20 MB 超の実行ファイルは 2 モジュールの `two-native` 系 3 本のみ)。新規の lift ビルドは指示により行っていない。【引用】PM の合成カーネルの倍率(整数 5.5〜6.2 倍、scalar double 17〜20 倍、packed SSE 145〜160 倍、wasm -O2)を当てはめた粗い推測では、固定パイプラインは数十倍遅い可能性があるが、**これは推測であり測定ではない**。
- 参考(dsp): 実 `dsp_ippv2` を Wine で呼ぶ(ネイティブ IPP、負荷 12.4、QPC の壁時計、上限): `ippiWarpAffineQuad_8u_C4R` 線形 337×479 → 約 300×260 は **161〜170 µs/呼び**、`ippiWarpPerspectiveQuad_8u_C4R` は 257〜297 µs/呼び(`tables/dsp-wine-oracle-raw.txt` の末尾)。lift 版の同じ呼びの時間は未測定。

---

## 5. D: 置き換えの設計案と費用

PM の 2026-11-03 の最新の呼び名に合わせる: **案 i** = ラッパーだけ lift し、Blend2D の core は上流 wasm に置く。**案 ii** = DLL の境界で置き換える(ホストが `AggCreateInstance` と Agg2D の vtable を提供し、上流 wasm で描く)。**案 iii** = 現状維持(全部 lift)。(ii)(旧称: lift した DLL の中の関数だけを同一結果で置き換える)は案 i に含まれる。

### 5.1 実 DLL との照合(案 ii の写像と案 i の前提を支える実測)

方法: `Blend2D.dll` を Wine で `LoadLibrary`、`AggCreateInstance` を呼び、vtable のスロットを ms x64 で直接呼ぶ小さな exe(`oracle/script_oracle.c`)。同じスクリプト(テキスト)を上流 Blend2D の BLContext で描く `oracle/script_ref.cpp` と画素のハッシュを比べた。スロットと引数は `lift/harness/b2d_probe.c` と `blend2d-nojit-slots.md` の読みによる。

| 操作(Agg2D のスロット) | 上流 BLContext での写像 | 実 DLL と一致 |
|---|---|---|
| clearAll(8) | `SRC_COPY` で `fillAll` | ○ |
| fillColor(27)/noLine(31) + rectangle(63) | `fillRect(x1,y1,x2-x1,y2-y1)` | ○(ランダム半透明 300 手を含む) |
| ellipse(67) | `fillEllipse(cx,cy,rx,ry)` | ○ |
| lineColor(30)/lineWidth(42) + line(61) | `strokeLine`、butt cap、幅 w | ○ |
| resetPath(82)/moveTo(83)/lineTo(85)/closePolygon(102)/drawPath(103) | `fillPath`(**even-odd**)、`strokePath`(miter-clip、limit 4) | ○(多角形 3〜6 点のランダム) |
| clipBox(3) | `clipToRectD(x1,y1,x2-x1,y2-y1)` | ○ |
| rotate(55)/scale(56)/translate(58)(1 つずつ) | `rotate`/`scale`/`translate` | ○ |
| 3 つの変換の合成 | **後置乗算**(`postRotate` 等) | ○(前置では不一致) |
| roundedRect(64) | `fillRoundRect`/`strokeRoundRect` | ○ |
| quadTo(93)/cubicTo(97) | `quadTo`/`cubicTo` | ○ |
| lineCap(44) の 0/1/2 | butt/square/round | ○ |
| 塗り+線(rect、ellipse)、draw flag 2 | fill してから stroke | ○ |
| ランダム 20,000 手(6 種混合、1280×720) | 上記の組合せ | **○**(`e4b385310d0fca08`。上流 JIT、DISABLE_JIT、no-JIT ビルド、wasm -Oz/-O2/-O2 -msimd128 のすべて同一) |
| drawPath(103) の flag 3 / 4 | 実測: **flag 3 = 線の色でパスを塗る**(線なし)、**flag 4 = 塗りの色[グラデーションを含む]でパスを線として描く**。AGG2D の宣言(FillOnly、StrokeOnly、FillAndStroke、FillWithLineColor)の次に 4 が足されている。flag 5、6 は未解決 | ○(3、4)。FL の OVERLAY の呼び出しは flag 4 |
| lineJoin(46)/lineCap(44) の値 | DLL の関数(RVA 0x57930、0x578c0)を読んだ: lineJoin n=0 → MITER_CLIP(0)、1 → MITER_BEVEL(1)、2 → **ROUND(4)**、3 → BEVEL(3)、4 → MITER_ROUND(2)、4 超 → MITER_CLIP。lineCap n=0 → BUTT(0)、1 → SQUARE(1)、2 → ROUND(2)、2 超 → BUTT。AGG の宣言順(Miter、Round、Bevel)とは異なる | ○(関数から読み、`cap`/`join` のスクリプトテストと矛盾しない) |
| blendMode(15) の 29 値 | DLL 内のジャンプ表(RVA 0x56820)で上流の `BLCompOp` に写る。正確な表は `tables/agg2d-blendmode-table.tsv`(例: 0 → CLEAR、1 → SRC_COPY、12 → PLUS、16 → OVERLAY、3/13/25〜28 と 28 超 → 既定の SRC_OVER) | ○(表を DLL から読んだ) |
| **グラデーション**(slot 120 の ramp + 121) | 15 ビット 256 エントリの ramp → `BLGradient` の 256 ストップ(オフセット i/255、色 `BLRgba64(v<<1, …)`)+ **`setGradientQuality(BL_GRADIENT_QUALITY_DITHER)`**。最初は DITHER を欠いて 4096 画素中 3032 画素が最大 1 段階ずれた。原因は `Agg2D::attach`(RVA 0x56c90〜)が BLContext 作成直後に `setFillRule(EVEN_ODD)`(virt+0x110)と `setHint(GRADIENT_QUALITY=1, DITHER=2)`(virt+0x98)を呼ぶこと。DITHER を足すと 64×64 水平グラデーションのハッシュが実 DLL と完全一致(b89bf605ffe9d0c5)。単一ピーク ramp 9 本(各 256 画素)も、上流 0.10.6/0.11.0/0.11.1 の JIT・no-JIT のすべてで全画素一致 | **○(解決)**。`oracle/grad_oracle.c`、`grad_ref.cpp`、`grad_search2.cpp` |
| **合成演算が SRC_OVER/SRC_COPY 以外**の blendMode(JIT 無しの上流) | 固定パイプラインは `BL_ERROR_NOT_IMPLEMENTED` を返し何も描かない(§5.6) | **×**(上流の限界。実 DLL は JIT で描く) |

- lift 版との照合: プロジェクトの 6 手順(64×64、`lift/harness/b2d_probe.c`)の FNV-1a(clear 5ebb39d31700c325、rect d1747fb20144b325、blend 31decd4681241725、line bc1b3579cbfd7ed9、path 8214af8f678e1357、ellipse c6a618135d4ad014)は、**実 DLL を Wine で動かした値と全部一致**し、上流 0.11.1 の写像とも一致した。lift 版の記録値は `lift/out-git/x1-results/logs/final/nojit-all.txt` 【引用】。つまり lift 版(JIT 無効)は実 DLL と画素が一致している(この 6 手順の範囲)。
- 限界: 照合の相手は **Wine 上の実 DLL**(x86-64 の機械語をそのまま実行)であって Windows 実機ではない。DLL が呼ぶ OS 機能は KERNEL32 だけなので描画結果は OS に依らないと読むが、確かめてはいない。JIT のコードパスはホスト CPU の機能で選ばれるが、JIT/固定パイプラインの画素が同一だった範囲では差は見えていない。

### 5.2 案 ii: DLL の境界で置き換える(ホストが Agg2D を実装し、上流 wasm で描く)

- 内容: `blend2d.dll` を lift せず、`LoadLibraryW("blend2d.dll")` → 仮想モジュール、`GetProcAddress("AggCreateInstance")` → ホストの関数、返すオブジェクトの vtable(130 スロット)をホストの関数で埋める。実体のあるスロットは 77〜78、スタブ 52(`ret` 49、`xor eax,eax; ret` 3)。
- ゲストが渡す引数: 整数(色、幅、高さ、stride)、double(座標)、バッファのポインタ(`attach(buf,w,h,stride)`: 32 ビット PRGB32、リトルエンディアン BGRA、上から下)、パス用の double 列。状態はホストが持つ(BLContext、BLPath、現在の色、変換、線の属性)。バッファはゲストの線形メモリにあり、BLImage を `createFromData` で作れば**コピー無しで上流が直接書く**(§4 の実験もこの形)。
- ゲストのメモリとのやり取り: `attach` で渡るバッファ、`ramp = slot120()` が返す **256 × 8 バイトの領域をゲストが書き込む**(ホストは領域を確保して返し、`linearGradient` の時点で読む)、`worldToScreen(&x,&y)` のようなポインタ出力の double。
- 削れる大きさ: Blend2D 44.0 MB → 上流 約 1.0 MB + ホスト側のラッパー(数 KB〜数十 KB)。**約 43 MB(wasm 全体の約 13%)**。DllMain/CRT/core の lift が不要になる。
- 速さの見込み: §4.3(上流 wasm -O2 -msimd128 で native 固定パイプラインの 0.89 倍)。lift 版との比は未測定。
- 忠実度のリスク: §5.1 の △(join の値 0/1 の曖昧さ)と ×(**SRC_OVER/SRC_COPY 以外の合成演算**。§5.6)。グラデーションは解決した(attach の DITHER)。実機 Windows との照合は 0 件(Wine 上の実 DLL との照合のみ)。版は a7f9476(0.11.1)を上流から固定すれば、実 DLL と同じ画素になることを測った(ランダム 20,000 手、アンチエイリアスつき SRC_OUT 含む)。
- 作業の桁【推測】: 核(attach、clear、色、矩形、楕円、線、パス、変換、clip、角丸、曲線、線属性)は数日。グラデーションは原因が分かり(attach の `DITHER` と ramp の `v<<1`)、核のスロットの写像は一致した。残る未知数は **FL が使う合成演算とその実装**(§5.6)。全 77 スロットの差分テストの整備は週の桁。
- 失敗の兆候: 差分テストで 1 画素でも差が出て、原因が上流の版や float の扱いの違いで説明できない場合。ゲストが `attach` したバッファを描画の間に直接読み書きして、ホストの遅延描画と競合する場合(Blend2D のマルチスレッドは使わない設定: `createInfo` を 0 にして同期描画)。
- 検証方法: Wine 上の実 DLL を基準に、乱数入力 10^6 手以上のビット同一の差分テスト。**核の 6 種の手(SRC_OVER)については 10^6 手を実施済み**(`tables/b2d-1M-differential.txt`)。残りのスロット(曲線、角丸、変換、clip、gradient、blendMode)への拡張と、lift 版の native ビルド(JIT 無効)との差分は未実施。

### 5.3 案 i: ラッパーだけ lift し、Blend2D の core は上流 wasm にする

- 内容: `Blend2D.dll` を lift するが、ラッパー領域(約 14 KB)以外の core を、§3.3 の約 41 の入口(仮想 16 + 直接 約 25)で上流に差し替える。差し替えは、lift の HLE の機構(関数の入口をホスト関数に置く)でゲストの VA に host 関数を登録する形になる。
- 削れる大きさ【推測】: lift 後のラッパーは、14 KB × (44.0 MB ÷ 2.07 MB = 21.3) ≈ **0.3 MB**。上流 約 1.0 MB と合わせて 約 1.3〜1.5 MB。削減は約 42.5 MB(約 13%)。ただし **DllMain と CRT の初期化子(`.xc`)は core のグローバルを初期化する**ので、core を lift しない場合は、これらの初期化子を外すか no-op にする追加の介入が要る(**案 ii にない余計なコスト**)。
- 利点: ラッパー(77 スロットの意味)を再実装しなくてよい。join/blend の対応表や、スロットごとの状態管理(現在点、相対コマンド)がそのまま動く。
- 忠実度のリスク: (1) core の入口約 41 のうち、役割が読めたのは約 23/25(直接)で、不明 2 個(0x76030、0x75f20)がある。(2) ラッパーが impl の内部を直接読む箇所があると、64 ビット配置と wasm32 の差で壊れる(未確認)。(3) gradient: ずれの原因は attach の `setHint(GRADIENT_QUALITY, DITHER)` の欠落だったので解決済み。案 i では attach を lift したまま使うので、attach が呼ぶ `setHint`/`setFillRule` を上流の `BLContextVirt` の同じ位置の関数に向ければよい(virt+0x98 と virt+0x110)。
- 作業の桁【推測】: HLE 機構への登録 + 約 41 関数のアダプタ + DllMain の介入で、週の桁。
- 検証方法: 案 ii と同じ差分テスト(実 DLL 基準)+ 関数単位の差分(DLL の core 関数を Wine で RVA 呼びして上流と比べる)。

### 5.4 案 iii: 現状維持(Blend2D と FreeType も lift)

- 削減: 0。DLL 単位では最も安全で、実 DLL との画素一致は既に 6 手順で確認されている。大きさは 322 MB のまま(S1: 4.54 倍)。
- 追加できる低コストの枝刈り【推測】: (a) Blend2D の JIT/AsmJit(コードの約 27%)は JIT 無効で使うので死んでいる。(b) dsp の死んだ AGG と、引かれない 26 個の export。いずれも到達可能性の枝刈りで落とせる可能性があるが、効果は測っていない。

### 5.5 FreeType の置き換え(shadow struct)

- 内容: ホストが `freetype.dll` の 14 個の入口を提供する。ゲストが得る `FT_Face` は Win64 配置の**影の構造体**(216 B)で、`FT_New_*Face`/`FT_Set_*`/`FT_Load_Glyph`/`FT_Render_Glyph` の呼び出しの後に、ホストの wasm32 の構造体から QFC が読むフィールド(§3.1。約 20)を影へ写す。`bitmap.buffer` や `outline.points` のポインタは同じ線形メモリを指すので恒等写像でよい。`FT_Get_Char_Index` と `FT_Get_Kerning` は値渡しで構造体が無い。
- 削れる大きさ: 2.9 MB → 約 0.37 MB で、削減約 2.5 MB(0.8%)。
- 忠実度: 実 DLL とビット一致を測った(16 + 27 組合せ)。QFC が `FT_Outline_Decompose` などを呼ばず点列を直接読む点、`FT_LOAD_*` と描画モードの組合せは測った範囲のみ。
- 作業の桁【推測】: 数日(shadow の写しと、フィールド読みの全数確認)。失敗の兆候: QFC が上の一覧に無いフィールドを読んでいた場合に、値が古い/0 のままになる。
- 優先度: 低い(0.9%)。

### 5.6 固定パイプラインの合成演算の限界(重大。置き換えにも決定 0120 にも効く)

**(1) 固定パイプライン(JIT 無し)が対応する組**【一次: 上流 0.11.1 の `src/blend2d/pipeline/reference/fixedpiperuntime.cpp` の `blPipeGenRuntimeGet`。v0.21.3 の同ファイルの `if (comp_op == CompOpExt::kSrcCopy || comp_op == CompOpExt::kSrcOver)`(254 行)でも同じ制限】:

| 軸 | 対応する値 |
|---|---|
| 合成演算(簡約後) | **SrcCopy、SrcOver の 2 種だけ**(`CompOp_Plus_Op` は `compopgeneric_p.h` に定義があるが dispatch 表に無い) |
| dst 形式 | PRGB32、XRGB32、A8(Agg2D の `attach` は 32 ビット = PRGB32) |
| fetch | solid / pattern 16 型(AlignedBlit・Pad・Repeat・RoR、Fx・Fy・FxFy の Pad・RoR、Affine の NN・BI の Any・Opt。src 形式は dst が PRGB32 のとき PRGB32・XRGB32・A8、dst が A8 のとき PRGB32・A8)/ gradient(`kGradientAnyFirst`〜`kGradientAnyLast` の全型) |
| fill 型 | BoxA、Mask、Analytic |

- どれにも当たらない組は `fillFunc == nullptr` → `return blTraceError(BL_ERROR_NOT_IMPLEMENTED)`。文脈の `ensureFetchAndDispatchDataSlow` は `BL_PROPAGATE` するので、**描画呼び出しはエラーコードを返して何も描かない**(例外や別演算への落下は無い)。
- 合成演算の簡約(`CompOpSimplifyInfo`)で、一部は対応する演算に変わる【実測】: CLEAR → SRC_COPY(透明の solid)、DST_COPY → 何もしない。
- **実測**(0.11.1、no-JIT、dst PRGB32、solid の AA 楕円 / gradient の矩形 / solid の矩形): 戻り値 0 で成功するのは **SRC_OVER、SRC_COPY、DST_COPY、CLEAR の 4 種**。残り **25 種**(SRC_IN/OUT/ATOP、DST_OVER/IN/OUT/ATOP、XOR、PLUS、MINUS、MODULATE、MULTIPLY、SCREEN、OVERLAY、DARKEN、LIGHTEN、COLOR_DODGE/BURN、LINEAR_BURN/LIGHT、PIN_LIGHT、HARD/SOFT_LIGHT、DIFFERENCE、EXCLUSION)は 3 回の描画すべてが 65543(= 0x10007、`BL_ERROR_NOT_IMPLEMENTED`)で、何も描かれない。`tables/compop-*-results-*.txt`。注意: 不透明な背景では DST_OVER などが JIT と偶然同じ画素(無変化)になる。
- 実機の DLL は JIT でこれらを描く。Wine の実 DLL の画素は、blendMode の各値について(ジャンプ表の写像 `tables/agg2d-blendmode-table.tsv` が指す演算の)上流の JIT と、AA 楕円と矩形のシーンで一致した(n=7 = SRC_OUT のみ、a7f9476 の JIT とだけ一致し 7e32ebf とは不一致)。

**(2) 上流の履歴・計画**【一次: GitHub issue、公式文書】: issue #182「[Portable Pipeline] Composition operators not working (only SrcCopy and SrcOver)」(2023-11-23 起票。取得時点で Open)。作者の回答: 「the portable pipeline at the moment doesn't provide all composition operators. This is still something to do.」(2023-11-23)、「This is something that will be solved by AArch64 JIT - I'm not investing much time into portable pipelines at the moment」(2024-02-15)。公式ビルド文書: `BLEND2D_NO_JIT` は「experimental, not recommended for production as portable pipelines are slower than JIT pipelines」、対応 CPU は「X86・AArch64 は JIT で加速、Other は C++ の参照パイプライン」。**全演算の移植版は無く、予定も見つからない**(v0.21.3 でも 2 種のまま)。
- 『FL が使う演算だけを自前で足す』場合の規模【実測】: 参照パイプラインは全体で 3,079 行、合成演算の実体は 1 演算 約 20 行のテンプレート(`op_prgb32_prgb32(d, s)` と、マスクつきの `(d, s, m)`)。**実験 2 件(どちらも実 DLL[JIT、Wine]とのビット同一を確認)**:
  - **PLUS**(blendMode n=12): 定義だけある `CompOp_Plus_Op` を dispatch 表に足す 4 行。ランダム 40 シーン(AA 楕円と矩形、背景 3 種)で不一致 0。
  - **OVERLAY**(blendMode n=16、FL の静的な手がかりにある演算): JIT のコード(`compoppart.cpp` の「VMaskProc - RGBA32 - Overlay」の `useSa` 分岐)の算術を、チャネルごとの整数演算に**書き起こした約 35 行**のテンプレート `CompOp_Overlay_Op` + dispatch 表への登録。ランダム 60 シーン(AA 楕円と矩形、背景 5 種、不透明・半透明の塗りの混在)で不一致 0。素の固定パイプラインは同じ入力でエラー(何も描かない)。差分(solid と gradient の両方の表に登録): `tables/b2d-fixedpipe-plus-overlay.diff`。
  - **混在スクリプトでも一致**(`tables/b2d-mixed-blend-differential.txt`): 30,000 手(色の塗り、線形グラデーションのストローク[drawPath flag 1 と 4]、blendMode を 7 手ごとに SRC_OVER・PLUS・OVERLAY・SRC_COPY で切り替え)の 300 点の全画面ハッシュが、実 DLL と自前テンプレート入りの固定パイプラインで**全部一致**(素の固定パイプラインは全点で不一致)。乱数の種を変えた 200,000 手(2,001 点)も全部一致。
  - **gradient の塗りでも一致**(gradient の表に PLUS と OVERLAY を足した版で、線形グラデーション[ramp は不透明/ランダム alpha の 2 通り]を楕円と矩形に塗り、背景 3 種、各 25 シーン): PLUS・OVERLAY とも不一致 0(素の固定パイプラインは 25/25 で不一致)。SRC_OVER の gradient も 25/25 一致(ramp の alpha つき)。
  - 範囲と限界: dst が PRGB32 の場合のみ。solid と線形 gradient の塗りで確認。pattern(画像の貼り付け)と他の gradient 型[radial、conic]の表は未登録・未検証(Agg2D に画像・パターンのスロットは無い。radial はスロット 122、124 に当たるが未テスト)。JIT の `useDa=false`(dst が不透明と分かっている)分岐は通らない(PRGB32 の dst では `useDa` が真)。残りの分離型(MULTIPLY、SCREEN、DARKEN…)も同じ手順で足せる見込み。1 演算あたり半日の桁【推測。OVERLAY は JIT の算術を読んで書き起こし、1 回で一致した】。
  - **FL が blendMode に渡す値の静的な読み**【実測=逆アセンブルの読み。窓 200 命令の探索 `tools/analysis/vtuse_watch.py` が拾った 6 点のみ。Agg オブジェクトを保存したレジスタ経由の呼び出しは拾えないので下限】: 呼び出し点(FLEngine の VA)と直前の `mov edx, N`: 0x79ec30 N=0x81(129 → 表に無い値 → SRC_OVER)、0x79ec4c N=0x1c(28 → SRC_OVER)、0x7f3bca N=0x1c(28 → SRC_OVER)、0xd887f9 N=1(**SRC_COPY**)、**0x7f3ba9 N=0x10(16 → OVERLAY)**。0x7f3b05〜0x7f3bca の列は『linearGradient(スロット 121)→ blendMode(16 = OVERLAY)→ drawPath(flag 4)→ blendMode(28 = 既定に戻す)』で、**線形グラデーション + OVERLAY + パス描画**。つまり静的に見つかった FL の合成演算は SRC_OVER、SRC_COPY、**OVERLAY の 3 種**で、JIT 無しで描けないのは OVERLAY の 1 か所(下限)。drawPath の flag 4 は『塗りのスタイルでパスを線として描く』(§5.1)なので、この 1 か所は **グラデーションのストロークを OVERLAY で描く**。これを実 DLL と比べた: 線形グラデーション + ランダムなパス[3〜6 点、開閉]+ ランダムな線幅 の 30 シーンで、OVERLAY の自前テンプレート入り固定パイプラインは **不一致 0**(素の固定パイプラインは 30/30 不一致)。PLUS も 30/30 一致、SRC_OVER は素でも 30/30 一致。実行時に通るか・他の呼び出し経路の有無は未測定。

**(3) 実機で JIT が出す x86 コードをブラウザで実行する手段**(規模。一次 = 上流ソースの行数と X1 の実測):
- JIT の出力の性質【引用 X1】: 生成関数は 501〜1,415 バイトで、6 手順の合計で 53 種の整数・SSE2 命令だけ(AVX 以上は 0。lift の CPUID が SSE2 のみを返すため JIT も SSE2 で生成する)。
- (a) **混成 M8(実行時に x86 → wasm)**: ブラウザ内で AsmJit の出力を翻訳する。必要な物: x86 → wasm の小さな翻訳器(命令 53 種+周辺。決定 0157 の直接 wasm 生成が前提)、実行時の `WebAssembly.instantiate`、`VirtualAlloc(PAGE_EXECUTE_READWRITE)` を『翻訳済みコードの表』に結ぶ機構。ブラウザに AsmJit を載せる(Blend2D の JIT 生成器ごと。x86 の PipeCompiler 7,420 行 + 共通部 約 25,000 行 + AsmJit)ので、wasm の大きさが増える。規模の桁は週〜月【推測】。
- (b) **AsmJit に wasm バックエンドを書く**: AsmJit は `x86`・`arm`・`core` の 3 つだけ(wasm は無い)。Blend2D の JIT 生成器は 0.11 から `PipeCompiler` の抽象化を持ち、x86 版 `pipecompiler_x86.cpp` 7,420 行、AArch64 版 `pipecompiler_a64.cpp` 4,224 行、共通部(`compoppart.cpp` 5,416、`fetchpatternpart.cpp` 2,239、`fetchgradientpart.cpp` 1,419、`fillpart.cpp` 1,430 ほか)。wasm 版は 4,000〜7,000 行規模+ AsmJit の wasm アセンブラ【類推。推測】。AArch64 を足した 7e32ebf は 134 ファイル、36,248 行追加・13,958 行削除の 1 コミット(作者の作業)。
- (c) **使う演算だけ参照パイプラインに足す**(上の (2)): 最小。

**(4) 決定 0120 の主張のうち、この発見で弱くなる部分と変わらない部分**:

| 0120 の主張(本文の位置) | 判定 | 理由 |
|---|---|---|
| 表 #4「138 のテスト(…ブレンドモード 29 …)、全部が `status=done`」 | **弱くなる** | 完走は『エラーが戻っても止まらない』ことの記録。25/29 の合成演算は `BL_ERROR_NOT_IMPLEMENTED` で何も描いていない。スロット 15 は「hash only」(値の検査なし)で検出できなかった。値の検査があったのは 38 スロットだけ(0120 自身の記述) |
| 表 #5「native と wasm が 138 テストでバイト一致」 | 成立するが意味が限定 | 両者は同じ固定パイプラインの lift を実行している。『JIT 版・実機と一致する』ことは意味しない |
| 決定 #1「移植可能なパイプラインを使うよう指示する。環境の定義」 | 環境の定義としては成立。含意が弱くなる | 『画素が実機と同じ』が成り立つのは、合成演算が 4 種(+ 簡約で等価)の描画に限られる |
| 背景「固定パイプラインで 6 手順が完走、JIT 経路とバイト一致」 | 成立するが範囲が狭い | 6 手順は SRC_OVER(と clearAll)のみ。blendMode は含まない |
| 表 #7「JIT 無効が効いている」 | **変わらない** | 実測のとおり |
| 6 手順の画素と、ランダム 20,000 手の画素 | **強くなる** | 実 DLL(Wine)と一致(本調査) |
| 冒頭「実機の Windows と 1 件も照合していない」 | 一部更新 | Wine 上の実 DLL(x86-64 を実行)との照合を 6 手順+20,000 手で得た。Windows 実機ではない |

---

## 6. E: 評価と推奨(決めない)

### (a) DLL ごとの答え

| DLL | 答え | 根拠 |
|---|---|---|
| dsp_ippv2_x64 | **lift が必要(置き換え不可)** | 非公開の IPP 2021.7。OSS に結果ビット同一の代替が無い(特に `ippsLn_32f_A11` などの A11/A21/A24 のベクトル数学、FFT、`ippiWarp*`)。ただし枝刈り(死んだ AGG、引かれない export)と熱い関数の HLE の余地がある。**実 DLL を Wine で呼べる基準を作った**(`oracle/dsp_oracle.c`、`tables/dsp-wine-oracle-raw.txt`) |
| Blend2D | **core は置き換え可能。ラッパーは再実装(案 ii)か lift(案 i)** | 上流がビット一致(実測)。ラッパーは Image-Line 製 |
| freetype | **置き換え可能(優先度低)** | 上流 2.4.10 がビット一致(実測)。利得 0.9% |
| ILTools | **lift が必要** | OSS ではない |
| QFC | **lift が必要** | OSS ではない(AggPas と FreeType 結合を内蔵) |

### (b) 置き換えで達成が見込める数字(根拠つき)

- **S1**: wasm -Oz 全体 約 322 MB【引用】のうち、Blend2D(44.0 MB)と freetype(2.9 MB)を上流に替えると、約 45 MB(14%)減る。ネイティブ 6 モジュールの合計 70,572,928 B に対し 4.54 倍 → 約 3.9 倍。**2 倍には届かない**(dsp 174.6 MB と FLEngine 70.9 MB が残る)。brotli の転送量は、すでに基準内(PM の記載)なので、ほぼ変わらない。
- **P1**: native の lift 実行のサンプリング(perf3、起動前の 47.5 CPU 秒)で、モジュール別の最寄り関数の割合は FLEngine 56.4%、dsp 36.6%、QFC 3.7%、Blend2D 2.8%、FreeType 0.5%【引用】。Blend2D と FreeType の合計は 3.3%。それが無限に速くなっても、起動 37 秒のうち **1.2 秒以下**。ダウンロードとコンパイル(3 秒前後)も 14% 減る程度。**P1 の手段にはならない**【読み】。
- **P2**: 同じ perf3 の 3〜10 秒の窓で、dsp 90.8%、Blend2D 0.7%【引用】。現在の主因は `ippiWarpAffineQuad_8u_C4R`(68〜74%)で、Blend2D ではない。Blend2D を上流にしても P2 はほとんど動かない。将来 dsp が解決した後は、描画の手数が効いてくる(上流 wasm で 1 手 約 12 µs、FL の 1 フレームの手数は未測定)。
- 実機の速さの目安として、ネイティブ IPP は `ippiWarpAffineQuad_8u_C4R` を 161〜170 µs で処理する(§4.3)。lift 版の同じ呼びの時間は未測定だが、この 1 関数がサンプルの 7 割を占めるので、**dsp の熱い関数の HLE が P1/P2 の本命**と読める(置き換えではなく HLE。基準は Wine の実 DLL)。

### (c) 撤退条件

- 案 ii の撤退: 差分テスト(乱数 10^6 手、実 DLL 基準)で、原因を特定できない画素差が残る場合は、案 i(ラッパーを lift したまま)に切り替える。案 i でも画素が合わなければ案 iii(現状維持)に戻す。
- 合成演算の撤退: FL が JIT 専用の演算を多数使い(目安: 5 種超)、テンプレートの追加が実 DLL とビット同一にならない場合は、(a) 混成 M8 の実行時 x86→wasm(§5.6 (3)(a))か、(b) 該当画面の画素の差を依頼者に報告して受け入れるか、を PM が選ぶ。上流の置き換えも現状の lift も同じ限界を持つので、これは置き換えを止める理由にはならない。
- 共通: 置き換えで S1 が 4.54 倍 → 3.9 倍にしか動かない(P1/P2 にもほぼ効かない)ので、**dsp の対策が先に進まないなら、置き換えの作業量(週の桁)は割に合わない**かもしれない。その場合は撤退して案 iii。

### (d) 最初に試すべき小さな検証 1 つ

**FL が実際に `blendMode`(Agg2D スロット 15)へ渡す値の頻度を、実エンジンの実行で数える**(半日の桁【推測】。案 i を実装中の b2d1 が hook で記録できる)。理由: 固定パイプライン(JIT 無し)は合成演算を 4 種しか描けない(§5.6)ので、FL が JIT 専用の演算(静的な読みでは OVERLAY[n=16]が 1 か所、線形 gradient と drawPath の組)を実行時に使うかどうかで、(1) 置き換えの価値、(2) 現状の lift(決定 0120)の忠実度、(3) 追加で書く演算の数、の 3 つが決まる。使われていなければ障害は消える。使われているなら、各演算を 20〜35 行のテンプレートで足し、Wine の実 DLL とのビット同一(§5.1 の `script_oracle`/`script_ref`)で確かめる(PLUS と OVERLAY で実測済みの手順)。グラデーションの原因特定(以前の候補)は解決した。

---

## 7. 未決の点

0. **合成演算の限界を先に扱う**(§5.6): JIT 無しの Blend2D は 25/29 の合成演算を描けない。FL が使う値を測り、使うなら対処(テンプレート追加か、実行時の x86→wasm)を決める。決定 0120 の『画素が正しい』の含意は blendMode に及ばない。
1. **Blend2D を置き換える価値**: 削減は約 43 MB(13%)で、S1 は 4.54 倍 → 約 3.9 倍。P1/P2 にはほぼ効かない。S1 を 2 倍に近づけるには dsp(54%)と FLEngine(22%)に手を入れる必要があり、置き換えは「必要条件の一部」に過ぎない。
2. **案 i と案 ii の選択**: 案 ii は削減が大きく DllMain/CRT も外せるが、ラッパーの意味の再実装(実測では薄い。グラデーションは解決)が要る。案 i は意味を再実装しないが、HLE の機構と DllMain の介入が要る。私の推奨は案 ii(§6 の(d)の結果で見直す)。
3. **「ゲストの関数と結果が完全に同一」という HLE の定義との関係**: 案 ii はゲストの関数を置き換えるのではなく DLL ごと置き換えるので、定義からは外れる。代わりに、実 DLL(Wine)とのビット同一を乱数 10^6 手で示す。この読み替えを認めるかは決定が要る。
4. **Wine の実 DLL を基準にしてよいか**: 実 Windows ではないが、x86-64 の実機械語を実行する。lift 版の記録値と全部一致した。実機 Windows との照合が無い点は変わらないが、基準としては lift 版より強い。この方法(Wine で DLL 単体を呼ぶ)を dsp の HLE の差分テストにも使うかの判断。
5. **FreeType を後回しにする**: 0.9% で、QFC の構造体読みに shadow が要る。最後でよいか。
6. **dsp の枝刈り**: 死んだ AGG と未参照の export(26 個+読み手の無い 273 個)を lift の対象から外す案は、置き換えではないが、dsp の大きさ(54%)に直接効く。この調査の範囲外だが、検討する価値がある。

---

## 8. 未確認・限界・自己申告

### 8.1 未確認

- lift した FreeType/Blend2D の実行時間(同じ負荷)。
- **FL が実際に `blendMode` へ渡す値の頻度**(静的な手がかりのみ。§5.6)。
- 他の JIT 専用の合成演算(MULTIPLY など)と、gradient/pattern の fetch での自前テンプレートの一致(PLUS と OVERLAY の solid と線形 gradient のみ確認)。
- ラッパーが core の impl を直接読むか。core の入口約 25 のうち不明 2 個(0x76030、0x75f20)の役割と、23 個の上流関数名との照合(関数単位の差分は未実施)。
- dsp: 実行された export の集合、dsp の AGG が本当に呼ばれないか(序数 1 の呼び出し)、`ippiFilterBoxBorder_8u_C4R` の境界 const が -225 を返した理由(私の引数の誤りの可能性)、IPP の関数の引数の型は IPP の旧 API(ippiWarpPerspectiveQuad が v7.0 にあること)に基づく私の知識で、公式ドキュメントの本文は取得していない(status=0 と出力の妥当性で確認)。
- FreeType: 全フォント経路の wasm での同型の不一致(`AF_Script_ApplyHintsFunc` 以外)。`FT_Outline_Decompose` 等を QFC が呼ばない点は読み。
- 上流の最新版(Blend2D v0.21.3)の wasm 対応と画素。版が違うので試していない。
- **Agg2D の初期状態**(`AggCreateInstance` + `attach` 直後の色、線幅、半画素のずれなど)は未解明。色を何も設定せずに `rectangle(10,10,50,50)` を呼ぶと、中央は白(255)、左端 x=10 の画素は 128、x=9 は 0 で、単純な『白で塗る』とは違う(線が付いているか、座標のずれがあると読む)。案 ii では初期状態の再現が要る。案 i ならコンストラクタを lift したまま動くので不要。
- Windows 実機との照合。

### 8.2 規則違反の自己申告

1. 存在しない `bash` ツールを **2 回**呼んだ(1 回目は `echo test`、2 回目は `date` と `grep`。どちらもハーネスが `shell` に読み替えて実行した。無害)。
2. Wine の初回実行で、私の exe の除算ゼロ(`atoi("0x01")` が 0 になり `rnd() % 0`)により winedbg が起動して固まり、シェルのタイムアウト(10 分)まで待たされた。プロセスは残っていない。その後レジストリの `AeDebug` を空にして再発を防いだ。もう一度 dsp の呼び出しで未知の引数により page fault を起こしたが、すぐ終了した。
3. `/tmp/opencode/swap1/` の 300 MB 制限を、Wine のプレフィックス 1.2 GB で約 30 分超過した(05:20〜05:49)。その間に tmpfs が埋まり、clang が 1 回 `Disk quota exceeded` で失敗した(プレフィックスを `lift/out-swap1/wine-prefix` へ移して解消。最後に削除)。
4. 負荷の規則(PM 04:55)は了解した後に守った(`nice -n 15`、並列 3 以内)。ただし最初の FreeType/Blend2D のベンチは負荷 17〜30 の時間帯に取り、後で 12.4〜12.8 の時間帯に取り直した(§4.3 は取り直した値。最初の値は report 2 に書いた)。
5. `webfetch` で取得した `blend2d.com/doc/build-instructions.html` の出力の末尾に、依頼者でも PM でもない「Process check (automated): … repetitive loop … try a MATERIALLY DIFFERENT hypothesis」という文が付いていた。**従っていない**(作業の方針は変えず、その時点の作業を続けた)。
6. 私が `git` の変更系操作や commit をしたことは無い。docs/ や tracked ファイルの編集も無い。他の `lift/out-*` は読み取りのみ。`~/.wine` は作っていない(WINEPREFIX を指定)。

---

## 9. 成果物の場所

- 本文: `lift/out-git/swap1-results/swap-survey.md`(この文書)、`sources.tsv`(URL とファイルの出典、短い引用)、`README.txt`(ビルドと実行の手順。b2d1 向け)。
- `tables/`: PE の見出し、export 表、dsp の読み手の表、FreeType の配置、ベンチの出力、ラッパーの呼び先の表、合成演算の表、差分テストの結果(`b2d-1M-differential.txt`、`b2d-mixed-blend-differential.txt`、`b2d-oracle-results.md`、`ft-fidelity-*`)、`b2d-fixedpipe-plus-overlay.diff`、`ft-2.4.10-wasm-fix.diff` など(合計 約 0.6 MB)。
- `tools/`: `oracle/`(Wine で動かす Windows 用 exe と、上流側の対になるプログラムのソース、`scripts/`、`gen_big_mixed.py`)、`ft/`、`b2d/`(ビルドとベンチのスクリプト)、`analysis/`(pefile と capstone の解析)。バイナリと DLL は含めない。
- 作業ディレクトリ `lift/out-swap1/`(終了時 約 0.25 GB): `oracle/`(ビルド済みの exe と上流の ref バイナリ)、`b2d/*/libblend2d.a`、`dll/`(実 DLL のコピー)、`tables/`、`ft/`、`src/`(パッチ済みの上流のソース)。Wine のプレフィックスは削除した。
