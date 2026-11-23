---
draft: "107"
title: "Windows と同じ Fractional Scaling の実装を入れる: 物理画素の画面と DPI を、DPR から式で導く(測定・停止の解消・Browser は次の Issue)"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["hidpi", "fl-exe"]
size: XL
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の是正(2026-11-23)。要点は次のとおり。

- HiDPI の方針が完全に間違っている。物理画素に 1:1 で結びついており、全ての GUI が操作できないほど小さい。
- 描画の崩壊も起きている。
- 依頼者の追加の指示: 拡大されてぼやけた画面は即座に棄却する。Windows と同じ Fractional Scaling を正確に再現すること。
- 依頼者の追加の指示: FL のネイティブでは、Main GUI Scaling の System が OS の拡大率に自動で従う。今の版では従っていない。DPI・拡大率に関するメトリクス(FL 自身の Debug ログを含む)を、きちんと取ること。
- 依頼者の追加の指示: 特定の拡大率の数字を、予め用意しない。拡大率は、環境(DPR)から、Windows と同じ計算で導く。

この Issue の目標は、guest の画面を物理画素にし、拡大率と DPI を DPR から式で導いて FL に渡す土台を入れること。FL が UI を自分で拡大して描いていることの実測は `#{{113}}`、DPI 240 以上の停止は `#{{114}}`、Browser のパネルは `#{{115}}` が扱う。

1. 画面は物理画素の大きさで見える。guest の 1 画素は、表示の 1 物理画素にそのまま対応する(再標本化しない)。
2. 拡大率は、ページが起動時に engine へ渡す DPR(実数)から、Windows と同じ計算で導く。拡大率(%) = round(DPR × 100)。DPI = MulDiv(96, 拡大率(%), 100)。
3. システムは拡大も縮小もしない。アプリが自分で、部品・フォント・余白を拡大率に合わせて描く。
4. 結果として、UI の大きさは拡大率 100% の時と同じ見かけの大きさになり、かつ鮮明になる。

禁止: 論理の小さい画面を作り、DPR 倍に拡大して表示する実装(ぼやけた画面)は、中間の版としても作らない。この方式の成果物は依頼者が棄却する。受け入れが通らない時は、判定の段で止まり、push されない。通らなかった理由と到達点を報告に書く。

## 前回の試行で分かったこと

直前の試行は 3 回目(run 45089194964。約 4 時間 35 分)。受け入れに届かず `failed` になった。作業は保存されている(artifact `agent-1347` の `work.diff`。約 79 KB)。次の run は、この差分を作業ツリーに載せ、`.prior-work.md` で続きから行うよう指示する。判定を通らなかったので、main には入っていない。最初から作り直さない。

測定(前回の差分。runner の値。nproc 4。guest は engine の記録の SM_CXSCREEN・SM_CYSCREEN):

| DPR | 状態 | guest(物理画素) | LOGPIXELSY | canvas = display | blackShare |
|---|---|---|---|---|---|
| 1 | engine-idle | 1836×1056 | 96 | 一致 | 0.910 |
| 1.25 | engine-idle | 2295×1320 | 120 | 一致 | 0.912 |
| 1.5 | engine-idle | 2754×1584 | 144 | 一致 | 0.912 |
| 1.7647 | engine-idle | 3240×1864 | 169 | 一致 | 0.905 |
| 2 | engine-idle | 3672×2112 | 192 | 一致 | 0.914 |
| 1.77(乱数) | engine-idle | 3250×1869 | 170 | 一致 | 0.903 |
| 2.5 | 停止 | — | — | — | — |
| 2.71(乱数) | 停止 | — | (計算上 260) | — | — |

- DPR 2.09(DPI 201)と 2.25(DPI 216)は engine-idle に届いた(guest の値は記録していない)。
- canvas と表示は、表の DPR で物理画素が一致した。拡大して表示する版は入れていない。
- `blackShare` は DPR 1 でも 0.903〜0.914 で、DPR によらず同じ。Browser の領域は、DPI の変更の前から黒い可能性がある(`#{{115}}` が扱う)。矩形の定義も未確認。
- マニフェスト(`lift/out-app/host-extra/exe/`): `FL64.exe` は `dpiAware True/PM`(per-monitor)。`FL64 (scaled).exe` は DPI の宣言が無く、Windows が拡大して表示する起動用と読める(推定)。両方のサイズは同じで、ハッシュは違う。前回の差分は、`GetProcessDpiAwareness` の応答 2 を、この宣言から決めている。読みの道具は `tools/feat-c1-hidpi/read-manifest.mjs`(前回の差分が追加。製品には入れない)。
- guest が読む DPI は LOGPIXELSY(90) だけで、LOGPIXELSX(88) は読まれない。`GetDpiForMonitor` も同じ値を返す。カーソル・小アイコンの寸法は MulDiv どおり(DPR 2.25 で SM_CXCURSOR 72・SM_CXSMICON 36。DPR 2.5 で 80・40)。
- **停止(DPI 240 以上。DPR 2.5 以上)**: `[js] fle thread 2 failed: RangeError: Maximum call stack size exceeded`。乱数 2.71(DPI 260)でも同じ。
  - 原因は LOGPIXELSY の値 240。実験で、LOGPIXELSY だけを 96 に戻すと DPR 2.5 が idle に届く。
  - 原因でないと確認したもの: フォントの高さ単独(`FLE_SPI_FONT_HEIGHT`)、`GetDpiForMonitor` 単独、物理サイズの仮説(機器の密度から求めても止まる)、画面の大きさ(部屋 1000×600・画面 2500×1500 でも DPI 240 で止まる)。
  - 停止は窓の作成(`CreateWindowExW`、style `0x…24`)の直後。停止時のスタックに、同じ 12 個の戻り番地の並びが繰り返し、自己呼び出しを含む。
  - 閉路の入口と、どの runtime の応答が閉路を導くかは未特定。関数名への対応も未確定(関数名の表は `#{{040}}` が出す)。
- 未達(前回の試行で取れなかった物): F10 の Display の節・FL が採用した拡大率・Debug ログ(窓の操作と、窓の文字を読む経路が無い)・Main GUI Scaling の設定の読み(trace に出ていない)・部品の位置(窓の表は最上位の窓だけで、トップバーの部品は出ない)・縁の遷移の幅・ポインタの座標の変換・Browser の黒い領域の分析。
- 単体テスト 1333 件と、lift の `gdc`・`screen1`・`wall_s`(38 件)は通った。源コードの検査(追加行に特定の拡大率・DPI・DPR の値が無いこと)は 0 件。
- 前回の差分の式: `lift/runtime/fle_screen.h` の `fle_screen_scale_percent`(拡大率 = round(DPR × 100))と `fle_screen_dpi`(DPI = MulDiv(96, 拡大率, 100))。app 側は `app/src/ui/screen-fit.ts` の `chooseGuestScreen`(guest の画面 = `Math.round(部屋 × DPR)`)と `drawScale`。
- 前回の差分に入った経路: 窓の API の記録(`window-api:` の行)を、起動の前から有効にする経路(`app/src/main.ts`・`app/src/engine/client.ts`・`app/src/engine/fruity.ts`)。既定では出ない。測定の道具だけが有効にする。`FLE_MONITOR_DPR`(`FLE_MONITOR_W/H` と同じ、ホストが engine へ渡す実行時の入力)。
- 前回の一時的な実験の切り替え(`FLE_EXP_*`)と、停止時の診断は、commit の前に外されている。作業ツリーに残っていたら外す。
- 測定の道具 `tools/feat-c1-hidpi/measure-size.mjs` は前回の差分にあり、上の表の値を出せる。ただし DPR の乱数の範囲は 1.00〜3.00 のままなので、この Issue で 1.00〜2.25 に直す(下の「残りの作業」)。

## 残りの作業(この Issue の範囲)

前回の差分の上に、残りだけを行う。前回の試行の結果の再測定は、この Issue では行わない。

1. 載った作業ツリーを確かめる: `git status`・`git diff HEAD`。`.prior-work.md` を読む。差分が、上の式(`fle_screen_scale_percent`・`fle_screen_dpi`・`chooseGuestScreen`)と、測定の道具・読みの道具を含むことを確かめる。
2. 測定の道具 `tools/feat-c1-hidpi/measure-size.mjs` を、この Issue の受け入れに必要な項目に絞る。
   - 引数を次の形にする: `--dprs <一覧>`(既定 `1,1.25,1.5,1.7647,2`)・`--random <個数>`(既定 `2`)・`--random-range <最小,最大>`(既定 `1.00,2.25`)。乱数は 0.01 刻みで、起動のたびに crypto の乱数で種を選ぶ(前回の差分のとおり)。`#{{114}}` も同じ道具を、`--dprs 1,1.7647,2,2.5,3 --random 2 --random-range 2.25,3` で使う。
   - 行の項目は、受け入れに使う `dpr`・`randomDprs`・`room`・`state`・`standIn`・`error`・`guest`・`dpi`・`canvas`・`display` を、この Issue の範囲で正しく出す。`flScalingPercent`・`mainGuiScaling`・`parts`・`edgeWidth`・`blackShare`・`debugLog` は、この Issue では測らず null のままにする(`#{{113}}` が埋める。`blackShare` の矩形は `#{{115}}` が決める)。
   - 既定の実行の最後に、DPR 2.5 と 3.0 を起動し、engine-idle に届いたかを、JSON の 1 行 `{"highDpr":{"2.5":"engine-idle" または "stopped","3":"engine-idle" または "stopped"}}` で出す。この 1 行は判定に入れない(終了コードにも入れない)。報告に書く。
3. 作業ツリーに残った一時的な変更を確かめる: `grep -rn "FLE_EXP_" lift app/src` が 0 件であること。
4. 作業ツリーを commit する(受け入れが通れば push される。通らなくても commit する。通らなければ push されず、差分が次の run で続きから取り込まれる)。

## 起動用の実行ファイル(ラベル fl-exe)

この Issue には、ラベル `fl-exe` が付いている。そのため、作業の run では、作業ツリーの `lift/out-app/host-extra/exe/` に、起動用の実行ファイル(`FL64.exe`・`FL64 (scaled).exe`)が在る(commit はしない。git の対象外)。

- 前回の試行で読んだ結果: `FL64.exe` は `dpiAware True/PM`。`FL64 (scaled).exe` は DPI の宣言が無い(推定: Windows が拡大する起動用)。読みの道具は `node tools/feat-c1-hidpi/read-manifest.mjs <file.exe>`。
- ゲストの「プロセス」の DPI の認識は、起動用の実行ファイルのマニフェストが決める。この製品は実行ファイルを実行せず FLEngine の DLL を読むので、runtime は、マニフェストの宣言どおりの認識を返す。
- 製品には入れない。読みの道具は `tools/` に置く(前回の差分のとおり)。

## 背景

依頼者の実機の条件(依頼者が貼ったデバッグパネルの実測。runner の値ではない)。

- `guest screen 3240x1863 device px in a room of 1836x1056 CSS px (scale 0.5667 CSS px per guest px, device pixel ratio 1.7647, 6.04 Mpx)`
- guest(FL)の画面は物理画素(3240×1863)で、FL の UI は 100% の大きさのまま描かれる。CSS では 0.5667 倍に縮むため、操作できない。
- 左の Browser のパネルの下の領域(ブラウザの一覧)が黒いまま。Channel rack・Playlist・Mixer の配置が崩れ、Mixer が左下に小さく出る。ウェルカムのダイアログも小さい。
- 表示方式(現状): `display method D: the runtime describes the desktop (windows, positions, written rectangles) and the render Worker draws it with WebGL2, one texture per window`
- 依頼者が貼った FL の画面: 窓 `Settings - Debugging log` は、画面全体の約 1/4 の大きさ(物理画素 3240×1863 に対して、UI が 100% のまま)で出る。ヘッダーの `直近 60 秒の途切れ` は 343。`engine cycle 0.00 s (1058.1/s)`(入力 837 件・処理 835 件。マウスの 1 イベントごとに engine が 1 周している)。
- 依頼者の観察(ネイティブの Windows): Main GUI Scaling の System が、OS の拡大率(175%)に自動で従う。今の版では従っていない。

FL の仕様(FL 公式マニュアルの General settings の Display の節の記述。実機では未確認。確かめる対象とする):

- 設定の場所: `Options > General settings`(F10)の Display の節。
- Main GUI Scaling / resizing(FL の再起動で効く): 既定は System。System のとき、OS の拡大率に FL が自動で従う(例: OS が 200% なら FL も 200%)。カスタムの割合を手で指定もできる。UI はベクターで描き直される(画素を引き伸ばすのではない)。
- Pop-ups scaling・Toolbars scaling(再起動で効く): 既定は Main(Main GUI Scaling に従う)。個別にも指定できる。
- Cursors scaling: 「OS の設定を使う、またはカスタム」。
- Define 1 inch / set display's PPI: Windows 8.1 以降は画面の PPI が自動で検出される。タッチの校正用。値は FL の見た目を変えないが、一部のプラグインに影響する。
- Legacy scaling(再起動で効く。Advanced の節): 既定は Main(System の拡大に従う)。FL のネイティブプラグインの GUI を拡大する。100%・200%・300%・400% の整数だけ。分数の拡大はできない(Fractional scaling is not possible)。ネイティブプラグインは、プラグインの wrapper の Scaling の倍率でも拡大できる。サードパーティのプラグインは、Bridged の設定で扱う。
- Thick lines(200% 以上の線の太さ)・Animations・Smoothing・Animation refresh rate も、この節にある。

推定(未確認。実測で確かめる。前回の試行で一部を確かめた。上の「前回の試行で分かったこと」を参照):

- FL が UI を拡大するかどうかは、FL が環境から読む値で決まる(実行ファイルのマニフェストの DPI 対応の宣言、DPI の API の戻り値、`GetDeviceCaps` の LOGPIXELS、レジストリの値)。runtime は FL のプロセスを DPI 非対応として扱い、96 を返している(main の版)。
- FL は、96 DPI の 1 倍の UI を、画素数の多い画面に描いている。これが「小さい」の原因とみられる。

コードの事実(main の現在の版。前回の差分は作業ツリーに載る):

- DPI は 96 のまま。`lift/runtime/fle_gdc.h` 31 行(`FLE_GDC_LOGPIXELSX` は 96)、`lift/runtime/fle_screen.h` 57〜59 行(仮想ディスプレイの DPI は 96)、`lift/runtime/fle_appfile.c` 225〜239 行(`GetDpiForMonitor` は 96 を返す)、`lift/runtime/fle_proc.h` 94〜96 行(`GetProcessDpiAwareness` はプロセスを DPI 非対応(UNAWARE)として返す)。
- #1306 の commit `56f5d0e6`(guest の画面を物理画素にする)は、guest の画面を「部屋の CSS × 拡大率」の物理画素にし、canvas を 1 / 拡大率 の CSS px で表示した。その commit 本文に「DPI(LOGPIXELSX)は 96 のまま」と明記されている。commit `24e86883`(題名は「DPI を guest に伝える: 完了 (Closes #1306)」)は変更を含まない(空の commit)。つまり、DPI を guest に伝える変更は main に入っていない。
- `56f5d0e6` の親の版では、guest の画面は部屋の CSS 画素と同じ大きさで、canvas を DPR で拡大して表示していた。ぼやけるが、操作はできた。

Windows の仕様(確かめる対象。DPI の公式の文書で確認する):

- DPI 対応のアプリは、DPI を受け取り、物理画素で描く。DPI は拡大率から決まる(上の 2 の式)。
- DPI 非対応のアプリは、システムが画面を拡大して表示する(ぼやける)。
- 拡大率 100% の時は、どちらも同じ。

## 読む物

- 前回の差分: `lift/runtime/fle_screen.h`・`fle_gdc.c`・`fle_sysmetrics.c`・`fle_appfile.c`・`fle_proc.c`、`app/src/ui/screen-fit.ts`・`desktop.ts`・`main.ts`、`app/test/unit/screen-fit.test.ts`、`tools/feat-c1-hidpi/measure-size.mjs`・`read-manifest.mjs`
- `pm-work/findings/hidpi-fl-scaling.md`(前回の差分の資料。前回の測定の表と、停止の切り分け)
- `tools/boot-check/cdp.mjs`・`tools/boot-check/boot-bundle.mjs`
- `tools/fl-installer-extract.mjs`(インストーラの抽出)、`inputs/fl-installer-files.tsv`

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/size.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const R=L.filter((x)=>"dpr" in x);const H=L.find((x)=>"highDpr" in x);const eq=(x,y)=>Math.abs(x-y)<0.0001;const v=R.length?R[0].randomDprs.values:[];const want=[1,1.25,1.5,1.7647,2,...v];const ok=R.length===7&&H!==undefined&&v.length===2&&v.every((d)=>d>=1&&d<=2.25)&&want.every((d)=>R.some((x)=>eq(x.dpr,d)))&&R.every((x)=>x.error===null&&x.state==="engine-idle"&&x.standIn===false&&Math.abs(x.guest.w-Math.round(x.room.w*x.dpr))<=1&&Math.abs(x.guest.h-Math.round(x.room.h*x.dpr))<=1&&Math.abs(x.dpi-Math.round(96*x.dpr))<=1&&Math.abs(x.canvas.w-x.display.w)<=1&&Math.abs(x.canvas.h-x.display.h)<=1);process.exit(ok?0:1)' "$RUNNER_TEMP/size.jsonl" && node tools/feat-c1-hidpi/read-manifest.mjs "$PWD/lift/out-app/host-extra/exe/FL64.exe" | grep -q 'True/PM' && ! git diff -U0 refs/judge/main -- lift/runtime lift/harness app/src | grep -E '^\+[^+]' | grep -qE '\b(125|150|168|169|175|176)\b|1\.75\b|1\.7647|0\.5667' && npm test --prefix app
```

判定の基準: 道具の出力の生の値を、上の node -e が読む。`size.jsonl` の 7 行(DPR ごとの行)と、最後の 1 行(`highDpr`)。
- (a) 固定の一覧(1・1.25・1.5・1.7647・2)と `randomDprs.values` の 2 つが、全て行に在る。乱数は 1.00〜2.25 の範囲(`randomDprs.values` の 2 つ)。全行が、正常な起動(`error` が null・`state` が engine-idle・`standIn` が false)。
- (b) guest の w と h が、部屋(`room`)× DPR の round の ±1 画素(アプリの `chooseGuestScreen` と同じ式。前回の差分は `Math.round`。許容 ±1 は floor の場合も含む)。
- (c) dpi(LOGPIXELSY。X が無ければ Y)が round(96 × DPR) の ±1。
- (d) canvas と display が ±1 画素で一致する(canvas = 表示の物理画素)。
- (e) `read-manifest.mjs` の `FL64.exe` の出力に `True/PM` の宣言が在る。
- (f) `highDpr` の行は、判定に入れない(報告に書く)。
- 乱数の DPR は、作業者の実行と判定の実行で別になる。そのため、固定の値に合わせた実装は通らない。固定の一覧は DPR の入力としてだけ使い、期待値には使わない。期待値は、各行の DPR から式で計算する。
- ソースの検査(受け入れの最後の段): `lift/runtime`・`lift/harness`・`app/src` の、この変更で足した行(`git diff` の追加行)に、特定の拡大率・DPI・DPR の値(`125`・`150`・`168`・`169`・`175`・`176`・`1.75`・`1.7647`・`0.5667`)が、拡大率・DPI・DPR の意味で現れないこと。一致が 0 件であること。別の意味で一致する場合は、定数名を変えるか、式で書く。
- 単体テスト(`npm test --prefix app`)が通ること。
- 受け入れに入れない項目(測れない項目は null のままでよい): `flScalingPercent`・`mainGuiScaling`・`parts`・`edgeWidth`・`blackShare`・`debugLog`・ポインタの変換(`#{{113}}`)。DPR 2.5 以上(`#{{114}}`)。Browser のパネル(`#{{115}}`)。

## 測定の道具の出力

stdout に、DPR ごとに 1 行の JSON を出し(固定 5 + 乱数 2 = 7 行)、最後に `highDpr` の 1 行を続ける。

```
{"dpr","randomDprs":{"seed","values"},"room":{"w","h"},"state","standIn","error","guest":{"w","h"},"dpi","canvas":{"w","h"},"display":{"w","h"}, …(null のままの項目)}
{"highDpr":{"2.5":"engine-idle または stopped","3":"engine-idle または stopped"}}
```

- `randomDprs`: 起動のたびに選んだ乱数の種(`seed`)と、2 つの値(`values`)。全ての行に同じ値を入れる。
- `room`: 道具が測った部屋の CSS の大きさ。窓の大きさは、部屋が 1836×1056 CSS px になるように決める。
- `guest`: engine の記録の `GetSystemMetrics(SM_CXSCREEN/SM_CYSCREEN)`。
- `dpi`: guest が見た LOGPIXELSY(`GetDeviceCaps`)。LOGPIXELSX の記録が無ければ Y の値。
- `canvas`: `canvas[data-desk="1"]` の width・height(描画領域)。
- `display`: canvas の getBoundingClientRect × DPR(物理画素。四捨五入)。
- `highDpr`: 既定の実行の最後に、DPR 2.5 と 3.0 を起動し、engine-idle に届いたかを `"engine-idle"` または `"stopped"` で出す。判定に入れない。
- `flScalingPercent`・`mainGuiScaling`・`parts`・`edgeWidth`・`blackShare`・`debugLog`: この Issue では null のままでよい。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。既定で動く形にする。
- `FLE_MONITOR_DPR` は、`FLE_MONITOR_W/H` と同じ、ホストが engine へ渡す実行時の入力で、利用者向けの選択肢ではない。前回の作業が、この経路に置いた。新しい利用者向けの旗・URL パラメータ・ビルド変数は作らない。
- 特定の拡大率・DPI・DPR の値を、runtime・app・seed・設定・テストに、定数・表・分岐・seed の値として入れない。この Issue の本文に書いた例の数字(100・125・150・175・176・200%、DPI の 96 以外の値、DPR 1.7647 など)も、製品には入れない。拡大率は、実行時の DPR から式で導く。
- 論理の小さい画面を DPR 倍に拡大して表示する実装を、中間の版としても作らない。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 依頼者の資格情報・設定に触れない。
- Python は製品に使わない(調査と検証にだけ使う)。

## 作業の進め方

- engine の作り直しは約 30 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000)を渡して待つ。
- 1 回の試行(最大約 5.5 時間)で、ビルドは数回までなので、実験はビルドをまとめて計画する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。通らなければ push されず、差分が保存される(次の run が続きから取り込む)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: DPR ごとの表(guest・dpi・canvas と display・状態)、`randomDprs` の種と値、`highDpr` の結果(DPR 2.5 と 3.0 の状態)、`read-manifest.mjs` の `FL64.exe` の `dpiAware` の行、拡大率を導く式の場所(ファイルと行)、特定の値を用意していないことの確認(上の source の検査の grep の出力。一致 0 件)、拡大して表示する版を入れていないことの確認、`FLE_EXP_*` が残っていないことの確認、`#{{113}}`・`#{{114}}`・`#{{115}}` に残した点。

## 他のタスクとの関係

- `#{{113}}`(FL の UI の拡大の実測: F10 の Display・Debug ログ・DPI の API の全呼び出し・採用した拡大率・部品の位置・縁の遷移の幅)は、この Issue の後に行う。
- `#{{114}}`(DPI 240 以上の停止)は、この Issue の後に行う。停止の閉路の入口は未特定。
- `#{{115}}`(Browser のパネルの黒い領域)は、この Issue と並行して進めてよい。描画の経路(`app/src/ui/`・`app/src/engine/`)が重なる時は、後に入る方が pull して直す。
- `#{{108}}`(動的な変化と、全ての拡大率・全ての窓の種類の検証)は、`#{{113}}`・`#{{114}}` の後に行う。
- #1349(音が途切れる現象)は、この Issue の後の物理画素の画面で測る。
- #1334(音声の圧迫)と同じ描画の経路(`app/src/engine/client.ts`・`desk.ts`・`render-worker.ts`)を変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- #1346(トップバーの右クリック)は、部品の座標を使う。この Issue で座標の基準が変わる場合は、同じ手順で main を取り込む。
