# 0094: MSVC switch 表の基底証明を、「B への書き込みがちょうど 1 回」から「その jmp に到達する B の定義が `lea B,[ImageBase]` だけ」（到達定義）に置き換える

状態: **決定（仮ではない）**。PM の決定（2026-10-25）を、作業者（サブエージェント）が記録した。実行ではなく**バイト列と実行で検証できる**ので仮の決定にしない。決定の節、背景、範囲、範囲外は実装の前に保存し、「実装した場所」「テスト」「検証」「解釈が分かれた点」「限界」は実装と検証の後に書いた（記入済み）。

記入状況: すべて記入済み（決定の節・背景・範囲・範囲外は実装前に保存。実装した場所・テスト・検証・解釈が分かれた点・限界は実装と検証の後）。

事実の出典（節番号は各文書のもの）:

- `docs/investigations/msvc-base-reaching-defs.md`（コミット ded27a4。調査のみ。以下「調査」）§1（`msvc_rejected` の理由別内訳）、§2（site ごとの到達定義の分類。記号 R0、P1、P3、P4、P4c、PO は §2.1）、§3（効果の見積もり。P4 の net 26/4、P4c の net 10/3）、§4（`f_48a2060` と `f_37cc7e0`）、§5（選択肢）、§6（決定点）。作業物は `lift/out-rdef/`（読み取り専用。解析器 `analysis/rdef_lib.py` は到達定義の Python 実装）。
- `docs/decisions/0089-msvc-switch-tables-with-proven-base.md`（コミット d87e0ad）1(a)、「解釈が分かれた点」5（揮発 B と call の条件。作業者が足したもの）。`docs/decisions/0090-msvc-switch-base-dominance.md`（コミット b0dc3ab）項目 1、1'、2、3。`docs/decisions/0093-msvc-switch-nop-padding.md`（コミット b58b088）。決定 0079、0036。
- `lift/crates/fe-decode/src/jumptab.rs` の `prove_base`、`lea_dominates_jmp`、`recognize_msvc`、`resolve_msvc`、`verify_msvc`、`foreign_entries`、`landings_in_idioms`、`BaseProof`（HEAD ded27a4）。

## 決定（PM。内容を変えない）

| # | 項目 | 決定 |
|---|---|---|
| 1 | 基底証明 | **到達定義方式にする。** 決定 0090 の `prove_base` の「B への書き込みが関数内でちょうど 1 回」（`writers.len() != 1` の拒否）と「同じ関数にこの jmp 以外の間接 jmp があれば証明しない」を置き換える（0090 の項目 1、1'、2 が 0094 で置き換わる）。新しい条件は、**その jmp に到達する基底レジスタ B の定義がすべて `lea B,[rip+d]`（rip+d = ImageBase）であり、少なくとも 1 つの定義が届くこと**。定義が届かない経路（B が未初期化のまま使われる経路）があるなら拒否。`push B`／`pop B`（`push B` があるとき）は従来どおり書き込みに数えない |
| 2 | 到達定義の CFG と他の間接 jmp | 到達定義は関数（連鎖ピース全部）の制御フローグラフ（直接分岐、fall-through。0090 の `lea_dominates_jmp` と同じ辺）で前向きに求める。**この jmp 自身の表の移動先は辺に入れる**（0090 のとおり）。**他の間接 jmp（`jmp reg`、`jmp [mem]`）は、PM の決定として「その関数内の任意の命令の先頭へ着地しうる 1 つの仮想ノード」として辺に入れる**（調査 §2.1 の P4）。他の間接 jmp が解決済みの switch でも、この決定ではワイルドカードとして扱ってよい（過大評価＝安全側）。解決済みの表の移動先を辺にする拡張（P3）はしない。仮想ノードが運ぶ定義の集合は、すべての他の間接 jmp に届く定義の和集合で、どの命令に着地してもその集合が流れ込む。**`lea` 以外の定義（例: `movzx r10d,[rdx+0xe]`、`mov r10,rdx`）が、他の間接 jmp のどれかに届くなら、仮想ノードを通じてすべての命令に流れ込むので、その B は拒否される**（調査 §2.2 の実データ 7 件はこの形で落ちる） |
| 3 | call の規則（置換） | 関数全体の「揮発 B（rax、rcx、rdx、r8〜r11）と関数内に call があれば拒否」（決定 0089「解釈が分かれた点」5 で作業者が足した条件で、PM の決定ではない）を、**経路上の call** に置き換える。新しい条件は「jmp に到達する経路上（`lea` から jmp まで）に call があり、B が揮発レジスタなら拒否」。callee-saved（rbx、rbp、rdi、rsi、r12〜r15）は call をまたいでも保たれるとみなす（Win64。0089 のとおり、`dataflow.rs` の流儀）。調査 §2.2 の実データでは call をまたぐ site が 0 件。置換の安全側の説明（経路単位）は「実装した場所」に書く。置換が難しければ作業者として関数全体の call 規則を残す（調査 §3.2 の P4c）を選んでもよいが、その場合は決定ファイルに理由を書き、報告に明記する |
| 4 | 変えないもの | **N の導出の拡張はしない**（`jmp r11` のように `cmp idx,imm; ja` が無く `shr` で添字を作る形は、従来どおり `N not derived` で拒否し、`fle_dispatch(R[x])` のまま）。表の形 (c1)/(c2)、移動先の検査（同じ `.pdata` の RUNTIME_FUNCTION 内）、`verify_msvc`、`foreign_entries`、`landings_in_idioms`（0093 で窓 71 バイト）、決定 0093 の `nop` は変えない |
| 5 | 種・基底 | 種 58 個・基底 0x4700000・0x2000000 は測定用で、この実装はそれらに依存しない（種なしの Blend2D でも解決される） |
| 6 | FLEngine への影響 | FLEngine の生成 C の SHA-256 は **`feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f` のまま**が期待（FLEngine は表の idiom として認識される site が 0 件）。変わったら、変わった関数の一覧と原因を報告（隠さない） |
| 7 | 実装の形・状態 | `BaseProof` は `Dominating`（0090。比較用に残す）と新しい `ReachingDefs` を持ち、**既定を `ReachingDefs` にする**。`EntryBlock`（0089）と `Dominating`（0090）は比較専用に残し、`fe-lift` のフラグ（既存の `--msvc-base-entry-block`、新しく `--msvc-base-dominating`（0090 を選ぶ））で選べるようにする。0090 の本文は書き換えず、決定ファイルの注記だけを足す（0090 の項目 1' の直後に 1 行）。**仮の決定ではない**（バイト列と実行で検証できる） |

## 背景（事実。出典の節番号）

- 決定 0090 の基底証明（書き込みがちょうど 1 回の `lea`、同じ関数に他の間接 jmp なし）で解決できる site は、Blend2D（種なし・種 58 個あり）で 3 件（`0x4701384`、`0x4701cbb`、`0x470a89e`）、dsp_ippv2 で 1 件（`0x37c69eb`）。`msvc_rejected` は Blend2D 76（種あり 77）、dsp_ippv2 10（縮小 7）で、理由の内訳は「B が N 回書かれる」34/4、「N が導けない」40/6 ほか（調査 §1.1。HEAD の `fe-lift` の出力）。
- MSVC の switch が B を `lea B,[ImageBase]` で出し直す形は、書き込みが多いだけで、**jmp に届く定義は `lea` だけ**。N が導けて表が関数内の「有効」site 43 件（Blend2D 38、dsp_ippv2 5）のうち、到達定義で `lea` 以外が届くものは、辺が「その jmp 自身の表」だけなら 0 件（調査 §2.2）。
- 3 モジュール構成の停止点は `fle: indirect call to 0x48a20ff is not a known function entry`（`f_48a2060` の `jmp r9`＝`0x48a208b`。決定 0093 で認識は通るが、基底証明が `R10 is written 3 times` で拒否する）。`f_48a2060`（と dsp_ippv2 の同形 `f_37cc7e0`）は r10 の書き込みが 3 回（`lea r10,[ImageBase]`、`movzx r10d,byte [rdx+0xe]`、`mov r10,rdx`）、他の間接 jmp が 3 つ（`jmp r11`。添字が `shr` で作られ N が導けない）。`jmp r9` に届く r10 の定義は `lea` だけで、他の 2 つの書き込みは `ret` で終わる枝の中にあり、4 つの間接 jmp のどれにも届かない（調査 §4.3）。
- 他の間接 jmp を「任意の命令へ着地」として扱う規則（調査の P4）は、`f_48a2060` と `f_37cc7e0` を解き、`lea` 以外の定義が他の間接 jmp に届く実データ 7 件（`0x4745cf4`、`0x4745efd`、`0x4746512`、`0x488be9b`、`0x488e4e0`、`0x4890e43`、`0x489345a`）を拒否する（調査 §2.2、§4.4）。調査の静的な見積もり（解析器。実装との突き合わせは未実施）: P4 は Blend2D 新規 28（net 26）／dsp_ippv2 4（net 4）、関数全体の call 規則を残す P4c は 10（net 10）／3（net 3）（調査 §3.2）。

## 範囲（今回扱う）

- `prove_base` の新しい方式 `BaseProof::ReachingDefs`（既定）: B の定義の前向きデータフロー（定義の同定は命令のインデックス。`lea B,[rip+d]`（= ImageBase）と、それ以外の書き込み、揮発 B のときの call を区別する）、他の間接 jmp の仮想ノード、jmp に届く定義の判定。
- 関数全体の call 規則の、経路上の call への置換（決定 3）。
- `fe-lift` のフラグ `--msvc-base-dominating`（0090 を選ぶ）と `--msvc-base-entry-block`（0089。既存）。

## 範囲外（今回扱わない。従来どおり）

- N の導出の拡張（`shr` で添字を作る形。`N not derived` のまま）。
- 他の間接 jmp が解決済みの switch のとき、その表の移動先を辺にする拡張（P3）。他の間接 jmp はすべてワイルドカード。
- 基底が `lea B,[rip+disp]` 以外で得られるもの、表が 64 ビット絶対 VA、移動先が `.pdata` の範囲外のもの（決定 0089 の範囲外のまま）。
- 表の形 (c1)/(c2)、`verify_msvc`、`foreign_entries`、`landings_in_idioms`、決定 0093 の `nop`。
- 決定 0074、0079、0080、0086、0089、0090、0093 の他の項目。種 58 個は決定にしない。

## 実装した場所

- `lift/crates/fe-decode/src/jumptab.rs`（+209 −28）:
  - `BaseProof`: `ReachingDefs`（先頭。`#[default]`。決定）、`Dominating`（決定 0090。**比較専用**）、`EntryBlock`（決定 0089。**比較専用**）。
  - `cfg_succ`（新規。`lea_dominates_jmp` の `succ` を関数に切り出したもの。本体の辺は同じ: fall-through、関数内の直接分岐、この jmp 自身の表の移動先）。`lea_dominates_jmp` はこれを呼ぶだけで、結果は変わらない。
  - `prove_base_reaching_defs`（新規）: 命令のインデックスを定義の名前にした前向きのデータフロー。定義は `Lea`（`lea B,[rip+d]`、rip+d = ImageBase）、`Write`（それ以外の書き込み。ImageBase 以外の `lea` を含む）、`Call`（B が揮発のときの call）。32 ビット以上の無条件の書き込みと call は前の定義を置き換え、8／16 ビットと条件付きの書き込みは前の定義を残して足す。他の間接 jmp は**1 つの仮想ノード**（集合 1 つ。「他の間接 jmp に届く定義の和集合」）で、作業リストが空になったときだけ全命令の入力へ合流させる（変化したときだけ繰り返す。命令数に比例する辺は作らない）。判定: jmp の入力が空（入口から届かない）、`Write` が届く、`Call` が届く、入口の定義だけ（B を書かない）、入口の定義が混ざる（B が未定義の経路）→ 拒否。すべて `Lea` で 1 つ以上 → 証明。拒否の文言に `decision 0094` と、拒否の原因の命令の VA と mnemonic を含める。
  - `prove_base`: `ReachingDefs` なら `prove_base_reaching_defs` を呼んで返る。`Dominating`、`EntryBlock` は 0090、0089 のまま（本文は変えていない）。
  - **決定 3（call の置換）の安全側の説明（経路単位）**: B が揮発のとき、**すべての call（間接 call を含む）を B の定義**として数え、call は前の定義を置き換える。したがって (1) `lea` の後で jmp に至る経路上に call があれば、jmp に届く定義に `Call` が残って拒否される（call の呼び先が B を変えうる）、(2) call が `lea` の**前**に（その経路で `lea` が置き換えるので）、または jmp に**戻らない枝**（`ret` で終わる）にあるなら、`Call` は jmp に届かず拒否の理由にならない（0090 の関数全体の規則が拒否していた形）、(3) 他の間接 jmp に届く call も仮想ノードを通って全命令へ流れるので拒否される。call は常に定義として扱い（呼び先が B を保つ可能性は見ない）、呼び先の中は見ない（過大評価＝安全側）。callee-saved（rbx、rbp、rdi、rsi、r12〜r15）は call をまたいで保たれるとみなす（Win64。0089 のとおり、`dataflow.rs` の流儀。この仮定は 0089 から続く限界）。**P4c（関数全体の call 規則を残す）は選んでいない**。
  - 変えていないもの: `recognize_msvc`、`resolve_msvc`、`verify_msvc`、`foreign_entries`、`landings_in_idioms`、`read_msvc_targets`、`lea_dominates_jmp` の本体、決定 0093 の `nop`、`MSVC_SKIP`、`MSVC_IDIOM_MAX`。
- `lift/crates/fe-decode/src/lib.rs`: コメント 1 行のみ。
- `lift/crates/fe-lift/src/main.rs`（+4 −4）: `--msvc-base-dominating`（以前は受け付けて何もしなかった）が決定 0090（`BaseProof::Dominating`）を選ぶ。`--msvc-base-entry-block`（決定 0089。既存）。usage の更新。**フラグの名前の衝突はない**。
- `lift/runtime` は編集していない。0090 の本文は書き換えず、項目 1' の直後に 1 行の注記だけを足した。

## テスト

詳細は `docs/investigations/blend2d-switch-reaching-defs.md` §2。`fe-decode/tests/msvc_switch.rs` 45 → 57 件、`fe-lift/tests/msvc_switch.rs` 15 → 18 件。

- 合成 PE（`msvc_common/mod.rs` に `SideWrite`、`CallAt`、`Lea::{LeaOrArg, Never}`、`OtherJmp::{ReachedReg, ReachedAfterWrite, ReachedAfterWriteMem, ReachedBeforeLea, UnguardedSwitch}`）:
  - **解決される**: (a) 書き込み 2 回で jmp に届く定義が `lea` だけ（`f_48a2060` と同形。`ja` の行き先の枝で B を書いて `ret`）、(b) 他の間接 jmp（`jmp reg`、`jmp [mem]`、N が導けない表）があり、非 `lea` の定義がそれに届かない形、第 2 の解決済み switch がある関数（両方）、`lea` が両方の経路にある形、`push B` なしの `pop B` が jmp の後にある形、(f) callee-saved の B は call が経路上にあっても、揮発の B は `lea` の前の call・jmp に戻らない枝の call があっても。
  - **解決されない**: (c) 非 `lea` の定義が他の間接 jmp に届く形（`movzx B,[rdx+0xe]` の後に `jmp rdx`／`jmp [rdx]`）、(d) jmp に届く定義に `lea` 以外が含まれる形（`lea` と `mov B,rcx` の合流。ImageBase 以外の `lea`）、(e) 定義が届かない経路（B を書かない、`lea` が片側だけ、`lea` の前の分岐が間接 jmp に届き入口の B が流れる形）、書き込みが jmp の自身の表を通って戻る形、(f) 揮発の B で `lea` と jmp の間に call、jmp の自身の表を戻る経路上に call、N が導けない表は `N not derived` のまま。
  - 比較専用の方式: 0090（`Dominating`）が解決する形は 0094 も解決し、0090 の拒否の期待（書き込み 2 回、他の間接 jmp、関数全体の call 規則）は `Dominating` を渡して保つ。0089（`EntryBlock`）は入口ブロックの `lea` だけ。`Options::default().msvc_base == ReachingDefs`。
- 実 DLL（fe-decode）: 種なしの Blend2D で 30 site（0090 の 3 + 新規 27。VA 固定。0090 の 3 site の表は同一）、`f_48a2060`（`0x48a208b`）と dsp_ippv2 の `f_37cc7e0`（`0x37cc80b`）が解決され **16 エントリがファイルの表どおり**（objdump と capstone で全件照合）、dsp_ippv2 の MSVC switch 5 site の VA を固定。N が導けて非 `lea` の定義が他の間接 jmp に届く 6 site は拒否。
- 実 DLL（fe-lift）: **`0x48a208b` と `0x37cc80b` の 16 エントリ（r8 = 0..15）と範囲外 8 通り（16、17、31、32、33、0x100、0xffffffff、`u64::MAX`）の計 24 通りを、native（本物の命令）と生成 C の両方で動かし、到達先がファイルの RVA 表どおり**（2 DLL で 96 回。0090 の `0x470a89e` のテストの流儀）。
- **変異検査**（実施。元に戻して `cmp` で一致を確認）: 仮想ノードを外す → 5 件失敗、call の定義を外す → 3 件失敗、定義が前の定義を置き換えない → 32 件失敗。
- **既存の期待値を変えた行**（理由は既定が 0090 → 0094 に変わったため）: 0090 の期待（書き込み 2 回、他の間接 jmp、関数全体の call 規則、`f_48a2060` が解決されない）を固定していたテストは、比較専用の `BaseProof::Dominating` を渡す形に直して**元の期待を保った**。0094 の既定で結果が変わる行は新しい値に直した（`jt_cpu.rs` の `msvc_tables` 1 → 5、`jumptabs` 435 → 439、`jmp:unresolved` 548 → 544、`jmp:msvc_table` 1 → 5。fe-lift の `jt_cpu.rs` の `jumptabs` 434 → `(jumptabs, msvc_tables) == (437, 3)`。`real_blend2d_site_is_resolved_by_default` の 3 → 30）。一覧は調査 §2 の表。

## 検証

実測値と手順は `docs/investigations/blend2d-switch-reaching-defs.md`。作業物は `lift/out-rdefs/`。

| 項目 | 結果 |
|---|---|
| `cargo test --workspace`（`CARGO_TARGET_DIR=lift/out-rdefs/target`、`FLENGINE_PE` 指定） | **98 binaries / 1,228 passed / 0 failed / 0 ignored**（基準 98 / 1,213 / 0 / 0。**+15 件**。SKIPPED 0 件） |
| **FLEngine の生成 C の SHA-256** | **`feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f` のまま**（`lift/out-srw/fle.c` と `cmp` でバイト一致）。変わった関数 0。functions 59,297、bad_nodes 0、unsupported_total 525、register_jmp_sites 20、jumptabs 4、msvc_tables 0、msvc_rejected 0。単独構成の a4 は生成 C も runtime も変わらないので**省略** |
| DLL ごとの統計の前後（HEAD → 0094。`--pdata`） | Blend2D 種なし: closure 399,148 → 405,045、functions 2,018 → 2,019、jumptabs／msvc_tables 3 → 30、msvc_rejected 76 → 49、bad_nodes 0、unsupported_total 596 のまま、register_jmp_sites 99 のまま。種 58 個あり: closure 399,777 → 405,674、functions 2,066 → 2,067、jumptabs／msvc_tables 3 → 30、msvc_rejected 77 → 50、bad_nodes 0。dsp_ippv2: closure 6,465,008 → 6,465,171、jumptabs 435 → 439、msvc_tables 1 → 5、msvc_rejected 10 → 6、bad_nodes 0。dsp_ippv2 縮小: closure 3,038,781 → 3,038,921、jumptabs 434 → 437、msvc_tables 0 → 3、msvc_rejected 7 → 4、bad_nodes 0。QFC、FLEngine: **全項目変化なし**。**bad_nodes はどこも増えなかった**。`--msvc-base-dominating` は 6 構成すべてで HEAD と統計の全行が同一 |
| 調査 §3.2 の見積もりとの比較 | 調査の P4（net）Blend2D **26**／dsp_ippv2 **4**、P4c 10／3 に対し、実装（決定 3 を実装）は Blend2D **27**／dsp_ippv2 **4**（縮小は 3）。**差の +1 は `0x4889443`**（調査は他関数の `0x487e0f5` の `jcc 0x487e10e` を `foreign_entries` が落とすと見積もったが、これは直前の関数の RVA 表のバイトを iced-x86 が `loopne` と読んだ偽の分岐で、閉包は持たないので落ちない）。call の規則で拒否された site は 0 件（調査の「call をまたぐ site は 0 件」と一致）。`0x47639ae` は既存の `landings_in_idioms` が落とした。非 `lea` の定義が他の間接 jmp に届く 7 site のうち 6 は `reaches the jump` で拒否、`0x4745efd` は閉包に未到達（0090 でも同じ） |
| 新しく解決された site | Blend2D 27: `0x470749d`、`0x4707d3a`、`0x470b983`、`0x470c607`、`0x470d019`、`0x47210ff`、`0x4722765`、`0x4724748`、`0x4725144`、`0x4725db7`、`0x4728561`、`0x47290af`、`0x4760a33`、`0x476b168`、`0x476cf0a`、`0x47802d9`、`0x478129c`、`0x47aa0bc`、`0x47aa54e`、`0x48421c1`、`0x48654a6`、`0x48712cf`、`0x487d636`、`0x4889443`、`0x48a208b`、`0x48aef10`、`0x48af10a`。dsp_ippv2 4: `0x37b25bd`、`0x37cc80b`、`0x37d9670`、`0x37d986a`。種なしと種 58 個ありで同一 |
| 解決した全 site の全エントリの機械検査 | Blend2D 30 site（272 エントリ）、dsp_ippv2 5 site（68 エントリ）の**全エントリが、関数のピース内の線形走査の命令の先頭（capstone）、関数内、生成 C のラベルがちょうど 1 つ**、case は互いに異なり既定は `fle_jumptab_miss`。失敗 0 件 |
| `f_48a2060`／`f_37cc7e0` の表の 16 エントリ | ファイルのバイト列から読んだ表を、**objdump と capstone の両方で全 16 件個別に照合**（16 個とも異なり、命令の先頭、関数内）。native と生成 C を r8 = 0..15 と範囲外 8 通りで動かし、到達先が表どおり（2 DLL で 96 回） |
| 3 モジュール（種 58 個あり、基底 0x4700000。測定のみ。runtime は HEAD のまま） | a4: init_count **150/942**、last unit 149（proc `0x664260`）、functions_distinct 1,069、function_entries **27,333**（前回 27,332）。2 回同一。**停止は `fle: indirect call to 0x48a20ff` から `fle: indirect call to 0x489f940 is not a known function entry`（top of stack `f_489f7d0`、`0x489f8fb: jmp r9`）へ動いた**。Blend2D の DllMain は戻らない、LoadLibraryW は 0x4700000 を返さない（`R[0]=0x2000000` の dsp_ippv2 の 1 回のみ）。ATTACH 内の import は 183 回で、前回と通し番号・名前が全 1,708 件で一致（2 回の gdb の import の列は `cmp` で一致）。**`0x48a20ff` は閉包のラベルになった**。停止点の分類は (v) その他（生成器が解決しない MSVC 型の `jmp reg`）。新しい停止の `jmp r9` は `lea r9,[ImageBase]; mov eax,[r9+r8*4+T]; add r9,rax; add rcx,r8; mov rax,r11; jmp r9` の形で、`recognize_msvc` の表の形（基底が `add` の第 2 オペランド）に当たらない（0094 の範囲外。直していない）。unit 158 には届いていない |

## 解釈が分かれた点

詳細は調査 §7.1。決定の内容は変えていない。**実装できなかった項目は無い**。

1. **決定 3（call の置換）は実装した。P4c は選んでいない**（安全側の説明は「実装した場所」）。
2. 定義の置き換え: 32 ビット以上の無条件の書き込みと call は前の定義を置き換え、8／16 ビットと条件付きの書き込み（`setcc`、`cmov`）は前の定義を残して足す（調査の解析器は `cmov` だけを足す扱いにした）。実データで `0x4745cf4`（`sete r12b`）がこの扱いで `lea` と `sete` の両方が届く形になる。
3. `pop B` は 0090 と同じ（同じ関数に `push B` があれば定義に数えない）。調査の解析器は常に定義に数えた。0090 が書き込み 2 回で拒否していた「`push B` なしの `pop B` が jmp の後にある」合成の形は 0094 では解決される（jmp に戻る経路が無いので届かない）。
4. 「他の間接 jmp」は 0090 と同じ `FlowControl::IndirectBranch`（間接 call は含まない）。関数のピースの線形走査で数える。
5. 入口の B（呼び元の値）を 1 つの定義として扱い、jmp に届けば拒否。B を一度も書かない場合と、一部の経路だけ書く場合は別の文言。
6. `--msvc-base-dominating` は以前「受け付けて何もしない」だったので、0090 を明示的に選ぶ意味に直した。名前の衝突はない。

## 限界

- **線形走査の CFG**: 0090 と同じ。関数のピース内でデコードできる命令だけで作る。ピースの外への直接分岐は辺を持たない。SEH のハンドラ／フィルタ、`longjmp`、例外による遷移は見ない。
- **他の間接 jmp は「任意の命令へ着地」の過大評価**（安全側）。このために拒否されるが実際は安全な site が出うる（調査 §3.3 の D で実表を読むと届かない 6 site。実装でも拒否）。緩める（他の間接 jmp の実表を辺にする P3、辺なし PO）判断は PM。
- **N が導けない表は解決しない**（決定 4）。Blend2D 40 site、dsp_ippv2 6 site（`f_48a2060` の 3 つの `jmp r11`、3 モジュールの新しい停止点 `0x489f8fb` の `jmp r9` を含む）。
- call をまたぐ揮発 B は拒否するが、callee-saved が call をまたいで保たれる仮定（Win64。0089 から）は変わらない。呼び先の中は見ない。
- 実 DLL で native と生成 C を突き合わせたのは `0x48a208b`、`0x37cc80b`、（0090 の）`0x470a89e` の 3 site。新しく解決した他の 27 + 3 site は、バイト列（命令境界、関数内、ラベル）の機械検査に限る。実行して表どおりに動くかは未確認。
- 母集団の誤認識の検査は、FLEngine、dsp_ippv2、Blend2D、QFC の 4 つに限る（同梱の他の DLL は未実施）。
- `.pdata` を持たない image と、`.pdata` の RUNTIME_FUNCTION の外にある `jmp reg`（Blend2D の 4 か所）では何も解決しない（決定 0089 (d)）。
- 3 モジュールの測定は gdb の仮の値なし、runtime の応答追加なし。新しい停止（`0x489f940`）は直していない。実 Windows での挙動は未確認。wasm は使っていない。
