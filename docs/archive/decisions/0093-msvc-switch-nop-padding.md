# 0093: MSVC switch 表の認識（`recognize_msvc`）で、`ja` と表引きの間に挟まる中立な詰め物 `nop`（多バイト形を含む）を許す

状態: **決定（仮ではない）**。PM の決定（2026-10-25）を、作業者（サブエージェント）が記録した。実行ではなく**バイト列で検証できる**ので仮の決定にしない。決定の節と背景は実装の前に保存し、「実装した場所」「テスト」「検証」「解釈が分かれた点」「限界」は実装と検証の後に書いた（記入済みの節はその旨が分かる）。**背景の「作業者が実装前に確認した事実」は、PM の見込み（認識が通れば解決される）と合わない点を含む。決定の内容は変えていない**（基底証明は範囲外）。

事実の出典（節番号は各文書のもの）:

- `docs/decisions/0089-msvc-switch-tables-with-proven-base.md`（コミット d87e0ad）、`docs/decisions/0090-msvc-switch-base-dominance.md`（コミット b0dc3ab）。
- `docs/investigations/blend2d-srw.md`（決定 0092 の作業。この作業を始めたときはワークツリーの未コミット・未追跡、終わりの時点ではコミット 38df911）§6（3 モジュールの a4 の停止 `fle: indirect call to 0x48a20ff is not a known function entry`、top of stack `f_48a2060`）。
- この作業は HEAD a64a41c の `git archive` で始めた。途中で HEAD が 38df911（決定 0092 のコミット）に進んだ。**0093 が変えるファイル（`jumptab.rs`、`msvc_common/mod.rs`、`fe-decode/tests/msvc_switch.rs`、`fe-lift/tests/msvc_switch.rs`）は a64a41c と 38df911 でバイト一致**（`cmp`）。差分ファイル `lift/out-nop93/patches/0093.diff` はこの 4 ファイルと `docs/README.md`（0092 の行の次に 0093 の行）で、`git apply --check` は現在のワークツリーに対して通る（確認だけ。適用していない）。
- `docs/investigations/blend2d-switch-tables.md` §3（95 か所の分類）、`docs/investigations/blend2d-switch-dominance.md` §2、§4（`msvc_rejected` の内訳）。

## 決定（PM。内容を変えない）

| # | 項目 | 決定 |
|---|---|---|
| 1 | `recognize_msvc` の「`ja` と表引きの間に挟まってよい命令」 | **`nop` 系の詰め物（`Mnemonic::Nop`）を加える**。オペランドつきの多バイト nop（`0f 1f /0`）、`66` 等のプレフィックスつき（`data16 nop`）、`xchg ax,ax`（`66 90`）を含む。**レジスタ・フラグ・メモリを変えない命令のみ**。`nop` 以外（`mov`、`xchg` の別レジスタ、`int3` など）は**加えない**（`int3`、`ud2` は詰め物でも別の意味があるので入れない）。**`MSVC_SKIP` の上限は変えない**。`nop` だけの追加で足りるか（この関数は 1 つ）を確認し、**上限を緩める必要が出たら緩めずに報告**（PM が判断する） |
| 2 | `verify_msvc` | 表の idiom の内側に基本ブロックの先頭がないことの検査は、詰め物の `nop` が idiom の内側に入るので、**詰め物の範囲で誤って拒否しない**よう最小の変更で整合させる。詰め物の命令が `ja` の次〜表引きの前のブロック内にあり、そこへ分岐してくる命令がないことを要求する従来の趣旨は保つ |
| 3 | 範囲外（変えない） | 基底証明の条件（決定 0089 1(a)、0090）、N の導出（`cmp idx,imm; ja`）の規則、表の形 (c1)/(c2)、移動先の検査（同じ `.pdata` の RUNTIME_FUNCTION 内）、`msvc_rejected` の他の理由（「B が複数回書かれる」「N が導けない」ほか）。`jmp reg` の他の形、`fe-lift` の生成 C の出し方。認識されれば既存の実装で switch が出る |
| 4 | FLEngine への影響 | FLEngine の生成 C の SHA-256 は **`eb9d8e7fa74ae562742fb1999f6da9b3ca82f2884d1845b5b7319153e1b709cf` のまま**が期待。変わったら、変わった関数の一覧と原因（`nop` の詰め物を許したことで新しく認識された switch だけであること）を報告（隠さない）。`msvc_tables`、`msvc_rejected` の統計の変化を DLL ごとに記録する |
| 5 | 変えないもの・状態 | 決定 0074、0079、0080、0086、0089、0090 は変えない。**仮の決定ではない**（バイト列で検証できる）。種 58 個・基底 0x4700000 は測定用で決定ではない。**この実装は種 58 個に依存しない** |

## 背景（事実。出典の節番号）

- 決定 0092 の作業の 3 モジュール構成（FLEngine + 縮小 dsp_ippv2 基底 0x2000000 + Blend2D 基底 0x4700000。測定用の値。種 58 個）の次の停止は `fle: indirect call to 0x48a20ff is not a known function entry`（top of stack `f_48a2060`、`fle_dispatch_miss`）。a4: init_count 150/942、last unit 149、functions_distinct 1,069、function_entries 27,332（`blend2d-srw.md` §6）。
- `f_48a2060`（元の VA `0x1801a2060`）は、サイズ 0〜15 の小さなコピーを表で振り分ける関数（memcpy 風）。`rcx` = コピー先、`rdx` = コピー元、`r8` = サイズ。停止時は `r8 = 3`。
- 現状の `recognize_msvc` は、`ja` と表引きの間に `cdqe`（索引が rax のときだけ）と `lea B,[rip+d]` だけを許し（最大 `MSVC_SKIP` = 2 個）、他は `N not derived: no ja before the table idiom (only cdqe and lea B,[rip+d] may sit between)` で拒否する（決定 0089 の解釈 2）。この関数では `ja` と表引きの間に 12 バイトの多バイト nop があるので拒否される。調査の `msvc_rejected` 77 件（Blend2D 種あり）の 1 つ。

### 作業者が実装前に確認した事実（実測。`objdump -d -M intel` と capstone、`lift/out-nop93/analysis/`、`logs/`）

リベース後の Blend2D（基底 0x4700000。SHA-256 `24e01ec9…b4cb`）の `f_48a2060`（`.pdata` `[0x48a2060, 0x48a26d5)` の 1 エントリ。UNWIND_INFO のフラグ 0 = 連鎖なし。命令 346 個、`call` 無し）:

```
48a2060: mov rax, rcx
48a2063: lea r10, [rip-0x1a206a]          ; r10 = 0x4700000 = ImageBase
48a206a: cmp r8, 0xf
48a206e: ja  0x48a2180
48a2074: 66 66 66 66 0f 1f 84 00 00 00 00 00   ; nop word ptr [rax+rax] （12 バイト。1 命令。次は 0x48a2080）
48a2080: mov r9d, [r10+r8*4+0x2fd050]
48a2088: add r9, r10
48a208b: jmp r9
```

- 詰め物は **12 バイトの 1 命令**（`66 66 66 66 0f 1f 84 00 00 00 00 00`。iced-x86 / capstone とも `nop word ptr [rax+rax]`）。したがって `MSVC_SKIP` = 2 のうち 1 を使う。
- 表は RVA 表 0x49fd050（RVA `0x2fd050`）、N = 15（`cmp r8,0xf; ja`）、16 エントリ（r8 = 0..15）、**16 個すべて異なり、すべて同じ関数内の命令の先頭**（r8 → 移動先: 0 → 0x48a208e、1 → 0x48a2174、2 → 0x48a20c8、3 → 0x48a20ff、4 → 0x48a217a、5 → 0x48a215f、6 → 0x48a2150、7 → 0x48a20d0、8 → 0x48a216d、9 → 0x48a2135、10 → 0x48a2126、11 → 0x48a20b0、12 → 0x48a2143、13 → 0x48a2110、14 → 0x48a20e8、15 → 0x48a2090）。
- 0x48a2060〜0x48a208b の中へ（`0x48a2060` 以外へ）入る直接分岐は無い。
- **PM の見込み「`lea r10,[ImageBase]` が入口にあり、`jmp r9` 以外の間接 jmp は無いはず」は、実測では成り立たなかった**:
  - **r10 への書き込みが 3 回**ある: `lea r10,[rip-0x1a206a]`（0x48a2063）、`movzx r10d,byte [rdx+0xe]`（0x48a209b。r8 = 15 のコピーのブロック）、`mov r10,rdx`（0x48a25f3。大きなコピーの後半）。
  - **`jmp r9`（0x48a208b）以外の間接 jmp が 3 つ**ある: `jmp r11`（0x48a22d2、0x48a2402、0x48a2576。どれも `mov r11d,[r10+r11*4+T]; add r11,r10` の表引きで、索引は `shr r11,5`／`shr r11,4`）。
  - したがって決定 0090 の基底証明（書き込みが ImageBase の `lea` ちょうど 1 回、同じ関数に他の間接 jmp なし）は、`nop` を許して認識が通っても**証明できない**（`R10 is written 3 times in the function`）見込み。基底証明は決定 3 の範囲外なので**変えない**（変更は未決）。
  - 参考（到達定義。実測。決定ではない）: `jmp r9` に到達する r10 の定義は 0x48a2063 の `lea` だけ（他の 2 つの書き込みの定義は、4 つの `jmp` のどれにも到達しない。`jmp r9` の 16 エントリと `jmp r11` の 3 つの表の 27、18、9 エントリを辺に入れて計算。`lift/out-nop93/analysis/reaching_r10.py`）。ただし決定 0090 の 1' は到達定義の解析を範囲外にしている。
- 同じ形（`cmp r8,0xf; ja; nop; mov r9d,[r10+r8*4+T]; add r9,r10; jmp r9`）は dsp_ippv2（基底 0x2000000）にも 1 つある（`f_37cc7e0`、`0x37cc80b`）。r10 の書き込み 3 回、他の間接 jmp 3 つで同じ。

## 範囲（今回扱う）

- `recognize_msvc` の「`ja` と表引きの間」に `Mnemonic::Nop` の命令を許す。`MSVC_SKIP` の上限（2 個）は `cdqe`・`lea B,[rip+d]`・`nop` の合計に対して変えない。
- `verify_msvc`、`landings_in_idioms` が、詰め物の `nop` を idiom の内側として従来どおり扱う（`nop` へ分岐する命令があれば拒否）ことの確認と、そのために要る最小の整合。

## 範囲外（今回扱わない。従来どおり）

- 基底証明（決定 0089 1(a)、0090 の dominance、書き込みがちょうど 1 回、他の間接 jmp があれば証明しない、揮発 B と call）。
- N の導出（`cmp idx,imm; ja`）、表の形 (c1)/(c2)、移動先の検査、`msvc_rejected` の他の理由（B が複数回書かれる、N が導けない ほか）。
- `nop` 以外の命令（`mov`、`lea` で B 以外へ、`xchg`、`int3`、`ud2`、`pause`）を `ja` と表引きの間に許すこと。
- 決定 0074、0079、0080、0086、0089、0090 は変えない。58 個の種は決定にしない。

## 実装した場所

- `lift/crates/fe-decode/src/jumptab.rs`（差分は 5 か所。`fe-lift`、`lib.rs` は変更なし）:
  - `is_padding_nop(&Instruction) -> bool`（新規。`pub`）: `Mnemonic::Nop` で、かつ `iced_x86` が報告する**レジスタへの書き込みなし・メモリへの書き込みなし・`rflags_modified() == 0`**。`90`、`66 90`（`xchg ax,ax`）、`0f 1f /0` の 3〜9 バイト形、`66` を重ねた 12 バイト形（Blend2D の形）、`66` 6 個と `2e` つきの 15 バイト形、`48 90`、`0f 1f c0`（レジスタ形）が当たる。当たらない: `xchg r8,rax`（`49 90`）、`xchg eax,eax`（`87 c0`。rax の上位を 0 にする）、`pause`（`f3 90`）、`int3`、`ud2`、`hlt`、`mov`、`lea` で B 以外、予約 nop（`0f 19`）、`endbr64`。
  - `recognize_msvc`: `ja` へ戻る歩きの `neutral` に `|| is_padding_nop(i)` を足した（`cdqe`、`lea B,[rip+d]` の許可は変えない）。`MSVC_SKIP`（2）は `cdqe`・`lea`・`nop` の**合計**に効く（変えない）。拒否理由の文言は `(only cdqe, lea B,[rip+d] and nop may sit between)` に更新（`msvc_rejected` の統計に出る文字列が変わる。件数は変わらない）。
  - `MSVC_IDIOM_MAX`: `7 * MSVC_SKIP` を `15 * MSVC_SKIP` に変えた（窓の幅 55 → 71 バイト。`landings_in_idioms` が「ガードを飛ばして idiom の中へ着地する分岐」を探す窓で、`guard_cmp` の先頭から `jmp` の先頭までの距離がこの値以下の表だけを見る）。従来の見積もりは「`MSVC_SKIP` 個の命令 × 7 バイト」で、`cdqe`（2 バイト）と `lea B,[rip+d]`（7 バイト）には十分だったが、nop は 1 命令が最長 15 バイト（x86 の最長命令）なので、**長い nop が 2 つ並ぶ idiom**（`cmp` 7 + `ja rel32` 6 + 15 × 2 + `movzx` 10 + `mov` 10 + `add` 3 = `jmp` の先頭まで 66 バイト）が従来の 55 を超える。この関数（12 バイトの nop 1 つ）の `cmp r8,0xf`（0x48a206a）から `jmp r9`（0x48a208b）の先頭までは 0x21 = 33 バイトで、従来の窓でも足りていた。**これは `verify_msvc` の整合ではなく、`landings_in_idioms` の窓の整合**で、窓が足りないと、詰め物の**中へ**飛び込む分岐を**見逃して**受け入れる方向に壊れる（fail-open）ので、`MSVC_SKIP` の上限は変えずに窓だけ広げた。
- **`verify_msvc` は変更していない**: 検査は「`from`（`ja` の次）から `jmp` までを 1 命令ずつたどり、`from` 以外で閉包のブロックの先頭があれば拒否」で、詰め物の `nop` は `from` から `jmp` までの run の中の通常の命令になる。詰め物が fall-through であるだけならブロックの先頭ではないので誤って拒否しない。詰め物へ分岐する命令があれば、その着地点がブロックの先頭になり従来どおり拒否する。**変更なしで項目 2 の趣旨を満たす**ことを、テストで確かめた（下の `verify_msvc_accepts_the_padding_run_and_still_rejects_a_block_start_inside_it`。`verify_msvc` を直接呼ぶ）。
- `lift/runtime` は編集していない。

## テスト

`fe-decode/tests/msvc_common/mod.rs`（合成 PE の `Sw` に `pad`、`pad_before_cdqe`、`pad_landing` を足した。`Sw::c1()`、`Sw::c2()` の既定は従来どおり（詰め物なし）。nop の各長さの定数 `NOP_1`〜`NOP_15`、`ALL_NOPS` と、nop でない命令の定数）、`fe-decode/tests/msvc_switch.rs` 31 → **45 件**（+14）、`fe-lift/tests/msvc_switch.rs` 11 → **15 件**（+4）。

- **解決されること**（`fe-decode`）: `ja` と表引きの間に 1 つの nop（1、2（`66 90`）、3、4、5、6、7、8、9、12、15 バイト）がある (c1)、(c2)。`cdqe` の前後に nop、`sub`、call、`lea` が diamond の後、揮発 B（r10）、nop 2 つ（`MSVC_SKIP` ちょうど）。
- **解決されないこと**: `ja` と idiom の間に `mov edx,1`、`mov edx,edx`、`xchg r8,rax`、`xchg eax,eax`（`87 c0`）、`pause`、`lea r11,[rip+0]`、予約 nop（`0f 19`）のどれか 1 つ（nop が隣にあっても同じ）。nop が 3 つ（`MSVC_SKIP` 超過: `too many instructions between the ja and the table idiom`）。`cdqe` + nop 2 つ。`int3`、`ud2` が詰め物の中にある形（ブロックが終わり、idiom は `ja` の fall-through から到達せず、`jmp reg` が閉包に入らない: 何も報告されず、解決もされない）。
- **詰め物への分岐**: `ja` の別の行き先経由で詰め物の先頭、中（1、3、5、7 バイト目）、2 つの nop の間、`cdqe` の後ろへ着地する分岐は、すべて `inside the switch idiom`（先頭と中は `landings_in_idioms`: `the branch at … lands at …, inside the switch idiom (the guard cmp; ja is skipped)`）。idiom の最初の命令（nop の後の `mov`/`movzx`）への着地も拒否。`verify_msvc` を直接呼ぶテストで、詰め物の run を受け入れ、ブロックの先頭が 2 つ目の nop、idiom の最初、`jmp` にあるときに拒否することを確認。
- **他の条件は変わらない**: 詰め物つきで、`lea` が片側だけ、B が 2 回書かれる、B が引数から、他の間接 jmp、揮発 B + call、移動先が別の関数／命令境界でない、別の関数からの飛び込み、ガードが別のレジスタ／無い、が従来と同じ理由で拒否される。
- **`MSVC_IDIOM_MAX`**: 窓（71 バイト）が、2 つの 15 バイトの nop を含む idiom（`cmp` の先頭から `jmp` の先頭まで 66 バイト）を覆うことを、`landings_in_idioms` を直接呼んで確認（`the_landing_window_covers_two_longest_padding_nops`。窓の中ほど、詰め物の中、`jmp` 自身への着地が拒否され、`cmp` 自身と `jmp` の次は拒否されない）。
- **変異検査**（実施した。元に戻して差分を確認）: `is_padding_nop` の `Mnemonic::Nop` 判定を外す → `which_instructions_are_padding_nops`、`instructions_that_are_not_padding_nops_…` が失敗。`landings_in_idioms` の入力を空にする → 4 件が失敗。`MSVC_IDIOM_MAX` を従来の 7 倍に戻す → `the_landing_window_covers_two_longest_padding_nops` が失敗（窓 55 < idiom 66）。**実 DLL の 1 site（この関数）には窓の変更は不要だった**（33 バイト）ので、この変更の効果は合成の形（長い nop が 2 つ）に対するもので、実 DLL の結果は変えない。`verify_msvc` の `leaders.contains` を外す → `verify_msvc_accepts_…` が失敗（最初はこのテストが無く、素通りだったので、直接呼ぶテストを足した）。
- **合成 PE を clang で native にも実行**（`fe-lift`、`harness/image.c`）: 12 バイトの nop（c1、c2）、1、2（`66 90`）、15 バイト、nop 2 つ、`cdqe` + 64 ビットガード、nop が `cdqe` の前、call、`lea` が diamond の後、`sub`、揮発 B（r10）で、索引 0..N+3（+K）のすべてで native と生成 C の結果が同じで表のバイトの指す先と一致（詰め物は native では本当に実行される）。生成 C が switch になること（case 3、`fle_jumptab_miss` 1、`fle_dispatch(R[` なし）と、解決しない詰め物の形が `fle_dispatch(R[1]); return;` のままであること。
- **実 DLL（Blend2D `f_48a2060`、`0x48a208b`）**: 
  - `recognize_msvc` が実バイト列から `MsvcForm { first: 0x48a2080, from: 0x48a2074, guard_ja: 0x48a206e, guard_cmp: 0x48a206a, base: R10, n: 15, t1: None, t2: 0x2fd050 }` を返す（`from` は 12 バイトの nop の先頭）。詰め物のバイト列は `66 66 66 66 0f 1f 84 00 00 00 00 00`（12 バイト 1 命令）。
  - 表の 16 エントリを、ファイルの RVA 表（0x2fd050、64 バイト）から読み、objdump の一覧と**全 16 件を個別に照合**し、16 個とも異なること、すべて `[0x48a2060, 0x48a26d5)` の中で独立にデコードした線形走査の命令の先頭であることを確認（`FuncMap`: 主関数 `0x1a2060`、ピース 1 つ `[0x1a2060, 0x1a26d5)`、連鎖なし）。objdump での各エントリ（命令の先頭、`ret`、`movzx ecx,byte [rdx]` など）は `lift/out-nop93/logs/fn48a2060_facts.txt` に全 16 行。
  - **native**（本物の命令。関数の入口 0x48a2060 から `rcx`/`rdx`/`r8` を与え、16 個の移動先と `ja` の行き先 0x48a2180 に `0xCC`、SIGTRAP ハンドラが止まった番地を出力）: r8 = 0..15 の 16 通りと、16、17、31、32、33、0x100、0xffffffff、`u64::MAX` の 8 通り（計 24）で、到達先がすべてファイルの表どおり（0..15 は表、16 以上は 0x48a2180）。
  - **生成 C との突き合わせはできなかった**: この site は基底証明で解決されない（下の検証と限界）ので、生成 C に switch が無く、突き合わせる対象が無い。決定の項目の「native と生成 C を r8 = 0..15 と範囲外で動かして移動先が表どおりになること」のうち、**生成 C 側は未実施**。
  - **解決されない**ことの固定: `R10 is written 3 times in the function` が理由で、`N not derived` ではない。同じ関数の `jmp r11` の 3 つ（`0x48a22d2`、`0x48a2402`、`0x48a2576`）は従来と同じ理由（`N not derived: no ja before the table idiom`）。解決された表は従来の 3 つ（`0x4701384`、`0x4701cbb`、`0x470a89e`）のまま。
- **既存の全テスト**: 期待値を変えた行は**ない**（追加のみ）。ただし `msvc_rejected` の理由の文言を変えたため、文言をそのまま固定している既存テストがあれば影響するが、`grep` で既存テストに該当の文字列は無かった（全テスト通過で確認）。

## 検証

実測値と手順は `docs/investigations/blend2d-switch-nop.md`。作業物は `lift/out-nop93/`。

| 項目 | 結果 |
|---|---|
| `cargo test --workspace`（`CARGO_TARGET_DIR=lift/out-nop93/target`、HEAD a64a41c の展開 + 0093） | **97 binaries / 1,196 passed / 0 failed / 0 ignored**（基準 97 / 1,178。+18 件 = `fe-decode/tests/msvc_switch.rs` +14、`fe-lift/tests/msvc_switch.rs` +4。SKIPPED 0 件） |
| 同、0092 のコミット（38df911）の内容を重ねたコピー | 98 binaries / 1,213 passed / 0 failed / 0 ignored（0092 のコミットのテストは 98 binaries / 1,195 passed なので +18 は同じ） |
| **FLEngine の生成 C の SHA-256** | **`eb9d8e7fa74ae562742fb1999f6da9b3ca82f2884d1845b5b7319153e1b709cf` のまま**（HEAD の `fe-lift` の出力と `cmp` でバイト一致）。変わった関数 0。functions 59,297、bad_nodes 0、unsupported_total 525、register_jmp_sites 20、jumptabs 4、msvc_tables 0、msvc_rejected 0。単独構成の a4 は生成 C が変わらないので省略 |
| **解決される見込みの数（実装前の静的な見積もり）** | 拒否理由 `no ja before the table idiom` の間にあるのが `nop` だけのもの: Blend2D（種なし）**1 件**（`0x48a208b`）、種 58 個あり **1 件**、dsp_ippv2 **1 件**（`0x37cc80b`）、dsp_ippv2 縮小 **1 件**（同じ site）、QFC・FLEngine 0 件。**ただし、そのうち「解決される」と見積もれたものは 0 件**（基底証明で B = r10 が 3 回書かれる）。`msvc_rejected` の全件（Blend2D 種なし 76、種あり 77、dsp_ippv2 10、縮小 7）について、`ja` の後ろ 12 命令に nop 系があるものを調べた結果は、上の 1 件ずつ**だけ**（`lift/out-nop93/analysis/estimate_nop2.py`、`logs/estimate2_*.txt`）。理由別の内訳（Blend2D 種あり 77 件）: 「B が複数回書かれる」33、`no ja before the table idiom` 33（うち nop だけ 1、`lea B` の後に `mov`/`movsxd` などが続く 11、12 命令戻っても `ja` が無い 19（idiom の前がブロックの先頭で `je`/`jb`/`jmp` が来る形など）、`.pdata` の外の葉関数 2）、`the closure decoded nothing before` 7、`jump is not inside a .pdata function` 2、`no cmp idx,imm right before the ja` 2 |
| DLL ごとの統計の前後 | 下の「統計の前後」。**生成 C、`msvc_tables`、`jumptabs`、`bad_nodes`、`unsupported_total` は全 DLL で前後同一**。変わった行は `msvc_rejected` の **理由の 1 行ずつ**（`0x48a208b` と `0x37cc80b`: `N not derived…` → `R10 is written 3 times in the function…`）と、文言の更新 |
| 3 モジュール（種 58 個、基底 0x4700000。測定のみ。0092 のコミットの runtime を重ねた） | **a4 の行・停止・C のスタック・import の列が、0092 の測定と同一**（停止は `fle: indirect call to 0x48a20ff is not a known function entry`、top of stack `f_48a2060` のまま）。停止点は**動かなかった**。詳細は調査 §6 |

統計の前後（HEAD の `fe-lift` → 0093。release、`--pdata`）:

| 構成 | closure_instructions | functions | bad_nodes | unsupported_total | register_jmp_sites | jumptabs / msvc_tables | msvc_rejected |
|---|---|---|---|---|---|---|---|
| Blend2D 種なし | 399,148 → 399,148 | 2,018 → 2,018 | 0 → 0 | 661 → 661 | 99 → 99 | 3 / 3 → 3 / 3 | 76 → 76 |
| Blend2D 種 58 個あり | 399,777 → 399,777 | 2,066 → 2,066 | 0 → 0 | 664 → 664 | 100 → 100 | 3 / 3 → 3 / 3 | 77 → 77 |
| dsp_ippv2（`--pdata`） | 6,465,008 → 6,465,008 | 32,648 → 32,648 | 0 → 0 | 974,527 → 974,527 | 836 → 836 | 435 / 1 → 435 / 1 | 10 → 10 |
| dsp_ippv2 縮小（種 452） | 3,038,781 → 3,038,781 | 8,030 → 8,030 | 0 → 0 | 391,396 → 391,396 | 563 → 563 | 434 / 0 → 434 / 0 | 7 → 7 |
| QFC | 209,259 → 209,259 | 6,712 → 6,712 | 0 → 0 | 104 → 104 | 5 → 5 | 0 / 0 → 0 / 0 | 0 → 0 |
| FLEngine | 2,528,030 → 2,528,030 | 59,297 → 59,297 | 0 → 0 | 525 → 525 | 20 → 20 | 4 / 0 → 4 / 0 | 0 → 0 |

Blend2D（種なし、種あり）と縮小 dsp_ippv2 の生成 C（`--seeds` つきの縮小 dsp_ippv2、`--pdata` の Blend2D 2 通り）も HEAD の `fe-lift` の出力と `cmp` でバイト一致。

## 解釈が分かれた点

詳細は調査 `docs/investigations/blend2d-switch-nop.md` §8。**決定の内容は変えていない**。

1. **「PM の見込み（認識が通れば解決される）」が実測と合わなかった**（最重要）。決定の背景は「この関数は `lea r10,[ImageBase]` が入口にあり、`jmp r9` 以外の間接 jmp は無いはず」としたが、実測は **r10 への書き込みが 3 回、`jmp r9` 以外の間接 jmp が 3 つ**（`jmp r11`）。決定 0090 の基底証明（書き込みがちょうど 1 回、他の間接 jmp があれば証明しない）は**決定の範囲外（項目 3）なので変えていない**。結果、`recognize_msvc` は通るが、解決は `prove_base` で `R10 is written 3 times in the function` と拒否される。**3 モジュールの停止は動かない**。これを動かすには、(a) 到達定義の解析（0090 の 1' の範囲外）、または (b) `jmp r9` の idiom が r10 を使う直前の定義が `lea` だけであることの検査（定義が 1 つ、という条件を「この jmp に到達する定義が `lea` の 1 つ」に置き換える）、が要る。**どちらも未決**（実測の参考: `jmp r9` に到達する r10 の定義は 1 つ、0x48a2063 の `lea` だけ。他の 2 つの書き込みの定義は、`jmp r9` と `jmp r11` 3 つのどれにも到達しない。`jmp r9` の 16 エントリと `jmp r11` の 3 つの表の 27、18、9 エントリを辺に入れて計算。`lift/out-nop93/analysis/reaching_r10.py`）。**実装していない**。
2. **「`nop` 系の詰め物」の範囲**: `Mnemonic::Nop` のうち、iced-x86 が「レジスタへの書き込みなし、メモリへの書き込みなし、`rflags_modified() == 0`」と報告するもの。`xchg ax,ax`（`66 90`）は iced-x86 が `Mnemonic::Nop` と報告する（`xchg eax,eax`（`87 c0`）は `Xchg` で、rax の上位 32 ビットを 0 にするので入れない）。`pause`（`f3 90`）、`endbr64`、予約 nop（`0f 19`〜`0f 1e`）は別の mnemonic で入れない。
3. **`MSVC_SKIP`（2）を数える対象**: `cdqe`、`lea B,[rip+d]`、`nop` の**合計**。決定は「上限を緩める必要が出たら緩めずに報告」。この関数は nop 1 つで足りる（緩める必要は出ていない）。**別の問題として、nop は 1 命令が最長 15 バイトなので、`MSVC_IDIOM_MAX`（`landings_in_idioms` の窓の幅。`MSVC_SKIP` × 7 バイトで見積もっていた）は 55 → 71 バイトに広げた**（`MSVC_SKIP` の個数の上限ではなくバイト数の見積もり。`MSVC_SKIP` は 2 のまま。実 DLL のこの関数には不要で、合成の「長い nop が 2 つ」の形のため）。これを「上限を緩めた」と読むかは未決（窓を広げないと、詰め物の**中への分岐を見逃す**方向に壊れるので、広げたほうが fail-closed）。窓を広げることで拒否が増える方向にだけ働く（偽の拒否が増える可能性: 窓の中に `guard_cmp` と `jmp` の間ではない別の着地点があっても、`tgt <= jmp` の条件があるので `jmp` を超えた着地は拒否しない）。
4. **`verify_msvc` は変更しなかった**: 決定の項目 2 は「最小の変更で整合させる」だが、現状のコードは詰め物の nop を誤って拒否しない（`from` 以外のブロックの先頭だけを見るため）。変更が要らないことをテストで確かめた。変更しなかった理由はこれだけで、PM の指示の「最小」に含まれる（変更ゼロ）。
5. **拒否理由の文言を変えた**: `msvc_rejected` の「`no ja before the table idiom`」の括弧の中に `nop` を足した。**統計の出力のその 1 行の文字列が全 DLL で変わる**ので、文言を grep しているツールがあれば影響する（`lift/out-nop93/analysis/` は本作業のもので影響なし）。件数は変わらない。

## 限界

- **この変更だけでは 3 モジュールの停止点が動かない**（解釈 1）。`f_48a2060` の `jmp r9` は解決されないままで、`fle_dispatch(R[9]); return;` のまま、`0x48a20ff` は既知の関数入口でないので `fle_dispatch_miss` で止まる。
- 実 DLL で、この変更が新しく解決した site は **0 か所**。実 DLL での新規認識の誤りの検査（母集団）は、FLEngine、dsp_ippv2、Blend2D、QFC の 4 つに限る。**変更の効果が実 DLL で見えるのは、拒否理由の更新（`N not derived` → 基底証明）の 2 行だけ**で、解決の件数は変えない。
- 生成 C の switch の native との突き合わせ（r8 = 0..15 と範囲外）は、**実 DLL では未実施**（解決されないので生成 C に switch が無い）。native 側だけを実 DLL で実施し、合成 PE（12 バイトの nop を含む）では native と生成 C の両方で実施した。
- 実 Windows での挙動は未確認。wasm は使っていない。
