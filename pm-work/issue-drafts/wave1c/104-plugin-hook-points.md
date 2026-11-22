---
draft: "104"
title: "Delphi 系の内蔵プラグインの hook 点を、全プラグイン分、検出する道具を作り、`inputs/plugin-hooks.tsv` を commit する"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: []
labels: ["plugin-builtin"]
size: L
priority: Medium
project: signature-studio/3
---

## 目的

内蔵のプラグイン(本体は約 142 件)を lift するには、プラグインごとに hook 点が要る。hook 点は、DLL の Delphi の実行時が、自分の関数 3 つに書き込む飛び先(対象)の RVA である。今の 3 件(Fruity Balance・Fruity Filter・Fruity Delay 2)の 3 つの RVA と先頭 5 バイトは、`lift/harness/hooks/fruity-delphi-effect-hook-points.txt` に在る(手で見つけた値)。

この Issue は、次の 3 つをする。(1) その 3 件の hook 点の作り方を、規則として言葉にする。(2) 同じ規則で、全ての本体の hook 点を機械的に検出する道具 `tools/plugin-hooks/detect.mjs` を作る。(3) 検出の結果を `inputs/plugin-hooks.tsv` として commit する。第 2 波(内蔵プラグインを束で lift する Issue。#{{021}}・#{{022}} の子)の起票は、この表を入力にする。

## 背景

- 分類の表: `inputs/plugin-classes.tsv`(#{{103}} の成果。見出しと 182 件)。`hook の要否` は、本体で 要 110・既存 22・不要 10、補助で 要 3・不要 37。この Issue の対象は、`本体か補助か` が 本体 で `hook の要否` が 要 か 既存 の 132 件(Effects 88・Generators 44)。
- 「既存」「要」は、3 件の RVA で先頭 5 バイトを比べた第 1 の分類である。「要」は Delphi の印が在るが、同じ 3 点ではない、という意味。DLL ごとの 3 点を求めることが、この Issue の本題。
- 3 件の仕組み(`lift/harness/hooks/fruity-delphi-effect-hook-points.txt` の注釈。導入の記録は `patches/mt3w/0313/impl/0313-g9a-balance-module.diff`・`0313f-filter-on-balance.diff`・`0313g-delay2-on-filter.diff`)。Balance では、DLL の静的初期化が、構築関数(VA 0x5c2d70)を 3 回呼ぶ(呼び出し元は 0x5c6b97・0x5c6bbf・0x5c6be7)。各呼び出しの前に、`lea r8` で対象の VA を、`lea r9` で置き換えの VA を置く。対象の VA は 0x417bc0・0x431d70・0x431d90 で、ImageBase 0x400000 の時の値。構築関数は、VirtualProtect(対象, 5, PAGE_EXECUTE_READWRITE, &old) を kernel32 の IAT から呼び、`e9 rel32` を書く。Filter も同じ形(構築関数 0x5c2c90・呼び出し元 0x5c6ab7・0x5c6adf・0x5c6b07)。
- 対象の関数の先頭 5 バイトは、3 件とも次の 3 種類: `55 56 53 c6 44` / `53 48 83 ec 20` / `55 48 83 ec 50`。Delphi の実行時の同じ関数が、DLL ごとに写しとして入っている。だから RVA は DLL ごとに違う。注釈にある FLEngine・QFC・ILTools・Wrapper も同じ形。注釈によると、Frequency Shifter・Fruity Parametric EQ 2・Soundgoodizer は、別の 3 点を持つ。
- 既存の検出スクリプト(`find-hook-points.py`)は、このリポジトリに無い。手順は注釈の記録だけ。道具は node で書く。
- hook 点の条件(`lift/crates/fe-lift/src/hook.rs`): 対象は関数の先頭(lift される関数の入口)で、先頭 5 バイトが既に `e9` でなく、読み取りと実行のできるセクションに在ること。
- 実物: `lift/out-app/host-extra/plugins/`。この Issue の作業の段が、R2 のキャッシュから置く(`plugin-builtin` の作業だけ)。木は、インストール先からの相対のパス(`/` 区切り)。直下の `MANIFEST.tsv` が、guest・bytes・sha256 の表。今の 3 件の DLL も、同じ木に在る。

## 読む物

- `inputs/plugin-classes.tsv`(対象の行。`hook の要否` の列)
- `lift/harness/hooks/fruity-delphi-effect-hook-points.txt`(3 件の RVA と注釈)。同じ仕組みの注釈として、`lift/harness/hooks/flengine-hook-points.txt`・`qfc-hook-points.txt`・`iltools-hook-points.txt`・`wrapper-hook-points.txt`
- `patches/mt3w/0313/impl/0313-g9a-balance-module.diff`・`0313f-filter-on-balance.diff`・`0313g-delay2-on-filter.diff`(3 件の hook 点の導入)
- `tools/plugin-classes/pe.mjs`(RVA からファイルの位置への対応と、export の名前)、`tools/plugin-classes/plugin-classes.mjs`(分類の道具)、`tools/fl-installer-files.mjs`(`peImports`)
- `lift/crates/fe-lift/src/hook.rs`(hook 点の条件)
- `lift/out-app/host-extra/plugins/MANIFEST.tsv` と、木の中の DLL

## やること

1. 規則を言葉にする。Balance・Filter・Delay 2 の 3 件について、DLL のバイト列から、構築関数・3 回の呼び出し・`lea r8` と `lea r9` のオペランド(対象と置き換え)・VirtualProtect の呼び出しを見つける。3 件の対象の RVA が、0x17bc0・0x31d70・0x31d90 と一致することを、道具で確かめる。見つけ方の規則を、`tools/plugin-hooks/detect.mjs` の冒頭のコメントに書く。
2. 道具 `tools/plugin-hooks/detect.mjs`(node)を作る。PE の読みは `tools/plugin-classes/pe.mjs` と `tools/fl-installer-files.mjs` を再利用する。ImageBase は各 DLL の PE ヘッダの値を使い、0x400000 を決め打ちしない。候補の規則(作業者が実物で確かめ、外れたら直す):
   - (a) 命令の並び `lea r8,[rip+d]`(4C 8D 05 と d32)・`lea r9,[rip+d]`(4C 8D 0D と d32)・`call rel32`(E8 と d32)を、近い範囲(数十バイト以内)で探す。
   - (b) 呼び出し先ごとに、この並びの数を数える。ちょうど 3 組を持つ呼び出し先を、構築関数の候補とする。3 組の r8 の値(対象の VA から ImageBase を引いた値)が、hook 点の 3 つの RVA になる。
   - (c) 確かめる: 対象が .pdata の関数の先頭(RUNTIME_FUNCTION の BeginAddress)であること。対象の先頭 5 バイトが、上の 3 種類のいずれかであること。構築関数の呼び出しの先に、kernel32!VirtualProtect の呼び出しが在ること(注釈の記録では 2 段先)。
3. 本体 132 件を検出する。検出できた物は、3 つの RVA と先頭 5 バイトを書き、確信を付ける。確信の値は次のとおり。高 = (c) の 3 つとも満たす。中 = 3 組は見つかるが、(c) のどれかが外れる。低 = 3 組に満たない。検出できない物は、RVA と先頭 5 バイトを空にし、確信を低にする。備考に、どの段で外れたかと、手で見る必要がある点を書く。
4. `inputs/plugin-hooks.tsv` を書き、commit する。TAB 区切りで、1 行目が見出し。列は、`パス`・`hook 1 RVA`・`hook 2 RVA`・`hook 3 RVA`・`hook 1 先頭 5 バイト`・`hook 2 先頭 5 バイト`・`hook 3 先頭 5 バイト`・`検出の根拠`・`確信`・`備考`。`パス` は `inputs/plugin-classes.tsv` と同じ書き方(バックスラッシュ区切り)。RVA は `0x` の 16 進。先頭 5 バイトは `55 56 53 c6 44` の形(空白区切り)。1 行が 1 件の本体で、行数は 132 件。
5. 既存の 22 件について、検出した 3 つの RVA が 0x17bc0・0x31d70・0x31d90 と一致するかを、報告に書く。一致しない物は、分類(既存)と検出の食い違いとして報告に書く。
6. 参考の検証として、注釈にある 3 件の値(Frequency Shifter の 0x17e90・0x320b0・0x320d0、Fruity Parametric EQ 2 の 0x17ee0・0x32100・0x32120、Soundgoodizer の 0x1b7b0・0x389e0・0x38a00)と、道具の出力を比べ、結果を報告に書く。一致しなくても、受け入れは落とさない。
7. 試験 `spec/plugin-hooks.test.mjs` を作る。道具の数え方((a)と(b))を、手で作った小さなバイト列で確かめ、表の形(見出し・行数・列数)と、確信の値の範囲を確かめる。

## 受け入れ

```accept
node tools/plugin-hooks/detect.mjs --check && node spec/plugin-hooks.test.mjs
```

`--check` が 0 で終わる条件: (a) `inputs/plugin-hooks.tsv` が在り、見出しと列数が上の形で、`inputs/plugin-classes.tsv` の 本体 で `hook の要否` が 要 か 既存 の 132 件が、各 1 行ずつ全て在る(重複は数えない)。(b) 3 件(Fruity Balance・Fruity Filter・Fruity Delay 2)の行の 3 つの RVA と先頭 5 バイトが、`lift/harness/hooks/fruity-delphi-effect-hook-points.txt` の 3 つの RVA と、その先頭 5 バイトに完全に一致する。(c) 表の hook の値が、実物から道具で検出し直した値と一致する(表が道具の出力であること)。

判定の基準: 受け入れが 0 で終わること。(a)(b)(c) の確かめの結果を、報告に書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 件数は、道具の出力から書く。推定と実測を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 対象の件数(132。要 110・既存 22 の内訳)、検出できた件数(確信の高・中・低)、検出できなかった件数と、その DLL の名前と理由、既存 22 件の RVA の一致の結果、3 件の一致の結果、注釈の 3 件の参考の結果、規則の要約(候補の規則のどこを直したか)、変えたパス(commit したファイルだけ)。
- インストール先の既定の絶対パス(会社名を含む)を書かない。

## 制約

- 製品(`app/`・`lift/`)・`ci/`・`.github/`・`inputs/` の既存のファイル・`lift/harness/hooks/` の既存のファイルを変えない。新しいファイル(`tools/plugin-hooks/`・`spec/plugin-hooks.test.mjs`・`inputs/plugin-hooks.tsv`)を足すだけ。
- エンジンのビルドは要らない(分析の道具)。受け入れに `fe-build`・`cargo` を入れない。
- 道具は node で書く。Python の道具を commit しない。
- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit の名義は環境の git 設定のまま。Claude の行を付けない。ブランチを作らない。

## 他のタスクとの関係

- #{{103}}(分類の表)の結果が入力。`inputs/plugin-classes.tsv` は変えない。
- #{{021}}・#{{022}}(内蔵 Effects と内蔵インストゥルメントの残り)の子として起票する第 2 波の Issue は、この表を入力にする。
- `tools/` と `inputs/` は、他の作業が変えうる。作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
