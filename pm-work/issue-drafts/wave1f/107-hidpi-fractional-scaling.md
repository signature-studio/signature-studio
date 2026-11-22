---
draft: "107"
title: "Windows と同じ Fractional Scaling を再現し、FL が自分で UI を拡大して、操作できる大きさで鮮明に描く(拡大して表示する版は作らない)"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["hidpi"]
size: XL
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の是正(2026-11-23)。要点は三つ。

- HiDPI の方針が完全に間違っている。物理画素に 1:1 で結びついており、全ての GUI が操作できないほど小さい。
- 描画の崩壊も起きている。
- 依頼者の追加の指示: 拡大されてぼやけた画面は即座に棄却する。Windows と同じ Fractional Scaling を正確に再現すること。

この Issue の目標は、Windows で表示の拡大率(125%・150%・176% など任意の値)を設定した時と同じ見え方にすること。

1. 画面は物理画素の大きさで見える。guest の 1 画素は、表示の 1 物理画素にそのまま対応する(再標本化しない)。
2. DPI は 96 × 拡大率 で報告される(176% なら 169)。
3. システムは拡大も縮小もしない。アプリが自分で、部品・フォント・余白を拡大率に合わせて描く。
4. 結果として、UI の大きさは拡大率 100% の時と同じ見かけの大きさになり、かつ鮮明になる。

禁止: 論理の小さい画面を作り、DPR 倍に拡大して表示する実装(ぼやけた画面)は、中間の版としても作らない。この方式の成果物は依頼者が棄却する。受け入れが通らない時は、判定の段で止まり、push されない。通らなかった理由と到達点を報告に書く。

## 背景

依頼者の実機の条件(依頼者が貼ったデバッグパネルの実測。runner の値ではない)。

- `guest screen 3240x1863 device px in a room of 1836x1056 CSS px (scale 0.5667 CSS px per guest px, device pixel ratio 1.7647, 6.04 Mpx)`
- guest(FL)の画面は物理画素(3240×1863)で、FL の UI は 100% の大きさのまま描かれる。CSS では 0.5667 倍に縮むため、操作できない。
- 左の Browser のパネルの下の領域(ブラウザの一覧)が黒いまま。Channel rack・Playlist・Mixer の配置が崩れ、Mixer が左下に小さく出る。ウェルカムのダイアログも小さい。
- 表示方式(現状): `display method D: the runtime describes the desktop (windows, positions, written rectangles) and the render Worker draws it with WebGL2, one texture per window`

コードの事実(main の現在の版):

- DPI は 96 のまま。`lift/runtime/fle_gdc.h` 31 行(`FLE_GDC_LOGPIXELSX` は 96)、`lift/runtime/fle_screen.h` 57〜59 行(仮想ディスプレイの DPI は 96)、`lift/runtime/fle_appfile.c` 225〜239 行(`GetDpiForMonitor` は 96 を返す)、`lift/runtime/fle_proc.h` 94〜96 行(`GetProcessDpiAwareness` はプロセスを DPI 非対応(UNAWARE)として返す)。
- #1306 の commit `56f5d0e6`(guest の画面を物理画素にする)は、guest の画面を「部屋の CSS × 拡大率」の物理画素にし、canvas を 1 / 拡大率 の CSS px で表示した。その commit 本文に「DPI(LOGPIXELSX)は 96 のまま」と明記されている。commit `24e86883`(題名は「DPI を guest に伝える: 完了 (Closes #1306)」)は変更を含まない(空の commit)。つまり、DPI を guest に伝える変更は main に入っていない。
- `56f5d0e6` の親の版では、guest の画面は部屋の CSS 画素と同じ大きさで、canvas を DPR で拡大して表示していた。ぼやけるが、操作はできた。
- FL は、96 DPI の 1 倍の UI を、画素数の多い画面に描いている。これが「小さい」の原因と推定する(未確認)。

推定(未確認): FL が UI を拡大するかどうかは、FL が見る DPI の情報(実行ファイルのマニフェストの DPI 対応の宣言、DPI の API の戻り値、`GetDeviceCaps` の LOGPIXELS)で決まる。runtime は FL のプロセスを DPI 非対応として扱い、96 を返している。Windows の DPI 対応のアプリと違うところが、FL が拡大しない理由とみられる。実測で確かめる。

Windows の仕様(確かめる対象。DPI の公式の文書で確認する):

- DPI 対応のアプリは、DPI = 96 × 拡大率 を受け取り、物理画素で描く。
- DPI 非対応のアプリは、システムが画面を拡大して表示する(ぼやける)。
- 拡大率 100% の時は、どちらも同じ。

## 読む物

- `lift/runtime/fle_gdc.h`(31 行と 41〜70 行)、`lift/runtime/fle_screen.h`(1〜80 行)、`lift/runtime/fle_sysmetrics.h`(7 行・43〜51 行)、`lift/runtime/fle_proc.h`(94〜97 行)、`lift/runtime/fle_appfile.c`(225〜239 行)、`lift/runtime/fle_spi.h`(90〜93 行。フォントの高さは LOGPIXELSY で決まる)
- `app/src/ui/screen-fit.ts`(1〜6 行の注釈、`chooseGuestScreen`・`drawScale`)、`app/src/ui/desktop.ts`(21〜25 行 `deskCanvas`)、`app/src/main.ts`(`measureRoom`・`applyDisplay`)
- `app/src/engine/render-worker.ts`、`app/src/engine/desk.ts`、`app/src/engine/desk-gl.ts`、`app/src/input/pointer.ts`
- `app/test/unit/screen-fit.test.ts`
- `tools/feat-c1-hidpi/measure.mjs`(#1306 の道具。土台にする)、`tools/boot-check/cdp.mjs`
- `tools/fl-installer-extract.mjs`(インストーラの抽出。FL の実行ファイルを取り出す)、`inputs/fl-installer-files.tsv`(`imports` の列)
- `git show 56f5d0e6`、`git show 56f5d0e6^:app/src/ui/screen-fit.ts`(拡大して表示していた版の基準)
- `pm-work/findings/survey-c1-hidpi.md`

## やること

1. 変更の前に現状を測る。道具 `tools/feat-c1-hidpi/measure-size.mjs`(新規。`measure.mjs` を土台にし、下の「測定の道具の出力」の JSON を出す)で、DPR 1 と 1.7647 を測り、報告に表を貼る。
2. FL が DPI をどう決めるかを特定する(推定を実測で確かめる)。
   a. 実行ファイルのマニフェスト: インストーラを `tools/fl-installer-extract.mjs` で抽出し、主の実行ファイルの PE の RT_MANIFEST 資源を読む(node の道具で。製品には入れない)。`dpiAware`・`dpiAwareness`(PerMonitorV2 など)・`gdiScaling` の値を報告に書く。
   b. FL・FLEngine・プラグインが実際に呼ぶ DPI 関係の API を、engine のログの呼び出しの記録(`run N: <API名>`)で確かめる。対象: `SetProcessDpiAwareness`・`SetProcessDpiAwarenessContext`・`SetProcessDPIAware`・`GetProcessDpiAwareness`・`GetDpiForWindow`・`GetDpiForSystem`・`GetDpiForMonitor`・`GetSystemMetricsForDpi`・`SystemParametersInfoForDpi`・`AdjustWindowRectExForDpi`・`GetDeviceCaps`(LOGPIXELSX/Y・HORZRES/VERTRES・DESKTOPHORZRES)・`GetSystemMetrics`(SM_CXSCREEN/SM_CYSCREEN)・`GetScaleFactorForMonitor`・`SHGetScaleFactorForMonitor`。`inputs/fl-installer-files.tsv` の imports の列は、呼ばれる可能性のある DLL の一覧として使う。
   c. 各 API の戻り値を、Windows の仕様に照らす。拡大率 N% の時の値は、DPI = round(96 × N / 100)、物理画面の大きさ、`GetSystemMetricsForDpi` の拡大された値。
   d. 結論を報告に書く: 「FL が拡大を決める情報は X・Y・Z。runtime の現在の値は A・B・C」。
3. 直す(`lift/runtime` と `app/src`)。
   - guest の画面: round(部屋の CSS 幅 × DPR) × round(部屋の CSS 高さ × DPR) の物理画素。部屋の大きさは `measureRoom` の値を使う。
   - DPI: round(96 × DPR)。DPR は小数のまま扱い、1% 刻みにする(176% なら 169)。FL が見る LOGPIXELSX/Y・`GetDpiForWindow`・`GetDpiForMonitor`・マニフェストの宣言の経路を、Windows の DPI 対応のアプリと同じ値にする。
   - 起動時に DPR を engine に渡す。既存の engine の環境の仕組み(`FLE_MONITOR_W`・`FLE_MONITOR_H` と同じ経路)を使う。新しい環境変数・URL パラメータ・ビルド変数を作らない。既定で動く形にする。
   - ブラウザ側: canvas の描画領域 = 表示領域の物理画素(`ResizeObserver` の `devicePixelContentBoxSize` を使い、端数の CSS 寸法でも表示の物理画素に揃える)。1 guest 画素 = 1 物理画素。`image-rendering` などで誤魔化さない。
   - ポインタ: ブラウザのポインタ(CSS 座標、小数)を、物理画素の位置で guest の画素に変換する(`floor(clientX × DPR)` と同じ。ずれは ±1 画素以内)。
   - 単体テスト `app/test/unit/screen-fit.test.ts` の #1306 の期待値を、物理画素の規則に合わせて直す。
   - 画面の大きさと DPI が実行中に変わった時(動的な変化)は、既存の `WM_DISPLAYCHANGE` の経路で engine に伝える。`WM_DPICHANGED` の送り方は #{{108}} が検証する。この Issue の受け入れは、起動時の DPR で正しいこと。
   - 崩壊: Browser のパネルの黒い領域、Mixer の位置、配置のずれが、物理画素の画面で出ないこと。原因が guest の画面の大きさ(部品の座標の計算)なのか、render Worker(WebGL2・1 窓 1 テクスチャ・テクスチャの大きさの上限・描画の矩形の通知)なのかを切り分けて直し、報告に書く。
   - FL が自分で拡大しないと実測で分かった場合は、受け入れを満たせない。その証拠(呼ばれた API・返した値・マニフェストの値)を報告に書く。拡大して表示する版で代えない。
4. 測定の道具 `tools/feat-c1-hidpi/measure-size.mjs` を完成させる。道具の先頭に、部品の位置の求め方と、各閾値の根拠を書く。

## 測定の道具の出力

stdout に、DPR ごとに 1 行の JSON を、DPR 1・1.25・1.5・1.7647・2 の順に出す。

```
{"dpr","room":{"w","h"},"state","standIn","error","guest":{"w","h"},"dpi","scaleStep","parts":{"tempo":{"x","y"},"song-position":{"x","y"},"pattern-selector":{"x","y"}},"canvas":{"w","h"},"display":{"w","h"},"edgeWidth","blackShare"}
```

- `room`: 道具が測った部屋の CSS の大きさ。窓の大きさは、部屋が 1836×1056 CSS px になるように決める。
- `guest`: engine の記録の `GetSystemMetrics(SM_CXSCREEN/SM_CYSCREEN)`。
- `dpi`: guest が見た `GetDeviceCaps(LOGPIXELSX)`。
- `scaleStep`: FL が拡大を段に丸める場合だけ、FL が使った拡大率(%)を入れる。丸めない時は省く。
- `parts`: 部品(Tempo・Song Position・Pattern Selector)の左上の、ページの CSS 座標。
- `canvas`: `canvas[data-desk="1"]` の width・height(描画領域)。
- `display`: canvas の getBoundingClientRect × DPR(物理画素。四捨五入)。
- `edgeWidth`: スクリーンショットの、Tempo の部品の枠の縁の遷移の幅(画素。10%〜90% の遷移の、水平と垂直の中央値)。FL が自分で描いた縁は 2 画素以下。拡大されてぼやけた縁は 3 画素以上に広がる。
- `blackShare`: Browser のパネルの矩形(CSS 座標で道具が決め、DPR に依らず同じ矩形)の中で、RGB が全て 16 以下の画素の割合(0〜1)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/size.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const D=[1,1.25,1.5,1.7647,2];const P=["tempo","song-position","pattern-selector"];const near=(x,y,t)=>Math.abs(x-y)<=Math.max(2,t*Math.abs(y));const r=(d)=>L.find((x)=>Math.abs(x.dpr-d)<0.0001);const a=r(1);const row=(x,d)=>x.error===null&&x.state==="engine-idle"&&x.standIn===false&&Math.abs(x.guest.w-Math.round(x.room.w*d))<=1&&Math.abs(x.guest.h-Math.round(x.room.h*d))<=1&&Math.abs(x.dpi-Math.round(96*d))<=1&&Math.abs(x.canvas.w-x.display.w)<=1&&Math.abs(x.canvas.h-x.display.h)<=1&&x.edgeWidth<=2&&(x.scaleStep===undefined||Math.abs(x.scaleStep/(100*d)-1)<=0.03)&&P.every((p)=>near(x.parts[p].x,a.parts[p].x,0.05)&&near(x.parts[p].y,a.parts[p].y,0.05))&&Math.abs(x.blackShare-a.blackShare)<=0.02;const ok=L.length===5&&D.every((d)=>r(d)!==undefined)&&D.every((d)=>row(r(d),d))&&a.blackShare<0.5&&Math.abs(a.room.w-1836)<=2&&Math.abs(a.room.h-1056)<=2;process.exit(ok?0:1)' "$RUNNER_TEMP/size.jsonl" && npm test --prefix app
```

判定の基準: 道具の 5 行の生の値を、上の node -e が読む。全ての DPR(1・1.25・1.5・1.7647・2)で、(a) guest の w と h が round(room × DPR) の ±1 画素、(b) dpi が round(96 × DPR) の ±1、(c) 部品の parts が DPR 1 の値と ±5% 以内(小さい値は 2 CSS px まで)。scaleStep がある行は |scaleStep / (100 × DPR) − 1| が 0.03 以内。(d) canvas と display が ±1 画素で一致、(e) edgeWidth が 2 以下、(f) blackShare が DPR 1 の値と 2 ポイント以内。DPR 1 の blackShare は 0.5 未満、DPR 1 の room は 1836×1056 ±2 CSS px。単体テスト(`npm test --prefix app`)が通ること。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: FL が拡大を決める情報(マニフェストの値、呼ばれた API と戻り値)、DPR ごとの表(guest・dpi・部品の座標・edgeWidth・blackShare)、崩壊の原因(切り分けの結果)、変えたパス(app/・lift/・tools/ を含む)、拡大して表示する版を入れていないことの確認(`grep` の結果)、#{{108}} に残した点。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。既定で動く形にする。
- 論理の小さい画面を DPR 倍に拡大して表示する実装を、中間の版としても作らない。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 依頼者の資格情報・設定に触れない。

## 他のタスクとの関係

- #{{108}}(動的な変化と、全ての拡大率・全ての窓の種類の検証)は、この Issue の後に行う。
- #{{109}}(音が途切れる現象)は、この Issue の後の物理画素の画面で測る。
- #{{040}}(音声の圧迫)と同じ描画の経路(`app/src/engine/client.ts`・`desk.ts`・`render-worker.ts`)を変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- #{{006}}・#{{008}} の結果(DPI の経路と、その実測)は、この Issue が置き換える。
- #{{106}}(トップバーの右クリック)は、部品の座標を使う。この Issue で座標の基準が変わる場合は、同じ手順で main を取り込む。
