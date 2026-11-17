# 0090: MSVC switch 表の基底証明を、入口ブロックの簡易条件から「`lea B,[ImageBase]` が jmp に到達する全経路の前に実行される（dominance）」に置き換える

状態: **決定（仮ではない）**。PM の決定（2026-10-24）を、作業者（サブエージェント）が記録した。実行ではなく**バイト列で検証できる**ので仮の決定にしない。決定 0089 の 1(a) の「簡易条件（入口ブロック）」は、本来の条件「`lea` が jmp に到達する全経路の前に実行される」の**十分条件として書いたもの**で、PM が「緩めない」と書いたために実装が入口ブロックに限られた。これは決定文の曖昧さで、PM が今回訂正する。決定の節は実装の前に保存し、「実装した場所」「テスト」「検証」「解釈が分かれた点」「限界」は実装と検証の後に書いた（記入済み）。

事実の出典（節番号は各文書のもの）:

- `docs/decisions/0089-msvc-switch-tables-with-proven-base.md`（コミット d87e0ad）1(a)、「解釈が分かれた点」5、「限界」。
- `docs/investigations/blend2d-switch-tables.md`（コミット d87e0ad）§3（95 か所の内訳。§3.4 は B が複数回書かれる 33 か所）、§5.6（`0x470a89e` の native と生成 C の突き合わせ）、§6.3（`--msvc-base-dominating` の測定。停止が `0x4899fed` の `GetModuleHandleA` へ移る）、§8（解釈が分かれた点）。
- 測定専用の `BaseProof::Dominating`（`lift/crates/fe-decode/src/jumptab.rs` の `BaseProof`、`lea_dominates_jmp`、`prove_base`。`fe-lift --msvc-base-dominating`）は d87e0ad に既にある。

## 決定（PM。内容を変えない）

| # | 項目 | 決定 |
|---|---|---|
| 1 | 基底証明の既定 | **dominance にする**。関数内で B への書き込みが **`lea B,[rip+disp]`（rip+disp = ImageBase）のちょうど 1 回**（現行の条件と同じ。push/pop の退避復元は数えない）で、その `lea` が **jmp に到達するすべての経路（関数の入口から jmp までの全経路。線形走査の CFG）の前に実行される**こと。入口ブロックの条件は不要になる（入口ブロック内の `lea` は dominance の特別な場合として自動的に満たす） |
| 1' | 扱わない形 | 書き込みが 2 回以上ある形（MSVC が switch の直前で `lea B` を出し直す形。調査 §3.4 の 33 か所）は**今回も扱わない**（到達定義の解析は範囲外）。解決しない |
| | （注記） | 項目 1（ちょうど 1 回）と 1'（到達定義は範囲外）と項目 2（他の間接 jmp があれば証明しない）は決定 0094 で到達定義方式に置き換わった（0090 は `BaseProof::Dominating` として比較専用に残る） |
| 2 | dominance の健全性の条件（解決を減らす方向） | 現行の `lea_dominates_jmp` は「他の間接 jmp に辺がない」近似なので、**同じ関数（連鎖したピース全部）に、この jmp 以外の間接 jmp（`jmp reg`、`jmp [mem]`）が 1 つでもあれば、dominance は証明しない**（その jmp が `lea` を飛ばして jmp へ到達しうるため）。その間接 jmp が同じ関数の別の解決済み MSVC switch 表の jmp であって、その表の全移動先を辺として CFG に入れて同じ証明が成り立つなら許す、という拡張は**今回しない**（単純に、他の間接 jmp があれば証明しない）。tail call の直接 `jmp rel32` は間接ではない |
| 3 | 維持する条件 | 決定 0089 に足された既存の条件は**すべて維持**: 揮発レジスタ B で関数内に call があれば証明しない、`verify_msvc`、`foreign_entries`、`landings_in_idioms`。決定 0089 の 1(b)、1(c1)、1(c2)、1(d)、2、2' も変えない |
| 4 | 実装 | `BaseProof::Dominating` を既定にする（`#[default]`）。`EntryBlock` は**比較のための測定専用**として残す（`fe-lift --msvc-base-entry-block`。決定ではない）。既存の `--msvc-base-dominating` は受け付けるが何もしない（既定と同じ）。`lift/runtime` は編集しない。入口ブロックの簡易条件で解決していた 2 か所は、dominance でも解決される |
| 5 | FLEngine への影響 | FLEngine の生成 C の SHA-256 は **`43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1` のまま**が期待（FLEngine の MSVC 形 3 か所は決定 0036 の `Spec` が先に解決する）。変わったら、変わった関数の一覧と原因を報告に書く（隠さない。変わった場合の判断は PM が行う）。`unsupported_total`、`register_jmp_sites`、`jumptabs`、`msvc_tables`、`msvc_rejected` の変化も記録する |
| 6 | 変えないもの | 決定 0074、0079、0080、0086、0089 の他の項目は変えない。0089 の本文は書き換えない（1(a) の行の直後に 1 行の注記だけ追記する）。58 個の種は決定ではなく、この実装は種に依存しない（種なしの Blend2D でも動く） |
| 7 | 状態 | **仮の決定ではない**（バイト列と実行で検証できる）。dominance の近似（線形走査の CFG、他の間接 jmp があれば証明しない）の限界は「限界」に書く |

## 背景（事実。出典の節番号）

- 決定 0089 の実装は、実 DLL の `rva-switch` 95 か所のうち 2 か所（Blend2D の `0x4701384`、`0x4701cbb`）しか解決しなかった（調査 §3）。Blend2D の停止点 `0x470a89e`（`f_4709ea0` の `jmp rcx`）は、`lea r13,[ImageBase]` が 0x4709f18 にあり、入口ブロック（0x4709ea0〜0x4709efa）の外にあるため解決されなかった。ただし 0x4709efa の `jne 0x4709f07` のどちらの経路でも 0x4709f18 を通る（調査 §5.6、§6.3）。
- 測定専用の `--msvc-base-dominating` では、Blend2D で `jumptabs` が 2 → 3（`0x470a89e` が増える。閉包 +168 命令）、dsp_ippv2 は 435 のまま、FLEngine は不変。3 モジュールの停止は `fle: unsupported instruction at 0x4899fed`（`f_4899d90` の `call [IAT]` = KERNEL32!GetModuleHandleA。行なし import 15 名の 1 つ、B8）へ移り、ATTACH の import は 174 回、function_entries 27,317（調査 §6.3）。`0x470a89e` は native と生成 C の 40 通りの到達先が表どおりだった（調査 §5.6）。

## 範囲外（今回扱わない。従来どおり）

- 書き込みが 2 回以上ある形（`lea B` を出し直す形。到達定義の解析が要る）。
- 他の間接 jmp を持つ関数での dominance（その jmp が別の解決済み MSVC switch 表であっても、その表の移動先を辺にする拡張はしない）。
- 基底が `lea B,[rip+disp]` 以外で得られるもの、表が 64 ビット絶対 VA、`cmp`/`ja` から N を導けないもの、移動先が `.pdata` の範囲外のもの（決定 0089 の範囲外のまま）。
- 決定 0074、0079、0080、0086 は変えない。58 個の種は決定にしない。

## 実装した場所

- `lift/crates/fe-decode/src/jumptab.rs`:
  - `BaseProof`: `Dominating` を先頭にして `#[default]`（決定）。`EntryBlock` は残し、コメントで「比較のための測定専用。決定ではない」と明記。
  - `prove_base`: 書き込みがちょうど 1 回で `lea B,[rip+d]`（= ImageBase）、揮発 B と call の検査（0089 のまま）の後、`Dominating` では (1) **この jmp 以外の間接 jmp（`FlowControl::IndirectBranch`。`jmp reg`、`jmp [mem]`）が関数の線形走査（連鎖した全ピース）に 1 つでもあれば拒否**（理由に最初のものの VA と `decision 0090`）、(2) `lea_dominates_jmp`（既存。本体は変更なし）で全経路の前にあることを検査。`EntryBlock` は 0089 のまま（他の間接 jmp は見ない）。
  - 変えていないもの: `resolve_msvc` の順序、`verify_msvc`、`foreign_entries`、`landings_in_idioms`、`recognize_msvc`、`lea_dominates_jmp` の本体。
- `lift/crates/fe-decode/src/lib.rs`: コメント 1 行のみ。
- `lift/crates/fe-lift/src/main.rs`: `--msvc-base-dominating` は受け付けて何もしない。`--msvc-base-entry-block`（比較用）を追加。usage を更新。
- `lift/runtime` は編集していない。

## テスト

- 合成 PE（`fe-decode/tests/msvc_common/mod.rs`: `Lea::{AfterDiamond, OneSided, BothSides}`、`OtherJmp::{Reg, Mem, TailCall, SecondSwitch}` を追加）。`fe-decode/tests/msvc_switch.rs` 22 → 31 件、`fe-lift/tests/msvc_switch.rs` 9 → 11 件。
- **解決されること**（新しい期待）: `lea` が入口ブロックの外にあり jmp に dominates する形（`je` の次の命令へ、diamond の後）、入口ブロックの `lea`（dominance の特別な場合）、tail call の直接 `jmp rel32` を持つ関数。
- **解決されないこと**: `lea` が一方の経路にしかない形、両経路に `lea`（B が 2 回書かれる）、同じ関数に `jmp reg`／`jmp [mem]`／別の switch がある形（別の switch は両方とも拒否）、揮発 B と call、B が引数から来る、`lea` の値が ImageBase でない、ほか 0089 のもの。
- 合成 PE を clang で native にも実行: `lea` が入口ブロックの外の形でも、索引 0..N+3 のすべてで native と生成 C の結果が同じで表のバイトの指す先と一致。
- 実 DLL（Blend2D `0x470a89e`）: **既定で**解決され、表の 6 エントリ（0x470a8a0、0x470a8af、0x470aa40、0x470aa6b、0x470aab1、0x470aafc）、全 33 通りの索引＋範囲外 7 通りの 40 通りで native（本物の命令）と生成 C の到達先が、ファイルの索引バイトと RVA から読んだ値と一致。
- 既存テストで期待値を変えた行（その行と理由）は `docs/investigations/blend2d-switch-dominance.md` §5.2 の表。入口ブロックだけを前提にした「`lea` が入口ブロックの外 → 解決しない」のテスト 1 本は、dominance の有無で分けて 4 本に書き直した。

## 検証

実測値と手順は `docs/investigations/blend2d-switch-dominance.md`。作業物は `lift/out-jtb2d90/`。

| 項目 | 結果 |
|---|---|
| `cargo test --workspace` | **97 binaries / 1,172 passed / 0 failed / 0 ignored**（基準 97 / 1,161。+11 件。SKIPPED 0 件） |
| **FLEngine の生成 C の SHA-256** | **`43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1` のまま**（既定、`--msvc-base-entry-block`、`--msvc-base-dominating` のすべて）。変わった関数 0。functions 59,297、bad_nodes 0、unsupported_total 525、register_jmp_sites 20、jumptabs 4、msvc_tables 0、msvc_rejected 0。単独構成の a4 は生成 C が変わらないので省略 |
| 他の間接 jmp があるために証明されなくなる site（見積もり。実測） | **0 か所**。Blend2D の (c1)(c2) で N が導ける 37 か所のうち、同じ関数に他の間接 jmp があるのは 15 か所だが、すべて B が複数回書かれ（7〜407 回）、決定 1' で先に落ちる。実装後の `msvc_rejected` に `other indirect jump` の理由は 0 件（Blend2D 種なし・種あり、dsp_ippv2、dsp_ippv2 縮小、QFC、FLEngine） |
| **`f_4709ea0` に他の間接 jmp が無い**（必須の確認） | objdump の全 989 行で `jmp` は 6 個。間接は `0x470a89e jmp rcx` の 1 個だけ（残り 5 個は直接 `jmp rel`）。`call` も無い。r13 への書き込みは `lea r13,[ImageBase]`（0x4709f18）の 1 回（`push r13`／`pop r13` は数えない）。`.pdata` は 1 エントリで連鎖なし |
| DLL ごとの統計の前後（d87e0ad の既定 → 0090 の既定） | Blend2D 種なし: closure 398,980 → 399,148（+168）、jumptabs 2 → 3、msvc_tables 2 → 3、msvc_rejected 77 → 76、bad_nodes 0、unsupported_total 662 のまま、register_jmp_sites 99 のまま。種 58 個あり: 399,609 → 399,777、jumptabs 2 → 3、msvc_rejected 78 → 77、bad_nodes 0、unsupported_total 665 のまま。dsp_ippv2、dsp_ippv2 縮小、QFC、FLEngine: **全項目変化なし**。**bad_nodes はどこも増えなかった**。d87e0ad の測定専用 `--msvc-base-dominating` と 0090 の既定は、6 構成すべてで統計の全行（`msvc_rejected` の行を含む）が同一 |
| 解決した site の全エントリの個別照合（objdump / capstone） | 解決した 4 site（Blend2D 3、dsp_ippv2 1）の distinct な移動先 23 件すべてが、関数のピース内の命令の先頭（objdump の最初の行がちょうどその VA）。範囲外のエントリは 0 件 |
| 3 モジュール（種 58 個あり、基底 0x4700000。測定のみ） | **停止は `fle: unsupported instruction at 0x4899fed` へ移る**（`f_4899d90` の `call [IAT]` = `KERNEL32!GetModuleHandleA`。行なし 15 名の 1 つ。B8）。a4: init_count 150/942、last unit 149（proc 0x664260）、functions_distinct 1,058、function_entries 27,317。2 回同一。ATTACH の import 174 回。DllMain は戻らず、LoadLibraryW は 0x4700000 を返さない。分類 (i) 未応答の import。**前回の参考測定（`--msvc-base-dominating`）と a4 の行、停止、C のスタック、import の列が同一**。新しい停止は出ていない。直していない。種なしの Blend2D は停止が `0x48a57f8`（B5）のままで、前回と同一 |

## 解釈が分かれた点

詳細は調査 `docs/investigations/blend2d-switch-dominance.md` §8。決定の内容は変えていない。実装できなかった項目は無い。

1. 「他の間接 jmp」は `FlowControl::IndirectBranch`（`jmp reg`、`jmp [mem]`）で、間接 call は含まない。関数のピースの線形走査で数えるので、表のバイトが偶然の命令に見える場合も数える（解決を減らす方向）。実 DLL では 0 件。
2. 他の間接 jmp の検査の順序: 書き込みが 1 回で `lea` が ImageBase であること、揮発 B と call の検査の後、`lea_dominates_jmp` の前。`msvc_rejected` には最初に失敗した条件だけが出る。
3. `EntryBlock`（比較用）は他の間接 jmp を見ない（0089 のまま）。
4. `--msvc-base-dominating` は受け付けて何もしない。usage からは外した。

## 限界

- **線形走査の CFG**: 関数のピース内でデコードできる命令だけで作る。ピースの外への直接分岐は辺を持たない。実行時にだけ決まる経路（SEH のハンドラ／フィルタ、`longjmp` など）は見ない。`foreign_entries` は閉包が知っている分岐だけを見るので、閉包が知らない経路は検出できない。
- **他の間接 jmp があれば証明しない**は十分条件で必要条件ではない。他の間接 jmp が同じ関数の解決済み switch なら、その表の移動先を辺にして同じ証明ができるが、今回は扱わない。実 DLL では 0 か所の影響（上の検証）。
- **書き込みが 2 回以上ある形**（MSVC が switch の直前で `lea B` を出し直す形。調査 `blend2d-switch-tables.md` §3.4 の 33 か所）は扱わない（到達定義の解析が要る）。95 か所のうち解決できるのは種なしの Blend2D で 3 か所（0089 では 2 か所）。
- 解決した 4 か所のうち、native と生成 C の突き合わせをしたのは `0x470a89e` だけ（40 通り）。Blend2D `f_4701310`、`f_4701bb0` と dsp_ippv2 `f_37c6990` は、バイト列（命令境界、関数内）の確認に限る（実行されたかは未確認）。
- 母集団の誤認識の検査は、FLEngine、dsp_ippv2、Blend2D、QFC の 4 つに限る（同梱の他の DLL は未実施）。
- `.pdata` を持たない image では何も解決しない。`.pdata` に無い葉関数の中の `jmp reg`（Blend2D に 3 か所）は決定 0089 の (d) を満たせないので解決しない。
- 3 モジュールの測定は gdb の仮の値なし、runtime の応答追加なし。新しい停止（`GetModuleHandleA`）は直していない。実 Windows での挙動は未確認。wasm は使っていない。
