---
draft: "006"
title: "HiDPI guest の画面を物理画素にし、DPI を guest に伝える(画面をぼやけさせない)"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["hidpi"]
size: M
priority: High
project: signature-studio/3
---

## 目的

課題 C-1: HiDPI に対応する(画面がぼやけている)。拡大率 1・1.5・2 で、描画の画素と CSS の大きさが一致し、窓の文字や部品が、拡大率 1 の時と同じ CSS の大きさに見えること。

## 規模

規模: 大(想定 6 時間。ビルドを約 5 回回す規模)

## 背景

- 旧の測定(pm-work/findings/survey-c1-hidpi.md。runner の値。変更の前の版)。窓を 1580×957 にした時、拡大率 2 で canvas は 1580×957、CSS は 1580×957。式 `canvas 幅 ÷ (CSS 幅 × dpr)` は 0.5 だった(CSS 1px に描画バッファの 1 画素)。ブラウザが 2 倍に引き伸ばすので、ぼやける。
- 既定の窓では room が 500×99 CSS px しかなく、guest 画面が下限の 640×480 に丸められた(survey-c1 の手順 4)。測定の窓は固定する必要がある。
- 関係するコード(パスと行番号は main の現在の版で確認済み):
  - `app/src/ui/desktop.ts` 21〜25 行: `deskCanvas()` が canvas の大きさを guest 画面の画素数にする(`canvas.width = this.screen.width`)。45〜52 行: CSS の大きさ。
  - `app/src/ui/screen-fit.ts` 3〜4 行の注釈: 「1 guest pixel = 1 CSS pixel。ブラウザが canvas を devicePixelRatio で拡大する」。63〜65 行: `screenEnv` が `FLE_MONITOR_W`・`FLE_MONITOR_H` を engine に渡す。
  - `app/src/main.ts` 260〜262 行(`measureRoom`。room の CSS の大きさ)、264〜276 行(`applyDisplay`。guest 画面を room に合わせる。dpr は表示の文言)、450 行(beacon の `dpi: null`)。
  - `lift/runtime/fle_gdc.h` 31 行: `FLE_GDC_LOGPIXELSX` は 96(guest の DPI は固定)。
  - `app/src/engine/render-worker.ts`(描画の Worker。canvas の大きさを読む側。commit 43a6eb88 で分離済み)。
- 案 A(推定。旧計画の案): guest 画面の大きさを room × dpr(物理画素)にし、canvas の大きさも同じにする。engine へは既存の `FLE_MONITOR_W`・`FLE_MONITOR_H` に物理画素を渡す。canvas の CSS の大きさは room のまま。guest には 96 × dpr の DPI を伝える。
- 案 B(旧計画で退けた案): guest の大きさは据え置き、canvas だけ CSS × dpr にする。描画の元の画素が増えないので、ぼやけは直らない(推定)。
- 画素数は拡大率の 2 乗で増える(1580×957 → 3160×1914 は 4 倍。推定)。描画の所要が増える。影響は子 008 で測る。

## 読む物

- `app/src/ui/desktop.ts`(21〜25 行: 描画の canvas。45〜52 行: CSS の大きさ)
- `app/src/ui/screen-fit.ts`(3〜4 行: 注釈。44〜65 行: `chooseGuestScreen`・`drawScale`・`screenEnv`)
- `app/src/main.ts`(260〜276 行: `measureRoom`・`applyDisplay`。303〜304 行: ResizeObserver と dppx の監視。450 行: beacon の dpi)
- `app/src/engine/render-worker.ts`(canvas の大きさを読む側)
- `app/src/engine/desk.ts`(compose と surface の大きさ)、`app/src/engine/desk-gl.ts`
- `lift/runtime/fle_screen.h`(1〜13 行: 画面の大きさの注釈)、`lift/runtime/fle_gdc.h`(31 行: LOGPIXELSX)、`lift/runtime/fle_sysmetrics.h`(43〜51 行: 仮想ディスプレイの DPI の注釈)
- `tools/boot-check/boot-bundle.mjs`、`tools/boot-check/cdp.mjs`(起動の手順。`extraArgs` に拡大率と窓の大きさを足せる)
- `pm-work/findings/survey-c1-hidpi.md`(測定の手順と値)

## やること

1. 変更の前に、拡大率 1 と 2 で canvas の幅と高さ、CSS の大きさを 1 回ずつ測り、出力の行を報告に貼る。
2. 設計と実装。案 A を基本にする。DPI の経路(guest にどう伝えるか)は作業者が決め、理由を報告に書く。`FLE_GDC_LOGPIXELSX` を実行時の値にする場合は、`lift/runtime/fle_gdc.h` と `fle_screen.h` の `_Static_assert` の対応を壊さない。
3. 測定の道具 `tools/feat-c1-hidpi/measure.mjs` を作る。boot-bundle と同じ起動で、拡大率 1・1.5・2 の 3 回を起動する。拡大率は `--force-device-scale-factor` で指定し、効かない場合は CDP の `Emulation.setDeviceMetricsOverride` を使い、報告に書く。窓は 1580×957 に固定する(`--window-size` と Emulation の両方)。engine-idle の後、1 行の JSON を stdout に出す: `{"dpr","state","standIn","error","elapsedMs","canvasW","canvasH","cssW","cssH","composeMs","putMs","presents"}`。canvas は `canvas[data-desk="1"]` の width・height と getBoundingClientRect。composeMs・putMs・presents は `window.__app.deskStats`。
4. 拡大率 1.5 と 2 の画面を、`.tmp-*` の中に撮って、窓の文字の大きさが拡大率 1 と同じ CSS の大きさに見えるかを、画像を読んで報告する(判定には入れない)。終わったら `.tmp-*` を消す。
5. 画素数の影響を報告する: 拡大率 1 と 2 の、composeMs と putMs の 1 回あたり(累計 ÷ presents)。推定ではなく、この実行の値。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c1-hidpi/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/feat-c1.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=L.length===3&&[1,1.5,2].every((d)=>L.some((r)=>r.dpr===d))&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&Math.abs(r.canvasW-Math.round(r.cssW*r.dpr))<=1&&Math.abs(r.canvasH-Math.round(r.cssH*r.dpr))<=1);process.exit(ok?0:1)' "$RUNNER_TEMP/feat-c1.jsonl"
```

判定の基準: 道具の 3 行の生の値を、上の node -e が読む。`elapsedMs` は出力に含めるが、判定には入れない。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 変えたパス(app/・lift/ を含む)、DPI の経路の選び方、拡大率 1.5 が効いたか、拡大率 1 の値が変わっていないこと、画素数の増加の影響(composeMs・putMs の実測)。
- 「ぼやけが直った」の見た目は、画像を比べていなければ「未確認」と書く。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。既存の `FLE_MONITOR_W`・`FLE_MONITOR_H` は値を変えて使ってよいが、名前を増やさない。
- 製品の既定の動作を、測定のためだけに変えない。窓の大きさと拡大率は、測定の道具の中だけで決める。
- ベンダー名・Cloudflare の識別子・絶対パスを書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit の名義は環境の git の設定のまま。Claude の行を付けない。

## 他のタスクとの関係

- 002(描画の分離)の後に 008 で、画素数の増加の影響を測る。この Issue 自体は 002 を待たない。
- 同じ canvas の経路(`app/src/engine/client.ts`・`desk.ts`・`render-worker.ts`)を、002 が変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- 010(トップバーのメニュー)は、描画の座標に依る。HiDPI の後で座標が変わる場合は、010 の報告に書く。
