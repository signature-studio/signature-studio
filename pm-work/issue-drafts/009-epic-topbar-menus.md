---
draft: "009"
title: "E トップバーのコンテキストメニュー(Tempo・Song Position・Pattern Selector)"
type: Feature
milestone: "M3 トップバーのコンテキストメニュー(P2)"
parent: null
blocked_by: []
labels: ["topbar-menu"]
size: M
priority: High
project: signature-studio/3
---

## 目的

課題 C-6: トップバー(Tempo・Song Position・Pattern Selector など)のコンテキストメニューを機能させる。

## 背景

- 旧の測定(pm-work/findings/survey-c6-topbar-menus.md。runner の値): ページに Tempo・Song Position・Pattern Selector は無い(`app/index.html` の `#toolbar` は 3 要素だけ)。部品は FL Studio の画面の中に描かれ、engine の窓の中にある。起動直後の画面では、部品の位置を読めなかった。
- 右クリックは engine まで届く(`WM_CONTEXTMENU`。`lift/runtime/fle_input.c` 975〜992 行)。しかし `TrackPopupMenu` は、何も表示せず、何も送らずに閉じる(`lift/runtime/fle_crash1_menu.c` 600〜618 行。注釈は `lift/runtime/fle_crash1_menu.h` 5 行)。

## 完了の条件

- 子 010: 右クリックでメニューが画面に出て、項目を選ぶと engine が WM_COMMAND を受け取る。
- 子 011: 3 つの部品それぞれで、メニューの項目を選ぶと、部品の値が変わる。

## Sub-Issue の一覧

- 010 右クリックでメニューを表示し、項目の選択を engine に返す(blocked_by なし)
- 011 3 部品のメニューで値が変わる(blocked_by 010)

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- 製品の既定の起動の動作(起動時の確認ダイアログ)を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子をファイルに書かない。
