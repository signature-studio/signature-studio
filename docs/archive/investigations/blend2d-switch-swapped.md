# 決定 0095（MSVC switch の swapped 形）の実装・テスト・統計・3 モジュールの測定

状態: 決定 0095 の実装・テスト・統計・3 モジュールの測定の記録（測定のみ。新しい停止は直していない）。決定は `docs/decisions/0095-msvc-switch-swapped-form.md`。作業物は `lift/out-swapped/`（`.gitignore` の `lift/out*/` に当たる）。テストの総数は §3 末尾の「テスト総数」。

凡例: **実測** = コマンドの出力を見た事実。**読み** = 実測（バイト列・ソース）から読んだ解釈。**未確認** = 確かめていない。**意見** = 作業者の意見（事実ではない）。

## 目次

0. 前提（HEAD、入力、コマンド）
1. 静的な見積もり（調査 §1.3 の F1 のうち swapped 向き）
2. 実装（`jumptab.rs`、`lib.rs`）
3. テスト
4. DLL ごとの統計の前後と見積もりとの比較
5. FLEngine の生成 C、解決した site の機械検査
6. 3 モジュール構成の測定（測定のみ）
7. 解釈が分かれた点、実装できなかった点、限界
8. 作ったファイル

## 0. 前提

### 0.1 HEAD と環境（実測）

- HEAD `0ddcbf15b338f72c6f1f87f646ee33516391efbd`（調査 `docs/investigations/msvc-switch-other-forms.md` のコミット。コードは決定 0094 の d618380 と同じ）。作業開始時の `git status --short` は未追跡 3 件（`lift/harness/wasm/trace.c.new`、`trace_native.c.new`、`lift/iat-unsup-closure.txt`。触らない）。
- 「前」の fe-lift: `lift/out-git/target-m94/debug/fe-lift`（HEAD d618380 のビルド。読み取り専用）。「後」: `lift/out-swapped/target/debug/fe-lift`（`CARGO_TARGET_DIR=lift/out-swapped/target`、`TMPDIR=lift/out-swapped/tmp`）。
- ツール: clang、objdump、capstone 5.0.7（python）、pefile。`PYTHONDONTWRITEBYTECODE=1`。

## 1. 静的な見積もり（実装の前。`lift/out-forms/res/*.sites.tsv` を読んだ数。実測の読み取り）

調査 `msvc-switch-other-forms.md` §1.3 の F1（load／`add`／`jmp` が離れる形）を、`orient` 列（`std` か `swapped`）で数えた。列: `res/b2d0.sites.tsv`（Blend2D 種なし）、`b2d58.sites.tsv`（種 58 個）、`dsp.sites.tsv`（`--pdata`）、`dspred.sites.tsv`（縮小）、`qfc`、`fle`。

| 構成 | F1 全体 | F1 swapped | F1 std | swapped の site | 決定 1・2 の条件への当たり |
|---|---:|---:|---:|---|---|
| Blend2D 種なし | 13 | **1** | 12 | `0x489f8fb`（`f_489f7d0`） | B = r9 = jmp のレジスタ、`add r9,rax`、load は `mov eax,[r9+r8*4+0x2fd000]`、間は `add rcx,r8; mov rax,r11` の 2 命令、N は edge guard の `cmp R8,0x10; Jbe(taken)` = 0x10。**当たる** |
| Blend2D 種 58 個 | 13 | **1** | 12 | 同上 | 同上 |
| dsp_ippv2 `--pdata` | 2 | **1** | 1 | `0x37cd3eb`（`f_37cd2c0`） | 同形（基底 0x2000000、表 `0x26150b0`、N = 0x10）。**当たる** |
| dsp_ippv2 縮小 | 1 | **1** | 0 | `0x37cd3eb` | 同上 |
| QFC | 0 | 0 | 0 | — | — |
| FLEngine | 0 | 0 | 0 | — | — |

- **見積もり**: swapped 向きは Blend2D **1 件**、dsp **1 件**（縮小も同じ 1 件）。2 件とも `forms-scan` の `lib_resolve` は「基底が jmp レジスタ自身なので、そのまま渡すと `add r9,rax` が非 `lea` の定義として届き拒否」だった（`tsv` の `lib_resolve` 列。0094 の規則そのままでは通らない）。見積もりでは **Blend2D 30 → 31、dsp 5 → 6（縮小 3 → 4）**。QFC・FLEngine は 0 件なので変化なし。
- F1 std（Blend2D 12、dsp 1）と F2（Blend2D 3、dsp 1）は**この決定の範囲外**で、見積もりに入れない（実装後も解決されず、`msvc_rejected` にも出ないことを §3 のテストで固定する）。

## 2. 実装（実測。ソースを読んだ事実。`lift/crates/fe-decode/src/jumptab.rs` +460 −15、`lib.rs` +31 −9）

- **`MsvcForm`** に `swapped: Option<SwapForm>` を足した（`SwapForm { load, add, index }`）。swapped のとき `n`・`from`・`guard_ja`・`guard_cmp` は 0 のプレースホルダで、`resolve_msvc` が CFG から求めて `Origin::Msvc` に書く。**`Origin::Msvc` に `swapped: Option<u64>`**（`Some(lo)`: 「ガードの分岐だけが入ってよい run」の先頭アドレス。表引きのブロックの前の詰め物 `nop` を含む）を足した。
- **`recognize_msvc`** は standard（`recognize_msvc_standard` = 0089／0093 のコードそのまま）を先に試し、`Ok(None)` のときだけ `recognize_msvc_swapped` を試す。standard の `Ok(Some)` と `Err` は swapped で変わらない。`lib.rs` の閉包は両者を直接呼び、**基底証明が `ReachingDefs`（既定）のときだけ** swapped を試す（0089／0090 の比較専用の方式では、swapped の site は従来どおり認識も報告もされない）。
- **`recognize_msvc_swapped`**（`jmp B` から遡る。閉包が復号した命令列だけを使い、命令の長さを推測しない）: `add B,rX`（rX は 64 ビット、≠ B、≠ rsp）を探し、その前に `mov eX,[B+I*4+T2]`（`table_load`。(c1) と同じ。メモリ 4 バイト、セグメントなし、RIP 相対でない、索引は 64 ビットの GPR で B・rsp でない）を探す。**X ≠ I** も要求する。挟まる命令の検査 `swap_gap_ok`: 制御を変えない（`FlowControl::Next`）、フラグを読まない（`rflags_read() == 0`）、メモリを書かない、レジスタ（`iced` の `used_registers`、暗黙の使用を含む）が、**`add` の前の隙間では B・X・I のどれにも触れない／後ろの隙間では B に触れない**。上限は新しい定数 **`MSVC_SWAP_SKIP = 2`**（両方の隙間の合計。`MSVC_SKIP`・`MSVC_IDIOM_MAX` は変えていない）。load と `add` の間に不可の命令があれば `Err`、`add` が見つからない・上限を超える場合は `Ok(None)`（報告しない）。
- **N の導出 `derive_swapped_guard`**（`resolve_msvc` が呼ぶ。関数のピースの線形走査 `Sweep` の CFG）:
  1. 表引きのブロック: load から遡って、直前の命令が真っ直ぐ落ちてきて（`FlowControl::Next`）、その命令が直接の jmp／jcc／call の行き先でない間のかたまり。
  2. 先行辺: そのブロックの先頭を行き先にする直接の jmp／jcc／call、と、直前の命令がフォールスルーで入る辺（直前が分岐先でない詰め物 `nop` は飛ばして、その前を見る。`jmp` の後の `nop` は辺でない）。**ちょうど 1 本**でなければ `N not derived`。関数の入口を含むブロックも拒否。
  3. その 1 本が直接の条件分岐で、辺の側が**符号なしの I <= N**（`ConditionCode`: 成立側なら `be`／`b`、不成立側なら否定して `be`／`b`。つまり `ja` 不成立、`jbe` 成立、`jae` 不成立、`jb` 成立）。`b` なら N = imm − 1。それ以外（`ja` 成立、`jbe` 不成立など）は `N not derived`。
  4. (b) 分岐と同じ基本ブロックの中で、フラグを最後に書く命令が `cmp I,imm`（32／64 ビット、即値）で、その後ろ（分岐まで）に I を書く命令がない。(c) 表引きのブロックの先頭から load までに I を書く命令がない。
  5. N ≥ `MSVC_MAX_ENTRIES`（4096）は拒否。
- **基底証明**: `prove_base_reaching_defs` に問い合わせ位置 `site_va` を足した。standard は jmp（従来どおり）、swapped は**表引き（load）の直前**。load に届く B の定義がすべて `lea B,[rip+d]`（= ImageBase）で 1 つ以上届く、という 0094 の規則そのまま。`add B,rX` は load の後ろ（jmp の直前の idiom 内）にあるので load には届かず、**特別な許可の規則は要らない**。idiom の途中への入り口がない（下の 2 つの検査）ので、jmp に届く B の定義は `add` だけで、その入力は load に届く `lea` の値。`add`（や他の非 `lea` の定義）が他の間接 jmp の仮想ノードや jmp 自身の表を経由して load に戻って来る場合は、0094 のとおり「非 `lea` の定義が届く」として拒否される（テストで固定）。
- **事後の検査**: `landings_in_swapped_idioms`（`[lo, jmp]` に入る分岐のうち、ガードの分岐から `from` へのもの以外を拒否）と `verify_msvc_swapped`（ガードの `cmp` と分岐を閉包が復号していること、`from`〜jmp が順に復号され途中にブロックの先頭がないこと、関数の各命令が線形走査の命令の先頭であること、関数の中に他の閉包の関数が始まっていないこと）。`foreign_entries` は standard と共通。`lib.rs` で swapped の表は専用の 2 つに回す（standard の `landings_in_idioms`／`verify_msvc` は変えていない）。
- **生成 C**: 変更なし。`Select::Target`（case = 移動先の VA、既定 = `fle_jumptab_miss`）。
- 変えていないもの: standard の認識、0093 の nop、`verify_msvc`、`foreign_entries`、`landings_in_idioms`、`MSVC_SKIP`、`MSVC_IDIOM_MAX`、`read_msvc_targets`。`fe-lift/src/main.rs` は変更なし。`lift/runtime` は編集していない。

## 3. テスト（実測）

`fe-decode/tests/msvc_switch.rs` 57 → **78 件**（+21）、`fe-lift/tests/msvc_switch.rs` 18 → **23 件**（+5）。合成 PE は `msvc_common/mod.rs` に追加（`Swp`、`build_swapped`。`SwGuard`／`SwLea`／`SwIdiom`／`SwTarget`／`SwDef`）。

**合成 PE（fe-decode）— 解決されるもの**
- `Swp::real()`（`f_489f7d0` と同形: `cmp rcx,16; jbe tbl`、12 バイトの `nop`、`lea r9`、`mov eax,[r9+rcx*4+T]`、`add r9,rax`、`add rdx,rcx; mov rax,r11`、`jmp r9`）: 17 エントリ、`Origin` の `n = 16`、`swapped = Some(nop の先頭)`、`from - lo = 12`。
- N の導出の 5 つの側: `ja` 不成立、`jbe` 成立、`jae` 不成立（`cmp N+1`）、`jb` 成立（`cmp N+1`）、32 ビットの `cmp ecx`。N = 2、5、16、0x7f、0x80、200。
- 隙間: `add` の前に B・X・I に触れない 1〜2 命令、`add` の後に 0〜2 命令（`add rdx,rcx`（I を読む）、`mov rax,r11`（X を書く）を含む）、前後の合計がちょうど上限。B・X の別のレジスタ（r10／rbx、r13＋push／pop）。
- `lea` が入口（ガードの前）にある形、B を `def` ブロックで書いて `ret` する形（戻らない枝の書き込みは届かない）。

**合成 PE — 解決されてはいけないもの（`msvc_rejected` の理由か、認識されないこと）**
- 不成立側から入る（`ja` 成立、`jbe` 不成立、`jae` 成立、`jb` 不成立）: `N not derived`（理由に「unsigned I <= N side」）。
- `cmp` が別のレジスタ、フラグが `test` から、`cmp I,reg`、`cmp` と分岐の間で I が書かれる、分岐の後・load の前で I が書かれる。
- 表引きのブロックに入る辺が 2 本以上: 前に別の分岐（`test; jne tbl`）、`def` ブロックが落ちてくる、表の移動先が詰め物の先頭へ戻る、load へ戻る（いずれも `predecessors`）、`add` へ戻る（`inside the switch idiom`）。
- F2（`mov ecx,[rcx+rax*4+T]; lea rax,[ImageBase]; add rcx,rax; jmp rcx`）、`movsxd`、load の基底が jmp レジスタでない、`add` と `jmp` が load の基底と別のレジスタ、X が索引レジスタ、(c2) の索引バイト表との組合せ: **認識されない**（`msvc_rejected` にも出ない）。
- 隙間の命令: `add` の前に X を書く（`mov rax,r11`）、I を書く、I を読む、フラグを読む（`adc`）、メモリを書く。`add` の後に B を読む、B を書く、フラグを読む、メモリを書く: 解決されない。命令数が 3（後 3、前 1＋後 2、前 3）: 認識されない。
- 基底証明（0094 のまま）: `lea` なし、ImageBase でない `lea`、`mov` と `lea` が両方 load に届く、非 `lea` の定義が他の間接 jmp に届く、caller-saved の B と `lea`〜load の間の call（callee-saved なら通る）、`lea` がガードの前にあり表の移動先が `guard` へ戻る形（`add` の定義が load に戻る: 理由に `` `Add` `` と「the table load」）。
- 移動先: 別関数、実行不可、命令の先頭でない、`.pdata` なし、別関数からの飛び込み。連鎖ピースは解決される。
- 比較専用の方式（`Dominating`、`EntryBlock`）では swapped の site は認識も報告もされない。standard の結果は変わらない。
- `verify_msvc_swapped` を直接呼ぶテスト（分岐側・フォールスルー側）、`landings_in_swapped_idioms` の窓の関数。

**実 DLL（fe-decode）**: Blend2D `0x489f8fb` の認識（`MsvcForm` の `base = R9`、`t2 = 0x2fd000`、`swapped = Some(SwapForm { load: 0x489f8ea, add: 0x489f8f2, index: R8 })`）、バイト列、17 エントリを**ファイルの表から読んで objdump／capstone の 17 件と照合**（17 件別々、全部異なる、関数 `[0x489f7d0,0x489f960)` 内、iced の線形走査の命令の先頭）、表の後ろの 0・0・0 と次の表の先頭（`0x1a208e`）、`0x489f8e0` へ入る直接分岐が `0x489f7e8` の 1 本だけ、解決（`n = 0x10`、`from = 0x489f8e0`、`guard_ja = 0x489f7e8`、`guard_cmp = 0x489f7e4`、`swapped = Some(0x489f8d4)`）、Blend2D の解決は 31（30 + 1）、F2・F1 std は解決されない。dsp `0x37cd3eb`（17 エントリ、dsp の MSVC switch は 6）。FLEngine は MSVC switch 0。

**実 DLL（fe-lift）**: `real_blend2d_f_489f7d0_every_size_reaches_the_table_target_natively_and_lifted` と `real_dsp_ippv2_f_37cd2c0_…`: **native（本物の命令を関数の入口から実行。17 個の移動先と `0x489f7ee`（`jbe` が落ちる先）に `0xCC`、SIGTRAP ハンドラが止まった番地を出力）と生成 C（17 個のラベルに `fle_hit` のプローブ）を、r8 = 0..16 の 17 通りと範囲外 8 通り（17、18、31、32、0x100、0xffffffff、0x100000000、`u64::MAX`）の計 25 通りで動かし、到達先がファイルの表どおり**（範囲外は `jbe` の落ちる先 = 生成 C の `switch` に入らない）。2 DLL × 2 側 × 25 = 100 回。合成 PE の swapped 形を native と生成 C で全索引（0..N+3）動かして表どおり（6 通りの形）。解決しない swapped は `fle_dispatch(R[9]); return;` のまま。**ガードを飛ばして表引きのブロックから入る形**（合成 PE の関数のコピー）で、索引 0..=16 は表どおりに戻り、17・18・0x100 は `fle_jumptab_miss`（`jump table at <jmp の VA>: selector … is outside the table`）で名前付き停止し、対象のコードは動かない。

**既存の期待値を変えた行**（理由: 解決される site が増えた）: `fe-decode/tests/msvc_switch.rs` の `blend2d_the_reaching_definitions_of_decision_0094_…`（30 → 31。`0x489f8fb` を追加）、`dsp_ippv2_f_37cc7e0_…`（dsp の MSVC switch の集合に `0x37cd3eb` を追加、5 → 6）、`MsvcForm` の期待に `swapped: None`、`fe-decode/tests/jt_cpu.rs` の `dsp_ippv2_has_434_stubs`（`msvc_tables` 5 → 6、`jumptabs` 439 → 440、`jmp:unresolved` 544 → 543、`jmp:msvc_table` 5 → 6）、`fe-lift/tests/jt_cpu.rs`（`(jumptabs, msvc_tables)` 437, 3 → 438, 4）、`fe-lift/tests/msvc_switch.rs` の `real_blend2d_site_is_resolved_by_default`（30 → 31）。`F16`（0094 の 16 エントリ用の駆動）は表の数と範囲外の値を持つ形に一般化した（16 エントリの 2 DLL の値は変えていない）。

**テスト総数（実測）**: **98 binaries / 1,254 passed / 0 failed / 0 ignored**（基準 98 / 1,228。+26 = `fe-decode/tests/msvc_switch.rs` +21、`fe-lift/tests/msvc_switch.rs` +5）。fe-pe 10 binaries 83、fe-decode 12 binaries 119、fe-lift 76 binaries 1,052。SKIPPED 0 件（`grep -c SKIPPED` が 0）。`cargo test --workspace` を 1 回で流すと他の作業の負荷で 1 時間の枠を超え、84 binaries 目で打ち切られたので、パッケージごとに実行した（`out-swapped/test-ws-fe-pe.txt`、`test-ws-fe-decode.txt`、`test-ws-fe-lift.txt`）。fe-lift の一式は追加テスト（`fle_jumptab_miss` のテスト）の前のソースで流して 1,051 件だったため、最終ソースで fe-decode 全体と fe-lift の `msvc_switch`（22 → 23 件。`test-ws-fe-lift-msvc_switch.txt`）を再実行して差し替え、合算した（fe-lift の他のバイナリは追加前後でソースが同じ）。

**変異検査**（実施。`lift/out-swapped/mut/`。別の target で実行し、ソースは `cmp` で元と一致を確認）:

| # | 変異 | 結果 |
|---|---|---|
| 1 | I <= N の側に `ja` 成立・`jae` 成立も許す | 1 件失敗（`swapped_the_wrong_side_of_the_guard_is_not_a_bound`） |
| 2 | 先行辺が 1 本の要件を「1 本以上」に | 1 件失敗（`…needs_exactly_one_predecessor`） |
| 3 | 基底証明の問い合わせ位置を load から jmp に | 12 件失敗（`add` の定義が届き、実物も合成も拒否される） |
| 4 | 隙間の命令の検査を外す | 1 件失敗 |
| 5 | `cmp` の後・load の前の I の書き込みの検査を外す | 1 件失敗 |
| 6 | `MSVC_SWAP_SKIP` を 3 に | 2 件失敗 |
| 7 | `landings_in_swapped_idioms` を無効化 | 2 件失敗 |
| 8 | `verify_msvc_swapped` の「途中にブロックの先頭」検査を外す | **最初は 0 件失敗**（7 が先に同じ着地を拒否するため。0093 と同じ重なり）→ `verify_msvc_swapped` を直接呼ぶテストを足して 1 件失敗 |

## 4. DLL ごとの統計の前後と見積もりとの比較（実測。`--pdata`。生成 C は `/dev/null`。ログ `lift/out-swapped/logs/stats.NAME.{before,after}.txt`、スクリプト `lift/out-swapped/stats.sh`）

「前」= `lift/out-git/target-m94/debug/fe-lift`（HEAD d618380 のビルド。読み取り専用）、「後」= `lift/out-swapped/target/debug/fe-lift`（この作業）。入力の SHA-256 先頭: Blend2D `24e01ec9…`、dsp_ippv2 `4fd25502…`、dsp の種 `cf664439…`、Blend2D の種 `ca6439b6…`、QFC `afc5a0ed…`、FLEngine `b2c64da4…`。

| 構成 | 前後 | closure_instructions | functions | bad_nodes | unsupported_total | register_jmp_sites | jumptabs | msvc_tables | msvc_rejected |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Blend2D 種なし | 前 | 405,045 | 2,019 | 0 | 596 | 99 | 30 | 30 | 49 |
| | 後 | 405,076 | 2,019 | 0 | 596 | 99 | **31** | **31** | 49 |
| Blend2D 種 58 個 | 前 | 405,674 | 2,067 | 0 | 599 | 100 | 30 | 30 | 50 |
| | 後 | 405,705 | 2,067 | 0 | 599 | 100 | **31** | **31** | 50 |
| dsp_ippv2 `--pdata` | 前 | 6,465,171 | 32,648 | 0 | 974,527 | 836 | 439 | 5 | 6 |
| | 後 | 6,465,202 | 32,648 | 0 | 974,527 | 836 | **440** | **6** | 6 |
| dsp_ippv2 縮小（種 452、`--pdata` なし） | 前 | 3,038,921 | 8,030 | 0 | 391,396 | 563 | 437 | 3 | 4 |
| | 後 | 3,038,952 | 8,030 | 0 | 391,396 | 563 | **438** | **4** | 4 |
| QFC | 前後 | 209,259 | 6,712 | 0 | 104 | 5 | 0 | 0 | 0 |
| FLEngine | 前後 | 2,528,030 | 59,297 | 0 | 525 | 20 | 4 | 0 | 0 |

- **bad_nodes はどこも 0 のまま**（増えなかった）。QFC と FLEngine は**全行が同一**（`diff` で差なし）。
- `msvc_rejected` は**件数も site の集合も理由の文字列も全 6 構成で前後同一**（スクリプトで site ごとに比較: 増減 0、理由の変化 0）。つまり既存の (c1)/(c2) の認識の結果は変わらず、F1 std・F2・F3 は従来どおり（認識されないか、同じ理由で拒否）。
- `closure_instructions` の +31 は、4 構成とも**新しく閉包に入った 17 エントリの先のコード 31 命令**（`f_489f7d0`／`f_37cd2c0` の表の移動先）。`functions` は変わらない（2 つの関数は閉包に既にあった）。
- **新しく解決された site**（前に無く後にある `msvc_tables`）: Blend2D **`0x489f8fb`**（種なし・種 58 個の両方）、dsp_ippv2 **`0x37cd3eb`**（`--pdata` と縮小の両方）。他に増減した site はない（生成 C の `switch` 行を `check_sites.py` で全件読んだ: Blend2D 31 site／289 エントリ（0094 の 272 + 17）、dsp 6 site／85 エントリ（0094 の 68 + 17））。

**調査の見積もり（§1。swapped 向きの件数）との比較**

| | 見積もり | 実装の結果 |
|---|---:|---:|
| Blend2D 解決される MSVC switch | 30 → 31（`0x489f8fb`） | 30 → **31**（`0x489f8fb`）。一致 |
| dsp_ippv2（`--pdata`） | 5 → 6（`0x37cd3eb`） | 5 → **6**（`0x37cd3eb`）。一致 |
| dsp_ippv2 縮小 | 3 → 4 | 3 → **4**。一致 |
| QFC、FLEngine | 0 件（変化なし） | 変化なし。一致 |
| F1 std（Blend2D 12、dsp 1）、F2（Blend2D 3、dsp 1） | 範囲外 | 解決されず、`msvc_rejected` にも出ない（テストで固定）。一致 |

調査 §5.3 の「(C) だけ: Blend2D +1、dsp +1」と同じ。差はない。調査 §7 が挙げた「`foreign_entries`／`landings_in_idioms` を通るか、bad_nodes が 0 のままか」は、この 2 site では**通った／0 のまま**（実測）。

**17 エントリの objdump での照合**（`lift/out-swapped/logs/objdump-{b2d,dsp}.txt`、`fn/{b2d,dsp}.bin`）: Blend2D `f_489f7d0` と dsp `f_37cd2c0` の表の 17 件は、objdump の逆アセンブルの**命令の先頭の行に 17/17**（互いに異なる 17、関数内 17）。r8 = 0 は `ret`、1 は `mov [rcx-1],dl`、…、11 は `0x489f940`／`0x37cd430`（`mov [rax],rdx`）、…、16 は `0x489f958`／`0x37cd448`。

## 5. FLEngine の生成 C と、解決した site の機械検査（実測）

- **FLEngine の生成 C の SHA-256**: `fe-lift lift ../binaries/win64/FLEngine_x64.dll out-swapped/fle.c --pdata` → **`feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f`**（基準と同じ。`lift/out-srw/fle.c` と SHA 一致）。**変わった関数 0**。functions 59,297、bad_nodes 0、unsupported_total 525、register_jmp_sites 20、jumptabs 4、msvc_tables 0、msvc_rejected 0（前後同一）。単独構成の a4（基準 145/942）は生成 C も runtime も同じなので**省略**。
- **解決した全 site の全エントリの機械検査**（`lift/out-swapped/check_sites.py`。fe-decode とは独立に、生成 C の `switch` の行を読み、PE の `.pdata` の連鎖ピースと capstone の線形走査と照合）: Blend2D **31 site／289 エントリ**、dsp_ippv2 **6 site／85 エントリ**。**全エントリが、関数のピース内、capstone の命令の先頭、case は互いに異なり `goto L_<同じ値>`、生成 C のラベルがファイル内にちょうど 1 つ、既定は `fle_jumptab_miss(<jmp の VA>, t)`**。問題のある site 0 件（`logs/check-b2d.txt`、`check-dsp.txt`）。新しい 2 site は `0x489f8fb`: `f_489f7d0` 17 case（`0x489f900`〜`0x489f958`）、`0x37cd3eb`: `f_37cd2c0` 17 case（`0x37cd3f0`〜`0x37cd448`）。

## 6. 3 モジュール構成の測定（測定のみ。gdb の仮の値なし、runtime の応答追加なし。新しい停止は直していない）

### 6.1 手順（実測）

- 生成: この作業の release の `fe-lift`（`lift/out-swapped/target/release`、決定 0095 の実装）。runtime と harness は HEAD のまま（`git log` で `lift/runtime`・`lift/harness` の最後の変更は決定 0092 のコミット 38df911。前回の 0094 の測定と同じ）。FLEngine の生成 C は `gen_two.sh` の `gen` 段階が**この作業の fe-lift で生成して SHA を検査**（`feb5b908…`、期待どおり）。FLEngine の C を分割した `src/fle-00.c`〜`fle-16.c` は `lift/out-rdefs/three/src/` のものと**全 17 ファイルが `cmp` で一致**、`gen/flengine.c` も一致。そのため FLEngine のオブジェクト（`native/fle/*.o` 17 個、505 MB）は `out-rdefs/three/native/fle` から `cp -a` で再利用した（同じ C・同じフラグ。gdb のスタックの行番号の元ファイル名が `out-srw/three/src/…` になっているのはこのオブジェクトの `-g` の情報）。runtime と harness のオブジェクトはこの作業のビルドで作り直した。
- 入力: リベース済みの DLL は `out-rdefs/three/dll/` のコピー（`lift/out-swapped/three/dll/`。SHA-256 先頭 `24e01ec9…`（Blend2D、基底 0x4700000）、`4fd25502…`（縮小 dsp_ippv2、基底 0x2000000）、種 `cf664439…`、`ca6439b6…`（Blend2D の種 58 個））。スクリプトは `lift/out-swapped/three/`（`run-gen.sh`、`run-build.sh`、`run-a4.sh`、`harness-two/` は `out-rdefs/three/harness-two/` の複写で、`gen_two.sh` が `EXPECT_FLE_SHA` を検査する）。
- 生成 C: 縮小 dsp_ippv2（`real.c`）は functions 8,030、closure **3,038,952**（前回 3,038,921）、bad_nodes 0。Blend2D（`blend2d.c`）は functions 2,067、closure **405,705**（前回 405,674）、bad_nodes 0。表の統合: 59,565 + 8,298 + 2,335 → **69,662 行**（前回と同じ。複数の表に在る行 268、`f_<VA>` の衝突 0）。two-native は 612,732,944 B（2 GB 未満）。ビルドの stderr 0 行。
- a4-survey を 2 回（`out-swapped/three/out-1/`、`out-2/`）。読み取りだけの gdb（`out-swapped/three/gdb/ro_srw3.py` = `out-rdefs/gdb/ro_srw3.py` の複写。`return`、`set var`、`call`、値の書き込みなし）を 2 回（`out-swapped/three/run/`）。

### 6.2 a4 の行（2 回。実測）

```
A4 survey 20261026T220828Z   dll: /home/ubuntu/work/signature-studio-develop/binaries/win64/FLEngine_x64.dll   harness exit: 134
static : units=942 runnable=895 empty=46 noproc=1 (header 0x13768b0, units 0x13768e0)
stop   : fle: LoadLibraryW: module "quickfontcache_x64.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)
         top of stack: 0x423990 f_423990
depth  : init_count=159 / 942   last unit: index=158 run_rank=146/895 proc=0x66d1a0 in_progress
         started=146 completed=145 in_progress=1 of 895 runnable   all_units_done=no
         entry check: started_but_not_entered=0 runnable_entered_beyond_init_count=0
funcs  : executed (distinct)=1106  entries=27842
```

- 2 回の出力は `logs` 行の OUT 名以外**同一**（stderr は両方 0 バイト）。
- **前回（決定 0094 の測定）からの変化**: init_count **150 → 159** / 942、last unit **149 → 158**（proc `0x664260` → `0x66d1a0`）、started 137 → 146、completed 136 → 145、functions_distinct 1,069 → **1,106**、function_entries 27,333 → **27,842**。**停止が `fle: indirect call to 0x489f940 is not a known function entry`（top of stack `f_489f7d0`）から、`fle: LoadLibraryW: module "quickfontcache_x64.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)`（top of stack `f_423990`）に動いた**。**unit 158（`quickfontcache_x64.dll` の `LoadLibraryW`）に届いたので、指示どおりここで止めた**（直していない）。
- **`0x489f940` は閉包のラベルになった**: 生成 C に `L_489f940: …` がちょうど 1 つ、`0x489f8fb` は 17 case の `switch`（`blend2d.c` の 392120 行目）。**停止は `0x489f940` から動いた**ので、「停止が動かなければ原因を調べる」の場合ではない。
- **C のスタック**（`out-1/a4/*-native.txt`、`call depth at stop: 10`）:

```
#0 0x423990 f_423990 guest-return=0x44db70
#1 0x44db40 f_44db40 guest-return=0x653a56
#2 0x6539e0 f_6539e0 guest-return=0x665bf2
#3 0x665bd0 f_665bd0 guest-return=0x66d066
#4 0x66d050 f_66d050 guest-return=0x66d1be
#5 0x66d1a0 f_66d1a0 guest-return=0x412129
#6 0x4120c0 f_4120c0 guest-return=0x41229a
#7 0x412160 f_412160 guest-return=0x41d989
#8 0x41d8f0 f_41d8f0 guest-return=0x1376875
#9 0x1376820 f_1376820 guest-return=0xdead0000
```

### 6.3 gdb（読み取りだけ。2 回。import のログは `cmp` で全バイト一致。`out-swapped/three/run/m3-ro{1,2}-imports.txt`）

- **Blend2D の DllMain(ATTACH) は戻ったか**: **戻った**。`call_dllmain idx=1 reason=1` の `DLLMAIN-RETURN reason=1: call_dllmain returned 0x1`（**eax = 1**）。`fle_realmod.c:144 reached (m->state = ST_LOADED: DllMain returned nonzero)`。前回は戻っていなかった。
- **LoadLibraryW が 0x4700000 を返したか**: **返した**。`LOADLIBRARY-RETURN fle_realmod_load returned to load_by_name: R[0]=0x4700000`（dsp_ippv2 のときの `R[0]=0x2000000` に続く 2 回目）。
- **ATTACH 内の import の列**: Blend2D の DllMain に入るまで `fle_imp_*` は 1,525 回（前回と同じ）。**ATTACH 内は 188 回（通し番号 1526〜1713。前回は停止まで 183 回、1526〜1708）**。**通し番号 1〜1708 は、前回の列と番号・名前・`GetProcAddress`／`LoadLibrary*` の名前引数が全件一致**（引数の生のポインタ値は実行ごとに違うので行全体は一致しない）。前回より後ろの 5 回: `HeapAlloc` ×2（1709、1710）、`HeapFree`（1711）、`EnterCriticalSection`（1712）、`LeaveCriticalSection`（1713）。名前ごとの回数（188 回）: `InitializeCriticalSectionAndSpinCount` 83、`GetProcAddress` 15、`HeapAlloc` 11、`EnterCriticalSection` 8、`LeaveCriticalSection` 8、`LoadLibraryExW` 6、`MultiByteToWideChar` 6、`WideCharToMultiByte` 6、`LCMapStringW` 4、`GetStdHandle` 3、`GetSystemInfo` 3、`TlsSetValue` 3、`GetCPInfo` 2、`HeapFree` 2、`QueryPerformanceCounter` 2、`TlsAlloc` 2、`TlsGetValue` 2、残りは 1 回ずつ（`AcquireSRWLockExclusive`、`ReleaseSRWLockExclusive`、`CreateEventW`、`FreeEnvironmentStringsW`、`GetACP`、`GetCommandLineA`、`GetCommandLineW`、`GetCurrentProcessId`、`GetCurrentThreadId`、`GetEnvironmentStringsW`、`GetLastError`、`GetModuleFileNameW`、`GetModuleHandleA`、`GetModuleHandleW`、`GetProcessHeap`、`GetStartupInfoW`、`GetStringTypeW`、`GetSystemTimeAsFileTime`、`InitializeCriticalSection`、`InitializeSListHead`、`IsValidCodePage`、`SetLastError`）。名前付き停止の import なし。
- **DllMain が戻った後、停止まで**（通し番号 1714〜1747、34 回）: `SetErrorMode`、`WideCharToMultiByte` ×2、**`GetProcAddress(0x4700000, "AggCreateInstance")`**、`RegCreateKeyExW`、`RegQueryInfoKeyW`、`GetCurrentThreadId` ×6、`RegCloseKey`、`GetSystemInfo`、`CompareStringW`、`GetFileAttributesW`／`GetLastError` の組、`GetModuleFileNameW`、`RegOpenKeyExW` ×6、`SetDllDirectoryW`、`SetErrorMode`、**`LoadLibraryW("QuickFontCache_x64.dll")`（1747。停止）**。
- 停止時の gdb の C バックトレースは上の a4 のスタックと同じ関数の列（`#5 fle_stop`、`#6 load_by_name`、`#7 fle_imp_LoadLibraryW`、`#8 f_423990` …）。

### 6.4 停止点の分類

**(i) 未応答の import**: `LoadLibraryW` が、仮想モジュールでも「欠如を定義した名前」でもない外部モジュール名 `quickfontcache_x64.dll`（実体は `lift/out-qfc/quickfontcache_x64.dll`）で名前付き停止する（runtime のメッセージが `external modules are M2` と言う）。**これは unit 158 そのもの**（指示の終点）。(ii) TLS、(iii) JIT、(iv) ロケール、(v) 生成器が解決しない `jmp reg` のいずれでもない。**この先は 3 つ目の実モジュール（QFC）を 3 モジュール構成に加えるかどうかの話**で、この測定の範囲外（直していない）。

### 6.5 `0x489f8fb` の switch が実行されたか（読み）

停止が `f_489f7d0` の `jmp r9` から `unit 158` まで 9 unit 動いたので、`jmp r9` を含む `f_489f7d0` は少なくとも 1 回、switch の case に入って戻った（r8 = 11 のとき `0x489f940`）。switch のヒット回数は数えていない（**未確認**）。

## 7. 解釈が分かれた点、実装できなかった点、限界

**実装できなかった項目は無い**。決定の内容は変えていない。PM が見るべき解釈の点（決定の番号は 0095 の表）:

1. **決定 1 の「挟まる命令の上限」**: 新しい定数 `MSVC_SWAP_SKIP = 2` を、**`add B,rX` の前後の隙間の合計**に対する上限とした（`MSVC_SKIP` と同じく合計。隙間ごとに 2 とは読んでいない）。今回の形は `add` の後に 2 命令なので**ちょうど上限**。上限は緩めていない（緩める必要は出なかった）。隙間ごとの上限（前 2＋後 2）という読みもありうるが、狭い読みを選んだ。
2. **決定 1 の「B、X、I を読み書きせず、フラグを使わず」**: 次のように読んだ。(a) **`add` の前の隙間**は B・X・I のどれにも触れない（読みも書きも不可）。(b) **`add` の後の隙間は B にだけ触れない**（X を書いてよい: `mov rax,r11`。I を読んでよい: `add rcx,r8` は I = r8 を読む。決定の本文が「`mov rax,r11` は X を書くので `add` より後ろなら問題ない」と書いており、実物の隙間がこの 2 命令だから）。(c) 「フラグを使わず」は**フラグを読まない**（`rflags_read() == 0`）と読んだ。実物の `add rcx,r8` はフラグを**書く**ので、書くことまで禁じると実物が通らない。(d) メモリを書かない、制御を変えない（`FlowControl::Next`）。**X ≠ I**（決定の文）も要求する。
3. **決定 2 の「唯一の先行ブロック」の判定の形**: 「ブロック」を線形走査の CFG で次のように定義した。表引きのブロック = 直前の命令が真っ直ぐ落ちてきて（`Next`）、その命令が直接の jmp／jcc／call の行き先でない間のかたまり。**直前が分岐先でない詰め物 `nop` は、ブロックの外の前置きとして飛ばす**（`jmp` の後の `nop` は先行辺にならない。実物 `0x489f8d4` はこれで先行辺が `jbe` の 1 本だけ）。**直接の call の行き先も入り口として数える**（保守側）。先行辺はちょうど 1 本で、それが直接の条件分岐でなければ `N not derived`。**`cmp` と分岐は同じ基本ブロックの中**（間に分岐の行き先がない）で、`cmp` の後ろにフラグを書く命令がなく I を書く命令もない、とした（決定の (b)。分岐と `cmp` の間にフラグも I も触らない命令があるのは許す）。(c) は「表引きのブロックの先頭から load まで」。関数の入口を含むブロックは拒否。
4. **決定 4 の `add B,rX` の扱い**: 到達定義の規則を緩める代わりに、**基底証明の問い合わせ位置を jmp から表引き（load）に移した**（swapped のとき）。load に届く B の定義がすべて `lea B,[ImageBase]` なら、load〜jmp の間で B を書くのは `add B,rX` だけ（隙間は B に触れない）で、途中に入る道もない（`landings_in_swapped_idioms`、`verify_msvc_swapped`）ので、jmp に届く定義は「`lea` の値に `add` したもの」だけ。決定の言う「`add` を idiom の一部として許す」と同じ内容を、`add` を定義として数えずに実現した。その結果、`lea` がガードの前にあり、表の移動先がガードへ戻る形（`add` の定義が jmp の表を通って load に戻る）は拒否になる（テストで固定。実物 2 site は `lea` が表引きのブロックの先頭なので影響しない）。0094 の規則そのものは変えていない（`prove_base_reaching_defs` は問い合わせ位置が増えただけで、`site_va == jmp_va` の standard の結果は同一）。
5. **swapped を比較専用の方式で試さない**: `--msvc-base-dominating`／`--msvc-base-entry-block` では swapped の site は認識も報告もされない（決定 0095 は 0094 の到達定義の上の追加なので）。決定の文には無い選択。
6. **32 ビットの `cmp ecx,imm` と 64 ビットの索引**: standard（0089）と同じく `cmp` の 32／64 ビットを許す。索引の上位 32 ビットが 0 でないとき、native は表の外を読むが、生成 C は jmp レジスタの値で `switch` するので、native と同じ値なら同じ所へ、どの case でもなければ `fle_jumptab_miss` で止まる（安全側。standard と同じ性質）。

**限界**

- **線形走査の CFG**: 0090／0094 と同じ。関数のピース内でデコードできる命令だけで先行辺を数える。**未解決の間接 jmp が実行時にこのブロックへ着地する場合は見えない**（解決済みの間接 jmp と表の移動先は閉包の `landings_in_swapped_idioms` が見る）。SEH、例外による遷移は見ない。
- **この認識は swapped の 1 つの形だけ**: F1 std（Blend2D 12、dsp 1）、F2（Blend2D 3、dsp 1）、F3（Blend2D 40、dsp 6）、F4、F5 は解決しない（認識もされないか、従来の理由で拒否）。N ≥ 4096 は拒否。
- **native と生成 C の突き合わせ**は swapped の 2 site（`0x489f8fb`、`0x37cd3eb`）と合成 PE の 6 通りの形。表の外の索引（17 以上）は、実物では `jbe` が落ちる先で止まる（`switch` に入らない）。**`fle_jumptab_miss` に届く入力**は、合成 PE でガードを飛ばして表引きのブロックから入る形（索引 17、18、0x100）で名前付き停止になることを確認した（索引 0..=16 は表どおりに戻る対照つき）。**実物の DLL ではこの入り方（ガードを飛ばす）は実施していない**。`f_489f7d0` の他の経路（0x11 バイト以上の埋め込み、非一時書き込み）は動かしていない。memset 風という読みは調査のままで、実行して確かめていない。
- **3 モジュールの測定**は 1 構成（FLEngine + 縮小 dsp_ippv2 基底 0x2000000 + Blend2D 基底 0x4700000、種 58 個。いずれも測定用の値）。単独構成の a4、wasm、実 Windows は実施していない。switch のヒット回数は数えていない。
- 母集団の検査は FLEngine、dsp_ippv2、Blend2D、QFC の 4 つ。同梱の他の DLL は未実施。
- callee-saved の B が call をまたいで保たれる仮定（Win64。0089 から）は変わらない。`.pdata` を持たない image と `.pdata` の外の `jmp reg` では何も解決しない。

## 8. 作ったファイル

- 新規（追跡対象）: `docs/decisions/0095-msvc-switch-swapped-form.md`、`docs/investigations/blend2d-switch-swapped.md`（この文書）。
- 編集（追跡対象）: `docs/README.md`（0095 の 1 行）、`lift/crates/fe-decode/src/jumptab.rs`、`lift/crates/fe-decode/src/lib.rs`、`lift/crates/fe-decode/tests/msvc_common/mod.rs`（`build_swapped` と型）、`lift/crates/fe-decode/tests/msvc_switch.rs`、`lift/crates/fe-decode/tests/jt_cpu.rs`（期待値）、`lift/crates/fe-lift/tests/msvc_switch.rs`、`lift/crates/fe-lift/tests/jt_cpu.rs`（期待値）。
- 新規（`.gitignore` の `lift/out*/` の作業物 `lift/out-swapped/`）: `logs/`（統計の前後、`check-*.txt`、`objdump-*.txt`、`facts-*.txt`、`flag-*.txt`）、`stats.sh`、`check_sites.py`、`facts.py`、`mut/`（変異検査のログと `jumptab.rs.good`、`lib.rs.good`）、`fn/`（objdump 用の関数のバイト列）、`b2d0.c`／`dsp.c`／`fle.c`（生成 C。dsp.c は 537 MB）、`test-ws-*.txt`（パッケージごとのテストの出力）、`three/`（3 モジュール: `run-gen.sh`、`run-build.sh`、`run-a4.sh`、`harness-two/`、`dll/`、`seeds/`、`gdb/`、`v-b2d58/`、`native/`、`out-1/`、`out-2/`、`run/`、`survey-*.txt`）、`target/`、`target-mut/`、`tmp/`。
- **追跡ファイルへの副作用なし**: `lift/runtime`、`lift/harness`、他の docs は編集していない。未追跡の `lift/harness/wasm/trace.c.new`、`trace_native.c.new`、`lift/iat-unsup-closure.txt` は触っていない。
