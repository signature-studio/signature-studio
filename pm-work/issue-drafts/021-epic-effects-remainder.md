---
draft: "021"
title: "E 内蔵 Effects の残り(約 87 件。束ねた Sub-Issue は WAVES.md の第 2 波で起票)"
type: Feature
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: ["020"]
labels: ["plugin-builtin"]
size: M
priority: Medium
project: signature-studio/3
---

## 目的

依頼者の指示: `Plugins\Fruity\Effects` の内蔵のエフェクト(約 90 件)のうち、#{{019}} で扱う 2 件を除いた残り(約 87 件)を、1 プラグイン 1 wasm で起動後に届ける。OpenGL を使う VFX 系(#{{023}} の一覧)は除く。

## 背景

- 件数の出どころ: 依頼者の申告(約 90 件。手元の版の数で、lift 対象の版とは ±2 ほど違いうる)。インストーラの一覧(#{{013}})の実数で確かめる。
- 群の分け方: 子の Sub-Issue は、WAVES.md の第 2 波で、PM が一覧(#{{013}})の中身から 3 件ずつ束ねて起票する。各 Issue の内容(名前・import・hook 点・大きさ)は、束ごとに違うように書く。
- 型: #{{015}}(Fruity Balance)と #{{019}}(Filter・Delay 2)で、1 プラグイン 1 wasm の起動後の届け方を確かめる。

## 完了の条件

- 残りの全件(#{{023}} の VFX 系を除く)が、起動確認(engine-idle・standIn が false・error が null)を通る。
- 全件が、起動後の取得で届く。
- 群ごとの Sub-Issue が、この Epic の下に起票され、各群が完了している。

## Sub-Issue の一覧

- (起票待ち。WAVES.md の第 2 波で PM が 3 件ずつ束ねて起票する)

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- 1 タスク 1 関数に戻さない。群のタスクは、利用者から見える動作、または設計課題を終える大きさにする。
- 同じ内容のタスクを機械的に量産しない。
