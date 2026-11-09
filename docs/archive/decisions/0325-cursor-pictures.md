# O18(マウスカーソルがホバーで変わらない): カーソルの絵と hot spot をブラウザの CSS カーソルへ渡す — 差分と試験(ci1)

決定番号は PM が付ける。差分のコメントに入れた「decision 0325」は**仮の番号**(④ S-disabled-drops の差分は仮 0324)。確定したら `sed -i 's/decision 0325/decision NNNN/g'` を差分の適用後の木に 1 回掛ける(対象: 差分に入る 10 ファイル。`grep -rl "decision 0325"` で一覧)。

## 0. 実際に実行した物(先頭)

| 実行した | 内容 |
|---|---|
| clang -O0(nice 19、1 回に 1 つ)で ランタイム 34 ファイル(.o)と新しい試験を native ビルド、`-Wall -Wextra -fsyntax-only` | `fle_gdi.c` `fle_res.c` `fle_appview.c` で警告 0 |
| native 試験 `lift/harness/c0325/cursor_img.c` | シナリオ I(`CreateIconIndirect`)・M(メモリ上に作った PE の資源)・S(絵が無い handle)で **PASS 38 / FAIL 0** |
| **実物の FLEngine_x64.dll の 56 個のカーソル**(`binaries/win64/FLEngine_x64.dll`、序数 32762〜32767 と名前つき 50 個)を、ランタイムの C(`LoadImageW` / `LoadCursorW` → `fle_av_cursor_image`)と、独立に書いた python の復号(pefile + numpy、`real-cursors.py`)で読み、幅・高さ・hot spot・RGBA の FNV-1a を比べた | **56 行すべて一致** |
| 実物の 56 個の絵を並べた PNG(`logs/real-cursors-contact.png`、hot spot はマゼンタ) | 向き(上下)・色・形が正しいことを目で確認(砂時計 SQL、ドラッグのコピー/移動、ペン、ブラシ、ズーム、リサイズ矢印など) |
| vitest(`nice -n 19`、node)`test/unit` 全体、`tsc --noEmit`(`tsconfig.json` `tsconfig.webworker.json` `tsconfig.test.json`) | 新しい 14 件を含め通過。全体は HEAD と同じ 18 件が落ちる(私の抜き出した木に `scripts/` などが無いため。HEAD の木で同じ 18 件)、新たな失敗 0、型検査の新しい誤り 0 |
| `patch -p1 --dry-run` と実適用(`git archive HEAD` の複製) | rc 0、適用後の木 = 作業木 |

**実行していない物**: ブラウザ(Chrome・Firefox)は 0 回 = **CSS カーソルが実際に変わる所は未確認**。実エンジン(G3 / N1 / wasm)は 0 回。emcc の build は 0 回。ミューテーション検査は 0 回(このリポジトリの慣習にある物。窓の指示待ち)。依頼者の機械の値は 0 件。

## 1. 原因と直す仕組み(全体)

- **原因**(PM の判断どおり): ページ(`ui/cursor.ts`)は IDC の標準カーソルだけを CSS に変換し、FL 自作のカーソル(`CreateIconIndirect`)とモジュールから読んだカーソル(`LoadImage` / `LoadCursor`、名前つき)は**矢印**で表示していた(S-module-cursor-default / S-created-cursor-default)。ランタイムは絵を持っていない(`CreateIconIndirect` は「ピクセルを保持しない」、`LoadImageW` は「絵と hot spot は読まない」と各関数のコメントが明記)。区分 **(b)**: カーソルは「絵 + hot spot」(Learn About Cursors / ICONINFO / `SetCursor`)なのに、絵がブラウザに渡っていなかった。
- **実物の数**: FLEngine_x64.dll には RT_GROUP_CURSOR が **57 個**(序数 32761〜32767 の 7 個と名前つき 50 個: `EEPEN` `EEBRUSH` `EEZOOM` `EESLIP` `DRAGCOPY` `HAND2` `SIZEL` `VT_MOVE*` など = ピアノロール・プレイリストの道具のカーソル)。すべて 32×32、深さは 1 / 4 / 32 bit、`BITMAPINFOHEADER` 40 バイト・`BI_RGB`。**この読みは実物 56 個(32761 は LoadImage の対象外)で一致を確認した**。
- **直す仕組み**: 『HCURSOR の絵と hot spot をブラウザの CSS カーソルへ渡す』全体。(1) ランタイムが現在のカーソルの絵(RGBA)と hot spot を取り出す口を持つ、(2) Worker が(handle, serial)ごとに 1 回だけページへ送る、(3) ページが PNG の `url()` カーソルにする。FL のクラス名・窓の種類・カーソルの種類による分岐は 0。

## 2. 差分(`diffs/0325-cursor-pictures.diff`、1270 行、sha256 先頭 `9a9e03a6889433d3`、適用していない)

| ファイル | 内容 |
|---|---|
| `lift/runtime/fle_gdi.c` `fle_gdi.h` | `CreateIconIndirect` が作る時点で絵を RGBA に作って保持(Learn:「システムは ICONINFO のビットマップを複製してから作る」)。カラー版は mask 0 → 色、mask 1 かつ黒 → 透明、mask 1 かつ黒以外 → 色(本当は画面 XOR 色。ブラウザは反転できないので色を見せる: S-cursor-xor)。単色版は mask の上半分が AND・下半分が XOR(AND0 XOR0 黒、AND0 XOR1 白、AND1 XOR0 透明、AND1 XOR1 白)。icon(fIcon TRUE)の hot spot は中心。`DestroyIcon` / `DestroyCursor` で絵を解放。`fle_gdi_icon_image()` が引く。作るたびに新しい serial |
| `lift/runtime/fle_res.c` `fle_res.h` | `fle_cursor_shared_image()`: `LoadImageW` / `LoadCursorW` が返す共有 handle(`FLE_LI_MODULE_HANDLE_BASE + 0x1000 * k + 4 * q`)を RT_GROUP_CURSOR に戻し、32×32 に最も近い RT_CURSOR 成分(同点なら深い方)の DIB(1/4/8/24/32 bit の XOR 画像 + 1 bit の AND mask、hot spot は先頭の 2 WORD)を RGBA にする。32 bit で alpha が 1 つでもあれば alpha を使い、無ければ AND mask の規則。PNG・128 超・その他の形式は 0(絵なし)を返し、**ランタイムを止めない** |
| `lift/runtime/fle_appview.c` `fle_appview.h` | `fle_av_state.cursor_serial`(予約領域の先頭、オフセット 56。サイズ不変)。新しい export `fle_av_cursor_image(handle)`(32 バイトの構造体 + 静的な 64 KB の RGBA。標準カーソルと絵の無い handle は NULL) |
| `app/src/engine/appview.ts` `fruity.ts` `os-state.ts` `protocol.ts` `worker.ts` `client.ts` | `cursorSerial` を osstate の鍵に含める。Worker は osstate が変わったとき、標準でないカーソルの絵を**(handle, serial)ごとに 1 回だけ**、osstate より**前**に `cursor-image` で送る(RGBA を transfer)。export を呼ぶのも 1 回だけ(`cursorsSent` で先に判定) |
| `app/src/ui/cursor.ts` `main.ts` | `CursorPictureStore`: 受けた絵を canvas の PNG にして `cursor: url(...) hx hy, default`。device pixel ratio > 1 の画面のため、幅高さが 2 倍して 128 以下なら最近傍で 2 倍の絵も作り `image-set(url 1x, url 2x)`(`CSS.supports` で使えるときだけ。無効な値は直前のカーソルを残すため)。絵が届く前・無いときは矢印(従来どおり) |
| `lift/harness/c0325/` `app/test/unit/` | native 試験(`cursor_img.c`、`build.sh`、実物比較の `real-cursors.py`)、vitest `cursor-picture.test.ts`(14 件)と既存 2 本の更新 |

通信と変換: 絵は(handle, serial)ごとに Worker → ページへ 1 通(32×32 なら 4 KB。最大 64 KB)。毎フレームの通信は増えない。osstate の中身は serial が 1 つ増えただけ。ピクセルのコピーは wasm → Worker で 1 回(export が返す静的バッファ)、Worker → ページは transfer(コピーなし)。

## 3. 試験の内容

- **I(native、`CreateIconIndirect`)**: カラー版 4×4(mask 0 で色、mask 1 と黒で透明、mask 1 と黒以外で色、hot spot)、単色版 8×4(mask 8 行の AND / XOR の 4 通り)、icon の中心の hot spot、`DestroyCursor` で絵が消える、作り直すと serial が変わる。
- **M(native、メモリ上の PE を 0x400000 に置く)**: 1 bit の名前つきカーソル(`LoadCursorW(L"MYCUR")`)、32 bit alpha つき(序数 32762)、24 bit + AND mask(序数 32763)。
- **S**: 標準の矢印・NULL・存在しない handle・存在しない群は絵なし。
- **R(実物)**: 上記のとおり 56 / 56 一致(`logs/real-cursors-runtime.out` と `logs/real-cursors-python.txt` が同一)。**実物の内訳 = 1 bit 13 個、4 bit 10 個(`VT_MOVE*`)、32 bit 33 個(`EE*` `DRAG*` `HAND*` `SIZE*` など。33 個すべてに alpha がある)。3 つの深さをすべて通した**。AND = 1 かつ黒以外の画素(S-cursor-xor の近似に当たる画素)を持つ実物は **0 個 / 56**(python で数えた)。
- **vitest**: 構造体の配置を C のヘッダ(ファイルを読む)に固定、`readCursorImage`(コピーする、hot spot を内側に収める、不正な入力は null)、`scale2x`、`CursorPictureStore`(url / image-set / 大きい絵 / serial)、`cursorCss`(絵あり / なし / 標準 / NULL)、osstate の鍵に serial。

## 4. 壊れる物の仮説・残る限界

- **未確認**: ブラウザで実際にカーソルが変わること。Chrome は 128×128 まで、32×32 を超えると画面の端やブラウザの UI の上で出ない場合がある(FL のカーソルは 32×32 なので該当しない)。`image-set()` を cursor に使えるか(`CSS.supports` で判定し、使えなければ 1x の `url()` のみ)。hot spot の座標は CSS ピクセル。
- 反転(AND1 XOR≠0)は色を見せる近似(S-cursor-xor)。実 Windows は画面との XOR。FL の実物 56 個には AND1 かつ XOR≠0 の画素が 0 個(§3)なので、FL 自身の絵ではこの近似は効かない(`CreateIconIndirect` で作るカーソルは実行時の内容次第で、未確認)。
- 標準 IDC のカーソルは従来どおり CSS のキーワード(システムのテーマ)。`.ani` 形式や 128 超のカーソルは絵なし(矢印)。
- 『ホバーしても変わらない』が絵だけの問題かは、実エンジンで `WM_SETCURSOR` → `SetCursor(h)` が FL から呼ばれることを窓で確認する必要がある(ランタイムは移動のたびに `WM_SETCURSOR` を送っている。決定 0136)。`SetCursor` の handle が `CreateCursor`(AND/XOR 配列)なら**未実装**(`CreateCursor` はランタイムに無い)。
- 他の差分との重なり: 0323 / ④ とは別ファイル(`fle_gdi.c` `fle_res.c` `fle_appview.*`)。alpha1 の ScrollWindowEx(fle_win.c)とも重ならないと読む。

## 5. 自己申告

1. 試験の初版で、(a) 試験用 PE の先頭を自分で 0 にして MZ 署名を消す、(b) `uint32_t rgba` に 64 bit のアドレスを入れて native で落ちる、という 2 つの誤りがあった。(b) は構造体を `uint64_t rgba`(オフセット 24、サイズ 32 のまま)に直した(JS は下位 32 bit を読む)。
2. 決定番号は仮(冒頭)。
3. vitest 全体の 18 件の失敗は私の抜き出した木の不足で、HEAD でも同じ(差 0)。PM の木での全体の確認は未実施。

## 6. 再現

```
git archive HEAD lift/runtime lift/harness app/src app/test | tar -x -C T
(cd T && patch -p1 < 0325-cursor-pictures.diff)
bash T/lift/harness/c0325/build.sh T W                                   # native: PASS 38
python3 T/lift/harness/c0325/real-cursors.py binaries/win64/FLEngine_x64.dll D --png
bash T/lift/harness/c0325/build.sh T W R D/flat.bin D/names.txt          # 実物 56 個。D/expected.txt と同じ行(順序は sort)
(cd T/app && node node_modules/vitest/vitest.mjs run test/unit/cursor-picture.test.ts)
```


## PM の判断と取り込み(2026-11-07 08:39、docs のみ。製品の差分なし)

取り込み: ci1 の 0325(O18: カーソルの絵と hot spot をブラウザの CSS カーソルへ渡す)、差分(`docs/investigations/ci1/0325/diffs/`)、実物のカーソルの復号の証拠(`evidence/`)。**PM の検証**: `0325-cursor-pictures.diff`(1270 行)を cand1 の積み(HEAD + 0323 + 0313f2 + scroll-model + time-env + stop-context + d1 + d2 + diag-g9-fl + diag-g9-fl-scroll)+ 0323b + alpha1 の scrolldc の複写に実適用して rc 0・FAILED 0・.rej 0、`emcc -fsyntax-only -Wall -Wextra` で fle_gdi.c・fle_res.c・fle_appview.c は警告 0。ci1 の報告(私は再実行していない): 実物の FLEngine_x64.dll のカーソル 56 個を C と独立の python で復号して幅・高さ・hot spot・RGBA が一致、native の試験 38 PASS。**ブラウザで CSS カーソルが実際に変わるかは未確認**(RT-A の bundle ができてから gpu1 が確認)。区分 (b)。RT-A の bundle に載せる。依頼者の機械は 0 件。
