---
draft: "004"
title: "インストーラの demo プロジェクトで P3(Song Mode の underrun)を測る"
type: task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: ["003", "040"]
labels: ["task", "audio", "verify", "priority-p3"]
size: 3
---

## 目的

P3 の完成の条件: プロジェクトを Song Mode で再生して、音が途切れない。underrun が 0 であることを、runner で n=3 で実測する。この実測は、子 040(音声への圧迫の解消)の後に行う。

## 規模

規模: 大(想定 3 時間。実測の 3 回が、それぞれ数分の再生を含む)

## 背景

- 入力のプロジェクト: インストーラに含まれる demo・テンプレートのプロジェクト(`.flp`)のうち、Song Mode の playlist を持つ物を、作業者が選ぶ。依頼者の個人のファイルは使わない。
- 選ぶ元は、子 013 の成果物 `inputs/fl-installer-files.tsv`(インストーラ全件の一覧)。`Data\` の下の `.flp` を候補にする。playlist の件数で、Song Mode の再生があるかを確かめる。
- 選んだ 1 件を `inputs/p3-project.tsv` に書いて commit する(列: `guest`・`bytes`・`sha256`・`playlist`。見出し行を 1 行残す)。インストール先の既定の値は書かない。
- 実行に使うファイルは、インストーラの展開の木から取り、`.tmp-p3-project.flp` に置く。受け入れの後に消す。リポジトリには入れない。展開の木に無い場合は、ここで止めて報告する。
- 定義(旧 ci タスク 0005 の記述による。出典の文書 docs/ops/fidelity-and-performance.md は、リポジトリに無い): 所定のプロジェクトを Song Mode で再生し、AudioWorklet のリングの枯渇(underrun)が 0。
- 台本: `tools/verify-kit/tools/scn-p3-mixmove-300.json` を参考にしてよい。ただし、Song Mode の再生は子 003 の道具の中の手順で行う。
- 前提: 子 003(音声の道具)と子 040(音声への圧迫の解消)が終わっている。

## 読む物

- `tools/p3-audio-probe/measure.mjs`(子 003 が作る道具。プロジェクトを開いて Song Mode で再生する手順)
- `inputs/fl-installer-files.tsv`(子 013 の成果物)
- `app/src/audio/page-audio.ts`(42〜43 行: 音声の状態の項目)

## やること

1. `inputs/fl-installer-files.tsv` から、demo・テンプレートの `.flp` の候補を出す。Song Mode の playlist を持つ 1 件を選ぶ。選んだ理由(playlist の件数)を報告に書く。
2. 選んだ 1 件を展開の木から `.tmp-p3-project.flp` に置き、その guest パス・大きさ・SHA-256 を `inputs/p3-project.tsv` に書いて commit する。
3. 同じ道具で、n=3 を実行する。各回の道具の JSON の 1 行を、そのまま報告に貼る。
4. underrun の合計を各回で出す。0 でない回は、その回の数と時刻の範囲を書く。
5. 耳の確認は行わない。「音が途切れない」とは書かず、underrun の数で「受理」「差し戻し」「未確認」を判定する。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && test -s .tmp-p3-project.flp && test -s inputs/p3-project.tsv && for i in 1 2 3; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-p3-project.flp" > "$RUNNER_TEMP/p3-$i.json" || exit 1; done && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/p3-"+i+".json","utf8").trim()));process.exit(L.every((r)=>r.error===null&&r.quantaWhilePlaying>0&&r.underrunQuanta===0)?0:1)'
```

判定の基準: 3 回すべてで、`error` が null、再生の quantum が 1 以上、`underrunQuanta` が 0。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」「音が途切れない」とは書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 展開の木に選んだ 1 件が無い場合は、「実エンジン: 実行なし(選んだ物が展開の木に無い)」と書いて止まる。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- 依頼者の個人のプロジェクトは無い。インストーラの demo を使う。展開したプロジェクトは、リポジトリに入れない(`.tmp-*` に置き、消す)。
- ベンダー名と Cloudflare の識別子を書かない。インストール先の既定の値を書かない。

## 他のタスクとの関係

- blocked_by: 003(道具)と 040(圧迫の解消)。
- 031(P3 の再生中の命令の内訳)は、この Issue が commit する `inputs/p3-project.tsv` を使う。
- 008(HiDPI の後の実測)と 038(完成の検証)が、この Issue の結果を使う。
