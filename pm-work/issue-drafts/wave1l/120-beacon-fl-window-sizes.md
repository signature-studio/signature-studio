---
draft: "120"
title: "ビーコンに、FL の窓の大きさ(上位 8 窓: クラス名・幅・高さ)を足し、実機の FL の UI の拡大率を、記録から求められるようにする"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["hidpi", "verify"]
size: M
priority: High
project: signature-studio/3
---

## 目的

実機の FL の UI の拡大率を、ビーコンの記録から求められるようにする。依頼者の実機で「UI が小さい」と見えるとき、FL が拡大した結果の窓の大きさが記録に無ければ、拡大率を実機で測れない。FL の窓の大きさを、ビーコンに足す。

## 背景

- runner の測定(#{{113}}): Welcome 窓の幅は、DPR 1 のとき 874、DPI 169(DPR 1.7647)のとき 1529。FL の拡大率は、窓の幅 ÷ DPR 1 の幅(1529 ÷ 874 ≈ 175%)で求めた(`pm-work/issue-drafts/wave1i/113-fl-ui-scale-measure.md` の表。`pm-work/findings/hidpi-fl-scaling.md`)。
- 今のビーコン(版 3)には、画面の大きさ(`perf.guestScreen`)と engine の性能の項目はあるが、FL の窓の大きさが無い。
- 窓の一覧は、画面のデバッグパネル(`app/index.html` の `Guest top-level windows` の表 `#windows`)に出ている。その元は `window.__app.summary.windows`(`app/src/main.ts` の `refreshPanels`。`app/src/engine/fruity-log.ts` の `EngineWindow`: `className`・`width`・`height`・`relation`・`z`。エンジンのログの行から作る)。
- ビーコンの版は 3 のまま、任意の項目を足す。既存の項目の定義は変えない。古い記録には、この項目が無い(報告の表で示す)。

## 読む物

- `app/src/perf/beacon.ts`(`BeaconPayload`。版 3 の注釈。任意の項目の書き方: `files`・`assets`・`desk`・`engineRun` の注釈)
- `app/shared/perf.ts`(`beaconProblems`。任意の項目の検査の書き方)
- `app/worker/perf-store.ts`・`app/worker/perf-summary.ts`(保存と要約。項目が増えても壊れないこと)
- `app/src/main.ts`(`facts()` の組み立て。`refreshPanels` と `window.__app`)
- `app/src/engine/fruity-log.ts`(`EngineWindow`)
- `app/test/unit/perf-beacon.test.ts`(既存の単体テスト)

## やること

0. 作業ツリーの確認。`.prior-work.md` が在れば、最初に読む。
1. `app/src/perf/beacon.ts` の `BeaconInput`・`buildBeacon` に、任意の項目 `uiWindows` を足す。型は `{cls: string, w: number, h: number}` の配列で、最大 8 件。`cls` は窓のクラス名、`w`・`h` は窓の幅と高さ(整数)。個人のパスや題名(`title`)は含めない。元は `window.__app.summary.windows` のうち、`relation` が空の最上位の窓を、`z` の小さい順(前面が先)に 8 件。窓の一覧が無いとき(エンジンのログがまだ無いとき)は、項目を出さない(版 3 の既存の形と同じ)。
2. `app/shared/perf.ts` の `beaconProblems` が、`uiWindows` を任意の項目として検査する。在れば配列で、各要素が `cls`(文字列)・`w`・`h`(整数)の形であること。違えば問題として返す。無ければ、従来どおり問題は無い。保存側(`app/worker/perf-store.ts`)と要約(`app/worker/perf-summary.ts`)は、従来の項目の扱いを変えない。
3. `app/src/main.ts` の `facts()` で、`uiWindows` を組み立てる。窓の一覧は既にあるものを読む(新しい問い合わせを増やさない)。
4. 単体テスト(`app/test/unit/perf-beacon.test.ts`)を足す。(a) 窓の一覧を渡すと、ビーコンに `uiWindows` が入り、各要素が `{cls, w, h}` の形で、8 件以下、最上位の窓だけで、`z` の順に並ぶ。(b) 窓の一覧が無いと、項目が無い。(c) `beaconProblems` が、形の違う `uiWindows` を問題にし、正しい物は通す。
5. 拡大率・DPI・DPR の特定の値を、アプリ・テストに入れない。拡大率は記録から求める。アプリは式を持たない。テストの値は窓の大きさの任意の例でよい。
6. 受け入れを通し、作業ツリーを commit する。

## 受け入れ

```accept
npm run build --prefix app && npm test --prefix app && grep -q uiWindows app/test/unit/perf-beacon.test.ts
```

判定の基準: ビルドと単体テストの全てが通る。単体テストの中で、`uiWindows` の形を確かめている(上の 4 の (a)〜(c))。

## 報告の規則

- 1 行目は「実行: <実行したコマンド>、終了コード <数>」で始める。
- 報告に書く: (1) `uiWindows` の形(型と件数の上限)、(2) 単体テストの名前と結果の行、(3) 保存・要約の項目の扱いが変わらないことの根拠。
- `app/` を変えたので、push で `deploy.yml` が動き、本番に上がる。報告に、その旨を書く。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 特定の拡大率・DPI・DPR の値を、アプリ・テストに入れない。
- 既存のビーコン項目の定義を変えない(版は 3 のまま)。
- ベンダー名と Cloudflare の識別子を書かない。
- Python は使わない。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。sleep・timeout・nohup・末尾の `&` は使わない。
- この機械で製品のビルドやテストをしない(Actions で行う)。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000。上限 5400000)を渡して待つ。背景で起動して待たない。

## 他のタスクとの関係

- blocked_by: なし。
- #{{113}}(#1353。runner の測定。拡大率を窓の幅から求めた根拠)。
- #{{119}}(同じ Epic。本番の実機のビーコンを読む。この項目が入る前の記録には、窓の大きさが無い。119 は動いてよい。120 を待たない)。
