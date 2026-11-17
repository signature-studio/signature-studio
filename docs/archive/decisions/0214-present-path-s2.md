# 0214 段 2: GAP-U1 の費用の内訳(PM 指示 1)、表示方式 P = プール済み OffscreenCanvas(opt-in の土台。PM 指示 2 と、その後の方針変更)、WebGL2・SAB 入力リングの判断、desk1 / win2 への引き継ぎ — 仮の決定

状態: **仮の決定(すべての選択)**。依頼は PM(決定 0214)。作業者 `present2`。段 1(`present2-runtime.diff`)と段 1b(`present2-runtime-2.diff`、GAP-U1)は HEAD に取り込み済み(`0214-present-path.md`)。この文書は、その後の指示 (1)(2) への答えと、段 2 の diff(`present2-display.diff` = 方式 P の opt-in、`present2-glprobe.diff` = WebGL2 の測定専用)。diff は HEAD **c728f44** の `git archive` に `patch -p1 --dry-run` が fuzz も offset もなく当たることを確認した。

**方針の変更(PM、依頼者の指示 `docs/policies/lift-fidelity.md`)**: 当初 PM は『P を既定にする方向』と言ったが、これは窓ごとの canvas とページ側の配置を前提にしているため**撤回**された。表示経路の本線は desk1(決定 0223: ランタイムがデスクトップを 1 枚に合成、Worker が OffscreenCanvas へ直接提示、入力は SAB リング、ページは窓の知識を持たない)に移る。この文書の P は**既定にしない opt-in の土台**で、既定の切替 diff は作っていない(作りかけは取り下げた)。点検表による自分の差分の分類は `../MERGE-NOTES.md`「lift の忠実性の点検」。

区分の凡例は 0214 本文と同じ(**実測**＝このセッションのコマンドの出力を見た事実、**読み**、**未確認**、**出典なしの選択**＝`S-p2-…`)。値は**この機械(16 コア、Linux VM、隔離 headless Chrome 156〔SwiftShader〕と Firefox 158 headless、load 17〜42 で他担当と共有)**のもので、依頼者の機械(Firefox 159 / Windows 10 / GTX 980)ではない。入力は CDP / WebDriver の模擬で、人の操作ではない。実機 Windows との照合は 0 件。

## 冒頭の三点(正直さ)

1. **何を実行したか**: 実エンジン(`FLEngine_x64.dll` と同梱 DLL を x86-64 → C → wasm にした物)のテストバンドル(ship2 の F2xH2 の objects に、HEAD の runtime 〔A = 段 1、U = 段 1 + 1b〕だけを再コンパイルして web link した物。wasm-opt なし)を、隔離 Chrome 156 と Firefox 158 headless(geckodriver、JSPI 動作)で実行して測った。Wine 10.0 は api2 の小さな C の harness(`win_paintseq.c`)の WM_PAINT 列の比較にだけ使い、**実エンジンを Wine で走らせてはいない**。代役・画像の再生・事前に生成した結果は、この文書の数字の中に**ない**。
2. **結論(測った値)**:
   (i) **GAP-U1 の追加の費用は、起動のウィザードの 2 区間(最初の idle の後の約 1.25 仮想秒と Enter の後の約 1.3 仮想秒)にだけ出る**(16 スナップの node: BeginPaint 808 → 1,083 回、面積 247 → 536 Mpx。増加の窓は `TWPControl`〔主窓の作業領域〕・`TEventEditForm`・`TFXForm` の 3 つで、ほかの 8 区間は 0.02〜0.06 Mpx で同じ)。合成の WM_PAINT 列は Wine 10.0 と **16 段中 0 段の違い**(HEAD の runtime は 11 段が違った)。(b) の統合は『文書にある範囲』には無く、入れていない。(c) の時系列: 最初の idle の後 75 s の Worker CPU/s を A と U で交互に 2 回ずつ測ると、Enter(9 s 目)の後の 10〜24 s は A 0.32・U 0.46(差 約 +0.13 × 15 s ≒ +2 CPU 秒)、**25 s 以降は A 0.265/0.254、U 0.266/0.269 で同じ**。費用は起動(Enter 後の約 25 s)だけで、定常には出ない。
   (ii) **方式 P は Chrome 156 と Firefox 158 の両方で動き、`verify`(全窓の canvas を Worker で読み戻して エンジンの面と SHA-256 比較)は全窓一致**(Firefox 158: アイドル・再生・ホバー・ドラッグの 4 点と起動直後の計 5 点で 2 窓 × 5 = 全部一致、Chrome: F キー操作・ドラッグを含む列で一致)。ページのメインスレッドの CPU は、再生で B 0.14(Chrome)/ 0.12(Firefox)に対し P・A とも 0.03(Chrome)/ 0.03(Firefox)。アイドルは B もすでに 0.03〜0.04(決定 0201 の dedupe が既定)で差が小さい。ホバー・ドラッグの main は CDP / WebDriver の入力合成の費用を含み、方式による差は小さい。表は §2。
   (iii) **WebGL2 の 1 枚 canvas と SAB 入力リングは作っていない**(§4 のデータは desk1 の設計入力として渡す。測定の条件と手順は `../tools/README.md`)。
3. **未確認**: 依頼者の Firefox 159・実機 Windows・GTX 980 での確認 0 件。Firefox 158 headless の WebGL2 は無く('no webgl2 context')、**Firefox の WebGL2 は未測定**。この機械の負荷(17〜42)で Worker の CPU・仮想時計の追従(virtual/wall)は方式の差と負荷の差を分けられなかった(§2)。a4 FRUITY 無効・native 71 行は再実行していない(`fe-lift` に触れていない)。現在の既定の入口(SHIP2 の wasm)には段 1・1b の runtime がまだ入っていない(ship3)。e2e の新しい群 `engine-present-P` だけを私のテストバンドル(U)で走らせた(§5)。

## 1. GAP-U1 の費用の内訳(PM 指示 1: 既定 ON、逆スイッチ `-DFLE_NO_UPD_PROPAGATE`)

### (a) 段別の表と WM_PAINT 数(Wine 10.0 との比較)

`tables/gap-u1-paint-cost.md`(16 スナップ node、`FLE_RASTER_TRACE` の paint 行を仮想時計のスナップで区切る。A = 段 1、U = 段 1 + 1b):

| 区間 | BeginPaint A → U | 面積 Mpx A → U | 増加の大きい窓 |
|---|---|---|---|
| ウィザード表示中(a0250〜a1000 の 4 区間) | 260 → 398 | 151.9 → 323.7 | lid 16 `TWPControl` +24.7、lid 53 `TEventEditForm` +16.7、lid 64 `TFXForm` +12.1(Mpx / 区間) |
| Enter 後(e0300〜e1300 の 2 区間) | 125 → 260 | 76.7 → 191.9 | 同じ 3 つ |
| そのほか(wiz0、a1250〜a4000、e2300、e3300) | 同じ(±2 回) | 0.02〜0.06(wiz0 は 18.1 → 20.0) | - |
| 合計(最初のスナップ以降) | **808 → 1,083(+34%)** | **247.1 → 535.8(+117%)** | |

読み: 起動中のウィザードのページ(±479 px の `SetWindowPos`)と、ページ・ナンバーバーの表示/非表示/移動で、**WS_CLIPCHILDREN を持たない親(FL の主窓)が無効化されると、親の領域に重なる大きな子窓(`TWPControl` 1546×998 など)にも更新が伝わり、全面を描き直す**。これは Learn の RedrawWindow('non-WS_CLIPCHILDREN windows are recursively … invalidated until a WS_CLIPCHILDREN window is encountered')と Wine 10.0 が示す Windows の振る舞いそのもの。

WM_PAINT 数の Wine との比較は、合成の 16 段の harness(`tools/win_paintseq.c`、親が WS_CLIPCHILDREN の有無 × InvalidateRect 全体・矩形・1×150 の帯 5 回・子の InvalidateRect・子の移動と戻し・HIDE・Destroy)で、窓と更新矩形つきの WM_PAINT の列が **diff 1 + 2 で 0/16 段違い、HEAD の runtime は 11/16 段違い**(`tables/paintseq-vs-wine.txt`)。**実エンジンを Wine で走らせた WM_PAINT 数との比較はしていない**(Wine 上で FL を動かせない)。

### (b) 二重に描かれる画素の統合

**『文書にある範囲』に、これ以上の統合は無かった。入れていない。** 理由(読み): ① 窓ごとの更新矩形は既に 1 つの外接矩形で、同じ窓への複数回の無効化は 1 回の WM_PAINT にまとまる(S-updregion)。② 親と子が同じ画素を 2 度塗るのは、**親が WS_CLIPCHILDREN を持たないときの Windows の振る舞い**(Learn の Window Features / Clipping と Wine の列: 親と子の両方に WM_PAINT)で、描画の省略は文書に書かれていない。省くなら『子が全面を不透明に塗る』『親の描画が子に隠れる部分に副作用がない』を前提にした出典のない近道(S-p2-n を増やす)になるので、やっていない。

### (c) 起動だけか(時系列)

- 起動から最初の idle まで(Chrome、Worker CPU 秒): A 8.0/8.8/9.7、U 8.6/8.5/9.0、HEAD の runtime B 14.6/9.1/9.4(B の 14.6 は最初の idle が 35.5 s に遅れた 1 回目、load 31)(`tables/startup-first-idle-cpu.json`、3 回ずつ、load 27〜32)。最初の idle の時刻は A 18.1/19.3/22.7 s、U 16.7/25.4/19.6 s、B 35.5/20.0/20.6 s(ばらつきが差より大きい)。
- 最初の idle の後の Worker CPU/s(Chrome、A と U を交互に 2 回ずつ、`tables/series75.json`、`tools/series.mjs --secs 75`、load 29〜41、Enter は 9 s 目。平均):

| 区間(最初の idle から) | A(段 1) | U(段 1 + 1b) |
|---|---|---|
| 1〜9 s(ウィザード表示中) | 0.61 | 0.67 |
| 10〜24 s(Enter の後、ウィザードが閉じる) | 0.32 | **0.46** |
| 25〜45 s | 0.27 | 0.27 |
| 46〜75 s | 0.25 | 0.27 |

  = **GAP-U1 の追加の Worker CPU は、Enter の後の約 15 s に 約 +2 CPU 秒だけ。25 s 以降の定常は同じ**(以前の 24 s の測定 `tables/series-worker-cpu.json` は定常に届かず、U が長引くように見えた〔読み〕が、75 s で確認した)。
- 16 スナップの node の user CPU: 基準 21〜24 s、段 1 22〜24 s、段 1 + 1b 31〜44 s(4 回、load 17〜40)。

## 2. 方式 P(`?present=P`): プール済み OffscreenCanvas

**設計**(`present2-display.diff`): ページが起動前に 24 枚の canvas を作って Worker へ transfer しておく(`engine/canvas-pool.ts`)。Worker は新しい窓にプールの canvas を**自分で**割り当て、その番号を `SurfaceInfo.pool` でページへ一方向に知らせる(ページは該当 canvas を DOM に入れるだけ)。窓が消えれば canvas はプールへ戻る。24 枚を使い切った窓は方式 A と同じ『bind の往復』に落ちる(`out.pool.overflow` で数える)。方式 A の問題(Worker が event loop に戻らないと bind が届かず、起動中に出た窓が idle まで描かれない)は、transfer を起動前に済ませるので起きない。ほかに: `SurfaceSync.gone`、bind を post の前に行う順序、Worker のコマンド `verify`(全窓の canvas を読み戻して SHA-256 を面と比較)、`app.verifyCanvases()`、idle メッセージの `out {drawn, pool}` とビーコンの `engine.drawn` / `engine.pool`、計測専用の `?glprobe=1|2`(§4)。

**ビーコン(指示 2(c))**: `engine.present`(`B`/`A`/`P`)、`engine.updatesObservable`、`engine.drawn`(Worker が描いた矩形数)、`engine.pool.{used,capacity,overflow}`(プールの使用数と、方式 A へのフォールバックの数)。`perf-beacon.test.ts` に P のビーコンのテストを追加。

**同一性(指示 2(a))**: Firefox 158 headless の `verify`: A・P とも 起動直後・アイドル・再生・ホバー・ドラッグの 5 点で 2 窓 × 全部一致(`logs/m3-ff-{A,P}.log`)。さらに最終ビルドの P で、F キー(F9 ×2・F6 ×2・F7・Esc・F8・Esc)の各段で一致し、F8 でレイヤー窓 lid 150 が増えて 3 窓、プール使用 2 → 3、オーバーフロー 0(`logs/final-ff.log`)。Chrome の最終ビルドの P: ウィザード表示中・閉じた後・F9 ×2・F6 ×2・F7・Esc・F8・Esc・F10(設定窓 lid 22)・Esc・ドラッグ 3 種の 15 点すべてで全窓一致(canvas を持つ窓は 2〜3、skipped はウィザード表示中のレイヤー窓 1 つだけ。`logs/final-chrome.log`)。ページのスクリーンショット(Firefox、P、ドラッグ後)にはエンジンの画面が描かれ、移動したチャンネルラックも正しい位置にある(`evidence/firefox-P-after-drag.png`)。

**main スレッドの CPU の表(指示 2(b))**: 全表は `tables/main-cpu-B-A-P.md`(`tools/cpu-table.py`、生の結果は `tables/m3/`、`tables/m4/`)。値は壁 1 秒あたりの CPU 秒。『速度』ではない(16 コアに load 17〜42)。

| ブラウザ | 方式 | アイドル main | 再生 main | ホバー main | ドラッグ main | 再生 Worker |
|---|---|---|---|---|---|---|
| Chrome 156 | B | 0.039 | 0.144 | 0.136 | 0.098 | 0.438 |
| Chrome 156 | A | 0.029 | 0.032 | 0.104 | 0.061 | 0.460 |
| Chrome 156 | P | 0.038 | 0.030 | 0.099 | 0.072 | 0.348 |
| Firefox 158 | B | 0.026 | 0.122 | 0.219 | 0.135 | 0.472 |
| Firefox 158 | A | 0.025 | 0.027 | 0.203 | 0.105 | 0.531 |
| Firefox 158 | P | 0.023 | 0.026 | 0.207 | 0.117 | 0.519 |

**同じ操作を 3 回交互に繰り返した(Chrome、アイドル 30 s と再生 30 s、P A B P A B P A B の順、`tables/m4/`)**。負荷の影響を方式の差から分けるため。中央値(3 回の値は `tables/main-cpu-B-A-P.md`):

| 方式 | アイドル main | 再生 main | 再生 Worker | 再生 virtual/wall(3 回) | 再生 load(3 回) |
|---|---|---|---|---|---|
| B | 0.039 | 0.142 | 0.404 | 0.871 / 1.002 / 1.001 | 28 / 24 / 14 |
| A | 0.028 | 0.031 | 0.413 | 0.865 / 1.002 / 1.001 | 42 / 29 / 21 |
| P | 0.029 | 0.031 | 0.432 | 0.777 / 0.989 / 0.989 | 40 / 34 / 20 |

- main は 3 回とも同じ向き: 再生で B 0.112〜0.144、A 0.030〜0.031、P 0.031〜0.032。アイドルは B 0.036〜0.039、A 0.027〜0.032、P 0.028〜0.029。
- Worker の CPU は 3 方式で同じ桁(再生 0.35〜0.45)。1 回目の組(load 28〜42)は Worker が追従できず virtual/wall が 0.78〜0.87 に落ちたが **A も B も同じ**(負荷の影響)。load 14〜34 の 2・3 回目は A・B が 1.00、**P が 0.989(2 回とも)**。P の方が約 1% 低い。この差が本物か負荷のばらつきかは、この回数では言えない(P3 への影響は未確認)。
- Worker の putImageData(1 回あたりの壁時間、再生): A 1.33 / 1.06 / 0.40 ms、P 1.57 / 1.01 / 0.80 ms(負荷が高い順)。組ごとに P ≧ A で、load 20 では P が 2 倍(0.80 対 0.40)。原因は調べていない(**未確認**)。合計では P の再生 30 s の putImageData は 1.3〜2.1 秒(0.04〜0.07 コア)。

読み: **再生で、A・P は main の CPU を B の約 1/5(Chrome 0.144 → 0.03、Firefox 0.122 → 0.026)に下げる**。アイドルは B が既に低く(dedupe 既定 ON)、差は 0.01 程度。ホバー・ドラッグの main の 0.10〜0.2 は、入力の合成(CDP / WebDriver)・ページの窓一覧の更新・入力送出の費用を含む(読み。内訳は測っていない)ので、方式の差は小さい(ホバーの Firefox は B 0.219、A 0.203、P 0.207)。Worker の CPU は A・P が putImageData を負うので B と同じ桁で、**負荷が高い状態では方式の差と分けられなかった**。

**既定にはしない**: 実エンジンの既定は従来どおり B(代役は A)。`?present=P` を付けたときだけ P になる(OffscreenCanvas の無い環境は従来どおり B にフォールバック)。P の通信の削減(ピクセルをメインに送らない・bind の往復なし)は lift の方針の『ピクセルのバケツリレーを避ける』に合うが、窓ごとの canvas とページ側の配置という方針が除去を求める形を前提にしているので、desk1 の 1 枚のデスクトップ面ができるまでの土台に留める。

## 3. 窓のドラッグ(段 1・1b)の現在の確認

実エンジン(Chrome、U = 段 1 + 1b)で、チャンネルラックのキャプションを押してドラッグした結果は、パネルが動いた先に描かれ、元の位置が親の灰色で埋まる(`evidence/chrome-drag-U-with-gap-u1.png`、node の `node-rack-u1.png`)。FL の Browser パネルの左の黒い領域は別の原因で、直っていない。

## 4. WebGL2 の 1 枚 canvas と SAB 入力リング: 作っていない(データによる判断)

- **WebGL2**(desk1 の設計入力。測定専用の `present2-glprobe.diff`): `tables/glprobe.md`(計測専用の `?glprobe=1|2`、Chrome の SwiftShader)。実エンジンの矩形の GPU 側のアップロード(`texSubImage2D` を wasm のヒープから直接)だけなら、Worker 上で 2D の『変換 + putImageData』の 2.6〜5 倍安い(アイドル 2.8×、再生 2.6×、ホバー 4.9×)が、**四角形を 1 枚描くとソフトウェア GL を待って 0.29〜2.8 倍になる**(SwiftShader の速さで、GPU の無いこの機械では GPU 側は測れない)。Firefox 158 headless には WebGL2 が無く表示にも届かないため **Firefox は未測定**。さらに、P では Worker が窓の描画に使う CPU は小さい(再生 30 s の Worker の putImageData の時間は A で 638 ms〔load 18〕、P で 2,078 ms〔load 28〕= 0.02〜0.07 コア〔壁の時間なので負荷で伸びる〕。Worker の CPU は wasm が 60〜88%、表示経路は 10〜35%〔`tables/profile-s2.md`〕)ので、置き換えても伸びしろは小さい。1 枚 canvas にする設計変更(窓ごとの canvas、DOM の z-order、dedupe、verify を捨てる)に見合う根拠を示せなかった。依頼者の GPU(GTX 980)で GPU アップロードが安いかは**未確認**。
- **SAB 入力リング**: 入力の postMessage が Worker に届くのは Worker の event loop が回る時で、エンジンの idle は壁で 1,050/s(アイドル)・2,000/s(再生)(`engine idles/s`、`tables/main-cpu-B-A-P.md` の元データ)。idle の平均間隔は約 1 ms(再生で約 0.5 ms)なので、postMessage の入力が Worker に届くまで待つのは長くて 1 ms 程度(読み)。エンジンがその入力を読むのも idle なので、SAB リングで縮められるのは 1 ms 未満。エンジンが 1 歩に数十 ms 使うとき(起動中など)は、リングがあってもエンジンが読まない。作っていない。

## 5. 検証の状況

- **段 2 の display diff**(HEAD c728f44 + `present2-display.diff`): 型検査 rc=0(`npm run typecheck`)、vitest **1,442 passed + 7 skipped**(HEAD は 1,437 + 7、追加は canvas-pool 4 件とビーコン 1 件)。
- **e2e(新しい群 `engine-present-P`、テストバンドル U = HEAD の runtime、preview)**: `?present=P` で P が選ばれる・プールが窓を担当しオーバーフロー 0・全窓 `verify` 一致・ビーコンの `engine.pool`・canvas が transferred(`getContext` が失敗)・console エラーなし・ネットワークなし = **8/8**(`logs/e2e-p.log`)。他の実エンジンの群(`engine-desktop` など)は既定が B のままなので今回は走らせていない(取り下げた既定切替のテストでは、`engine-desktop` が page の canvas を `getContext('2d')` で読むため P で落ちること、その wizard の SHA の検査が私のテストバンドルでは基準でも落ちることを見た。バンドルの違いで、P の違いではない)。
- 全体テスト(`cargo test`)・a4・FLEngine の sha: この段の diff はアプリ(`app/`)だけで `lift/` を触らない。PM が段 1b を取り込んだ時の値(全体テスト 184/2,314/0、a4 942/942・895/895・5,310・1,009,731、FLEngine の既定の生成 C sha256 cd9107ce…)が基準。私は再実行していない。

## 6. 他の担当との衝突

`app/src/main.ts`(opfs1 のファイルダイアログ、aud3 の音、dd2 の dedupe の行の近く: 先頭の URL パラメータの説明と、`present` を決める 1 か所、`client.idle` の `m.out`)、`app/src/perf/beacon.ts`(crash1/aud3 のビーコンの版)、`app/src/engine/worker.ts`・`client.ts`・`protocol.ts`(dd2/draw1 の変更の近く)。HEAD c728f44 では全部 clean に当たる。desk1 が `worker.ts` / `desktop.ts` を作り直すなら、この差分はその前に取り込むか、参照だけにする(MERGE-NOTES)。

## 7. 出典のない選択(S-p2-…、段 1 の S-p2-1〜8 に続く。撤回条件つき)

| # | 選択 | 理由・代替 |
|---|---|---|
| S-p2-9 | 動いた子窓(ビットをコピー済み)の部分木には、その移動の無効化を伝えない(親だけが WM_PAINT) | Learn は触れない。Wine 10.0 の WM_PAINT 列(子・孫は無し)に合わせた。違いが見つかれば `upd_skip` を外す |
| S-p2-10 | 描画面がまだ無い窓では、動いた子の有効領域は『コピー済み』とみなす | 面があって書き込めない形式のときだけ『コピーなし = 全体が無効』 |
| S-p2-11 | プールの canvas は 24 枚 | 起動時の窓は 10 前後、メニューやプラグイン一覧で数枚増える程度の測定から。超えた分は方式 A の往復に落ちる(`overflow` で数える)。1 窓 1 枚の canvas のメモリは窓の大きさ次第 |
| S-p2-12 | (欠番: 既定の切替は取り下げた) | |
| S-p2-13 | `verify` は canvas 全体の SHA-256 を Worker で読み戻して面と比べる(Worker が覗く面の形式だけ) | 画面に出る画素の比較ではない(合成後のスクリーンショットは Firefox で別に撮った) |
| S-p2-14 | CPU の測定は 30 s のセル(最初の idle の後 6 s 待つ)、スレッド別の CPU は /proc(Chrome)と WebDriver の Firefox のプロセス別集計 | 負荷が高いので速度は主張しない |

## 8. 未確認と限界

- 依頼者の Firefox 159・Windows 10・GTX 980 での確認 0 件。人の操作ではなく CDP / WebDriver の合成入力。
- Firefox の WebGL2 は未測定。
- 方式 P の長時間(数十分)の安定性、メモリの増減は測っていない(30 s のセルのみ)。
- この機械の負荷(17〜42)では、Worker の CPU と仮想時計の追従(virtual/wall)に方式の差を見出せなかった。P3(再生で音が途切れない)に対する P の影響は**未確認**。
- 現在の既定の入口の wasm(SHIP2)には段 1・1b の runtime がまだ無い(ship3 で確認)。
- 長時間(数十分)の安定性は測っていない(`tools/soak.mjs` を用意したが走らせていない)。dedupe(決定 0187 / 0201)を外したときの P の費用も測っていない。

---

## PM の決定(2026-11-04)

- **方式 P(`present2-display.diff`)は main に入れない。** 窓ごとの canvas とページ側の配置が前提で、lift の忠実性の方針(`docs/policies/lift-fidelity.md`)に照らすと、ページが窓の知識を持つ設計そのもの。表示経路の本線は desk1(決定 0223)に移した。差分は `lift/out-git/present2-results/present2-display.diff` に残し、desk1 が土台にしてよい。測定(再生中のメインスレッド CPU: B 0.12〜0.14、A/P 約 0.03)は desk1 の物差しとして使う。
- **GAP-U1(段 1b)は既定 ON のまま**: 費用は起動のウィザード 2 区間だけ(担当の実測)。
- **点検表の結果(担当の点検を受領)**: 余計の候補 = ページ側の画素重複排除 `?dedupe`、`report-throttle`、窓ごとの canvas の配置、主窓の選択(クラス名)。desk1 で抹消の対象。win2 へは『前面化・閉じる・配置の不具合は、窓を操作する API が契約どおりの無効化・WM_PAINT を出していない型で出る見込み。`unc_*` の枠を広げるのが筋。抜け: SetParent / BringWindowToTop など apply_pos を通らない z-order 変更、トップレベル窓の移動・非表示・破棄、更新領域が窓ごとに 1 つの外接矩形、ScrollWindowEx』を引き継いだ。
- **未確認**: 依頼者の Firefox 159・GPU、Firefox の WebGL2、P の長時間安定性と P3 への影響。
