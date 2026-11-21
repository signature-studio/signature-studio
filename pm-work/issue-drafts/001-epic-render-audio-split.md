---
draft: "001"
title: "E 描画と音声の分離(P3)"
type: epic
milestone: "M1 描画と音声の分離(P3)"
parent: null
blocked_by: []
labels: ["epic", "render", "priority-p3"]
size: "子の合計"
needs_owner: false
---

## 目的

依頼者の言葉: 「画面の描画処理が音声処理を圧迫している。Worker 分離は完了していないのか」(台帳 L-033 の (2)。優先は P3: Song Mode で音が途切れない)。描画の合成と提示を、engine と音声の書き込みを行う Worker から分け、所定のプロジェクトで P3 を実測する。

## 背景

- 旧の測定(pm-work/findings/survey-c2-worker-split.md。Worker 分離の前の版で測った。runner の値)では、描画の合成と提示が engine と同じ Worker で動いていた。画面が動かない間も、compose が 36〜48 回/秒走っていた。
- その後、commit 43a6eb88 で描画の Worker(render-worker.ts)を分けた。受理の記録は main に無い。子 002 で受理を判定する。
- P3 の underrun は、所定のプロジェクト(依頼者の個人のファイル)が runner に無いので、まだ測っていない。

## 完了の条件

- 子 002: 描画の Worker の分離が、実測で確かめられ、受理または差し戻しの判定がついている。差し戻しなら、直して受理にする。
- 子 003: P3 の音声の実測の道具があり、AudioContext を開始して underrun と再生の quantum を数えられる。
- 子 004: 所定のプロジェクトを Song Mode で再生し、underrun が 0(n=3)。入力は依頼者が用意する(QUESTIONS 1)。

## Sub-Issue の一覧

- 002 C-2 の受理を確かめ、残りを直す(blocked_by なし)
- 003 P3 の音声の実測の道具(blocked_by なし)
- 004 所定のプロジェクトで P3 の underrun を測る(blocked_by 002, 003。needs-owner)

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。オプトインを作らない(台帳 L-002)。
- ベンダー名と Cloudflare の識別子をファイルに書かない(L-005・L-006)。
- 作業者に依頼者の個人の資格を置かない(L-011)。
