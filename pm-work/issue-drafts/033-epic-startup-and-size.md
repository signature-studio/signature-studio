---
draft: "033"
title: "E 起動を 10 秒以内にする(P1)。単一の wasm は 1GB 以下にする(サイズ)"
type: epic
milestone: "M8 起動 10 秒以内とサイズ(P1)"
parent: null
blocked_by: []
labels: ["epic", "startup", "priority-p1"]
size: "子の合計"
---

## 目的

P1: 起動を 10 秒以内にする(ページを開いてから engine-idle まで)。サイズは、P1 の次の優先(AGENTS.md の順: P3、P2、P1、サイズ)。サイズの方針は、単一の wasm ファイルがブラウザの読み込める上限(約 1GB)に収まること。

## 背景

- 旧の測定(runner の値。出典は pm-work/findings/)。engine-idle の時刻(boot-bundle の `timelineIdle`): 9,260.8〜9,401.5 ms(survey-c2 の 4 回)、8,748.7 ms(survey-c3 の 1 回)。navigate から engine-idle の確認まで(poll を含む): 10,205〜10,584 ms(survey-c1)。probe を含む測り方の 12,330〜12,849 ms(survey-c6。P1 の測り方とは違う)。測り方が違うので、この Epic では、測り方を先に決める(子 034)。
- 起動前に取る物(runner の値。survey-c2・c3): engine wasm 167,414,828 B(br 17,717,323 B)、seed pack 20,780,949 B、DLL pack 88,675,800 B(11 ファイル)。
- P1 の定義(旧 ci タスク 0003 の記述による): ページを開いてから、エンジンが最初の idle に達するまで。10 秒以内。初回とキャッシュ後を分ける。出典の文書 `docs/ops/fidelity-and-performance.md` は、リポジトリに無い。
- サイズの方針(依頼者の指摘): engine wasm の大きさの目標は無い。WebAssembly は単一の wasm ファイルの上限を定めていない。強いて言えば、ブラウザが読み込める約 1GB あたり。それを超える場合は分割する。起動前に取る量は、P1 の実測で決める。

## 完了の条件

- 子 034: 起動の内訳(取得・コンパイル・インスタンス化・起動)を実測し、P1 の測り方を確定する。
- 子 035: 起動の中央値(n=3)が 10 秒以内。
- 単一の wasm ファイルが 1GB 以下である(分割が必要な物は分割されている)。この条件は、035 の報告で確かめる。

## Sub-Issue の一覧

- 034 起動の内訳を実測し、P1 の測り方を決める(blocked_by なし)
- 035 起動を 10 秒以内にする(blocked_by 034, 014)

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- ベンダー名と Cloudflare の識別子を書かない。
- 製品の既定の起動の動作を、測定のためだけに変えない。
