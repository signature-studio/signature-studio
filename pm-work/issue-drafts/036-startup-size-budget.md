---
draft: "036"
title: "起動前に取る量の予算と、サイズの目標を依頼者が決める(数値の確定)"
type: task
milestone: "M8 起動 10 秒以内とサイズ(P1)"
parent: "033"
blocked_by: []
labels: ["task", "startup", "needs-owner"]
size: 3
needs_owner: true
---

## 目的

起動前に取る量の上限(予算)と、サイズの目標値を、依頼者が決める。決まった値を、035 と 038 の判定の基準にする。この Issue は、数値を決める作業であり、製品は変えない。

## 規模

規模: 大(想定 3 時間。測定の表と、依頼者への確認の材料を作る)

## 背景

- 起動前に取る物の現在の大きさ(runner の値。pm-work/findings/survey-c2-worker-split.md・survey-c3-fl-resources.md): engine wasm 167,414,828 B(br 17,717,323 B)、seed pack 20,780,949 B、DLL pack 88,675,800 B(11 ファイル)。合計(非圧縮)は 277,000,000 B 程度。
- 配信は br 圧縮で返る(`app/worker/index.ts` の 2 行目の注釈: 保存された Content-Encoding で返す)。圧縮の後の大きさを、予算の基準にするかは依頼者が決める。
- P1 は時間。予算は、時間の目標(10 秒)を満たすための、取得量の上限として使う。

## 完了の条件

- 依頼者が、次の数値を決める(QUESTIONS 8): (a) 起動前に取る量の上限(圧縮後と非圧縮後のどちらか。バイト)、(b) engine wasm の大きさの目標(バイト)。
- 決まった値を、`inputs/startup-budget.json` に書く(キー: `preStartBytesMax`・`engineWasmBytesMax`・`basis`)。

## 読む物

- `app/worker/index.ts`(2 行目の注釈: 配信の仕方)
- `app/src/bundle/manifest.ts`(73〜75 行: seedPack と dllPack)
- `pm-work/findings/survey-c2-worker-split.md`(手順 2: 大きさ)

## やること

1. 依頼者の判断(QUESTIONS 8)を待つ。待っている間は、起動前の取得の大きさの表を作り、報告に書く(圧縮の有無ごと)。
2. 依頼者が決めた値を、`inputs/startup-budget.json` に書く。
3. 決まった値で、035 の判定が通るかの見込みを、報告に書く(推定)。

## 受け入れ

```accept
node -e 'const b=JSON.parse(require("fs").readFileSync("inputs/startup-budget.json","utf8"));process.exit(typeof b.preStartBytesMax==="number"&&typeof b.engineWasmBytesMax==="number"&&typeof b.basis==="string"?0:1)'
```

判定の基準: `inputs/startup-budget.json` が、数値と基準の語を持つこと。数値は依頼者が決める。

## 報告の規則

- 1 行目は「実エンジン: 実行なし(数値の決定と、表の作成のみ)」と書く。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 依頼者の判断が出るまでは、「未確認(依頼者の数値が無い)」と書く。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない(L-002)。
- ベンダー名と Cloudflare の識別子を書かない(L-005・L-006)。
- 依頼者が言っていない制限を、この Issue で作らない。数値は依頼者が決める。

## 他のタスクとの関係

- 035(起動を 10 秒以内にする)と 038(完成の検証)が、この Issue の数値を使う。
