---
draft: "137"
title: "plugin の窓と tooltip の窓を、全ての拡大率で開いて拡大を測る"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: ["127", "108"]
labels: ["hidpi", "verify"]
size: S
priority: Medium
project: signature-studio/3
---

## 目的

#{{108}} の判定から外した、plugin の窓と tooltip の窓を、全ての拡大率(固定の 7 点と乱数の 3 点)で開き、その拡大が FL の採用段と一致することを確かめる。ぼやけた・拡大した画面を出さない(依頼者の指示)。特定の拡大率の数字を、定数・表・seed として予め用意しない(拡大率は、環境の DPR から Windows と同じ計算で導く)。

## 背景

- #{{108}} の測定で、plugin と tooltip の 20 行は、窓を開けなかった(`error` に理由)。
  - plugin: このプロジェクトに Fruity Balance が無く、ミキサーのインサート欄への挿入を、道具が自動化していない。
  - tooltip: 上部バーの 3 点に 2.5 秒ずつホバーしても、新しい最上位の窓が出ない(32 点の掃引でも出なかった)。仕組みは未確認。
- #{{127}}(内蔵プラグインを 1 本ずつ確かめる道具)が、ミキサーの FX スロットから、プラグインの名前で足す経路を作る。この Issue は、その経路で plugin の窓を開く。
- 道具 `tools/feat-c1-hidpi/measure-size.mjs` の plugin の行は、`pluginScalePercent`(その窓が出した拡大率。整数の %)を出す欄を持つが、まだ測っていない(null)。
- FL の仕様(General settings の Display の節。実機では未確認): ネイティブプラグインの GUI は、100%・200%・300%・400% の整数だけ(Legacy scaling)。この Issue の判定は、窓の拡大率が FL の採用段と一致することを確かめる。整数だけの仕様との関係は、報告に実測と推定を分けて書く。

## 読む物

- #{{127}} の成果物(ミキサーに名前で足す経路)
- `tools/feat-c1-hidpi/measure-size.mjs`(`openWindow`・`windowRow`・`rectEdges` の plugin と tooltip の処理)
- `pm-work/findings/hidpi-fl-scaling.md`(#{{107}} の測定の結果。FL の拡大の決め方と、Debug ログの取り方)
- `tools/feat-c6-popup-menus/`(メニューの操作の参考)

## やること

1. plugin の窓: #{{127}} の経路で Fruity Balance をミキサーのスロットに足し、その窓を開く。`measure-size.mjs` の `openWindow('plugin')` を、その経路に直す(測定の道具の中だけ。製品の既定の起動の動作は変えない)。
2. plugin の窓の拡大率: 窓の幅を、DPR 1 の同じ窓と比べ、`pluginScalePercent` = round(100 × 幅 ÷ DPR 1 の幅) として出す。全ての段で、FL の採用段(`debugLog.scalingFactor` の factor × 100)と一致すること。一致しなければ、測り方を直す。
3. tooltip の窓: 出す手順を見つける(ヒントの表示の条件を、guest の仕組みから調べる)。出せたら、その窓の縁(`edgeWidth`)を、#{{108}} と同じ方式で測る。出せない場合は、調べた範囲と理由を報告に書く。
4. Tempo 枠の検出の理由の調査(#{{108}} の判定から外した): 乱数の DPR と、動的の段 2・3 で、Tempo 枠の検出が外れる理由を、測定の値から確かめる。直すかどうかは、報告で決める。
5. 製品に新しい環境変数・URL パラメータ・ビルド変数を足さない。拡大率の数値を runtime・app・seed に書かない。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dprs 1,1.25,1.5,1.75,2,2.5,3 --windows plugin,tooltip --random 3 --random-range 1,3 > "$RUNNER_TEMP/fs.jsonl" && node -e 'const fs = require("fs"); const L = fs.readFileSync(process.argv[1], "utf8").trim().split("\n").map((l) => JSON.parse(l)); const eq = (x, y) => Math.abs(x - y) < 0.0001; const R = L.length ? L[0].randomDprs.values : []; const F = [1, 1.25, 1.5, 1.75, 2, 2.5, 3].concat(R); const adp = (x) => Math.floor(x.dpi / 96 * 4 + 1e-9) * 25; const rowOk = (x) => x.mode === "static" && x.state === "engine-idle" && x.standIn === false && x.error === null && x.debugLog.captured === true && x.debugLog.lines >= 1; const plugOk = (x) => x.window !== "plugin" || x.pluginScalePercent === adp(x); const checks = { rows: L.length === 2 * F.length && R.length === 3 && R.every((d) => d >= 1 && d <= 3), dprs: F.every((d) => ["plugin", "tooltip"].every((w) => L.some((x) => eq(x.dpr, d) && x.window === w))), rows_ok: L.every(rowOk), plugin_scale: L.filter((x) => x.window === "plugin").every(plugOk), }; for (const k of Object.keys(checks)) if (!checks[k]) console.log(`judge: 失敗 ${k}`); const bad = L.filter((x) => !rowOk(x) || !plugOk(x)).map((x) => `${x.window}@${x.dpr}: ${x.error ? x.error.slice(0, 80) : "拡大率 " + x.pluginScalePercent}`); if (bad.length) console.log(`judge: 外れた行: ${bad.join("; ")}`); const ok = Object.values(checks).every(Boolean); console.log(ok ? "judge: 通った" : "judge: 通らない"); process.exit(ok ? 0 : 1);' "$RUNNER_TEMP/fs.jsonl" && b=$(git merge-base HEAD origin/main) && d=$(git diff -U0 "$b" -- lift/runtime lift/harness app/src) && ! grep -E '^\+[^+]' <<<"$d" | grep -qE '\b(125|150|168|169|175|176)\b|1\.75\b|1\.7647|0\.5667' && npm test --prefix app
```

判定の基準:

- 全ての DPR(固定の 7 点と `randomDprs.values` の 3 点)に、plugin と tooltip の行が在る(20 行)。
- 全ての行で、mode が static、state が engine-idle、standIn が false、`error` が null、debugLog が captured(lines が 1 以上)。
- plugin の行の `pluginScalePercent` が、その行の FL の採用段(DPI ÷ 96 × 4 を切り捨てて 25 を掛けた値)と一致する。
- ソースの検査(受け入れの最後の段): `git merge-base HEAD origin/main` からの差分の、`lift/runtime`・`lift/harness`・`app/src` の追加行に、特定の拡大率・DPI・DPR の値が、拡大率・DPI・DPR の意味で現れない(一致 0 件)。
- 単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: plugin の窓を開けた経路と理由、tooltip を出せた手順、出せなかった場合は調べた範囲、拡大率の表(DPR・DPI・採用段・`pluginScalePercent`・tooltip の `edgeWidth`)、整数だけの仕様との関係、Tempo 枠の検出の理由(項目 4)、確認できなかった Windows の実際の挙動。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 特定の拡大率・DPI・DPR の値を、runtime・app・seed・設定・テストに、定数・表・分岐として入れない。
- 製品の既定の起動の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。Python は製品に使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 受け入れ 1 回は約 90 分(engine の作り直し約 35 分、測定約 50 分)。判定の段で、もう一度走る。
- 長いコマンド(ビルド・受け入れ)は、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- 開始から 2 時間 30 分を過ぎたら、受け入れを新たに始めない。開始から 4 時間 00 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit して、報告を書いて終える。

## 他のタスクとの関係

- blocked_by: #{{127}}(ミキサーに名前で足す経路)。
- #{{108}}(Fractional Scaling の動的な変化と全ての拡大率。この Issue の判定の外の窓を受ける)。#{{108}} と同じ測定の道具(`tools/feat-c1-hidpi/measure-size.mjs`)を変えるので、#{{108}} が main に入る前に始めると、衝突しうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
