---
draft: "005"
title: "E HiDPI と P2 の測り方(画面がぼやけない。UI の 120 fps)"
type: Feature
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: null
blocked_by: []
labels: ["hidpi", "perf"]
size: M
priority: High
project: signature-studio/3
---

## 目的

課題 C-1: HiDPI に対応する(画面がぼやけている)。併せて P2(UI が 120 fps で動く)を測る道具を作る。

## 背景

- 旧の測定(pm-work/findings/survey-c1-hidpi.md。runner の値): canvas の描画バッファは guest 画面の画素数で、CSS の大きさと 1:1 だった。拡大率 2 では、ブラウザが canvas を 2 倍に引き伸ばしていた(式の値 0.5)。`devicePixelRatio` は表示と記録にだけ使われ、描画の大きさには掛けられていなかった。
- P2 の定義(旧 ci タスク 0004 の記述による): UI が 120 fps で動き、入力から画面の p95 が 17 ms 以下。出典の文書 `docs/ops/fidelity-and-performance.md` は、リポジトリに無い。旧記録では、この機械の表示は 60 Hz だった。120 fps の測定は、表示の上限に縛られないよう、headless Chrome のフレーム数の上限を外す起動の引数で行う(引数は作業者が選び、報告に書く)。
- 依存の設計: HiDPI の実装(#{{006}})は、描画の分離の測定(#{{002}})を待たない。両方は別の箇所を変えるので、並行に作業できる。依存は、先に終わっていないと作業できない物だけにする。描画の負荷の影響は、#{{006}} と #{{002}} の後に #{{008}} で測る。

## 完了の条件

- #{{006}}: 拡大率 1・1.5・2 で、canvas の幅と高さが CSS × 拡大率(±1 画素)に一致し、guest に DPI が伝わる。
- #{{007}}: P2 の道具が、フレーム間隔と入力の遅延を JSON で出す。
- #{{008}}: HiDPI と描画の分離の後の、描画の所要と P2 の値が「受理」「差し戻し」「未確認」のいずれかで判定されている。

## Sub-Issue の一覧

- #{{006}} HiDPI: guest の画面を物理画素にし、DPI を伝える(blocked_by なし)
- #{{007}} P2 の測定の道具(blocked_by なし)
- #{{008}} HiDPI と描画の分離の後の実測(blocked_by #{{006}}, #{{002}}, #{{007}})

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。既存の `FLE_MONITOR_W`・`FLE_MONITOR_H` は値を変えて使ってよいが、名前は増やさない。
- ベンダー名と Cloudflare の識別子をファイルに書かない。
