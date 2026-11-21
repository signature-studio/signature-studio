---
draft: "018"
title: "E 内蔵のエフェクトとインストゥルメント(約 141 件。OpenGL の VFX 系は後回し)"
type: epic
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: null
blocked_by: []
labels: ["epic", "plugin-builtin", "priority-p1"]
size: "子の合計"
needs_owner: false
---

## 目的

台帳 L-033 の (4): 100 以上の内蔵のエフェクトとインストゥルメントを存在させる。台帳 L-034: 内蔵のエフェクトは `Plugins\Fruity\Effects` に約 90 件、インストゥルメントは `Plugins\Fruity\Generators` に約 51 件(依頼者の手元の版の数。lift 対象の版とは ±2 ほど違いうる)。OpenGL を使う VFX 系は後回しにしてよい。それ以外は全て lift の対象。

## 背景

- 進み具合(実測。lift の対象の表の数): 内蔵は 3 件だけ(Fruity Balance・Fruity Filter・Fruity Delay 2)。`lift/harness/b2d2/modules.tsv` の 8〜10 行。インストゥルメントは 0 件。
- 抽出の木の DLL は 14 件(FLEngine・エンジンのライブラリ 7・Fruity 3 件 × 2 箇所)。インストーラの全件の数は、子 013 の一覧で確かめる。
- 直接 wasm の関数の割合は、この Epic の範囲外(M7)。
- 1 プラグインの追加の手数(推定。pm-work/findings/survey-c4-builtin-plugins.md の手順 7・8): 表の行(`ci/extract-dlls.tsv`、`lift/harness/b2d2/modules.tsv`、seed の行、`lift/harness/web/pack.args` の `--args`)と、hook 点(Delphi の DLL で必要な場合)。履歴からは測れない。
- 1 プラグインは、1 wasm で起動後に届ける(台帳 L-035)。最初の 1 件は Fruity Balance(子 015)。

## 完了の条件

- 子 019: Fruity Filter と Fruity Delay 2 が、起動後の取得で届く。
- 子 020: 内蔵の全 DLL の抽出の表が、インストーラの一覧(013)の SHA-256 と一致して揃っている。
- 子 021・022: 内蔵の群(Effects と Generators)が、群ごとの Sub-Issue として起票され、全件が lift・起動確認を通る(OpenGL の VFX 系を除く)。起票は、020 の後に、監査・受付が行う。
- 子 023: OpenGL を使う VFX 系の一覧と、後回しの理由が書かれている。
- 子 039: 後回しの VFX 系の実装。依頼者の判断が出た後に行う(完了の条件の対象は、後回しを解いた場合のみ)。

## Sub-Issue の一覧

- 019 Fruity Filter と Fruity Delay 2 を、起動後の取得で届ける(blocked_by 015)
- 020 抽出の対象を内蔵の全 DLL に広げ、表を揃える(blocked_by 013, 023)
- 021 内蔵 Effects の残り(Epic。群の Sub-Issue は 020 の後に監査・受付が起票する)
- 022 内蔵 Generators(Epic。群の Sub-Issue は 020 の後に監査・受付が起票する)
- 023 OpenGL を使う VFX 系の判定と、後回しの一覧(blocked_by 013)
- 039 VFX 系を全群の後に 1 プラグイン 1 wasm で入れる(blocked_by 021, 022, 023。needs-owner。依頼者が着手を決める)

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない(L-002)。内蔵の数を、オプションで増減させない。
- 1 タスク 1 関数に戻さない(L-037)。タスクは、利用者から見える動作 1 つ、または設計課題 1 つを終える大きさにする。
- 同じ内容のタスクを機械的に量産しない(L-038)。群の Sub-Issue は、一覧の中身(名前・import・hook 点)から、群ごとに内容を書いて起票する。
- ベンダー名と Cloudflare の識別子を書かない(L-005・L-006)。インストール先の既定の値を書かない。
