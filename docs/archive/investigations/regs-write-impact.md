# Capstone の regs_write の欠陥が既存の分類に与えた影響

対象: `binaries/win64/FLEngine_x64.dll`（読み取りのみ）。
分析: `lift/out-regsfix/`（新規）。既存の `lift/out-indir/`、`lift/out-zexact/`、`lift/out-table/`、`lift/out-seed/`、`lift/crates/`、`docs/investigations/*.md` は変更していない。

「実測」は計算と逆アセンブル、または別デコーダとの比較の結果。「推測」は未検証の解釈。

## 0. 結論（先に要点）

- Capstone 5.0.7 の `regs_write` は、宛先が明示オペランドの命令で宛先を含まないことがある。`mov` だけでなく `add`、`sub`、`xor`、`lea`、`pop` などでも起きる（実測）。
- 閉包内の間接 call の分類（(a)〜(d)）は、欠陥を補うと大きく変わる。`zexact6` の最終版は既に補正済みで、補正版（iced）の再計算と一致した（実測）。
- 欠陥の影響を受けるのは、主に **`indir_scan*.py` の読み手分類（`indirect-readers-852.md` の数字）** と、途中版の `zexact-*` 〜 `zexact5-*`。最終版 `zexact6-*`、`table-reach-exact.md` の数字は変わらなかった（実測）。
- `closure-count.md` の「未解決 6,478 / 6,490」は、Capstone ではなく Rust（iced-x86）の `fe-decode` で得た数字なので、この欠陥の影響は受けない（実測。コードの根拠は後述）。

## 1. regs_write の欠陥の一覧（実測）

### 1.1 方法

- 対象: `indir_scan.pdata_tasks()` が返す pdata の関数範囲を線形に Capstone で復号した全命令。2,566,540 命令（関数数は計測で出力していない）。
- 各命令の「書き込むレジスタ集合」を 3 通りで得た。
  1. Capstone の `regs_write`（既存の `indir_scan.build()` が使う値）
  2. Capstone の明示オペランドの書き込み属性（`CS_AC_WRITE`）
  3. iced-x86 1.21.0 の `InstructionInfo`（`OpAccess` が WRITE / READ_WRITE / COND_WRITE / READ_COND_WRITE のもの）
- 基準は iced-x86 とした（推測を含む前提。iced の `COND_WRITE` は、cmov や set 系で書き込みとして数えている）。
- 別デコーダとして GNU objdump との照合は行っていない（残りの作業を参照）。

### 1.2 件数（実測）

| 区分 | 件数 |
|---|---:|
| 命令数 | 2,566,540 |
| 宛先が `regs_write` に欠けた命令（iced にあり、Capstone の `regs_write` に無い） | **1,475,159** |
| うち、Capstone の明示オペランドの書き込み属性には入っていたもの | 1,475,107 |
| うち、明示オペランドの属性にも入っていなかったもの（内容は未分析） | 52 |
| `regs_write` に余分に入っていた命令（読むだけのレジスタを書き込みとして返す） | 2,514 |
| 長さの不一致（Capstone と iced の命令長が違う） | 0 |

注: 欠けた命令のうち、宛先のオペランドが `reg` のものは 1,474,912、`mem` のものは 247（`mem` のオペランドに対しては、暗黙の書き込み、例えば `push`/`pop` の rsp などの扱い）。

### 1.3 ニーモニック別（主なもの。実測）

欠けた件数（`regs_write` に宛先が無い）:

| ニーモニック | 件数 | 注 |
|---|---:|---|
| `mov` | 825,409 | 最多 |
| `lea` | 184,847 | |
| `sub` | 90,558 | `sub rsp,N` を含む |
| `pop` | 88,040 | 宛先（rbp 等）が欠ける。暗黙の rsp は `regs_write` に入っている（census の例 0x423664 で確認） |
| `add` | 58,949 | |
| `xor` | 44,318 | `xor eax,eax` を含む |
| `movzx` | 34,411 | |
| `movsxd` | 22,518 | |
| `and` | 8,930 | |
| `shl` | 8,062 | |
| `shr` | 5,586 | |
| `sar` | 1,697 | |
| `imul` | 5,112 | |
| `or` | 6,815 | |
| `setcc` 系（sete、setne、setg、setl 等） | 8,618 | 全種の和（census-summary.txt の値から計算） |
| `cmovcc` 系 | 約 140 | |
| SSE 系（`movss`、`movsd`、`cvt*`、`mul*`、`add*`、`sub*`、`movaps` 等） | 未集計（概算で数万件。合計は未計算） | 主なもの: `movss` 11,176、`cvtss2sd` 10,278、`cvtsi2sd` 5,108 |
| `lock xadd`、`lock cmpxchg`、`lock xchg`、`xchg` | 約 250 | |

余分に入っていた（`regs_write` が読むだけのレジスタを含む）: `cdq` 607、`cqo` 87、`stosq` 1,820（計 2,514。いずれも rax、rcx など読むだけのレジスタを書き込みに含める）。

### 1.4 実測の根拠（代表例）

| 命令 | Capstone の `regs_write` | iced の書き込み |
|---|---|---|
| `0x412173 mov rbx,r9` | 空 | rbx（WRITE） |
| `0x41221f mov rax,[rbx]` | 空 | rax（WRITE） |
| `0x41223e call [rbx+rsi*8]` | rsp | rsp（READ_WRITE） |
| `0x4235f5 mov rbp,rsp` | 空 | rbp（WRITE） |
| `0x423664 pop rbp` | rsp（暗黙の rsp のみ） | rbp（WRITE）と rsp（READ_WRITE） |

比較ツール: `lift/out-regsfix/probe_regs.py`、`probe2.py`、`probe3.py`（試し実行。`probe3.py` は iced の呼び方の誤りで途中で止まったが、結果には使っていない）。本計測: `lift/out-regsfix/census.py`。出力: `lift/out-regsfix/out/census-summary.txt`、`census-examples.tsv`（先頭の例のみ。全件は未確認）。

## 2. 分類と件数の比較（既存と修正後）

### 2.1 閉包内の間接 call（6,490 件）と間接 jmp（425 件）（実測）

既存（`zexact6`、補正済み）と、欠陥あり（`regs_write` のみ。`regsfix_run.py defect zexact`）の比較。補正済みの `iced` は `zexact6` とバイト単位で一致した（`cmp` 0）。

| 分類 | 欠陥あり（defect） | 補正後（iced = 既存 zexact6） | 差 |
|---|---:|---:|---:|
| call (a) 表（非添字、添字） | 12 | 13 | +1 |
| call (b)→(a) 引数経由で表 | 0 | 2 | +2 |
| call (b)→(c) 引数経由でメモリ | 0 | 191 | +191 |
| call (c) メモリ読み出し | 161 | 4,972 | +4,811 |
| call (d) その他 | 6,317 | 1,312 | −5,005 |
| 合計 | 6,490 | 6,490 | 0 |
| jmp (a) | 0 | 0 | 0 |
| jmp (b)→(c) | 0 | 1 | +1 |
| jmp (c) | 14 | 15 | +1 |
| jmp (d) | 411（うち IAT 409） | 409（IAT 409） | −2 |
| Z_direct（非添字、表の値が閉包外） | 8 | 9 | +1 |
| Z_direct_or_via_arg | 8 | 11 | +3 |
| Z_exact | 0 | 0 | 0 |

注: 既存の `zexact6-summary.txt` の値は、補正後（iced）と一致する。`Z_any_candidates_call` は、保存値では 9、defect では 8。

（d) の内訳の変化（defect）: `entry_reg` 3,852（欠陥で、rbx や rbp などの定義を見逃して関数入口に到達した扱い）、`clobber` 2,359、`other` 106。補正後は `entry_reg` が 0、代わりに `clobber`、`outside_caller`、`depth_hops` 等に分かれた。

### 2.2 閉包内外の読み手（`indir_scan2.py` の分類、既存 `indir2b` との比較）（実測）

既存の `indir2b`（defect 版）を再現した結果（`defect-indir2`）は既存と完全一致（diff 0）。補正版（iced）との差:

| 項目 | 既存（defect） | 補正後（iced） | 差 |
|---|---:|---:|---:|
| 閉包内 間接参照 origin: `rip_direct` | 594 | 594 | 0 |
| 閉包内 origin: `unresolved` | 6,321 | 5,888 | −433 |
| 閉包内 origin: `load_rip`（新規） | 0 | 431 | +431 |
| 閉包内 origin: `lea_rip`（新規） | 0 | 2 | +2 |
| 閉包外 origin: `rip_direct` | 321 | 321 | 0 |
| 閉包外 origin: `unresolved` | 35,750 | 33,502 | −2,248 |
| 閉包外 origin: `load_rip`（新規） | 0 | 2,242 | +2,242 |
| 閉包外 origin: `lea_rip`（新規） | 0 | 6 | +6 |
| 閉包内の読み手が seed を読む（`seeds_may_read`） | 0 | 1 | +1 |
| 閉包外の読み手が seed を読む | 6 | 8 | +2 |
| seed を閉包内の読み手だけが読む | 0 | 2 | +2 |
| seed を閉包外の読み手だけが読む | 4 | 26 | +22 |
| 読み手が特定できない seed | 848 | 824 | −24 |
| `table_any_seed_run_base`（閉包内） | 8 | 9 | +1 |
| `table_any_seed_run_base`（閉包外） | 7 | 9 | +2 |
| `table_T852_base`（閉包内、閉包外） | 0、0 | 0、0 | 0 |

変わらなかった項目: 間接 call/jmp の件数（閉包内 6,490 / 425、閉包外 36,068 / 3）、オペランド形式別の件数、`rip_direct` の件数、`table_T852_base`。

注: `indir_scan2.py` の `reader_row` は同一基本ブロック内だけを見る。このため、`0x41223e` の読み出しは、欠陥の有無によらず `cross_block` のまま（`indir2` の段階では「手動の逆追跡」の経路は見えない）。

## 3. 変化の原因（代表例。実測）

### 3.1 zexact（閉包内の間接 call の分類）

| 番地 | 欠陥あり | 補正後 | 原因 |
|---|---|---|---|
| `0x41223e` `call [rbx+rsi*8]` | (d) `entry_reg@0x412160` | (b)→(a) `rip_lea@0x13a9410+arg` | `0x412173 mov rbx,r9` の書き込みを見逃したため、rbx の定義を辿れず関数入口へ。補正後は r9 を辿り、呼び出し元 `0x41d984` の `lea r9,[rip+...]`（表 0x13a9410）に到達 |
| `0x41226d` `call [rbx+rsi*8]` | (d) `entry_reg` | (b)→(a) | 同上 |
| `0x412126` `call [rbp+0x38]` | (d) `entry_reg@0x4120c0` | (c) `deref_stack_reg`（`stack_reg`） | 関数内の rbp の定義（`mov rbp,...`）の宛先を見逃し、関数入口まで遡ったため。補正後は (c) |
| `0xa99d3b` `call [rsi+rdi*8]` | (d) `entry_reg` | (a) `rip_lea@0x14e0408` | `lea rsi,[rip+...]` の宛先を見逃したため |
| `0x410715` `call [rsi+8]` | (d) `entry_reg@0x4106f0` | (c) `ptr_from_global`（`rip_load@0x15e69a0`） | `mov rsi,[rip+...]` の宛先を見逃したため（rsi の定義を辿れず入口へ） |
| `0x40d079` `call [rbx+0x28]` | (d) `entry_reg` | (d) `outside_caller+mixed` | 分類は (d) のまま。内訳が変わった（`entry_reg` → `outside_caller`） |

### 3.2 indir2（読み手の origin）

- 閉包内の `unresolved` 433 件が `load_rip` 431 件と `lea_rip` 2 件に変わった。原因は、`mov reg,[rip+X]` と `lea reg,[rip+X]` の宛先を見逃していたこと（例: `0x410715`、`0x40d253`、`0xa99d3b`）。
- seed を読む手が増えた例: `0xa99d3b`（閉包内、`lea` の表から `0xa99270`、`0xa99730` を読む）、`0x5e0fbb`（閉包外、`0x5dea70` を読む。表 0x13d9f00 の run）。

## 4. 既存の文書のうち、欠陥の影響を受ける数字の一覧（影響の一覧のみ。修正はしない）

| 文書 | 箇所 | 数字 | 影響 |
|---|---|---|---|
| `indirect-readers-852.md` | §2 の表 | 閉包内 `unresolved` 6,321、閉包外 35,750。`rip_direct` 594、321 | 変わる（iced: 5,888 / 33,502。新規の `load_rip` 431 / 2,242、`lea_rip` 2 / 6） |
| `indirect-readers-852.md` | §2 の注（cross_block 39,561、定義あり 9,735） | `indir3`（`indir_scan3.py`）の数字 | 未再計算。`indir3` は `reader_row` と `regs_write` を経由するので、変わる可能性が高い（推測） |
| `indirect-readers-852.md` | §5.1 (c) | 閉包内 6,321、閉包外 35,750 | 変わる（上と同じ） |
| `indirect-readers-852.md` | §5.1 (b) | 閉包外 6 箇所 | 変わる。閉包外の seed 読みは 6 → 8 |
| `indirect-readers-852.md` | §5.2 | 閉包内の読み手が読む 1 件、閉包外のみ 4 件、特定できない 847 件 | 変わる（iced: 閉包内 2、閉包外のみ 26、特定できない 824） |
| `indirect-readers-852.md` | §1、§3 の 0x41223e の経路（手動） | 手動の経路 1 件 | 手動の経路は変わらない（実測の経路は補正版の逆アセンブルで確認済み） |
| `table-reach-exact.md` | §0、§1、§2 | Z exact 0、Z 上限 3、(a) 13、(b)→(a) 2、(b)→(c) 191、(c) 4,972、(d) 1,312 | 変わらない（補正版 `zexact6` と iced の再計算がバイト単位で一致） |
| `table-reach-exact.md` | §4.1「regs_write の欠陥（重要）」 | 欠陥の可能性の記述 | 記述は正しい。今回、欠陥の影響範囲が確定した（本文書）。文書の数字は補正済みの値 |
| `table-reach-exact.md` | §7 の「規約違反 1」など | 作業の記録 | 本件とは別 |
| `closure-count.md` | 再計測 U-E2 の分類（(a) 2、(b) 260、(c) 573+159、(d) 5,484、jmp (a)1 など） | 未解決 6,478 / 6,490 | 影響を受けない可能性が高い。この分類の算出元は Rust の `fe-decode`（iced-x86）で、Capstone の `regs_write` を使っていない（実測。`lift/crates/fe-decode/src/indirect.rs`、`dataflow.rs` は iced を使用）。ただし、文書の「独立の走査」のスクリプトは未特定（推測） |
| `closure-count.md` | 閉包の数字（6,969 関数、460,479 命令、6,490 間接 call、425 間接 jmp） | 閉包 | 影響なし（閉包は fe-decode の iced 計測による。`regs_write` は関係しない） |
| `table-reachable-internal.md` | 「Capstone を使った」（§80） | 表の到達の数字（run、Z_ref 等） | 未確認。Capstone で逆アセンブルしている（§27）。この文書の計算が `regs_write` を使うかは未確認 |
| `seed-0x41e7e0.md`、`ref-852-function-pointers.md` など | 個別の番地の記録 | 0x41d050、0x41223e などの番地 | 未確認（今回は読んでいない） |

補足: `lift/out-table/table_reach.py` は Capstone を import しているが、`regs_write` の参照は無い（grep 実測）。このため、この欠陥の影響は受けないと推測されるが、未検証。

## 5. 変わったもの・変わらなかったもの（結論）

### 変わったもの（実測）

1. 欠陥あり（defect）と補正後（iced）の、閉包内の間接 call の分類（(a)〜(d)）の件数。最大の変化は (c) 161→4,972 と (d) 6,317→1,312。
2. 閉包内外の読み手の origin（`unresolved` が減り、`load_rip` と `lea_rip` が増えた）。
3. seed の読み手の件数（閉包内 0→2、閉包外のみ 4→26、特定できない 848→824）。
4. 途中版（`zexact-*`〜`zexact5-*`）の数字の一部は、欠陥を含む条件だった可能性がある（推測。`zexact5` は `zexact6` とほぼ同じ分類のため、補正済みの条件と思われるが、`zexact` 〜 `zexact4` は未検証）。

### 変わらなかったもの（実測）

1. 閉包（6,969 関数、460,479 命令）。`fe-decode`（iced）による計測のため。
2. 間接 call 6,490、間接 jmp 425、閉包外の間接 call 36,068 と jmp 3 の件数。
3. `zexact6`（最終版）の全数字（`zexact6-insns.tsv` と `zexact6-targets.txt` が、補正版の再計算とバイト単位で一致）。
4. `table-reach-exact.md` の結論（Z exact 0、Z 上限 3）。
5. `table_T852_base`（閉包内外とも 0）。
6. `0x41d050` が表の [1] に当たるという事実、0x41223e の呼び出し条件（逆アセンブルによる）。

## 6. 推測（未検証）

- iced-x86 の `InstructionInfo` が正しいとみなした（基準の妥当性は別のデコーダで未検証）。
- `COND_WRITE` を書き込みとして数えたことで、cmov や set 系の件数が多くなっている可能性。
- `indir3`（`indir_scan3.py`）は再計算していない。`indirect-readers-852.md` の `cross_block` 関連の数字は未確認。
- `closure-count.md` の U-E2 の分類の算出元スクリプトは特定できていない。
- `0x41223e` が実行時に `rsi=1` で `0x41d050` を読むかは未検証（既存文書と同じ）。

## 7. 残りの作業量

- 別デコーダ（GNU objdump の AT&T 形式と iced の比較）による照合: 未実施。
- 欠けた 52 件（明示オペランドの属性にも無いもの）の内容: 未分析。
- `indir_scan3.py` の欠陥版と補正版の再計算: 未実施（約数分の計算。`indir3-cross-block.tsv` の 13 MB を新規に作る必要がある）。
- `table_reach.py`（`out-table`）と `table-reachable-internal.md` の、Capstone 依存の確認: 未実施。
- `closure-count.md` の U-E2 分類の算出元（「独立の走査」）の特定: 未実施。
- 途中版 `zexact`〜`zexact4` の欠陥の有無の確認: 未実施。

## 8. 成果物と失敗・拒否（実測）

成果物（すべて新規。既存ファイルの上書きなし）:
- `lift/out-regsfix/census.py`（欠陥の計測）
- `lift/out-regsfix/regsfix_run.py`（defect / iced の再実行）
- `lift/out-regsfix/probe_regs.py`、`probe2.py`、`probe3.py`（試し実行。`probe3.py` は途中で停止。結果には未使用）
- `lift/out-regsfix/out/census-summary.txt`、`census-examples.tsv`（43 MB）
- `lift/out-regsfix/out/defect-indir2/`、`iced-indir2/`、`defect-zexact/`、`iced-zexact/`、`run-*.log`
- `lift/out-regsfix/tmp/pylib/`（iced-x86 1.21.0 を pip で導入。`--target` で TMPDIR 内に入れた）

失敗・拒否:
- 権限拒否: なし。
- 削除（rm）: なし。既存ファイルの上書き: なし（`'x'` モードと、新規のパスのみ）。
- git 操作: なし。
- `/tmp`、`cd /tmp`: なし。
- sleep、timeout: なし。
- 作業ディレクトリの指定の誤り（`cd lift/out-regsfix/out` が相対パスの二重指定で 2 回失敗）。副作用なし。
- `probe3.py` の iced の呼び方の誤り（途中で停止）。副作用なし。
- `pip install` のキャッシュは、既存の `~/.cache` を使った可能性（推測。TMPDIR の環境変数は設定していない）。
- 2 GB 級の中間ファイルは作っていない（最大 43 MB）。
