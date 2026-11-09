# 0095: MSVC switch 表の認識に、「基底レジスタに表の値を足して jmp する向き」（swapped）と、成立側の先行分岐から N を導く規則を加える

状態: **決定（仮ではない）**。PM の決定（2026-10-25）を、作業者（サブエージェント）が記録した。実行ではなく**バイト列と実行で検証できる**ので仮の決定にしない。決定の節、背景、範囲、範囲外は実装の前に保存し、「実装した場所」「テスト」「検証」「解釈が分かれた点」「限界」は実装と検証の後に書く。

記入状況: すべて記入済み（決定の節・背景・範囲・範囲外は実装前に保存。実装した場所・テスト・検証・解釈が分かれた点・限界は実装と検証の後）。

事実の出典（節番号は各文書のもの）:

- `docs/investigations/msvc-switch-other-forms.md`（コミット 0ddcbf1。調査のみ。以下「調査」）§1（形の分類 F0〜F5 と件数。F1 の内訳は §1.3）、§2（代表例）、§3（停止点 `0x489f8fb`、`f_489f7d0`。命令列 §3.1、表 §3.2、ガード §3.3、表の長さ §3.4、移動先 §3.5、件数 §3.6）、§5（選択肢 A〜C）、§6（決定点）、§7（限界）。作業物は `lift/out-forms/`（読み取り専用。`forms-scan` の分類は `res/NAME.sites.tsv`）。
- `docs/decisions/0089-msvc-switch-tables-with-proven-base.md`（コミット d87e0ad）1(a)〜1(d)、`docs/decisions/0090-msvc-switch-base-dominance.md`、`docs/decisions/0093-msvc-switch-nop-padding.md`（`MSVC_SKIP`、`is_padding_nop`、`MSVC_IDIOM_MAX`）、`docs/decisions/0094-msvc-switch-base-reaching-defs.md`（到達定義。`prove_base_reaching_defs`）。
- `lift/crates/fe-decode/src/jumptab.rs` の `recognize_msvc`、`table_load`、`byte_load`、`MsvcForm`、`read_msvc_targets`、`resolve_msvc`、`verify_msvc`、`foreign_entries`、`landings_in_idioms`、`prove_base_reaching_defs`（HEAD 0ddcbf1。コードは d618380 のまま）。

## 決定（PM。内容を変えない）

| # | 項目 | 決定 |
|---|---|---|
| 1 | 認識の追加（swapped） | **`jmp rT` の直前に近い命令列として次を認める。** (1) **基底 B は `lea B,[rip+d]`**（rip+d = ImageBase）で関数内で決められる（基底証明は 0094 の到達定義のまま）。**B は jmp するレジスタ rT と同じ**（`rT == B`）。(2) 表の値の取り方は `mov eX,[B + I*4 + T2]`（(c1) と同じ 32 ビットの RVA 表。B はベース、I は索引、T2 は変位）。**X は B と I のどちらとも別のレジスタ。** (3) 足し込みは `add B,rX`（rX は 64 ビットの別名。eax → rax）。**この `add B,rX` で B が基底から移動先になる。** `add B,rX` と `mov eX,[B+I*4+T2]` の間、`add B,rX` と `jmp B` の間に挟まってよい命令は、**調査 §3.1 の形に合わせた「B、X、I を読み書きせず、フラグを使わず、メモリを書かず、制御を変えない」命令だけ**（今回の形では `add rcx,r8`、`mov rax,r11` の 2 命令。`mov rax,r11` は X = rax を書くので、`add B,rX` より**後ろ**にあるなら rX の値は使い切った後で問題ないが、**`add B,rX` の前に X を書く命令は許さない**）。挟まる命令数の上限は決定 0093 の `MSVC_SKIP`（2）の流儀で**別の定数**にする（今回の形は間に 2 命令）。**上限を緩める必要が出たら緩めずに報告**（PM が判断する）。(4) **他の表の値の取り方（`movsxd`、`mov eX,[rY+I*4]`（基底でない）、(c2) の索引バイト表との組合せ）は扱わない**（調査 F2 の 4 件を誤って通さない。**F2 の site は拒否されることをテストで固定する**） |
| 2 | N の導出の拡張（成立側のガード） | swapped では N を、**jmp に至る経路の先頭の条件分岐**から導く。索引レジスタ I について、**表引き `mov eX,[B+I*4+T2]` に至る全経路が、`cmp I,imm`（imm = N）の後の「I <= N の側」（`ja` の不成立側、`jbe` の成立側、`jae`／`jb` の対応する側。厳密な不等号の解釈（`jb imm` なら N = imm−1）は iced の `ConditionCode` で書く）を通る**こと。ただし**関数内の単純な形に限る**: (a) 表引きの基本ブロックの**唯一の先行ブロック**が、その `cmp I,imm` を末尾とする条件分岐（`jbe`／`ja` など）の I <= N 側の辺であること、(b) その `cmp` と分岐の間に I を書く命令がない、(c) `cmp` の後（分岐から表引きまで）に I を書く命令がない。**この条件で N が導けなければ拒否する（`N not derived`）**。「唯一の先行ブロック」の判定は、`.pdata` の関数のピース内の線形走査の CFG で行う（`verify_msvc` が同様の走査をしている流儀。表引きの先頭への分岐が他にあれば拒否）。調査 §3.3 のとおり、`f_489f7d0` では `cmp r8,0x10; jbe 0x489f8e0` で、`0x489f8e0` の先行ブロックはその `jbe` の成立辺だけ（実装時に確認する） |
| 3 | 移動先の検査 | 0089 1(d) のとおり（同じ `.pdata` の RUNTIME_FUNCTION 内、命令の先頭、bad_nodes 0）。表の長さは N+1 = 17。N が 32 を超える等の上限は既存の `MSVC_MAX_ENTRIES`（4096）のまま。**表の外の索引（N を超える）は `fle_jumptab_miss`**（既存の生成 C の switch の既定のまま。`Select::Target`） |
| 4 | 基底証明 | **0094（到達定義）のまま。** B = jmp するレジスタ rT なので、**B の定義には `add B,rX` も入る**。到達定義の判定で「jmp に届く B の定義が `lea` だけ」とは別に、**`add B,rX` は swapped の idiom の一部として許す**（その定義は `jmp` の**直前**の idiom 内にあり、`lea` の定義と idiom 内の `add` 以外の定義が届かないことを要求する。実装の形は作業者が決めてよい）。**到達定義の規則そのものは緩めない**: `lea` の後に別の経路で B を書き換える命令があり、それが jmp に届く場合は従来どおり拒否する。決定 0090／0094 の本文は書き換えない |
| 5 | 範囲外（変えない） | (c1)／(c2) の既存の認識、0093 の nop、`verify_msvc`、`foreign_entries`、`landings_in_idioms` の趣旨、N の導出が `cmp idx,imm; ja`（直前の `ja`）である既存の形。`jmp reg` の他の形（F2、F3 の `shr` で作る添字、F4 の関数ポインタ、F5 の dsp のオフセット表）は扱わない |
| 6 | FLEngine への影響 | FLEngine の生成 C の SHA-256 は **`feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f` のまま**が期待（FLEngine は F1 に当たる site が 0 件）。変わったら、変わった関数の一覧と原因を報告（隠さない） |
| 7 | 状態・種 | **仮の決定ではない**（バイト列と実行で検証できる）。種 58 個・基底 0x4700000・0x2000000 は測定用で、この実装はそれらに依存しない（種なしの Blend2D でも解決される） |

## 背景（事実。調査の節番号）

- 3 モジュール構成（FLEngine + 縮小 dsp_ippv2 基底 0x2000000 + Blend2D 基底 0x4700000。測定用の値。種 58 個）の停止は `fle: indirect call to 0x489f940 is not a known function entry`（top of stack `f_489f7d0`。調査 §3、0094 の検証欄）。`f_489f7d0` は memset 風の関数（読み。実行して確かめていない。調査 §4.1）で、`0x489f8fb: jmp r9`（元の VA `0x18019f8fb`）が停止点。
- 命令列（調査 §3.1。objdump、capstone、iced で確認済み）: `cmp r8,0x10`（`0x489f7e4`）、`jbe 0x489f8e0`（`0x489f7e8`。成立側が表引きへ）、`0x489f8e0: mov rdx,r9`、`lea r9,[rip-0x19f8ea]`（r9 = 0x4700000 = ImageBase）、`mov eax,[r9+r8*4+0x2fd000]`、`add r9,rax`（基底のレジスタ自身に足す）、`add rcx,r8`、`mov rax,r11`、`jmp r9`。
- 表は RVA `0x2fd000`（`_RDATA`）、32 ビットの RVA 配列、**17 エントリ（N = 0x10）**。後ろは 0 の dword 3 つで、次の表（`0x2fd050`）との距離では長さが導けない（調査 §3.4）。17 エントリの移動先は**すべて同じ関数 `[0x489f7d0,0x489f960)` 内の命令の先頭**（objdump 17/17、capstone 17/17、iced 17/17。調査 §3.5）。
- ガードは idiom の直前の `ja` ではなく、**別のブロックの `cmp r8,0x10; jbe`（成立側）**。`0x489f8e0` へ入る直接分岐は `jbe` の 1 本だけで、直前 `0x489f8d4` は `jmp short 0x489f870` の後の padding `nop`（調査 §3.3）。
- 現在の `recognize_msvc` は `add` が `jmp` の直前でないと `Ok(None)` を返すので、この site は `msvc_rejected` にも出ない（調査 §3.3）。
- この形（swapped）は Blend2D に 1 件（`0x489f8fb`）、dsp_ippv2 に 1 件（`0x37cd3eb`。基底 0x2000000。`f_37cd2c0`。表 `0x26150b0`、N = 0x10）。**QFC 0 件、FLEngine 0 件**（調査 §1.2、§1.3、§3.6）。F1 の std 向き（load／`add`／`jmp` の間に別の命令が挟まる）は別の形で、この決定の範囲外（調査 §1.3、§5）。
- F2（表の値の取り方が正準でない。Blend2D 3、dsp 1）は `mov ecx,[rcx+rax*4+T]; lea rax,[ImageBase]; add rcx,rax; jmp rcx` など、**load の基底と `add` の相手が別のレジスタ**の形で、この決定の認識で誤って通してはならない（調査 §1.4）。

## 範囲（今回扱う）

- `recognize_msvc` の swapped の認識（`jmp B` ← `add B,rX` ← `mov eX,[B+I*4+T2]` の遡りと、挟まる命令の検査）と、その上限の定数。
- N の導出（成立側。`cmp I,imm` と条件分岐、表引きのブロックの唯一の先行辺）。`.pdata` の関数のピースの線形走査で行う。
- 基底証明（0094）の swapped への適用: B が基底であることを「表引き」の位置で到達定義により証明し、jmp では `lea` の定義と idiom 内の `add` だけが届くことを求める。
- `verify_msvc` に相当する swapped の検査、`landings_in_idioms` に渡す範囲。

## 範囲外（今回扱わない。従来どおり）

- (c1)／(c2) の既存の認識とその結果、決定 0093 の nop、`verify_msvc`（standard の形）、`foreign_entries`、`landings_in_idioms` の趣旨、N の導出が `cmp idx,imm; ja`（直前の `ja`）である既存の形。
- `jmp reg` の他の形: F2（表の値の取り方が正準でない）、F3（`shr` などで作る添字）、F4（関数ポインタ）、F5（dsp の 16／8 ビットのオフセット表）、F1 の std 向き（load／`add`／`jmp` の間に別の命令が挟まる形）。
- `movsxd`、基底でないレジスタの `mov eX,[rY+I*4]`、(c2) の索引バイト表と swapped の組合せ。
- 決定 0074、0079、0080、0086、0089、0090、0093、0094 の他の項目。種 58 個は決定にしない。

## 実装した場所

詳細は `docs/investigations/blend2d-switch-swapped.md` §2。

- `lift/crates/fe-decode/src/jumptab.rs`（+460 −15）:
  - `MsvcForm` に `swapped: Option<SwapForm>`（`SwapForm { load, add, index }`）。swapped のとき `n`・`from`・`guard_ja`・`guard_cmp` は 0 のプレースホルダで、`resolve_msvc` が CFG から求める。`Origin::Msvc` に `swapped: Option<u64>`（`Some(lo)`: ガードの分岐だけが入ってよい run の先頭。表引きのブロックの前の詰め物 `nop` を含む）。
  - `recognize_msvc` = `recognize_msvc_standard`（0089／0093 のコードそのまま）→ `Ok(None)` のときだけ `recognize_msvc_swapped`。閉包（`lib.rs`）は両者を直接呼び、**基底証明が `ReachingDefs`（既定）のときだけ** swapped を試す。
  - `recognize_msvc_swapped`: `jmp B` ← `add B,rX`（rX は 64 ビット、≠ B）← `mov eX,[B+I*4+T2]`（`table_load`）、X ≠ I。隙間の命令は `swap_gap_ok`（制御を変えない、フラグを読まない、メモリを書かない、`add` の前は B・X・I に、後は B に触れない）、上限は新しい定数 `MSVC_SWAP_SKIP = 2`（両方の隙間の合計。`MSVC_SKIP`・`MSVC_IDIOM_MAX` は変えていない）。
  - `derive_swapped_guard`（N の導出）: 表引きのブロックの先行辺がちょうど 1 本の直接の条件分岐で、辺の側が符号なしの I <= N（`ConditionCode` の `be`／`b`。`ja` 不成立、`jbe` 成立、`jae` 不成立、`jb` 成立。`b` なら N = imm − 1）、同じ基本ブロックの `cmp I,imm` がその分岐のフラグを最後に書き、`cmp` の後と表引きまでに I を書く命令がない。導けなければ `N not derived (swapped form, decision 0095)`。
  - `prove_base_reaching_defs` に問い合わせ位置 `site_va` を足した（standard は jmp、swapped は表引き）。到達定義の規則は 0094 のまま。
  - `verify_msvc_swapped`、`landings_in_swapped_idioms`（swapped の表の事後検査。standard の `verify_msvc`／`landings_in_idioms` は変えていない）。
- `lift/crates/fe-decode/src/lib.rs`（+31 −9）: 閉包での swapped の認識、swapped の表を専用の 2 検査へ回す。コメント 1 行。
- 変えていないもの: standard の認識の結果、決定 0093 の nop、`verify_msvc`、`foreign_entries`、`landings_in_idioms`、`read_msvc_targets`、`lift/crates/fe-lift/src/`（生成 C の出し方は `Select::Target` のまま）、`lift/runtime`。0089／0090／0093／0094 の本文は書き換えていない。
- **0089 の「範囲外」（基底が別の方法で得られる形）にあたる箇所への注記**: 0089 の 2'（(c1)／(c2) 以外は扱わない）は、(c1) の表の値の取り方（`mov eT,[B+I*4+T2]`）を使い**`add` の向きだけが違う**この形を、PM の決定 0095 で扱うことにした。0089 の本文は書き換えない。

## テスト

詳細は `docs/investigations/blend2d-switch-swapped.md` §3。`fe-decode/tests/msvc_switch.rs` 57 → 78 件（+21）、`fe-lift/tests/msvc_switch.rs` 18 → 23 件（+5）。合成 PE は `msvc_common/mod.rs` に `build_swapped`（`Swp`）を追加。

- **解決される（合成 PE）**: `f_489f7d0` と同形（`cmp rcx,16; jbe tbl`、12 バイトの `nop`、`lea r9`、`mov eax,[r9+rcx*4+T]; add r9,rax; add rdx,rcx; mov rax,r11; jmp r9`、17 エントリ）。N の導出の 5 つの側（`ja` 不成立、`jbe` 成立、`jae` 不成立、`jb` 成立、32 ビット `cmp`）。隙間（`add` の前後、合計が上限ちょうど）、B・X の別のレジスタ、`lea` が入口にある形。
- **解決されない（合成 PE）**: 不成立側から入る 4 通り、`cmp` が別のレジスタ／`test`／`cmp I,reg`、`cmp` と分岐の間・分岐と load の間の I の書き込み、先行辺が 2 本以上（前に別の分岐、`def` が落ちてくる、移動先が詰め物・load・`add` へ戻る）、**F2・`movsxd`・load の基底が jmp レジスタでない・`add`／`jmp` が別のレジスタ・X が索引・(c2) との組合せ（認識されない）**、`add` の前に X／I を書く・I を読む・フラグを読む・メモリを書く、`add` の後に B に触れる・フラグを読む・メモリを書く、命令数が上限を超える、基底証明の拒否（`lea` なし、ImageBase でない `lea`、`lea` と `mov` が両方届く、非 `lea` の定義が他の間接 jmp に届く、caller-saved の B と call、`add` の定義が表を通って load に戻る）、移動先の拒否（別関数、実行不可、命令の先頭でない、`.pdata` なし、別関数からの飛び込み）。比較専用の方式では認識も報告もされない。
- **実 DLL**: Blend2D `0x489f8fb` と dsp_ippv2 `0x37cd3eb` が解決され、**17 エントリをファイルの表から読んで objdump／capstone／iced の全件と照合**（命令の先頭、関数内、全部異なる）。**native（本物の命令）と生成 C を r8 = 0..16 と範囲外 8 通り（17、18、31、32、0x100、0xffffffff、0x100000000、`u64::MAX`）で動かし、到達先が表どおり**（2 DLL × 2 側 × 25 通り）。合成 PE の swapped 形 6 通りを全索引で native と生成 C に動かして表どおり。ガードを飛ばして表引きから入る形で、索引 N 超は `fle_jumptab_miss`。
- **変異検査**（実施。ソースは `cmp` で復元を確認）: 8 通り（N の側の判定、先行辺の数、基底証明の問い合わせ位置、隙間の検査、I の書き込みの検査、上限、`landings_in_swapped_idioms`、`verify_msvc_swapped`）がすべて 1 件以上の失敗で検出される。8 つ目は最初は検出されず（`landings_in_swapped_idioms` が同じ着地を先に拒否するため。0093 と同じ重なり）、`verify_msvc_swapped` を直接呼ぶテストを足して検出されるようにした。
- **既存の期待値を変えた行**（理由: 解決される site が増えた）: `fe-decode/tests/msvc_switch.rs` の `blend2d_the_reaching_definitions_of_decision_0094_…`（30 → 31）、`dsp_ippv2_f_37cc7e0_…`（dsp の MSVC switch の集合に `0x37cd3eb`）、`MsvcForm` の期待に `swapped: None`。`fe-decode/tests/jt_cpu.rs` の `dsp_ippv2_has_434_stubs`（`msvc_tables` 5 → 6、`jumptabs` 439 → 440、`jmp:unresolved` 544 → 543、`jmp:msvc_table` 5 → 6）。`fe-lift/tests/jt_cpu.rs`（`(jumptabs, msvc_tables)` 437, 3 → 438, 4）。`fe-lift/tests/msvc_switch.rs` の `real_blend2d_site_is_resolved_by_default`（30 → 31）。16 エントリ用の駆動 `F16` は表の数と範囲外の値を持つ形に一般化（16 エントリの 2 DLL の値は変えていない）。

## 検証

実測値と手順は `docs/investigations/blend2d-switch-swapped.md`。作業物は `lift/out-swapped/`。

| 項目 | 結果 |
|---|---|
| `cargo test`（`CARGO_TARGET_DIR=lift/out-swapped/target`、`FLENGINE_PE` 指定。パッケージごとに実行） | **98 binaries / 1,254 passed / 0 failed / 0 ignored**（基準 98 / 1,228。**+26 件** = `fe-decode/tests/msvc_switch.rs` +21、`fe-lift/tests/msvc_switch.rs` +5。SKIPPED 0 件）。内訳: fe-pe 10 binaries 83、fe-decode 12 binaries 119、fe-lift 76 binaries 1,052。`cargo test --workspace` は他の作業で負荷が高く 1 時間の枠で打ち切られたため、パッケージごとに実行した（fe-lift の一式は追加テストの前のソースで 1,051 件、`msvc_switch` のバイナリ 22 → 23 件と fe-decode 全体は最終ソースで再実行して合算） |
| **FLEngine の生成 C の SHA-256** | **`feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f` のまま**（`lift/out-srw/fle.c` と SHA 一致）。変わった関数 0。functions 59,297、bad_nodes 0、unsupported_total 525、msvc_tables 0、msvc_rejected 0（前後同一）。単独構成の a4 は生成 C も runtime も変わらないので**省略** |
| DLL ごとの統計の前後（HEAD d618380 のビルド → この実装。`--pdata`） | Blend2D 種なし: closure 405,045 → 405,076、functions 2,019、jumptabs／msvc_tables **30 → 31**、msvc_rejected 49、bad_nodes 0、unsupported_total 596。種 58 個あり: closure 405,674 → 405,705、functions 2,067、jumptabs／msvc_tables **30 → 31**、msvc_rejected 50、bad_nodes 0。dsp_ippv2: closure 6,465,171 → 6,465,202、jumptabs 439 → 440、msvc_tables **5 → 6**、msvc_rejected 6、bad_nodes 0。dsp_ippv2 縮小: closure 3,038,921 → 3,038,952、jumptabs 437 → 438、msvc_tables **3 → 4**、msvc_rejected 4、bad_nodes 0。QFC、FLEngine: **全項目変化なし**。**bad_nodes はどこも増えなかった**。`msvc_rejected` は 6 構成すべてで件数・site・理由が前後同一。比較専用の `--msvc-base-dominating`（Blend2D・dsp_ippv2）と `--msvc-base-entry-block`（Blend2D）は HEAD と統計の全行が同一 |
| 調査（`msvc-switch-other-forms.md`）の見積もりとの比較 | swapped 向きは Blend2D 1 件（`0x489f8fb`）、dsp 1 件（`0x37cd3eb`）、QFC・FLEngine 0 件。見積もり（Blend2D 30 → 31、dsp 5 → 6）と**一致**。F1 std（Blend2D 12、dsp 1）と F2（Blend2D 3、dsp 1）は解決されない（見積もりどおり） |
| 新しく解決された site | Blend2D `0x489f8fb`（`f_489f7d0`）、dsp_ippv2 `0x37cd3eb`（`f_37cd2c0`）。種なしと種 58 個ありで同一 |
| 解決した全 site の全エントリの機械検査 | Blend2D 31 site（289 エントリ）、dsp_ippv2 6 site（85 エントリ）の**全エントリが、関数のピース内、capstone の命令の先頭、case は互いに異なり、生成 C のラベルがちょうど 1 つ**。失敗 0 件。`f_489f7d0`／`f_37cd2c0` の 17 エントリは objdump でも 17/17 が命令の先頭 |
| native と生成 C の突き合わせ（`0x489f8fb`、`0x37cd3eb`） | r8 = 0..16 と範囲外 8 通りで、到達先がファイルの RVA 表どおり（2 DLL × 2 側 × 25 通り） |
| 3 モジュール（種 58 個あり、基底 0x4700000。測定のみ。runtime は HEAD のまま） | a4: init_count **150/942 → 159/942**、last unit 149 → **158**（proc `0x66d1a0`）、functions_distinct 1,069 → 1,106、function_entries 27,333 → 27,842。2 回同一。**停止は `fle: indirect call to 0x489f940 is not a known function entry`（`f_489f7d0`）から `fle: LoadLibraryW: module "quickfontcache_x64.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; …)`（top of stack `f_423990`）へ動いた**。Blend2D の DllMain は**戻った**（eax = 1）、LoadLibraryW は **0x4700000 を返した**。ATTACH 内の import は 188 回（1526〜1713。前回の 1〜1708 は番号・名前が全件一致）。`0x489f940` は閉包のラベルになった。**unit 158（`quickfontcache_x64.dll` の `LoadLibraryW`）に届いたので止めた**。停止点の分類は (i) 未応答の import（外部モジュール名の LoadLibraryW） |

## 解釈が分かれた点

詳細は調査 `docs/investigations/blend2d-switch-swapped.md` §7。**決定の内容は変えていない。実装できなかった項目は無い**。

1. **決定 1 の挟まる命令の上限**: `MSVC_SWAP_SKIP = 2` を `add B,rX` の**前後の隙間の合計**に対する上限とした（隙間ごとに 2 とは読まない）。今回の形は `add` の後に 2 命令でちょうど上限。緩めていない（緩める必要は出なかった）。
2. **決定 1 の「B、X、I を読み書きせず、フラグを使わず」**: `add` の前の隙間は B・X・I のどれにも触れない。`add` の後の隙間は B にだけ触れない（X を書く `mov rax,r11` と I を読む `add rcx,r8` が実物の形だから）。「フラグを使わず」は**フラグを読まない**と読んだ（`add rcx,r8` はフラグを書く）。X ≠ I も要求。
3. **決定 2 の「唯一の先行ブロック」**: 線形走査の CFG で、表引きのブロック（直前の命令が真っ直ぐ落ちてきて、直接の jmp／jcc／call の行き先でない間のかたまり）の先行辺を数える。直前が分岐先でない詰め物 `nop` は飛ばす（`jmp` の後の `nop` は辺にならない）。直接の call の行き先も入り口として数える（保守側）。ちょうど 1 本で、直接の条件分岐でなければ `N not derived`。`cmp` と分岐は同じ基本ブロック内。
4. **決定 4 の `add B,rX`**: 到達定義の規則を緩めず、基底証明の問い合わせ位置を jmp から表引き（load）に移した。load に届く B の定義がすべて `lea B,[ImageBase]` で、隙間は B に触れず、idiom に途中から入る道がないので、jmp に届く定義は「`lea` の値に `add` したもの」だけになる。`lea` がガードの前にあり表の移動先がガードへ戻る形（`add` の定義が load に戻る）は拒否される。
5. swapped は基底証明が `ReachingDefs`（既定）のときだけ試す。比較専用の `Dominating`／`EntryBlock` では従来どおり認識も報告もされない。

## 限界

- **線形走査の CFG**: 0090／0094 と同じ。未解決の間接 jmp が実行時にこのブロックへ着地する場合は見えない（解決済みの間接 jmp と表の移動先は事後の検査が見る）。SEH、例外による遷移は見ない。
- **swapped の 1 つの形だけ**: F1 std、F2、F3（`shr` などの添字）、F4、F5 は解決しない。N ≥ 4096 は拒否。
- 実 DLL の native と生成 C の突き合わせは 2 site（`0x489f8fb`、`0x37cd3eb`）。表の外の索引が `fle_jumptab_miss` に届く入り方は合成 PE で確認し、実物の DLL では実施していない。`f_489f7d0` が memset 風であることは読み（実行して確かめていない）。
- 3 モジュールの測定は 1 構成（FLEngine + 縮小 dsp_ippv2 基底 0x2000000 + Blend2D 基底 0x4700000、種 58 個。測定用の値）。単独構成の a4、wasm、実 Windows は実施していない。母集団の検査は FLEngine、dsp_ippv2、Blend2D、QFC の 4 つ。
- 新しい停止（`quickfontcache_x64.dll` の `LoadLibraryW`）は直していない。callee-saved の B が call をまたいで保たれる仮定（Win64。0089 から）は変わらない。
