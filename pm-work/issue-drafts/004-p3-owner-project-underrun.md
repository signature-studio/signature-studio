---
draft: "004"
title: "所定のプロジェクトで P3(Song Mode の underrun)を測る"
type: task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: ["002", "003"]
labels: ["task", "audio", "verify", "priority-p3", "needs-owner"]
size: 3
needs_owner: true
---

## 目的

台帳 L-033 の (2) の完成の条件: 所定のプロジェクトを Song Mode で再生して、音が途切れない(P3)。underrun が 0 であることを、runner で n=3 で実測する。最優先の P3 の判定。

## 規模

規模: 大(想定 3 時間。実測の 3 回が、それぞれ数分の再生を含む)

## 背景

- P3 の入力は、依頼者の個人のプロジェクトファイル(所定のプロジェクト)である。runner に無い。取得の経路は未確定(QUESTIONS 1)。このため、この Issue は、依頼者が入力を用意してから起動する(label `needs-owner`)。
- 定義(旧 ci タスク 0005 の記述による。P3 の定義元の `docs/ops/fidelity-and-performance.md` は、リポジトリに無い。QUESTIONS 6): 所定のプロジェクトを Song Mode で再生し、AudioWorklet のリングの枯渇(underrun)が 0。再生時間と消費フレーム数の比を見る。
- 台本の候補: `tools/verify-kit/tools/scn-p3-mixmove-300.json`(5 分の台本。マウス移動と Mixer 表示を含む)。既存の配信の道具: `tools/verify-kit/serve-once-p3-bundle.mjs`、`tools/verify-kit/tools/r635-run.sh`。
- 前提: 子 002(描画の分離の受理)と子 003(音声の実測の道具)が終わっている。

## 読む物

- `tools/p3-audio-probe/measure.mjs`(子 003 が作る道具)
- `tools/verify-kit/serve-once-p3-bundle.mjs`(bundle の配信)
- `tools/verify-kit/tools/r635-run.sh`(1 回の列。使い方は先頭のコメント)
- `tools/verify-kit/tools/scn-p3-mixmove-300.json`(台本)
- `app/src/audio/page-audio.ts`(42〜43 行: 音声の状態の項目)

## やること

1. 入力が runner にあることを確かめる(`test -f` の結果を報告に書く)。無ければ、ここで止めて「実エンジン: 実行なし(入力の所定プロジェクトが runner に無い)」と報告する。
2. 台本が所定のプロジェクトを開くかを、台本の該当行で確かめて報告に書く。
3. 同じ台本と同じ入力で、n=3 を実行する。各回の道具の JSON の 1 行を、そのまま報告に貼る。
4. underrun の合計を、各回で出す。0 でない回は、その回の数と時刻の範囲を書く。
5. 耳の確認は行わない。「音が途切れない」とは書かず、underrun の数で「受理」「差し戻し」「未確認」を判定する。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && for i in 1 2 3; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$RUNNER_TEMP/owner-project.flp" > "$RUNNER_TEMP/owner-p3-$i.json" || exit 1; done && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/owner-p3-"+i+".json","utf8").trim()));process.exit(L.every((r)=>r.error===null&&r.quantaWhilePlaying>0&&r.underrunQuanta===0)?0:1)'
```

判定の基準: 3 回すべてで、`error` が null、再生の quantum が 1 以上、`underrunQuanta` が 0。

依存(入力): この受け入れは、依頼者が所定のプロジェクトを R2 に置くことに依存する。受け入れの `$RUNNER_TEMP/owner-project.flp` は、その取得の段が用意する物で、取得の経路は QUESTIONS 1 で依頼者が決める。入力が runner に無い場合は、受け入れを実行せず、「実エンジン: 実行なし(入力が R2 に無い)」と報告して止まる。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」「音が途切れない」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 入力が無い場合は、「実エンジン: 実行なし(理由)」と書き、止まる。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない(L-002)。
- 依頼者の個人のプロジェクトを、リポジトリに入れない。取得の経路は依頼者の判断(QUESTIONS 1)。
- Python を製品に使わない(L-008)。検証の集計の道具は、既存のものを使う。
- ベンダー名と Cloudflare の識別子を書かない(L-005・L-006)。
- 作業者に依頼者の個人の資格を置かない(L-011)。

## 他のタスクとの関係

- 002(描画の分離の受理)と 003(音声の道具)が blocked_by。
- 008(HiDPI の後の実測)とは、同じ道具を使うが、互いに待たない。
