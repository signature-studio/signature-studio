---
draft: "018"
title: "E 内蔵のエフェクトとインストゥルメント(DLL で 100 以上。OpenGL の VFX 系は後で入れる)"
type: Feature
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: null
blocked_by: []
labels: ["plugin-builtin"]
size: L
priority: High
project: signature-studio/3
---

## 目的

課題 C-4: 100 以上の内蔵のエフェクトとインストゥルメントを存在させる。数えるのは DLL のファイル数(決定)。依頼者の申告の内訳は、内蔵のエフェクトが `Plugins\Fruity\Effects` に約 90 件、インストゥルメントが `Plugins\Fruity\Generators` に約 51 件(依頼者の手元の版の数。lift 対象の版とは ±2 ほど違いうる)。OpenGL を使う VFX 系は、後回しにし、全群の後に入れる(子 039)。

## 背景

- 進み具合(実測。lift の対象の表の数): 内蔵は 3 件だけ(Fruity Balance・Fruity Filter・Fruity Delay 2)。`lift/harness/b2d2/modules.tsv` の 8〜10 行。インストゥルメントは 0 件。
- 抽出の木の DLL は 14 件(FLEngine・エンジンのライブラリ 7・Fruity 3 件 × 2 箇所)。インストーラの全件の数は、子 013 の一覧で確かめる。
- 直接 wasm の関数の割合は、この Epic の範囲外(M7)。
- 1 プラグインの追加の手数(推定。pm-work/findings/survey-c4-builtin-plugins.md の手順 7・8): 表の行(`ci/extract-dlls.tsv`、`lift/harness/b2d2/modules.tsv`、seed の行、`lift/harness/web/pack.args` の `--args`)と、hook 点(Delphi の DLL で必要な場合)。履歴からは測れない。
- 1 プラグインは、1 wasm で起動後に届ける。最初の 1 件は Fruity Balance(子 015)。
- 子の起票: 内蔵の全件は、WAVES.md の第 2 波で、3 件ずつ束ねた Issue として起票する(約 47 件)。021 と 022 は、その束を入れる群の Epic である。

## 完了の条件

- 子 019: Fruity Filter と Fruity Delay 2 が、起動後の取得で届く。
- 子 020: 内蔵の全 DLL の抽出の表が、インストーラの一覧(013)の SHA-256 と一致して揃っている。
- 子 021・022: 内蔵の群(Effects と Generators)の全件が、起動確認(engine-idle・standIn が false・error が null)を通り、起動後の取得で届く(OpenGL の VFX 系を除く)。
- 子 023: OpenGL を使う VFX 系の一覧と、後回しの理由が報告に出ている。
- 子 039: 後回しの VFX 系を、全群の後に 1 プラグイン 1 wasm で入れる。

## Sub-Issue の一覧

- 019 Fruity Filter と Fruity Delay 2 を、起動後の取得で届ける(blocked_by 015)
- 020 抽出の対象を内蔵の全 DLL に広げ、表を揃える(blocked_by 013, 023)
- 021 内蔵 Effects の残り(Epic。束ねた Issue は WAVES.md の第 2 波で起票する)
- 022 内蔵 Generators(Epic。束ねた Issue は WAVES.md の第 2 波で起票する)
- 023 OpenGL を使う VFX 系の判定と、後回しの一覧(blocked_by 013)
- 039 VFX 系を全群の後に 1 プラグイン 1 wasm で入れる(blocked_by 021, 022, 023)

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。内蔵の数を、オプションで増減させない。
- 1 タスク 1 関数に戻さない。タスクは、利用者から見える動作 1 つ、または設計課題 1 つを終える大きさにする。
- 同じ内容のタスクを機械的に量産しない。束ねた Issue は、一覧の中身(名前・import・hook 点・大きさ)から、内容を書いて起票する。
- ベンダー名と Cloudflare の識別子を書かない。インストール先の既定の値を書かない。
