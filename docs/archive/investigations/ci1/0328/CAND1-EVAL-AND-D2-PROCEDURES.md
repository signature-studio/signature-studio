# cand1 の実エンジン評価の手順と、D2 の効果測定の手順(ci1、仮の番号 0328。docs だけ・実行 0)

## 0. 実際に実行した物(先頭)

読みだけ: `docs/investigations/mt3w/0313/tools/g9f-cand1-build.sh`(cand1 の中身)、`alpha22/docs/0319-diag-g9-fl-reading-procedure.md`(診断の読み方)、`alpha22/impl/diag-g9-fl.diff`(grep)、`lift/out-git/ci1-results/0322/LIFT-FIDELITY-AUDIT.md`・`0323/REPORT-0323.md`・`0325/REPORT-0325.md`、`app/test/e2e/desk-synth.mjs`(grep)。**実エンジン・Chrome・Wine・clang・emcc の実行は 0。測定 0。** 以下の『見込み』『期待』はすべて読みで、cand1 の結果ではない。

## 1. cand1 の中身(`g9f-cand1-build.sh` を読んだ事実)

| 層 | 内容 |
|---|---|
| 基の木 | main + `0313-g9a+filter-combined.diff`(Balance + Filter)+ `diag-g9-loadlib.diff` + `0313f2-cursor-realmod-cap.diff`(`fle_cursor.c` の RM_MAX 16) |
| 契約の差分 | `0323-contract-fixes`(①背景の消去 ②HTTRANSPARENT ③WH_MOUSE/WH_KEYBOARD)、`0319-scrollwindow-model`、`0319-time-env-run-values`、`0319-fle-stop-context`(stop の文脈 1 行)、D1(`FLE_SCREEN_REMAXIMIZE` の除去)、D2(`fle_desk_describe` の遅延化) |
| 診断 | `diag-g9-fl`・`diag-g9-fl-scroll`(`FLE_DIAG_WIN`/`FLE_DIAG_SCROLL`/`FLE_DIAG_VFS_MISS`。renamed bundle の manifest が 3 つとも 1)、`diag-g9-hook`(間接呼びの log) |
| **入っていない物** | **0325(カーソルの絵)、0323b(無効な窓への入力)**。0325 は RT-A に入る(PM)。**cand1 でカーソルの変化は評価できない** |

## 2. cand1 の実エンジン評価の手順(Chrome。gpu1 の V3 の後に ci1。PM の合図で)

各段の合否は『読み』。判断は PM。実エンジンの結果が出るまで、どの『期待』も事実ではない。

| 段 | 見る物 | 手順(既存の道具) | 合格の見込み / 外れたときの意味 |
|---|---|---|---|
| E1 起動 | first idle に達する。`fle: unsupported` 0 行、`fle: stop` 0 行 | engine log を `saveLog`(`all`)で保存、`grep -c 'fle: unsupported'`、`grep 'fle: stop'` | Filter の V3 が `DestroyCursor` で止まった(PM の info)のが直っていれば到達。同じ stop なら 0313f2 の効果なし。**新しい名前つき stop は FL のフック手続きの初回の実行(0323 ③)の疑い**: `stop context` 行(0319)の guest return の VA を見る |
| E2 Filter の読み込み | `g9-diag: LoadLibraryW "…Fruity Filter_x64.dll" -> 0x7000000`(1 回)、`GetProcAddress("CreatePlugInstance")` 解決、Problems 窓の件数 7 → 6 | V3 と同じ。期待値は 0313 §14.3 | 外れ: Filter の module 表の位置(k = 9)の問題 |
| E3 0323 ①(背景の消去) | **Balance をスロットから開いた窓の client 領域が描かれるか**(stage 3 は枠だけ)。`DefWindowProc WM_ERASEBKGND|WM_PAINT` の行(窓ごと)と、`TWPControlForm` の子窓の数(`tools/a22-diag-read.py` の `[check] TWPControlForm … child windows`) | gpu1 の Balance の窓の scenario(0313 §12)。同じ座標・同じ縮尺で stage 3 の画像(PM が確認した物)と client の矩形を並べる。画素差は `tools/pngdiff.py` | 描かれれば ① が効いた可能性(**原因の確定にはならない**: 0323 は『どのクラスが `hbrBackground` を持つか』を実エンジンで数えていない)。変わらなければ、client が描かれない原因は ① 以外(子窓が 0 のまま、`vfs-miss` の欠け、ゲストの途中失敗)で、alpha1 の読み(0319 診断の読み方 §3.1)に従って行から決める |
| E4 0323 ③(フック) | FL のフック手続きが初めて実行されたとき、名前つき stop がない。入力が落ちない | 入力を含む既存の場面(File メニュー ×100 の `scn`、`scn-f9` など〔`E2E-INVENTORY.md`〕)を cand1 に対して実行。**入力が落ちる〔100/100 でない〕**なら、フックが 1 を返して捨てている疑い(契約どおりの動きだが FL の期待とずれる) | 外れたとき、どのフック(`WH_MOUSE`/`WH_KEYBOARD`)のどの nCode かを切り分けるには、フックの呼び出しの診断が要る(**cand1 には無い**) |
| E5 0323 ②(HTTRANSPARENT) | 変化なしの確認: 15 場面の通過、File ×100 | 既存の 15 場面(`scn-*`)を cand1 で | ② は HEAD で `HTTRANSPARENT` の窓が実走行で現れたか不明(未確認)。変化は見えない見込み |
| E6 0319 の 3 本 | `ScrollWindowEx` の model(O9: Settings の General のスクロール)、時刻・Temp・DPI(O7/O8) | `FLE_DIAG_SCROLL` の行(`a22-diag-read.py` §3.2)。Settings→General をスクロールした画の前後比較。Debug タブのログの時刻(12:00:00 AM でなくなる) | 画の崩れが消えるか。時刻は `fle_wall` の page 差分が bundle に入っていなければ変わらない(page 側の差分は別) |
| E7 D1/D2 | 変化が見えないこと(D1: `FLE_SCREEN_REMAXIMIZE` は読まれていない。D2: 描画は変えない) | 画面の fit、`document.title` の追従。D2 は §3 | — |
| E8 Export | Balance + Filter 込みの Export が完走し、sha を記録。**原 FLP**(`proj`)と **alpha1 の変種**(`ins1-lp.flp`)の 2 本 | `tools/mk-variant-assets.sh BUNDLE NAME [VARIANT.flp]`(本日作成。変種を `serina1-serum2.flp` の名前で `assets-NAME/proj` に置く)+ `scn-export-g9a.json`。結果の sha256 と WAV を alpha1 に渡す(`a22-filter-compare.py` が Wine の elog16 と比べる) | 原 FLP では Filter の効果がほぼ現れない(elog16 との差が ±1 の数サンプル。PM の読み)。**変種で効果が見える**ことが Filter の忠実性の判定の入口 |
| E9 性能の非後退(参考) | first idle の中央値(Chrome 数組)を E13 と並べる | 既存の first idle の手順。**cand1 は診断入りの test bundle なので『参考』**。CI の bundle の値ではない | 予算 +100 ms は CI の bundle(RT-A など)で測る |

**`RegisterClassW` の `hbrBackground` の数え**(PM の依頼の項目): **cand1 の診断では取れない**。`diag-g9-fl.diff` には `RegisterClass` の語が無く(grep 0 件)、`diag-g9-gui` の `RegisterClassW` の wrapper は『Balance/Filter の image から来た呼び』だけ。FL 本体のクラスの `hbrBackground` を数えるには、`FLE_DIAG_WIN` に乗る**観察だけの診断**(クラス名・`style`・`hbrBackground`〔色番号か実ハンドルか〕を 1 クラスにつき 1 行、上限 400 行)を `RegisterClassW`/`RegisterClassExW` に足す次の build が要る。**未作成**(要望があれば ci1 が差分を作る。その場合も、コンパイルは PM の窓で)。

**必要な Chrome の時間**(見込み。測っていない): E1〜E7 は gpu1 の V3 と重ねず ci1 が約 30〜40 分、E8 の Export は原 FLP と変種で各 約 20 分(PM の指示)。

## 3. D2(`fle_desk_describe` の遅延化)の効果測定

D2 は『診断を製品の描画経路から切り離す』(決定 0322 の方針)ので、**採用は性能の結果に依らない**(PM の判断で採用済み)。効果の大きさだけを見積もる手順:

| 段 | 内容 | 費用 |
|---|---|---|
| S1 上限の見積り(読み。今) | UI Worker の CPU の 80% は FL 自身のゲスト関数で、runtime の GDI・合成・present は 5〜10%(決定 0282)。D2 が消すのは compose 一回ごとの『可視窓ごとの `names_of` + `build_desc` の `snprintf`』で、5〜10% の内の一部。可視の最上位窓は参照プロジェクトで数個〜十数個の見込み(**未確認**: `fle_desk_describe` の行数で数える)。1 窓あたり数 µs とすると、1 compose で数十〜百数十 µs、present 30〜60/s で 0.1〜1% の 1 コア(**推定**) | 0 |
| S2 同じ機能試験(今のe2e) | `compositor-synth` 群は `APP_REPO_ROOT/lift/runtime/fle_desk.c` を emcc でビルドして `fle_desk_describe` を読む(`desk-synth.mjs` 20〜78 行)。**D2 を載せた木を `APP_REPO_ROOT` にして `run.mjs --only compositor-synth` を実行**すれば、D2 のコンパイルと『describe が壊れていない』が分かる(今の e2e は HEAD の木なので D2 は含まない) | 約 数分(Chrome 1 本) |
| S3 ネイティブの微小測定(推奨) | `lift/harness/alpha1/glass_seq.c` と同じ作りの harness で、窓を K 個(K = 5・20・64)作り、`fle_desk_compose()` を N = 2×10^5 回呼んで ns/compose を before(D2 なし)/after(D2 あり)で測る。clang -O0 の native は wasm の約 3 倍速いので **wasm の値は 3 倍して読む(推定)** | clang -O0(nice 19、1 つずつ)で 1 分未満。PM の窓 |
| S4 全体の A/B(推奨しない) | D2 あり/なしの cand1 を 2 本 build して、純再生の Worker の CPU(6 組)を比べる。E12 対 E13 の純再生の Worker CPU は 49.42 対 49.53 s(差 0.2%、0277-e13)で区別できず、同じ bundle でも窓の静けさで書き込み率が約 2% 動いた例(0.963 対 0.985、同)があるので、S1 の見積り(< 1%)の差は A/B では**見えない見込み**(推定) | build 2 本 + Chrome 数時間。費用が見合わない |

**判断の提案(PM が決める)**: S1 + S2 で足りる。S3 は PM が窓を許可したときだけ。S4 はしない。
