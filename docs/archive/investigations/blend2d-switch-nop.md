# MSVC switch 表の認識で `ja` と表引きの間の `nop` の詰め物を許す（決定 0093）: 77 件の見積もり、実装、検証、3 モジュールでの停止点（測定のみ）

実行日: 2026-10-25。基準は HEAD `a64a41c`（`git archive` を `lift/out-nop93/src/` に展開して作業した。途中で HEAD が決定 0092 のコミット `38df911` に進んだ。0093 が変えるファイルは両者でバイト一致）。決定は `docs/decisions/0093-msvc-switch-nop-padding.md`（PM の決定。仮の決定ではない）。前提は決定 0089、0090 と `docs/investigations/blend2d-switch-tables.md`、`blend2d-switch-dominance.md`、`blend2d-srw.md`（以下「前調査」）。作業物は `lift/out-nop93/`（gitignore 対象）。**この文書は何も決めない**。決定 0074、0079、0080、0086、0089、0090 は変えていない。基底 0x4700000 と種 58 個は測定用で、決定ではない。**ワークツリーのファイルは一切編集していない**。

## 0. 凡例

- **実測** = このセッションでコマンドを実行して得た値。ログは `lift/out-nop93/` の下（各節に書く）。
- **読み** = ソース、生成 C、別の文書を読んだ結果。実行して確かめていない。
- **未確認** = そのとおり。

## 1. 要約（先に結論）

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | 実装 | `recognize_msvc` の `ja` へ戻る歩きに、レジスタ・フラグ・メモリを変えない `Mnemonic::Nop`（`is_padding_nop`）を足した。`MSVC_SKIP`（2）は `cdqe`・`lea B,[rip+d]`・`nop` の合計のまま。`verify_msvc` は**変更不要**（詰め物を誤って拒否せず、詰め物への分岐は従来どおり拒否。テストで確認）。`landings_in_idioms` の窓 `MSVC_IDIOM_MAX` は 55 → 71 バイト（nop が最長 15 バイトのため。個数の上限は変えない） | 読み + 実測（§4、§5） |
| 2 | **`f_48a2060` の `jmp r9`（`0x48a208b`）は、この変更だけでは解決されない** | 認識は通る（`MsvcForm { base: R10, n: 15, t2: 0x2fd050, from: 0x48a2074 }`）。しかし基底証明（決定 0089、0090。範囲外）で `R10 is written 3 times in the function` と拒否される。r10 は `lea r10,[ImageBase]`（0x48a2063）のほかに `movzx r10d,byte [rdx+0xe]`（0x48a209b）と `mov r10,rdx`（0x48a25f3）で書かれ、同じ関数に `jmp r11` が 3 つある。**決定の背景の見込み（「`lea r10` が入口にあり、他の間接 jmp は無いはず」）は実測と合わなかった** | 実測（§3、§6.2） |
| 3 | 3 モジュール（種 58 個、基底 0x4700000）の停止点 | **動かなかった**。a4: init_count 150/942、last unit 149、functions_distinct 1,069、function_entries 27,332。停止 `fle: indirect call to 0x48a20ff is not a known function entry`（top of stack `f_48a2060`）。**0092 の測定と a4 の行、停止、C のスタック、import の列（名前と通し番号）が同一**。新しい停止は出ていない | 実測（§7） |
| 4 | 77 件のうち解決される見込み（静的な見積もり。実装前） | **0 件**（nop だけが挟まるのは 1 件 = `0x48a208b` で、基底証明で落ちる）。実装後の実測も 0 件（`msvc_tables` は全 DLL で前後同一） | 実測（§2、§6） |
| 5 | FLEngine の生成 C の SHA-256 | **`eb9d8e7fa74ae562742fb1999f6da9b3ca82f2884d1845b5b7319153e1b709cf` のまま**（HEAD の `fe-lift` の出力と `cmp` でバイト一致）。変わった関数 0 | 実測（§6.1） |
| 6 | テスト | `cargo test --workspace`: **97 binaries / 1,196 passed / 0 failed / 0 ignored**（基準 97 / 1,178。+18）。0092 のコミットの内容を重ねたコピーでは 98 / 1,213 / 0 / 0 | 実測（§6.1） |

## 2. 77 件の見積もり（実装前。静的。実測）

方法: HEAD の `fe-lift`（release、`lift/out-nop93/target-base`）の統計の `msvc_rejected` の行（`lift/out-nop93/stats/base.*.txt`）を入力にして、各 site の `jmp reg` から `add`、表引き、`ja` へ capstone で戻り、`ja` と idiom の間の命令を分類した（`lift/out-nop93/analysis/estimate_nop2.py`、`estimate_nop3.py`。自前の `.pdata` パーサ。fe-decode に依存しない）。「`nop` だけが挟まるもの」について、新しい規則を当てて N（`cmp idx,imm`）、基底証明（0090: 書き込み 1 回・`lea` が ImageBase・他の間接 jmp なし・揮発 B と call・dominance）、移動先を静的に確かめた。ログ `logs/estimate2_*.txt`、`logs/estimate3_*.txt`。

**Blend2D 種 58 個あり（`--pdata --seeds seeds58-4700000.txt`）の `msvc_rejected` 77 件**（理由別。実測、`stats/base.b2d58.txt`）:

| 理由 | 件 | 今回の変更で変わるか |
|---|---|---|
| B が複数回書かれる（`R… is written N times`） | 33 | 変わらない（範囲外） |
| **`N not derived: no ja before the table idiom`** | **33** | 内訳は下 |
| `N not derived: the closure decoded nothing before …` | 7 | 変わらない |
| `the jump is not inside a .pdata function` | 2 | 変わらない |
| `N not derived: no cmp idx,imm right before the ja` | 2 | 変わらない |

「`no ja before the table idiom`」33 件の内訳（`ja` の後ろから idiom の最初までに見える命令）:

| 区分 | 件 | 内容 |
|---|---|---|
| **`nop` だけが挟まる** | **1** | `0x48a208b`（`f_48a2060`）。間は `nop word ptr [rax+rax]`（12 バイト 1 命令） |
| `lea B,[rip+d]` の後に `mov`／`movsxd` など | 11 | `lea; mov eax,r8d`（5 件）、`lea; mov eax,edx`（2 件。`0x4701a97`、`0x4771e23`）、`lea; mov eax,edi`（1 件）、`lea; movsxd rax,edx`（1 件）、`lea; mov; lea; movsxd; sub; push; mov` の 7 命令（1 件、`0x47596c2`）、`jne; cmp; movzx; lea r13`（1 件、`0x477462c`）。nop ではない（`nop` を許しても拒否のまま） |
| 12 命令戻っても `ja` が無い | 19 | idiom の前がブロックの先頭で `je`／`jb`／`jmp` が来る形など（ガードが別の経路にある）。nop ではない |
| `.pdata` の外の葉関数 | 2 | `0x478136e`、`0x48a3479`（`.pdata` に無い関数の中。決定 0089 の (d) を満たせない） |

- **nop だけが挟まるのは `0x48a208b` の 1 件**。他の理由（B 複数回書き込み 33、N が導けない他、`.pdata` 外）で拒否されている 76 件のうち、`ja` の後ろ 12 命令に nop 系があるものは**他に 0 件**（`logs/estimate2_base.b2d58.txt` の「any rejected site … with a nop-family instruction」は `0x48a208b` の 1 行だけ）。
- 種なしの Blend2D（`msvc_rejected` 76）も同じ（`0x48a208b` の 1 件。`0x478136e` は種ありだけの site）。dsp_ippv2（`--pdata`、`msvc_rejected` 10）は 1 件（`0x37cc80b`。Blend2D の `f_48a2060` と同じ形の関数 `f_37cc7e0`）。dsp_ippv2 縮小（種 452、7 件）も同じ 1 件。QFC、FLEngine は 0 件（`msvc_rejected` が 0）。
- **その 1 件を新しい規則で通した場合**（静的）: N = 15（`cmp r8,0xf; ja` が `nop` の直前）、スキップ数 1（`MSVC_SKIP` 内）、B = r10。**基底証明で落ちる**（r10 の書き込みが 3 回）。したがって**解決される見込みは 0 件**。

## 3. `f_48a2060` の事実（実測。`objdump -d -M intel`、capstone、`logs/objdump_f_48a2060.txt`、`fn48a2060_facts.txt`、`fn48a2060_blockers.txt`、`reaching_r10_b2d.txt`）

リベース後の Blend2D（基底 0x4700000。SHA-256 `24e01ec9…b4cb`）。`.pdata` は `[0x48a2060, 0x48a26d5)` の 1 エントリ（UNWIND_INFO のフラグ 0 = 連鎖なし）、命令 346 個、`call` 無し。

```
48a2060: mov rax, rcx
48a2063: lea r10, [rip-0x1a206a]                 ; r10 = 0x4700000
48a206a: cmp r8, 0xf
48a206e: ja  0x48a2180
48a2074: 66 66 66 66 0f 1f 84 00 00 00 00 00      ; nop word ptr [rax+rax]（12 バイト、1 命令。次は 0x48a2080）
48a2080: mov r9d, [r10+r8*4+0x2fd050]
48a2088: add r9, r10
48a208b: jmp r9
```

- 詰め物は**12 バイトの 1 命令**（`66 66 66 66 0f 1f 84 00 00 00 00 00`）。PM の依頼の「11 バイトかも／12 バイトかも」は **12 バイト**。
- 表（RVA `0x2fd050`、基底 + RVA = 0x49fd050）は 16 エントリ（r8 = 0..15）。**16 個すべて異なり、すべて同じ関数内で、独立にデコードした線形走査の命令の先頭**: r8 → 移動先（元の VA は `+0x17b00000`）: 0 → 0x48a208e（`ret`）、1 → 0x48a2174、2 → 0x48a20c8、3 → 0x48a20ff、4 → 0x48a217a、5 → 0x48a215f、6 → 0x48a2150、7 → 0x48a20d0、8 → 0x48a216d、9 → 0x48a2135、10 → 0x48a2126、11 → 0x48a20b0、12 → 0x48a2143、13 → 0x48a2110、14 → 0x48a20e8、15 → 0x48a2090。全 16 行（命令つき）は `logs/fn48a2060_facts.txt`。
- 0x48a2060 の外から 0x48a2074〜0x48a208b へ入る直接分岐は無い。
- **PM の見込みとの違い**:
  - r10 への書き込みは **3 回**: `lea r10`（0x48a2063）、`movzx r10d,byte [rdx+0xe]`（0x48a209b。r8 = 15 のコピーのブロック）、`mov r10,rdx`（0x48a25f3。大きなコピーの後半）。
  - **`jmp r9` 以外の間接 jmp が 3 つ**: `jmp r11`（0x48a22d2、0x48a2402、0x48a2576。`mov r11d,[r10+r11*4+T]; add r11,r10` の表引き。索引は `shr r11,5`／`shr r11,4`）。
  - したがって決定 0090 の `prove_base`（書き込みがちょうど 1 回、他の間接 jmp があれば証明しない）は通らない。
- 参考（到達定義。実測。**決定ではない**。決定 0090 の 1' は到達定義の解析を範囲外にしている）: 4 つの `jmp`（`r9` 1、`r11` 3。後者の表は 27、18、9 エントリを辺に入れた）すべてで、r10 に到達する定義は 0x48a2063 の `lea` **だけ**（他の 2 つの書き込みの定義は、4 つの `jmp` のどれにも到達しない）。この結果は `reaching_r10.py` の出力（`logs/reaching_r10_b2d.txt`）で、表の辺は `jmp r9` が 16 エントリ、`jmp r11` が各表の関数内のエントリ。dsp_ippv2 の `f_37cc7e0` も同じ（`reaching_r10_dsp.txt`）。

## 4. 実装（読み、変更ファイル）

決定 0093 の「実装した場所」と同じ。`lift/crates/fe-decode/src/jumptab.rs` の 5 か所（`+25 -8`）。`fe-lift`、`lib.rs`、`lift/runtime` は変更なし。

| 変更 | 内容 |
|---|---|
| `is_padding_nop`（新規、`pub`） | `Mnemonic::Nop`（`90`、`66 90`、`0f 1f /0`、プレフィックスつき）で、`iced_x86` が報告する**レジスタ・メモリへの書き込みなし、`rflags_modified() == 0`**。`nop [mem]` のメモリオペランドはアドレスだけで読み書きされない |
| `recognize_msvc` | `ja` へ戻る歩きの `neutral` に `\|\| is_padding_nop(i)`。`MSVC_SKIP`（2）は `cdqe`・`lea`・`nop` の合計。拒否理由の文言に `nop` を追記 |
| `MSVC_IDIOM_MAX` | `7 * MSVC_SKIP` → `15 * MSVC_SKIP`（55 → 71 バイト）。`landings_in_idioms` の窓 |
| `verify_msvc` | **変更なし**。`from`（`ja` の次）から `jmp` までを 1 命令ずつたどり、`from` 以外のブロックの先頭があれば拒否する。詰め物の `nop` は fall-through であればブロックの先頭ではなく、誤って拒否しない。分岐が着地する詰め物はブロックの先頭になり従来どおり拒否 |

`is_padding_nop` が当たるもの（`which_instructions_are_padding_nops`）: `90`、`66 90`、`0f 1f 00`／`40 00`／`44 00 00`／`80 …`／`84 00 …`（3〜8 バイト）、`66` つき 6、9 バイト、`66` 4 個つき 12 バイト（Blend2D の形）、`66` 6 個 + `2e` つき 15 バイト、`48 90`、`0f 1f c0`。当たらないもの: `mov edx,1`、`mov edx,edx`、`xchg r8,rax`（`49 90`）、`xchg r8d,eax`（`41 90`）、`xchg eax,eax`（`87 c0`。rax の上位 32 ビットを 0 にする）、`pause`（`f3 90`）、`int3`、`ud2`、`hlt`、`lea r11,[rip+0]`、`0f 19 00`（予約 nop）、`0f 1e fa`、`endbr64`。

## 5. テスト（実測）

`fe-decode/tests/msvc_common/mod.rs`: 合成 PE の `Sw` に `pad`（`ja` と idiom の間の命令の生バイト列）、`pad_before_cdqe`、`pad_landing`（詰め物の途中へ分岐するブロック）を足し、nop の各長さの定数（`NOP_1`〜`NOP_15`）と nop でない命令の定数を足した。既存の `Sw::c1()`、`Sw::c2()` は詰め物なしのまま（既存テストの期待値は変えていない）。

`fe-decode/tests/msvc_switch.rs` 31 → **45 件**（+14）、`fe-lift/tests/msvc_switch.rs` 11 → **15 件**（+4）。

**解決されること**: nop 1 つ（1、2、3、4、5、6、7、8、9、12、15 バイト）の (c1)、(c2)。`cdqe` の前後に nop、`sub` つき、call つき、`lea` が diamond の後、揮発 B（r10）、nop 2 つ（`MSVC_SKIP` ちょうど）。`from`（`ja` の次）は詰め物そのもの。

**解決されないこと**: `mov edx,1`、`mov edx,edx`、`xchg r8,rax`、`xchg eax,eax`、`pause`、`lea r11,[rip+0]`、予約 nop のどれか（nop が隣にあっても）。nop 3 つ（`too many instructions between the ja and the table idiom`）。`cdqe` + nop 2 つ。`int3`、`ud2` が詰め物にある形（ブロックが終わり idiom は到達せず、`jmp reg` が閉包に入らない: 何も報告されない）。

**詰め物への分岐**: 詰め物の先頭、1、3、5、7 バイト目、2 つの nop の間、`cdqe` の後ろへ着地する分岐は、すべて拒否（`landings_in_idioms`: `the branch at … lands at …, inside the switch idiom (the guard cmp; ja is skipped)`）。idiom の最初の命令（nop の後の `mov`/`movzx`）への着地も拒否。`verify_msvc` を直接呼ぶテスト（`verify_msvc_accepts_the_padding_run_and_still_rejects_a_block_start_inside_it`）で、詰め物の run を受け入れ、ブロックの先頭が 2 つ目の nop、idiom の最初、`jmp` にあるときに `a block starts at …, inside the switch idiom` で拒否することを確認。

**他の条件は変わらない**: 詰め物つきで、`lea` が片側だけ、B が 2 回書かれる、B が引数から、他の間接 jmp、揮発 B + call、移動先が別の関数／命令境界でない、別の関数からの飛び込み、ガードが別のレジスタ／無い、が従来の理由で拒否される。

**合成 PE を clang で native にも実行**（`fe-lift`、`harness/image.c`。詰め物は native では実際に実行される）: 12 バイトの nop（c1、c2）、1、2（`66 90`）、15 バイト、nop 2 つ、`cdqe` + 64 ビットガード、nop が `cdqe` の前 + call、`lea` が diamond の後、`sub`、揮発 B（r10）で、索引 0..N+3（+K）のすべてで native と生成 C の結果が同じで表のバイトの指す先と一致。生成 C は `case` 3、`fle_jumptab_miss` 1、`fle_dispatch(R[` なしの switch。解決しない詰め物の形は `fle_dispatch(R[1]); return;` のまま。

**実 DLL（Blend2D `f_48a2060`）**:

1. 12 バイトの詰め物のバイト列、`lea r10`、`cmp r8,0xf`、`ja`、`mov r9d,[r10+r8*4+0x2fd050]`、`add r9,r10; jmp r9` の命令バイトを、ファイルから読んで確認。
2. `recognize_msvc` が実バイト列から `MsvcForm { first: 0x48a2080, from: 0x48a2074, guard_ja: 0x48a206e, guard_cmp: 0x48a206a, base: R10, n: 15, t1: None, t2: 0x2fd050 }` を返す。
3. **表の 16 エントリの個別照合**: ファイルの RVA 表（64 バイト）から読んだ 16 個が、objdump の一覧（§3）と**全 16 件一致**。16 個とも異なり、すべて `[0x48a2060, 0x48a26d5)` の中で、独立にデコードした命令の先頭。`FuncMap` は主関数 `0x1a2060`、ピース 1 つ。
4. **native**（本物の命令。入口 0x48a2060 から `rcx`/`rdx`/`r8` を与え、16 個の移動先と `ja` の行き先 0x48a2180 に `0xCC`、SIGTRAP ハンドラが止まった番地を出力）: r8 = 0..15 の 16 通りと 16、17、31、32、33、0x100、0xffffffff、`u64::MAX` の 8 通り、計 24 で、到達先がすべてファイルの表どおり（0..15 は表、16 以上は 0x48a2180）。`real_blend2d_f_48a2060_native_every_size_reaches_the_table_target`。
5. **生成 C との突き合わせは未実施**: この site は基底証明で解決されないので生成 C に switch が無く、突き合わせる対象が無い。
6. **解決されないことの固定**: `R10 is written 3 times in the function`（`N not derived` ではない）。同じ関数の `jmp r11` 3 つは従来と同じ理由（`N not derived: no ja before the table idiom`）。解決された表は従来の 3 つ（`0x4701384`、`0x4701cbb`、`0x470a89e`）のまま。

**変異検査**（実施。元に戻して `cmp` で確認）: `is_padding_nop` の `Mnemonic::Nop` の判定を外す → 2 件が失敗。`landings_in_idioms` の入力を空にする → 4 件が失敗。`MSVC_IDIOM_MAX` を 7 倍に戻す → `the_landing_window_covers_two_longest_padding_nops` が失敗（窓 55 < idiom 66）。`verify_msvc` の `leaders.contains` を外す → 最初は**どのテストも失敗しなかった**（`landings_in_idioms` が先に拒否するため。素通り）ので、`verify_msvc` を直接呼ぶテストを足し、再度の変異で失敗することを確認した。

**既存テストの期待値を変えた行: なし**（追加のみ）。`msvc_rejected` の文言を変えたが、既存のテストに該当の文字列を固定するものは無く、全テストが通った。

## 6. 検証（実測）

### 6.1 テストと FLEngine

```
cd lift/out-nop93/src/lift && TMPDIR=…/out-nop93/tmp CARGO_TARGET_DIR=…/out-nop93/target FLENGINE_PE=…/binaries/win64/FLEngine_x64.dll cargo test --workspace --no-fail-fast > ../../test-ws.txt 2>&1
```

終了コード 0。`grep '^test result' | awk`: **97 binaries / 1,196 passed / 0 failed / 0 ignored**（基準 97 / 1,178 / 0 / 0 は依頼の値で、HEAD では再測定していない。+18 は新規のテスト数 14 + 4 と一致）。SKIPPED の行 0 件（実 DLL のテストはすべて実行された）。ログ `lift/out-nop93/test-ws.txt`。0092 のコミット（38df911）の内容を `src0092/` に重ねたコピーでは **98 binaries / 1,213 / 0 / 0**（0092 のコミットの 98 / 1,195 に +18。`test-ws-with0092.txt`）。

FLEngine の生成 C: `fe-lift lift binaries/win64/FLEngine_x64.dll OUT.c --pdata`（HEAD のビルドと 0093 のビルド）: SHA-256 **`eb9d8e7fa74ae562742fb1999f6da9b3ca82f2884d1845b5b7319153e1b709cf`**（`cmp` でバイト一致）。**変わった関数 0**。単独構成の a4（基準 145/942）は、FLEngine の生成 C が変わらず runtime も変えていないので**省略した**。

### 6.2 DLL ごとの統計の前後（`lift/out-nop93/stats.sh`、HEAD の `fe-lift` → 0093。release）

| 構成 | closure_instructions | functions | bad_nodes | unsupported_total | register_jmp_sites | jumptabs / msvc_tables | msvc_rejected |
|---|---|---|---|---|---|---|---|
| Blend2D 種なし `--pdata` | 399,148 → 399,148 | 2,018 | 0 → 0 | 661 → 661 | 99 → 99 | 3 / 3 → 3 / 3 | 76 → 76 |
| Blend2D 種 58 個あり | 399,777 → 399,777 | 2,066 | 0 → 0 | 664 → 664 | 100 → 100 | 3 / 3 → 3 / 3 | 77 → 77 |
| dsp_ippv2 `--pdata` | 6,465,008 → 6,465,008 | 32,648 | 0 → 0 | 974,527 → 974,527 | 836 → 836 | 435 / 1 → 435 / 1 | 10 → 10 |
| dsp_ippv2 縮小（種 452） | 3,038,781 → 3,038,781 | 8,030 | 0 → 0 | 391,396 → 391,396 | 563 → 563 | 434 / 0 → 434 / 0 | 7 → 7 |
| QFC `--pdata` | 209,259 → 209,259 | 6,712 | 0 → 0 | 104 → 104 | 5 → 5 | 0 / 0 → 0 / 0 | 0 → 0 |
| FLEngine `--pdata` | 2,528,030 → 2,528,030 | 59,297 | 0 → 0 | 525 → 525 | 20 → 20 | 4 / 0 → 4 / 0 | 0 → 0 |

- **`bad_nodes` はどこも増えなかった**（全構成で 0）。**新しく解決された site は 0**（`msvc_tables`、`jumptabs` が前後同一）。
- 統計の出力の全行を文言を正規化して比べると、違うのは `msvc_rejected` の 1 行ずつだけ: Blend2D（種なし、種あり）の `0x48a208b`、dsp_ippv2（`--pdata`、縮小）の `0x37cc80b`: **`N not derived: no ja before…` → `R10 is written 3 times in the function (the proof needs exactly one lea R10,[rip+d] = ImageBase)`**。QFC、FLEngine は文言を除いて全行同一。
- 生成 C: Blend2D（種なし、種 58 個あり）と縮小 dsp_ippv2 は、HEAD の `fe-lift` の出力と `cmp` でバイト一致（`tmp/b2d0_*.c`、`b2d58_*.c`、`dspred_*.c`）。
- 拒否理由の文言を変えたので、「`no ja before the table idiom`」の行は全 DLL で括弧の中が `(only cdqe, lea B,[rip+d] and nop may sit between)` に変わる（件数は 1 つずつ減って、上の基底証明の理由へ移る）。

## 7. 3 モジュール構成の測定（測定のみ。gdb の仮の値なし、runtime の応答追加なし。新しい停止は直していない）

### 7.1 手順（実測）

- 生成器: HEAD a64a41c の展開（`src0092/`）に、決定 0092 の `lift/runtime/fle_sys.c`、`fle_sys.h`、`lift/crates/fe-lift/src/imports_table.rs` を**ワークツリーから読み取りでコピー**（`with0092/`。後で、これら 3 つが 38df911 のコミットの内容と `cmp` で一致することを確認）し、0093 の変更を重ねた。release の `fe-lift`、`fe-pe`（`lift/out-nop93/target-0092/release`）。
- FLEngine の生成 C（`--pdata`）の SHA-256 は **`feb5b9087f8d…`**（0092 の基準。`EXPECT_FLE_SHA` で渡した）。縮小 dsp_ippv2（基底 0x2000000、種 452）、Blend2D（基底 0x4700000、`--pdata --seeds seeds58-4700000.txt`。種 58 個）。`lift/out-nop93/three/run-gen.sh`（`out-srw/three/harness-two` の無変更のコピー）。
- **生成 C は 0092 の測定と同一**: Blend2D の生成 C の SHA-256 `a240ceec…c1068c`（0092 の `out-srw/three/v-b2d58/gen/blend2d.c` と同じ）、縮小 dsp_ippv2 `f2f23c67…e4ed13`。**0093 は生成 C を変えない**ので当然（§6.2）。表の統合: 59,565 + 8,298 + 2,334 → 69,661 行（複数の表に在る行 268、`f_<VA>` の衝突 0）。two-native 612,296,520 B（2 GB 未満）。`build.err` 0 行。
- a4: `lift/out-nop93/three/run-a4.sh`（`harness/a4-survey.sh`、`two-native-run.sh`）を 2 回。

### 7.2 a4 の行（2 回: 00:24:53Z、00:25:03Z。gdb なし。ログ `lift/out-nop93/three/out/a4/`）

```
A4 survey 20261025T002453Z   dll: …/binaries/win64/FLEngine_x64.dll   harness exit: 134
static : units=942 runnable=895 empty=46 noproc=1 (header 0x13768b0, units 0x13768e0)
stop   : fle: indirect call to 0x48a20ff is not a known function entry
         top of stack: 0x48a2060 f_48a2060
depth  : init_count=150 / 942   last unit: index=149 run_rank=137/895 proc=0x664260 in_progress
         started=137 completed=136 in_progress=1 of 895 runnable   all_units_done=no
         entry check: started_but_not_entered=0 runnable_entered_beyond_init_count=0
funcs  : executed (distinct)=1069  entries=27332
```

- 2 回の出力は見出しの時刻と `logs` 行以外同一。stderr は `cmp` で一致、stdout は `rsp=` の行を除いて一致。
- **停止メッセージ全文**: **`fle: indirect call to 0x48a20ff is not a known function entry`**（`abort kind: fle_dispatch_miss`、`dispatch target: 0x48a20ff`、`call depth at stop: 20`）。
- **C のスタック**（先頭 10 行。`#0 0x48a2060 f_48a2060` ← `#1 f_489d880` ← `#2 f_476ed70` ← `#3 f_4899d90` ← `#4 f_4701000` ← `#5 f_48b5b90` ← `#6 f_48a59a8` ← `#7 f_489e2b4` ← `#8 f_489e264` ← `#9 f_489e450`）。**0092 の測定（`out-srw/three/out/a4/*-native.txt`）の停止報告から `entry log`（先頭 256 件）までと `diff` で差なし**（a4 の行、停止、スタック、entry log が同一）。
- `f_48a2060` の `jmp r9` は 0092 の測定と同じく `fle_dispatch(R[9]); return;` のまま（`stats/after.b2d58.txt` の `msvc_rejected 0x48a208b: R10 is written 3 times…`）。停止時のレジスタ `r8 = 0x3`、`r9 = 0x48a20ff`、`rcx = 0x48f32c8`（gdb。0092 の測定と同じ）。

### 7.3 読み取りだけの gdb（`return`、`set var`、`call`、値の書き込みなし。`lift/out-nop93/gdb/ro_nop93.py` は `lift/out-srw/gdb/ro_srw3.py` の無変更のコピー。ログ `lift/out-nop93/run/m3-ro1b-*`、`m3-ro2b-*`）

2 回実行し、**import のログは `cmp` で全バイト一致**。

- **Blend2D の DllMain(ATTACH) は戻ったか**: **戻っていない**（`DLLMAIN-RETURN reason=1: out of scope (did not return normally)`。スタックに `f_489e584` が残る。停止時の `rax = 0x48f32c8` は途中の値）。
- **LoadLibraryW が 0x4700000 を返したか**: **返していない**（`LOADLIBRARY-RETURN … R[0]=0x2000000`（dsp_ippv2）が 1 回だけ。Blend2D の `fle_realmod_load` は戻っていない）。
- Blend2D の DllMain に入るまでの `fle_imp_*` は 1,525 回。
- **ATTACH 内の import の列**: **183 回**（通し番号 1526〜1708。0092 の測定と同じ）。最初は `GetSystemTimeAsFileTime`、`QueryPerformanceCounter`、`GetCurrentThreadId`、`GetCurrentProcessId`、`QueryPerformanceCounter`、`LoadLibraryExW`（`api-ms-win-core-synch-l1-2-0`）、…。最後の 9 個: `GetModuleHandleA`（1700）、`GetProcAddress`（`WaitOnAddress`、`WakeByAddressSingle`、`WakeByAddressAll`。1701〜1703）、`AcquireSRWLockExclusive`（1704）、`ReleaseSRWLockExclusive`（1705）、`GetSystemInfo`（1706）、`HeapAlloc`（1707）、`InitializeCriticalSection`（1708）。名前ごとの回数は `lift/out-nop93/run/m3-attach-counts.txt`（`InitializeCriticalSectionAndSpinCount` 83、`GetProcAddress` 15、`HeapAlloc` 9、`EnterCriticalSection` 7、`LeaveCriticalSection` 7、`LoadLibraryExW` 6、`MultiByteToWideChar` 6、`WideCharToMultiByte` 6、…）。名前付き停止の import は無い。
- **0092 の測定（`out-srw/run/m3-ro1-imports.txt`）との比較**: import の行（通し番号・名前）、DllMain／LoadLibrary の行、`CELLS`、停止時のレジスタが、ホスト（Linux）のスタックとヒープのアドレス（`0x7ff…`、`0x5555…`、`0x7ffff…`）を伏せると**同一**（`diff`）。伏せずに差が出るのは 2 行（`QueryPerformanceCounter` の `rcx` = ホストのスタックのアドレス）だけ。
- CELLS（`[0x48f3280]`、`[0x48f3288]`、`[0x48f3290]`、`[0x48f31a0]`）は停止時もすべて 0（0092 と同じ）。
- **停止点の分類**: **(v) その他**（MSVC の switch 表 `jmp r9` が解決されず `fle_dispatch(R[9])` のまま、移動先 `0x48a20ff` が既知の関数入口でない）。(i) 未応答の import、(ii) TLS、(iii) JIT、(iv) ロケール、のどれでもない。unit 158（quickfontcache_x64.dll の LoadLibraryW）には届いていない。
- 補足（手順の訂正）: 最初の gdb の実行（`m3-ro1-*`、`m3-ro2-*`）は `out-three/gdb/ro_imports3.py` を使ったが、その `LoadRet` の行番号（`fle_load.c:153`）が現在の `fle_load.c`（`fle_realmod_load` の呼び出しは 154 行目）と合わず、`LOADLIBRARY-RETURN` の行が `fle_realmod_load` の戻りの値ではなく、`load_by_name` の途中の別の行で止まった値（`R[0]=0x0`、`0x17`、`0x3f` など、LoadLibrary の戻り値ではない）を記録した（`m3-ro1-imports.txt` に `R[0]=0x2000000` の行は 0 件）。このため、`m3-ro1`、`m3-ro2` の `LOADLIBRARY-RETURN` の行は LoadLibraryW が 0x2000000 / 0x4700000 を返したかの判断には**使えない**。import の列（通し番号と名前）は m3-ro1b と同じ。0092 の測定が使った `ro_srw3.py`（行番号 155）に替えて取り直したのが `m3-ro1b-*`、`m3-ro2b-*`。両方残してある。

## 8. 解釈が分かれた点、実装できなかった点（PM への報告）

決定の内容は変えていない。実装できなかった決定の項目は無い（ただし次の 1 のとおり、決定の目的「3 モジュールの停止点を動かす」は達成されない）。

1. **決定の背景の見込みが実測と合わなかった**（最重要）。「`lea r10,[ImageBase]` が入口にあり、`jmp r9` 以外の間接 jmp は無いはず」 → 実測は **r10 への書き込み 3 回、`jmp r9` 以外の間接 jmp 3 つ**（§3）。基底証明は決定の項目 3 で範囲外なので変えていない。結果、`recognize_msvc` は通るが `prove_base` が `R10 is written 3 times` で拒否し、停止点は動かない。**動かすには基底証明の変更が要る**: 例えば (a)「書き込みがちょうど 1 回」を「この jmp に到達する定義が `lea ImageBase` の 1 つだけ」（到達定義。決定 0090 の 1' の範囲外。§3 の実測: 到達する定義は `lea` だけ）に、(b)「他の間接 jmp があれば証明しない」を「他の間接 jmp の表を辺にして同じ証明をする」（0090 の「今回しない」拡張）に、置き換える。**どちらも PM の判断で、実装していない**。この関数は dsp_ippv2 にも 1 つある（`f_37cc7e0`）。
2. **「`nop` 系の詰め物」の範囲**: `Mnemonic::Nop` のうち、iced-x86 が「レジスタ・メモリへの書き込みなし、`rflags_modified() == 0`」と報告するもの。`66 90`（`xchg ax,ax`）は iced-x86 が `Nop` と報告するので入る。`xchg eax,eax`（`87 c0`。rax の上位を 0 にする）、`pause`、`endbr64`、予約 nop（`0f 19`〜`0f 1e`）は別の mnemonic で入れない。
3. **`MSVC_SKIP`（2）の数え方**: `cdqe`・`lea`・`nop` の合計。この関数は nop 1 つで足りるので、上限を緩める必要は出ていない。**別の問題として `MSVC_IDIOM_MAX`（`landings_in_idioms` の窓。個数ではなくバイト数）を 55 → 71 に広げた**（nop が最長 15 バイトなので、長い nop が 2 つ並ぶ idiom が窓を超える。窓が足りないと詰め物の中への分岐を見逃す fail-open）。この関数には不要（33 バイト）。これを「上限を緩めた」と読むかは PM の判断。
4. **`verify_msvc` は変更しなかった**: 現状のコードで詰め物を誤って拒否しない（テストで確認）。決定の項目 2 の「最小の変更」を、変更ゼロで満たした。
5. **拒否理由の文言を変えた**（`msvc_rejected` の「`no ja before…`」に `nop` を追記）。この 1 行の文字列が全 DLL で変わる。文言を grep するツールがあれば影響する。

## 9. 限界、未確認

- **この変更だけでは 3 モジュールの停止点は動かない**（§8 の 1）。実 DLL で新しく解決した site は 0 か所。
- 母集団の誤認識の検査は、FLEngine、dsp_ippv2、Blend2D、QFC の 4 つに限る（同梱の他の DLL は未実施）。
- 実 DLL の site について、**生成 C と native の突き合わせは未実施**（解決されないので生成 C に switch が無い）。native 側だけ（24 通り）と、合成 PE（12 バイトの nop を含む）での native と生成 C の両方（索引 0..N+3）を実施した。
- `is_padding_nop` は iced-x86 の `InstructionInfo` に依存する（`Nop` の書き込みなし、フラグ変更なしの報告）。nop 以外の命令は許さない。
- 3 モジュールの測定は gdb の仮の値なし、runtime の応答追加なし。0092 の runtime はワークツリーからのコピー（38df911 と `cmp` 一致）。実 Windows での挙動は未確認。wasm は使っていない（native のみ）。
- 到達定義の参考値（§3）は capstone の線形走査と、`jmp r11` の表をエントリが関数内である間だけ読んだ結果。決定の根拠にしたものではない。

## 10. 作ったファイル

- 新規（追跡対象にする候補。PM がワークツリーへ移す）: `lift/out-nop93/docs-new/0093-msvc-switch-nop-padding.md`、`lift/out-nop93/docs-new/blend2d-switch-nop.md`（この文書）、`lift/out-nop93/docs-new/README-0093-line.txt`（`docs/README.md` の 0092 の行の次に足す 1 行）。
- 差分: `lift/out-nop93/patches/0093.diff`（`38df911` の `docs/README.md`、`lift/crates/fe-decode/src/jumptab.rs`、`fe-decode/tests/msvc_common/mod.rs`、`fe-decode/tests/msvc_switch.rs`、`fe-lift/tests/msvc_switch.rs` との `diff -u`。`patch -p1` と `git apply --check` で確認）。
- 作業物（`lift/out-nop93/`、gitignore 対象）: `src-orig/`（HEAD a64a41c の展開）、`src/`（+0093）、`src0092/`（+0092 のコピー +0093）、`src-head/`（38df911 の展開。差分の基準）、`with0092/`（ワークツリーから読み取りでコピーした 3 ファイル）、`analysis/`、`logs/`、`stats/`、`stats.sh`、`three/`（3 モジュール）、`run/`、`gdb/`、`target/`、`target-0092/`、`target-base/`、`test-ws.txt`、`test-ws-with0092.txt`、`patch-tree/`。
