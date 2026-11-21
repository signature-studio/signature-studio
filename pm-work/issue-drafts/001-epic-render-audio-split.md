---
draft: "001"
title: "E 描画と音声の分離(P3)"
type: Feature
milestone: "M1 描画と音声の分離(P3)"
parent: null
blocked_by: []
labels: ["render"]
size: L
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の課題 C-2: 「画面の描画処理が音声処理を圧迫している。Worker 分離は完了していないのか」。優先は P3(Song Mode で音が途切れない)。描画の合成と提示を、engine と音声の書き込みを行う Worker から分けた効果を実測し、残る圧迫を取り除き、所定のプロジェクトで P3 を実測する。

## 背景

- 旧の測定(pm-work/findings/survey-c2-worker-split.md。分離の前の版。runner の値)では、描画の合成と提示が engine と同じ Worker で動いていた。画面が動かない間も、compose が 36〜48 回/秒走っていた。
- 分離の commit 43a6eb88 は main に在る。しかし本番には出ていない。最後に成功した deploy(run 44869716216。head 7a0c75ef)は、この commit を含まない。その後の deploy は、1 件が `Set up job` で失敗し、他は取り消された。依頼者が本番で見た圧迫は、分離の前の版のものである。
- 分離の後の版で圧迫が消えたかは、まだ測っていない。このため、#{{002}} で前後を同じ道具で測り、#{{040}} で残る経路を取り除く。
- P3 の入力は、インストーラに含まれる demo・テンプレートのプロジェクトのうち、Song Mode の playlist を持つ物を、作業者が一覧(#{{013}} の成果物)から選ぶ(#{{004}})。
- 定義(旧 ci タスク 0005 の記述による。出典の文書 docs/ops/fidelity-and-performance.md は、リポジトリに無い): Song Mode の再生で、AudioWorklet のリングの枯渇(underrun)が 0。

## 完了の条件

- #{{002}}: 分離の前後の版を、同じ道具と同じ負荷で測り、音声の余裕の差を数字で報告する。
- #{{003}}: P3 の音声の実測の道具があり、AudioContext を開始して underrun と再生の quantum を数えられる。
- #{{040}}: UI の負荷をかけても、underrun が 0(n=3)。起動・engine-idle・standIn が false・error が null。
- #{{004}}: 所定のプロジェクト(#{{004}} で選ぶ demo・テンプレート)を Song Mode で再生し、underrun が 0(n=3)。

## Sub-Issue の一覧

- #{{003}} P3 の音声の実測の道具(blocked_by なし)
- #{{002}} 分離の前後の音声の余裕を、同じ道具で測る(blocked_by #{{003}})
- #{{040}} 描画が音声を圧迫する経路を取り除く(blocked_by #{{002}})
- #{{004}} 所定のプロジェクトで P3 の underrun を測る(blocked_by #{{003}}, #{{040}})

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- ベンダー名と Cloudflare の識別子をファイルに書かない。
- 作業者に依頼者の個人の資格を置かない。
