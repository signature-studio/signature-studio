# 0214: 画面更新の経路と窓のドラッグ — (段 1) ドラッグ中・ドラッグ後に画面が更新されない原因は runtime が『子窓の移動で露出した領域を無効化せず、ビットもコピーしない』ことだった。修正は `present2-runtime.diff`。(段 2〜) 表示経路の測定と短縮は下に追記。仮の決定

状態: **仮の決定(すべての選択)**。依頼は PM(決定 0214)。調査・実装・測定・文書は作業者 `present2`(2026-11-04 JST 03:52〜、総計 5 時間)。ワークツリーは HEAD **e7af37b** の `git archive`(`lift/out-present2/src/`)。最終の diff は HEAD **dddad9c** の `git archive` に `patch -p1 --dry-run` が当たることを確認した。

区分の凡例は決定 0121〜0152 と同じ。**実測**＝このセッションのコマンドの出力を見た事実。**読み**＝コード・文書を読んだ解釈。**未確認**。**出典なしの選択**＝`S-p2-…`(§8)。**この機械(16 コア、Linux VM、隔離 headless Chrome 156〔SwiftShader〕、Firefox 158 headless、load 20〜60 で他担当と共有)の値で、依頼者の機械(Firefox 159 / Windows 10 / GTX 980)ではない。入力はすべて CDP / G3 の模擬で、人の操作ではない。実機 Windows との照合は 0 件。**

## 冒頭の三点(正直さ)

1. **何を実測したか(実行した本物と範囲)**: 実エンジン(`FLEngine_x64.dll` と同梱 DLL を x86-64 → C → wasm にした物。既定の入口 SHIPH〔cc975e05e136〕と、ship2 の F2xH2 の objects を hardlink で借りて runtime の 2 ファイルだけ再コンパイルし web link したテストバンドル 2 本: 基準 B = HEAD の runtime〔wasm sha256 2efe3967…〕、修正 A = `present2-runtime.diff` 入り〔8020b324…〕、wasm-opt 無し〕を、(a) 隔離 Chrome 156 で、アイドル・Space 再生・ホバー・パネルのキャプションのドラッグを方式 B(既定)/ A(`?present=A`)/ B+`?dedupe=1` で測り(Worker の CPU 内訳は CDP の V8 プロファイル、スレッド別 CPU は /proc)、(b) `FLE_G3_TRACE=1` と `FLE_RASTER_TRACE` でゲストが何を呼ぶかを見て、(c) テストバンドル A/B で同じドラッグを Chrome と node(G3 スクリプト)で行いスクリーンショットを取り、(d) node で 16 スナップ(cmp-short.g3、82 行)を A/B で比べた。(e) Worker 内の微小ベンチ(JS 変換・2D putImageData・postMessage・WebGL2 texSubImage2D)を Chrome と Firefox で測った。代役・モック・画像の再生・事前生成は使っていない。
2. **結論**: (i) 依頼者の『ウインドウのドラッグ中は描画が更新されず、ドラッグ後もホバーした部品から順に更新される』の原因は、FL の**パネル(チャンネルラック = `TStepSeqForm` lid 17、親は `TWPControl` lid 16)が主窓の子窓で、キャプションを押すとゲストが SetCapture してマウス移動の度に `SetWindowPos(flags 0x14 = NOACTIVATE|NOZORDER)` で自分を動かす**のに、runtime(`fle_win.c apply_pos`)が**サイズの変わった窓の全クライアント領域しか無効化せず、移動した子窓のビットをコピーせず、露出した親・兄弟の領域も無効化しない**ことである(実測: 12 歩のドラッグ後、ゲストの hit-test は新しい位置で TNewCaption を、元の位置で TWPControl を返すのに、画面は動かない〔HEAD の runtime〕。残像が積み重なる場合もある)。(ii) Learn の SetWindowPos(SWP_NOCOPYBITS / SWP_NOREDRAW)・The WM_PAINT Message・The Update Region に沿って、子窓の SetWindowPos/MoveWindow/ShowWindow(HIDE)/DestroyWindow で『前後の可視領域の差』を無効化し、移動した子窓の有効なビットを新しい位置へコピーする修正を実装した(`fle_win.c` +約 250 行、'-' 行 1 行 = `destroy_tree(api, i);` の呼びを囲みの中へ移しただけ。`fle_wall_ia2w.c` +7 行 = ScrollWindowEx が動かした画素を `fle_raster_wrote` に知らせる)。新テスト `present2_uncover.rs`(10 件、期待値は Learn から手計算)は**修正版で 10/10 通過、HEAD の runtime で 9/10 失敗**。(iii) テストバンドルでの実機相当の確認: Chrome と node の両方で、修正 A はパネルが**動いた先に描かれ、元の位置は親の絵で埋まる**(スクリーンショットを viewed)。HEAD の B は動いた先が塗られず元の位置の絵も残る。(iv) **16 スナップ 82 行は変わる**: 窓表(位置・可視・z)の SHA は 16 本とも同一だが、`g3-progress`(擬似プリエンプションの進捗)が +15〜+244、窓 lid 14 の面 SHA が 16 本中 10 本で変わる(画素の違いは 13〜69 画素、ツールバーの `TQuickEditToolbarPanel` の右端〜メーター帯)。原因は起動中のウィザードのページ(±479 px の移動)・ナンバーバー(±1 px)の移動・非表示・破棄で 294 回の無効化が起き、WM_PAINT が増えるため(読み、`tables/unc-events-16snap.md`)。**期待表を更新するかは 未決の点**。
3. **未確認**: 依頼者の Firefox 159 での実機確認 0 件。**SHIP2 バンドルへの組み込み(wasm-opt 後)は未実施**(テストバンドルは wasm-opt 無し・非ハイブリッドの web link)。**native 71 行の 16 スナップ**と **a4 FRUITY 無効(942/942 など)・FLEngine の既定の生成 C の sha** は再実行していない(§6 に理由)。ドラッグ中の『全く更新されない』の後半(エンジンが 1 回の移動に数十〜数百 ms 使い、描画は idle でしか pump されないために更新が遅れてまとまって出る)は、この機械の負荷 50 の Chrome で観測した読みで、依頼者の Firefox では未確認。表示経路の短縮(方式 A を既定にする設計、SAB 入力リング、WebGL2 の 1 枚 canvas)は**段 2 以降**(この文書の後半に追記)。

## 1. 現状の費用(実エンジン、隔離 Chrome 156、方式 B/A、SHIPH、load 42〜53)

`tables/cost-s1.md`(全表)、`tables/profile-s2.md`(Worker の V8 プロファイル)。値は『壁 1 秒あたりの CPU 秒』で、負荷が高いので**速度は主張しない**。

| 方式 | セル | Worker | main | compositor | GPU | 窓更新/s | Worker→page MB/s |
|---|---|---|---|---|---|---|---|
| B | アイドル | 0.24 | 0.10 | 0.03 | 0.03 | 46 | 18.3 |
| B | 再生 | 0.30 | 0.14 | 0.03 | 0.06 | 60 | 83.6 |
| B | ホバー | 0.24 | 0.17 | 0.04 | 0.05 | 46 | 16.8 |
| B | ドラッグ | 0.18 | 0.13 | 0.03 | 0.03 | 41 | 2.9 |
| A | アイドル | 0.28 | 0.03 | 0.01 | 0.03 | 54 | 0 |
| A | 再生 | 0.19 | 0.03 | 0.02 | 0.04 | 36 | 0 |
| A | ホバー | 0.17 | 0.08 | 0.02 | 0.03 | 43 | 0 |
| A | ドラッグ | 0.19 | 0.06 | 0.02 | 0.03 | 68 | 0 |

読み: **方式 A は実エンジンで動き**、main の CPU を 0.10〜0.14 → 0.03 に下げる(転送 0)が、その描画(putImageData)を Worker が負うので **Worker の合計 CPU はほぼ同じ**(A の再生で putImageData 363 回・1,104 ms/10 s)。Worker の CPU の大半はエンジン本体の wasm(プロファイルでアイドル 82〜88%、再生 60〜63%)で、アプリの JS は 0.1〜2.6%、表示経路(postMessage + (program) + native)は B の再生で約 34%、アイドルで約 11%。**依頼者の機械の Worker が飽和しているかは、このビーコンからは言えない**(PM の訂正、2026-11-04 04:55: 23.7/s は ReportThrottle の上限)。

微小ベンチ(`tables/bench-chrome.json`、`bench-firefox.json`。Worker 内、BGRX 1930×1092 が wasm メモリにある想定。Chrome は SwiftShader なので GPU 側の仕事は測れない): 全面 8.4 MB で JS 変換 28 ms → 2D putImageData 0.45 ms / slice+postMessage 18 ms + ページ側 put 2.4 ms / **WebGL2 `texSubImage2D` を wasm のヒープから直接 1.4 ms**(変換・コピー無し)。400×300 で変換 0.61 ms + put 0.015 ms / postMessage 0.26 ms / texSubImage2D 0.11 ms。Firefox 158 headless には WebGL2 が無く('no webgl2 context')、2D のみ(全面 変換 10 ms、put 1.2 ms、postMessage 8.9 ms)。

## 2. 原因の特定(§冒頭 2(i) の根拠)

- 誰の仕事か: ページは canvas を動かしていない(canvas はトップレベル窓だけ)。ゲストのパネルは主窓の子窓。`FLE_G3_TRACE=1`: `TNewCaption`(lid 98)への WM_LBUTTONDOWN の後、マウスの target が `TStepSeqForm`(lid 17)で `captured 1` になり、WM_MOUSEMOVE の lParam が常に同じ(約 286,279)= 窓が動いてマウスとの相対位置が変わらない。`FLE_RASTER_TRACE` の z 行: `swp(flags=0x14, SetWindowPos) lid 17 parent_lid 16`(NOREDRAW も NOCOPYBITS も無い)。ドラッグ後の hit-test: 新しい位置(801,247)で TNewCaption、元の位置(651,97)で TWPControl。窓表(node G3): lid 17 の rect が (0,0)→(150,150)。
- runtime の欠け: `apply_pos` の末尾は `if (!(nflags & FLE_SWP_NOREDRAW) && sized && is_visible(i))` の窓の全クライアント領域だけを無効化する。**移動だけの SetWindowPos は何も無効化せず、ビットも動かさない**。ShowWindow(HIDE)・DestroyWindow も親・兄弟の露出部分を無効化しない。
- Learn: SetWindowPos「SWP_NOCOPYBITS: Discards the entire contents of the client area. If this flag is not specified, the valid contents of the client area are saved and copied back into the client area after the window is sized or repositioned.」「SWP_NOREDRAW: Does not redraw changes. If this flag is set, no repainting of any kind occurs. This applies to the client area, the nonclient area (including the title bar and scroll bars), and any part of the parent window uncovered as a result of the window being moved. When this flag is set, the application must explicitly invalidate or redraw any parts of the window and parent window that need redrawing.」(= NOREDRAW が無ければ親の露出部分は再描画される)。The WM_PAINT Message「The system sends this message to a window procedure when changes to the window have altered the content of the client area.」The Update Region「The system adds only the invalid portion of the window to the update region」。Window Features / Child Windows「The child window is responsible for painting its client area after the move.」CreateWindowExW Remarks「If the created window is a child window, its default position is at the bottom of the Z-order.」(ローカル: `docs/investigations/pin1/logs/learn/SetWindowPos.txt`、`lift/out-present2/learn/*.txt`)。
- 症状との対応(読み): 動いた先は塗られず元の位置の絵が残る → 『ドラッグ中は更新されない』『ドラッグが終わっても更新されない』。ホバーした部品(スクロールバー・ボタン)は自分で再描画するので、そこだけが新しい位置に現れる → 『ホバーした部品から順に更新される』。

## 3. 修正(`present2-runtime.diff`)

`lift/runtime/fle_win.c`: `unc_begin`/`unc_end`(新規の static、`apply_pos` の WM_WINDOWPOSCHANGING の後と末尾、ShowWindow(HIDE) の前〔WS_VISIBLE を消す前に撮る〕、DestroyWindow の前後)。手順: 変更する窓が子窓で可視なら、その窓の木で『古い矩形と新しい矩形のどちらかに触れる可視窓』(最大 160)の可視領域(`fle_win_dc_region`、CLIPCHILDREN|CLIPSIBLINGS = その窓が最上位に見える所)を変更前に取る。変更後: ① 動いた子窓(NOCOPYBITS なし、可視)は、旧可視領域(子を含む、有効な部分 = 更新矩形を除く)を移動量だけずらして新しい可視領域と交わる所へ、gather → scatter でビットをコピーし、`fle_raster_wrote` でダーティ矩形に通知。② NOREDRAW なしなら、各窓の『後の可視領域 − 前の可視領域』(動いた窓とその子孫は『前をずらしてコピーされた所』を引く)の外接矩形をその窓の更新矩形に足し、erase を立てる。トップレベル窓は対象外(各自の面をブラウザが合成)。`fle_wall_ia2w.c`: ScrollWindowEx の `memmove` の後に `fle_raster_wrote`。

テスト `present2_uncover.rs`(10 件。期待値は Learn から手計算): 重ならない移動(親の旧矩形が無効、子は無効なし、ビットのコピー、ダーティ矩形 (100,120)-(200,170))/ 重なる移動(帯 (10,20)-(40,70) だけが無効、重なりを越えたコピー)/ NOREDRAW(コピーのみ)・NOCOPYBITS(子の全体が無効、コピー無し)/ MoveWindow(bRepaint)/ ShowWindow(HIDE)・DestroyWindow / 兄弟の移動(下の兄弟 A に (40,40)-(100,100)、親に外接矩形)/ 兄弟の HWND_BOTTOM / 孫の移動(子が無効、トップレベルは無効なし)/ トップレベルの移動(何も無効にしない)/ ScrollWindowEx のダーティ通知。**修正版 10/10、HEAD の runtime 1/10(通るのはトップレベルのテストだけ)**。

## 4. 実エンジンでの確認(テストバンドル A/B、同じ操作)

- Chrome(`runs/drag-real-{A,B}/`): 12 歩のドラッグ(キャプション (651,97) から (+150,+150))。B は動いた先に何も描かれず(窓表では動いている)、A はパネルが新しい位置に描かれ元の位置が親の灰色で埋まる。`results` の `chrome-drag-B.png` / `chrome-drag-A.png`。
- node G3(`stool/rack-drag.g3`): 窓表 lid 17 = (150,150) は A・B とも同じ。最終面(lid 14)は A/B で 125,765 画素違い、B は元の位置に残り A は動いた先に描かれる(`node-rack-base.png` / `node-rack-patched.png`、左上 1930×700 を 1/2 に縮小)。
- 起動後の窓操作(F9・F6・F7・F8・Escape の連続、パネル 3 枚のドラッグ。`tools/ops.mjs`)は A/B とも停止せず(engine-idle)、最終の画面は同じ。p2 のアイドル/再生/ホバー/ドラッグ(`runs/p2-real-{A,B}`)は停止なし、Worker/main の CPU は A/B で同程度(load 33〜40: A 0.29/0.13、B 0.26/0.12 アイドル)。node の起動 16 スナップの user CPU は B 24.16 s、A 24.01 s(負荷による差の範囲)。

## 5. 16 スナップの変化(未決の点)

`tables/snap16-compare.md`、`tables/unc-events-16snap.md`。基準 B は `g3-lines-F1x-layer3-baseline.txt` と 82/82 一致(再現)。A は窓表 16 本とも同一、`g3-progress` が +15(wiz0〜a1750)〜+244、lid 14 の面 SHA が a1000〜a1750・a3000・e0300〜end で変わる。差の画素は a1000 で 13 画素(x=1042、y=12〜24 = `TQuickEditToolbarPanel` の右端列)、e0300 で 4 画素、script 終了時で 69 画素(x=1042〜1125、y=12〜42)。ウィザードの最終画面の見た目は同じ。**どちらが実機の Windows に近いかは Learn だけでは言えない**(ウィザードのページの ±479 px の移動でそのページと下の `TWPControl` が無効になるのは Learn どおりで、WM_PAINT が増えるのも自然。画素の 1 列の違いはツールバーのメーターの描画タイミングで、読み)。

## 6. 検証の状況(基準: 全体テスト 178 results / 2,274 passed / 0 failed、a4 FRUITY 無効 942/942・895/895・5,310・1,009,731、FLEngine 既定の生成 C の sha256 cd9107ce…)

- 新テスト `present2_uncover` 10/10(HEAD の runtime では 9 件失敗)。
- 全体テスト(`cargo test --offline --workspace --no-fail-fast`、`logs/fulltest-final.txt`): **179 results・2,284 passed・0 failed**(基準 178/2,274 に新しい実行ファイル `fe-lift/present2_uncover` の 1 結果・10 件が加わった値)。1 回目の実行は `wall_ia2` の 3 件が落ちた(`fle_wall_ia2w.c` に足した `fle_raster_wrote` の直接呼びが、ラスタ側を持たないテストのリンクで未定義になった)。weak 宣言と `if (fle_raster_wrote)` の確認に直して通した。
- FLEngine の既定の生成 C の sha256: **runtime の `fle_win.c`・`fle_wall_ia2w.c` と新テストだけを変え、fe-lift・fe-decode・crates の本体は触っていないので変わる理由がない**(再実行していない)。
- a4 FRUITY 無効: 同じく再実行していない。FRUITY 無効の a4 はゲストの init の途中(unit 314/942、`VirtualProtect` で停止)までで、窓の移動・非表示・破棄に達する前に止まると読むが、**確認していない**(dec1 の a4 木で runtime だけ再リンクする手はあるが、構成〔5,310/1,009,731〕が dec1 の a4-head〔4,209/852,104〕と違うので同じ基準では測れなかった)。
- native 71 行: 再実行していない(native -O0 の objects は ship2 が片付けた)。

## 7. 他の担当との衝突

`fle_win.c` は ia2 取り込み済み、crash1(Drag 系は別ファイルの見込み)、screen1(`fle_screen.h`・`fle_gdc.h`・`fle_monitor.h`)、tbl1/obs1 が触りうる。本 diff は `fle_win.c` の ①宣言の追加(`fle_visrgn_intersect`・`fle_visrgn_bounds`・`fle_raster_surface_peek` ほか 6 行)、② `apply_pos` の直前に新しい関数群、③ `apply_pos` の 2 か所(開始と末尾)、④ `fle_imp_ShowWindow` の hide の分岐、⑤ `fle_imp_DestroyWindow` の 1 呼び出しの囲み、の 5 箇所だけ。screen1 の `fle_gdc.h`/`fle_monitor.h` の変更とは別の場所。

## 8. 出典のない選択(S-p2-…、撤回条件つき)

| # | 選択 | 理由・代替 |
|---|---|---|
| S-p2-1 | 更新領域は窓ごと 1 つの外接矩形(既存の S-updregion)なので、L 字の露出部分は外接矩形で無効化する | 過剰に無効化する(正しさは保つ)。領域を持つ実装にすれば減る |
| S-p2-2 | システムが作った無効領域は erase 付き(WM_ERASEBKGND が BeginPaint から来る) | Learn は『背景を消す必要があれば』としか言わない |
| S-p2-3 | z-order の変更で露出した部分も無効化する(動かない窓の『後 − 前』) | Learn の SetWindowPos は z-order で露出した部分の無効化を明言しない(読み) |
| S-p2-4 | 1 回の変更で見る窓は最大 160(`UNC_MAX`)、領域は `FLE_VISRGN_MAX`(128 矩形)まで。超えると取りこぼし(無効化が足りない側) | 起動中の最大は 10 窓前後。超過を数える計器は未実装 |
| S-p2-5 | 有効なビット = 旧可視領域(子を含む)から窓とその子孫の更新矩形を引いた所。子孫は親と一緒に剛体として動く | 実機の Windows の WVR_ の詳細は未確認 |
| S-p2-6 | SWP_NOREDRAW でもビットはコピーする(Learn はコピーを NOCOPYBITS だけに結ぶ) | 読み |
| S-p2-7 | トップレベル窓の移動・非表示・破棄は対象外(各自の面があり、ブラウザが合成する) | 面が残るので覗いた窓の再描画は要らない。実機は WM_PAINT が来る |
| S-p2-8 | SetParent・BringWindowToTop 等、`apply_pos` を通らない z-order 変更は対象外 | 未実装(最大の穴) |

## PM の決定と取り込み(段 1: runtime の修正、2026-11-04)

取り込み: `present2-runtime.diff`(`lift/runtime/fle_win.c`〔子窓の SetWindowPos/MoveWindow/ShowWindow(HIDE)/DestroyWindow で、前後の可視領域の差を更新矩形に足し、動いた子窓の有効なビットを新しい位置へコピーしてダーティ矩形に通知〕、`fle_wall_ia2w.c`〔ScrollWindowEx が動かした画素の通知、weak〕、新規 `lift/crates/fe-lift/tests/present2_uncover.rs` 10 件。'-' 行は `fle_imp_DestroyWindow` の `destroy_tree(api, i);` を無効化の囲みの中へ移した 1 行)を HEAD 0a3ab9a に当てた(clean)。PM の測定: FLEngine の既定の生成 C の sha256 `cd9107ce…`(215,811,678 B)**不変**、全体テスト **180 results・2,287 passed・0 failed**(179・2,277 + `present2_uncover` の 1 結果・10 件)。a4 FRUITY 無効の結果は取り込みコミットに記す。実エンジンでの確認(担当、Chrome と node G3 の同じドラッグ): 基準ではパネルが動いた先に描かれず、修正版では動いた先に描かれ、元の位置が親の灰色で埋まる(画像 `evidence/chrome-drag-{B-baseline,A-patched}-runtime.png`。PM も修正版の画像を確認した)。

判断:

1. **取り込む**(依頼者の『ウインドウのドラッグ中・後に描画が更新されない』『ホバーした部品から順に更新される』の原因=動いた子窓の移動の後に親・兄弟の露出部分が無効化されず、ビットもコピーされない、の修正。Learn の SetWindowPos〔SWP_NOCOPYBITS/SWP_NOREDRAW〕、The WM_PAINT Message、The Update Region が出典)。
2. **起動の面が変わる**: 修正版の runtime で作った node の 16 スナップ(82 行)は、窓表の SHA 16 本が同一のまま、`g3-progress` 16 行と `g3-surface`(lid 14)10 行が違う(画素の差は a1000 で 13 画素、e0300 で 4 画素、script 終了時で 69 画素=`TQuickEditToolbarPanel` の右端列とメーター帯。起動中のウィザードのページ・ナンバーバーの移動と、ページの非表示・破棄で 294 回の無効化が起き WM_PAINT が増えるため)。**既存の期待表は上書きしない**。present2 の runtime を含むバンドルの期待表は `lift/harness/b2d2/expected/g3-lines-present2-runtime.txt`(新規。担当の木で作った非ハイブリッド・wasm-opt なし。ship3 で作り直して確認する)。native 71 行は未実行(次の native 構築で)。
3. **GAP-U1**(親が CLIPCHILDREN を持たないとき、無効化が可視の子に伝わる。Learn の RedrawWindow に明文)は `present2-runtime-2.diff` として別に出る予定。取り込み前に、メインフォームが CLIPCHILDREN を持たないためアイドルの 1 画素幅の再描画(決定 0187)が子の再描画を毎フレーム引き起こさないか(CPU)を測る。
4. 担当の申告(a4 と sha を再実行していない・hardlink で借りた木の誤り〔最初の『16 スナップ一致』は借り元の runtime を使っていた。やり直して『変わる』が正〕・規則違反)は記録のみ。方式 A の既定化、SAB 入力リング、WebGL2 は段 2 以降。

## PM の決定と取り込み(段 1b: GAP-U1、2026-11-04)

- **取り込み**: `present2-runtime-2.diff`(`fle_win.c` と `present2_uncover.rs`。`-` 行 25 本は『更新矩形を足す外接矩形の合併』の同じ塊 2 つを `upd_add` / `upd_invalidate` の呼びに置き換えたもので、PM が全部読んだ)と `present2-runtime-2-api2-flip.diff`(決定 0210 の `win-known-divergence.txt` から GAP-U1 の 2 行を消す)を HEAD a0d658a に当てた。PM 実測: 全体テスト 184 results / 2,314 passed / 0 failed(a4 は 942/942、895/895、関数 5,310、入口 1,009,731(不変))。
- **内容**: 窓に更新矩形を足すとき、その窓が WS_CLIPCHILDREN を持たなければ、触れる可視の子窓にも(子の座標で)同じ規則で足す(Learn の RedrawWindow と Window Features / Clipping の記述)。Wine 10.0 の答え付きのテスト(api2 の `win_upd.c`)の出力は、Wine 専用の ScrollWindowEx の 4 行を除いて全行一致。
- **既定 ON を採る**。逆スイッチは `-DFLE_NO_UPD_PROPAGATE`(ビルド時マクロ。窓表が環境を読まないという既存のテストの約束を守るため getenv は使わない)。
- **費用(担当の実測、PM は未再測定)**: ウィザードの閉じるアニメーションから Enter 後で、16 スナップの user CPU が +45〜+100%(基準 21.0〜24.2 s に対し 31.1〜44.3 s、load 17〜40 でばらつく)。起動から最初の idle まで(Chrome)と、起動後のアイドル・再生・ホバー・ドラッグは差が小さい。増加分の内訳と、文書の範囲での統合は担当に指示した(出典のない近道は禁止)。
- **新しい期待表**: `lift/harness/b2d2/expected/g3-lines-present2-u1.txt`(84 行。基準に対し g3-progress 16 / g3-surface 15 / g3-snap 5 が変わる。e0300 以降の仮想時計が 1 ms ずれる)。既存の表は上書きしない。
- **限界**: Browser パネルの黒い領域は GAP-U1 でも直らない(別の原因)。現在の既定の入口(SHIP2)の wasm にはまだ入っていない(ship3)。
