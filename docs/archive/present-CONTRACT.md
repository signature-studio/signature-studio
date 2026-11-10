# 描画面の契約案: wasm(C)が JS に出す最小の API と、JS 側の責務

状態: **案**(何も決めない。決めるのは PM)。決定 0126(仮)の取り込み前に、JS 側を先に作って測るための取り決め。
凡例: **実測** = この harness で動かして確かめた。**読み** = 決定 0126 のワークツリー `lift/out-fruity11/`(読み取りのみ)のコードを読んだ解釈で、実行していない。**未確認** = 確かめていない。
この契約を満たす「エンジンの代役」(`engine.c`。FL のコードではない)に対して、JS 側(`core.js`、`engine-worker.js`、`main.js`)を動かして測った。実エンジンの wasm には C 側の API がまだ無い(決定 0126 は native)。

## 1. 責務の分け方

| 側 | 持つもの | 持たないもの |
|---|---|---|
| wasm(C) | 描画面の**画素**(BGRX、線形メモリの中)、**面の表**(窓の論理 id、位置、大きさ、stride、画素の位置、世代番号、可視、ダーティ矩形) | canvas、RGBA への変換、ポップアップ、表示のタイミング |
| JS | 表を読む、BGRX → RGBA、canvas / ポップアップ、論理 id と canvas の対応、DPR・CSS サイズ、表示のタイミング | 画素の描画、窓の状態 |

- 面は **top-level 窓 1 枚につき 1 枚**(detach-window.md R10。決定 0126 の `fle_raster.c` が既にそうしている: 読み)。子窓は親の top-level の面に描く。
- JS は面に**書かない**。読むだけ(ダーティ矩形の「取り出し」で表の 1 欄だけ書く: §3)。
- 画素は **BGRX 32bit、上の行が先(top-down)、1 画素 4 バイト = B, G, R, X の順、X は常に 0**(`fle_raster.h` の記述。読み)。stride = 幅 × 4(読み: `ws_fill_view`)。**JS は A = 255 を強制して RGBA にする**(X = 0 のまま canvas に渡すと透明になる)。**ただしレイヤード窓(version 2、`layer_mode` = 1 の ULW)の面は 4 バイト目が事前乗算のアルファ**: A = 255 を強制してはならない(不透明な箱になる)。JS は B・G・R を A で割って RGBA にし、定数アルファを canvas の合成で掛ける(決定 0145)。

## 2. 面の表(線形メモリ上。実測)

表の位置は `fle_surface_table()` が返すバイトオフセット。32 ビット語(リトルエンディアン)。

ヘッダ(8 語 = 32 B):

| 語 | 名前 | 内容 |
|---:|---|---|
| 0 | magic | `0x46534c46`('FLSF') |
| 1 | version | 1(決定 0128)。**2**(決定 0145。欄が 28 語に増え、レイヤード窓の語が付く。下の「version 2」) |
| 2 | entry_size | 96(version 1)/ **112**(version 2)。**読み手はこの値で欄を進める**(固定の 96 を前提にしない) |
| 3 | capacity | 欄の数(harness の代役は 8。**実際の値は C 側が決める**。0126 の停止時に生きていた面は 2 枚: lid 13 と lid 115) |
| 4 | live | 生きている欄の数(Atomics で読む) |
| 5 | doorbell | 方式 C(共有メモリ)で「見てほしい」と知らせる語。JS が Atomics.add / notify する |
| 6-7 | pad | |

欄(24 語 = 96 B)。`flags & 1` が立っている欄が生きている。lid が 0 でも生きている(0126 では lid 0 = デスクトップ/画面 DC)ので、空き欄の判定は lid ではなく `flags & 1`。

| 語 | 名前 | 内容 | 保護 |
|---:|---|---|---|
| 0 | lseq | 配置部(語 1〜15)の seqlock。書き手は変更の前後で +1(変更中は奇数) | — |
| 1 | lid | top-level 窓の論理 id(0126 の `tab[].lid`)。**再利用しない** | lseq |
| 2 | hwnd | 現在の HWND の下位 32 ビット(情報用。窓の再作成で変わる) | lseq |
| 3 | flags | bit0 = 生きている、bit1 = 可視(窓とその親が WS_VISIBLE)、**bit2 = layered(version 2。窓が WS_EX_LAYERED を持つ)** | lseq |
| 4, 5 | x, y | 仮想画面上の窓の位置(0126 の `tab[].px, py`。主窓は (-5, -6)) | lseq |
| 6, 7 | width, height | 面の大きさ = top-level 窓の大きさ(主窓 1930 × 1092) | lseq |
| 8 | stride | 1 行のバイト数 | lseq |
| 9 | format | 1 = BGRX8888 | lseq |
| 10 | pixels | 画素 (0, 0) の線形メモリ内のバイトオフセット | lseq |
| 11 | layout_gen | width / height / stride / pixels が変わるたびに +1 | lseq |
| 12 | owner_lid | ヒント: オーナー窓の lid(0 = なし)。**実エンジン(`fruity_surface.c`)はこの語に何も書かない(常に 0。決定 0150、読み)。オーナーは `fle_av_poll`(`fle_appview.h`)の窓リストの `owner_lid` を読む** | lseq |
| 13, 14 | style, exstyle | ヒント | lseq |
| 15 | form_key | ヒント: 窓の再作成をまたいで同じ値になるキー(0 = なし)。§5。**実エンジン(`fruity_surface.c`)はこの語にも何も書かない(常に 0。決定 0150、読み)** | lseq |
| 16 | dlock | ダーティ部(語 17〜23)のスピンロック。0 = 空き、1 = 保持中 | — |
| 17 | gen | 書き込み呼び出しごとに +1(`fle_raster_wrote` の呼び出し) | dlock |
| 18 | taken_gen | 最後の取り出しの gen(情報用) | dlock |
| 19〜22 | dx0, dy0, dx1, dy1 | 最後の取り出し以降のダーティ矩形 [x0, x1) × [y0, y1)。空 = x0 ≥ x1 または y0 ≥ y1 | dlock |
| 23 | dirty_first_gen | ダーティ矩形に入っている最初の書き込みの gen(情報用) | dlock |
| 24 | layer_mode | **version 2 のみ**(欄は 28 語)。0 = WS_EX_LAYERED だが UpdateLayeredWindow も SetLayeredWindowAttributes もまだ呼ばれていない(**表示しない**。Learn: 呼ぶまで見えない)、1 = ULW(面の 4 バイト目 = 事前乗算のアルファ)、2 = SLWA(面は BGRX)。`flags` の bit2 が立っているときだけ意味がある(立っていなければ 0) | lseq |
| 25 | layer_alpha | 0〜255。定数アルファ(ULW: SourceConstantAlpha、blend 無し・OPAQUE・COLORKEY は 255。SLWA: LWA_ALPHA のときの bAlpha)。`layer_flags` の bit0 が立っているときだけ使う(無ければ 255) | lseq |
| 26 | layer_key | SLWA + LWA_COLORKEY のときの COLORREF(0x00BBGGRR)。`layer_flags` の bit1 が立っているときだけ使う。ULW ではキーは面に焼いてある(A = 0) | lseq |
| 27 | layer_flags | bit0 = layer_alpha が有効、bit1 = layer_key が有効、bit2 = 面の 4 バイト目は事前乗算のアルファ(B・G・R はそれで掛けてある) | lseq |

**version 2(決定 0145)**: 欄が 24 語 → 28 語(24〜27 が上の 4 語)、`flags` の **bit2 = layered**(窓が WS_EX_LAYERED を持つ。`layer_mode` が 0 のままなら『まだ何も呼ばれていない』なので表示しない)。`lseq`(seqlock)は語 24〜27 も守る。読み手の規則: 欄の間隔は `entry_size`、レイヤードの語を読むのは `version >= 2` かつ `entry_size >= 112` のときだけ。version 1 の表(96 バイト)は従来どおり(レイヤード窓は無い)。**書き手(`fruity_surface.c`)は、レイヤードの語を『書き込みのとき』と『エンジンの idle のたび(`fruity_idle.c` の idle フックが待つ前に `fruity_surface_idle_sync()`)』に `fle_layered_query` から更新する**(ULW は面を書いた**後**に状態を更新するので、書き込みの時点の語は 1 呼び出し分古いことがある。idle で直る)。語の意味は `runtime/fle_layered.h` の `fle_layered_info` と 1 対 1: mode = mode、alpha = alpha、key = colorkey、bit0 = (ULW か LWA_ALPHA)、bit1 = LWA_COLORKEY、bit2 = (ULW かつ per_pixel)。

## 3. C の API(最小)

```c
uint32_t fle_surface_table(void);                     /* 表のバイトオフセット。常に同じ値 */
uint32_t fle_surface_count(void);                     /* 生きている欄の数 */
int      fle_surface_info(uint32_t i, fle_surface_info_t *out);  /* i 番目の生きている欄の配置部を seqlock の下でコピー(0 = 成功、-1 = 無い)。gen も 1 回読む */
int      fle_surface_take_dirty(uint32_t lid, uint32_t out[5]);   /* ダーティ矩形を取り出して空にする(下)。1 = 空でなかった、0 = 空、-1 = その lid は無い */
```

`fle_surface_take_dirty` は dlock の下で `out = {gen, x0, y0, x1, y1}` を読み、矩形を空にする。**取り出しの後の書き込みは新しい矩形に入る**。消費側は「取り出してからコピーする」。この順序なら、コピー中に着地した書き込みは次の取り出しで拾われる(実測: 20/20 の飽和後の最終状態が一致)。

JS は表の語を直接読んでもよい(`core.js` の `SurfaceTable`。上の C 関数と同じ答えを返すことを `test-exports.mjs` で確かめた)。**C の関数は、JS が表を読めない状況(別の言語・別の wasm)のための同じ意味の入口**で、JS だけで読むなら要らない。

エンジン側の更新(0126 のコードへの足し方。**この harness は実装していない。読みで書く**):

| 場所(0126 のワークツリー) | 足すもの |
|---|---|
| `fle_raster_window_surface()`(面を作る・大きさを変える) | 欄を作る / `width`, `height`, `stride`, `pixels` を更新し `layout_gen++`(lseq の中)。大きさが変わったら `pixels` は変わる(`calloc` し直して旧を `free`)ので、全面ダーティ |
| `fle_raster_window_destroyed()` | `flags` を 0 にする(lseq の中) |
| `fle_raster_wrote(surf, r, api)`(窓への描画の**唯一の出口**と読む。`fle_gdi.c` の `target_wrote` 13 箇所が呼ぶ) | `gen++`、`r` を dlock の下でダーティ矩形に合併 |
| `fle_win.c`(窓表) | `x, y, visible, hwnd, owner_lid, style, exstyle, form_key` を面の欄に反映する弱い hook(`fle_win_dc_query` と同じ流儀)。`fle_raster.c` は窓表を知らない |

## 4. 順序の規則(書き手と読み手)

1. **画素を書いてから公開する**(write, then publish): 画素を書き終えてから `gen++` とダーティ矩形の更新。
2. **取り出してからコピーする**: `take` → 画素のコピー。逆にすると、コピー中の書き込みを取りこぼす(実測で、最初に作った「世代の ack でクリアする」方式は、書き手がコピーより速いと矩形が縮まず、毎回全面をコピーした。`out-git/present1-results/first-pass-flawed-dirty-protocol/`)。
3. **配置部は seqlock で読む**: lseq が奇数、または読む前後で違えば読み直す。**C 方式(別スレッドが読む)ではコピーの後にも lseq を確かめ、変わっていたらそのコピーを捨てる**(窓の大きさが変わると旧バッファが `free` され、再利用される)。**この再確認は harness の presenter には未実装**(JSPI の方式では読み手が動くのはエンジンが中断中だけなので競合が無い。C 方式は未実装 = 未確認の穴)。
4. **JSPI(方式 A・B)では競合が無い**: JS が走るのは `WebAssembly.Suspending` の import の中、またはエンジンが中断している間だけ。`wrote()` の途中で中断することは無い。dlock は競合しない(取っても空いている)。
5. **共有メモリ(方式 C)で書き手が読み手より速いと、コピーが裂ける**(2 つのフレームが混ざる)。実測: 書き手が待たずに連続して書くと、検査したコピーの 91〜99.9 % が裂けた(C/jspi と C/block、256×256 と全面。例: 256×256 で 104,760/107,201)。裂けを防ぐには、書き手が読み手の確認を待つ(実測: エンジンが約 3,600〜4,300 回/秒(256×256)・約 250〜270 回/秒(全面)に落ち、199,000 回の検査で裂け 0)か、2 枚持つ(未試作)。検査は 4 点の画素を見るだけで、裂けの見逃しはありうる(「裂けなし」は 0/199,000 の下限の意味)。
6. **`memory.grow`**: `pixels` はバイトオフセットなので grow をまたいで有効。非共有メモリでは grow が古い `ArrayBuffer` を **detach** する。それを元にした `Uint8ClampedArray` / `ImageData` は死ぬ。**キャッシュしたビューは `memory.buffer` が変わったら作り直す**(実測: 最初のビルドは `putImageData` が "The source data has been detached" で止まった。キャッシュの鍵に buffer を足して直った。4 方式とも grow 3 回で最終一致)。
7. **可視でない窓への描画は 0126 が捨てる**(`S-draw-hidden`)ので、面の `gen` は動かない。可視の変化は `flags` の bit1 に出す。

## 5. 窓の論理 id と canvas の対応(R10)

- JS は **top-level 窓ごとに 1 つの出口(sink = canvas)** を持つ。キーは **`form_key` があればそれ、なければ `lid`**(`Presenter.keyOf`)。名前空間は別(`'f'+form_key` / `'l'+lid`)。
- **HWND には結ばない**(detach-window.md R5)。
- **主窓を `lid` の数値で決めない(決定 0150)。lid は窓の作成順の番号で、作成順が変われば(決定 0147 の仮想時計など)、また窓が作り直されれば変わる。アプリは窓のクラス名 `TFruityLoopsMainForm`(`fle_av_poll` の窓リスト)で決める。**
- **決定 0126 の `lid` は CreateWindowExW 1 回ごとに `next_lid++` で付く(`fle_win.c` 940〜942 行。読み)**。主窓は HWND 0x7f990034、lid 13(wall-log.md。`(0x34)/4 = 13` で窓表の添字 + 1 と一致するので、lid は HWND と同じ通し番号と読める)。VCL が窓を作り直す(デタッチ/アタッチ)と**新しい lid になる**。つまり 0126 の `lid` は「作り直しをまたぐ論理窓」ではなく、**再利用しない HWND の通し番号に近い**。detach-window.md R5 が言う「同じ論理窓(フォーム)に新しい HWND」を表せない。
  - 実測: `form_key` がある場合、窓を作り直して lid 21 → 22、512×384 → 640×480 になっても**同じ canvas が追従し、画素が一致**した(方式 A・B・C)。`form_key` が無い場合は、**新しい lid に canvas が無く、旧い canvas に旧い画像が残った**。
  - 提案(未確認): `form_key` は Delphi のフォームのインスタンスから取る。VCL は HWND に `Self` のポインタを `SetPropW` で結ぶ(detach-window.md R7、`f_612190`。2 つのアトム)ので、その値は窓の作り直しをまたぐはず。**0126 の `props`(`fle_win.c` の `struct wprop`)から引けるかは見ていない**。
- 新しい面(lid)が現れたら、JS は `layout_gen` / `lid` の変化を見て canvas を `resize` し、全面を描き直す(実測)。
- ポップアップ(別の top-level 窓の出口): `window.open` はユーザー操作の直後に 1 窓(detach-window.md §4.3)。ポップアップの canvas を、その `form_key` の sink にする。**実測: 2 枚目の窓をポップアップに出し、メインの canvas とポップアップの canvas の両方が画素一致**(方式 A・B・C。A は `transferControlToOffscreen` した canvas をオープナー経由でエンジンの Worker に渡した)。ポップアップを閉じる/開き直す動作は**未測定**。

## 6. 表示方式(JS 側。実測は `docs/investigations/canvas-present.md`)

| 方式 | 誰が描くか | 前提 |
|---|---|---|
| A | エンジンの Worker が OffscreenCanvas(`transferControlToOffscreen` で受け取る)に `putImageData`(ダーティ矩形だけ) | エンジンが JSPI で中断する(Worker のイベントループが回る)こと。**`Atomics.wait` で戻らない Worker では表示されない**(再確認) |
| B(Bab / Bbmp) | Worker が変換し、`ArrayBuffer`(転送)または `ImageBitmap` を `postMessage`。ページが描く | 受け取り側の ack を返す経路(SAB の語。ブロックする Worker は `onmessage` が動かないので、隔離が必要) |
| C | ページが共有 `WebAssembly.Memory` の表と画素を直接読む | エンジンの wasm を**共有メモリ**で作る(現在の lift の wasm は共有ではない: 引用)。COOP/COEP 必須。§4-5 の裂けに注意 |

BGRX → RGBA の変換は 3 通りを実装した: 1 バイトずつの JS、`Uint32Array` のビット演算、wasm SIMD(`i8x16.shuffle`)。変換は**エンジンの線形メモリの中**で行い(SIMD の wasm は同じメモリを import する)、結果の RGBA を `ImageData` としてそのメモリの上に作る(非共有メモリならコピー無し)。

## 7. 決定 0126 の実データ構造との食い違い・未確認

1. **`lid` が窓の作り直しをまたがない**(§5)。`form_key` が必要。0126 の担当に確認が要る。
2. 0126 の面(`wsurf`)は `lid`、`live`、`w`、`h`、`bits`、`view`、`writes`、`discards`、`first_done` だけを持ち、**x, y、可視、gen、ダーティ矩形、`hwnd` は持たない**(`fle_raster.c` 485〜493 行。読み)。x, y、可視は `fle_win.c`(`tab[].px, py`、`is_visible`)にある。面の欄を埋めるには窓表との弱い hook が要る。
3. **窓への書き込みが全部 `fle_raster_wrote` を通るかは未確認**。`fle_gdi.c` に描画関数の呼び出し行が 27、`target_wrote` が 13 箇所(grep で数えただけ。各関数で呼び出しの後に `target_wrote` が来ていると読んだが、全部を照合してはいない)。`target_wrote` は `n`(書いた画素数)が 0 のとき呼ばない。
4. **面の寸法は窓の寸法**(主窓 1930×1092、仮想画面 1920×1080 より大きい。位置 (-5, -6))。canvas の内部サイズは面の寸法で、画面の寸法ではない。
5. 面は**最初の描画で作られる**(`fle_raster_window_surface` が DC の取得時に呼ばれる)。可視でない窓は面を持たない。JS は「面が増える・消える」を表の `live` / `flags` で見る。
6. 面の画素は `calloc` の領域(wasm では線形メモリのヒープ)。**大きさが変わると別の領域になる**(`calloc` + 重なる部分のコピー + `free`)。`pixels` は変わりうる(§4-3)。
7. `X` バイトは 0(窓の面は `keep_x` が 0。0126 のコメントの記述。読み)。PNG は X を持たないので、PNG → BGRX は X = 0 で作った。実際の面の X が 0 だったかは、PNG からは確かめられない。
8. 面は 1 窓あたり最大 16384 × 16384 × 4 B(`SURF_MAX_DIM`)。wasm32 の線形メモリ(上限 2 GiB 未満)に入る範囲は C 側の責務。
9. `fle_win.c` の `destroy_tree` は top-level 窓の破棄で `fle_raster_window_destroyed(lid)` を呼ぶ(読み)。**そのときの JS 側(canvas を消すか残すか)は未決**: デタッチ窓は閉じても canvas は Worker から取り戻せない(detach-window.md §3.3)。

## 8. この harness の「エンジンの代役」について

`engine.c` は FL のコードではない。面の表と上の API を実装した小さな wasm(約 250 行)で、1 枚の面(実エンジンの PNG を BGRX にしたもの)を持ち、毎フレーム矩形を 1 つ書き換えて `present(seq)` を呼ぶ。`present` は JS の import で、JSPI の `Suspending`(中断)にも、`Atomics.wait`(ブロック)にもなる。`fle_test_*` はテスト用で、契約の一部ではない。
