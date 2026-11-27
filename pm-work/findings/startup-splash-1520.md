# 起動の順序とネイティブのスプラッシュ(Issue #1520。2026-11-27)

- 範囲: `app/`(モーダル・描画の経路)、`tools/e2e/`(判定の E2E・`--only` の経路・`network-slow-start` の進捗の見方)。`lift/` は変えていない(スプラッシュの描画は、既に lift されたエンジンのまま走る)。
- 区分: 「実測」は本作業の runner での観測(この機械とは別枠の値)。「確認できない」は、読みにも記録にも無いもの。

## 1. モーダル(実測)

- 起動の順序: 読み込みの直後に `app/index.html` の `#loading-modal`(静的な HTML。画面の中央に固定。進捗の行 4 件を最初から持つ)。ダウンロードの 4 項目が `role="progressbar"` で 0 から動く(`app/src/ui/progress.ts`・`app/src/ui/loading-modal.ts`)。
- 消す時刻: ダウンロードの完了後、2 フレーム描いてから `client.init` の前に DOM から取り除く(`app/src/main.ts`。`modal_removed` の計時)。エラーの時も消す(`showError`)。
- 実測(runner の値。`page/startup-sequence` の (a)〜(c)):

| 表示 | モーダル | 中央からの距離 | 消した時刻 | `engine_start` | `first_pixel` |
|---|---|---|---|---|---|
| 4k-dpr1 | 460 x 212 at (1690,974) | 0 px | 847 ms | 925 ms | 2592 ms |
| 4k-dpr2 | 460 x 212 at (730,434) | 0 px | 779 ms | 835 ms | 2485 ms |
| 4k-random (DPR 1.58) | 460 x 212(位置は記録していない。(a) は通過) | 判定内 | 783 ms | 841 ms | 2508 ms |
| 4k-random (DPR 1.7) | 460 x 212 at (900,530) | 0 px | 841 ms | 906 ms | 2547 ms |

- 進捗の推移(DPR 1 の記録): 33 ms に 4 行とも 0、114 ms に engine glue が 1、279 ms に DLL pack、362 ms に seed pack、842 ms に engine wasm が 1 で、全て 100 %。手元の runner では資産が OPFS の複製から来る(`storage-cache hit`)ので、途中の値は短い。
- 最初の描画の黒: モーダルの外の画素の 100 %(DPR 1・2・乱数の全てで)。
- 単体試験: `app/test/unit/loading-modal.test.ts`(進捗の値 0〜1、行の用意、`close()` の後に何も残らないこと)。`npm test --prefix app` は 1491 件が通る。

## 2. スプラッシュ

### 2-1. 読んだ範囲

- DLL の文字列(`lift/out-app/host/binaries/win64/FLEngine_x64.dll`): `SplashBox`(ASCII と UTF-16)、`NOSPLASH`・`SplashImage`・`SplashAnim`(UTF-16 のみ)。
- 抑止の有無: 起動の引数は `lift/harness/web/pack.args` の `--args` に DLL の列だけで、`NOSPLASH` は無い。`GetCommandLineW` は仮想の DLL パス 1 つを返す(`lift/runtime/fle_env.c` の `fle_imp_GetCommandLineW`。引数無し)。レジストリの seed(`lift/harness/seeds/reg-seed-*.tsv`)に `SplashBox` は無い(`git grep` 0 件)。よって、読んだ範囲では抑止は無く、既定で表示される。harness も抑止していない。
- 描画の関数: `lift/crates/fe-lift/src/imports_table.rs:763` の注釈では、`UpdateLayeredWindow`・`UpdateLayeredWindowIndirect` は `f_71e570`(`TLightweightLayeredControl`)から呼ばれる。**スプラッシュの描画の関数だと確かめたのは、注釈と、窓の記録・絵(2-2・2-4)による。逆アセンブルでは確かめていない**。
- 画像: seed に `Artwork\Wallpapers\Splash.png`(800 x 400、パレット)と `Artwork\Themes\DEDZII\dedzii_splash.png`(20000 x 569)がある。窓の 334 x 476 とは一致しない(画像そのものではなく、窓の中に描かれた絵)。ページは画像を読まない(`git grep -i splash -- app/` は 0 件。`<img>`・`background-image`・data URL も 0 件、E2E の (e))。

### 2-2. エンジンの記録(実測。DPR 1 の run)

- 窓の表: 最初の可視の最上位窓は lid 8、class `TLightweightLayeredControl`(ログの 1256 行の後の表)。最初の idle は 4400 行の後。
- 描画の呼び出し: ログ 1265 行 `[err] first non-background pixel on a drawing surface (decision 0126): written by UpdateLayeredWindowIndirect into the surface of top-level window lid 8`。
- 描画の呼び出しは、最初の idle より前に、エンジン内で行われる。

### 2-3. 以前、canvas に出なかった理由(前回の読みと実測)

- 画面の合成は、描画の担当(render Worker)の問い(`want`)に、エンジンの Worker が答えるときだけ行われた(`app/src/engine/worker.ts` の `onRender`)。その問いは Worker のメッセージとして届き、Worker の事象の処理は、エンジンが idle になってから動く。長い実行の間(起動)は、問いに答えられない。
- エンジンの入力の読み取り(`inputPoll`。スケジューラの切り替え点から呼ばれる)は、ドライバができる(最初の idle)まで戻っていた。よって、起動の間は、どの経路でも合成されなかった。
- エンジンのログも、Worker のタイマーで一括して送られる。長い実行の間は届かず、idle で届く。

### 2-4. 直した経路(lift は変えていない)

- `app/src/engine/worker.ts` の `deskPoll`: スケジューラの切り替え点(`inputPoll`)から、最大 50 ms に 1 回、画面を合成して提示を依頼する。エンジンが idle になってから 50 ms 以内は行わない(idle の後は、render Worker の問いが答える)。`sendDeskReady` は 1 回だけ送る(`deskReady` で守る)。
- 合成の前提: render Worker が提示の器(`presenter`)を作ったあと、共有の制御語(`app/src/engine/desk-link.ts` の `MODE`)に、その種類(画像 1 / WebGL2 の場面 2)を書く。エンジンは、その語が 0 でない間だけ合成する(最初の合成は画面全体を報告するので、器が無い間に合成すると、その絵は失われる)。
- 場面モード(`fle_desk_scene_mode`)は、WebGL2 の器の時に、エンジン側で直接入れる(`sceneOn`)。メッセージで入れると、エンジンの idle まで届かず、最初の合成が場面の形にならないため。
- 提示の経路の選び方: 描画の担当(render Worker)が提示の仕組みを持つので、その問いの入口を、エンジンの切り替え点にも作った。ページがスプラッシュの画像を描く経路は作っていない。

### 2-5. 受け入れの実測(runner の値。この機械とは別枠)

- **赤(直す前。HEAD の `app/` と `tools/` を `.tmp-head/` に展開し、同じシナリオを実行。DPR 1)**: `page/startup-sequence (4k-dpr1, DPR 1): ok false, engine engine-idle, errors 0/0/0, stuck 0, 19151 ms`。判定の内訳は `a: false, b: false, c: false, d: true, e: true, f: false`。(a) no modal in the page。(b) bars 0 %、bar moved above 0 % は false。(c) modal removed at never。(f) 0 of 2478 points(0.0 %)、first_pixel はない(idle の後)。(d)(e) は HEAD でも通る(エンジンは描いているが、画面に出ていない)。
- **直した後**(同じ scenario):
  - 4k-dpr1: `ok true`、19704 ms。(f) 863 / 2478 点(34.8 %)、first_pixel 2592 ms(idle より前)。
  - 4k-dpr2: `ok true`、18826 ms。(f) 2520 / 2520 点(100 %)、first_pixel 2485 ms。
  - 4k-random: DPR 1.58 で `ok true`、19012 ms(f) 862 / 2520(34.2 %)。DPR 1.7 で `ok true`、19386 ms(f) 2520 / 2520(100 %)。
- **目視**(`.tmp-head` で、同じ scenario の一時のコピーに保存先を指定して 1 回。DPR 1): 最初の idle の前の絵(`pre-idle.png`)では、画面の中央に FL のロゴ(オレンジの果物と葉)が、FL の主の窓(Playlist・Mixer・Channel rack。起動の途中で既に描かれている)の上に出ている。ロゴの位置は、判定の窓の矩形(CSS 1754,843 から 2088,1319 / 2 倍の縮小で中央)と一致する。
- **注意**: (f) の「黒でない画素」は、窓の矩形の中の FL の主の窓の画素も数える。よって、(f) の数値だけでは、スプラッシュ単独の証拠にならない。目視の絵が、スプラッシュが canvas に出ていることの根拠になる。
- 黒の基準: 3 バイトが全て 16 以下を黒とする。デスクトップの地色は (5, 7, 10)。

### 2-6. 追加の確認(実測)

- `delivery/network-slow-start`(この作業で、進捗の行の数の見方を変えた): `ok true`、30 s、進捗の行 4(待ちの間に、モーダルの行が見えた)。
- 起動の確認: `boot.json` は engine-idle、`timelineIdle` 8844 ms(runner の値)。
- 流れ A〜D: 4 行とも `error` null・engine-idle・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true。B と D は `welcomeClosed` true。

## 3. 確認できない点

- FL のスプラッシュの描画の関数の番地(2-1)。注釈と記録で、窓と呼び出しは確かめたが、逆アセンブルでは確かめていない。
- `f_71e570` が、スプラッシュ専用の描画か、他のレイヤー窓も兼ねるか(lift の注釈だけで読んだ。他の呼び出しは未確認)。
- Song Mode の音への影響(P3)。idle の無い長い実行の間は、合成が 50 ms に 1 回増えるため、その分の負荷を測っていない。
- 窓の前後の順序(スプラッシュと FL の主の窓のどちらが先か)は、絵の 1 枚では決められない(ログは、スプラッシュの窓が最初の可視の窓だと示す)。
- 実際のネットワークの速さでのモーダルの推移(この機械では資産が OPFS の複製から来るので、途中の値は短い)。

## 4. 受け入れのコマンドの判定式(記録)

- 判定の `L.length>=3`(乱数の DPR の行が 2 件出るため、行は 4 件になる)。
- `--only` の経路: `--only` の ID が領域の形(`<領域>/<名前>`)で、`--scenarios` が無いときは、その領域の全件を読む(`tools/e2e/run.mjs` の `loadSet` と `tools/e2e/lib/plan.mjs` の `areasOfOnly`。本作業の衝突は、main の `areasOfOnly` を採った)。
