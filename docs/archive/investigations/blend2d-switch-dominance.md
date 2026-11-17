# MSVC switch 表の基底証明を dominance にする（決定 0090）: 実装前の見積もり、実装、検証、3 モジュールでの停止点の移動（測定のみ）

実行日: 2026-10-24。基準の HEAD は `d87e0ad`（ワークツリーの未コミットの変更が本文書の対象）。決定は `docs/decisions/0090-msvc-switch-base-dominance.md`（PM の決定。仮の決定ではない）。前提は決定 0089 と `docs/investigations/blend2d-switch-tables.md`（以下「前調査」。節番号はその文書のもの）。作業物は `lift/out-jtb2d90/`（gitignore 対象）。**この文書は何も決めない**。決定 0074、0079、0080、0086、0089 の他の項目は変えていない。基底 0x4700000 と種 58 個は測定用で、決定ではない。

## 0. 凡例

- **実測** = このセッションでコマンドを実行して得た値。ログは `lift/out-jtb2d90/` の下（各節に書く）。
- **読み** = ソース、生成 C、別の文書を読んだ結果。実行して確かめていない。
- **未確認** = そのとおり。

## 1. 要約（先に結論）

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | Blend2D の停止点 `0x470a89e`（`f_4709ea0` の `jmp rcx`）が**既定で**解決される | `lea r13,[ImageBase]` は 0x4709f18（入口ブロック 0x4709ea0〜0x4709efa の外）。0x4709efa の `jne 0x4709f07` のどちらの経路も 0x4709f18 を通る（objdump: 0x4709f05 の `je 0x4709f14` も 0x4709f14 → 0x4709f18 へ落ちる）。**`f_4709ea0` に他の間接 jmp は無い**（jmp は 6 個で、間接は `0x470a89e jmp rcx` の 1 個、残り 5 個は直接 `jmp rel`）。N = 31、表 6 エントリ、全移動先は関数内で命令の先頭。jumptabs 2 → 3 | 実測（§2、§4、§5） |
| 2 | 他の間接 jmp がある関数で、この規則**だけ**が理由で証明されなくなる site | **0 か所**（Blend2D 種なし・種 58 個あり、dsp_ippv2、dsp_ippv2 縮小、QFC、FLEngine のすべて）。他の間接 jmp を持つ関数にある 15 か所は、どれも先に「B が複数回書かれる」か「N が導けない」で落ちる | 実測（§2.2、§4.2） |
| 3 | 実装後の既定は、d87e0ad の測定専用 `--msvc-base-dominating` と**統計の全行が同一**（6 構成すべて）。生成 C も、Blend2D 種 58 個あり・dsp_ippv2・FLEngine で前回の `--msvc-base-dominating` の C と同一 | 他の間接 jmp の規則が実 DLL で解決を減らさなかったことを意味する | 実測（§4.2、§6.1） |
| 4 | FLEngine の生成 C の SHA-256 | **`43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1` のまま**（既定、`--msvc-base-entry-block`、`--msvc-base-dominating` のすべて）。変わった関数 0。functions 59,297、bad_nodes 0、unsupported_total 525、register_jmp_sites 20、jumptabs 4、msvc_tables 0、msvc_rejected 0 | 実測（§4.3） |
| 5 | テスト | `cargo test --workspace`: **97 binaries / 1,172 passed / 0 failed / 0 ignored**（基準 97 / 1,161。+11 件。binaries は増えない。SKIPPED 0 件） | 実測（§5.1） |
| 6 | 3 モジュール（種 58 個あり、基底 0x4700000）の停止点 | **`fle: unsupported instruction at 0x4899fed`**（`f_4899d90` の `call [rip+0x1d0d5]` = IAT 0x48b70c8 = `KERNEL32.dll!GetModuleHandleA`）へ移る。a4: init_count 150/942、last unit 149（proc 0x664260）、functions_distinct 1,058、function_entries **27,317**。2 回同一。ATTACH の import は **174 回**。DllMain は戻らず、LoadLibraryW は 0x4700000 を返さない。分類 (i)（未応答の import。行なし 15 名の 1 つ）。**前回の参考測定（`--msvc-base-dominating`）と、a4 の行、停止メッセージ、C のスタック、import の列がすべて同一** | 実測（§7） |
| 7 | 新しい停止 | 出なかった（予想どおり `GetModuleHandleA`）。直していない。`GetModuleHandleA` は実装していない | 実測 |

## 2. 実装前の見積もり

### 2.1 方法（実測）

d87e0ad の `git` ツリーの release ビルド（`lift/out-jtb2d90/target-base`）に、既定（入口ブロック）と `--msvc-base-dominating`（測定専用）をかけて 6 構成の統計を取った（`lift/out-jtb2d90/stats.sh` → `stats/base-entry.*`、`stats/base-dom.*`）。構成: Blend2D 種なし（`--pdata`）、Blend2D 種 58 個あり（`--pdata --seeds seeds58-4700000.txt`）、dsp_ippv2（リベース済み 0x2000000、`--pdata`）、dsp_ippv2 縮小（`--seeds seeds-idx0-plus-initterm2.txt`）、QFC（0x400000、`--pdata`）、FLEngine（`--pdata`）。入力の SHA-256 は Blend2D `24e01ec9…b4cb`、dsp_ippv2 `4fd25502…efc5`（決定の依頼の値と一致）。

95 か所（`lift/out-blend2d/jmpreg.txt` の `rva-switch`）については、前調査の `pre_survey3.py`（読み取り専用。`lift/out-jtb2d/analysis/`）を変更せずに読み込み、その上に「関数内の他の間接 jmp」を数える `lift/out-jtb2d90/analysis/survey90.py` を書いた（capstone。fe-decode に依存しない。ログ `logs/survey90_b2d.txt`）。実装との結合は `analysis/join95_90.py`（`logs/join95_90.txt`）。

### 2.2 見積もり（実測）: 決定 1〜2 の条件で解決できる数

実装前の基準（d87e0ad のビルド）:

| 構成 | 既定（入口ブロック、決定 0089） | `--msvc-base-dominating`（測定専用。他の間接 jmp の規則なし） |
|---|---|---|
| Blend2D 種なし | jumptabs 2、closure 398,980、msvc_rejected 77 | jumptabs **3**、closure 399,148（+168）、msvc_rejected 76 |
| Blend2D 種 58 個あり | 2、399,609、78 | **3**、399,777、77 |
| dsp_ippv2 | 435（msvc 1）、6,465,008、10 | 435（msvc 1）、6,465,008、10（変化なし） |
| dsp_ippv2 縮小 | 434（msvc 0）、3,038,781、7 | 同じ |
| QFC | 0、209,259、0 | 同じ |
| FLEngine | 4（msvc 0）、2,528,030、0 | 同じ |

95 か所（Blend2D 種なし）の内訳（`join95_90.txt`。最初に失敗した条件。閉包に未到達の 4 か所は「解決または未到達」に含まれる）:

| 結果 | d87e0ad 既定 | d87e0ad `--msvc-base-dominating` | **決定 0090 の既定（実装後）** |
|---|---|---|---|
| 解決（または閉包に未到達） | 23（うち解決 2、未到達と他の形 21） | 24（解決 3） | **24（解決 3）** |
| N が導けない | 40 | 40 | 40 |
| B が複数回書かれる | 33 | 33 | 33 |
| `lea` が入口ブロックの外 | 1（`0x470a89e`） | 0 | 0 |
| **他の間接 jmp があるために証明されない（決定 2）** | — | — | **0** |

（この表の「解決（または閉包に未到達）」は `jmpreg.txt` の 97 行のうち `msvc_rejected` に出てこないもの。形が (c1)(c2) でない 15 か所や、閉包に届かない 4 か所を含む。解決した数は上の統計の `msvc_tables`。）

**決定 2（他の間接 jmp があれば証明しない）の影響**（`survey90.py`）: (c1)(c2) で N が導ける 37 か所（`survey90.py` の一覧。基底の分類は入口ブロック 2、dominance のみ 1、その他 34）のうち、同じ関数に他の間接 jmp があるものは **15 か所**。関数と他の間接 jmp の数: `f_4745bd0`（3 か所の site、他の間接 jmp 3）、`f_4776f60`（1）、`f_47aa1a0`（1）、`f_4859b70`（1）、`f_4865f60`（1）、`f_4871d60`（1）、`f_487e100`（1）、`f_4889ef0`（1）、`f_488c8c0`（1）、`f_488ef00`（1）、`f_4891880`（2）、`f_48aeed0`（2 か所の site、他の間接 jmp 2）。**この 15 か所はすべて B が複数回書かれる**（書き込みは 7〜407 回、`lea` は 1〜8 回）ので、決定 2 の規則が働く前に決定 1' で落ちる。つまり決定 2 の規則は実 DLL で解決を 1 か所も減らさない。dominance の条件（`lea` が 1 回、他の書き込みなし）を満たし、かつ他の間接 jmp を持つ関数は、Blend2D、dsp_ippv2、QFC、FLEngine のどこにも無い（実装後の `msvc_rejected` に `other indirect jump` の理由が 0 件）。

**`0x470a89e`（`f_4709ea0`）の確認（必須。実測）**: `objdump -d -M intel --start-address=0x4709ea0 --stop-address=0x470abac` の全 989 行（`logs/objdump_f_4709ea0.txt`）で `jmp` は 6 個:

```
470a89e: jmp rcx        <- 間接（この switch）
470a8aa: jmp 0x470aafc
470aa2e: jmp 0x470aafc
470aa66: jmp 0x470aafc
470aaaf: jmp 0x470aafc
470ab4f: jmp 0x470ab54
```

他の間接 jmp は **無い**（`jmp reg` も `jmp [mem]` も 0x470a89e の 1 個だけ。`.pdata` の RUNTIME_FUNCTION は `[0x4709ea0, 0x470abac)` の 1 つで連鎖なし。UNWIND_INFO のフラグは 0）。`call` は無い（r13 は非揮発）。B = r13 への書き込みは `lea r13,[rip-0x9fb9f]`（= 0x4700000）が 0x4709f18 の 1 回だけ（`push r13` は数えない）。止まらずに進めた。

## 3. 実装（読み）

決定 0090 の「実装した場所」と同じ。変更は `lift/crates/fe-decode/src/jumptab.rs`（`git diff --stat` で 61 行の増減）、`lib.rs`（コメント 1 行）、`lift/crates/fe-lift/src/main.rs`（8 行の増減）だけ。`lift/runtime` は編集していない。

| 変更 | 内容 |
|---|---|
| `BaseProof` | `Dominating` を先頭にして `#[default]`。`EntryBlock` は残す（コメントで「比較のための測定専用。決定ではない」） |
| `prove_base`（`Dominating`） | 書き込み 1 回・`lea` が ImageBase・揮発 B と call の検査（0089 のまま）の後に、**この jmp 以外の間接 jmp（`FlowControl::IndirectBranch`。`jmp reg`、`jmp [mem]`）が関数の線形走査（全ピース）に 1 つでもあれば拒否**し、無ければ `lea_dominates_jmp`（既存。変更なし）で全経路の前にあることを検査する。拒否の理由文字列: `the function <entry> has N other indirect jump(s), the first `jmp REG`/`jmp [mem]` at <VA>: dominance of `lea B,[ImageBase]` at <VA> is not proven (decision 0090)` |
| `prove_base`（`EntryBlock`） | 0089 のまま（他の間接 jmp は見ない。比較用） |
| `lea_dominates_jmp` | 本体は変えていない（コメントだけ更新: 他の間接 jmp は `prove_base` が先に拒否するので、辺の集合は関数について完全） |
| `fe-lift` | `--msvc-base-dominating` は受け付けるが何もしない。`--msvc-base-entry-block` を足した（`BaseProof::EntryBlock`）。usage の表示を更新 |
| そのまま | `resolve_msvc` の順序（関数 → 基底証明 → 表と移動先）、`verify_msvc`、`foreign_entries`、`landings_in_idioms`、揮発 B と call、`recognize_msvc` |

読み: `lea_dominates_jmp` の CFG は、`ConditionalBranch` は分岐先と次の命令、`UnconditionalBranch`（直接）は分岐先、関数のピースの外への直接分岐は辺なし、`IndirectBranch` は jmp 自身（解決済みの表の移動先）だけに辺を持つ。他の間接 jmp は辺を持たないので、その jmp が `lea` を飛ばして switch の jmp へ着地する経路を辺として見落とす。**だから決定 2 は、そういう jmp が関数にあれば証明しない**（解決を減らす方向）。

## 4. DLL ごとの統計の前後（実測。release、`fe-lift lift ... /dev/null`）

### 4.1 前（d87e0ad のビルド、既定）→ 後（決定 0090 の既定）

| DLL / 引数 | 項目 | 前 | 後 |
|---|---|---|---|
| **Blend2D** 種なし `--pdata` | functions | 2,018 | 2,018 |
| | closure_instructions | 398,980 | **399,148**（+168） |
| | bad_nodes | 0 | **0** |
| | unsupported_total | 662 | 662 |
| | register_jmp_sites | 99 | 99 |
| | jumptabs / msvc_tables | 2 / 2 | **3 / 3** |
| | msvc_rejected | 77 | 76 |
| | `jmp_ind:reg_dispatch` / `msvc_table` | 97 / 2 | 96 / 3 |
| | gotos / c_lines / c_bytes | 31,908 / 411,895 / 34,699,913 | 31,922 / 412,063 / 34,717,559 |
| **Blend2D** 種 58 個あり | functions | 2,066 | 2,066 |
| | closure_instructions | 399,609 | **399,777**（+168） |
| | bad_nodes | 0 | **0** |
| | unsupported_total | 665 | 665 |
| | register_jmp_sites | 100 | 100 |
| | jumptabs / msvc_tables | 2 / 2 | **3 / 3** |
| | msvc_rejected | 78 | 77 |
| **dsp_ippv2** `--pdata` | functions | 32,648 | 32,648 |
| | closure_instructions | 6,465,008 | 6,465,008 |
| | bad_nodes / unsupported_total / register_jmp_sites | 0 / 974,527 / 836 | 同じ |
| | jumptabs / msvc_tables / msvc_rejected | 435 / 1 / 10 | 同じ |
| | gotos / c_lines / c_bytes | 450,917 / 6,708,131 / 536,761,588 | 同じ |
| **dsp_ippv2 縮小**（種 452） | functions / closure_instructions / bad_nodes / unsupported_total | 8,030 / 3,038,781 / 0 / 391,396 | 同じ |
| | register_jmp_sites / jumptabs / msvc_tables / msvc_rejected | 563 / 434 / 0 / 7 | 同じ |
| **QFC** `--pdata` | functions / closure_instructions / bad_nodes / unsupported_total | 6,712 / 209,259 / 0 / 104 | 同じ |
| | register_jmp_sites / jumptabs / msvc_tables / msvc_rejected | 5 / 0 / 0 / 0 | 同じ |
| **FLEngine** `--pdata` | functions / closure_instructions / bad_nodes / unsupported_total | 59,297 / 2,528,030 / 0 / 525 | 同じ |
| | register_jmp_sites / jumptabs / msvc_tables / msvc_rejected | 20 / 4 / 0 / 0 | 同じ |
| | gotos / tail_calls / c_lines / c_bytes | 203,462 / 610 / 2,885,412 / 215,707,512 | 同じ |

**`bad_nodes` はどこも増えなかった**（全構成で 0）。`unsupported_total` も変わらない（新しく閉包に入った 168 命令の中に未対応命令は無い）。

### 4.2 後の 3 つの設定の比較（実測）

実装後のビルドで、既定、`--msvc-base-entry-block`、`--msvc-base-dominating` をかけた（`stats/after-default.*`、`after-entry.*`、`after-domflag.*`）:

- 既定 と `--msvc-base-dominating`: **6 構成すべて統計の出力が全行同一**（`cmp`）。
- `--msvc-base-entry-block`（実装後のビルド）と d87e0ad の既定: 6 構成すべて全行同一（比較のための設定が、決定 0089 の挙動を保っている）。
- d87e0ad の `--msvc-base-dominating` と 実装後の既定: 6 構成すべて、統計の全行が同一で、`msvc_rejected` の行（jmp VA と理由）も同一。他の間接 jmp の規則（決定 2）は、実 DLL で解決を減らさず、拒否の増減もない。
- 実装後の既定の `msvc_rejected` に `other indirect jump` の理由は **0 件**（6 構成すべて）、`does not run before the jump on every path` も 0 件。

### 4.3 FLEngine の生成 C

`fe-lift lift binaries/win64/FLEngine_x64.dll OUT.c --pdata`（実装後の release ビルド）: SHA-256 **`43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1`**（d87e0ad のビルドの出力、3 つの設定の出力、3 モジュールの測定用のコピーと `cmp` で一致）。**変わった関数は 0**。FLEngine の MSVC 形 3 か所（0x5e5eb6、0x5e9ae8、0x5f1499）は決定 0036 の `Spec` が先に解決するので、新しい認識（`recognize_msvc`）には回らない。単独構成の a4（基準 145/942）は、FLEngine の生成 C が変わらず runtime も変えていないので**省略した**。

### 4.4 解決した site の全エントリの個別照合（実測。objdump / capstone）

`lift/out-jtb2d/analysis/verify_targets.py`（読み取り専用。fe-decode / fe-lift に依存せず、生成 C の `switch` を読み、capstone でイディオムと表を読み直し、各移動先が (1) 関数のピース内、(2) 線形走査の命令開始、(3) `objdump -d -M intel --start-address=T` の最初の行がちょうど T、であることを調べる）を、実装後の既定で作った生成 C にかけた（`logs/verify_targets_b2d0.txt`、`b2d58.txt`、`dsp.txt`。終了コード 0）:

| image / site | 関数 | B / N | エントリ → 異なる移動先 | case 数（生成 C） | 結果 |
|---|---|---|---|---|---|
| Blend2D `0x4701384` | `0x4701310` | r12 / 7 | 8 → 3（`0x4701386`、`0x4701393`、`0x47013a0`） | 3 | 表と case が一致、3 件とも命令の先頭 |
| Blend2D `0x4701cbb` | `0x4701bb0` | r12 / 6 | 7 → 3（`0x4701cbd`、`0x4701ce3`、`0x4701d65`） | 3 | 同上 |
| **Blend2D `0x470a89e`** | `0x4709ea0` | r13 / 31、索引バイト表 0xab8c、RVA 表 0xab74 | 6 → 6（`0x470a8a0`、`0x470a8af`、`0x470aa40`、`0x470aa6b`、`0x470aab1`、`0x470aafc`） | 6 | 同上（6 件とも命令の先頭） |
| dsp_ippv2 `0x37c69eb` | `0x37c6990` | r13 / 10 | 11 → 11 | 11 | 同上（11 件） |

解決した 4 site（Blend2D 3、dsp_ippv2 1）の distinct な移動先 **23 件すべてが関数のピース内の命令の先頭**（失敗 0）。Blend2D は種なしと種 58 個ありの両方の生成 C で同じ 3 site を確認。**範囲外のエントリは、解決した site にも拒否した site にも 0 件**。

## 5. テスト

### 5.1 `cargo test --workspace`（実測）

```
cd lift && CARGO_TARGET_DIR=<repo>/lift/out-jtb2d90/target TMPDIR=<repo>/lift/out-jtb2d90/tmp FLENGINE_PE=<repo>/binaries/win64/FLEngine_x64.dll cargo test --workspace --no-fail-fast > out-jtb2d90/test-ws.txt 2>&1
```

終了コード 0。`grep '^test result' | awk`: **97 binaries / 1,172 passed / 0 failed / 0 ignored**（基準 97 / 1,161 / 0 / 0。**+11 件、binaries は同じ**）。SKIPPED の行 0 件（実 DLL のテストはすべて実行された）。ログ `lift/out-jtb2d90/test-ws.txt`。警告 2 件は既存のテスト（`fe-lift/tests/monitor.rs:275`、`sse_shuf.rs:353`）のもので、変更したファイルではない。

内訳: `fe-decode/tests/msvc_switch.rs` 22 → **31**（+9）、`fe-lift/tests/msvc_switch.rs` 9 → **11**（+2）。

### 5.2 合成バイト列のテスト（`fe-decode/tests/msvc_switch.rs` 31 件、`fe-lift/tests/msvc_switch.rs` 11 件）

合成 PE と小さなアセンブラは `fe-decode/tests/msvc_common/mod.rs`。`Lea` に `AfterDiamond`（`test ecx,ecx; jne other; mov edx,1; jmp join; other: mov edx,2; join: lea B`）、`OneSided`（`test; je skip; lea B; skip:`）、`BothSides`（両経路に `lea`）を足し、`OtherJmp`（`Reg`: ブロック `ret` の後ろの `jmp rdx`、`Mem`: `jmp [rdx]`、`TailCall`: `pop B; jmp rel32` で helper へ、`SecondSwitch`: `def` ブロックが 2 つ目の (c1) switch）を足した。

**新しい期待（決定 0090）**:

| テスト | 内容 |
|---|---|
| `base_lea_outside_the_entry_block_on_every_path_is_proven_by_dominance` | `lea` が `je`（次の命令へ）の後、diamond の後（Blend2D の `f_4709ea0` の形）にある (c1)(c2)、call つきの c2 が**解決される** |
| `the_entry_block_lea_is_the_special_case_that_dominance_also_proves` | 入口ブロックの `lea` の形（c1、c2、cdqe+rax、push なし）が `Dominating` でも `EntryBlock` でも解決される（入口ブロックは dominance の特別な場合） |
| `base_lea_on_one_path_only_is_not_proven` | `test; je skip; lea B; skip:` は**解決されない**（`does not run before the jump on every path`）。`EntryBlock` でも解決されない（`is not in the entry block`） |
| `a_lea_on_each_path_is_two_writes_and_is_not_proven` | 両経路に `lea`: B が 2 回書かれるので解決されない（`is written 2 times`。決定 1' の維持） |
| `another_indirect_jump_in_the_function_means_dominance_is_not_proven` | 同じ関数に `jmp reg`／`jmp [mem]` があると**解決されない**（c1、c2、`lea` が入口ブロック内・外のどちらでも）。メッセージに最初の他の間接 jmp と `decision 0090`。`EntryBlock`（比較用）では同じ形が解決される |
| `a_second_resolvable_switch_in_the_function_is_not_allowed_either` | 同じ関数に別の switch（それ自体は解決できる形）があると、**両方とも解決されない**（拡張をしない）。`EntryBlock` では 2 つとも解決される |
| `a_tail_call_by_a_direct_jmp_is_not_an_indirect_jump` | tail call の直接 `jmp rel32` は間接ではない: 解決される |
| `the_entry_block_condition_of_decision_0089_does_not_prove_a_lea_after_a_branch` | `EntryBlock`（比較用）の挙動: `lea` が `je` の後・diamond の後だと `is not in the entry block` |
| `blend2d_without_seeds_resolves_the_sites_by_dominance_including_the_one_that_stopped_the_run` | 実 DLL: `--pdata` の閉包で 3 site（`0x4701384`、`0x4701cbb`、`0x470a89e`。case 数 3、3、6）。`0x470a89e` の `Origin::Msvc { n: 31, t1: 0xab8c, t2: 0xab74, function: 0x9ea0 }`、case は 6 つ、`0x4709ea0..0x470abac` の間接 jmp は `0x470a89e` の 1 個だけ |
| `blend2d_with_the_entry_block_condition_of_decision_0089_resolves_two_sites_only` | `EntryBlock`: 2 site のみ。`0x470a89e` の拒否理由は `lea R13,[ImageBase] at 0x4709f18 is not in the entry block of 0x4709ea0`。入口ブロックで解決していた 2 か所は Dominating でも同じ表で解決される |
| `blend2d_the_other_indirect_jump_rule_costs_no_site` | 実 DLL: `msvc_rejected` に `other indirect jump(s)` の理由が 0 件、`f_4745bd0` の site は B の書き込みか N で拒否される |
| fe-lift: `a_lea_outside_the_entry_block_on_every_path_every_index_matches_native_and_the_table` | 合成 PE を clang で native にも実行（`harness/image.c`）。`lea` が `je` の後・diamond の後（cdqe、rax ガード、call つき、tail call つき）で、索引 0..N+3 のすべてで native と生成 C の結果が同じで表のバイトの指す先と一致 |
| fe-lift: `real_blend2d_site_is_resolved_by_default` | 既定で 3 site が `switch`（`uint64_t t = R[`）になる。`0x470a89e` に `fle_dispatch(R[1]); return;` が残っていない。`BaseProof::Dominating` を明示した C と既定の C が同一 |
| fe-lift: `real_blend2d_f_4709ea0_every_index_reaches_the_table_target_by_default` | 実 DLL（下の §5.3） |

**既存テストで期待値を変えた行（報告用の列挙）**:

| ファイルとテスト | 変更前 | 変更後 | 理由 |
|---|---|---|---|
| `fe-decode/tests/msvc_switch.rs` の `base_lea_not_in_the_entry_block_is_not_proven`（`Lea::AfterBranch` が `is not in the entry block` で拒否） | 拒否 | **削除し、dominance の有無で 4 本に分けた**: 解決される（`base_lea_outside_the_entry_block_on_every_path_is_proven_by_dominance`）、`EntryBlock` で拒否される（`the_entry_block_condition_of_decision_0089_does_not_prove_a_lea_after_a_branch`）、片側にしか `lea` が無い形は両方で拒否される（`base_lea_on_one_path_only_is_not_proven`）、入口ブロックの `lea` は両方で解決される | 入口ブロックだけを前提にした「`lea` が入口ブロックの外 → 解決しない」のテストは、dominance では `AfterBranch`（`je` の両辺が `lea` へ落ちる）が解決されるので成り立たない |
| `fe-decode/tests/msvc_switch.rs` の `the_dispatch_form_is_unchanged_for_rejected_sites` | `Lea::AfterBranch` を「拒否される site」に使用 | `Lea::OneSided` に変更 | `AfterBranch` が解決されるようになったため。確認する内容（拒否した site は `register_jumps` 1、`jmp reg` のまま、移動先ブロックは閉包に入らない）は同じ |
| `fe-decode/tests/msvc_switch.rs` の `blend2d_without_seeds_resolves_the_sites_that_meet_the_conditions` | 2 site（`0x4701384`、`0x4701cbb`）、`0x470a89e` は拒否 | **`blend2d_without_seeds_resolves_the_sites_by_dominance_including_the_one_that_stopped_the_run` に改名**: 3 site（case 数 3、3、6）、`0x470a89e` は解決。2 site のみの期待は `blend2d_with_the_entry_block_condition_of_decision_0089_resolves_two_sites_only`（`EntryBlock`）に移した | 決定 0090 の既定の挙動 |
| `fe-lift/tests/msvc_switch.rs` の `real_blend2d_site_is_not_resolved_by_the_decisions_base_proof` | 既定で 0x470a89e は `fle_dispatch(R[1]); return;` | **`real_blend2d_site_is_not_resolved_by_the_entry_block_condition_of_decision_0089` に改名し `EntryBlock` を明示**。既定の期待は新規の `real_blend2d_site_is_resolved_by_default` | 同上 |
| `fe-lift/tests/msvc_switch.rs` の `real_blend2d_f_4709ea0_every_index_reaches_the_table_target_with_the_dominating_variant` | `BaseProof::Dominating`（測定専用）を明示して 40 通り | **`..._by_default` に改名し、既定のオプションで 40 通り** | 既定が dominance になったため。突き合わせの内容は同じ |
| `fe-lift/tests/msvc_switch.rs` の `unresolved_sites_stay_a_tail_dispatch` | 拒否する 5 形のうち `Lea::AfterBranch` を含む | `AfterBranch` を `OneSided`、`BothSides`、`OtherJmp::Reg`、`OtherJmp::Mem` に差し替え（計 8 形） | 同上と、新しい拒否の形を足すため |
| `fe-lift/tests/msvc_switch.rs` の出力先 | `lift/out-jtb2d/test` | `lift/out-jtb2d90/test` | 作業物の規則（`out-jtb2d` は読み取り専用） |
| `fe-decode/tests/jt_cpu.rs` など他の既存テスト | — | **変更なし**（dsp_ippv2 の 435、FLEngine の 4 は不変） | |

### 5.3 実 DLL: Blend2D `f_4709ea0` の `0x470a89e`（既定。実測）

`fe-lift/tests/msvc_switch.rs`（`BLEND2D_PE`、既定 `lift/out-seed58/s58/dll/Blend2D.4700000.dll`）。

1. **解決される**: `msvc_rejected` に 0x470a89e は無い。生成 C は 6 つの `case UINT64_C(VA): goto L_VA;` と `fle_jumptab_miss(UINT64_C(0x470a89e), t)`。
2. **表の 6 エントリ**（ファイルのバイトから読んだ。テストが生成器を使わず idiom の命令バイトを確認した上で読む）: `0x470a8a0`、`0x470a8af`、`0x470aa40`、`0x470aa6b`、`0x470aab1`、`0x470aafc` = 元の VA `0x18000a8a0`、`0x18000a8af`、`0x18000aa40`、`0x18000aa6b`、`0x18000aab1`、`0x18000aafc`。索引バイト 32 個: `[0,1,2,2,2,5,5,5,3,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,4]`。
3. **native と生成 C の突き合わせ**: 全 33 通りの索引（0..32）と範囲外の値 7 通り（33、34、63、100、0xff、0x80000000、0xffffffff）の計 40 通りを、`0x470a881`（`cmp eax,0x1f`）から入り、native（本物の命令。6 つの移動先に `0xCC`、SIGTRAP ハンドラ）と生成 C（6 つのラベルに到達の印）の両方で実行した（計 80 回）。**到達先はすべて、ファイルの索引バイトと RVA 表から読んだ値と一致**（索引 0..31 は表どおり、32 以上は `ja` の `0x470aafc`）。**既定のオプションで**通った（`--msvc-base-dominating` を使わない）。

## 6. 3 モジュール構成の測定（測定のみ。gdb の仮の値なし、runtime の応答追加なし。`GetModuleHandleA` は実装していない）

### 6.1 手順（実測）

`lift/out-seed58/harness-two/` を `lift/out-jtb2d90/three/harness-two/`（`out-jtb2d/three/harness-two` のコピー。変更なし）にコピー。リベース済み DLL は `lift/out-jtb2d/three/dll/`（`lift/out-seed58/s58/dll/` と同じもの）から作業物へコピーし、SHA-256 が `4fd25502…efc5`（dsp_ippv2）、`24e01ec9…b4cb`（Blend2D）と一致。FLEngine の生成 C と分割 C、`native/fle`、`native/rt`、`native/h` のオブジェクトは `out-jtb2d/three/` から**コピーして再利用**（FLEngine の生成 C は今回のビルドの出力と `cmp` で同一、SHA `43985d62…`。runtime と harness は変えていない）。作業コピー（HEAD + 変更）の release の `fe-lift`、`fe-pe`（`lift/out-jtb2d90/target/release`）で、`gen_two.sh gen split merge`、`build_two_native.sh`（`JOBS=8`）を実行した（`MODULES` と引数は前回と同じ。`--pdata --seeds seeds58-4700000.txt`）。

| TAG | 内容 | Blend2D の生成 C |
|---|---|---|
| `b2d58` | **決定 0090 の既定**（種 58 個あり） | 2,066 関数、399,784 命令（emitted）、bad_nodes 0、SHA-256 `3298bfb80e7ee3c30c05e57d57b93c82483489c3b4451f51935a46a28f221f98`。**前回の `--msvc-base-dominating` の C（`out-jtb2d/three/v-b2d58dom/gen/blend2d.c`）と `cmp` で同一**。dsp_ippv2 の縮小 C（SHA `6035da1d…9dbf`）も前回と同一 |
| `b2d0` | 既定、種なし（任意の測定） | 2,018 関数 |

表の統合: b2d58 は 59,557 + 8,290 + 2,326 → 69,653 行（前回と同じ。複数の表に在る行 260、`f_<VA>` の衝突 0）。`two-native` 612,278,432 B。2 GB 未満。ログ `three/log/`。

### 6.2 a4 の行（既定、種 58 個あり。2 回: 13:49:22Z、13:49:28Z。gdb なし。ログ `lift/out-jtb2d90/three-out/a4/`）

```
A4 survey 20261024T134922Z   dll: …/binaries/win64/FLEngine_x64.dll   harness exit: 134
static : units=942 runnable=895 empty=46 noproc=1 (header 0x13768b0, units 0x13768e0)
stop   : fle: unsupported instruction at 0x4899fed
         top of stack: 0x4899d90 f_4899d90
depth  : init_count=150 / 942   last unit: index=149 run_rank=137/895 proc=0x664260 in_progress
         started=137 completed=136 in_progress=1 of 895 runnable   all_units_done=no
         entry check: started_but_not_entered=0 runnable_entered_beyond_init_count=0
funcs  : executed (distinct)=1058  entries=27317
```

（1 回目の直後に同じ秒の `units.tsv` と衝突して 2 回目の最初の呼び出しが `File exists` で終了したため、5 秒空けて再実行した。a4-survey は変えていない。）

- 停止メッセージ全文: **`fle: unsupported instruction at 0x4899fed`**（`abort kind: fle_unsupported`、`call depth at stop: 17`）。stderr も同じ 1 行。
- **C のスタック**（先頭 10 行。`call depth at stop: 17`）:

```
#0 0x4899d90 f_4899d90 guest-return=0x4701009
#1 0x4701000 f_4701000 guest-return=0x48a59eb
#2 0x48b5b90 f_48b5b90 guest-return=0x48a59eb
#3 0x48a59a8 f_48a59a8 guest-return=0x489e34e
#4 0x489e2b4 f_489e2b4 guest-return=0x489e4c4
#5 0x489e264 f_489e264 guest-return=0x489e4c4
#6 0x489e450 f_489e450 guest-return=0xcb000000
#7 0x489e584 f_489e584 guest-return=0xcb000000
#8 0x423990 f_423990 guest-return=0x44db70
#9 0x44db40 f_44db40 guest-return=0x653a56
```

- 2 回の比較: stderr は `cmp` で一致、stdout は `rsp=` の行を除いて一致。
- **前回の参考測定（`--msvc-base-dominating`、`out-jtb2d/three-out-dom/a4/`）との比較**: `a4:` の行、`== stop` から `a4: init_count` までの停止報告と C のスタックが**同一**（`diff` で差なし）。functions_distinct 1,058、function_entries 27,317（前回の決定どおりの測定 27,316 から +1）。
- `0x4899fed` は `call qword ptr [rip + 0x1d0d5]` で、IAT スロット `0x48b70c8` = **`KERNEL32.dll!GetModuleHandleA`**（pefile でスロットを解決した。実測）。`imports_table` に行が無い import（行なし 15 名のうちの 1 つ。B8）。`f_4899d90` は CPU 情報の初期化（`f_470abb0` → `f_4709ea0`）を終えて `GetSystemInfo`（今回 2 回呼ばれた）の後に `GetModuleHandleA(…)` を呼ぶところまで進んだ（読み）。
- 種なしの Blend2D（任意。TAG `b2d0`、1 回、13:52:44Z）: 停止 `fle: indirect call to 0x48a57f8 is not a known function entry`（top of stack `f_48b5b90`）、init_count 150/942、functions_distinct 960、function_entries 26,604。**前調査 §6.4 および `blend2d-seed58.md` §4 の対照（種なし）と同一**。種なしでは停止点（B5 の `0x48a57f8`）に届く前に止まるので、`0x470a89e` の解決は a4 の結果を変えない。

### 6.3 読み取りだけの gdb（`lift/out-jtb2d90/gdb/ro_imports.py` は `lift/out-jtb2d/gdb/ro_imports.py` の無変更のコピー。`return`、`set var`、`call` なし。ログ `lift/out-jtb2d90/run/b2d58-imports.txt`）

- **Blend2D の DllMain(ATTACH) は戻ったか**: **戻っていない**（`DLLMAIN-RETURN reason=1: out of scope (did not return normally)`。スタックに `f_489e584` が残る）。**LoadLibraryW は 0x4700000 を返したか**: **返していない**（`LOADLIBRARY-RETURN` は 1 回だけで `R[0]=0x2000000`（dsp_ippv2））。
- Blend2D の DllMain に入るまでの `fle_imp_*` は 1,525 回（`BLEND2D DllMain entered (f_489e584) rcx=0x4700000 rdx=1 r8=0`）。
- **ATTACH 内の import の列**: **174 回**（通し番号 1526〜1699）。最初は `GetSystemTimeAsFileTime`、`QueryPerformanceCounter`、`GetCurrentThreadId`、`GetCurrentProcessId`、`QueryPerformanceCounter`、`LoadLibraryExW`、…、最後の 3 つは `HeapFree`（1697）、`GetSystemInfo`（1698）、`GetSystemInfo`（1699）。名前ごとの回数: `InitializeCriticalSectionAndSpinCount` 83、`GetProcAddress` 12、`HeapAlloc` 8、`EnterCriticalSection` 7、`LeaveCriticalSection` 7、`LoadLibraryExW` 6、`MultiByteToWideChar` 6、`WideCharToMultiByte` 6、`LCMapStringW` 4、`GetStdHandle` 3、`TlsSetValue` 3、`GetCPInfo` 2、`GetSystemInfo` 2、`QueryPerformanceCounter` 2、`TlsAlloc` 2、`TlsGetValue` 2、残りは各 1 回。前回の決定どおりの測定（172 回）との差は `GetSystemInfo` の 2 回だけ（前回の `--msvc-base-dominating` の測定の列と通し番号・名前が `diff` で同一）。名前付き停止の import なし。
- **停止点の分類**: **(i) 未応答の import**（`imports_table` に行が無い 15 名のうちの `KERNEL32!GetModuleHandleA`）。(ii) TLS、(iii) JIT、(iv) ロケール、(v) その他、のどれでもない。`fle_unsupported` は API 名を持たず VA を出すので、API 名は IAT スロットから導いた。unit 158（quickfontcache_x64.dll の LoadLibraryW）には届いていない。
- **新しい停止は出なかった**（予想どおり `0x4899fed` の `GetModuleHandleA`。参考測定と同一）。直していない。

## 7. 事実の並べ（決めない）

1. 決定 0090 の既定の実装は、Blend2D の停止点 `0x470a89e` を解決し、3 モジュールの停止は B3（`fle_dispatch_miss` 0x470a8a0）から B8（`GetModuleHandleA`）へ移る。測定は前回の参考測定（`--msvc-base-dominating`）と a4、停止、C のスタック、import の列まで同一。
2. 決定 2（他の間接 jmp があれば証明しない）は、実 DLL 4 つ（Blend2D、dsp_ippv2、QFC、FLEngine）で解決を 1 か所も減らさなかった（該当する 15 か所は先に決定 1' で落ちる）。この規則の実害は、実 DLL では無い。
3. 95 か所のうち解決できるのは種なしの Blend2D で **3 か所**（前回 2 か所）。残りの大半は B が複数回書かれる 33 か所と N が導けない 40 か所（決定 0090 でも範囲外）。
4. FLEngine の生成 C、単独構成の a4、dsp_ippv2、QFC に影響しない。Blend2D の closure は +168 命令、`bad_nodes` と `unsupported_total` は不変。

## 8. 解釈が分かれた点、実装できなかった点（PM への報告）

実装できなかった決定の項目は無い。次は決定の文言の解釈を要した点で、決定の内容は変えていない。

1. **「他の間接 jmp」の範囲**: `FlowControl::IndirectBranch` を使った。これは `jmp reg`、`jmp [mem]`（IAT 経由の `jmp [rip+x]` の import thunk 形を含む）を含み、間接 call（`call reg`、`call [mem]`）は含まない。決定の文言「間接 jmp（`jmp reg`、`jmp [mem]`）」に従った。関数のピース内の線形走査で数える（デコードできないバイトは飛ばす。switch 表のバイトが偶然の命令に見える場合に偽の「他の間接 jmp」として数えうる。その場合は解決を減らす方向（fail closed）で、実 DLL では 0 件）。
2. **順序**: 他の間接 jmp の検査は、基底への書き込みが 1 回で `lea` が ImageBase であること、揮発 B と call の検査の**後**、`lea_dominates_jmp` の**前**に置いた。同じ site が複数の理由で落ちるとき、`msvc_rejected` に出る理由は最初の 1 つ（既存と同じ流儀）。したがって決定 2 の規則の影響を数えるには「規則を外した測定」との差が要る（§4.2 の `--msvc-base-entry-block` ではなく、d87e0ad の `--msvc-base-dominating` との比較）。
3. **`EntryBlock` は他の間接 jmp を見ない**（決定 0089 のまま）。決定 3 は「`EntryBlock` は比較のための測定専用として残す」とだけ書いたので、0089 の挙動を保った。
4. **`--msvc-base-dominating`**: 受け付けて何もしない（既定と同じ）。usage からは外した（`--msvc-base-entry-block` を載せた）。
5. **自分の関数の別の switch の表の移動先を辺にしない**: `lea_dominates_jmp` は jmp 自身の表の移動先を辺にする（既存。0089 の測定専用版と同じ）。決定 2 の拡張（他の解決済み switch の表を辺にして許す）は実装していない。

## 9. 限界、未確認

- **線形走査の CFG**: 関数のピース内のデコードできる命令だけで CFG を作る。ピースの外への直接分岐は辺を持たない。実行時にだけ決まる経路（例外のハンドラ、SEH のスコープ表で呼ばれるフィルタ／ハンドラ、`longjmp`）は見ない。`foreign_entries`（閉包が知っている分岐だけ）で補うが、閉包が知らない経路は検出できない。
- **他の間接 jmp があれば証明しない**は十分条件で、必要条件ではない。他の間接 jmp が解決済みの switch であれば表の移動先を辺にして同じ証明ができるが、今回は扱わない。実 DLL では 0 か所の影響。
- **書き込みが 2 回以上ある形**（MSVC が switch の直前で `lea B` を出し直す形。前調査 §3.4 の 33 か所）は扱わない。到達定義の解析が要る。
- 解決した 4 か所のうち、**native と生成 C の突き合わせをしたのは `0x470a89e` だけ**（40 通り）。Blend2D `f_4701310`、`f_4701bb0` と dsp_ippv2 `f_37c6990` は、バイト列（命令境界、関数内）の確認に限る（実行されたかは未確認）。
- 母集団の誤認識の検査は、FLEngine、dsp_ippv2、Blend2D、QFC の 4 つに限る（同梱の他の DLL は未実施）。
- `.pdata` を持たない image では何も解決しない。`.pdata` に無い葉関数の中の `jmp reg`（Blend2D の 3 か所）は決定 0089 の (d) を満たせないので解決しない。
- 3 モジュールの測定は gdb の仮の値なし、runtime の応答追加なし。`GetModuleHandleA` は直していない。実 Windows での挙動は未確認。wasm は使っていない（native のみ）。

## 10. 作ったファイル

- 変更: `docs/README.md`（0090 の行）、`docs/decisions/0089-msvc-switch-tables-with-proven-base.md`（決定の表の 1(a) の次に注記 1 行）、`lift/crates/fe-decode/src/jumptab.rs`、`lib.rs`（コメント 1 行）、`lift/crates/fe-lift/src/main.rs`、`lift/crates/fe-decode/tests/msvc_common/mod.rs`、`msvc_switch.rs`、`lift/crates/fe-lift/tests/msvc_switch.rs`。
- 新規: `docs/decisions/0090-msvc-switch-base-dominance.md`、本文書。
- 作業物（`lift/out-jtb2d90/`、gitignore 対象）: `stats.sh`、`stats/`、`analysis/{survey90,join95_90}.py`、`logs/`、`test-ws.txt`、`target/`、`target-base/`、`three/`、`three-out/`、`three-out-b2d0/`、`gdb/`、`run/`、`b2d0.c`、`b2d58.c`、`dsp.c`。
