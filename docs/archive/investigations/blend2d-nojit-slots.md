# X2: JIT 無効の Blend2D（`Agg2D`）の全スロット走査、wasm との一致、基準の SHA の切り分け

実行日: 2026-10-30〜31。基準 HEAD `018ba73`（`git archive` を `lift/out-x2/src/` に展開。ワークツリーは読み取りのみ。HEAD は途中で 167a1db に進んだが、変更したファイルは同一）。決定文書: `docs/decisions/0120-blend2d-no-jit.md`（仮の決定）。差分は `lift/out-git/x2-results/x2.diff`、ログと道具は同じ場所の `logs/`、`tools/`。

**実機の Windows との照合は 0 件**。凡例: **実測**＝コマンドの出力を見た事実、**読み**＝逆アセンブルとコードを読んだ解釈（実行して確かめていない）、**未確認**。

## 1. 結論（先に）

| # | 問い | 答え | 区分 |
|---|---|---|---|
| 1 | 基準の再現（6 モジュール、FRUITY 無効、フォント配置あり） | a4: `init_count=942 of 942`、`895/895`、`returned from entry`、終了 0、functions **5,308**、entries **1,007,947**、native SHA-256 **`27cdf764f10ffabc08bba2d4afb45815441cdb4e35c05221b102ce5908d23f6d`**（決定 0118 の値と一致）。期待どおり | 実測 |
| 2 | X1 の「同じ入力から作った 2 つの native で SHA が違う」の原因 | **ビルドしたディレクトリの名前が `-g` のデバッグ情報（`.debug_str`）に入るため**。`-g` を外すと OUT の名前を変えても SHA は同一 | 実測（X1 のバイナリは消えていて直接比較せず。同じ現象を再現して特定。X1 の差も同じ原因と推測） |
| 3 | スロット走査（native、JIT 無効） | **130 スロットすべてを呼んだ**（実体 78 + 1 命令スタブ 52）。**138 テスト全部が完走**（`status=done`、stderr 0 バイト）。停止 0、壁 0 | 実測 |
| 4 | 画素の期待値の照合 | SPEC（仕様から手計算）で検査できたスロット **38**、READING（ゲストの関数の読みに対する検査）**18**、**画素ハッシュのみ 22**（実体 78 の内訳。ある検査が見る状態は、その直前に呼んだスロットが作ったものとして数えた）。CHECK 100 件が全部 PASS（初回は 5 件 FAIL。原因の分類と期待値を直した理由は §4.3） | 実測 |
| 5 | native と wasm の一致 | **138 テストすべてで `RESULT`／`CHECK`／`VALUE` の行がバイト一致**。130 スロットすべてについて、それを呼ぶ全テストが一致。極端な値の 128 通りも一致 | 実測 |
| 6 | `-O1` の別翻訳単位の関数 | 63 関数（dsp 15 + Blend2D 48）。**走査が実行したのは 44 で、すべて Blend2D**。依頼が挙げた 4 関数（`f_4859b70`、`f_4865f60`、`f_4871d60`、`f_487e100`）はすべて実行され、native と一致。**-O1 版の浮動小数点・SSE の差による不一致は 0 件** | 実測 |
| 7 | 壁 | 当たらなかった（0 件）。**ただし極端な値のうち 4 通り（slot 67 の楕円の半径が NaN／±無限大／DBL_MAX）は native で止まらない**（20 秒で打ち切り）。wasm では実行していない | 実測 |
| 8 | 優先順（FLEngine が呼ぶ入口の静的な数） | 窓 200 命令: 103（50）、85（44）、82（31）、42（27）、121（25）、11（23）、9（22）、83（21）、26（17）、63（17）。全部、本走査で呼んでいる。値の検査は 103、85、82、42、121、9、83、63 が SPEC、11、26 が READING（ハッシュのみのスロットは上位 10 にはない） | 読み（静的）。上位 10 は下の表 |

## 2. 方法

### 2.1 構成

6 モジュール構成（`a4-multi.sh` の 5 モジュール指定 + FLEngine。基底: dsp 0x2000000、Blend2D 0x4700000、QFC 0x1d00000、freetype 0x4b00000、ILTools 0x6000000。FRUITY 無効、VFS のフォント 2 ファイル、FLEngine と QFC と ILTools のフック点の注釈）。素材は X1 と同じ（Blend2D.dll の SHA-256 `66634d24…66066c7`）。道具は `lift/out-x2/tools/run6.sh`（引数: OUT のディレクトリ）。

走査用のビルドは Blend2D の種に X1 の 964 個（`lift/harness/seeds/blend2d-4700000-seeds964.txt`）を使う（種 58 個では `AggCreateInstance` の先の switch 表が足りない。X1 §4.1）。注釈 `lift/harness/hooks/blend2d-no-jit-sites.txt`（決定 0120）を付ける。

### 2.2 プローブ

`lift/harness/b2d_scan.c`（+ `b2d_scan_tests.h`、`b2d_scan_tests2.h`、`b2d_scan_table.h`）。**1 プロセス 1 テスト**: DllMain の後で `AggCreateInstance`、`attach(buffer, w, h, stride)`、テストの呼び出し、結果行。各スロットの呼び出しの前に `scan: CALL <slot>` を出すので、停止したときは最後の行が止まったスロットを示す。出力は**アドレスを含まない**ので native と wasm の行を比較できる。native と wasm で同じソース（`FLE_B2D_SCAN=<test>`、`list` で一覧）。

引数の形は逆アセンブルの**読み**（x64 の呼び出し規約: `this` が rcx、整数が rdx／r8／r9 とスタック、`double` は同じ位置の xmm1〜3 とスタック）。`cp()` は各位置の値を整数レジスタと xmm の両方に入れて呼ぶので、関数が読む方に合う。スロットの引数の意味は、本表の「役割」列が**読み**。

### 2.3 測定の手順

1. native: **138 テスト** = 名前付きのテスト 69（`stubs`、`graddump`、`clipdump`、`tiny*` ほかを含む）+ `blendmode:0..28`（29）+ `fuzz:1..40`（40）を 1 本ずつ `tools/scan_all.sh`。`list` は 72 の名前を出す（`blendmode`、`fuzz`、`extreme` は引数つきで別に走らせる）。`extreme` は 12 の値 × 11 の操作の **132 通りを別に**走らせた（138 に含まない。§4.5）。
2. wasm: 同じ構成の生成 C から `build_nmod_wasm.sh`（`B2D_SCAN=1`、FORCE、JOBS=2、dsp の単位は 1 本ずつ）。units 4 分 15 秒（最大 RSS 8.4 GiB）、o1 29 分 19 秒（最大 RSS 3.5 GiB）、link 9 秒。`flengine6.wasm` 453,210,447 バイト。node v24.19.0 で `run_node_nmod.cjs`（`FLE_WASM_FAST_RETADDR=1`）。`tools/wasm_all.sh`。
3. 比較: `tools/cmp_native_wasm.py`（`RESULT`／`CHECK`／`VALUE` の行をテストごとに）。

## 3. Agg2D の vtable（読み）

130 スロット、異なる関数 79 個（X1）。スロット 14、16、18 は `xor eax,eax; ret`、49 スロットは `ret`（RVA 0xc530）の**1 命令のスタブ**（合計 52 スロット）。実体のあるスロットは **78**。本作業が `static/agg-slots-dis.txt`（77 関数の逆アセンブル、`tools/x2_slots_dis.py`）を読んで役割と引数を整理した。

**X1 の読みの訂正**: X1 §3.3 はスロット 121 を「`ellipse(cx, cy, rx, ry)`（FLEngine の `P_DigitWheel` が呼ぶ）」と書いたが、逆アセンブルでは **67 が `ellipse(cx,cy,rx,ry)`**（`rx`、`ry` を渡して `0x4759490` 系を呼ぶ形）で、**121 は線形グラデーションの設定（`linearGradient(x1,y1,x2,y2,fill?)`）**。FLEngine の `f_6cb3d0` はスロット 0x3c0（120）で色の勾配表のポインタを得て、`0x3c8`（121）を呼ぶ。つまり FLEngine の「121」の 25 か所は楕円ではなくグラデーション。X1 の「楕円」の使用箇所数の読みは訂正が要る（本作業の読み。実測した `gradient`／`gradvert` の画素が、`linearGradient` として端と中央の色に一致した）。

## 4. 結果

### 4.1 スロット別（native、JIT 無効、wasm との一致）

列: 「FLEngine の箇所」は静的な数（窓 40／窓 200／上界。上界は `call [reg+disp]` で変位が 8×スロットの全部で、他の vtable も数えるため**過大**）。「テスト」は、そのスロットを呼ぶ非ファズのテスト数／ファズのシード数／呼び出し総数。「値の検査」は SPEC か READING か「ハッシュのみ」か（後述）。**wasm との一致は全 130 スロットで、そのスロットを呼ぶ全テストが一致**（`logs/slot-wasm-agreement.txt`）。

| slot | RVA of the function | role (reading) | FLEngine sites: window 40 / 200 / upper bound | tests that call it (non-fuzz) / fuzz seeds / calls | value check |
|---:|---|---|---|---|---|
| 0 | 0x56700 | destructor (delete this, size 0x1170) | 0 / 0 / 0 | 1 / 0 / 1 | hash only |
| 1 | 0x56c90 | attach(buf,w,h,stride) | 0 / 0 / 0 | 69 / 40 / 160 | hash only |
| 2 | 0xc530 (stub) | stub: ret | 0 / 1 / 1093 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 3 | 0x56d70 | clipBox(x1,y1,x2,y2) doubles | 3 / 13 / 3603 | 3 / 40 / 414 | SPEC (clipbox) |
| 4 | 0xc530 (stub) | stub: ret | 0 / 0 / 1551 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 5 | 0xc530 (stub) | stub: ret | 0 / 0 / 3620 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 6 | 0x56dc0 | clearAll (tail-jumps to 8) | 0 / 0 / 895 | 1 / 0 / 1 | READING (clearall6) |
| 7 | 0xc530 (stub) | stub: ret | 0 / 0 / 1999 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 8 | 0x56dd0 | clearAll(r,g,b,a) | 0 / 0 / 1625 | 13 / 0 / 54 | SPEC (clear, clipbox) |
| 9 | 0x56e70 | worldToScreen(&x,&y) | 0 / 22 / 656 | 2 / 0 / 5 | SPEC (transform) |
| 10 | 0x56ec0 | screenToWorld(&x,&y) | 0 / 0 / 410 | 1 / 0 / 1 | SPEC (scalar) |
| 11 | 0x56f50 | worldToScreen(scalar) | 8 / 23 / 471 | 1 / 0 / 1 | READING (scalar) |
| 12 | 0x56fd0 | screenToWorld(scalar) | 0 / 0 / 519 | 1 / 0 / 1 | READING (scalar) |
| 13 | 0x57070 | alignPoint(&x,&y) | 0 / 0 / 311 | 1 / 0 / 1 | hash only |
| 14 | 0xb820 (stub) | stub: xor eax,eax; ret | 1 / 3 / 560 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 15 | 0x56730 | blendMode(n 0..28 -> table) | 2 / 6 / 664 | 0 / 40 / 1540 | hash only |
| 16 | 0xb820 (stub) | stub: xor eax,eax; ret | 0 / 0 / 273 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 17 | 0xc530 (stub) | stub: ret | 0 / 0 / 334 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 18 | 0xb820 (stub) | stub: xor eax,eax; ret | 1 / 11 / 369 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 19 | 0xc530 (stub) | stub: ret | 0 / 0 / 379 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 20 | 0xc530 (stub) | stub: ret | 0 / 0 / 219 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 21 | 0xc530 (stub) | stub: ret | 0 / 0 / 217 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 22 | 0x57180 | context call (vtable+0x100), no argument | 0 / 6 / 189 | 1 / 0 / 1 | hash only |
| 23 | 0x571a0 | returns double at impl+0x28 | 0 / 2 / 156 | 1 / 0 / 1 | READING (misc) |
| 24 | 0xc530 (stub) | stub: ret | 0 / 0 / 106 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 25 | 0x571b0 | returns 0.0 | 0 / 0 / 143 | 1 / 0 / 1 | READING (misc) |
| 26 | 0x572f0 | fillColor(packed word) | 10 / 17 / 242 | 1 / 0 / 1 | READING (fillcolor) |
| 27 | 0x57310 | fillColor(r,g,b,a) | 0 / 1 / 222 | 40 / 40 / 4109 | SPEC (alphafill, blend) |
| 28 | 0x57350 | noFill | 0 / 0 / 249 | 12 / 40 / 1994 | SPEC (curve, nofill) |
| 29 | 0x57360 | lineColor(packed word) | 5 / 15 / 389 | 1 / 0 / 1 | READING (fillcolor) |
| 30 | 0x57380 | lineColor(r,g,b,a) | 1 / 1 / 105 | 18 / 40 / 3026 | SPEC (curve, line) |
| 31 | 0x573c0 | noLine | 11 / 16 / 397 | 39 / 40 / 3087 | SPEC (alphafill, blend) |
| 32 | 0xc530 (stub) | stub: ret | 0 / 0 / 183 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 33 | 0xc530 (stub) | stub: ret | 0 / 0 / 101 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 34 | 0xc530 (stub) | stub: ret | 0 / 0 / 107 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 35 | 0xc530 (stub) | stub: ret | 0 / 0 / 151 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 36 | 0xc530 (stub) | stub: ret | 0 / 0 / 114 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 37 | 0xc530 (stub) | stub: ret | 0 / 0 / 143 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 38 | 0xc530 (stub) | stub: ret | 0 / 0 / 352 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 39 | 0xc530 (stub) | stub: ret | 0 / 0 / 298 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 40 | 0xc530 (stub) | stub: ret | 0 / 0 / 70 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 41 | 0xc530 (stub) | stub: ret | 0 / 0 / 119 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 42 | 0x57880 | lineWidth(double) set | 13 / 27 / 196 | 19 / 40 / 6029 | SPEC (curve, line) |
| 43 | 0x578b0 | lineWidth get | 1 / 1 / 40 | 2 / 0 / 3 | READING (linewidth) |
| 44 | 0x578c0 | lineCap set | 0 / 0 / 37 | 2 / 40 / 6006 | READING (linecaps) |
| 45 | 0x57900 | lineCap get(&int) | 0 / 0 / 28 | 1 / 0 / 3 | READING (linecaps) |
| 46 | 0x57930 | lineJoin set | 4 / 16 / 62 | 1 / 40 / 6005 | READING (linecaps) |
| 47 | 0x57990 | lineJoin get(&int) | 0 / 0 / 793 | 1 / 0 / 5 | READING (linecaps) |
| 48 | 0x579e0 | bool set (impl+0x21) | 0 / 0 / 161 | 1 / 0 / 2 | READING (misc) |
| 49 | 0x57a10 | bool get | 0 / 0 / 353 | 1 / 0 / 4 | READING (misc) |
| 50 | 0xc530 (stub) | stub: ret | 0 / 0 / 68 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 51 | 0xc530 (stub) | stub: ret | 0 / 0 / 24 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 52 | 0x571c0 | resetTransformations | 0 / 0 / 37 | 4 / 40 / 358 | SPEC (scalar, transdraw) |
| 53 | 0xc530 (stub) | stub: ret | 0 / 0 / 13 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 54 | 0xc530 (stub) | stub: ret | 0 / 0 / 37 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 55 | 0x571f0 | rotate(angle rad) | 0 / 0 / 9 | 2 / 30 / 61 | SPEC (transform) |
| 56 | 0x572a0 | scale(sx,sy) | 0 / 0 / 19 | 2 / 30 / 53 | SPEC (transdraw, transform) |
| 57 | 0x57220 | skew/2-double matrix op | 0 / 0 / 14 | 1 / 31 / 54 | hash only |
| 58 | 0x57260 | translate(dx,dy) | 1 / 2 / 12 | 3 / 31 / 65 | SPEC (scalar, transdraw) |
| 59 | 0x572e0 | flag clear (Agg2D+0x58) | 0 / 0 / 104 | 1 / 0 / 1 | hash only |
| 60 | 0x572e0 | flag clear (same function as 59) | 0 / 0 / 98 | 1 / 0 / 1 | hash only |
| 61 | 0x57a30 | line(x1,y1,x2,y2) | 2 / 3 / 39 | 6 / 40 / 267 | SPEC (line) |
| 62 | 0x57ab0 | triangle(6 doubles) | 0 / 4 / 626 | 1 / 40 / 280 | SPEC (geometry) |
| 63 | 0x57bd0 | rectangle(x1,y1,x2,y2) | 7 / 17 / 199 | 31 / 40 / 629 | SPEC (alphafill, blend) |
| 64 | 0x57cd0 | roundedRect(x1,y1,x2,y2,r) | 0 / 0 / 443 | 4 / 40 / 272 | SPEC (roundrect) |
| 65 | 0x57d00 | roundedRect(..,rx,ry) | 0 / 0 / 29 | 1 / 40 / 270 | SPEC (roundrect2) |
| 66 | 0x57e30 | roundedRect(..,4 radii) | 0 / 0 / 44 | 1 / 40 / 292 | SPEC (roundrect3) |
| 67 | 0x580a0 | ellipse(cx,cy,rx,ry) | 1 / 5 / 47 | 5 / 40 / 268 | SPEC (ellipse) |
| 68 | 0x581a0 | arc(cx,cy,rx,ry,start,sweep) | 0 / 0 / 17 | 1 / 40 / 256 | READING (arc) |
| 69 | 0x582c0 | star(cx,cy,r1,r2,start,n) | 0 / 0 / 35 | 1 / 39 / 269 | hash only |
| 70 | 0x58440 | quadricCurve(6 doubles) | 0 / 0 / 26 | 1 / 40 / 303 | SPEC (curve) |
| 71 | 0x584f0 | cubicCurve(8 doubles) | 0 / 0 / 38 | 1 / 40 / 272 | SPEC (curve) |
| 72 | 0x585c0 | polygon(xy*,n) | 0 / 0 / 74 | 1 / 40 / 279 | READING (geometry) |
| 73 | 0x58690 | polyline(xy*,n) | 0 / 0 / 33 | 1 / 40 / 296 | READING (geometry) |
| 74 | 0xc530 (stub) | stub: ret | 0 / 0 / 8 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 75 | 0xc530 (stub) | stub: ret | 0 / 0 / 20 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 76 | 0xc530 (stub) | stub: ret | 0 / 0 / 176 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 77 | 0xc530 (stub) | stub: ret | 0 / 0 / 43 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 78 | 0xc530 (stub) | stub: ret | 0 / 0 / 125 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 79 | 0xc530 (stub) | stub: ret | 0 / 0 / 33 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 80 | 0xc530 (stub) | stub: ret | 0 / 0 / 24 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 81 | 0xc530 (stub) | stub: ret | 0 / 0 / 22 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 82 | 0x586e0 | resetPath | 19 / 31 / 134 | 15 / 40 / 1057 | SPEC (curves, path) |
| 83 | 0x586f0 | moveTo | 10 / 21 / 185 | 15 / 40 / 1055 | SPEC (curves, path) |
| 84 | 0x58700 | moveRel | 0 / 0 / 44 | 1 / 0 / 1 | READING (pathrel) |
| 85 | 0x58780 | lineTo | 19 / 44 / 179 | 9 / 38 / 180 | SPEC (path, pathboth) |
| 86 | 0x58790 | lineRel | 0 / 0 / 14 | 1 / 38 / 182 | SPEC (pathrel) |
| 87 | 0x58810 | horLineTo | 1 / 6 / 57 | 2 / 39 / 161 | SPEC (curves) |
| 88 | 0x58880 | horLineRel | 0 / 0 / 29 | 1 / 38 / 166 | SPEC (pathrel) |
| 89 | 0x588f0 | verLineTo | 0 / 1 / 47 | 2 / 39 / 165 | SPEC (curves) |
| 90 | 0x58960 | verLineRel | 0 / 0 / 18 | 1 / 38 / 168 | SPEC (pathrel) |
| 91 | 0x589d0 | arcTo | 0 / 2 / 74 | 1 / 38 / 135 | hash only |
| 92 | 0x58a00 | arcRel | 0 / 0 / 27 | 1 / 40 / 125 | hash only |
| 93 | 0x58ae0 | quadTo | 0 / 14 / 131 | 1 / 40 / 418 | SPEC (pathcurve) |
| 94 | 0x58af0 | quadRel | 0 / 0 / 29 | 1 / 37 / 146 | hash only |
| 95 | 0x58b90 | quad smooth to | 0 / 0 / 13 | 1 / 40 / 261 | hash only |
| 96 | 0x58ba0 | quad smooth rel | 0 / 1 / 26 | 1 / 40 / 261 | hash only |
| 97 | 0x58c20 | cubicTo | 3 / 4 / 47 | 1 / 38 / 167 | SPEC (pathcurve) |
| 98 | 0x58c30 | cubicRel | 0 / 0 / 44 | 1 / 39 / 169 | hash only |
| 99 | 0x58d20 | cubic smooth to | 0 / 0 / 24 | 1 / 40 / 261 | hash only |
| 100 | 0x58d30 | cubic smooth rel | 0 / 0 / 30 | 1 / 40 / 261 | hash only |
| 101 | 0x58dd0 | addEllipse(cx,cy,rx,ry,dir) | 1 / 2 / 45 | 1 / 0 / 1 | hash only |
| 102 | 0x58e30 | closePolygon | 3 / 14 / 93 | 11 / 39 / 143 | SPEC (curves, path) |
| 103 | 0x59690 | drawPath(flag 0..5) | 12 / 50 / 212 | 14 / 40 / 1056 | SPEC (curves, path) |
| 104 | 0xc530 (stub) | stub: ret | 1 / 7 / 46 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 105 | 0xc530 (stub) | stub: ret | 0 / 0 / 25 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 106 | 0xc530 (stub) | stub: ret | 0 / 0 / 31 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 107 | 0xc530 (stub) | stub: ret | 0 / 0 / 12 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 108 | 0xc530 (stub) | stub: ret | 0 / 0 / 9 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 109 | 0xc530 (stub) | stub: ret | 0 / 0 / 5 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 110 | 0xc530 (stub) | stub: ret | 0 / 0 / 52 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 111 | 0xc530 (stub) | stub: ret | 0 / 0 / 92 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 112 | 0xc530 (stub) | stub: ret | 0 / 0 / 7 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 113 | 0xc530 (stub) | stub: ret | 0 / 0 / 30 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 114 | 0xc530 (stub) | stub: ret | 0 / 0 / 9 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 115 | 0xc530 (stub) | stub: ret | 0 / 0 / 26 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 116 | 0xc530 (stub) | stub: ret | 0 / 0 / 8 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 117 | 0xc530 (stub) | stub: ret | 0 / 0 / 16 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 118 | 0xc530 (stub) | stub: ret | 0 / 0 / 10 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 119 | 0xc530 (stub) | stub: ret | 0 / 0 / 3 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 120 | 0x57860 | fill/line colour ramp: returns a pointer to 256 x 8 bytes (4 x int16) | 1 / 1 / 13 | 9 / 40 / 258 | SPEC (gradient, gradvert) |
| 121 | 0x573d0 | linearGradient(x1,y1,x2,y2,fill?) | 8 / 25 / 133 | 7 / 36 / 85 | SPEC (gradient, gradvert) |
| 122 | 0x574b0 | radialGradient(x,y,r,fill?) | 0 / 0 / 39 | 1 / 37 / 96 | hash only |
| 123 | 0x575a0 | gradient with angle/scale (6 args) | 2 / 5 / 58 | 1 / 34 / 77 | hash only |
| 124 | 0x59740 | clipBox with ints? | 0 / 0 / 4 | 1 / 40 / 145 | SPEC (clipint) |
| 125 | 0xc530 (stub) | stub: ret | 0 / 0 / 2 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 126 | 0x59900 | pointInPath(x,y) | 1 / 1 / 3 | 1 / 0 / 6 | SPEC (pointinpath) |
| 127 | 0xc530 (stub) | stub: ret | 0 / 0 / 2 | 1 / 0 / 1 | called by `stubs` (returns 0 / nothing) |
| 128 | 0x59950 | reset to constructor defaults | 0 / 0 / 6 | 1 / 0 / 1 | hash only |
| 129 | 0x599f0 | 2-double store + call 42 | 1 / 4 / 21 | 1 / 0 / 1 | hash only |

値の検査の列の読み方: `SPEC (テスト名)` = 仕様から手計算した値と照合した（テスト名はそのうちの 2 つまで）。`READING` = ゲストの関数の読み（そのスロットが何をするか）に対する検査で、仕様との照合ではない。`hash only` = 画素ハッシュだけ（実機との照合なし。native と wasm の一致は測った）。「スタブ」は 1 命令の関数で、`stubs` テストで呼んで戻り値（0）とバッファ不変を確認した。

### 4.2 テスト別

`logs/table-tests.md`（ファズ 40 とブレンドモード 29 を除く 69 のテスト）:

| test | slots (declared) | result | SPEC pass | READING pass | FAIL | hash |
|---|---|---|---:|---:|---:|---|
| alphafill | 27 31 63 | done | 2 | 0 | 0 | 0x23208382fd213f25 |
| arc | 28 30 42 68 | done | 0 | 1 | 0 | 0x8a75dcb20de402b1 |
| arcrel | 28 30 42 82 83 92 103 | done | 0 | 0 | 0 | 0xd39e2df5f7e3e90c |
| big640 | 1 27 31 63 67 64 61 82 83 85 102 103 120 121 | done | 3 | 0 | 0 | 0x5ebb39d31700c325 |
| bigodd | 1 27 31 63 67 64 61 82 83 85 102 103 120 121 | done | 1 | 0 | 0 | 0x5ebb39d31700c325 |
| bigtall | 1 27 31 63 67 64 61 82 83 85 102 103 120 121 | done | 0 | 0 | 0 | 0x5ebb39d31700c325 |
| blend | 27 31 63 | done | 2 | 0 | 0 | 0x31decd4681241725 |
| clear | 8 | done | 2 | 0 | 0 | 0x5ebb39d31700c325 |
| clearall6 | 6 | done | 0 | 1 | 0 | 0x69bdfe076e1a2325 |
| clipbox | 3 8 27 31 63 | done | 4 | 0 | 0 | 0xeecdb29f92fc7325 |
| clipdump | 3 8 27 31 63 | done | 0 | 0 | 0 | 0xeecdb29f92fc7325 |
| clipint | 27 31 63 124 | done | 1 | 0 | 0 | 0xeecdb29f92fc7325 |
| clipreset | 3 27 31 63 | done | 0 | 0 | 0 | 0xeecdb29f92fc7325 |
| curve | 28 30 42 70 71 | done | 3 | 0 | 0 | 0x8e9a0454b809f457 |
| curves | 27 31 82 83 87 89 102 103 | done | 2 | 0 | 0 | 0x317826fbc09162a5 |
| drawflags | 27 30 42 82 83 85 102 103 | done | 0 | 0 | 0 | 0x5ebb39d31700c325 |
| dtor | 0 | done | 0 | 0 | 0 | 0x5ebb39d31700c325 |
| ellipse | 27 31 67 | done | 3 | 0 | 0 | 0xa33bf7274376093d |
| ellipse2 | 28 30 42 67 | done | 0 | 2 | 0 | 0x915b52c260855233 |
| fillcolor | 26 27 28 29 30 31 | done | 0 | 2 | 0 | 0x5ebb39d31700c325 |
| geometry | 27 28 30 31 42 62 72 73 8 | done | 2 | 3 | 0 | 0x47fe730a73a883c7 |
| graddump | 63 120 121 31 | done | 0 | 0 | 0 | 0xb89bf605ffe9d0c5 |
| gradient | 63 120 121 | done | 4 | 0 | 0 | 0x7d19bdc54568dce1 |
| gradline | 42 61 120 121 | done | 0 | 0 | 0 | 0x854b061371b55475 |
| gradradial | 63 120 122 | done | 0 | 0 | 0 | 0x7536fb7c1dc0cfed |
| gradrot | 63 120 123 | done | 0 | 0 | 0 | 0xc314a204d5254ce |
| gradvert | 63 120 121 | done | 3 | 0 | 0 | 0xee5133b36823ba88 |
| line | 30 42 61 | done | 2 | 0 | 0 | 0x191acb2c53753092 |
| linecaps | 44 45 46 47 | done | 0 | 2 | 0 | 0x5ebb39d31700c325 |
| linewidth | 42 43 | done | 0 | 1 | 0 | 0x5ebb39d31700c325 |
| misc | 22 23 25 48 49 | done | 0 | 1 | 0 | 0x5ebb39d31700c325 |
| nofill | 27 28 31 63 | done | 1 | 0 | 0 | 0x5ebb39d31700c325 |
| path | 27 31 82 83 85 102 103 | done | 2 | 0 | 0 | 0x8f7b6894179bbc5 |
| pathboth | 27 30 42 82 83 85 102 103 | done | 2 | 0 | 0 | 0x69fbc783453704f |
| pathcurve | 28 30 42 82 83 93 97 103 | done | 2 | 0 | 0 | 0x8e9a0454b809f457 |
| pathellipse | 27 28 30 31 42 82 83 91 101 103 | done | 0 | 0 | 0 | 0xc4dfc69ef0b4df43 |
| pathfill | 27 31 82 83 85 102 103 | done | 3 | 0 | 0 | 0x8f7b6894179bbc5 |
| pathrel | 27 31 82 83 84 86 87 88 89 90 102 103 | done | 1 | 1 | 0 | 0x317826fbc09162a5 |
| pathsmooth | 28 30 42 82 83 94 95 96 98 99 100 103 | done | 0 | 0 | 0 | 0x87e520d898225ba4 |
| pathstroke | 28 30 42 82 83 85 102 103 | done | 2 | 0 | 0 | 0xcaac2ae6f7d0a714 |
| pointinpath | 27 31 82 83 85 102 126 | done | 2 | 0 | 0 | 0x5ebb39d31700c325 |
| reattach | 1 27 31 63 | done | 7 | 0 | 0 | 0x5ebb39d31700c325 |
| rect | 27 31 63 | done | 3 | 0 | 0 | 0xd1747fb20144b325 |
| rectline | 28 30 42 63 | done | 3 | 0 | 0 | 0x34ae4c3fe79ae225 |
| reset | 27 31 63 128 | done | 0 | 0 | 0 | 0x4bba308940b98ffd |
| roundrect | 27 31 64 | done | 4 | 0 | 0 | 0x2800bf475898d793 |
| roundrect2 | 27 31 65 | done | 2 | 0 | 0 | 0xbc27e487b4a61fb2 |
| roundrect3 | 27 31 66 | done | 1 | 0 | 0 | 0x55eace607f262e0e |
| scalar | 10 11 12 13 52 58 | done | 1 | 2 | 0 | 0x5ebb39d31700c325 |
| skew | 9 27 31 52 57 63 | done | 0 | 0 | 0 | 0xf4e5841b2865db47 |
| slot129 | 43 129 | done | 0 | 0 | 0 | 0x5ebb39d31700c325 |
| slots5960 | 59 60 | done | 0 | 0 | 0 | 0x5ebb39d31700c325 |
| star | 27 31 69 | done | 0 | 0 | 0 | 0x29ecf7d19d0bebe2 |
| stubs | the 52 stub slots: 2 4 5 7 14 16 17 18 19 20 21 24 32-41 50 51 53 54 74-81 104-119 125 127 | done | 0 | 4 | 0 | 0x5ebb39d31700c325 |
| thickline | 30 42 44 61 | done | 0 | 0 | 0 | 0xd989ee94f9958d68 |
| tiny | 1 27 31 63 | done | 2 | 0 | 0 | 0x5ebb39d31700c325 |
| tiny1 | 1 27 31 63 | done | 0 | 0 | 0 | 0x5ebb39d31700c325 |
| tiny16x1 | 1 27 31 63 | done | 0 | 0 | 0 | 0x5ebb39d31700c325 |
| tiny1b | 1 | done | 0 | 0 | 0 | 0x5ebb39d31700c325 |
| tiny1x16 | 1 27 31 63 | done | 0 | 0 | 0 | 0x5ebb39d31700c325 |
| tiny2 | 1 27 31 63 | done | 0 | 0 | 0 | 0x5ebb39d31700c325 |
| tiny3 | 1 27 31 63 | done | 0 | 0 | 0 | 0x5ebb39d31700c325 |
| tiny3b | 1 | done | 0 | 0 | 0 | 0x5ebb39d31700c325 |
| tiny4 | 1 27 31 63 | done | 0 | 0 | 0 | 0x5ebb39d31700c325 |
| tiny5 | 1 27 31 63 | done | 0 | 0 | 0 | 0x5ebb39d31700c325 |
| tiny7 | 1 27 31 63 | done | 0 | 0 | 0 | 0x5ebb39d31700c325 |
| tiny9 | 1 27 31 63 | done | 0 | 0 | 0 | 0x5ebb39d31700c325 |
| transdraw | 27 31 52 55 56 58 63 | done | 4 | 0 | 0 | 0x7a30ff617a449e85 |
| transform | 9 52 55 56 58 | done | 4 | 0 | 0 | 0x5ebb39d31700c325 |

(ファズ 40 のシードとブレンドモード 29 は `logs/final-native/fuzz_*.out`、`blendmode_*.out`。ブレンドモードの値は 4.4。)

### 4.3 初回の FAIL 5 件（期待値を直した理由）

最初の走査（`logs/scan-first/`、保存）で CHECK の FAIL が 5 件出た。**出力に合わせて期待値を直したのではなく、原因を調べて分類した**（期待値の修正は計算に基づく。しきい値だけを後から決めたものは、そう書いた）:

| テスト | 初回の結果 | 原因（実測／計算） | 扱い |
|---|---|---|---|
| `alphafill` | B=0x19、期待 0x1a | **私の手計算の誤り**。source-over（r=255, a=128）で、プリマルチプライドのソース R=128、`B = 0 + 0x33 × (255−128)/255 = 25.4` → 25 = 0x19。期待値 0x1a は 25.4 を切り上げた誤り | 期待値を 0x19 に直した（±1 の許容も維持） |
| `clipreset` | 赤 1,024 画素、期待 4,096（「新しい clipBox は古いものを置き換える」） | **私の仮定の誤り**。`clipBox` を 2 回呼ぶと（16..48 の次に 0..64）、**交差**になる（1,024）。`clipdump` で、8..56 の次に 16..48 も 1,024 | CHECK を外し、値（`red-after-16-48-then-0-64`）を記録。「置き換える」は仕様とは書いていない |
| `curve` の `quad-start` | `(10,50)` が `62 55 48`（背景に近い）、期待は全白 | **私の仮定の誤り**。ストロークの始点は butt cap で四角に切れるので、始点の画素は部分的にしか覆われない。「始点の画素が全白」とは言えない | 「始点の画素が背景ではない」に弱めた（SPEC: 太さ 2 のストロークが始点を通る） |
| `ellipse2` の `ellipse-stroke-right` | `(52,32)` が `03 f4 f3`、期待 `00 ff ff` | 太さ 2 のストロークの右端での曲率で、画素が約 95% しか覆われない（曲率半径 b²/a = 7.2） | **しきい値（> 0xe8）は初回の出力を見た後で決めた**。READING と書いた（導出した値ではない） |
| `gradient` の `grad-rows-equal` | 行 5 と行 50 が違う | `graddump` で、**行どうしの差は最大 1（8 ビットの 1 段）、63 チャネル**。原因は未確定（丸めの経路の違いと思われるが、未調査） | 「差が 1 以内」に弱め、差の最大値を VALUE に出す（`grad-row5-vs-row50-largest-channel-difference` = 1）。**同じ水平グラデーションの行が 1 ずれるのは、Blend2D の固定パイプラインの性質か、私のランプの作り方か未確認** |

### 4.4 ブレンドモード（slot 15、`blendMode(n)`）

背景 `33 22 11 ff` の上に、半透明の青 `(r=0,g=0,b=255,a=128)` の矩形（8..56）を描いた中央の画素（`logs/table-blend.md`。native と wasm は一致）:

- **0 と 1**: `80 00 00 80`（背景が消え、ソースがそのまま入る。単純コピーに見える）。
- **3、13、25、26、27、28**: `99 11 08 ff`（X1 の source-over の値と同じ）。
- **2、4〜12、14〜24**: `33 22 11 ff`（**背景のまま。何も描かれない**）。

29 種のうち 8 種だけが画素を変え、21 種は変えない。**これが仕様どおりかは検証していない**（`blendMode(n)` の n から BLCompOp への変換は表（RVA 0x56820、29 項目）で、0→0xb、1→1、2→6、3→`edx`…と読めるが、読んだだけ。BLCompOp の 11 は CLEAR、6 は DST_COPY（何もしない）。表の読みと整合する値もある（slot 15 の n=2 が DST_COPY で「何も描かない」）が、全 29 項目の照合はしていない）。**FLEngine がスロット 15 を呼ぶ箇所は窓 200 で 6**。

### 4.5 ファズと極端な値

- `fuzz:1..40`（決定的な乱数、150 手 × 40、全スロットの描画・パス・変換・クリップ・グラデーション・ブレンドモードをランダムに混ぜる）: 全部完走、wasm と一致。
- `extreme:V*100+O`（12 の値 × 11 の操作 = 132 通り。値: NaN、+inf、−inf、DBL_MAX、最小の非正規数、−0.0、±1e20、2^31、2^32、2^52、小さな正規数）: native は **128 通り完走、4 通りが止まらない**（`extreme:3`／`103`／`203`／`303` = 楕円の半径に NaN／+inf／−inf／DBL_MAX）。20 秒の `alarm` で打ち切り。gdb で止めると、`f_487e100`（-O1 の別翻訳単位の関数）← `f_48594d0` ← `f_4840d30`（塗りの共通入口）← `f_484b4c0` ← slot 67 の呼び出しの中の `fle_mx_addsd`（SSE の浮動小数点加算のヘルパー）を実行中。**無限ループか極端に長い反復**で、これが lift の誤りか、実機の Blend2D も同じ入力で止まらない（楕円の分割数が半径に比例する）のかは**未確認**。FLEngine が半径に NaN／極端な値を渡す可能性も未確認。完走した 128 通りは wasm でも完走し、native と一致。
- 極端な値の後の `clear` + 矩形の画素（1,024 画素）が正しい（`after-extremes-a-plain-rect-is-right`）。

### 4.6 `-O1` の別翻訳単位（wasm の locals 超過）

`build_nmod_wasm.sh` の `o1` 段が、locals が 50,000 を超える関数を別の翻訳単位に出して `-O1` でコンパイルする。本構成（種 964 個）の対象は **63 関数**: `m1-05.c`（dsp）10、`m1-10.c`（dsp）5、`m2-02.c`（Blend2D）48（`logs/list-o1.tsv`）。`a4-six-module-wasm-verification` の 19 個（dsp 15 + Blend2D 4）は種 58 個の構成。

gdb の計数ブレークポイントを native の走査バイナリの 63 関数に置いて（`tools/gdb_count_o1.py`）、全 138 テストで入った回数を数えた（`logs/o1count/`）:

| 単位 | 関数数 | 走査で 1 回以上実行 |
|---|---:|---:|
| `m1-05.o`（dsp） | 10 | 0 |
| `m1-10.o`（dsp） | 5 | 0 |
| `m2-02.o`（Blend2D） | 48 | **44** |

依頼が挙げた 4 関数の実行回数（全テストの合計）: `f_487e100` 1,232（72 テスト）、`f_4865f60` 557（62）、`f_4871d60` 288（22）、`f_4859b70` 132（21）。X1 の「線・パス・楕円で使われる」と整合し、ファズとスロット別テストでも実行される。**これらの -O1 版は native と同じ画素を出した**（4 関数を通る全テストで一致）。

**測れなかったこと**: dsp の 15 関数の -O1 版（Blend2D の走査は dsp を呼ばない）、Blend2D の 4 関数（`f_48643d6`、`f_487026b`、`f_487c551`、`f_48883c7`。走査で入らなかった）の -O1 版。**走査に含めていない入力（画像、テキスト）で別の -O1 関数が使われる可能性**はある。

**浮動小数点・SSE の差（-O1 と -O0）は、一致したので切り分けの必要がなかった**。ただし wasm の浮動小数点は、ゲストの SSE をランタイムのヘルパー（`fle_mx_*`。MXCSR の丸めと例外フラグを再現）で実行するので、**コンパイラの最適化の差が結果に出にくい構造**。-O1 でも一致したのは、この構造の帰結とも読める（読み）。

### 4.7 Blend2D の関数の被覆と未解決 `jmp reg`

gdb の計数ブレークポイント（`tools/gdb_cover.py`。`b2d_scan_run` の開始後だけ）で、Blend2D の生成 C の 3,093 関数のうち、138 テストの間に 1 回以上入ったのは **350 関数**（ファズを除く 69 + ブレンド 29 のテストだけでは 243）。

未解決の `jmp reg`（生成 C で `fle_dispatch(R[x])` になっているもの）は、**出力に 351 か所（関数のコピーを含む）、サイトの VA として 81**（X1 と同数）。そのうち、走査が入った関数の中のサイトは 18（コピー込みで 145 か所）で、**走査はその 18 サイトのどれでも停止しなかった**（`logs/unresolved-jmp-sites.tsv`）。理由の内訳（`msvc_rejected` 62 行と、固定の未解決 19）: switch の表の幅を決められない（`N not derived`）が大半、`.pdata` の外の関数、別の関数が内側から始まる、など。これらを直す対象は本作業では出なかった（走査が当たらなかったため）。**残りの 63 サイトは走査が入っていないので、そこに当たる入力は未測定**。

### 4.8 FLEngine が呼ぶ入口の優先順（静的）

`tools/x2_vtuse_x1.py`（X1 の `b2d_vtuse.py` と同じ。窓 40 は X1 の `fl-vtuse.txt` と一致）。`f_6cb190` と `f_6cb0b0` の呼び出し 130 か所のうち、窓 200 命令で Agg の vtable を使うのは **99 か所**（窓 40 では 81 か所）、変位は 45 種（`logs/vtuse-x1-window200.txt`）。上位:

| 順 | スロット | 役割（読み） | 窓 200 | 窓 40 | 値の検査 |
|---:|---:|---|---:|---:|---|
| 1 | 103 | drawPath(flag) | 50 | 12 | SPEC（`path`、`pathfill`、`pathstroke`、`pathboth`、`drawflags` ほか） |
| 2 | 85 | lineTo | 44 | 19 | SPEC（`pathfill` ほか） |
| 3 | 82 | resetPath | 31 | 19 | SPEC |
| 4 | 42 | lineWidth set | 27 | 13 | SPEC（線の太さの効果で。`line`、`rectline`） |
| 5 | 121 | **linearGradient**（X1 は楕円と読んだ） | 25 | 8 | SPEC（`gradient`、`gradvert`。端と中央の色） |
| 6 | 11 | worldToScreen(scalar) | 23 | 8 | READING（`scalar`。単位行列で 7 → 7 のみ） |
| 7 | 9 | worldToScreen(&x,&y) | 22 | 0 | SPEC（`transform`） |
| 8 | 83 | moveTo | 21 | 10 | SPEC |
| 9 | 26 | fillColor(packed) | 17 | 10 | READING（`fillcolor`） |
| 10 | 63 | rectangle | 17 | 7 | SPEC |
| … | 31 | noLine | 16 | 11 | SPEC |
| … | 46 | lineJoin set | 16 | 4 | READING |
| … | 29 | lineColor(packed) | 15 | 5 | READING |
| … | 93 | quadTo | 14 | 0 | SPEC（`pathcurve`。曲線の中点） |
| … | 102 | closePolygon | 14 | 3 | SPEC |
| … | 3 | clipBox | 13 | 3 | SPEC（`clipbox`） |

窓 200 の合計 445 箇所のうち、**スタブのスロット（2、14、18、104）は 22**、実体のあるスロットは 423。**この集計は FLEngine が実際に何回呼ぶかではない**（静的な箇所数。`CreateFruityInstance` の先は未到達）。`f_6cb190` の戻りの後の窓内で `[obj]` を経由する形だけを追うので、**保存したオブジェクトポインタを後で使う呼び出しは含まない**（下限）。

## 5. 壁

**見つからなかった**。依頼が挙げた壁の候補ごと:

| 候補 | 結果 |
|---|---|
| 未解決 `jmp reg` | 走査が通る関数の 18 サイトで停止 0。残り 63 サイトは入らなかった（未測定） |
| 未対応命令 | 走査で `fle_unsupported` に当たらなかった（`stderr` 0 バイト）。生成 C には AVX/FMA の未対応が 643 命令あるが、走査では実行されなかった（CPUID が AVX を名乗らない構成。X1 §4.2） |
| 行の無い import | 当たらなかった |
| 暗黙 TLS | JIT 無しの経路では不要（`FLE_B2D_TLS` なしで 138 テスト完走） |
| 実行可能メモリ | 138 テストの間 `VirtualAlloc`／`VirtualProtect`／`CreateFileMappingW`／`MapViewOfFile` は 0 回（`logs/memcalls/`、gdb の計数。`b2d_scan_run` の開始後） |
| スレッド | 使われなかった |
| **ハング（壁ではなく問題）** | slot 67（楕円）の半径が NaN／±無限大／DBL_MAX の 4 通り。native で 20 秒止まらない。wasm 未実行 |

「壁を最大 12」の枠は使わなかった。**これは「壁が無い」ことの証明ではない**: 走査が試した入力は 64×64 を中心に、描画の基本形（矩形、楕円、パス、線、グラデーション、変換、クリップ、ブレンドモード）で、画像・パターン・テキスト・複雑なクリップ（マスク）は含まない。

## 6. 基準の SHA の差の切り分け

### 6.1 手順

1. HEAD の木で 6 モジュール構成（`tools/run6.sh`、`EXPECT_FLE_SHA=skip`）を OUT の名前 `b1` で作る（`-g` あり）。native の SHA `7831cbea96a5245f677ba5e691a7a3508a1285f9977117316d5f61d6c080396d`。a4 は期待どおり。
2. 同じものを OUT の名前 `b2` で作る。native の SHA `1fad5a72417841dd22a0b48f99aebf26b57ad8b1b11c3c1f687b942c8b97669a`。**2 つは違う**（X1 と同じ現象を再現）。a4 は同一。
3. **112 個のオブジェクト（`.o`）を `cmp`**: 107 が違い、**5 が同じ**（`a4.o`、`image.o`、`main_two.o`、`trace.o`、`realmod_table.o`。ハーネス側。`-g` つきだが OUT のパスを埋め込むソースが無い）。
4. 違う 107 個すべてを `objcopy --strip-debug` して比較: **107 すべてが一致**（`tools/cmp_stripped.sh`）。→ **機械語・データは同じで、デバッグ情報だけが違う**。
5. 違う理由: `readelf --debug-dump=info` の `DW_AT_name`／`DW_AT_comp_dir` に `.../out-x2/b1/v-t2331d00e8ebb/src/m2-00.c` のような**ビルドの絶対パス**が入る。
6. リンク後のバイナリ（686,060,688 バイトで両方同じ）の差のセクション: `.note.gnu.build-id`（36 バイト）、`.debug_str`、`.debug_line_str` の 3 つだけ。`.text`、`.rodata`、`.data`、`.eh_frame`、`.dynsym` ほか 30 セクションは一致（`logs/link-sections-b1-b2.txt`）。
7. **`-g` を外す**ラッパー（`tools/wrap/clang`。`-g` の引数だけを落とし、他は同じ）で、OUT の名前を 3 通り（`g1`、`g2`、長い名前の `g-a-much-longer-output-directory-name-3`）で作る: native の SHA はすべて **`27cdf764f10ffabc…`**（決定 0118 の値）。functions 5,308、entries 1,007,947。

### 6.2 結論

**OUT の名前・パスが違うと、`-g` つきのビルドの SHA は変わる**（実測）。理由は `.debug_str` と `.debug_line_str` にビルドのパスが入るため。**X1 の 2 つのバイナリ（`f469d29f…`、`65c904e3…`）は消えていて直接は比べていない**。X1 の OUT の名前は `o1`／`o5` と違うので、同じ原因と**推測**する（X1 自身の推測と一致）。決定 0118 の「別の OUT・別の cargo target で独立な 2 回が同一」は、ビルドに使うラッパー（`tools/wrap/clang` で `-g` を落とす。`docs/investigations/fruity5/tools/wrap/clang`）が同じだったための一致と読める。`-g` つきで比べるなら、OUT のパスを揃えるか、`--strip-debug` 後に比べる。

**`27cdf764…` の再現は、`-g` を外したビルドで、OUT の名前が違っても同一**だった。

## 7. 限界・未確認

- **実機の Windows との照合は 0 件**。画素が「正しい」と言えるのは SPEC の範囲（38 スロット、CHECK 80 件）だけで、READING の 20 件は関数の読みに対する検査。**スロット 1（attach）は、全テストが依存するのでハッシュのみの列に出るが、attach 後の `pipeline provider fixed` の行（全テスト）が確認している**。
- 測ったのは **64×64 を中心にした 32 ビット PRGB32 の描画の基本形**。画像、パターン、テキスト、マスクのクリップ、アンチエイリアスの縁の画素値（楕円の面積は 3% 以内の確認だけ）は未測定。
- **ブラウザ（Chrome）では測っていない**。wasm は node（v24.19.0）だけ。`-sJSPI` なし（FORCE の既定）。
- wasm の `extreme` の 4 通り（ハング）は未実行。wasm の 138 テストの実行時間は 513 秒、native は 80 秒（最終の走り。プロセスごとの DllMain を含む）。速度は測っていない。
- スロット 15（`blendMode`）の 29 種のうち、画素が変わるのは 8 種。仕様どおりかは未検証。
- スロット 69（star）、91／92（arcTo／arcRel）、122／123（グラデーション）などは**引数の意味を確認していない**（呼べた、画素が変わったことだけ）。
- FLEngine の呼び出し箇所の静的な集計は下限（窓 200 で 99/130 か所）。**どのスロットを何回呼ぶか**は、`CreateFruityInstance` の先が未到達なので未測定。
- 「SPEC のスロット」の数え方は、検査の直前の（前の検査または `clear_bg` の後の）呼び出しを全部その検査のスロットとして数える**寛大な数え方**で、検査が実際に見ているのはそのうちの一部のこともある（`tools/x2_check_kinds.py`。窓を前の検査で区切る）。最初の数え方（テスト単位のタグ）は 45 と過大だった。
- 走査の期待値の一部は、初回の出力を見た後で直した（4.3）。直した理由は書いたが、**独立に導いた値ではないもの**が 2 件（`ellipse-stroke-right` のしきい値、`gradient` の行差 1）。
- 全体テストは 1 回の通しでは通っていない（`0120` の「テスト」）。
- `fe-lift` の `--site-overrides` は 32 ビットの `store32` だけ。複数サイトの競合（同じ語への 2 つの上書き）は検査していない。

## 8. 成果物

`lift/out-git/x2-results/`: `x2.diff`、`MERGE-NOTES.md`、`docs/0120-blend2d-no-jit.md`、`docs/README-0120-line.txt`、`docs/investigations/blend2d-nojit-slots.md`（これ）、`logs/`、`tools/`、`seeds/`。
