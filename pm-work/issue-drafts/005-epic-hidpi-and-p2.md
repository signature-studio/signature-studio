---
draft: "005"
title: "E HiDPI と P2 の測り方(画面がぼやけない。UI の 120 fps)"
type: epic
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: null
blocked_by: []
labels: ["epic", "hidpi", "perf", "priority-p2"]
size: "子の合計"
needs_owner: false
---

## 目的

台帳 L-033 の (1): HiDPI に対応する(画面がぼやけている)。併せて P2(UI が 120 fps で動く)を測る道具を作る。

## 背景

- 旧の測定(pm-work/findings/survey-c1-hidpi.md。runner の値): canvas の描画バッファは guest 画面の画素数で、CSS の大きさと 1:1 だった。拡大率 2 では、ブラウザが canvas を 2 倍に引き伸ばしていた(式の値 0.5)。`devicePixelRatio` は表示と記録にだけ使われ、描画の大きさには掛けられていなかった。
- P2 の定義(旧 ci タスク 0004 の記述による。定義元の `docs/ops/fidelity-and-performance.md` は、リポジトリに無い。QUESTIONS 6): UI が 120 fps で動き、入力から画面の p95 が 17 ms 以下。旧記録では、この機械の rAF は 60 Hz だった(QUESTIONS 7)。
- 依存の設計: HiDPI の実装(006)は、描画の分離(002)を待たない。両方は別の箇所を変えるので、並行に作業できる。描画の負荷の影響は、両方が揃った後の 008 で測る。これは、台帳 L-039 の「依存は、先に終わっていないと作業できない物だけ」に従う判断で、旧の計画の「C-2 の後に行う」の順序を、測定の段に移した。

## 完了の条件

- 子 006: 拡大率 1・1.5・2 で、canvas の幅と高さが CSS × 拡大率(±1 画素)に一致し、guest に DPI が伝わる。
- 子 007: P2 の道具が、フレーム間隔と入力の遅延を JSON で出す。
- 子 008: HiDPI と描画の分離の後の、描画の所要と P2 の値が「受理」「差し戻し」「未確認」のいずれかで判定されている。

## Sub-Issue の一覧

- 006 HiDPI: guest の画面を物理画素にし、DPI を伝える(blocked_by なし)
- 007 P2 の測定の道具(blocked_by なし)
- 008 HiDPI と描画の分離の後の実測(blocked_by 006, 002, 007)

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない(L-002)。既存の `FLE_MONITOR_W`・`FLE_MONITOR_H` は値を変えて使ってよいが、名前は増やさない。
- ベンダー名と Cloudflare の識別子をファイルに書かない(L-005・L-006)。
