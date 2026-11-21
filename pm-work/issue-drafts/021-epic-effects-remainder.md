---
draft: "021"
title: "E 内蔵 Effects の残り(約 87 件。群の Sub-Issue は 020 の後に起票)"
type: epic
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: ["020"]
labels: ["epic", "plugin-builtin", "priority-p1"]
size: "子の合計"
needs_owner: false
---

## 目的

台帳 L-034: `Plugins\Fruity\Effects` の内蔵のエフェクト(約 90 件)のうち、019 で扱う 2 件を除いた残り(約 87 件)を、1 プラグイン 1 wasm で起動後に届ける。OpenGL を使う VFX 系(023 の一覧)は除く。

## 背景

- 件数の出どころ: 依頼者の申告(約 90 件。手元の版の数で、lift 対象の版とは ±2 ほど違いうる)。インストーラの一覧(013)の実数で確かめる。
- 群の分け方: 子の Sub-Issue は、020 の後に、監査・受付が一覧(013)の中身から群に分けて起票する。群の内容(名前・import・hook 点・大きさ)が違うように書く(台帳 L-038)。
- 型: 015(Fruity Balance)と 019(Filter・Delay 2)で、1 プラグイン 1 wasm の起動後の届け方を確かめる。

## 完了の条件

- 残りの全件(023 の VFX 系を除く)が、起動確認(engine-idle・standIn が false・error が null)を通る。
- 全件が、起動後の取得で届く。
- 群ごとの Sub-Issue が、この Epic の下に起票され、各群が完了している。

## Sub-Issue の一覧

- (起票待ち。020 の完了後に、監査・受付が、群ごとに Sub-Issue を起票する)

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない(L-002)。
- 1 タスク 1 関数に戻さない(L-037)。群のタスクは、利用者から見える動作、または設計課題を終える大きさにする。
- 同じ内容のタスクを機械的に量産しない(L-038)。
