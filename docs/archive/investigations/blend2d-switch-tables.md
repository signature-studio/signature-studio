# MSVC の switch 表（`jmp reg`）を基底証明つきで解決する（決定 0089）: 95 か所の見積もり、実装、検証、3 モジュールでの停止点の移動（測定のみ）

実行日: 2026-10-24。基準の HEAD は `04f2eed`（作業ツリーの未コミットの変更が本文書の対象）。決定は `docs/decisions/0089-msvc-switch-tables-with-proven-base.md`（PM の決定。仮の決定ではない）。背景は `docs/investigations/blend2d-seed58.md` §6.4、§7、§8 と `docs/investigations/blend2d-lift-feasibility.md` の B3（§3.4、§10.3）。作業物は `lift/out-jtb2d/`（gitignore 対象）。**この文書は何も決めない**。決定 0074、0079、0080、0086 は変えていない。基底 0x4700000 と種 58 個は測定用で、決定ではない。

## 0. 凡例

- **実測** = このセッションでコマンドを実行して得た値。ログは `lift/out-jtb2d/` の下（各節に書く）。
- **読み** = ソース、生成 C、別の文書を読んだ結果。実行して確かめていない。
- **未確認** = そのとおり。

## 1. 要約（先に結論）

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | **決定どおりの実装（基底証明の簡易条件。緩めない）で、Blend2D の停止点 `f_4709ea0` の `0x470a89e: jmp rcx` は解決されない** | 理由: B = r13 への書き込みは `lea r13,[rip+d]`（= ImageBase）1 回だけだが、それは **0x4709f18** にあり、関数の入口ブロック（0x4709ea0〜0x4709efa。`jne 0x4709f07` で終わる）の**外**。N = 31（`cmp eax,0x1f; ja`）、表は 32 バイト + 6 エントリ、全移動先は関数内。**基底以外の条件はすべて満たす**。したがって**停止点は動かない**（`fle: indirect call to 0x470a8a0 is not a known function entry` のまま。3 モジュールの a4 で確認） | 実測（§5、§6.2） |
| 2 | 95 か所（`jmpreg.txt` の `rva-switch`）のうち、決定どおりで解決できるもの | **Blend2D（リベース後 0x4700000、`--pdata`、種なし）で 2 か所**（`0x4701384` と `0x4701cbb`。元の VA は `0x180001384`、`0x180001cbb`。どちらも (c1)、B = r12、各 3 つの異なる移動先）。静的な見積もり（Python + capstone）と実装の結果は **95 か所すべてで一致** | 実測（§3） |
| 3 | 解決されない 93 か所の理由（最初に失敗した条件） | N が導けない 40（うち 33 は `cmp; ja` の組が無い、7 は idiom がブロックの先頭で、ガードが別の経路にある）、基底が証明できない 34（B が複数回書かれる 33、`lea` が入口ブロックの外 1）、形が (c1)(c2) の文字どおりでない 15（`mov`/`add`/`jmp` の間に別の命令が挟まる）、閉包が到達していない 4。**移動先が関数の外のものは 0**（表を読めた (c1)(c2) の 37 か所では 0） | 実測（§3） |
| 4 | FLEngine | 生成 C の SHA-256 は **`43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1` のまま**（HEAD のビルドの出力と `cmp` でバイト一致）。functions 59,297、bad_nodes 0、unsupported_total 525、register_jmp_sites 20、jumptabs 4、`msvc_tables` 0、`msvc_rejected` 0 | 実測（§5.2） |
| 5 | テスト | `cargo test --workspace`: **97 binaries / 1,161 passed / 0 failed / 0 ignored**（基準 95 / 1,130。+2 binaries、+31 件 = fe-decode 22 + fe-lift 9 の新規。既存の期待値を変えた行は 1 か所: `fe-decode/tests/jt_cpu.rs` の dsp_ippv2 の件数、§5.1） | 実測 |
| 6 | **dsp_ippv2 に決定どおりで 1 か所解決された**（`0x1817c69eb`、リベース後 0x37c69eb。(c1)、B = r13、N = 10、11 個の異なる移動先、全部関数内） | 影響: functions 32,648 のまま、closure_instructions +209、`jmp:unresolved` 549 → 548。bad_nodes 0 | 実測（§5.3） |
| 7 | 3 モジュール構成（Blend2D 種 58 個あり、基底 0x4700000、決定どおり） | **a4: init_count 150/942、last unit 149（proc 0x664260）、functions_distinct 1,058、entries 27,316。前回（04f2eed の測定）と同一。停止は同じ `fle: indirect call to 0x470a8a0 is not a known function entry`**（top of stack `f_4709ea0`）。2 回同一。DllMain は戻らず、LoadLibraryW は 0x4700000 を返さない。分類は (v) その他（B3）のまま | 実測（§6.2） |
| 8 | **参考（決定ではない）: 基底証明を「定義が jmp へ到達する全経路の前に実行される」に緩めた測定** | `fe-lift --msvc-base-dominating`（測定専用。既定ではない）では `0x470a89e` が解決され、停止は **`fle: unsupported instruction at 0x4899fed`**（`f_4899d90` の `call [rip+...]` = `GetModuleHandleA`。行なしの import。決定 0087 以前から分かっていた B8）へ移る。ATTACH の import は 172 → 174 回（`GetSystemInfo` が 2 回増える）。**これは PM が決める事項で、実装の既定には入れていない**（§6.3、§7） | 実測 |

## 2. 実装前の読み（コードを読んで整理したこと）

### 2.1 既存の実装が解決できた形と、今回の形の違い（読み）

| | 決定 0036 の FLEngine の MSVC 形（3 か所。注釈 `Spec`） | 決定 0079 のスタブ（434 個） | **今回（決定 0089）** |
|---|---|---|---|
| 認識 | 人が `objdump` で読んで `Spec`（jmp VA、命令バイト、関数の範囲、表、エントリ数）に書く。パターン認識はしない | `movsxd; lea; jmp [B+I*8]` の 3 命令（7 バイトずつ）の形だけ | **形から認識**（`mov eT,[B+I*4+T2]; add rT,B; jmp rT`、あれば `movzx` の索引バイト表） |
| 基底 | `Spec` の `Rel32 { base }` に人が書く（`lea` の結果。r15 の 1 か所は自作の must-constant 解析で人が確認） | スタブの `lea B,[rip+表]` の直前 | **コードが証明する**（関数内の書き込みが `lea B,[rip+d]` = ImageBase の 1 回だけ、入口ブロックの中） |
| 範囲 | 人が `cmp`/`ja` を読んで `count` に書く | `.reloc` の DIR64 の連続 | **`cmp idx,N; ja` から N を導く**。(c2) は索引バイトの最大値 + 1 |
| 表の移動先の検査 | `Spec` の `func` の範囲内 | 実行セクション内 | **`.pdata` の RUNTIME_FUNCTION（連鎖含む）の範囲内**、命令境界 |
| 生成 C | `Select::Target` の `switch`（case は移動先 VA）+ `goto` | `Select::Slot` の `switch` + 関数の tail call | 0036 と**同じ**（`Select::Target`、case は同じ関数内のラベルへの `goto`、既定は `fle_jumptab_miss`） |
| 閉包 | 移動先を**ブロック**として積む | 移動先を**関数**として積む | ブロックとして積む（同じ関数の中） |

読み（コード）: 0036 の `jumptab::resolve` は「jmp の位置の命令バイトが注釈と一致」を検査するだけで、基底レジスタがその時点でその値を持つことは検査しない。`switch` の選択子が「jmp のレジスタの実際の値」なので、基底が想定と違えば `fle_jumptab_miss` で止まるだけで、別の場所へ飛ぶことはない（`fe-lift/src/jumptab.rs` のコメント。0036 の検討した代替の節にも同じ理由）。0089 の基底証明は、この「止まるだけ」の安全性に加えて、**誤った表を作ることを事前に減らす**もの。

### 2.2 既存の `Resolver` は今回の形を解かない（読み + 実測）

`fe-decode/src/indirect.rs` の `Resolver` は `lea reg,[rip+X]` / `mov reg,imm` / `mov reg,[rip+X]` の**直前 1 命令**だけを見る。`jmp rcx` の直前は `add rcx,r13` で、定義の形に当たらない。`dataflow.rs` は run 内の定数評価をするが、表の中身（`Mem(c)`、8 バイトのファイル内容）を足し算で保てない（`Mem` は load だけが作る）。実測: HEAD のビルドの Blend2D（`--pdata`）は `jumptabs 0`、`register_jmps` のうち `jmp_ind:reg_dispatch` 99 で、MSVC 形はすべて `fle_dispatch(R[x]); return;` だった。

## 3. 95 か所の分類（実装前の見積もり。静的解析、そして実装との照合）

### 3.1 方法（実測）

`lift/out-jtb2d/analysis/pre_survey3.py`（capstone + 自前の `.pdata` パーサ。fe-decode も pefile の `.pdata` も使わない。実装前に書いた）。入力はリベース済み Blend2D（`lift/out-seed58/s58/dll/Blend2D.4700000.dll`、SHA-256 `24e01ec9…b4cb`）と `lift/out-blend2d/jmpreg.txt`（元の VA。`− 0x180000000 + 0x4700000` で変換。97 行 = `rva-switch` 95 + `other` 2）。`.pdata` は 2,493 エントリ、連鎖の主関数 1,843。ログ: `lift/out-jtb2d/logs/pre3_b2d.txt`、`.csv`。形の内訳の補助: `between.py`（`between_b2d.txt`）、`ctx_sites.py`（`ctx_all_b2d.txt`）。

その後、実装した fe-lift（`--pdata`、種なし）の `msvc_rejected` の行と生成 C を、`join95.py` で 95 か所に結合し、静的な見積もりと突き合わせた（`join95.txt`）。**95 か所すべてで一致**（違いが出た 4 か所は、`jmp rax` が閉包に到達していないだけで、解析の違いではなかった。`join95.py` が「閉包が到達していない」を別の類にした後は不一致 0）。

### 3.2 内訳（実測。リベース後 0x4700000、`--pdata` のみ、種なし）

| 最初に失敗した条件 | か所 | 内容 |
|---|---|---|
| (a)–(d) すべて満たす → **解決** | **2** | `0x4701384`（`f_4701310`。c1、B = r12、N = 7、8 エントリ、異なる移動先 3）、`0x4701cbb`（`f_4701bb0`。c1、B = r12、N = 6、7 エントリ、異なる移動先 3）。どちらも `cmp; ja; cdqe; mov; add; jmp` で、`lea r12,[rip-…]` は入口ブロック内、r12 への書き込みはこの 1 回だけ |
| 形: (c1)(c2) の文字どおりでない | 15 | `mov eT,[B+I*4+T]` と `add rT,B` の間、または `add` と `jmp` の間に別の命令が入る。内訳（`mov`/`add`/`jmp` の間に挟まる命令）: `movaps` 8（`mov; movaps xmm; add; jmp`）、`mov` を挟む 2、`lea` を挟む 2（`mov; lea; add; jmp`）、`add; lea; lea` 1、`mov; add; xor; jmp` 1、`lea; add; mov; jmp` 1（`0x1800195fa`。調査 §3.4 の `rva-switch` 95 に入っているが、`add rT,B` の直後に `mov` が入るので、決定の形ではない）。**決定の (c1)(c2) は「直前の命令列」が `mov; add; jmp` の隣接**なので、これらは解決しない（決定の文言どおり。解釈が分かれる点として §8 に挙げる） |
| N が導けない | 40 | 33 か所: idiom の前に `cmp idx,imm; ja` の組が無い（ガードが別の場所にある、`cmp reg,reg`、`lea eax,[r-K]; cmp eax,imm` の後に `ja` が来ない、`sub` の後の `jae`/`jb` など）。7 か所: idiom の最初の命令がブロックの先頭（`jmp`/`ja` の移動先）で、ガードは別の経路にある（`mov ecx,[r+rax*4+T]` が直前に分岐の着地点になっている形） |
| 基底が証明できない | 34 | 33 か所: B への書き込みが複数回（`lea B,[rip+d]` が 1 回あっても、他の用途で B を別の値にする。最大は rdx の 407 回）。1 か所: `lea r13,[ImageBase]` が 1 回だけだが入口ブロックの外 = **停止点の `0x470a89e`** |
| 移動先が関数の外 | **0** | (c1)(c2) の形で、N を導いた 37 か所（静的な見積もり `pre3_b2d.txt` の「N loose」。実装が N を導いた 36 か所 + 閉包に未到達の 1 か所）の全エントリが同じ関数のピースの内側（実測: `pre3_b2d.txt` の `targets all inside the function (N loose): 37 of 37`）。参考（`targets_in_fn.txt`）: N を導かずに `jmpreg.txt` のエントリ数で表を読むと、2 か所が「関数の外」と出る（`0x4723238` の 13 エントリ目 → `0x47b0000`、`0x476cf0a` の 4 エントリ → `0x4710000`、`0x4710101`）。どちらも**エントリ数を超えて読んだ**ため（`jmpreg.txt` のエントリ数は「.text の RVA である間」の長さで、(c2) の 2 段では索引バイトの最大値 + 1 より長くなりうる）で、決定の導き方（N から）では読まれない |
| 閉包が到達していない | 4 | `0x4745efd`、`0x474c382`、`0x474c5b9`、`0x4752623`（すべて `jmp rax`、B = r12/r14 など）。`--pdata` の閉包に入っていない（直接呼び出しで届かない）。種 58 個ありでも同じ（実測 `join95_seeds58.txt`）。判断されない |
| 計 | 95 | |

分類は「**最初に失敗した条件**」の順（形 → N → 基底 → 移動先）。基底と N を独立に数えると、(c1)(c2) の 80 か所のうち、N が導けるのは 37 か所（決定の隣接規則 + `cdqe`/`lea B` の許容）で、そのうち基底が証明できるのは 2 か所。

### 3.3 形の内訳（実測）

| 項目 | か所 |
|---|---|
| (c1)（`mov; add; jmp`。1 段） | 53 |
| (c2)（`movzx byte; mov; add; jmp`。2 段） | 27 |
| どちらでもない（上の「形」の 15） | 15 |
| (c1)(c2) の 80 のうち B が rdx | 18 |
| 同 r8 | 16 |
| 同 rcx | 7 |
| 同 r13 | 6 |
| 同 r12 | 5 |
| 同 r10 | 6 |
| 同 r9 | 5 |
| 同 rdi 5、r14 3、r11 3、rsi 3、rbx 2、r15 1 | |

基底が r13（停止点と同じ）の (c2) は 4 か所あるが、決定どおりで解決できるものはない（`0x470a89e` は入口ブロックの外、`0x4722765` は r13 を 2 回書く）。

### 3.4 参考: 「基底の定義が jmp の直前の同じブロックにある」形（実測。決定ではない。実装していない）

`lift/out-jtb2d/analysis/local_lea.py`（`local_lea.txt`）。(c1)(c2) の 80 か所のうち、idiom の手前の最寄りの B への書き込みが `lea B,[rip+d]` = ImageBase で、そこから jmp までにブロックの先頭が無いものは **38 か所**。そのうち N が導けて全移動先が関数内のものは **20 か所**（`0x470749d`、`0x4707d3a`、`0x470b983`、`0x470d019`、`0x4725144`、`0x4725db7`、`0x4728561`、`0x47290af`、`0x4746512`、`0x4760a33`、`0x476b168`、`0x476cf0a`、`0x478129c`、`0x47aa0bc`、`0x48654a6`、`0x48712cf`、`0x488be9b`、`0x488e4e0`、`0x4890e43`、`0x489345a`）。そのうち揮発レジスタ（rax rcx rdx r8–r11）が B のものが 19 か所。つまり **MSVC は switch の直前で `lea B,[ImageBase]` を出し直す形が多く、決定の「入口ブロックに 1 回だけ」の簡易条件は、この形を意図的に解決しない**（`lea` が関数内に複数あると「B が複数回書かれる」になる）。これらを解くには、基底証明の条件を変える PM の判断が要る。**この数は見積もりで、これらの 20 か所が実際に正しい表であることの確認はしていない**（移動先が関数内と、命令境界に当たることだけを見た）。

## 4. 実装（読み、変更ファイル）

決定 0089 の「実装した場所」の節にも同じ内容を書いた。要点:

- `lift/crates/fe-decode/src/jumptab.rs`（+620 行）: `recognize_msvc`、`FuncMap`、`Sweep`、`prove_base`、`resolve_msvc`、`verify_msvc`、`foreign_entries`、`landings_in_idioms`、`Origin::Msvc`、`BaseProof`、`Options`。
- `lift/crates/fe-decode/src/lib.rs`（+99 行）: `analyze_with_options`、未解決の `jmp reg` への `recognize_msvc` の適用、閉包の完成後の検証、`Closure::msvc_tables`、`Closure::msvc_rejected`。
- `lift/crates/fe-lift/src/jumptab.rs`（+4 行）、`lib.rs`（+19 行）、`main.rs`（+16 行）: ハンドラ名、`generate_with_options`、統計と `--msvc-base-dominating`。
- `lift/runtime` は編集していない。`fle_jumptab_miss` は既存。

### 4.1 決定の条件との対応、決定に足した条件（明示）

| 決定 | 実装 |
|---|---|
| 1(a) 基底: 関数内の B への書き込みが `lea B,[rip+disp]`（= ImageBase）1 回だけ、入口ブロックにある。push/pop は書き込みに数えない | `prove_base`。書き込みは iced-x86 の `used_registers` の write 系（暗黙を含む）。`push B` は数えず、`pop B` は**同じ関数に `push B` がある場合だけ**数えない（無ければ書き込み）。入口ブロック = 入口から最初の非フォールスルー命令まで（call は終わらせない）。関数 = 連鎖したピースすべて |
| 1(b) N | `recognize_msvc`: `cmp idx,imm; ja`（`cmp` は idx の 32/64 ビット）。決定は「`sub idx,K; cmp idx,N; ja`」も挙げるが、`sub` は `cmp` の前にあるだけで N は `cmp` の即値なので、**`sub` の有無は N の導出に影響しない**（`cmp` が比べるレジスタが表の添字と同じであることだけ見る）。`ja` と idiom の間は `cdqe`（rax のとき）と `lea B,[rip+d]` だけ許す（最大 2 個）。それ以外は N を導かない |
| 1(c1)(c2) 表 | 同上。`mov eT,[B+I*4+T2]; add rT,B; jmp rT`、あれば `movzx eX,byte [B+I0+T1]`。(c2) の RVA 表のエントリ数 = N+1 バイトの最大値 + 1 |
| 1(d) 移動先 | `resolve_msvc`: B+RVA が同じ関数のピース内かつ実行セクション内、線形走査の命令開始、1 つでも外なら site ごと拒否（外のエントリを列挙） |
| 2 生成 C | 既存の `jumptab::lift`（`Select::Target`） |
| **決定に足した条件（4 つ）** | (i) B が揮発レジスタで関数内に call があれば証明しない（呼び先が B を変えうる）。(ii) `verify_msvc`: ガードの `cmp; ja` を閉包がその形でデコードしたこと、`ja` の次から jmp の間にブロックの先頭が無いこと、jmp が関数の入口の関数に属すこと。(iii) `foreign_entries`: 関数の外の分岐が入口以外へ飛び込むなら、その関数の表を捨てる（入口ブロックの `lea` を通らずに jmp へ来うる）。(iv) `landings_in_idioms`: ガードを飛ばして idiom に着地する分岐があれば捨てる。**いずれも解決を減らす方向の条件（緩めない）**。実測では (i)–(iv) のどれも、Blend2D、dsp_ippv2、FLEngine で解決を落とさなかった（落ちたものは条件 (a)(b) で先に落ちる） |
| 3 FLEngine | §5.2 |
| 4 種 58 個に依存しない | `--pdata` のみ（種なし）で 2 か所解決（§3.2）。種 58 個ありでも同じ 2 か所（§5.3） |

## 5. 検証（実測）

### 5.1 テスト（`cargo test --workspace`）

```
cd lift && CARGO_TARGET_DIR=<repo>/lift/out-jtb2d/target FLENGINE_PE=<repo>/binaries/win64/FLEngine_x64.dll cargo test --workspace --no-fail-fast > out-jtb2d/test-ws.txt 2>&1
```

終了コード 0。結果行 **97 binaries / 1,161 passed / 0 failed / 0 ignored**（基準 95 / 1,130 / 0 / 0。**+2 binaries、+31 件**）。内訳の増分 = 新規の 2 binaries: `fe-decode/tests/msvc_switch.rs` **22 件**、`fe-lift/tests/msvc_switch.rs` **9 件**。ログ `lift/out-jtb2d/test-ws.txt`。他の作業の target は使っていない（`CARGO_TARGET_DIR=lift/out-jtb2d/target`）。

**既存テストの期待値を変えた行（1 か所）**: `lift/crates/fe-decode/tests/jt_cpu.rs` の `dsp_ippv2_has_434_stubs`（実 DLL。`lift/out-ext/dsp_ippv2_x64.dll`）。理由: この DLL に決定どおりで解決される MSVC switch が 1 か所あった（`0x1817c69eb`、`f_1817c6990`。§5.3）。変更前の `jumptabs == 434`、`jmp:unresolved == 549`、`memory_form - a.jumptabs.len() == 183`:
- `jumptabs` 434 → **435**（`msvc_tables == 1` を足して表明）。
- スタブの内訳の集計（`by_var`、`by_len`）は `Origin::Msvc` を除いて数える（434 のまま、値は変えていない）。
- `jmp:unresolved` 549 → **548**、`jmp:msvc_table == 1` を表明。
- `memory_form - a.jumptabs.len() == 183` → `memory_form - a.closure.stub_tables == 183`（式を `stub_tables` に変えただけで、183 は変えていない。MSVC の表は register 形で `memory_form` に入らないため、`jumptabs.len()` から引く対象はスタブだけ）。

ほかの既存の表明（FLEngine の `jumptabs == 4`、`stub_tables == 0`、`jmpreg` 系、`tests/jumptab.rs`、`tests/jt_cpu.rs` の fe-lift 側）は変更なしで通った。

### 5.2 FLEngine の生成 C

`fe-lift lift binaries/win64/FLEngine_x64.dll OUT.c --pdata`（HEAD の `git archive` から作った release ビルドと、変更後の release ビルドと、変更後の debug ビルド）。

| 項目 | HEAD | 変更後 |
|---|---|---|
| SHA-256 | `43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1` | **同じ**（`cmp` でバイト一致。215,707,512 B） |
| functions | 59,297 | 59,297 |
| closure_instructions | 2,528,030 | 2,528,030 |
| bad_nodes | 0 | 0 |
| unsupported_total | 525 | 525 |
| register_jmp_sites | 20 | 20 |
| jumptabs | 4 | 4 |
| msvc_tables / msvc_rejected | — | 0 / 0 |
| c_lines / c_bytes / gotos / tail_calls | 2,885,412 / 215,707,512 / 203,462 / 610 | 同じ |

FLEngine の MSVC 形 3 か所（0x5e5eb6、0x5e9ae8、0x5f1499）は決定 0036 の注釈（`Spec`）が先に解決するため、新しい認識には回らない（`msvc_rejected` が 0 なのはそのため。`recognize_msvc` は `Spec` に当たらない `jmp reg` にだけ働く）。**変わった関数は 0**。

a4（単独構成）は、FLEngine の生成 C が変わらず、runtime も変えていないので**省略した**（依頼のとおり）。

### 5.3 DLL ごとの統計の前後（`fe-lift lift ... /dev/null`。release、HEAD の `git archive` のビルド → 変更後のビルド）

| DLL / 引数 | 項目 | 変更前 | 変更後 |
|---|---|---|---|
| **Blend2D**（0x4700000）`--pdata`（種なし） | functions | 2,018 | 2,018 |
| | closure_instructions | 398,879 | 398,980（+101） |
| | bad_nodes | 0 | **0** |
| | unsupported_total | 662 | 662 |
| | register_jmp_sites | 99 | 99 |
| | jumptabs（msvc_tables） | 0 | **2**（2） |
| | `jmp_ind:reg_dispatch` | 99 | 97 |
| | gotos / c_lines / c_bytes | 31,879 / 411,794 / 34,691,060 | 31,908 / 411,895 / 34,699,913 |
| Blend2D `--pdata --seeds seeds58-4700000.txt`（種 58 個あり） | functions | 2,066 | 2,066 |
| | closure_instructions | 399,508 | 399,609（+101） |
| | bad_nodes | 0 | **0** |
| | unsupported_total | 665 | 665 |
| | register_jmp_sites | 100 | 100 |
| | jumptabs（msvc_tables） | 0 | **2**（2。種なしと同じ 2 か所） |
| | `jmp_ind:reg_dispatch` | 100 | 98 |
| **dsp_ippv2**（0x2000000）`--pdata` | functions | 32,648 | 32,648 |
| | closure_instructions | 6,464,799 | 6,465,008（+209） |
| | bad_nodes | 0 | **0** |
| | unsupported_total | 974,527 | 974,527 |
| | register_jmp_sites | 836 | 836 |
| | jumptabs（msvc_tables） | 434 | **435**（1） |
| | `jmp_ind:sel_table` | 434 | 434 |
| | `jmp_ind:reg_dispatch` | 366 | 365 |
| dsp_ippv2 縮小（`--seeds seeds-idx0-plus-initterm2.txt`） | functions / closure_instructions / bad_nodes / unsupported_total | 8,030 / 3,038,781 / 0 / 391,396 | 同じ |
| | register_jmp_sites / jumptabs / msvc_tables | 563 / 434 / — | 563 / 434 / 0 |
| **QFC**（0x400000）`--pdata` | functions / closure_instructions / bad_nodes / unsupported_total | 6,712 / 209,259 / 0 / 104 | 同じ |
| | register_jmp_sites / jumptabs / msvc_tables | 5 / 0 / — | 5 / 0 / 0 |

**`bad_nodes` はどこも増えなかった**。解決した表の移動先は、`recognize`/`resolve` が命令境界であることを確認した上で閉包に積む（`verify` が通らなければ表を捨てる）。`unsupported_total` は Blend2D と dsp_ippv2 の両方で**変わらなかった**（新しく閉包に入った 101 / 209 命令の中に未対応命令は無かった）。

統計の出力に `msvc_tables` と `msvc_rejected`（件数と 1 件ずつの jmp VA と理由）を追加した。拒否の件数（変更後）: Blend2D 種なし 77、種 58 個あり 78、dsp_ippv2 10、dsp 縮小 7、QFC 0、FLEngine 0。**Blend2D の種 58 個ありで拒否が 1 件増える**（`0x478136e`。N が導けない。種なしの閉包には入らず、種ありで入る。どの種から届くかは調べていない）。

**`jmpreg.txt` の 95 か所の外にある拒否が Blend2D に 3 件ある**（種 58 個ありでは +1 で 4 件。上の「+1」の `0x478136e`）: `0x4724226` と `0x472442b`（`add rcx,r8; jmp rcx`）は `msvc_rejected` が「`the jump is not inside a .pdata function`」。`0x48a3479` は「N が導けない」。実測した事実: (1) `0x4724226`、`0x472442b` は `.pdata` のエントリが無い範囲 `[0x47241a1, 0x4724690)`（前のエントリの終わりと次のエントリの始まりの間）の中、`0x48a3479` も `[0x48a3456, 0x48a353c)` の中にある。(2) 3 か所とも、生成 C の関数（`f_47241b0`、`f_48a3460`。**どちらも種 58 個に入っていない**。種なしの `--pdata` の生成 C にも入っている）の中にあり、その範囲を capstone で線形走査すると、`[0x47241a1, 0x4724690)` は 217 命令（`ret` 5 個）、`[0x48a3456, 0x48a353c)` は 81 命令（`ret` 6 個）できれいに切れる。(3) `bad_nodes` は 0。**読み（未確認）**: `.pdata` に載っていない葉関数（スタックを動かさない関数は unwind 情報を持たない。調査 §3 の「`.pdata` に無い葉関数」と同じ種類）に当たり、`jmp reg` はそこにある switch。決定の (d)「同じ `.pdata` の RUNTIME_FUNCTION」を満たせないので、解決しない（決定どおり）。これらの switch が解ければ移動先は同じ葉関数の中だが、決定の条件では扱えない。

### 5.4 解決した site の全エントリの個別照合（objdump / capstone）

`lift/out-jtb2d/analysis/verify_targets.py`（fe-decode / fe-lift に依存しない。生成 C の `switch` を読み、capstone で idiom と表をもう一度読み直し、各移動先が (1) 関数のピースの中、(2) 線形走査の命令開始、(3) `objdump -d -M intel --start-address=T` の最初の行がちょうど T、であることを調べる）。結果（ログ `verify_targets_b2d.txt`、`verify_targets_dsp.txt`、`verify_targets_b2d_dom.txt`）:

| image / site | 関数 | B / N | エントリ → 異なる移動先 | case 数（生成 C） | 結果 |
|---|---|---|---|---|---|
| Blend2D `0x4701384` | `0x4701310` | r12 / 7 | 8 → 3（`0x4701386`、`0x4701393`、`0x47013a0`） | 3 | 表と case が一致、3 件とも命令の先頭 |
| Blend2D `0x4701cbb` | `0x4701bb0` | r12 / 6 | 7 → 3（`0x4701cbd`、`0x4701ce3`、`0x4701d65`） | 3 | 同上 |
| dsp_ippv2 `0x37c69eb`（元 `0x1817c69eb`） | `0x37c6990` | r13 / 10 | 11 → 11 | 11 | 同上（11 件） |
| （参考。決定ではない）Blend2D `0x470a89e` | `0x4709ea0` | r13 / 31 | 6 → 6（`0x470a8a0`、`0x470a8af`、`0x470aa40`、`0x470aa6b`、`0x470aab1`、`0x470aafc`） | 6 | 同上（`--msvc-base-dominating` のときだけ） |

照合した distinct な移動先は 決定どおり 6 + 11 = **17 件、すべて命令の先頭**（失敗 0）、参考の変種で +6 件（失敗 0）。**範囲外のエントリは、解決した site にも拒否した site にも 0 件**（拒否 93 か所の理由は §3.2。「移動先が関数の外」で拒否したものは実 DLL に 0 件。合成 PE のテストでは、別の関数の開始、実行セクション外、命令境界でない、の 3 つが拒否されることを確認した: §5.5）。

### 5.5 合成バイト列のテスト（`fe-decode/tests/msvc_switch.rs` 22 件、`fe-lift/tests/msvc_switch.rs` 9 件）

合成 PE は `fe-decode/tests/msvc_common/mod.rs`（小さなアセンブラ + 4 セクション: `.text`、`.rdata`、`.xdata`、`.pdata`）。関数は `[push B;] lea B,[ImageBase]; mov eax,ecx; [sub eax,K;] cmp eax,N; ja def; [cdqe;] [movzx eax,byte [rax+B+T1];] mov ecx,[B+rax*4+T2]; add rcx,B; jmp rcx` と 3 つの移動先ブロック。

**解決されること**（`fe-decode`）: (c1)（4 エントリ、異なる 3）、(c2)（索引バイト 6 個、RVA 表は max+1 = 3 エントリ。4 つ目のエントリがファイルにあるが読まれない）、`cmp rax`、`cdqe`、`sub eax,K` つき、`push/pop B` あり・なし、call を含む関数での非揮発 B（r12、r13）、call の無い関数での揮発 B（r9、r10）、連鎖した RUNTIME_FUNCTION のピースにある移動先。

**解決されてはいけないもの**（`fe-decode`。いずれも jmp が `Unresolved` のまま、`msvc_rejected` に理由、`jumptabs` 0）:

| ケース | 確認した理由の文字列 |
|---|---|
| `lea B,[ImageBase]` が入口ブロックの外（`test; je` の後） | `is not in the entry block` |
| B が 2 回書かれる（`lea` の後の `xor`） | `is written 2 times` |
| `pop B` だけで `push B` が無い | `is written 2 times` |
| B が引数から来る（`mov B,rcx`） | `is not lea ...` |
| `lea` の値が ImageBase でない（+0x10、−0x1000） | `is not lea ...` |
| B が揮発（r8、r11）で関数に call がある | `caller-saved` |
| ガードが無い | `N not derived` |
| ガードが別のレジスタ（`cmp edx,N`） | `N not derived` |
| ガードを飛ばして idiom に着地する分岐 | `inside the switch idiom` |
| 別の関数が途中（`t0`）へ jmp する | `outside the function` |
| 移動先が別の関数の開始 | `target outside the function: 1 of 3 distinct target(s)`、メッセージに別の関数の開始の VA |
| 移動先が `.rdata`（実行セクション外） | `in no .pdata function, not executable` |
| 移動先が命令の途中（命令の 1 バイト目の次） | `not an instruction start` |
| 例外ディレクトリが無い | `not inside a .pdata function` |

`the_dispatch_form_is_unchanged_for_rejected_sites`: 拒否した site は従来どおり `register_jumps` 1 に数えられ、`fle_dispatch(R[1]); return;` のまま、表の移動先ブロック（t1、t2）は閉包に入らない。

**native との突き合わせ**（`fe-lift/tests/msvc_switch.rs`。clang -O0、`harness/image.c` で ImageBase に写して本物の命令を ms_abi で呼ぶ）: (c1)、(c2)、`cdqe` + `cmp rax` + call、`sub eax,5` つきの (c1)(c2)、それぞれ索引 0..N+K+3 のすべてで、native と生成 C の戻り値が同じで、表のバイトが指すブロックの値と一致（N を超える索引は元のコードの `ja` が既定ブロックへ行く）。ガードを飛ばして B を ImageBase でない値にして idiom に入ると、生成 C は `fle: jump table at 0x… : selector 0x… is outside the table`（`fle_jumptab_miss`）で止まり、標準出力に何も出さない（c1、c2）。生成 C の形: 3 つの `case UINT64_C(VA): goto L_VA;`、`fle_jumptab_miss(UINT64_C(jmp), t)` が 1 つ、`uint64_t t = R[1]; switch (t)`（選択子は jmp のレジスタ）、関数内に `fle_dispatch(R[` が無い。未解決の 5 種（`AfterBranch`、`Twice`、`FromArg`、ガードなし、別の関数への移動先）は `fle_dispatch(R[1]); return;` のままで `fle_jumptab_miss` が出ない。

### 5.6 実 DLL: Blend2D `f_4709ea0` の site `0x470a89e`（元 `0x18000a89e`）

`fe-lift/tests/msvc_switch.rs`（`BLEND2D_PE`、既定 `lift/out-seed58/s58/dll/Blend2D.4700000.dll`）。

1. **決定どおり（既定）**: **解決されない**。`msvc_rejected`: `` `lea R13,[ImageBase]` at 0x4709f18 is not in the entry block of 0x4709ea0 (the block ends at 0x4709efa) ``。生成 C は `L_470a89e: { /* 0x470a89e Jmp */ { fle_dispatch(R[1]); return; }}` のまま（テストが文字列で確認）。同じ関数の中で N = 31（`cmp eax,0x1f` は 0x470a881）、索引バイト表 `0x470ab8c`（32 バイト）、RVA 表 `0x470ab74`（索引バイトの最大値 5 → 6 エントリ）は決定の条件 (b)(c)(d) を満たす（`--msvc-base-dominating` の測定で確認）。
2. **表の 6 エントリ**（ファイルのバイトから読んだ。テストが生成器を使わず idiom の命令バイト 4 行を確認した上で読む）: `0x470a8a0`、`0x470a8af`、`0x470aa40`、`0x470aa6b`、`0x470aab1`、`0x470aafc` = 元の VA `0x18000a8a0`、`0x18000a8af`、`0x18000aa40`、`0x18000aa6b`、`0x18000aab1`、`0x18000aafc`（決定の依頼の値と一致）。索引バイト 32 個: `[0,1,2,2,2,5,5,5,3,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,4]`（索引 0→`0x470a8a0`、1→`0x470a8af`、2〜4→`0x470aa40`、5〜7→`0x470aafc`、8→`0x470aa6b`、9〜30→`0x470aafc`、31→`0x470aab1`）。索引バイト表の次のバイト（0x470abac）は 0xcc で、次の関数との間のパディング。
3. **参考（決定ではない）`BaseProof::Dominating`**: 解決される。全 33 通りの索引（0..32）とそれ以上の値（33、34、63、100、0xff、0x80000000、0xffffffff）の計 40 通りを、`0x470a881`（`cmp eax,0x1f`）から入り、native（本物の命令。6 つの移動先に `0xCC`、SIGTRAP ハンドラ）と生成 C（6 つのラベルに到達の印）の両方で実行した（計 80 回）。**到達先はすべて、ファイルの索引バイトと RVA 表から読んだ値と一致**（索引 0..31 は表どおり、32 以上は `ja` の `0x470aafc`）。索引 32 は表の外（`cmp eax,0x1f; ja` が既定へ）で、`0x470aafc`。

## 6. 3 モジュール構成の測定（測定のみ。gdb の仮の値なし、runtime の応答追加なし）

### 6.1 手順（実測）

`lift/out-seed58/harness-two/` を `lift/out-jtb2d/three/harness-two/` にコピー（変更なし）。作業コピー（HEAD + この変更）の release の `fe-lift`、`fe-pe`（`lift/out-jtb2d/target-after/release`）で作り直した（`gen_two.sh rebase gen split merge`、`build_two_native.sh`、`MODULES` と引数は `blend2d-seed58.md` §2 と同じ。`--pdata --seeds seeds58-4700000.txt`）。リベース済み DLL は `lift/out-seed58/s58/dll/` のものを使い、SHA-256 が `4fd25502…efc5`（dsp_ippv2）、`24e01ec9…b4cb`（Blend2D）と一致。FLEngine の生成 C は `43985d62…37137a1`（`gen_two.sh` の検査が一致と表示）。runtime、harness は変えていないので、runtime のオブジェクトは `out-seed58/s58/native/` からコピーして再利用した。

| TAG | 内容 | Blend2D の生成 C |
|---|---|---|
| `b2d58` | **決定どおり**（種 58 個あり） | 2,066 関数、399,616 命令、bad_nodes 0、SHA-256 `84a0dacaecd581452e5bd299f7ace5171403d7f3b0e14f489bf66a2f3b3d46e9`（前回 `3dccba70…d5cd` から変わった関数は **2 つ**: `f_4701310`、`f_4701bb0`。関数の増減は 0、関数外のテキストは同一） |
| `b2d0` | 決定どおり、種なし（任意の測定） | 2,018 関数 |
| `b2d58dom` | 参考（決定ではない）: `--msvc-base-dominating` | 2,066 関数、`0x470a89e` が `switch` |

表の統合: b2d58 は 59,557 + 8,290 + 2,326 → 69,653 行（前回と同じ。複数の表に在る行 260、`f_<VA>` の衝突 0）。`two-native` 612,258,040 B（前回 612,248,328 B。表の変更でなく、生成 C の `switch` の分）。コンパイルの警告 0。2 GB 未満。

### 6.2 a4 の行（決定どおり、種 58 個あり。2 回: 11:07:59Z、11:08:05Z。gdb なし。ログ `lift/out-jtb2d/three-out/a4/`）

```
stop   : fle: indirect call to 0x470a8a0 is not a known function entry
         top of stack: 0x4709ea0 f_4709ea0
depth  : init_count=150 / 942   last unit: index=149 run_rank=137/895 proc=0x664260 in_progress
         started=137 completed=136 in_progress=1 of 895 runnable   all_units_done=no
         entry check: started_but_not_entered=0 runnable_entered_beyond_init_count=0
funcs  : executed (distinct)=1058  entries=27316
```

- **前回（`blend2d-seed58.md` §6.1）と a4 の全行が同一**: init_count 150/942、last unit 149（proc 0x664260、in_progress）、started 137 / completed 136、functions_distinct **1,058**、function_entries **27,316**、停止 `fle: indirect call to 0x470a8a0 is not a known function entry`、top of stack `f_4709ea0`。ハーネスの出力 `abort kind: fle_dispatch_miss`、`dispatch target: 0x470a8a0`、`call depth at stop: 19`。2 回の stderr は `cmp` で一致、stdout は `rsp=` を除いて一致。
- **C のスタックの先頭 10 行**（`call depth at stop: 19`）は前回と同じ:

```
#0 0x4709ea0 f_4709ea0 guest-return=0x470acab
#1 0x470abb0 f_470abb0 guest-return=0x4899dd3
#2 0x4899d90 f_4899d90 guest-return=0x4701009
#3 0x4701000 f_4701000 guest-return=0x48a59eb
#4 0x48b5b90 f_48b5b90 guest-return=0x48a59eb
#5 0x48a59a8 f_48a59a8 guest-return=0x489e34e
#6 0x489e2b4 f_489e2b4 guest-return=0x489e4c4
#7 0x489e264 f_489e264 guest-return=0x489e4c4
#8 0x489e450 f_489e450 guest-return=0xcb000000
#9 0x489e584 f_489e584 guest-return=0xcb000000
```

- 全体のスタック（読み取りだけの gdb の `bt`。`lift/out-jtb2d/gdb/ro_imports.py` は `out-seed58/gdb/ro_imports_s58b.py` のコピー。`return`/`set var`/`call` なし。ログ `run/b2d58-imports.txt`）: 前回と同じ形（`f_4709ea0` ← `f_470abb0` ← `f_4899d90` ← `f_4701000` ← `fle_dispatch` ← `f_48b5b90` ← `f_48a59a8` ← `f_489e2b4` ← `f_489e264` ← `f_489e450` ← `f_489e584` ← `call_dllmain(idx=1)` ← `fle_realmod_load` ← `fle_imp_LoadLibraryW` ← `f_423990` ← `f_44db40` ← `f_6539e0` ← `f_664190` ← `f_664260`）。
- **Blend2D の DllMain(ATTACH) は戻ったか**: **戻っていない**（`DLLMAIN-RETURN reason=1: out of scope (did not return normally)`。スタックに `f_489e584` が残る）。**LoadLibraryW は 0x4700000 を返したか**: **返していない**（`LOADLIBRARY-RETURN` は 1 回だけで `R[0]=0x2000000`（dsp_ippv2））。
- **ATTACH 内の import の列**（`BLEND2D DllMain entered` の後の番号つき `fle_imp_*` 行）: **172 回**（通し番号 1526〜1697）。前回と同じ。名前ごとの回数も前回と同じ（`InitializeCriticalSectionAndSpinCount` 83、`GetProcAddress` 12、`HeapAlloc` 8、`EnterCriticalSection` 7、`LeaveCriticalSection` 7、`WideCharToMultiByte` 6、`MultiByteToWideChar` 6、`LoadLibraryExW` 6、`LCMapStringW` 4、…、`GetModuleHandleW` 1）。名前付き停止の import なし。
- **停止点の分類**: **(v) その他（B3）のまま**。`blend2d-seed58.md` §7.1 と同じ根拠（`f_4709ea0` の `jmp rcx`、表の移動先 5 つは生成 C にラベルが無い。`msvc_rejected` の理由は決定の入口ブロックの条件）。(i)〜(iv) のどれでもない。unit 158（quickfontcache_x64.dll の LoadLibraryW）には届いていない。
- **新しい停止は出なかった**（停止は動かなかった）。

### 6.3 参考: 基底証明を緩めた測定（決定ではない。PM の判断の材料）

`fe-lift --msvc-base-dominating`（`BaseProof::Dominating`）で Blend2D の生成 C だけを作り直した（TAG `b2d58dom`。他は同じ）。**既定の実装にはこの条件を入れていない**。決定の簡易条件を緩めた場合の挙動を、PM が判断できるように測ったもの。

- 条件: B への書き込みが `lea B,[rip+d]` = ImageBase の 1 回だけで、それが jmp に到達する全経路（線形走査の CFG。jmp 自身の表の移動先を辺にし、他の間接 jmp は辺なし）の前に実行される。これで `0x470a89e`（`lea r13` が 0x4709f18。入口ブロックの外だが、`jne 0x4709f07` のどちらの経路でも 0x4709f18 を通る）が解決され、`f_4709ea0` の switch は 6 つの `goto` になる（`verify_targets_b2d_dom.txt`: 6 件とも命令の先頭）。
- 他の site: Blend2D 種なしで jumptabs 2 → 3、閉包 398,980 → 399,148（+168）、`bad_nodes` 0、`unsupported_total` 662 のまま。種 58 個ありで 2 → 3、399,609 → 399,777。dsp_ippv2 は 435 のまま（変化なし）。
- a4（2 回: 11:10:15Z、11:10:42Z。ログ `three-out-dom/a4/`）: init_count 150/942、last unit 149（proc 0x664260）、functions_distinct 1,058、function_entries **27,317**（+1）。**停止は移動する**: `fle: unsupported instruction at 0x4899fed`（`abort kind: fle_unsupported`、top of stack `f_4899d90`、`call depth at stop: 17`）。2 回の stderr は `cmp` で一致。
- `0x4899fed` は `call qword ptr [rip + 0x1d0d5]` で、IAT スロット `0x48b70c8` = **`KERNEL32.dll!GetModuleHandleA`**（pefile でスロットを解決）。`imports_table` に行が無い import（行なし 15 名のうちの 1 つ。`blend2d-seed58.md` §6.3 (d)、§7.2 の 4: 停止は `GetModuleHandleA` より前、と読んでいた）。つまり読みどおり、`f_4899d90` が CPU 情報の初期化（`f_470abb0` → `f_4709ea0`）を終えて `GetSystemInfo`（今回 2 回呼ばれた）の後に `GetModuleHandleA(…)` を呼ぶ所まで進んだ。
- ATTACH 内の import の列: 174 回（前回 172。差は `GetSystemInfo` の 2 回だけ。他の名前と順序は同じ。ログ `run/b2d58dom-imports.txt`）。DllMain は戻らず、LoadLibraryW は 0x4700000 を返さない。分類は (i)（未応答の import。行なし 15 名の `GetModuleHandleA`）に移る（`fle_unsupported` は API 名を持たず VA を出すので、API 名は IAT スロットから導いた）。
- **この結果が示すこと**: 決定の簡易条件（入口ブロックの `lea`）を「到達する全経路の前に実行される `lea`」に緩めれば、停止は B3 から B8（`GetModuleHandleA`）に移る。ただし緩めると、(1) 線形走査の CFG に他の間接 jmp の辺を入れていない（Win64 の関数の途中への jmp を仮定しない）、(2) 他の site で同じ緩め方が誤った表を作らないかは、dsp_ippv2、Blend2D、FLEngine の 3 つでしか確認していない（Blend2D と dsp_ippv2 では、`--msvc-base-dominating` で増えるのは `0x470a89e` の 1 か所だけ）。緩めるかどうかは PM が決める。

### 6.4 参考: 種なしの Blend2D（任意。決定どおり、TAG `b2d0`、1 回）

a4: 停止 `fle: indirect call to 0x48a57f8 is not a known function entry`（top of stack `f_48b5b90`）、init_count 150/942、last unit 149、functions_distinct 960、function_entries 26,604。`blend2d-seed58.md` §4 の対照（種なし）と**同一**。Blend2D の生成 C は 2 つの MSVC switch が解決された以外は同じで、停止点（B5 の `0x48a57f8`）には届く前に止まるので、MSVC switch の解決は a4 の結果を変えない。表の統合は 69,605 行（前回と同じ）。ログ `three-out-b2d0/a4/`。

## 7. 事実の並べ（決めない）

1. **決定どおりの実装は、Blend2D の停止点を動かさない。** 理由は決定の簡易条件のうち「`lea` は入口ブロックにある」の 1 つだけ（`f_4709ea0` の `lea r13,[ImageBase]` は 0x4709f18、入口ブロックは `jne 0x4709f07` までの 0x4709ea0〜0x4709efa）。N（31）、表、全 6 移動先（関数内、命令境界）は条件を満たす。
2. 95 か所のうち決定どおりで解決できるのは、Blend2D 種なし（`--pdata`）で **2 か所**だけ。多数（34 か所）が「基底が証明できない」で、その大半（33 か所）は B が関数内で複数回書かれる形（MSVC が switch の直前に `lea B,[ImageBase]` を出し直す形など。§3.4 の参考: 38 か所が同じブロック内に `lea` がある）。N が導けない 40 か所は、`cmp; ja` が隣接しない形で、条件 (b) の文言（`cmp idx,N; ja` または `sub idx,K; cmp idx,N; ja`）の範囲では導けない。形が (c1)(c2) の文字どおりでない 15 か所は、`mov; add; jmp` の間に命令が挟まる。
3. 基底を「jmp に到達する全経路の前に実行される `lea`」に緩めれば `0x470a89e` は解決され、停止は `GetModuleHandleA`（B8）へ移る（§6.3）。ただし決定の簡易条件を変える判断は PM が行う。この文書は決めない。
4. FLEngine の生成 C、単独構成の a4 に影響しない（§5.2）。dsp_ippv2 には 1 か所、Blend2D には 2 か所の switch が新しく解決され、閉包が +209 / +101 命令増えた。`bad_nodes` と `unsupported_total` は変わらない。

## 8. 解釈が分かれた点、実装できなかった点（PM への報告）

1. **「`sub idx,K; cmp idx,N; ja`」の `sub` の扱い**: N は `cmp` の即値で、`sub` が前にあってもなくても同じ。したがって `sub` を特別に扱うコードは無い（`cmp` が比べるレジスタが表の添字と一致することだけ見る）。決定の文言「`sub idx, K; cmp idx, N; ja`」を、**`cmp` の直前に `sub` があってもよい**と解釈した（合成テストで `sub eax,K` ありを確認）。`sub` の直後の `cmp` を、`ja` の前に別の命令なしで要求している（`ja` と idiom の間は `cdqe`/`lea B,[rip+d]` だけ許す）。
2. **`ja` と idiom の間の `cdqe` と `lea B,[rip+d]`**: 決定は「`cmp idx,N; ja default`」の直後を明示しない。実 DLL で、`cdqe`（`lea eax,[r-K]; cmp eax,N; ja; cdqe; mov ecx,[r12+rax*4+T]`）と、ガードの直後の `lea B,[rip+d]`（MSVC が switch の直前に基底を出し直す）が頻繁に入るので許した（`cdqe` は添字が rax のとき、最大 2 個）。**許さなければ、解決する 2 か所（`0x4701384`、`0x4701cbb`。どちらも `cdqe` が入る）が 0 か所になる**。解釈を広げた点として明記する。これを狭める、または広げる（`mov eax,r` などを許す）かは PM の判断。
3. **形の隣接**: (c1)(c2) の「`mov; add; jmp`」は隣接を要求した（間に `movaps` などが入る 15 か所を解決しない）。決定の文言「直前の命令列から次がすべて証明できる」を、命令の並びの隣接ではなく**値の依存の証明**と読めば、15 か所のいくつかは対象になりうるが、各 site で挟まる命令が `rT`/`B`/`I` を変えないことの証明が要る。実装していない。
4. **入口ブロックの定義**: 「関数の入口ブロック」を、入口から最初の非フォールスルー命令（条件分岐、jmp、ret）まで、**call では終わらせない**と定義した。call を終わらせる定義にすると、`lea` が call の後ろにある形（例: `call; lea r13,...`）が入口ブロックの外になる。実 DLL では差が出ない（影響する site は 0）。
5. **決定に足した 4 つの条件**（§4.1）: 揮発 B と call、`verify_msvc` の 3 検査、`foreign_entries`、`landings_in_idioms`。いずれも解決を**減らす**方向（緩めていない）。簡易条件の「入口ブロックの `lea` + 他の書き込みなし」だけでは、(a) 関数の途中へ外から飛び込む分岐、(b) ガードを飛ばす分岐、(c) 呼び先で壊される揮発 B が、誤った表を作る余地として残るため、決定の趣旨（誤った表を作らない）に沿って足した。足したことの影響（解決が減った site）は、実 DLL 3 つで **0**（(a)(b)(c) に当たったものはなく、先に条件 (a)(b) で落ちる）。PM が不要と判断すれば外せる（`verify_msvc`、`foreign_entries`、`landings_in_idioms`、`prove_base` の揮発の検査）。
6. **種 58 個ありで拒否が 1 件増える**（`0x478136e`）: 58 個で閉包に入った関数の site で、N が導けない。解決の数は変わらない。
7. **`.pdata` に無い範囲の `jmp reg`**（Blend2D の `0x4724226`、`0x472442b`、`0x48a3479`。`jmpreg.txt` の 95 か所の外）: 生成 C の関数の中にあるが、`.pdata` のエントリが無い範囲（葉関数と読む。§5.3）。決定の (d)「同じ `.pdata` の RUNTIME_FUNCTION」を満たせないので解決しない（`0x4724226`、`0x472442b` は「`.pdata` の関数の中にない」、`0x48a3479` は N が導けない）。`.pdata` に無い葉関数の switch を扱うかは PM の判断（今回は扱わない）。
8. **FLEngine の MSVC 形**: 決定 0036 の `Spec` が先に解決するため新しい認識には回らない。0036 の 3 か所の表を、新しい認識が（`Spec` が無くても）同じに解くかは確認していない（`Spec` を外したテストは書いていない）。

## 9. 限界、未確認

- **解決した 3 か所の switch が実機で正しい表であること**: バイト列（索引バイトと RVA 表、移動先の命令境界）だけを根拠とする。実行時の値（実際の索引）は 3 か所のうち Blend2D の `f_4701310`、`f_4701bb0`、dsp_ippv2 の `f_37c6990` では実行して確かめていない（これらは停止までに実行されていない: 3 モジュールの測定は `0x470a89e` で止まる。実行されたかは調べていない）。参考の変種で解決した `0x470a89e` だけが、native との突き合わせ（40 通り）を通った。
- 決定の簡易条件（入口ブロックの `lea`）が緩すぎるか、厳しすぎるかは、この作業では判断していない。実測は、厳しい側に倒すと解決が 95 か所のうち 2 か所になることを示す（§3）。
- 他の DLL（FLEngine、dsp_ippv2、QFC、Blend2D 以外の同梱 DLL）での誤認識の母集団検査は、3 つの DLL に限る。FLEngine では `msvc_rejected` 0（新しい認識に回る `jmp reg` が無い）。
- `fe-lift --msvc-base-dominating` の条件（§6.3）の健全性は、他の間接 jmp への辺を持たない近似に依る。決定ではなく、実装の既定にも入れていない。
- 閉包の増加（+101 / +209 命令）の中に、`bad_nodes`、`unsupported` は無かった。新しく閉包に入った命令が実行されたかは調べていない。
- `.pdata` を持たない image（例: 一部の同梱 DLL）では、この実装は何も解決しない（`not inside a .pdata function`）。
- wasm は使っていない（native のみ）。実 Windows での挙動は未確認。
- 3 モジュールの測定は gdb の仮の値なし、runtime の応答追加なし。**新しい停止は決定どおりの実装では出なかった**（停止が動かなかった）。参考の変種で出た `GetModuleHandleA` の停止（`0x4899fed`）は直していない。

## 10. 作ったファイル

- 変更: `docs/README.md`（0089 の行）、`lift/crates/fe-decode/src/jumptab.rs`、`lib.rs`、`lift/crates/fe-lift/src/jumptab.rs`、`lib.rs`、`main.rs`、`lift/crates/fe-decode/tests/jt_cpu.rs`（§5.1）。
- 新規: `docs/decisions/0089-msvc-switch-tables-with-proven-base.md`、本文書、`lift/crates/fe-decode/tests/msvc_common/mod.rs`、`lift/crates/fe-decode/tests/msvc_switch.rs`、`lift/crates/fe-lift/tests/msvc_switch.rs`。
- 作業物（gitignore 対象）`lift/out-jtb2d/`: `analysis/`（`pre_survey.py`、`pre_survey2.py`、`pre_survey3.py`、`between.py`、`ctx_sites.py`、`ambig.py`、`join95.py`、`verify_targets.py`、`targets_in_fn.py`、`local_lea.py`）、`logs/`、`stats/`（DLL ごとの前後の統計）、`test-ws.txt`、`three/`（3 モジュールの生成と build）、`three-out*/a4/`（a4 のログ）、`run/`、`gdb/`、`base-src/`（HEAD の `git archive`）、`target*`（cargo の出力）。
