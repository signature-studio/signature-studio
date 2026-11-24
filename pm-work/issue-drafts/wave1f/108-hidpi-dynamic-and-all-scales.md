---
draft: "108"
title: "Fractional Scaling の動的な変化と、全ての拡大率(100%〜300%)・全ての窓の種類の検証"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: ["107", "113", "114"]
labels: ["hidpi", "verify"]
size: L
priority: High
project: signature-studio/3
---

## 目的

#{{107}} で、起動時の DPR に対して Windows と同じ Fractional Scaling を再現した。この Issue は、その結果を全ての拡大率と全ての窓の種類で確かめ、ブラウザのズームや窓の移動による DPR の変化(動的な変化)で、FL の仕様(拡大率は再起動で適用される)に合った振る舞いを確かめる。

依頼者の要求: 全ての GUI が操作できる大きさで、ぼやけないこと。拡大されてぼやけた画面は棄却する(依頼者の指示)。特定の拡大率の数字を、定数・表・seed として予め用意しない(拡大率は、環境の DPR から Windows と同じ計算で導く)。HiDPI は依頼者の実機で正常に動いている(2026-11-24 確認)ので、この Issue の範囲は、動的な変化と全ての拡大率の確認に限る。

## 背景

- #{{107}} の道具 `tools/feat-c1-hidpi/measure-size.mjs` を、この Issue で拡張して使う(下の「やること」1)。
- FL の仕様(FL 公式マニュアルの General settings の Display の節。実機では未確認):
  - Main GUI Scaling・Pop-ups・Toolbars・Legacy scaling は、FL の再起動で効く。実行中に変えても適用されない。
  - ネイティブプラグインの GUI は、100%・200%・300%・400% の整数だけ(Legacy scaling)。
  - Cursors scaling と Thick lines(200% 以上の線の太さ)がある。
- Windows の動的な変化: DPI が変わると、DPI 対応のアプリは `WM_DPICHANGED` を受け取り、画面の大きさが変わると `WM_DISPLAYCHANGE` が届く。ただし FL は拡大率を再起動で適用するので、Windows の振る舞いは FL の仕様より優先しない。
- Windows の FL の実際の挙動は、この機械では確かめられない。確認できない部分は「確認できない」と報告に書く。
- 起動用の実行ファイル: `FL64.exe`(マニフェストに `dpiAware True/PM`)と `FL64 (scaled).exe`(DPI の宣言なし。Windows が拡大する形と読める。推定)。この製品は `FL64.exe` の振る舞いに合わせる。`(scaled)` の方式は、この Issue の範囲外。
- 既存の経路: 画面の大きさの更新は `app/src/main.ts`(`applyScreenParam`・`applyDisplay`・DPR の監視)と `lift/runtime` の `WM_DISPLAYCHANGE`(`grep -rn WM_DISPLAYCHANGE lift/runtime` で探す)。
- DPI が 240 以上(DPR 2.5 以上)の起動は、#{{114}} で直った。この Issue では DPR 2.5〜3.0 を含めて確かめる。
- #{{113}}(FL の UI の拡大の実測)と #{{115}}(Browser のパネル)と #{{118}}(WebView2 の面)の結果を使う。

## 前回の試行で分かったこと

前回の試行: run 45327829718(3 時間 37 分)。判定は終了コード 1 で、push は無し。作業は保存され、次の試行は、その差分を `.prior-work.md` と共に作業ツリーで受け取る(取り込めたかは、`.prior-work.md` と `git status` で確かめる)。

- 製品の変更(前回の作業の中身。作業ツリーに在る):
  - `app/src/main.ts`: DPR の変化を、部屋の測り直しと画面の追随へ渡す。`matchMedia` の参照を保持し、`resize` と 250 ms の確認を足した。
  - `app/src/ui/screen-fit.ts`(`restartNeeded`: DPR の百分率が起動時と違うか)、`app/index.html`・`app/src/style.css`(`#restart-note`。`data-restart-notice`)。
  - `lift/runtime/fle_input.c`: 画面の大きさの変化を、engine のログに `input: WM_DISPLAYCHANGE` の 1 行で出す。
  - `tools/feat-c1-hidpi/measure-size.mjs`: `--windows`・`--dynamic`・`randomDprs`・`restartNotice`・`debugLog`・`pointerErrPx` を足した。
  - 転送後の canvas の幅を設定すると、例外(`Cannot resize canvas after call to transferControlToOffscreen()`)で `applyDisplay` が止まる。canvas の大きさは既存の経路で正しく追随するので、その設定は入れない。
- 動的(4 段: DPR 1 → 乱数の中間 → 2 → 1): ゲストの画面は round(部屋 × DPR) ±1 で追随し、canvas と表示が一致した。ページの例外 0、DPI の問い合わせ 0。FL の DPI(96)と拡大率(100)は起動時の値のまま。再起動の案内は、DPR の百分率が起動時と違う段(段 2・3)だけに出た。`WM_DISPLAYCHANGE` は段ごとに 1 行。WM_DPICHANGED は送らなかった(FL の DPI は起動時に決まるため。FL の拡大率は再起動まで変わらない)。
- 静的(10 段 × 窓 8 種 = 80 行): 60 行で窓を開けて測れた。開けなかったのは plugin・tooltip の 20 行(#{{137}} で扱う)。主窓の 3 部品のポインタの誤差は、全 10 段で 0 画素。
- 採用段(エンジンのログの値): 固定の 7 点と乱数の 3 点の 10 点で、DPI ÷ 96 × 4 を切り捨て、25 を掛けた値(25% 刻みで下に丸める)と一致した。
- 主窓の縁(`edgeWidth`): 2.2〜3.5 px(DPR 1 と乱数の 1.04 で 2.24 px)。縁の幅は採用段に比例して広がらず、物理画素で描かれている。
- menu の縁: 2.5・2.69・3 以外の段で測れた(2.19〜3.04 px)。2.5・2.69・3 では測れなかった(null。原因は未確認)。menu は必須の窓なので、この 3 点で測れるように直す。
- blackShare: guest の canvas の Browser 矩形は全段で 1.0(#{{115}} の問題)。ページの画面では 0(#{{118}} の面)。判定はページの画面で測る(`tools/browser-panel/measure.mjs`)。
- 部品の座標: 固定の DPR では、CSS px で 1 px 以内に揃った。乱数の DPR と、動的の段 2・3 では Tempo 枠の検出が外れた(理由は未確認。#{{137}} で調べる)。判定からは外す。
- 前回の判定が落ちた理由(判定の式の欠陥。製品の失敗ではない):
  1. 縁の「2 以下」は、DPR 1 の 2.24 px と両立しない。
  2. 採用段の「round(100 × DPR) ±3」は、乱数の DPR で丸めの規則と外れた。
  3. 部品の座標を guest 画素の生の値で比べ、DPR ≠ 1 で必ず外れた。
  4. plugin・tooltip の 20 行が開けず、必須の判定が落ちた。
  5. blackShare が guest の canvas の黒を見ていた。
  6. 縁が null のとき「null ≤ 2」が真になり、測れなかった縁が通った。
  7. ソースの検査の grep が `refs/judge/main` を使い、作業者のセッションでは空振りした(判定の段では有る)。

## 読む物

- #{{107}} の成果物(`tools/feat-c1-hidpi/measure-size.mjs`、`lift/runtime` の DPI の経路、`app/src/ui/screen-fit.ts`・`desktop.ts`)
- 前回の作業の差分(作業ツリーの `.prior-work.md` と `git diff HEAD`)
- `lift/runtime/fle_win.c`(窓の大きさの変更と、`WM_DISPLAYCHANGE`・`WM_DPICHANGED` の送り方)
- `app/src/main.ts`(`applyScreenParam`・`applyDisplay`・DPR の監視)
- `app/src/engine/render-worker.ts`(窓ごとのテクスチャ)、`app/src/input/pointer.ts`(ポインタの変換)
- `pm-work/findings/hidpi-fl-scaling.md`(#{{107}} の測定の結果。FL の拡大の決め方と、Debug ログの取り方)
- `tools/browser-panel/measure.mjs`(ページの画面の blackShare の測り方)

## やること

1. 道具 `tools/feat-c1-hidpi/measure-size.mjs` を、前回の差分を土台に直す(受け入れの引数と同じ形を出す)。
   - `--dprs 1,1.25,1.5,1.75,2,2.5,3`(固定の 7 点。DPR の入力としてだけ使い、期待値には使わない)と、`--random 3 --random-range 1,3`(乱数の 3 点。0.01 刻み。起動のたびに種を選び、種と値を `randomDprs` に出す)。静的の行は 10 段 × 窓 8 種 = 80 行。
   - `--windows main,welcome,mixer,playlist,piano-roll,plugin,tooltip,menu`(窓の種類ごと。開けない窓は、その行の `error` に理由を書く)。
   - `--dynamic 1,<中間>,2,1`(1 回の実行の中で DPR をこの順に変える。`Emulation.setDeviceMetricsOverride`)。中間の値は受け入れのたびに乱数で選ぶ(下の受け入れ)。動的の行は段ごとに 1 行。
   - 各行に `window`・`mode`・`randomDprs`・`debugLog`(FL の Debug ログ。`scalingFactor` を含む)・`flScalingPercent`・`edgeWidth`・`pointerErrPx`・`restartNotice`(動的の行)を出す。
2. 静的の行: 下の判定の基準を全ての行で満たす。
3. 動的な変化: 段ごとに、guest が round(部屋 × 段の DPR) に追随し、canvas と表示が一致し、FL の DPI と採用段が起動時の値のまま変わらず、再起動の案内が DPR の百分率が違う段だけに出ること。`WM_DPICHANGED` を送るかどうかは、FL の実測で決め、理由を報告に書く(前回は送らなかった)。
4. 直す(動的の変化で崩れる所だけ。`lift/runtime` と `app/src`)。
   - 拡大率(FL が見る DPI と、FL の UI の拡大率)は、再起動まで変えない。
   - 利用者への案内: 既存の `#restart-note` で、拡大率の変更は FL を再起動すると適用される旨を出す。自動で再起動を提案しない(FL の再起動は利用者の操作)。
   - 転送後の canvas の幅を、設定しない(例外になる)。
   - 新しい環境変数・URL パラメータ・ビルド変数を作らない。拡大率の数値を runtime・app・seed に書かない。
5. menu の縁: DPR 2.5・2.69・3 で測れなかった原因を調べ、測れるようにする(測り方か、窓の開き方を直す。製品の既定の動作は、測定のためだけに変えない)。
6. 部品の座標と Tempo 枠の検出: 判定から外す。報告には、乱数の DPR と動的の段 2・3 で外れた値を書く。理由の調査は #{{137}} で行う。
7. ポインタ: 静的の主窓で、3 部品の中心を押し、engine が受け取った画素と押した画素の差(`pointerErrPx`)が 1 以下であること。
8. plugin・tooltip の窓: この Issue では開けなくてよい(行の `error` に理由を書く)。#{{137}} で扱う。
9. Debug ログ: 全ての行で、`debugLog.scalingLines` に error・warn の行が無いこと。出ていれば、その行を報告に書き、原因を直す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && D=$(node -e 'process.stdout.write((require("crypto").randomInt(101,301)/100).toFixed(2))') && node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dprs 1,1.25,1.5,1.75,2,2.5,3 --windows main,welcome,mixer,playlist,piano-roll,plugin,tooltip,menu --random 3 --random-range 1,3 --dynamic 1,$D,2,1 > "$RUNNER_TEMP/fs.jsonl" && node tools/browser-panel/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/browser.json" && node -e 'const fs = require("fs"); const L = fs.readFileSync(process.argv[1], "utf8").trim().split("\n").map((l) => JSON.parse(l)); const B = JSON.parse(fs.readFileSync(process.argv[2], "utf8")); const W = ["main", "welcome", "mixer", "playlist", "piano-roll", "plugin", "tooltip", "menu"]; const REQ = ["main", "menu"]; const eq = (x, y) => Math.abs(x - y) < 0.0001; const S = L.filter((x) => x.mode === "static"); const Dy = L.filter((x) => x.mode === "dynamic"); const R = S.length ? S[0].randomDprs.values : []; const F = [1, 1.25, 1.5, 1.75, 2, 2.5, 3].concat(R); const a = S.find((x) => eq(x.dpr, 1) && x.window === "main"); if (!a || a.edgeWidth === null || !a.room) { console.log("judge: DPR 1 の主窓の行が無い、または縁が測れない"); process.exit(1); } const adp = (x) => Math.floor(x.dpi / 96 * 4 + 1e-9) * 25; const boot = (x) => x.state === "engine-idle" && x.standIn === false && x.room !== null && x.guest !== null && x.canvas !== null && x.display !== null && x.dpi !== null; const reqOk = (x) => !REQ.includes(x.window) || x.error === null; const geo = (x) => Math.abs(x.guest.w - Math.round(x.room.w * x.dpr)) <= 1 && Math.abs(x.guest.h - Math.round(x.room.h * x.dpr)) <= 1 && Math.abs(x.canvas.w - x.display.w) <= 1 && Math.abs(x.canvas.h - x.display.h) <= 1; const dbg = (x) => x.debugLog.captured === true && x.debugLog.lines >= 1 && !x.debugLog.scalingLines.some((l) => /error|warn/i.test(l)); const baseOf = (w) => { const b = S.find((x) => eq(x.dpr, 1) && x.window === w); return b ? b.edgeWidth : null; }; const edge = (x) => { if (x.edgeWidth === null) return !REQ.includes(x.window); const b = baseOf(x.window); return b === null || x.edgeWidth <= b * (1 + (x.dpr - 1) / 2) + 0.5; }; const sRow = (x) => boot(x) && reqOk(x) && geo(x) && x.mainGuiScaling === "System" && dbg(x) && Math.abs(x.dpi - Math.round(96 * x.dpr)) <= 1 && x.debugLog.scalingFactor !== null && Math.round(x.debugLog.scalingFactor.factor * 100) === adp(x) && edge(x); const dRow = (x) => boot(x) && reqOk(x) && geo(x) && dbg(x) && x.dpi === a.dpi && x.flScalingPercent === adp(a) && edge(x) && x.restartNotice === (Math.round(x.dpr * 100) !== Math.round(a.dpr * 100)); const ptrOk = (x) => x.pointerErrPx !== null && x.pointerErrPx <= 1; const bad = (rows, f) => rows.filter((x) => !f(x)).map((x) => `${x.window}@${x.dpr}${x.mode === "dynamic" ? " step " + x.step : ""}`); const last = Dy.length === 4 ? Dy[3] : { edgeWidth: null }; const checks = { rows: S.length === 80 && Dy.length === 4, dprs: F.every((d) => W.every((w) => S.some((x) => eq(x.dpr, d) && x.window === w))), rnd: R.length === 3 && R.every((d) => d >= 1 && d <= 3), room: Math.abs(a.room.w - 1836) <= 2 && Math.abs(a.room.h - 1056) <= 2, static: S.every(sRow), pointer: S.filter((x) => x.window === "main").every(ptrOk), steps: Dy.length === 4 && Dy.every((x, i) => x.step === i + 1) && eq(Dy[0].dpr, 1) && Dy[1].dpr > 1 && Dy[1].dpr <= 3 && eq(Dy[2].dpr, 2) && eq(Dy[3].dpr, 1), dynamic: Dy.every(dRow), last: last.edgeWidth !== null && Math.abs(last.edgeWidth - a.edgeWidth) <= 0.5, browser: B.error === null && B.state === "engine-idle" && B.standIn === false && B.blackShare !== null && B.blackShare < 0.5, }; for (const k of Object.keys(checks)) if (!checks[k]) console.log(`judge: 失敗 ${k}`); const badS = bad(S, sRow); if (badS.length) console.log(`judge: 静的の行で外れた: ${badS.join(", ")}`); const badD = bad(Dy, dRow); if (badD.length) console.log(`judge: 動的の行で外れた: ${badD.join(", ")}`); const badP = S.filter((x) => x.window === "main" && !ptrOk(x)).map((x) => `main@${x.dpr}`); if (badP.length) console.log(`judge: ポインタが外れた: ${badP.join(", ")}`); console.log(`judge: ${Object.entries(checks).map(([k, v]) => `${k}=${v}`).join(" ")}`); const ok = Object.values(checks).every(Boolean); console.log(ok ? "judge: 通った" : "judge: 通らない"); process.exit(ok ? 0 : 1);' "$RUNNER_TEMP/fs.jsonl" "$RUNNER_TEMP/browser.json" && b=$(git merge-base HEAD origin/main) && d=$(git diff -U0 "$b" -- lift/runtime lift/harness app/src) && ! grep -E '^\+[^+]' <<<"$d" | grep -qE '\b(125|150|168|169|175|176)\b|1\.75\b|1\.7647|0\.5667' && npm test --prefix app
```

判定の基準(道具の出力の生の値を、上の node -e が読む。3 本の出力: `fs.jsonl`(測定)・`browser.json`(ページの画面の黒)・ソースの検査):

- (a) 静的の行は 80 行。固定の 7 点と `randomDprs.values` の 3 点の全てに、窓 8 種の行が在る。全ての行で、state が engine-idle、standIn が false、guest が round(部屋 × DPR) の ±1 画素、canvas と display が ±1 画素、dpi が round(96 × DPR) の ±1、Main GUI Scaling が "System"、debugLog が captured(lines が 1 以上、error・warn の行が無い)。FL の採用段(`debugLog.scalingFactor` の factor × 100)が、DPI ÷ 96 × 4 を切り捨てて 25 を掛けた値と一致する。
- (b) 窓: main と menu は、error が null で、縁(`edgeWidth`)が測れている。他の窓(welcome・mixer・playlist・piano-roll・plugin・tooltip)は、error と null を許す。縁が測れた行は、同じ窓の DPR 1 の縁(`base`。DPR 1 が測れていなければ判定しない)に対して、`edgeWidth` ≤ base × (1 + (DPR − 1) ÷ 2) + 0.5 画素を満たす。拡大による縁のぼやけは、DPR に比例して縁が広がるので、この上限を超える。
- (c) 主窓の静的の行: `pointerErrPx` が 1 以下。
- (d) 動的の行は 4 行(段 1〜4)。段 1 と 4 は DPR 1、段 2 は乱数の中間(1.01〜3.00)、段 3 は DPR 2。各段で、guest・canvas と display・debugLog の基準を満たし、DPI が起動時(DPR 1)の値のまま、FL の採用段が起動時の値のまま、縁が (b) の基準を満たす。`restartNotice` が、DPR の百分率が DPR 1 と違う段だけ true。段 4 の縁は、DPR 1 の主窓の縁と ±0.5 画素以内。
- (e) ページの画面の黒: `tools/browser-panel/measure.mjs` の `blackShare` が 0.5 未満(error が null、state が engine-idle)。guest の canvas だけの黒は判定から外す(報告に書く)。
- (f) 乱数の DPR と、動的の中間の DPR は、作業者の実行と判定の実行で別になる。固定の値に合わせた実装は通らない。
- (g) ソースの検査(受け入れの最後の段): `git merge-base HEAD origin/main` からの差分の、`lift/runtime`・`lift/harness`・`app/src` の追加行に、特定の拡大率・DPI・DPR の値が、拡大率・DPI・DPR の意味で現れない(一致 0 件)。
- (h) 単体テスト(`npm test --prefix app`)が通る。
- 判定の式は、部品の座標・Tempo 枠の検出・plugin と tooltip の拡大・guest の canvas の黒を見ない(報告にだけ書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 拡大率と窓の種類ごとの表(`edgeWidth`・`blackShare`(ページの画面)・採用段)、`randomDprs` の種と値、動的の変化の結果(段ごとの guest・DPI・採用段と、engine のログの `WM_DISPLAYCHANGE`・`WM_DPICHANGED` の行)、`WM_DPICHANGED` を送ったかどうかと理由、再起動の案内の方式と理由、ポインタの結果、Debug ログの error・warn の有無、開けなかった窓と理由、menu の縁が測れなかった原因、部品の座標と Tempo 枠の検出の値(判定の外)、確認できなかった Windows の実際の挙動、ソースの検査の grep の出力。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 特定の拡大率・DPI・DPR の値を、runtime・app・seed・設定・テストに、定数・表・分岐・seed の値として入れない。拡大率は、実行時の DPR から式で導く。
- 論理の小さい画面を DPR 倍に拡大して表示する実装を、作らない。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- ビルド(engine の作り直し)は約 35 分。測定は約 50 分。受け入れ 1 回は約 90 分。判定の段で、もう一度走る。
- 長いコマンド(ビルド・受け入れ)は、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- 開始から 2 時間 30 分を過ぎたら、受け入れを新たに始めない。開始から 4 時間 00 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit して、報告を書いて終える(判定の段の約 90 分が、ジョブの上限に収まるように)。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{107}}(起動時の DPR に対する Windows と同じ Fractional Scaling)、#{{113}}(FL の UI の拡大の実測)、#{{114}}(DPI 240 以上の停止)。
- #{{137}}(plugin と tooltip の窓。Tempo 枠の検出の理由の調査)は、この Issue の判定の外。
- #{{109}}(音が途切れる現象)は、この Issue と並行して進めてよい。ただし、物理画素の画面で測る。
- 同じ描画の経路を変える #1334 と #1346 と衝突しうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
