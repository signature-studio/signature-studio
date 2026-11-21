---
draft: "022"
title: "E 内蔵インストゥルメント(Generators。約 51 件。束ねた Sub-Issue は WAVES.md の第 2 波で起票)"
type: epic
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: ["020"]
labels: ["epic", "plugin-builtin", "priority-p1"]
size: "子の合計"
---

## 目的

依頼者の指示: `Plugins\Fruity\Generators` の内蔵のインストゥルメント(約 51 件)を、1 プラグイン 1 wasm で起動後に届ける。OpenGL を使う VFX 系(023 の一覧)は除く。

## 背景

- 件数の出どころ: 依頼者の申告(約 51 件。手元の版の数)。インストーラの一覧(013)の実数で確かめる。
- 現状: 抽出の表に Generators の DLL は 0 件(pm-work/findings/survey-c4-builtin-plugins.md の手順 2)。seed の表には `GM CCs.txt` の 1 行だけ(データで、DLL ではない)。このため、表の追加(020)が前提になる。
- 型: 015 と 019 の届け方を使う。

## 完了の条件

- 全件(023 の VFX 系を除く)が、起動確認(engine-idle・standIn が false・error が null)を通る。
- 全件が、起動後の取得で届く。
- 群ごとの Sub-Issue が、この Epic の下に起票され、各群が完了している。

## Sub-Issue の一覧

- (起票待ち。WAVES.md の第 2 波で PM が 3 件ずつ束ねて起票する)

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- 1 タスク 1 関数に戻さない。同じ内容のタスクを機械的に量産しない。
- ベンダー名と Cloudflare の識別子を書かない。
