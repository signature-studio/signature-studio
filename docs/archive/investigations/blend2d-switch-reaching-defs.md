# Blend2D の MSVC switch 表: 基底証明を到達定義にした実装（決定 0094）の検証と、3 モジュール構成の停止点の測定

状態: **実装と測定の記録**（決定は `docs/decisions/0094-msvc-switch-base-reaching-defs.md`）。作業物は `lift/out-rdefs/`（`stats.sh`、`analysis/table_facts.py`、`logs/`、`test-ws.txt`）。ビルドは `CARGO_TARGET_DIR=lift/out-rdefs/target`、`TMPDIR=lift/out-rdefs/tmp`。

凡例: **実測** = コマンドの出力を見た事実。**読み** = 実測から読んだ解釈。**未確認** = 確かめていない。**意見** = 作業者の意見。

記入状況（この文書は節ごとに追記して保存する）: §1 実装、§2 テスト、§3 統計の前後と見積もりとの比較、§4 解決した site の機械検査、§5 FLEngine、§6 3 モジュールの測定 = 記入済み。§7 限界 = 後で記入。

## 1. 実装（実測。ソースを読んだ事実）

HEAD ded27a4。変更したソース: `lift/crates/fe-decode/src/jumptab.rs`、`fe-decode/src/lib.rs`（コメント 1 行）、`fe-lift/src/main.rs`（フラグ 1 つ）。

- `BaseProof` は `ReachingDefs`（既定。決定 0094）、`Dominating`（決定 0090。比較専用）、`EntryBlock`（決定 0089。比較専用）の 3 つ。
- `prove_base_reaching_defs`（新規）。命令のインデックスを定義の名前にした前向きのデータフロー（不動点。作業リスト方式）。辺は `cfg_succ`（`lea_dominates_jmp` の `succ` をそのまま関数に切り出したもの。fall-through、関数内の直接分岐、この jmp 自身の表の移動先）。他の間接 jmp（`FlowControl::IndirectBranch`）は 1 つの仮想ノード（集合 1 つ）で、そこへ流れ込む定義は「他の間接 jmp に届く定義の和集合」。この集合は作業リストが空になったときに、すべての命令の入力へ 1 回合流させる（変化したときだけ繰り返す）。辺を命令数に比例して作ることはしない。
- 定義の種類は `Lea`（`lea B,[rip+d]` で rip+d = ImageBase）、`Write`（それ以外の書き込み。ImageBase 以外の `lea` を含む）、`Call`（B が揮発のときの call）。32 ビット以上の無条件書き込みは前の定義を置き換える（kill）。8 ビット・16 ビット・条件付き書き込み（`cmov` 等）は置き換えず追加する。`push B` は定義に数えず、`pop B` は同じ関数に `push B` があるときだけ数えない（0090 と同じ）。
- 判定: jmp の入力集合が空（jmp が入口から届かない）→ 拒否。`Write` が含まれる → 拒否。`Call` が含まれる → 拒否（B が揮発のとき、call を定義として数えるので、`lea` から jmp までの経路上に call があれば `Call` が残る）。集合が入口の定義だけ（書き込みなし）→ 拒否（`no definition ... reaches the jump`）。入口の定義が混ざる（B が未初期化の経路がある）→ 拒否（`is not defined on every path`）。それ以外（すべて `Lea` で 1 つ以上）→ 証明。
- 決定 3（call の置換）は**実装した**（P4c は選んでいない）。理由は §1.1。
- `fe-lift`: `--msvc-base-dominating`（決定 0090 を選ぶ。以前は「受け付けて何もしない」だったので意味を直した）、`--msvc-base-entry-block`（決定 0089。既存）。**名前の衝突はない**（既存の 2 つのフラグ名のまま。`--msvc-base-dominating` の意味だけが変わった: 以前は既定と同じ = 0090、今は 0090 を明示的に選ぶ。既定は 0094）。
- 変えていない: `recognize_msvc`、`resolve_msvc`、`verify_msvc`、`foreign_entries`、`landings_in_idioms`、`lea_dominates_jmp` の本体、`read_msvc_targets`、決定 0093 の `nop`、`lift/runtime`。

### 1.1 call の規則（経路単位）の安全側の説明（読み）

- 0089 の規則は「関数内に call があり B が揮発なら拒否」（関数全体）。0094 は B が揮発のとき**すべての call を B の定義**として数える。call は前の定義を置き換える（kill）ので、call の後で `lea` が B を出し直せば `lea` が置き換え、call は届かない。call が `lea`（または入口）から jmp への経路上にあれば、jmp に届く定義に `Call` が残り、拒否される。つまり「B が揮発で、B の値が call をまたいで jmp に届く」経路が 1 つでもあれば拒否する。これは経路単位の同じ主張（call の呼び先が B を変えうるので、call をまたいだ B は ImageBase と言えない）の、より細かい表現。
- 安全側である理由: (1) call は常に定義として扱う（呼び先が B を保つ可能性は見ない）。(2) 間接 call も同じ。(3) CFG に呼び先の中は入れない（呼び先は B を別の値にして戻るかもしれないという扱いが call を定義にすること）。(4) call が戻らない（`noreturn`）経路でも定義として流れるので、過大評価。(5) 仮想ノード（他の間接 jmp が任意の命令へ着地）を通って流れる定義にも `Call` が含まれうるので、他の間接 jmp に届く call も拒否の理由になる。callee-saved（rbx、rbp、rdi、rsi、r12〜r15）は call をまたいで保たれるとみなす（Win64。0089 のとおり、`dataflow.rs` の流儀）。callee が保存しない場合は仮定が崩れる（0089 から続く限界）。
- 関数全体の規則を残す（P4c）案は選ばなかった。実装は難しくなく（上のとおり call を定義として数えるだけ）、実データでは call をまたぐ site が 0 件（調査 `msvc-base-reaching-defs.md` §2.2）なので、P4 の置換で新しく通る site が call をまたぐ形になることはない。実測で `Call` が理由の拒否は §3 を参照。

## 2. テスト（実測）

`cargo test --workspace --no-fail-fast`（`FLENGINE_PE` を指定。`CARGO_TARGET_DIR=lift/out-rdefs/target`。ログ `lift/out-rdefs/test-ws.txt`）: **98 binaries / 1,228 passed / 0 failed / 0 ignored**（基準 98 / 1,213 / 0 / 0。**+15 件**）。SKIPPED の表示 0 件。警告 2 件はこの作業が触っていない既存のテスト（`sse_shuf.rs`、`monitor.rs`）のもの。

内訳（+15 = 追加 − 置き換え）:

- `fe-decode/tests/msvc_switch.rs` 45 → 57（+12）、`fe-lift/tests/msvc_switch.rs` 15 → 18（+3）。`fe-decode/tests/jt_cpu.rs`、`fe-lift/tests/jt_cpu.rs` は件数変わらず（期待値の更新のみ）。
- 合成 PE（`msvc_common/mod.rs` に `Lea::{LeaOrArg, Never}`、`SideWrite`、`CallAt`、`OtherJmp::{ReachedReg, ReachedAfterWrite, ReachedAfterWriteMem, ReachedBeforeLea, UnguardedSwitch}` を追加。既定は従来どおり）。
  - (a) 書き込み 2 回で到達定義が `lea` だけ（`f_48a2060` と同形: `ja` の行き先の枝の先頭で B を書き、`ret` で終わる）が**解決される**: `a_write_on_a_branch_that_never_reaches_the_jump_is_not_a_definition_that_reaches_it`（`SideWrite::InDef`、`InCase`、揮発 B = r10、`lea` が 2 つの経路に 1 つずつ）。同じ形は 0090（`BaseProof::Dominating`）では `is written 2 times` で拒否されることも固定。書き込みが jmp の自身の表を通って jmp に戻る形（`InCaseLoop`）は拒否: `a_write_that_comes_back_to_the_jump_through_the_jumps_own_table_is_rejected`。
  - (b) 他の間接 jmp があり（`jmp rdx`、`jmp [rdx]`、到達する `jmp rdx`、`and eax,3` で添字を作る N が導けない表）、`lea` 以外の定義がそれに届かないものが**解決される**: `another_indirect_jump_that_no_other_write_reaches_does_not_stop_the_proof`。N が導けない表は `N not derived: no ja before the table idiom` のまま（決定 4）。第 2 の解決済み switch がある関数も両方解決される: `a_second_switch_is_a_wildcard_jump_and_both_are_resolved_by_reaching_definitions`（P3 ではなく P4）。
  - (c) `lea` 以外の定義が他の間接 jmp に届くものが**拒否される**: `a_definition_that_is_not_lea_and_reaches_another_indirect_jump_rejects_the_proof`（`movzx B,[rdx+0xe]` の後に `jmp rdx`／`jmp [rdx]`。`lea` が diamond の後でも）。
  - (d) jmp に届く定義に `lea` 以外が含まれる（条件分岐で B が `lea` か `mov B,rcx` になって合流）ものが**拒否される**: `a_definition_that_does_not_come_from_a_lea_of_the_image_base_among_the_reaching_ones_rejects_it`（ImageBase 以外の `lea` も）。
  - (e) 定義が届かない経路があるものが**拒否される**: `b_that_is_undefined_on_some_path_to_the_jump_is_rejected`（B を一度も書かない、`lea` が片側だけ、`lea` の前の分岐が間接 jmp に届き入口の B が流れる形）。
  - (f) call の規則: `a_call_on_a_path_from_the_lea_to_the_jump_rejects_a_caller_saved_base_only`。callee-saved（r12、r13）は call が経路上にあっても通る。揮発（r8、r11）は `lea` と jmp の間の call、jmp の自身の表を戻る経路上の call で拒否される。**`lea` の前の call、jmp に戻らない枝（`ret` で終わる）の call は拒否の理由にならない**（0090 の関数全体の規則は拒否する）。
  - 比較専用の 2 方式: `decision_0090_is_kept_for_comparison`（0090 と 0094 が同じ site を解決する形、0089 の入口ブロックの条件、`ReachingDefs` が `Options::default()`）。
- 実 DLL（fe-decode）: `blend2d_the_reaching_definitions_of_decision_0094_resolve_the_three_sites_of_0090_and_27_more`（種なしの Blend2D で 30 site = 0090 の 3 + 新規 27。27 の VA を固定。0090 の 3 site の表は同一。全 case が閉包の命令の先頭で関数のピースの中。`msvc_rejected` 49。N が導けるのに非 `lea` の定義が他の間接 jmp に届く 6 site は `reaches the jump` で拒否）、`blend2d_f_48a2060_is_resolved_by_reaching_definitions_with_the_sixteen_entries_of_the_table`、`dsp_ippv2_f_37cc7e0_is_resolved_by_reaching_definitions_with_the_sixteen_entries_of_the_table`（`0x37cc80b` の 16 エントリ。dsp_ippv2 の MSVC switch 5 site の VA を固定）。
- 実 DLL（fe-lift。native と生成 C）: `real_blend2d_f_48a2060_every_size_reaches_the_table_target_natively_and_lifted`、`real_dsp_ippv2_f_37cc7e0_every_size_reaches_the_table_target_natively_and_lifted`（§4.2）。
- **既存の期待値を変えた行とその理由**（既定の `BaseProof` が 0090 → 0094 に変わったため。比較専用の方式を渡す形に直したものと、新しい既定の結果に直したもの）:

| ファイル | テスト | 変更 | 理由 |
|---|---|---|---|
| `fe-decode/tests/msvc_switch.rs` | `a_lea_on_each_path_is_two_writes_and_is_not_proven` → `…_by_dominance` | `rejected_with(…, "is written 2 times", Dominating)` に。0094 では `Lea::BothSides` は解決される（`resolved_ok`、新しいテストで確認） | 0090 の「書き込み 2 回」の期待を比較用フラグで保つ |
| 同 | `another_indirect_jump_in_the_function_means_dominance_is_not_proven` | `run_with(…, Dominating)`、`rejected_with(…, Dominating)` | 0090 の項目 2（他の間接 jmp があれば拒否）の期待を `Dominating` で保つ。0094 では解決される |
| 同 | `a_second_resolvable_switch_in_the_function_is_not_allowed_either` → `…_by_dominance` | `run_with(…, Dominating)` | 同上。0094 は両方を解決（新しいテスト） |
| 同 | `base_lea_on_one_path_only_is_not_proven` | 既定の理由を `is not defined on every path` に。`Dominating` の旧理由（`does not run before the jump on every path`）も固定 | 0094 の拒否理由の文言が違う（拒否は同じ） |
| 同 | `base_written_twice_is_not_proven` | `Lea::Twice` は既定で `reaches the jump`（`xor` が届く）、`Dominating` で `is written 2 times`。**`push_pop: false, pop_only: true`（`pop B` が `push B` なしで jmp の後）は 0094 では解決される**（`resolved_ok`）。`Dominating` では従来どおり拒否 | `pop B` は jmp より後の定義で、jmp に戻る経路が無い。0090 は書き込み 2 回で拒否 |
| 同 | `the_padding_does_not_change_the_rejections_of_the_other_conditions` | `lea` 片側・`Twice` の理由文言、`OtherJmp::Reg` → `ReachedAfterWrite`（`jmp:unresolved` 2） | 同上 |
| 同 | `blend2d_without_seeds_resolves_the_sites_by_dominance_including_the_one_that_stopped_the_run` | `analyze_blend2d(…, Dominating)` に。3 site | 0090 の期待を比較用で保つ |
| 同 | `blend2d_f_48a2060_with_the_padding_allowed_the_site_is_still_not_resolved_by_the_base_proof` → `…_by_dominance` | `analyze_blend2d(…, Dominating)` に | 同上。0094 は解決（新しいテスト） |
| `fe-decode/tests/jt_cpu.rs` | `dsp_ippv2_has_434_stubs` | `msvc_tables` 1 → 5、`jumptabs` 435 → 439、`jmp:unresolved` 548 → 544、`jmp:msvc_table` 1 → 5 | dsp_ippv2（`--pdata`）で 0094 が 4 site を解決（§3、§4） |
| `fe-lift/tests/jt_cpu.rs` | `real_exe` の `assert_eq!(st.jumptabs, 434)` | `(jumptabs, msvc_tables) == (437, 3)` | 434 個のスタブの種で到達する 3 site（`0x37cc80b`、`0x37d9670`、`0x37d986a`）を 0094 が解決 |
| `fe-lift/tests/msvc_switch.rs` | `unresolved_sites_stay_a_tail_dispatch` | 解決されない形の一覧を 0094 の拒否理由の形に（`ReachedAfterWrite` ほか）。`OtherJmp::Reg`、`Mem`、`Lea::BothSides` は外した（0094 で解決される）。`jmp reg` の件数を数える | 同上 |
| 同 | `padded_sites_that_are_not_resolved_stay_a_tail_dispatch` | `OtherJmp::Reg` → `ReachedAfterWrite` | 同上 |
| 同 | `real_blend2d_site_is_resolved_by_default` | `msvc_tables`、`jumptabs` 3 → 30。0090 の比較は新しいテスト `real_blend2d_decision_0090_is_kept_for_comparison_and_resolves_the_three_sites` に移した（3 site、0094 の C と 3 site の `switch` の行が同一） | 0094 が既定 |

- **変異検査**（実施。元に戻して `cmp` で一致を確認）: (1) 仮想ノード（他の間接 jmp の合流）を外す → 5 件が失敗、(2) call の定義を外す → 3 件が失敗、(3) 定義が前の定義を置き換えない（kill しない）→ 32 件が失敗。ログ `lift/out-rdefs/mut1.txt`、`mut2.txt`、`mut3.txt`。

## 3. DLL ごとの統計の前後（実測）

前 = HEAD の `fe-lift`（`lift/out-git/target-m93/debug/fe-lift`。SHA-256 `afc53360…`。読み取り専用）。後 = この作業のビルド（`lift/out-rdefs/target/release/fe-lift`）。生成 C は `/dev/null`（`lift/out-rdefs/stats.sh`。ログ `logs/stats.{before,after}.*.txt`、表 `logs/stats-table.txt`、理由別 `logs/reasons-after.txt`）。入力 DLL は調査（`msvc-base-reaching-defs.md` §0.1）と同一（SHA-256 一致）。

| 構成 | closure_instructions | functions | bad_nodes | unsupported_total | register_jmp_sites | jumptabs | msvc_tables | msvc_rejected |
|---|---|---|---|---|---|---|---|---|
| Blend2D 種なし (`--pdata`) | 399,148 → 405,045 | 2,018 → 2,019 | 0 → 0 | 596 → 596 | 99 → 99 | 3 → 30 | 3 → 30 | 76 → 49 |
| Blend2D 種 58 個 (`--pdata --seeds`) | 399,777 → 405,674 | 2,066 → 2,067 | 0 → 0 | 599 → 599 | 100 → 100 | 3 → 30 | 3 → 30 | 77 → 50 |
| dsp_ippv2 (`--pdata`) | 6,465,008 → 6,465,171 | 32,648 → 同 | 0 → 0 | 974,527 → 同 | 836 → 同 | 435 → 439 | 1 → 5 | 10 → 6 |
| dsp_ippv2 縮小（種 452） | 3,038,781 → 3,038,921 | 8,030 → 同 | 0 → 0 | 391,396 → 同 | 563 → 同 | 434 → 437 | 0 → 3 | 7 → 4 |
| QFC (`--pdata`) | 209,259 → 同 | 6,712 → 同 | 0 → 0 | 104 → 同 | 5 → 同 | 0 → 同 | 0 → 同 | 0 → 同 |
| FLEngine (`--pdata`) | 2,528,030 → 同 | 59,297 → 同 | 0 → 0 | 525 → 同 | 20 → 同 | 4 → 同 | 0 → 同 | 0 → 同 |

- **bad_nodes は 6 構成のどれも 0 のまま（増えなかった）**。誤デコードの調査は不要。`unsupported_total`、`register_jmp_sites` も不変。QFC と FLEngine は統計の全行が前後で同一。
- 種なしと種 58 個ありで、新しく解決される site の集合は**同一**（27 件）。種 58 個は Blend2D の site に影響しない（決定 5。種に依存しない）。
- Blend2D の `functions` が +1（2,018 → 2,019）: `f_4847320`（実測: 生成 C の `L_4842590: call f_4847320()` の呼び先。`0x4842590` は `f_4841870`（`0x48421c1` の switch を持つ関数）の中の命令で、解決前の C には無く（`L_4842590:` が 0 個）、解決後の C にある。**読み**: 新しく解決した switch の移動先から閉包が先へ進んで見つけた命令。どの case から `0x4842590` へ届くかは追っていない（**未確認**））。それ以外の関数は増減なし（`logs/fns.before.txt`、`fns.after.txt` の `comm`）。
- `--msvc-base-dominating`（決定 0090）をつけた後ビルドは、6 構成すべてで HEAD の `fe-lift`（前）と統計の全行が一致（`logs/stats.dom.*.txt`。`msvc_rejected` の「no `ja` …」の文言は 0093 で同じなので差なし）。比較用フラグが 0090 の挙動を保つ。

### 3.1 調査 §3.2 の見積もりとの比較（実測と、調査の静的な見積もり）

調査の見積もりは解析器（Python）の静的な数で、実装との突き合わせは未実施だった（調査 §7）。

| | 調査の見積もり（net） | 実装（0094。決定 3 を実装） | 差 |
|---|---|---|---|
| Blend2D の新規解決（0090 の 3 に加えて） | P4 **26**（生の 28 から `0x47639ae`、`0x4889443` を既存の検査が落とすと見積もった）。P4c 10 | **27** | **+1**: `0x4889443` |
| dsp_ippv2（`--pdata`）の新規解決 | P4 **4**（`0x37b25bd`、`0x37cc80b`、`0x37d9670`、`0x37d986a`）。P4c 3 | **4**（同じ 4 site） | 0 |
| dsp_ippv2 縮小（種 452）の新規解決 | P4 3（調査 §3.2 の「到達 3」）、P4c 3 | **3**（`0x37cc80b`、`0x37d9670`、`0x37d986a`） | 0 |
| `0x48a208b`（`f_48a2060`）、`0x37cc80b`（`f_37cc7e0`） | 通る | **通る** | 0 |
| 0090 の 3 site（`0x4701384`、`0x4701cbb`、`0x470a89e`）と dsp の `0x37c69eb` | 通る | 通り、表も同一 | 0 |
| `0x47639ae` | 既存の検査（`landings_in_idioms`）が落とす | **落ちた**（`the branch at 0x47652a0 lands at 0x47639a3, inside the switch idiom`） | 0 |
| 非 `lea` の定義が他の間接 jmp に届く 7 site | P4 で拒否 | `0x4745efd` は閉包に未到達（拒否の行なし。0090 でも同じ）、他の **6 site は `reaches the jump` で拒否**（`0x4745cf4`、`0x4746512`、`0x488be9b`、`0x488e4e0`、`0x4890e43`、`0x489345a`） | 0 |

- **差の 1 件（`0x4889443`）の原因**: 調査は「他の `.pdata` 関数の `0x487e0f5` が `jcc 0x487e10e` で関数 `f_487e100` の途中へ飛び込む。`foreign_entries` が落とす」と見積もった（`rdef-idump --xrefs`。iced-x86 の**全イメージの線形走査**で数えた分岐）。実際は `0x487e0f5` は直前の関数の **RVA 表のバイト**（実測: `0x487e0b8`〜`0x487e0fc` は 4 バイトずつの RVA で、`0x487e0f4` の 4 バイトは `3c e0 17 00` = RVA `0x17e03c`。その 2 バイト目からの `e0 17` を iced-x86 の線形走査が `loopne 0x487e10e` と読んだ）偽の分岐で、**閉包はその番地に命令を持たない**（生成 C に `L_487e0d…`〜`L_487e0f…` のラベルが 0 個）。`foreign_entries` は閉包が知る分岐だけを見る（調査 §3 の注記どおり「closure が知っている分岐は静的に全イメージで数えた分岐の部分集合」）ので、この偽の分岐は落とさず、site は解決された。**読み**: この site の移動先 9 件は関数内の命令の先頭で（§4）、閉包に偽の分岐は無いので、解決は妥当。**意見**: 調査の net は下限寄りの見積もりという注記の範囲内の差（`+1`）。
- P4 と P4c の差（調査: 関数全体の call 規則を残すと Blend2D で 28 → 10）: **実装は決定 3（経路上の call）を実装したので P4 側の数**。call の規則で拒否された site は **0 件**（`msvc_rejected` に `lies on a path from` の行なし）。調査 §2.2 の「call をまたぐ揮発 B は 0 件」と一致。

### 3.2 `msvc_rejected` の理由別（実測。`logs/reasons-after.txt`）

| 理由 | Blend2D 種なし 前 → 後 | dsp_ippv2 (`--pdata`) 前 → 後 | dsp_ippv2 縮小 前 → 後 |
|---|---|---|---|
| B が N 回書かれる（決定 0090） | 34 → **0** | 4 → **0** | 3 → **0** |
| N が導けない（`no ja before the idiom` ほか 3 種の合計） | 40 → 40 | 6 → 6 | 4 → 4 |
| jmp が `.pdata` の関数の外 | 2 → 2 | 0 → 0 | 0 → 0 |
| **0094: 非 `lea` の定義が（他の間接 jmp 経由で）届く** | 0 → 6 | 0 → 0 | 0 → 0 |
| `landings_in_idioms`（`0x47639ae`） | 0 → 1 | 0 → 0 | 0 → 0 |
| 合計 | 76 → 49 | 10 → 6 | 7 → 4 |

「B が N 回書かれる」34 件はすべて新しい規則で解決または別の理由（上の 6 + 1）に移り、**0 件**になった。N が導けないものは 0094 の範囲外（決定 4）なので不変。

## 4. 解決した site の機械検査（実測）

### 4.1 全 site の全エントリ（`lift/out-rdefs/analysis/check_sites.py`。ログ `logs/sites-b2d0.txt`、`logs/sites-dsp.txt`）

生成 C（Blend2D 種なし `--pdata`、dsp_ippv2 `--pdata`）の `switch` の行から case の VA を読み、PE のバイト列から次を検査した（capstone の線形走査と `.pdata`/`.xdata` の自前のパーサ。fe-decode のコードは使っていない）:

1. 全 case が、その site の関数（`.pdata` の RUNTIME_FUNCTION＋連鎖の全ピース）の線形走査の**命令の先頭**、2. 全 case が関数のピースの中、3. 全 case に生成 C のラベル `L_<VA>:` がちょうど 1 つ、4. case が互いに異なり、既定が `fle_jumptab_miss(UINT64_C(<site>), t)`。

| | site 数 | case の総数 | 失敗 |
|---|---|---|---|
| Blend2D 種なし | 30（0090 の 3 + 新規 27） | 272 | **0** |
| dsp_ippv2 `--pdata` | 5（0089 の 1 + 新規 4） | 68 | **0** |

新しく解決された site（VA。`msvc_rejected` から消えたもの。種あり・種なしで同一）:

- **Blend2D（27）**: `0x470749d`、`0x4707d3a`、`0x470b983`、`0x470c607`、`0x470d019`、`0x47210ff`、`0x4722765`、`0x4724748`、`0x4725144`、`0x4725db7`、`0x4728561`、`0x47290af`、`0x4760a33`、`0x476b168`、`0x476cf0a`、`0x47802d9`、`0x478129c`、`0x47aa0bc`、`0x47aa54e`、`0x48421c1`、`0x48654a6`、`0x48712cf`、`0x487d636`、`0x4889443`、`0x48a208b`、`0x48aef10`、`0x48af10a`。
- **dsp_ippv2（4）**: `0x37b25bd`、`0x37cc80b`、`0x37d9670`、`0x37d986a`（縮小の構成では `0x37cc80b`、`0x37d9670`、`0x37d986a` の 3）。

### 4.2 `f_48a2060`（Blend2D `0x48a208b`）と `f_37cc7e0`（dsp_ippv2 `0x37cc80b`）の 16 エントリの個別照合（実測）

- 表（RVA `0x2fd050`、`0x2615000`）の 16 エントリを**ファイルのバイト列から**読み、capstone と objdump の**両方**で全 16 件を個別に照合した（`analysis/table_facts.py`。ログ `logs/table-facts.txt`）: 16 個とも異なり、関数内で、capstone の命令の先頭かつ objdump の命令の先頭（関数の命令数は capstone 346、objdump 346 で一致）。

Blend2D（r8 → 移動先。命令）: 0 → `0x48a208e`（`ret`）、1 → `0x48a2174`（`movzx ecx,byte [rdx]`）、2 → `0x48a20c8`（`movzx ecx,word [rdx]`）、3 → `0x48a20ff`（`movzx ecx,word [rdx]`）、4 → `0x48a217a`、5 → `0x48a215f`、6 → `0x48a2150`、7 → `0x48a20d0`、8 → `0x48a216d`、9 → `0x48a2135`、10 → `0x48a2126`、11 → `0x48a20b0`、12 → `0x48a2143`、13 → `0x48a2110`、14 → `0x48a20e8`、15 → `0x48a2090`。dsp_ippv2（同じ関数。基底 0x2000000）: 0 → `0x37cc80e`、1 → `0x37cc8f4`、2 → `0x37cc848`、3 → `0x37cc87f`、4 → `0x37cc8fa`、5 → `0x37cc8df`、6 → `0x37cc8d0`、7 → `0x37cc850`、8 → `0x37cc8ed`、9 → `0x37cc8b5`、10 → `0x37cc8a6`、11 → `0x37cc830`、12 → `0x37cc8c3`、13 → `0x37cc890`、14 → `0x37cc868`、15 → `0x37cc810`。

- **native と生成 C を r8 = 0..15 と範囲外 8 通り（16、17、31、32、33、0x100、0xffffffff、`u64::MAX`）の計 24 通りで動かした**（`fe-lift/tests/msvc_switch.rs`: `real_blend2d_f_48a2060_every_size_reaches_the_table_target_natively_and_lifted`、`real_dsp_ippv2_f_37cc7e0_every_size_reaches_the_table_target_natively_and_lifted`。0090 の `0x470a89e` のテストの流儀）。native は本物の命令を関数の入口から実行し、16 個の移動先と `ja` の行き先（`0x48a2180`、`0x37cc900`）に `0xCC` を置いて SIGTRAP ハンドラで止まった番地を出力。生成 C は同じ関数の 17 個のラベルにプローブ（`fle_hit`）を入れて入口から実行。**24 通りすべてで、native と生成 C の到達先が、ファイルの RVA 表から読んだ値と一致**（0..15 は表、16 以上は `ja` の行き先）。2 つの DLL で計 96 回（24 × 2 モード × 2 DLL）。生成 C は `switch` が 16 case で `fle_dispatch(R[` が無いことも確認。
- **実行の範囲**: 関数の入口から `rcx` = コピー先、`rdx` = コピー元、`r8` = サイズで実行した。コピーの結果（メモリの内容）は比べていない（比べたのは到達先）。

## 5. FLEngine（実測）

- `fe-lift lift ../binaries/win64/FLEngine_x64.dll out-rdefs/fle.c --pdata` の生成 C の SHA-256 は **`feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f`**（215,709,232 バイト）で、**期待どおり変わらない**。`lift/out-srw/fle.c`（0092 の測定の生成 C）と `cmp` でバイト一致。変わった関数 0。統計も前後で同一: functions 59,297、bad_nodes 0、unsupported_total 525、register_jmp_sites 20、jumptabs 4、msvc_tables 0、msvc_rejected 0。
- 単独構成の a4（基準 145/942、functions 821、entries 15,685、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`）: FLEngine の生成 C が変わらず runtime も変えていないので、runtime も C も同じ。**省略した**（実行していない）。

## 6. 3 モジュール構成の測定（測定のみ。gdb の仮の値なし、runtime の応答追加なし。新しい停止は直していない）

### 6.1 手順（実測）

- 生成: ワークツリーの release の `fe-lift`（`lift/out-rdefs/target/release`。決定 0094 の実装。runtime は HEAD ded27a4 のまま）。FLEngine の生成 C は `lift/out-srw/` のもの（`gen/flengine.c`、`src/fle-*`）を再利用し、**今回生成した `out-rdefs/fle.c`（SHA `feb5b908…`）と `cmp` で一致**を確認した。`gen_two.sh` も `EXPECT_FLE_SHA=feb5b908…` で検査して一致。縮小 dsp_ippv2（基底 0x2000000、種 452）、Blend2D（基底 0x4700000、`--pdata --seeds seeds58-4700000.txt` の種 58 個）。リベース済みの DLL は `lift/out-srw/three/dll/` のコピー（SHA-256 `24e01ec9…`、`4fd25502…`、`cf664439…`（dsp の種）、`ca6439b6…`（Blend2D の種）。一致）。スクリプトは `lift/out-srw/three/harness-two/`（`diff -rq` で無変更）と、決定 0093 の `out-nop93/three/run-*.sh` に倣った `lift/out-rdefs/three/run-gen.sh`、`run-build.sh`、`run-a4.sh`。
- 生成 C: 縮小 dsp_ippv2（`real.c`）は functions 8,030、closure 3,038,921、bad_nodes 0。Blend2D（`blend2d.c`）は functions 2,067、closure 405,674、bad_nodes 0。表の統合: 59,565 + 8,298 + 2,335 → 69,662 行（前回 69,661。複数の表に在る行 268、`f_<VA>` の衝突 0）。two-native は 612,729,728 B（2 GB 未満）。ビルドの stderr 0 行。
- a4-survey を 2 回（`out-rdefs/three/out-1/`、`out-2/`。07:27:10Z、07:27:14Z）。読み取りだけの gdb（`lift/out-rdefs/gdb/ro_srw3.py` = `lift/out-srw/gdb/ro_srw3.py` の無変更のコピー（`cmp`）。`return`、`set var`、`call`、値の書き込みなし）を 2 回（`lift/out-rdefs/run/`）。

### 6.2 a4 の行（2 回。実測）

```
A4 survey 20261025T072710Z   dll: /home/ubuntu/work/signature-studio-develop/binaries/win64/FLEngine_x64.dll   harness exit: 134
static : units=942 runnable=895 empty=46 noproc=1 (header 0x13768b0, units 0x13768e0)
stop   : fle: indirect call to 0x489f940 is not a known function entry
         top of stack: 0x489f7d0 f_489f7d0
depth  : init_count=150 / 942   last unit: index=149 run_rank=137/895 proc=0x664260 in_progress
         started=137 completed=136 in_progress=1 of 895 runnable   all_units_done=no
         entry check: started_but_not_entered=0 runnable_entered_beyond_init_count=0
funcs  : executed (distinct)=1069  entries=27333
```

- 2 回の出力は時刻と `logs` 行の OUT 名以外**同一**（stderr は `cmp` で一致）。
- **前回（決定 0092 の測定、0093 でも同じ）からの変化**: init_count 150/942、last unit 149（proc `0x664260`）、functions_distinct 1,069、started 137 / completed 136 は**同じ**。function_entries 27,332 → **27,333**（+1）。**停止が `fle: indirect call to 0x48a20ff is not a known function entry`（top of stack `f_48a2060`）から `fle: indirect call to 0x489f940 is not a known function entry`（top of stack `f_489f7d0`）に動いた**。
- **次の停止メッセージ全文**: **`fle: indirect call to 0x489f940 is not a known function entry`**（`abort kind: fle_dispatch_miss`、`dispatch target: 0x489f940`、`call depth at stop: 20`）。stderr も同じ 1 行。
- **C のスタック**（先頭 10 行。`out-1/a4/*-native.txt`）:

```
#0 0x489f7d0 f_489f7d0 guest-return=0x489daac
#1 0x489d880 f_489d880 guest-return=0x476ee62
#2 0x476ed70 f_476ed70 guest-return=0x489b08c
#3 0x4899d90 f_4899d90 guest-return=0x4701009
#4 0x4701000 f_4701000 guest-return=0x48a59eb
#5 0x48b5b90 f_48b5b90 guest-return=0x48a59eb
#6 0x48a59a8 f_48a59a8 guest-return=0x489e34e
#7 0x489e2b4 f_489e2b4 guest-return=0x489e4c4
#8 0x489e264 f_489e264 guest-return=0x489e4c4
#9 0x489e450 f_489e450 guest-return=0xcb000000
```

  前回の `#0 f_48a2060` が `#0 f_489f7d0` に変わり、`#1` 以降（`f_489d880` ← `f_476ed70` ← `f_4899d90` …）は前回と同一。`f_489d880` は `0x489daa7: call 0x489f7d0`（戻りアドレス `0x489daac`。objdump で確認）で `f_489f7d0` を呼ぶ（前回の `f_48a2060` の呼び出しは `0x489da6b: call 0x48a2060`）。**`0x48a20ff` は閉包のラベルになった**（調査 §4.6 の読みどおり。生成 C に `L_48a20ff: …` が 1 つ。`0x48a208b` は 16 case の `switch`）。**停止は `0x48a20ff` から動いた**ので、「停止がそこから動かなければ原因を調べる」の場合ではない。

### 6.3 新しい停止点（実測と読み）

- **gdb の停止時のレジスタ（読み取り）**: `rcx = 0x48f32d6`、`rdx = 0`、`r8 = 0xb`（= 11）、`r9 = 0x489f940`、`rax = 0x48f32cb`。
- `f_489f7d0`（`mov r11,rcx; movzx edx,dl; movabs r9,0x0101010101010101; imul r9,rdx; cmp r8,0x10; jbe 0x489f8e0 …` と始まる関数。**memset 風**に見える（`rcx` = 先頭、`dl` = 埋める値、`r8` = サイズ）のは**読み**。実行して確かめていない）の `0x489f8fb: jmp r9` が、解決されていない `jmp reg` として `fle_dispatch(R[9]); return;`（生成 C の `L_489f8fb`）になり、`r9 = 0x489f940` が既知の関数入口でないので `fle_dispatch_miss` で止まる。停止時のサイズ `r8 = 11`。
- **この `jmp r9` の形（実測。objdump と capstone）**:

```
489f8e0: mov rdx, r9
489f8e3: lea r9, [rip-0x19f8ea]                ; r9 = 0x4700000 = ImageBase
489f8ea: mov eax, [r9+r8*4+0x2fd000]
489f8f2: add r9, rax                           ; 基底に表のエントリを足し、結果が r9 に残る（基底のレジスタ自身がジャンプレジスタ）
489f8f5: add rcx, r8
489f8f8: mov rax, r11
489f8fb: jmp r9
```

  表（RVA `0x2fd000`）の 17 エントリを読むと、r8 = 11 は `0x489f940`（`mov [rax],rdx; mov [rax+8],dx; mov [rax+10],dl; ret`。objdump で確認。17 エントリという数は、`jbe` の `cmp r8,0x10` から読んだ上限 16 + 1 で、表の長さをバイト列から導いたものではない）。`recognize_msvc` は `jmp rT` の直前が `add rT,B`（B は rT と別のレジスタ）で、その前が `mov eT,[B+I*4+T]` の形（eT = rT のレジスタ）を要求する。**この site は、足されるレジスタ（`add r9,rax` の `rax`）が基底ではなく、基底が `r9`（= ジャンプレジスタ）自身で、`add rcx,r8` と `mov rax,r11` が `add` と `jmp` の間に挟まる**ので、この形に当たらない（**読み**: `recognize_msvc` のソースの条件と上のバイト列から。この site は `msvc_rejected` にも出ない = `Ok(None)`）。さらに `cmp r8,0x10; jbe` は `0x489f7e4` にあり、idiom の直前ではない（`jbe` の行き先が `0x489f8e0`）。**決定 0094 の範囲外**（決定 4: 表の形 (c1)/(c2)、N の導出は変えない）。
- **停止点の分類**: **(v) その他**（未応答の import ではなく、生成器が解決しない MSVC 型の `jmp reg` で、`fle_dispatch_miss` で止まる。前回の `0x48a20ff`、その前の `0x470a8a0`、`0x48a57f8` と同じ種類。(ii) TLS、(iii) JIT、(iv) ロケールではない）。新しい停止は**直していない**。
- **gdb の import の列（2 回。`cmp` で全バイト一致）**: **Blend2D の DllMain(ATTACH) は戻ったか**: **戻っていない**（`DLLMAIN-RETURN reason=1: out of scope (did not return normally)`）。**LoadLibraryW が 0x4700000 を返したか**: **返していない**（`LOADLIBRARY-RETURN` は 1 回だけで `R[0]=0x2000000`（dsp_ippv2））。Blend2D の DllMain に入るまでの `fle_imp_*` は 1,525 回（前回と同じ）。**ATTACH 内の import は 183 回（通し番号 1526〜1708）で、前回（`blend2d-srw.md` §6.3）と通し番号・名前の組が全 1,708 件で一致**（`diff` で差なし。最後の 3 つは `GetSystemInfo`（1706）、`HeapAlloc`（1707）、`InitializeCriticalSection`（1708））。名前ごとの回数: `InitializeCriticalSectionAndSpinCount` 83、`GetProcAddress` 15、`HeapAlloc` 9、`EnterCriticalSection` 7、`LeaveCriticalSection` 7、`WideCharToMultiByte` 6、`MultiByteToWideChar` 6、`LoadLibraryExW` 6、`LCMapStringW` 4、`TlsSetValue` 3、`GetSystemInfo` 3、`GetStdHandle` 3、残りは 2 回以下。名前付き停止の import なし。`AcquireSRWLockExclusive`（1704）と `ReleaseSRWLockExclusive`（1705）は停止せず戻る。停止時の CELLS（`[0x48f3280]`、`[0x48f3288]`、`[0x48f3290]`、`[0x48f31a0]`）は 0 のまま。
- **import の列が同一**であることの意味（読み）: `f_48a2060` の `jmp r9` を解決したことで先へ進んだ分は、import を呼ぶ前に次の `jmp reg`（`f_489f7d0`）で止まる。ATTACH で呼ばれる import の列は前回と変わらない。
- unit 158（`quickfontcache_x64.dll` の `LoadLibraryW`）には届いていない（init_count 150/942。止める条件の手前）。

## 7. 解釈が分かれた点、実装できなかった点、限界

### 7.1 解釈が分かれた点（決定の内容は変えていない）

1. **決定 3（call の置換）は実装した（P4c は選んでいない）**。call は B が揮発のときすべて B の定義として数える（前の定義を置き換える）。安全側の説明は §1.1。
2. **定義の「置き換え」**: 32 ビット以上の無条件の書き込み（`mov r10d,…` は上位 32 ビットを 0 にする）は前の定義を置き換える。8 ビット・16 ビットの書き込みと条件付き書き込み（`cmov` 系、`setcc` は 8 ビットなので置き換えない）は前の定義を残して定義を足す。調査の解析器（`rdef_lib.py`）は `cmov` だけを「置き換えない」とした。実装は書き込みのレジスタ幅で分けた（iced-x86 の `used_registers` の `size()`）。**実データでの差**: 拒否の理由に `Sete`（`0x4745cf4`、8 ビットの書き込みで前の定義を残すので、`lea` と `sete` の両方が届く）が出る。
3. **`pop B`**: 0090 と同じく同じ関数に `push B` があれば定義に数えない。`push B` が無い `pop B` は定義（`Write`）。調査の解析器は `pop B` を常に定義に数えた（保守的）が、実装は 0090 に合わせた。0090 が書き込み 2 回で拒否した「`push B` なしの `pop B` が jmp の後にある」合成の形は、0094 では**解決される**（jmp に戻る経路が無いので届かない。§2 の表）。
4. **「他の間接 jmp」**: `FlowControl::IndirectBranch`（`jmp reg`、`jmp [mem]`。間接 call は含まない。0090 と同じ）。関数のピースの線形走査で数えるので、表のバイトが偶然の命令に見える場合も数える（拒否が増える方向。実データで `jmp [mem]` が拒否の理由になった site: `0x489345a`）。その jmp の入力状態が空（関数の入口から届かない）なら、仮想ノードへは何も流れない。
5. **入口の B**: 関数の入口の定義（呼び元の値）を `ENTRY` として 1 つ置く。jmp にそれが届けば拒否（`is not defined on every path`）。B が一度も書かれない場合は別の文言（`no definition … reaches the jump`）。
6. **`--msvc-base-dominating`**: 以前は「受け付けて何もしない」（既定と同じ = 0090）で、今回 0090 を明示的に選ぶ意味に直した。**名前の衝突はない**。0090 の usage の文面（`--msvc-base-entry-block` だけを載せていた）を、両方を載せる形にした。
7. **作業者が書いたテスト用の拡張**: 合成 PE の部品（`SideWrite`、`CallAt`、`OtherJmp` の 5 つ、`Lea` の 2 つ）。既定の `Sw::c1()`、`c2()` は変えていない。

### 7.2 実装できなかった点

PM の決定のうち実装できなかった項目は**無い**。

### 7.3 限界

- **線形走査の CFG**: 0090 と同じ。関数のピース内でデコードできる命令だけで作る。ピースの外への直接分岐は辺を持たない。SEH のハンドラ／フィルタ、`longjmp`、例外による遷移は見ない。
- **他の間接 jmp の「任意の命令へ着地」は過大評価**（実際の移動先は表の中。安全側）。この過大評価のために**拒否されるが実際は安全な site** が出うる: 調査 §3.3 の D（実表で読むと届かない）で `0x4745cf4`、`0x4746512`、`0x488be9b`、`0x488e4e0`、`0x4890e43`、`0x489345a` の 6 site。実装でも拒否された（§3.1）。**緩める判断は PM**（PO は健全でない。P3 は実データでは何も足さない）。
- **N が導けない表**（`jmp r11` の 3 つ、`f_489f7d0` の `jmp r9`）は `fle_dispatch(R[x])` のまま（決定 4）。0094 が解決するのは (c1)/(c2) の形で N が導ける site だけ。N が導けない site は Blend2D で 40、dsp_ippv2 で 6（§3.2）。
- **call をまたぐ揮発 B**: callee-saved レジスタが call をまたいで保たれる仮定（Win64。0089 から）。call は定義として扱うが、呼び先の中は見ない。
- **実機の Windows での挙動は未確認**（ホストは Linux）。native との突き合わせは `f_48a2060`、`f_37cc7e0`、`0x470a89e`（0090 の 40 通り）の 3 関数だけ。新しく解決した他の 27 + 3 site は、バイト列（命令境界、関数内、ラベル）の機械検査（§4.1）に限る。実行して表どおりに動くかは**未確認**。
- 到達定義は **jmp の手前の B** についてだけで、`lea` の値が jmp まで保たれる（B が `lea` と jmp の間で別の値にならない）ことは、データフローの結果（定義の集合）で見ている。**表の中身（エントリ）の正しさ**は `read_msvc_targets`（変更なし）の検査に依る。
- 母集団: Blend2D、dsp_ippv2、QFC、FLEngine の 4 つ。同梱のほかの DLL は見ていない（0090 の限界と同じ）。
- 3 モジュールの測定は、ATTACH の途中で止まるまで。**新しい停止（`0x489f940`）は直していない**。その先の停止は未確認。
- 基底 0x4700000（Blend2D）と 0x2000000（dsp_ippv2）、種 58 個は測定用の値（決定 0080 D1）。この実装はそれらに依存しない（種なしで Blend2D の 27 site が解決する。§3）。
- wasm、emcc は使っていない。

## 8. 作ったファイル

- 新規（追跡対象）: `docs/decisions/0094-msvc-switch-base-reaching-defs.md`、`docs/investigations/blend2d-switch-reaching-defs.md`（この文書）。
- 変更（追跡対象）: `lift/crates/fe-decode/src/jumptab.rs`、`lift/crates/fe-decode/src/lib.rs`（コメント 1 行）、`lift/crates/fe-lift/src/main.rs`、テスト `lift/crates/fe-decode/tests/{msvc_common/mod.rs,msvc_switch.rs,jt_cpu.rs}`、`lift/crates/fe-lift/tests/{msvc_switch.rs,jt_cpu.rs}`、`docs/README.md`（0094 の行）、`docs/decisions/0090-msvc-switch-base-dominance.md`（注記 1 行）。
- 作業物（`lift/out-rdefs/`）: `stats.sh`、`logs/`（統計の前後、理由別、サイト検査、表の照合）、`analysis/`（`table_facts.py`、`check_sites.py`）、`probe/`（生成 C の一部、`zz_probe_rdefs.rs.txt`）、`three/`（3 モジュール）、`gdb/`、`run/`、`test-ws.txt`、`jumptab.rs.good`（変異検査の前に保存した `jumptab.rs` のコピー）、`mut{1,2,3}.txt`、`target/`、`tmp/`。
