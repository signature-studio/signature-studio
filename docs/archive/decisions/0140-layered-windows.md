# 0140: レイヤード窓(UpdateLayeredWindow / UpdateLayeredWindowIndirect / SetLayeredWindowAttributes / GetLayeredWindowAttributes)のモデルと、ウェルカムウィザードを閉じた先

状態: **仮の決定**。PM の指示(ウィザードを閉じると `indirect call to 0`(`f_71e570` が `GetProcAddress` で取った `UpdateLayeredWindow` の変数が 0)になる壁を、スタブでなくモデルで通し、その先を連続して直す)を、作業者(lay1)が調査・設計・実装・測定した(2026-11-02、HEAD fa11947 で作業し、最後に HEAD 93881c0 に付け替えた)。仮にする理由: Learn に値が書かれない失敗時のエラー番号・メッセージの有無・失敗の条件は名前つきの選択(下の S- の表)で決めた。実機の Windows との照合は 0 件。凡例: **実測** = 動かして確かめた、**読み** = 逆アセンブル・Learn の読み、**未確認** = 確かめていない。

## 先に結論

| 項目 | 内容 |
|---|---|
| ゲストは何に使うか(**実測**) | **ウィザードを閉じた直後に始まる『FL のイチゴのロゴ(337×479 の RGBA 画像)が画面中央へ縮みながら消える演出』の窓**。クラス `TLightweightLayeredControl`(lid 18、`WS_EX_LAYERED \| TOPMOST \| TOOLWINDOW`、起動時は 16×16 で不可視)。**thread 2 の `TLayeredControlThread` が 1 フレームごと(仮想 16 ms)に `UpdateLayeredWindowIndirect`(`f_71e570` の 0x71e68a)を 1 回**呼ぶ。`UpdateLayeredWindow`(0x71e6d4)は同じ関数の変数が 0 のときの代替の経路で、実際にはどちらの変数も `GetProcAddress` で取れれば Indirect が使われる。1 回目 ret=0x71e68c、T=2。 |
| 1 回の呼び出しの中身(**実測**) | `hdcDst`=NULL、`hdcSrc`=そのフレームで `CreateCompatibleDC(0)` + `CreateDIBSection(32bpp BI_RGB、top-down)` + `SelectObject` した**新しいメモリ DC**(フレームごとに作って捨てる)、`pptSrc`=(0,0)、`crKey`=0、`pblend`={AC_SRC_OVER, 0, **SourceConstantAlpha 255**, **AC_SRC_ALPHA**}、`dwFlags`=ULW_ALPHA。`pptDst`/`psize` は毎回変わる: 1 回目 (794,303) と 334×476(**ウィザード lid 48 の矩形(523,255 874×570)とは無関係**)、300 ms で 328×464、2.3 s で 318×444、10.3 s で 284×400、50 s で 246×340(3,144 フレーム目、(845,377))。**頻度 = 1 フレーム 1 回、止まらない**(50 仮想秒の間、縮小は続く。終点には届かなかった。下の「アニメは終わらない」)。 |
| ソースのビットの中身(**実測**) | 1 フレームの流れ(thread 2。`FLE_TRACE` の関数ラップで取った): `f_7205c0`(描画)→ `f_6c7350`(枠の描画。色 0、α=0x100)→ **`f_6c1190` = `ippiWarpAffineQuad_8u_C4R`**(源 = ゲストのヒープの 337×479・step 1348 の BGRA、非零 296,416 バイト = **FL のイチゴのロゴ(組み込み PNG をゲストが復号したもの。画面キャプチャではない)**。出力 = ここで作った DIB 334×476、非零 292,991 バイト)→ 枠(非零 246,000)→ `f_6c1260`(合成。非零 178,027)→ **`f_6c8d00(bitmap, rect, α)`(xmm2 = 0.00198813)= 全バイトに α を掛ける(IPP MulC、定数 round(0.00198813 × 256) = 1、スケール 8)→ 非零 0** → `f_71e570`。つまり **ULW に渡るビットは全部 0(RGB も A も)**。0 の原因は**ゲストの計算(α = 0.00198813、269 フレームの間 1 つの値のまま)で、環境の取りこぼしではない**(画面 DC の読み取りは関与しない: thread 2 のフレームの間に BitBlt / StretchBlt / GetDIBits は 0 回)。ロゴ自体は IPP の出力に正しく出ている(`logs/shots/ipp-warp-output-334x476-before-guest-alpha-scaling.png`)。 |
| 実機の α は何か | **未確認**。α = 0.00198813 は `f_7205c0` の 0x720f4e の式(`(x − c1) × c2 + c3` を [0, 1] に丸める。定数は読み違えの恐れがあり読み切っていない)の結果で、アニメの状態量([this+0x104] ほか)から来る。縮小の速さも実機より遅い疑い(334×476 → 246×340 に 50 仮想秒)。PM の未決点 2。 |
| モデル | 新規 `runtime/fle_layered.c/.h`(4 API)。**窓の面は fle_raster の top-level 窓の面そのまま**(決定 0126。ULW の窓では 4 バイト目 = アルファ、B・G・R は事前乗算。他の窓の 4 バイト目は 0 のまま)。定数アルファ(SourceConstantAlpha / bAlpha)とカラーキー(SLWA)は面ではなく**属性**として持つ(`fle_layered_query`)。合成(下の窓との重ね合わせ)は runtime の責務にしない。 |
| 面の表の ABI | 変更案を下の「面の表の ABI 変更案」に書いた(app/ は触っていない)。**今の app は lid 18 を『不透明な黒い 328×464 の箱』として出すはず**(面の表の 4 バイト目が A なのに BGRX として読み、A = 255 を強制するため。**app では未確認**)。 |
| ウィザードを閉じた先の壁 | 名前付きの停止は**この壁(`indirect call to 0`)だけ**。入力経路(`TranslateMessage` の制御文字・`_TrackMouseEvent`・`OpenProcessToken`・`CompareStringW` の制御文字)は G3 の決定 0136 で HEAD に入っていた。この壁を通した後は、Enter・Esc・『No thanks』のクリック → Enter・Enter 2 回のどれでも**名前付きの停止は出ない**(50 仮想秒 = 3,144 フレームまで)。 |
| 新しい idle(fa11947 + lay1、**native -O2 = wasm -Oz(node)で一致**) | Enter 後の主窓 lid 13 の SHA は 2.3 仮想秒以降 **`4aa007b195a65787230db1f21b1e13307b831dbfcd45d977ea1acdb006d659b2`**(t = 2.3 / 3.3 / 4.3 / 6.3 / 10.3 s で同一。native は 50 s まで同一)、Settings(lid 22)は 1.3 s 以降 **`a368d8abb83f1a469f1ee2aa52164951e8b45e71f2e424709e66bb6837958d47`**。lid 18 の層は縮小アニメを続けるので時刻ごとの SHA を比べた(8 時刻すべて一致、下の表)。ウィザード前(第 1 の idle)の lid 13 `721b792e…` は不変。 |
| 新しい idle(HEAD 93881c0 = 音声担当の DirectSound モデルを含む木、native -O2) | **Settings(lid 22)がそもそも出ない**(窓表に TMIDIForm が無い)。Enter 後の lid 13 は 2.3 s 以降 `907ab7ee…`(4.3 s まで同一)。**wasm -Oz(node)も一致**: wiz0 `721b792e`、0.3 s `adbc77c0`、1.3 s `df5a10b6`、2.3 s と 4.3 s `907ab7ee`、lid 18 は 328×464 `ae29bd5b`・320×452 `66e7c949`・318×444 `71df48af`・314×434 `143cd5ce`(4 分 20 秒)。HEAD ごとに『新しい idle の SHA』は違う(PM の未決点 5)。 |
| 全体テスト | **fa11947 + lay1: 136 result・2,001 passed・0 failed・0 ignored、終了 0**(`cargo test --workspace --no-fail-fast --offline`、1 回の通し、2026-11-02 01:13:22Z〜02:01:18Z = 47 分 56 秒。基準 fa11947 の G3 報告 135 result・1,984 passed に対し +1 result(`wall_fr16`)・+17 passed)。**1 回目(00:11:52Z 開始)は 60 分の上限で打ち切られた**(107 result の時点で 1 件失敗 = `wall_fr14` の行数の固定、直して最初からやり直した)。`wall_fr16` の destroy のテストに『70 窓の作って壊す』を足したのは通しの**後**(件数は 17 のまま。単独で再実行して pass)。**HEAD 93881c0 に付け替えた後は全体を通していない**: 行数に触れる 8 ファイル(`wall_fr4/5/13/14/15/16`・`vcall_thunk_dll`・`imports_table`)だけ再実行して pass。 |
| a4(FRUITY 無効、独立 2 回、fa11947 + lay1) | offA・offB どちらも 942/942、895/895、functions 5,310、entries 1,009,721、終了 0(build 105 秒)。**最初の試行は `MemoryMax=4G` で build の途中で kill された**(7G にして成功)。 |
| 変異検査 | **17 件すべて検出**(木のコピー上、1 件ずつ変更 → `wall_fr16` が落ちる → 元に戻す)。最初の通しで 14 件、生き残り 3 件の内訳: M03 は**同値変異**(`sca` は ULW_ALPHA 以外では 255 のままなので『常に sca』は同じ振る舞い。『SourceConstantAlpha を読まない』に変えて検出)、M08 は変異の文言が木に無かった(直して検出)、M14 は**テストの穴**(破棄した窓の状態が残っても見えなかった。70 窓のループを足して検出)。`logs/mutations.txt`、`mutations-rerun.txt`。 |
| gencmp | clean の `fe-lift`(HEAD fa11947)と編集後を、a4-fruity.sh on-0129 の引数(+ `--seh-handler 0x112c0`)・同じ種・同じ DLL で比べた: FLEngine 65,142・m1 25,370・m2 3,093・m3 6,723・m4 1,747・m5 12,931 関数、**本体が変わったものは 0、増減 0**(C のファイルの SHA は応答表の 4 行で変わる)。 |
| 時間 | 開始 06:24 JST、本文書の時点 約 11:50 JST(5 時間 26 分)。 |

## 時刻ごとの面の SHA(fa11947 + lay1。native -O2 と wasm -Oz の node、同じ G3 スクリプト `scripts/wiz-enter-obs.g3`: 先頭で Enter)

| 仮想時刻(Enter から) | lid 13(主窓)先頭 8 桁 | lid 22(Settings) | lid 18(層)の大きさ・先頭 8 桁 | native = node |
|---|---|---|---|---|
| Enter の前 | `721b792e` | `7e34c765` | — | 一致 |
| 0.3 s | `6c1559d5` | `42d3ddd5` | 328×464 `ae29bd5b` | 一致 |
| 1.3 s | `271bb02d` | `a368d8ab` | 320×452 `66e7c949` | 一致 |
| 2.3 s | `4aa007b1` | `a368d8ab` | 318×444 `71df48af` | 一致 |
| 3.3 s | `4aa007b1` | `a368d8ab` | 316×440 `73557847` | 一致 |
| 4.3 s | `4aa007b1` | `a368d8ab` | 314×434 `143cd5ce` | 一致 |
| 6.3 s | `4aa007b1` | `a368d8ab` | 306×424 `36b86b01` | 一致 |
| 10.3 s | `4aa007b1` | `a368d8ab` | 284×400 `11bf2e5d` | 一致 |

native の 2 回目(`wiz-enter-cmp.g3`)も 4.3 s までの全 SHA が 1 回目と同一。lid 18 の面の RGB は常に 0(A も 0)なので、この SHA(A = 255 で RGBA にした値)は大きさにだけ依存する。**wasm の実時間**: 4.3 s まで 4 分 31 秒、10.3 s まで 10 分 34 秒(native は 4.3 s で約 2.5 分、50 s で 19 分)。node の最大 RSS 約 0.93 GB。

## 画像で見た変化(`logs/shots/`、私が画像を開いて見た)

- `before-wizard-lid48-consent.png`(Enter の前の lid 48): 『Help us improve FL Studio by sending usage data』の同意の枠(ボタン I'm in / No thanks / More information…)が、ウィザードの上に重なっている。
- `after-nothanks-click-lid48-welcome.png`: 『No thanks』をクリックすると同意の枠だけが消え、ウィザード本体が見える(Start a new project / Select an edition to trial = All Plugins Edition / Online は Loading… / Load a demo project / Ask Gopher / Unlock FL Studio / Buy FL Studio…)。**ウィザード自体は閉じない**(窓は有効なまま)。
- `before-enter-lid13-main.png` → `after-enter-t2300-lid13-main.png`(主窓): Enter の前は主窓が無効で、ミキサーのメーターが動き、右下に『Mixer - Master』のタイトルバーがある。Enter 後(2.3 s)は、トランスポートの PAT/SONG ボタンが変わり、ミキサーのメーター(緑のバー)と『Mixer - Master』の右パネルのタイトルが消え、**プレイリストの下半分とミキサーが重なった再描画の乱れ**(矩形の継ぎ目)がある。左の Browser の黒い矩形は Enter の前からある。これらは窓モデルの可視領域・クリップの限界で、決定 0142 の担当への記録(MERGE-NOTES)。
- `after-enter-t2300-lid22-settings.png`: Settings(Audio タブ、Input/output・CPU・Mixer の枠、Status Disabled)。Enter の前と比べて窓表の enabled が 0 → 1 になり、描画内容(SHA)が `7e34c765…` → `a368d8ab…` に変わる(題は同じ。Enter の前の画像との画素の差は私は見比べていない)。
- `after-nothanks-then-enter-lid13-main.png`: 『No thanks』→ Enter。主窓の左上に **(Trial) の帯**が出る(Enter だけの場合には無い)。SHA は `2c20db11…`(Enter だけは `4aa007b1…`、Esc は `907ab7ee…`)。入力の経路で主窓の最終状態が違う。
- `ipp-source-337x479.png`・`ipp-warp-output-334x476-before-guest-alpha-scaling.png`: FL のイチゴのロゴ(層の源画像と、warp 後。ゲストが α を ×0.002 する前)。`layered-lid18-t0300.png`: ULW の面(全 0 なので完全に透明の 1 ピクセル行の PNG)。

## 入力の経路とウィザード(実測、fa11947 + lay1、native -O2、G3 の `fruity_g3.c` のスクリプトの構文)

ウィザード(lid 48)の矩形は screen (523,255) 874×570。『No thanks』= ウィザード内 (513,306) = screen (1036,561)。

| 入力 | 結果 | 2.3 s の lid 13 / lid 22 |
|---|---|---|
| `tap 13`(Enter) | 閉じる。主窓と Settings が有効。lid 18 のアニメ開始 | `4aa007b1` / `a368d8ab` |
| `tap 27`(Esc) | 閉じる。**TMsgForm "Error" は出ない**(G3 の報告『Esc → 2 つ目の TMsgForm』とは違う。理由は未確認)。Settings は無効のまま(窓表の enabled 0) | `907ab7ee` / `7e34c765` |
| `click 1036 561`(No thanks) | 同意の枠だけ消える(上の画像) | — |
| No thanks → `tap 13` | 閉じる。(Trial) の帯 | `2c20db11` / `a368d8ab` |
| Enter 2 回 | 2 回目は無害 | `4aa007b1` / `a368d8ab` |

Space は試していない(G3 が測定済み)。

## モデル

### 窓と面

`WS_EX_LAYERED`(0x80000)の窓は今まで通り窓表の 1 件(exstyle にビットを持つ)。`fle_layered.c` は窓ごとに**小さな状態**(64 件まで)を持つ: `mode`(NONE = ビットだけ / ULW / SLWA)、SLWA の `lwa_flags`・`alpha`・`colorkey`、ULW の `alpha`(SourceConstantAlpha)・サイズ・更新回数。`SetWindowLongPtrW(GWL_EXSTYLE)` で WS_EX_LAYERED のビットが変わると状態を捨てる(Learn: 『until the layering style bit is cleared and set again』)。窓の破棄でも捨てる。

ULW の内容は**その窓の top-level の面**(`fle_raster_window_surface(root_lid, cx, cy)`)に書く: 面の大きさ = 窓の大きさ = `psize`(NULL なら今の大きさ)。1 画素 4 バイト B, G, R, A(top-down)。

| ULW の種類 | A の作り方 |
|---|---|
| ULW_ALPHA + AC_SRC_ALPHA(32bpp BI_RGB 必須) | ゲストの DIB の 4 バイト目をそのまま A、B・G・R もそのまま(**事前乗算はゲストの責任**。Learn: 『the red, green and blue channel values in the bitmap must be premultiplied』。されていなくても直さない)。SourceConstantAlpha は属性に保存。 |
| ULW_ALPHA(AC_SRC_ALPHA 無し) | A = 255(『If the source bitmap has no per-pixel alpha value … SourceConstantAlpha determines the blend』)。SourceConstantAlpha は属性。 |
| ULW_OPAQUE | A = 255、定数アルファ 255(blend 無視)。 |
| ULW_COLORKEY | キーと一致する色(下位 24 ビット = COLORREF の比較。未使用バイトは見ない)の画素は B=G=R=A=0、他は A=255(**S-ulw-colorkey-baked**)。 |

窓の位置と大きさ: `pptDst`/`psize` で窓の矩形を変える。**WM_WINDOWPOSCHANGING・WM_WINDOWPOSCHANGED・WM_MOVE・WM_SIZE は送らない**(S-ulw-no-messages。Learn は何も書かない)。窓が不可視でも内容は保持する(Learn Window Features: 『the system stores the image of a layered window』)。`hdcSrc` が NULL のときは位置(と属性)だけ変える。

SLWA の窓: 面は GDI が描いた BGRX のまま(『redirection』。Learn Window Features)。属性(`alpha`・`colorkey`・`lwa_flags`)だけ持つ。ピクセルは触らない。ULW は 87 で失敗(『subsequent UpdateLayeredWindow calls will fail』)。GLWA は SLWA 済みの窓にだけ成功する(『can be called only if … previously called SetLayeredWindowAttributes』『will fail if … setup with UpdateLayeredWindow』)。

### 引数の規則(Learn の文言と、名前つきの選択)

『must be NULL』の違反は失敗(戻り値 0 とエラー番号)、『should be NULL / zero』の違反と、ページに値の無い形は**名前付きの停止**(`fle_stop`)。

| 規則 | 扱い |
|---|---|
| hWnd が窓表の窓でない | 0、ERROR_INVALID_WINDOW_HANDLE(1400)。**S-ulw-error-invalid-hwnd**(SendMessageW と同じ選択) |
| hWnd が WS_EX_LAYERED を持たない | 0、ERROR_INVALID_PARAMETER(87)。**S-ulw-error-not-layered** |
| hdcSrc = NULL で hdcDst ≠ NULL(『must be NULL』) | 0、87。**S-ulw-error-null-src** |
| hdcSrc = NULL で psize ≠ NULL(『must be NULL』) | 0、87。同上 |
| hdcSrc = NULL で pptSrc ≠ NULL / dwFlags ≠ 0 / pblend ≠ NULL(『should』) | 停止 |
| hdcSrc ≠ NULL で hdcDst ≠ NULL | 停止(パレットの照合は無い) |
| hdcSrc がメモリ DC でない(画面 DC など) | 停止 |
| dwFlags に表に無いビット / 2 種以上 / ULW_EX_NORESIZE を ULW に / hdcSrc があって種類が 0 | 停止 |
| ULW_ALPHA で pblend = NULL | 0、87。**S-ulw-error-null-blend** |
| BlendOp ≠ AC_SRC_OVER / BlendFlags ≠ 0(『Must be zero』) | 0、87。**S-ulw-error-blend** |
| AlphaFormat に AC_SRC_ALPHA 以外のビット | 停止 |
| AC_SRC_ALPHA で源が 32bpp でない(『must be 32 bpp』) | 0、87。BI_BITFIELDS の 32bpp は停止 |
| psize の cx / cy ≤ 0 | 0、87。**S-ulw-error-size** |
| ULW_EX_NORESIZE(Indirect)で psize が今の大きさと違う | 0、87。**S-ulw-error-noresize** |
| 源の矩形(pptSrc, 大きさ)がビットマップの外 | 停止 |
| prcDirty ≠ NULL | 停止(座標系がページに無い) |
| UpdateLayeredWindowIndirect の cbSize ≠ 80、pULWInfo = NULL | 0、87。**S-ulw-error-cbsize**(x64 の大きさ 80 は型からの導出) |
| SLWA の後の ULW | 0、87。**S-ulw-after-slwa** |
| GLWA(SLWA していない窓、ULW の窓) | 0、87。**S-glwa-error** |
| SLWA の dwFlags が 0 / LWA_COLORKEY・LWA_ALPHA 以外 / ULW の窓への SLWA | 停止 |
| 子窓の WS_EX_LAYERED(Windows 8 以降の機能) | 停止(子窓の面は親の top-level の面の一部) |

## 面の表の ABI 変更案(app/ の次の担当への引き継ぎ。**コードには入れていない**)

今の表(`lift/harness/web/present/CONTRACT.md` §2 と `fruity_surface.c`)は 1 欄 24 語(96 バイト)、`format` 1 = BGRX8888、4 バイト目は 0。**レイヤード窓(ULW の面)では 4 バイト目が事前乗算の A** なので、このままだと Presenter は A = 255 を強制して**不透明な箱**にしてしまう(lid 18 は現にそうなるはず)。

提案(version 2):

| 変更 | 内容 |
|---|---|
| ヘッダ | `version` = 2、`entry_size` = 112(28 語)。**読み手は `entry_size` で欄を進める**(version 1 の固定 96 の読み手は壊れる: 次の担当が app の読み手を直す) |
| `flags`(語 3) | bit2 = **layered**(WS_EX_LAYERED で、SLWA か ULW が呼ばれている = 表示してよい。Learn: 呼ぶまで見えない) |
| 語 24 `layer_mode` | 0 = 無し、1 = ULW(面の 4 バイト目 = 事前乗算の A)、2 = SLWA(面は BGRX、A は無意味) |
| 語 25 `layer_alpha` | 0〜255。ULW: SourceConstantAlpha(blend 無し・OPAQUE・COLORKEY は 255)、SLWA: LWA_ALPHA のときの bAlpha(無ければ 255) |
| 語 26 `layer_key` | SLWA + LWA_COLORKEY のときの COLORREF(0x00BBGGRR)。ULW ではキーは面に焼いてあるので 0 |
| 語 27 `layer_flags` | bit0 = layer_alpha が有効、bit1 = layer_key が有効、bit2 = 面の 4 バイト目は事前乗算の A |
| 位置・大きさ | 既存の `x, y, width, height`(ULW が窓の矩形を更新するのでそのまま正しい) |
| 書き込み | 既存の `gen` と dirty 矩形(`fle_raster_wrote` を通るので、ULW 1 回 = 全面の dirty 1 回) |

Presenter の合成(Learn BLENDFUNCTION の式): ULW は『Src = Src × SCA / 255』の後に『Dst = Src + (1 − Src.A) × Dst』。canvas は ImageData(非事前乗算)なので、`layer_flags` bit2 のときは B・G・R を A で割って(A = 0 は 0)RGBA にし、`globalAlpha = layer_alpha / 255` で下の canvas に `drawImage` する。SLWA は BGRX(A = 255)に `globalAlpha = layer_alpha / 255`、カラーキーの画素は透明に。z 順は TOPMOST(窓の exstyle)を既存の規則で。**この案の実装(`fruity_surface.c` の `window_fields` に弱い問い合わせ `fle_layered_query`)は書いていない**: wasm のビルドと app の読み手の変更が要り、app/ は範囲外。runtime 側の問い合わせ(`fle_layered_query(hwnd, &info)`)は入っていて、`info.mode / alpha / colorkey / per_pixel / width / height / updates` が上の語に 1 対 1 で対応する。

## 出典なしの選択(名前・値・理由・撤回条件)

| 名前 | 値 | 理由 | 撤回条件 |
|---|---|---|---|
| S-ulw-error-* / S-glwa-error(上の表) | 1400 と 87 | Learn の 4 ページは失敗時のエラー番号を書かない。System Error Codes の条件の名前に合わせた(SendMessageW の選択と同じ流儀) | 実機の測定 |
| S-ulw-no-messages | ULW は窓の矩形を変えるが WM_* を送らない | ページに何も書かれていない。送ると TLightweightLayeredControl(thread 2 が呼ぶ)が再入する恐れ | 実機の測定、またはゲストが WM_SIZE / WM_MOVE を待つと分かったとき |
| S-ulw-colorkey-baked | キー色の画素を A = 0 にして面に焼く | 『Use crKey as the transparency color』。面を自己完結にして Presenter の仕事を減らす | カラーキーを属性で持つ必要が出たとき |
| S-ulw-const-alpha-attribute | SourceConstantAlpha を面に焼かず属性に持つ | hdcSrc = NULL で blend だけ変える呼び出し(ページが許す)で内容を失わないため | — |
| S-ulw-premultiplied-as-given | ゲストの B・G・R を A で再計算しない | 事前乗算はゲストの責任(ページ)。違反は実機でも未定義の見え方 | 実機の測定 |
| S-ulw-hidden-keeps-content | 不可視の窓にも内容を保持 | Window Features の『the system stores the image』 | — |
| S-layered-state-table | 状態 64 件。超えると停止 | 窓の破棄で捨てるので足りる(70 窓の作成・破棄のテストあり) | 実測で足りないとき |
| S-gdi-on-ulw-window-not-modelled | ULW の窓の DC に GDI で描く呼び出しは、普通の窓と同じに面へ書く(捨てない) | ゲストは thread 2 で ULW の窓に GDI を使っていない(実測。GetDC/ReleaseDC が各 1 回あるが描画なし)。『ULW の窓への GDI は表示されない』は未確認 | ゲストが使うとき |
| S-thread-cstack-8MiB(PM の決定) | wasm の guest スレッドの C スタックの既定 1 MiB → 8 MiB(`fle_thread_wasm.h`) | PM の指示(音声担当と G3 担当が同じ壁に当たった) | 8 MiB が重いと分かったとき |
| S-native-dump | `FLE_G3_DUMP_DIR` で `g3_snapshot` が面の生バイトをファイルに書く(native のみ) | 画像を見るため。wasm では何もしない | — |

## 壁の一覧

| # | 停止 | 場所 | 種類 | 直し |
|---|---|---|---|---|
| L1 | `indirect call to 0`(GetProcAddress("UpdateLayeredWindowIndirect"/"UpdateLayeredWindow") が NULL、127) | `f_71e570` の 0x71e68a / 0x71e6d4 | 仮想 user32 のエクスポート表(= 応答表の行)の欠落 | 4 行 + `fle_layered.c` のモデル |
| — | その先 | — | **無し**(Enter / Esc / No thanks → Enter / Enter 2 回、50 仮想秒まで。名前付きの停止は出ない) | — |

過去に記録のある『クロックを進めると `indirect call to 0x33d6e70`(lift の穴)』(決定 0128 の記述)は、この実行(仮想 50 秒まで)では**起きなかった**(未確認: 木・構成が違う)。

## 未確認・限界

- ULW の窓の DC への GDI 描画の扱い、子窓の WS_EX_LAYERED、prcDirty、hdcDst(パレット)、画面 DC やメモリ DC でない hdcSrc、WS_EX_TRANSPARENT とヒットテスト(Learn: 『Hit testing of a layered window is based on the shape and transparency』)は**モデルにない**(停止するか、何もしない)。
- 画面 DC からの BitBlt(画面の合成)は作っていない: この壁では不要(層の源はゲストの内蔵 PNG)。
- 縮小アニメが終わらない(50 仮想秒)・α が 0.002 のまま: ゲストの計算で、環境の誤りかどうかは**未確認**。`f_71ea30` の副ステップ数(`[this+0xc4]` → `[+0x50]` → `[+0x98] + 1`、無ければ `f_7371d0`)と α の式を読むのが次の一歩。
- wasm の確認は仮想 10.3 秒まで(10 分 34 秒)。native は 50 秒まで。
- 逆アセンブルの読み(`f_71e570` の引数、BLENDFUNCTION の組み立て)は Capstone と pefile による(実機でのデバッガ照合は無し)。`UpdateLayeredWindowIndirect` の Learn ページは取得できなかった(404)ので、規則は UPDATELAYEREDWINDOWINFO と UpdateLayeredWindow のページから。

## 出典(Microsoft Learn、2026-11-02 に取得、`docs/investigations/lay1/learn/`)

UpdateLayeredWindow(winuser/nf-winuser-updatelayeredwindow)、UPDATELAYEREDWINDOWINFO(ns-winuser-updatelayeredwindowinfo)、SetLayeredWindowAttributes、GetLayeredWindowAttributes、BLENDFUNCTION(wingdi/ns-wingdi-blendfunction)、AlphaBlend、Window Features(winmsg/window-features、Layered Windows の節)、Extended Window Styles(WS_EX_LAYERED)、TrackMouseEvent / TRACKMOUSEEVENT / _TrackMouseEvent(最初の木で自作して、G3 の取り込みで取り下げたときに読んだもの)。取得できなかった: UpdateLayeredWindowIndirect(nf-winuser-updatelayeredwindowindirect、404)。
