# 0109: freetype の `FT_Stream_ReadFields` の switch（`0x4b1674d`）を読むために、MSVC switch 表の認識に「scheduled 形」（コンパイラが guard と idiom の間、`cmp` と分岐の間に他の命令を散らした形。入口が 2 つのループ回転形を含む）を加える（仮の決定。HEAD 3a1050a に統合し、コンパイル・全体テスト・測定まで実施）

状態: **仮の決定**。作業者（サブエージェント）の記録で、PM の決定ではない。第 1 段階（調査・決定の案・実装案・テストの案。コンパイルもテストの実行もしていない）の案を、第 2 段階（統合担当）で **HEAD `3a1050a` に統合し、コンパイル（警告 0・エラー 0）、全体テスト（112 test result・**1,491 passed**・0 failed。グループ分けの合算で、1 回の通しではない）、変異検査 21 件、native の統計（freetype、Blend2D、dsp、QFC、ILTools、FLEngine）、6 モジュール構成の a4 の測定まで実施**した。受理条件は PM が依頼文で定めた（下の「PM の決定」）。**実機 Windows との照合は 0 件、wasm は未確認**。「予測」と書いた数は第 1 段階の独立のモデル（capstone）の値で、実測との差は下の「第 2 段階の結果」に書いた。書式は 0095 に倣う。

記入状況: 背景・決定・PM の決定・第 2 段階の結果（実測）・範囲・範囲外・実装した場所・テスト・検証・解釈が分かれた点・PM の判断が要る点・限界・撤回条件のすべて記入済み。第 1 段階の記述のうち数値が変わった箇所は `lift/out-git/t2-results/MERGE-NOTES.md` に旧 → 新で記録した。ゲストの実行（6 モジュール構成で freetype の `FT_Stream_ReadFields` に届くこと）は未確認（フォントを置いていない。別の作業）。

凡例: **実測** = コマンドの出力を見た事実。**読み** = 実測（バイト列・ソース）から読んだ解釈。**予測** = 独立の走査（capstone による Python のモデル。実装ではない）の結果で、実装の結果とずれうる。**未確認** = 確かめていない。**意見** = 作業者の意見。

事実の出典:

- `docs/decisions/0089`〜`0095`（MSVC switch 表の認識の系列。0094 = 基底証明は到達定義、0095 = swapped 形と成立側のガードの規則）、`docs/investigations/msvc-switch-other-forms.md`（形の分類 F0〜F5。Blend2D と dsp が対象で、freetype は未分類だった）。
- `lift/crates/fe-decode/src/jumptab.rs`（HEAD fc940a9）の `recognize_msvc_standard`（748〜762 行が `ja` までの遡り、775〜777 行が `cmp` の検査）、`resolve_msvc`、`derive_swapped_guard`、`verify_msvc_swapped`、`landings_in_swapped_idioms`、`prove_base_reaching_defs`。
- freetype の入力: `lift/out-wallQ/freetype/freetype_x64.4b00000.dll`（`freetype_x64.dll` = SHA-256 `6ea6c106…3cb8`、元の ImageBase `0x180000000`、rebase 後 `0x4b00000`。測定用の基底）。6 モジュール構成の生成ログ `lift/out-git/i2-results/e-6mod-final/log/log-gen-m4.txt`（HEAD + I2）。種は `lift/harness/seeds/ft-seeds-4b00000.txt`（160 行）。
- 作業物: `lift/out-git/t1-results/`（第 1 段階: この文書の元、`t1-freetype.diff`、`freetype-msvc-rejected.tsv`、`MERGE-NOTES.md`）。第 2 段階: `lift/out-git/t2-results/`（`resolved-T.diff`、この文書、`README-0109-line.txt`、`MERGE-NOTES.md`、`logs/`）。基準の木は HEAD `3a1050a`（`git archive`）。

## 決定（第 1 段階の案を、PM の決定で確定した形）

| # | 項目 | 案 |
|---|---|---|
| 1 | 認識の追加（scheduled 形） | 標準の idiom `[movzx eX,byte [B+I0+T1];] mov eT,[B+I*4+T2]; add rT,B; jmp rT`（0089 の (c1)／(c2)。これら 2〜3 命令は隣接）が見つかり、`recognize_msvc_standard` が N を導けず **`Err`（`N not derived`）** を返したとき、**基底証明が到達定義（既定）の場合に限り** `recognize_msvc_scheduled` で同じ idiom を読み直す。N は `resolve_msvc` が関数の線形走査の CFG から求める（`derive_scheduled_guard`）。標準形が `Ok(Some)`／`Ok(None)` の site、swapped 形、比較専用の基底証明（`--msvc-base-dominating`、`--msvc-base-entry-block`）の結果は変えない |
| 2 | N の導出 | idiom の先頭命令（(c2) は `movzx`）から、**直前の命令が落ちてくる限り**（`FlowControl::Next`）1 命令ずつ遡り、各命令に**直接分岐で入る辺**と、**直前の条件分岐の落ち側**を集める。**すべての入口**が「`cmp I,imm` と、符号なしの `I <= N` 側（`ja` の不成立、`jbe` の成立、`jae` の不成立、`jb` の成立。`jb imm` は N = imm − 1。0095 と同じ）の条件分岐」でなければならない。`call`、無条件の `jmp`、関数の入口が入口なら拒否（`N not derived`）。入口は**最大 2**（`MSVC_SCHED_GUARDS`）、遡りは**最大 16 個の位置**（`MSVC_SCHED_WALK`。idiom の先頭を含むので、隙間は 15 命令まで）。**N = 各入口の上限の最大**（表は N + 1 個読む。入口ごとの上限が小さい経路にとって余りは到達しない） |
| 3 | 入口と idiom の間・`cmp` と分岐の間の命令 | 制御を変えず（`FlowControl::Next`）、**確認する添字レジスタ I を書かない**命令は何でも許す（ストア、他のレジスタの `lea`／`mov`／`xor`、`nop`、`lea B,[ImageBase]`）。I を書いてよいのは次の 4 の符号拡張だけ。個数の上限は遡りの 16 命令だけ（`MSVC_SKIP` = 2 は標準形の遡りの上限のまま）。`cmp` と分岐の間は、さらに**旗を書く命令を許さない**（分岐は `cmp` の旗を見る）。メモリへの書き込みは許す（安全性は下の議論。**表が書き込み不可のセクションにあることの検査を行 9 で足した**） |
| 4 | I を書く命令 | **符号拡張だけ**: `cdqe`（rax）と `movsxd rD,eS`（D ≠ S でもよい。比較は S、添字は D）。値が変わらない（`cmp eS,N` の N < 4096 なので eS < 2³¹）ことに依る。`mov` のコピー、`lea`／`add`／`sub` の補正、`shr`、`movzx` は I の書き込みとして**拒否**（Blend2D の F3 のコピーをたどる形は広げない。0094 の決定 4 の方針を保つ） |
| 5 | 基底証明 | 0094（到達定義）のまま。問い合わせ位置は **idiom の先頭命令**（0095 の swapped と同じ。`add rT,B` が B を変えないので jmp での値と同じ）。第 2 の入口（ループの下端からの辺）の定義も流れ込む。非 `lea` の定義が届けば拒否、揮発レジスタの B をまたぐ `call` も拒否（0094） |
| 6 | 移動先の検査と事後検査 | 0089 の 1(d) のまま（`.pdata` の同じ関数の範囲内、命令の先頭）。閉包の完成後の検査は swapped と同じ 2 つ（`verify_msvc_swapped_entries`、`landings_in_swapped_idioms`）に、**第 2 の入口を許す**変更を加える（`Origin::Msvc::entry2`） |
| 7 | 報告 | 標準形の理由（例 `N not derived: too many instructions between the `ja` and the table idiom`）に、続けて ` [N not derived (scheduled form, decision 0109): …]` を付ける（既存のテストとログが見る前半は不変）。N 以外の理由（基底証明、移動先、`.pdata` の外）は新しい文言のまま |
| 8 | 範囲外（変えない） | 標準形の認識とその結果、swapped 形、F2（表の値の取り方が正準でない）、F4、F5、`.pdata` の外の `jmp reg`（0089 の (d)）、N の導出で `mov` のコピー・`lea`・`shr` を許すこと、入口が 3 つ以上、`jmp reg` が closure に到達しない site |
| 9 | 表のセクション（**第 2 段階で追加。PM の決定 3**） | scheduled 形の site は、読んだ**表（RVA 表、添字バイト表）が `IMAGE_SCN_MEM_WRITE` のないセクション（またはヘッダ）にある**ときだけ受理する（`scheduled_tables_are_read_only`。`resolve_msvc` で表の読み出しと基底証明のあと、移動先が命令の先頭かの検査の前。添字バイト表を先に見る）。隙間のストア（自己書き換え）で表が変わる誤受理を防ぐ。理由文: `the RVA table at 0x… lies in the writable section `.rdata`: the scheduled form (decision 0109) allows stores in the gap, so the table could change at run time`。**標準形と swapped 形には掛けない**（その形の隙間はストアを許さない。3 つの DLL では掛けても解決数も生成 C も変わらないことを実測: 「解釈が分かれた点」の 8） |
| 10 | 状態 | **仮の決定**。バイト列とネイティブ実行（合成 PE）で検証した部分が大きいが、ゲスト（6 モジュール構成）で受理した site が実行されたことは未確認で、実機 Windows との照合もない。**0093 の「`ja` と idiom の間は `nop` のみ・最大 2 個」を既定の基底証明では置き換える**判断を含む（PM が依頼文で採った） |

## PM の決定（第 2 段階の依頼文。第 1 段階の「PM の判断が要る点」への答え）

| 第 1 段階の判断点 | PM の決定 | この文書での扱い |
|---|---|---|
| 1 0093 の置き換え | 採る。置き換えは基底証明が到達定義（既定）のときだけ。比較専用（Dominating／EntryBlock）では従来の挙動 | 決定 1 のとおり。0093 の 2 テストと fe-lift の 1 テストは比較専用の基底証明に付け替えて元の期待を保つ |
| 1（隙間）、3（`movsxd`） | `ja` と idiom の間・`cmp` と `ja` の間の命令は、添字レジスタを書かなければ許す。書いてよいのは `cdqe` と `movsxd` だけ | 決定 3、4 のとおり |
| 2 入口が複数 | 複数あってよいが最大 2 つ。各入口は `cmp I,imm` と符号なしの条件分岐で守られていること。遡りは最大 16 命令位置 | 決定 2 のとおり |
| 4 メモリへの書き込み | 隙間の書き込みを許す。**表が書き込み不可のセクションにあることの検査を足す** | 案には無かったので**第 2 段階で実装**（決定 9、テスト 2 件） |
| 5 Blend2D と dsp の +2、+2 | 受け入れる。Blend2D と freetype の生成 C が変わることも受け入れる | 下の実測 |
| 6 残る 5 件 | 触らない（`.pdata` の外の葉関数、揮発 B が `call` をまたぐ形、ワイルドカード）。既存の拒否テスト（F2 の 4 件など）を壊さない | 5 件は同じ理由で拒否のまま。既存の拒否テストは 1 件も変えていない |

## 第 2 段階の結果（実測。HEAD 3a1050a、native。実機 Windows との照合なし）

- **統合**: `t1-freetype.diff` は HEAD `3a1050a`（d3be2a3 の `jumptab.rs` のテストの更新、3a1050a が入った後）に `patch -p1` で**ずれ・拒否なしに当たった**（6 ファイル）。足したのは決定 9 の検査（`fe-pe` の `Section::is_writable`／`contains_rva`／`IMAGE_SCN_MEM_WRITE`、`jumptab.rs` の `scheduled_tables_are_read_only`）とテストの追加・期待の更新だけ。`resolved-T.diff` は 7 ファイル、+1,506 −68。
- **コンパイル**: `cargo build --offline`（dev）は**警告 0・エラー 0 で初回に通った**。テスト binary のコンパイルも型・構文のエラー 0（警告は HEAD にある 2 件だけ: `tests/monitor.rs` の `INIT_HEX`、`tests/sse_shuf.rs` の不要な `mut`）。**直したコンパイルエラーは 0**。
- **全体テスト（グループ分けの合算。1 回の通しではない）: 112 test result・1,491 passed・0 failed**。HEAD の統合した木（112・1,469・0）に対し **+22**（fe-decode 17 = 第 1 段階の案 14 + 第 2 段階で足した 3、fe-lift 5 = 案 4 + 1）。binary ごとの比較で変わったのは fe-decode の `msvc_switch`（78 → 95）と fe-lift の `msvc_switch`（23 → 28）だけ。
- **期待値の更新（実測で確かめた）**: (1) fe-decode の `dsp_ippv2_f_37cc7e0_…` の解決 site の集合 6 → 8（`0x37b5cea`、`0x37b6554` を足す。第 1 段階の差分は同じ集合を見る 2 つ目のテストを直し忘れていた）。(2) fe-lift の `real_blend2d_site_is_resolved_by_default` `(31, 31)` → `(33, 33)`（`0x4756752`、`0x47596c2` が `switch` になっていることも確認）。(3) fe-lift の `padded_sites_that_are_not_resolved_stay_a_tail_dispatch`: 9 件のうち 3 件（`MOV_EDX_1`、`PAUSE`、`NOP_5,NOP_3,NOP_1`）は既定で受理に変わるので、比較専用の基底証明（`Dominating`）では従来どおり tail dispatch、既定の側は新しいテスト `padded_sites_that_decision_0093_rejected_are_resolved_by_the_scheduled_form_by_default` に分けた。(4) freetype の実 DLL のテストの `msvc_tables >= 28` を実測の `== 29`（と拒否 5）に締めた。第 1 段階の差分にあった他の期待値（Blend2D `(33, 33, 47)`、dsp の `jt_cpu`: `msvc_tables` 8、`jumptabs` 442、`jmp:unresolved` 541、`jmp:msvc_table` 8、fe-lift の `jt_cpu.rs` `(438, 4)` は**不変**）は予測どおりで、そのまま通った。
- **変異検査 21 件**（Rust 側。1 件ずつ変更 → 該当テストが落ちる → 元に戻す。木のコピー上）: 第 1 段階の提案 11 件と自分で足した 10 件。**20 件は既存のテストが検出**。**1 件（M17: 隙間の `call` を普通の命令として通す）は検出されなかった**（隙間にも `cmp` と分岐の間にも `call` を置くテストが無かった）ので、テスト `scheduled_a_call_in_the_gap_or_between_the_cmp_and_the_branch_is_rejected` を足し、足した後に検出されることを確認した。
- **解決数（予測との差）**: freetype 17 → **29**（予測 28。**差 +1 は 2 次効果**: `0x4b58c24` の表の移動先が届く `f_4b588d0` の中の標準形の switch `0x4b58f68`（N = 55、15 通り）。第 1 段階が「届けばさらに増える」と書いた site）、拒否 16 → **5**（予測と同じ 5 件）、`register_jmp_sites` 53 → 54。Blend2D（種 58 個）31 → **33**、拒否 50 → 48（種なし 31 → 33、49 → 47）。dsp `--pdata` 6 → **8**、拒否 6 → 4。dsp 縮小、QFC、ILTools、FLEngine は **不変**（`msvc_tables` も生成 C のファイルも）。予測した受理 15 件（freetype 11、Blend2D 2、dsp 2）の**異なる移動先の数と移動先の集合**は、実装の結果と独立のモデルが全件一致した（N は freetype の 11 件でテストが実装の値を固定している。Blend2D と dsp の 4 件の N は、実装の出力から取っていない）。
- **生成 C**: FLEngine（註釈なし）のファイル SHA-256 は **`94fe0c85…` のまま**（215,751,676 B、`unsupported_total` 494、functions 59,380、bad_nodes 0。註釈あり `ce205cfe…` も不変）。変わったのは freetype（本体が変わった関数 11、増えた関数 18、`functions` 1,625 → 1,643）、Blend2D（2 関数 `f_4756730`、`f_4759690`。関数は増えない）、dsp `--pdata`（2 関数 `f_37b5cc0`、`f_37b64b0`）。dsp 縮小、QFC、ILTools の C は不変。
- **着地の確認（独立の走査）**: 受理した全 site（freetype 12、Blend2D 2、dsp 2）で、生成 C の `case` の移動先が全部**同じ関数（`.pdata` のピース）の中の capstone の線形走査の命令の先頭**で、`goto` 先のラベルがあり、site 自体が `jmp reg`（違反 0）。独立のモデルの移動先の集合が freetype 11 + Blend2D 2 件で実装の `case` の集合と一致（`0x4b58f68` は標準形で N = 55、15 通りが一致。dsp の 2 件は移動先の数 9、6 の一致だけ）。
- **6 モジュール構成（a4。註釈あり。測定用の基底と種）**: init_count **246 / 942**、functions **3,377**、entries **133,965**、停止 `fle: unsupported instruction at 0x601da30`（`gdi32!MoveToEx`）、call depth 36。W6 統合時の基準と**同じ**（回帰なし）。freetype の C は変わっている（`msvc_tables` 29）が、この構成はフォントを置いていないので `FT_Stream_ReadFields` に届く前に止まる。**停止点 `0x4b1674f` が動いたかは確認できない**（A4 の達成ではない）。


## 背景

### 停止点（`FT_Stream_ReadFields`、`f_4b166e0` の `0x4b1674d`）

- 依頼文によると、6 モジュール構成の A4 の測定は `fle: indirect call to 0x4b1674f is not a known function entry` で止まり、QFC がフォントの実体（face）を作る経路（`ILCursors.ilfont` を開く）がここを通る（**未確認**。この作業では実行していない）。`0x4b1674f` は下の表のエントリ 0 の移動先。
- 生成ログ（HEAD + I2、6 モジュール構成の freetype）: `register_jmp_sites 53`、`jumptabs 17`、`msvc_tables 17`、`msvc_rejected 16`、closure 99,984、functions 1,625、bad_nodes 0。**HEAD の `fe-lift`（`h2/febin`）で再現した**（`fe-lift lift freetype_x64.4b00000.dll … --pdata --seeds ft-seeds-4b00000.txt`。統計は `c_lines`／`c_bytes` の 2 行だけが生成ログと違う（110685／8822210 と 110688／8822384。I2 の入った版と HEAD + H の版の差と読む）。種なし: closure 98,450、functions 1,489、`register_jmp_sites 52`、`msvc_rejected 15`（`0x4b3dcfe` は種ありでだけ closure に入る））。拒否 16 件の理由別（実測）:

| 理由（`msvc_rejected` の文言） | 件数 | site |
|---|---:|---|
| `N not derived: no `ja` before the table idiom (only `cdqe`, `lea B,[rip+d]` and `nop` may sit between)` | 9 | `0x4b1a9cc`、`0x4b07cce`、`0x4b2044e`、`0x4b2f423`、`0x4b30485`、`0x4b31175`、`0x4b4f316`、`0x4b58c24`、`0x4b3dcfe` |
| `N not derived: too many instructions between the `ja` and the table idiom` | 3 | `0x4b1674d`（**停止点**）、`0x4b1f22d`、`0x4b1f91d` |
| `N not derived: no `cmp idx,imm` right before the `ja`` | 3 | `0x4b46d08`、`0x4b0ef6a`、`0x4b2ffb4` |
| `the jump is not inside a .pdata function` | 1 | `0x4b434ce` |

  基底証明の失敗、移動先の失敗は 0 件（N の導出が先に落ちるので**まだ見えていない**）。解決済みの 17 件はすべて標準形（`cmp; ja` の直後に `cdqe`／`lea B`／`nop` が 0〜2 個だけ挟まって idiom。**実測**。capstone で全件確認）。
- 停止点の命令列（**実測**。capstone。再配置後の VA。元の VA は +`0x17b500000`）:

```
  4b16719: lea eax, [r8 - 4]
  4b16724: cmp eax, 0x15
  4b16727: mov qword ptr [rsp + 0x50], r14          ; ← cmp と ja の間（r14 の退避。コンパイラが前の部分に散らした spill と読む）
  4b1672c: ja 0x4b168e5
  4b16732: lea r14, [rip - 0x16739]                 ; r14 = ImageBase（0x4b00000）
  4b16739: nop dword ptr [rax]                      ; 3 バイト
  4b1673c: nop dword ptr [rax]                      ; 4 バイト（0x4b16740 を 16 バイト境界に揃える padding と読む）
  4b16740: cdqe                                     ; ← ループの頭。第 2 の入口が着地する
  4b16742: mov ecx, dword ptr [r14 + rax*4 + 0x16934]
  4b1674a: add rcx, r14
  4b1674d: jmp rcx
  ...
  4b168d4: movzx r8d, byte ptr [rbx]                ; ループの下端
  4b168d8: lea eax, [r8 - 4]
  4b168dc: cmp eax, 0x15
  4b168df: jbe 0x4b16740                            ; ← 第 2 の入口（loop rotation で複製された guard）
```

  `.pdata` では `f_4b166e0` は 7 ピースの連鎖（`[0x4b166e0,0x4b1698c)`。ピースの境界が spill の `mov` の直前に来る。**読み**）。表は RVA `0x16934`（VA `0x4b16934`）で、**最後のピース `[0x4b16934,0x4b1698c)` の中にあり、22 エントリ（N = 0x15）でちょうどピースが終わる**（**実測**）。22 エントリの移動先は 10 通り（`0x4b1674f`、`0x4b16775`、`0x4b167ab`、`0x4b167bd`、`0x4b167dd`、`0x4b167fe`、`0x4b16828`、`0x4b16853`、`0x4b16876`、`0x4b168e5`）で、**capstone の走査の命令の先頭 10/10、すべて関数の 7 ピースの内側**（**実測**）。
- **なぜ現行の認識器が拒否するか（どの検査が false を返すか）**（**読み**。ソースと `fe-lift` の理由文の一致から）:
  1. `recognize_msvc_standard` が `ja` へ遡る `loop`（HEAD の 741〜763 行）で、`mov ecx` の直前から `cdqe`（`neutral` = true、`skipped` = 1）、`nop`（2）、`nop`（`neutral` = true だが `skipped > MSVC_SKIP` が **true**、759 行）→ `too many instructions between the `ja` and the table idiom`。ここが理由文と一致する。
  2. この上限を緩めても、次の `lea r14` は `neutral`（B への `lea`）で、その先の `ja` に届いたあと、`ja` の直前が `mov [rsp+0x50],r14` なので 775〜777 行の `cmp` の検査が false。
  3. 両方を緩めても、idiom の途中（`cdqe`、`0x4b16740`）に**第 2 の入口** `jbe 0x4b16740`（`0x4b168df`）があり、閉包の完成後の `verify_msvc`（`leaders.contains(&va)` の「a block starts at …, inside the switch idiom」）と `landings_in_idioms`（着地が `(guard_cmp, jmp]` の内側）が落とす。
  つまり停止点には**3 つの障害が重なる**。1 だけ（`MSVC_SKIP` を 4 に上げる）では直らない。
- 16 件の分類は下の表と `freetype-msvc-rejected.tsv`。F0〜F5（`msvc-switch-other-forms.md` の定義）に当てはめると、15 件は **F3**（標準の (c1)／(c2) が `add`／`jmp` の直前にあり、N が導けない）、1 件（`0x4b434ce`）は **F0x**（形も N も正しく、`.pdata` の外で拒否）。freetype の F3 は Blend2D／dsp の F3（添字を `shr`、コピーで作る形）と**原因が違い**、次の 4 つの副形に分かれる（この文書の用語。決定ではない）:
  - **F3a**: `ja` と idiom の間に `MSVC_SKIP`（2）を超える、または `cdqe`／`lea B`／`nop` 以外の命令（ストア、他のレジスタの `lea`、`xor edi,edi`）が挟まる。
  - **F3b**: `cmp` と `ja` の間に旗を書かない命令（ストア、`mov`、`lea`）が挟まる。
  - **F3c**: 添字の拡張が `movsxd rD,eS`（`cdqe` の一般形。D ≠ S のことがある）。
  - **F3d**: ループ回転。`switch` がループの中にあり、ループの下端に複製された guard（`cmp; jbe`）が idiom の途中（アラインされたループの頭）へ飛び込む。**入口が 2 つ**。

| 再配置後の VA | 関数 | 形・基底・添字 | 余分な命令 | N／異なる移動先 | 現行の拒否（false になる検査） | F 分類（副形） | 案の結果 |
|---|---|---|---|---|---|---|---|
| `0x4b07cce` | `0x4b07bf0` | c2、B=rdx、比較 ecx、添字 rcx (movzx byte table) -> rax | ja〜idiom: `lea rdx, [rip - 0x7cb9]; movsxd rcx, ecx` | 45／4 | recognize_msvc_standard: `neutral` is false for `movsxd rcx, ecx` at 0x4b07cb9 (only cdqe, lea B,[rip+d], padding nop) | F3（F3c index extension (movsxd)） | **受理** |
| `0x4b0ef6a` | `0x4b0ed40` | c1、B=r10、比較 eax、添字 rax | cmp〜ja: `mov qword ptr [rsp + 0x48], r15; mov rcx, rax` | 37／35 | recognize_msvc_standard: the instruction before the `ja` at 0x4b0ef59 is `mov rcx, rax`, not `cmp idx,imm` | F3（F3b instructions between cmp and ja） | **受理** |
| `0x4b1674d` | `0x4b166e0` | c1、B=r14、比較 eax、添字 rax | cmp〜ja: `mov qword ptr [rsp + 0x50], r14`<br>ja〜idiom: `lea r14, [rip - 0x16739]; nop ; nop ; cdqe ` | 21／10 | recognize_msvc_standard: `skipped > MSVC_SKIP` (MSVC_SKIP = 2) while walking back from the load over cdqe / lea B / nop | F3（F3a gap between ja and idiom (more than MSVC_SKIP = 2 neutral ones) + F3b instructions between cmp and ja + F3d loop-rotated: second guard `cmp; jbe` at the loop bottom jumps to the aligned head (cdqe)） | **受理**（2 入口） |
| `0x4b1a9cc` | `0x4b1a8d0` | c1、B=rdx、比較 eax、添字 rax | ja〜idiom: `lea rdx, [rip - 0x1a9bb]; cdqe ; mov qword ptr [rsp + 0x60], rsi` | 5／4 | recognize_msvc_standard: `neutral` is false for `mov qword ptr [rsp + 0x60], rsi` at 0x4b1a9bd (only cdqe, lea B,[rip+d], padding nop) | F3（F3a gap between ja and idiom (instructions that are not cdqe / lea B / nop: mov)） | **受理** |
| `0x4b1f22d` | `0x4b1f1b0` | c1、B=r11、比較 eax、添字 rax | ja〜idiom: `lea r11, [rip - 0x1f215]; nop ; nop ; nop ; cdqe ` | 9／10 | recognize_msvc_standard: `skipped > MSVC_SKIP` (MSVC_SKIP = 2) while walking back from the load over cdqe / lea B / nop | F3（F3a gap between ja and idiom (more than MSVC_SKIP = 2 neutral ones) + F3d loop-rotated: second guard `cmp; jbe` at the loop bottom jumps to the aligned head (cdqe)） | **受理**（2 入口） |
| `0x4b1f91d` | `0x4b1f890` | c1、B=r8、比較 eax、添字 rax | ja〜idiom: `lea r8, [rip - 0x1f907]; xor edi, edi; nop ; nop ; cdqe ` | 9／10 | recognize_msvc_standard: `skipped > MSVC_SKIP` (MSVC_SKIP = 2) while walking back from the load over cdqe / lea B / nop | F3（F3a gap between ja and idiom (instructions that are not cdqe / lea B / nop: xor) + F3d loop-rotated: second guard `cmp; jbe` at the loop bottom jumps to the aligned head (cdqe)） | **受理**（2 入口） |
| `0x4b2044e` | `0x4b203c0` | c1、B=rbp、比較 eax、添字 rax | cmp〜ja: `mov qword ptr [rsp + 0x20], r15`<br>ja〜idiom: `lea rbp, [rip - 0x20426]; lea r13, [rip + 0x492f3]; lea r14, [rip + 0x492d4]; lea r15, [rip + 0x492b5]; lea r8, [rip + 0x49296]; cdqe ` | 13／14 | recognize_msvc_standard: `neutral` is false for `lea r8, [rip + 0x49296]` at 0x4b2043b (only cdqe, lea B,[rip+d], padding nop) | F3（F3a gap between ja and idiom (instructions that are not cdqe / lea B / nop: lea, lea, lea, lea) + F3b instructions between cmp and ja + F3d loop-rotated: second guard `cmp; jbe` at the loop bottom jumps to the aligned head (cdqe)） | **受理**（2 入口） |
| `0x4b2f423` | `0x4b2f2f0` | c1、B=r10、比較 edx、添字 rax | ja〜idiom: `movsxd rax, edx` | 6／6 | recognize_msvc_standard: `neutral` is false for `movsxd rax, edx` at 0x4b2f415 (only cdqe, lea B,[rip+d], padding nop) | F3（F3c index extension (movsxd)） | 拒否のまま: base proof (decision 0094): call at 0x4b2f400 lies on a path to the jump and r10 is caller-saved |
| `0x4b2ffb4` | `0x4b2fb10` | c1、B=r10、比較 r8d、添字 r8 | cmp〜ja: `lea r15, [r10 + rcx*4 - 8]; mov dword ptr [rsp + 0xe0], r9d; mov edi, r9d; mov dword ptr [rsp + 0x38], r9d`<br>ja〜idiom: `lea r10, [rip - 0x2ffa9]` | 28／16 | recognize_msvc_standard: the instruction before the `ja` at 0x4b2ff9c is `mov dword ptr [rsp + 0x38], r9d`, not `cmp idx,imm` | F3（F3b instructions between cmp and ja） | 拒否のまま: base proof (decision 0094): definition of r10 at 0x4b2fc5f (mov) reaches the jump and is not lea ImageBase |
| `0x4b30485` | `0x4b2fb10` | c1、B=r8、比較 edx、添字 rax | cmp〜ja: `lea rdi, [r10 + rcx*4]`<br>ja〜idiom: `movsxd rax, edx` | 25／25 | recognize_msvc_standard: `neutral` is false for `movsxd rax, edx` at 0x4b30477 (only cdqe, lea B,[rip+d], padding nop) | F3（F3c index extension (movsxd) + F3b instructions between cmp and ja） | 拒否のまま: base proof (decision 0094): definition of r8 at 0x4b2fb41 (xor) reaches the jump and is not lea ImageBase |
| `0x4b31175` | `0x4b31090` | c2、B=r15、比較 ecx、添字 rcx (movzx byte table) -> rcx | ja〜idiom: `movsxd rcx, ecx` | 55／4 | recognize_msvc_standard: `neutral` is false for `movsxd rcx, ecx` at 0x4b3115e (only cdqe, lea B,[rip+d], padding nop) | F3（F3c index extension (movsxd)） | **受理** |
| `0x4b3dcfe` | `none` | c1、B=r8、比較 edx、添字 rax | ja〜idiom: `lea r8, [rip - 0x3dcf0]; movsxd rax, edx` | 6／— | resolve_msvc: fm.function_at(jmp_rva) is None ("the jump is not inside a .pdata function"); recognize_msvc_standard fails first at the `movsxd` (not neutral) | F3（F3c index extension (movsxd) + no .pdata (leaf function)） | 拒否のまま: not inside a .pdata function |
| `0x4b434ce` | `none` | c1、B=r8、比較 eax、添字 rax | ja〜idiom: `lea r8, [rip - 0x434c3]` | 7／— | resolve_msvc: fm.function_at(jmp_rva) is None | F0x（F0 shape, no .pdata (leaf function)） | 拒否のまま: not inside a .pdata function |
| `0x4b46d08` | `0x4b46af0` | c1、B=r13、比較 edx、添字 rdx | cmp〜ja: `lea rsi, [r8 + rax*4]; mov byte ptr [rdi + 0x220], 1; mov rax, rdx; mov dword ptr [rdi + 0x18], r12d` | 143／114 | recognize_msvc_standard: the instruction before the `ja` at 0x4b46cf7 is `mov dword ptr [rdi + 0x18], r12d`, not `cmp idx,imm` | F3（F3b instructions between cmp and ja） | **受理** |
| `0x4b4f316` | `0x4b4f2e0` | c1、B=rcx、比較 edx、添字 rax | cmp〜ja: `mov rbx, r9; mov rdi, rcx`<br>ja〜idiom: `lea rcx, [rip - 0x4f309]; movsxd rax, edx` | 45／46 | recognize_msvc_standard: `neutral` is false for `movsxd rax, edx` at 0x4b4f309 (only cdqe, lea B,[rip+d], padding nop) | F3（F3c index extension (movsxd) + F3b instructions between cmp and ja） | **受理** |
| `0x4b58c24` | `0x4b588d0` | c1、B=r10、比較 r8d、添字 rax | cmp〜ja: `mov dword ptr [rsp + 0x6c], r8d`<br>ja〜idiom: `movsxd rax, r8d` | 7／8 | recognize_msvc_standard: `neutral` is false for `movsxd rax, r8d` at 0x4b58c16 (only cdqe, lea B,[rip+d], padding nop) | F3（F3c index extension (movsxd) + F3b instructions between cmp and ja） | **受理** |

  （異なる移動先の数は表を読んだ capstone の結果。`0x4b3dcfe`、`0x4b434ce` は `.pdata` の外なので表は読まない。）
  **案の結果（予測。第 2 段階の実測は受理 11 件 + 2 次効果 1 件で、拒否のまま 5 件は同じ。上の「第 2 段階の結果」）**: 受理 **11 件**（停止点を含む）、拒否のまま **5 件**（`0x4b2f423` = 揮発の `r10` が `call` をまたぐ（`lea r10,[ImageBase]` を `call 0x4b2d7a0` の前に置き、後で使う形。MSVC の全体最適化で callee が `r10` を壊さないと知っているのかもしれない（**読み**・未確認）。0094 の規則では拒否が正しい）、`0x4b2ffb4`／`0x4b30485` = 同じ関数に別の間接 `jmp` があり、非 `lea` の `r10`／`r8` の定義がそこへ届く（0094 のワイルドカード）、`0x4b3dcfe`／`0x4b434ce` = `.pdata` の外の葉関数（0089 の (d)））。

### 他の DLL（予測。実測は上の「第 2 段階の結果」）

- 案の新しい経路が動くのは、標準形が `Err` を返した site だけ。QFC、ILTools、FLEngine は `msvc_rejected` が 0（**実測**）なので**動かない**。Blend2D（6 モジュール構成の種 58 個あり／なし）は 2 件、dsp_ippv2 は `--pdata` で 2 件が受理に変わる（**予測**）。縮小 dsp（6 モジュール構成）は 0 件。

## 範囲（今回扱う）

- `recognize_msvc_scheduled`（標準の idiom の読み。`recognize_msvc_standard` の前半のコピー）、`derive_scheduled_guard`（入口の集め方と N）、`sched_find_cmp`、`sext_source`。
- `resolve_msvc` の scheduled の分岐、`Origin::Msvc::entry2`、`verify_msvc_swapped_entries`、`landings_in_swapped_idioms`（同じ jmp の 2 つ目の組を許す）、`lib.rs` の呼び出しと理由文。
- テストの追加と、0093 の「`ja` と idiom の間は `nop` のみ」を既定の基底証明で確かめていた既存のテストの直し（比較専用の基底証明で元の期待を保つ）。

## 範囲外（今回扱わない。従来どおり）

- 標準形の `recognize_msvc_standard`、swapped 形（0095）、F2、F4、F5 の認識。`MSVC_SKIP`、`MSVC_IDIOM_MAX`、`MSVC_SWAP_SKIP` の値。
- `.pdata` の外の葉関数の `jmp reg`（`0x4b3dcfe`、`0x4b434ce`。0089 の (d) を変える判断は別）。
- 揮発レジスタの基底が `call` をまたぐ形（`0x4b2f423`。callee がその B を保つことの証明が要る）、別の間接 `jmp` のワイルドカード（0094）の緩和。
- N の導出で `mov` のコピー・`lea`・`sub` の補正・`shr` を許すこと（Blend2D の F3 の大半。決定 0094 の 4）、`jmp reg` の closure への到達（`0x4b58f68` など）。
- 入口が 3 つ以上の idiom、`jmp reg`／`jmp [mem]` で idiom の途中へ着地する形（線形走査に見えない。事後の検査は閉包が解決したものだけ）。

## 実装した場所（`resolved-T.diff`。HEAD `3a1050a` の木に対する差分。fe-decode と、決定 9 のための fe-pe の追加）

- `lift/crates/fe-decode/src/jumptab.rs`（追加のみ。既存の関数の本体は変えていない。`verify_msvc_swapped` と `landings_in_swapped_idioms` は薄い包み／1 行の述語の置換）:
  - `Origin::Msvc` に `entry2: Option<(u64, u64)>`（第 2 の入口 = ループの下端の分岐と着地点）。`SwapForm` に `scheduled: bool`。定数 `MSVC_SCHED_WALK` = 16、`MSVC_SCHED_GUARDS` = 2。
  - `recognize_msvc_scheduled`、`sext_source`、`sched_find_cmp`、`derive_scheduled_guard`、`sched_nd`（構造体 `SchedEntry`／`SchedGuard`）。
  - `resolve_msvc`: `f.swapped` の `Some(s)` で `s.scheduled` なら `derive_scheduled_guard` を使う（swapped は従来の `derive_swapped_guard`）。基底証明は同じ `prove_base_reaching_defs`（問い合わせ位置は `s.load`）。
  - `verify_msvc_swapped_entries`（`verify_msvc_swapped` はこれを `extra = None` で呼ぶ包み）、`landings_in_swapped_idioms`（許す組を `(jmp, branch, target)` の集合にした。1 つの jmp に組が 1 つなら従来と同じ）。
- `lift/crates/fe-decode/src/lib.rs`: 標準形が `Err` のとき（基底証明が到達定義の場合）`recognize_msvc_scheduled` を試す。標準形の理由を保って ` [スケジュール形の理由]` を付ける。`entry2` を landings の組と事後検査に渡す。
- 第 2 段階で足した実装（決定 9）: `lift/crates/fe-pe/src/lib.rs` に `IMAGE_SCN_MEM_WRITE`、`Section::is_writable`、`Section::contains_rva`（追加のみ）。`jumptab.rs` に `scheduled_tables_are_read_only` と、`resolve_msvc` の表の読み出し・基底証明の後の呼び出し（`f.swapped` が `scheduled` のときだけ）。
- 変えていないもの: `fe-lift` の `src` の全ファイル、`lift/runtime`、`MSVC_SKIP`、`recognize_msvc_standard`、`derive_swapped_guard`、`prove_base_reaching_defs`。

## テスト（実行した。第 2 段階の結果は上）

新規 **22 件**（fe-decode 17、fe-lift 5。第 1 段階の案の 18 件 + 第 2 段階で足した 4 件: ①書き込み可能なセクションの表の拒否（`scheduled_a_table_in_a_writable_section_is_rejected_because_the_gap_may_hold_stores`。1 段・2 段・成立側・2 入口の 5 形、対照として同じ画像の読み取り専用版が解決することも確認）、②その検査が scheduled 形だけの範囲であること（`scheduled_the_writable_check_is_for_the_scheduled_form_only`）、③隙間と `cmp`〜分岐の間の `call` の拒否（`scheduled_a_call_in_the_gap_or_between_the_cmp_and_the_branch_is_rejected`。変異検査 M17 で足りなかった）、④fe-lift の既定の基底証明で解決される 0093 の隙間（`padded_sites_that_decision_0093_rejected_are_resolved_by_the_scheduled_form_by_default`））。合成 PE は `msvc_common/mod.rs` に `build_sched`（`Sch`、`SchQ`、`SchCmp`、`SchT2`）を追加（第 2 段階で `Img::rdata_writable`、`SchQ::Call`、`Sch::p_call` を足した）。

- **受理する形**: freetype 停止点の形（`Sch::real()`: `cmp eax,0x15; <store>; ja; lea r14; nop; nop; cdqe`、ループの下端の `jbe`、22 エントリ、3 通りの移動先、`entry2` の値）、第 2 の入口の N が大きい／小さい／`jb`、1 入口のスケジュール形（`cmp; <store>; ja; cdqe`、成立側、2 段）、ストア・他のレジスタの `lea`・`nop`・旗を書く命令の混在、`cmp eax`／`cmp edx`＋`movsxd rax,edx`／`movsxd rax,eax`／`cmp rax`、標準形は従来どおり標準形が読む。
- **拒否する形（変異）**: 添字を書く `mov`（32／64 ビット）・`lea`・`mov` のコピー（比較が edx の場合も）、別のレジスタの拡張、`cmp` と分岐の間の書き込み・旗の書き込み、第 2 の入口が `ja`／`jae`（誤った側）、旗が `cmp` に由来しない（間に `call`）、無条件の入口（`jmp head`、`t0` が隙間へ `jmp`）、入口が 3 つ、ガードなし（関数の入口まで遡る）、遡りが 16 命令を超える、基底が書かれない／ImageBase でない／ループの経路で非 `lea` が届く、移動先が別の関数／実行不可／命令の先頭でない、比較専用の基底証明では試さない。
- **ガードの境界・範囲外**: fe-lift の native 突き合わせ（索引 0..=max(N, N2)+3 を native と生成 C で動かし、表と一致。上限ちょうど、上限の次 = 既定の `0xdead`）、ガードを飛ばして idiom の手前へ入る形で索引 N 超が `fle_jumptab_miss`（by name）、`landings_in_swapped_idioms` と `verify_msvc_swapped_entries` の直接テスト（第 2 の入口だけを許す）。
- **実 DLL**: freetype の 11 件（N、異なる移動先の数、`Origin`）と停止点の第 2 の入口（`0x4b168df` → `0x4b16740`）、4 件のループ回転、拒否のまま 5 件の理由、標準形の 17 件は不変（fe-decode）。生成 C で `0x4b1674d` が 10 case の `switch`、`L_4b16740` がラベル、拒否 5 件は `fle_dispatch`（fe-lift）。
- **既存のテストの変更**（実測で確かめた。上の「期待値の更新」）: `SwapForm` の等値の 1 行（`scheduled: false`）、`Origin::Msvc` の網羅的な分解の 1 行（`entry2`）、0093 の 2 テスト（上限と非 `nop`）を比較専用の基底証明（`BaseProof::Dominating`）に付け替えて元の期待を保ち、既定では新しい期待を別のテスト（`scheduled_the_instructions_of_decision_0093_…`）に書いた、Blend2D の期待（`(31, 31, 49)` → `(33, 33, 47)`、解決 site の集合に 2 件、swapped でない site の判定）、dsp `--pdata` の site の集合（6 → 8。2 つのテスト）と `jt_cpu.rs`（`msvc_tables` 6 → 8、`jumptabs` 440 → 442、`jmp:unresolved` 543 → 541、`jmp:msvc_table` 6 → 8）。すべて予測どおりの値。
- **fe-lift の既存のテスト**（第 2 段階で実測して直した）: `msvc_switch.rs` の `real_blend2d_site_is_resolved_by_default`（`(31, 31)` → `(33, 33)`）、`padded_sites_that_are_not_resolved_stay_a_tail_dispatch`（9 件のうち `MOV_EDX_1`、`PAUSE`、`NOP_5,NOP_3,NOP_1` の 3 件は既定で受理に変わる。比較専用の基底証明に付け替え）。`jt_cpu.rs` 436 行の `(438, 4)`（434 個のスタブの種の dsp）は**不変**（`0x37b5cea`、`0x37b6554` はその closure に届かない）。`unresolved_sites_stay_a_tail_dispatch`（`Guard::Missing`）も不変。

## 検証（第 1 段階の記録。実測は上）

**第 1 段階（記録）**: cargo のビルドとテスト、clang、native の実行はしていない。`rustfmt --check --edition 2021`（構文のみ。構文エラーなし）。下の表は第 1 段階の独立のモデルの値（**予測**）で、第 2 段階の実測は上の「第 2 段階の結果」（受理 15 件の移動先の数と集合は全件一致、freetype だけ 2 次効果で +1）。

| 項目 | 結果 |
|---|---|
| 現行（HEAD + H の `fe-lift`）の統計 | freetype（6 モジュールの種あり）: 上のとおり。QFC 6,723 functions／`msvc_rejected` 0、ILTools 12,931／0、FLEngine 59,380／0。Blend2D（種 58 個）: `msvc_tables` 31、`msvc_rejected` 50（種なし 49）。dsp 縮小（6 モジュール構成の種）: 4／4、dsp `--pdata`: 6／6 |
| 独立のモデル（capstone。`work/model.py`。実装ではない）の自己検査 | freetype: 解決済み 17 件が全件「標準形で N が導け、基底証明と移動先が通る」と判定。拒否 16 件のうち 15 件は標準形の理由の種類が `fe-lift` と一致（残り 1 件 `0x4b3dcfe` は `fe-lift` が `no ja` を先に返し、モデルは `.pdata` の外を先に返す順序の差）。Blend2D（種 58 個）: 拒否 50 件のうち 40 件の種類が一致。差の 10 件の内訳: 7 件は `fe-lift` が closure の未復号（`the closure decoded nothing before`）とするところをモデルは線形走査で `no ja` とする（同じ F3 の族。いずれも案では拒否のまま）、1 件（`0x47639ae`）は標準形で N が通り事後の着地の検査で落ちる（モデルは事後の検査を持たない）、2 件（`0x478136e`、`0x48a3479`）は `.pdata` の外。基底証明の失敗 6 件（`0x4745cf4`、`0x4746512`、`0x488be9b`、`0x488e4e0`、`0x4890e43`、`0x489345a`）は文言の指す定義まで一致。モデルが標準形で受理とする site は freetype で 18（実際は 17。`0x4b58f68` は closure が届かない）、Blend2D で 31（実際は標準形の解決 30 + 事後の着地の検査で落ちる `0x47639ae`）。差は closure の到達と事後の検査による |
| 案のモデルの予測 | freetype: **受理 11、拒否のまま 5**（上の表）。Blend2D: **+2**（`0x4756752` = N 28、24 通り、`0x47596c2` = N 5、6 通り）。dsp `--pdata`: **+2**（`0x37b5cea` = N 8、9 通り、`0x37b6554` = N 5、6 通り）。縮小 dsp、QFC、ILTools、FLEngine: **0** |
| 受理した site の検査 | 受理 15 件（freetype 11、Blend2D 2、dsp 2）の表の全エントリが、関数のピースの内側、capstone の線形走査の命令の先頭、`.text` の中。関数の外から非入口へ入る直接分岐（`foreign_entries` 相当）を、freetype の全 `.pdata` 関数、Blend2D の全 `.pdata` 関数、dsp の全 `.pdata` 関数（6,569,501 命令）の線形走査で探して **0 件** |
| 合成 PE のモデル | `msvc_common` の `build_sched` を Python に移し（`work/synpe.py`）、40 通りの構成を同じ規則で判定した: 受理 19（標準形 1、スケジュール形 18）、拒否 21（N の導出 15、基底証明 3、移動先 3）。状態はすべて意図どおり。モデルの文言とテストの期待文字列の食い違いが 10 件あったが、状態は意図どおり（テストの `contains` の文字列は Rust の文言に合わせて書いた）。合成のバイト列（`lea`、`movsxd`、`jbe` の符号化と位置）の誤りを 1 件見つけて直した（`Sch::simple()` の `cmp` と分岐の間に何もなく標準形が読んでしまう形だった）。Rust の実装の動作の確認ではない（同じ作業者が両方を書いた） |

## 他の DLL への影響の予測（第 1 段階。実測との差は上の「解決数」）

| 構成 | 現在 `msvc_tables`／`msvc_rejected` | 予測 | 変わる site |
|---|---|---|---|
| freetype（6 モジュール、種 160） | 17／16 | **28／5** + 2 次効果 | 受理 11 件（上の表）。`0x4b58c24` の表の移動先から `0x4b58f68`（標準形で受理できるが現在は closure に無い）に届けばさらに増える |
| Blend2D（種 58 個） | 31／50 | **33／48** | `0x4756752`、`0x47596c2` |
| Blend2D（種なし `--pdata`） | 31／49 | 33／47 | 同上 |
| dsp 縮小（6 モジュール構成） | 4／4 | 4／4 | なし（拒否 4 件の理由は添字を `shr` で作る形などで不変） |
| dsp `--pdata` | 6／6 | 8／4 | `0x37b5cea`、`0x37b6554` |
| QFC、ILTools | 0／0 | 不変 | なし |
| FLEngine | 0／0 | 不変 | なし。**生成 C は変わらない**見込み（標準形が `Err` を返す site がなく、新しい経路に入らない）。`a4-multi.sh` の既定の `EXPECT_FLE_SHA`（C のファイルの SHA `8789c1a5…`）は更新不要 |

- Blend2D と freetype の**生成 C は変わる**（受理した site の `switch`、届いた新しいコード。Blend2D の closure と functions も増える）。dsp 縮小、QFC、ILTools、FLEngine の C は不変（**予測**）。
- 2 次効果（新しく届くコードの中の `jmp reg`）は予測できない。**第 2 段階で実測する**。

## 安全性の議論（受理を広げる点ごと）

- **生成 C は表の値で飛ぶ先を決めない**: `Select::Target` の `switch` は jmp レジスタの実際の値を case（ImageBase + RVA）と比べ、一致しなければ `fle_jumptab_miss`（by name）で止まる。N が小さすぎても大きすぎても、基底が違っても、**誤った場所へ飛ぶことはなく**、最悪は名前付き停止（0089 の「解釈」）。受理を誤ったときの害は (a) 余分なコード（読んだ表の余りのエントリ）が closure に入る、(b) 必要なエントリが足りず実行時に止まる。(a) は表の全エントリが「同じ関数のピースの中の命令の先頭」であることの検査で抑える。
- **ギャップに何でも許す点**（3）: N の主張は「idiom に着地したとき添字レジスタが `cmp` した値 v ≤ N を持つ」。(i) 添字レジスタを書く命令は 4 の符号拡張だけ（値は変わらない。`cdqe`／`movsxd` は N < 4096 なので符号拡張が恒等）、(ii) すべての入口が `cmp` と符号なしの I ≤ N 側の分岐を通る（入口の列挙は線形走査の直接分岐。`call`、無条件の `jmp` は拒否）、(iii) `cmp` と分岐の間は旗を書かない。他のレジスタ、メモリ、旗（分岐の後）は N と無関係。**基底 B の書き込みは基底証明が見る**（非 `lea` の定義が届けば拒否）。
- **メモリへの書き込み**: 表を**ファイルの像から静的に読む**のは 0089 以来の前提（表は読み取り専用の `.rdata`）。scheduled 形は隙間のストアを許すので、その前提が崩れる形（ストアで表を書き換える自己書き換え）を防ぐために、**PM の決定で、読んだ表のセクションが書き込み不可であることの検査を足した**（決定 9）。残る限界: 書き込み不可のセクションを `VirtualProtect` で書き込み可にして書き換える形、表が `.text` のように書き込み不可のコードセクションにある場合の書き換えは見ない（静的な像の前提）。標準形・swapped 形は隙間のストアを許さないので検査を足していない（3 つの DLL で足しても結果は同じだった）。
- **第 2 の入口**（ループ回転）: 入口ごとに自分の `cmp; 分岐` があり、N は最大。`landings_in_swapped_idioms` は同じ jmp の `(branch, target)` の組の集合に含まれる着地だけを許し、`verify_msvc_swapped_entries` は第 2 の着地点だけを例外にする。それ以外の着地（run の中のほかの命令）は従来どおり拒否。2 個を超える入口は拒否（固定上限）。
- **F2 は通らない**: F2（load の基底と `add` の相手が別のレジスタ）は `table_load(ld, rt, b)` が false で `recognize_msvc_scheduled` も `Ok(None)`（標準形と同じ前半）。swapped の F2 のテスト（0095）は不変。
- **Blend2D の F3 のコピー形は通らない**: 添字を `mov eax,edx` や `lea eax,[r9-1]` で作る形は 4 により `rax is written … not a sign extension` で拒否（**予測**: Blend2D の `N not derived` 41 件のうち受理は 2 件、`.pdata` の外が 2 件（`0x478136e`、`0x48a3479`）、残り 37 件は添字を `mov`／`lea`／`shr` で作る、旗が `cmp` に由来しない、辺の側が違う（`je`、`jne`、`jae` の成立、`jb` の不成立）、無条件の入口のいずれかで、案でも拒否のまま。理由はモデルで確認した）。
- **既存の拒否テストへの影響**: 0093 の「`ja` と idiom の間は `nop` のみ・最大 2 個」を既定の基底証明で確かめていたテストの一部は期待が変わる（上の「テスト」）。これは案の意図した変更（PM の判断点 1）。

## 解釈が分かれた点

1. **「唯一の先行ブロック」→「すべての入口」**: 0095 の swapped は表引きのブロックの先行辺がちょうど 1 本。freetype の停止点は入口が 2 本（ループ回転）で、1 本だけにすると停止点が直らない。案は「全入口が guard を通る」に一般化した（上限 2）。
2. **旗**: 0095 は「`cmp` と分岐の間で I を書かない」。案は「旗を書く命令を許さず、I を書くのは符号拡張だけ」。旗を**読む**命令は許す（分岐の前に旗を読む命令は実在しない見込みだが、禁じていない）。
3. **符号拡張**: `cdqe` に加え `movsxd rD,eS`（D ≠ S でも）を許した。freetype の 6 件が `movsxd rax,edx` で、`cdqe` の一般形だから。`mov` のコピーは許さない（Blend2D の F3 の拡張になるので別の判断）。
4. **N = 最大**: 2 つの入口の上限が違うとき、小さい側の経路にとって余りのエントリは到達しない。表は最大の側で読む。
5. **理由文**: N が導けず拒否のままの site は標準形の理由 + スケジュール形の理由。新しい理由だけにすると既存のテスト／ログが見る前半が消える。
6. **swapped との関係**: `Origin::Msvc::swapped` が `Some` であることは「N を CFG から導いた」という意味になり、swapped 形そのものではない（名前は 0095 のまま。`SwapForm::scheduled` で区別）。
7. **（第 2 段階）隙間の `call`**: 「添字を書かない命令は許す」の読みで、`call` は制御を変える（`FlowControl::Call`）ので許さない。被呼び出し側が `rax`（添字）を書く（戻り値）ので、安全側。隙間と `cmp`〜分岐の間のどちらでも拒否（変異検査 M17、M21 とテスト）。
8. **（第 2 段階）書き込み不可の検査の範囲**: PM の決定 3 は隙間の書き込みを許す scheduled 形に付けた条件と読み、scheduled 形だけに掛けた。標準形・swapped 形にも掛ける選択肢（均一）がある。それを入れた木で freetype、Blend2D、dsp `--pdata` の生成 C を作り直した（変異 M14 の木）ところ、**解決数もファイルの SHA も変わらなかった**（これらの DLL の解決済み site の表は全部書き込み不可のセクション）。掛けるなら掛けても損はないが、それは 0089〜0095 の挙動の変更なので PM の判断。
9. **（第 2 段階）検査の順序**: 添字バイト表を先に見る（2 段の形で、どちらの表が書き込み可能かで理由文が変わるテストのため）。表の読み出しと基底証明のあと、移動先が命令の先頭かの検査の前に置いた（既存の拒否の理由文の順序は変わらない）。
10. **（第 2 段階）ヘッダの中の表**: `SizeOfHeaders` より下の RVA の表はヘッダ（ローダーが読み取り専用で写す）として通す。実在する形ではない。
11. **（第 2 段階）freetype の +1**: `0x4b58f68` は scheduled 形ではなく標準形の switch で、`0x4b58c24` の表を受理した結果 closure に入った。予測の 28 は、この site を「届けばさらに増える」と書いた下限。

## 未決の点(PM)（第 2 段階）

第 1 段階の判断点 1〜6 は PM が依頼文で答えた（上の「PM の決定」）。残る点:

1. **書き込み不可の検査の範囲**（解釈 8）。scheduled 形だけ（今の実装）か、標準形・swapped 形にも掛けるか。掛けても 3 つの DLL の結果は変わらない（実測）。
2. **2 次効果の `0x4b58f68`** のような「新しく届くコード」の site は、これからも増えうる（受理を広げるたびに closure が増える）。個別に確認する運用にするか。
3. **残る 5 件**（`.pdata` の外の葉関数 2、揮発 B が `call` をまたぐ 1、別の間接 `jmp` のワイルドカード 2）は PM の決定どおり触っていない。freetype の次の停止は 6 モジュール構成では未測定（フォントを置いていないため `FT_Stream_ReadFields` に届かない）。
4. **状態（仮）の扱い**: ゲストの実行で受理した switch が表どおりに飛ぶことの確認がまだない（合成 PE のネイティブ照合 5 構成と、生成 C の静的な照合だけ）。実機 Windows との照合は 0 件。
5. **0093 の置き換え**は既定の基底証明でのみ（PM の決定 1）。0093 の「上限を緩める必要が出たら緩めずに報告」の趣旨の変更なので、`docs/investigations/provisional-decisions.md` の 0093 の行との整合は PM が見る。

## 限界

- **実機 Windows との照合は 0 件**。wasm は未確認。ゲスト（6 モジュール構成）で受理した switch を実行した記録は無い（停止点 `0x4b1674f` は、フォントを置いていないので、動いたかどうか確認できない）。コンパイル・テスト・native の生成・突き合わせは第 2 段階で実施済み。
- 第 1 段階の「予測」は capstone を使う Python のモデルで、実装の iced-x86 とは違う（命令の分類、`used_registers` の暗黙の書き込み、`nop` の形）。第 2 段階の突き合わせで受理 15 件は全件一致したが、モデルと実装を同じ作業者（第 1 段階）が書いたため、**独立性は完全ではない**。着地の確認（命令の先頭）は第 2 段階で別に書いた capstone の走査でも行った（違反 0）。
- 2 次効果は freetype で 1 件（`0x4b58f68`）を実測した。新しく届いたコードの `jmp reg` はほかに無い（`register_jmp_sites` 53 → 54、`msvc_rejected` 5 件は不変）。新しい停止点は、ゲストが届かないので未測定。
- 事後検査 `landings_in_swapped_idioms` の範囲は idiom の `lo` から jmp まで。`cmp` から分岐までの間への着地は線形走査の直接分岐だけが見る（0095 の swapped の `cmp`〜分岐の間と同じ限界）。未解決の間接 jmp が実行時に着地する場合は見えない。
- 表のセクションが書き込み不可かは scheduled 形だけが見る（決定 9）。標準形と swapped 形は 0089 以来見ない。
- 揮発レジスタの B が `call` をまたぐ（`0x4b2f423`）、callee-saved が `call` をまたいで保たれる仮定（Win64）は 0089 から変わらない。
- freetype の測定用基底（`0x4b00000`）は決定ではない。実機の Windows での確認なし。

## 撤回条件

- 受理した site の表の移動先が実機（または native 実行）で表と一致しない、または生成 C の `fle_jumptab_miss` が受理した site で実行中に出る（N の導出か基底の誤り）。
- 0093 の隙間の規則を、PM が既定の基底証明でも維持すると決めたとき（判断点 1 の代案へ縮める）。
- 第 2 段階で既存の拒否テスト（F2、Blend2D の F3 のコピー形、swapped の拒否）が受理に変わったとき。
