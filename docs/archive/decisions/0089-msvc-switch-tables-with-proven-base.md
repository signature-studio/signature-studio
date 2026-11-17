# 0089: MSVC の switch 表（`jmp reg`。索引バイト表 + 32 ビット RVA 表。基底レジスタがイメージ基底と証明できるもの）を fe-decode / fe-lift で解決し、表の移動先を生成 C に入れる

状態: **決定（仮ではない）**。PM の決定（2026-10-24）を、作業者（サブエージェント）が記録した。実行ではなく**バイト列で検証できる**ので仮の決定にしない。ただし (1)(a) の「基底証明」の簡易条件は PM の決定の一部で、緩めると誤った表を作りうる。**簡易条件を緩めない**。決定の節は実装の前に保存し、「実装した場所」「テスト」「検証」「解釈が分かれた点」「限界」は実装と検証の後に書く（書いた時点で、この節のその行を更新する）。

事実の出典（節番号は各文書のもの）:

- `docs/investigations/blend2d-seed58.md`（コミット 04f2eed）§6.4（停止した値 0x470a8a0 の性質）、§7.1（分類 (v)）、§7.2 の 2、3（B3 の 95 か所と、生成器の現状）。
- `docs/investigations/blend2d-lift-feasibility.md` の B3（§10.3 の表の行。§3.4 が分類 `rva-switch` 95 か所、移動先 distinct 1,116、うち `.pdata` 開始 1。出力 `lift/out-blend2d/jmpreg.txt`）。
- 決定 0036-indirect-jmp（`jmp reg` の tail dispatch と、注釈つき jump table。FLEngine の MSVC 形 3 か所）、決定 0079（CPU 振り分けスタブの表。`fle_jumptab_miss`）。

## 決定（PM。内容を変えない）

| # | 項目 | 決定 |
|---|---|---|
| 1 | 対象の形 | `jmp reg`（レジスタ間接 jmp）で、直前の命令列から次が**すべて**証明できるもの。**(a) 基底の証明**、**(b) 範囲の上限**、**(c) 表の形**、**(d) 移動先**（下の 1〜4）。1 つでも証明できなければ**解決しない**（従来の dispatch のまま。fail-closed） |
| 1(a) | 基底レジスタ B が**イメージ基底そのもの**である証明 | 関数内で B への書き込みが **`lea B, [rip + disp]` の 1 回だけ**（disp の結果がちょうど PE の ImageBase。MSVC の `__ImageBase` 参照）で、それが jmp から到達する経路の前に必ず実行される。**簡易条件（足りるならこれで足りる。緩めない）: 関数の入口ブロックで `lea B,[rip+disp]` があり、関数内の他の命令が B を書かない**。B を push/pop で退避・復元するのは書き込みに数えない。証明できなければ解決しない |
| | （注記） | 基底証明の簡易条件（入口ブロック）は決定 0090 で dominance に置き換わった |
| 1(b) | 索引の範囲検査 | `cmp idx, N; ja default` または `sub idx, K; cmp idx, N; ja` から**上限 N を導く**。導けなければ解決しない |
| 1(c1) | 表の形（1 段） | `mov ecx,[B + idx*4 + T]; add rcx,B; jmp rcx`（32 ビット RVA の表。エントリ数 = N+1） |
| 1(c2) | 表の形（2 段） | `movzx eax, byte [B + idx + T1]; mov ecx,[B + rax*4 + T2]; add rcx,B; jmp rcx`（索引バイト表 T1 は N+1 バイト、RVA 表 T2 のエントリ数 = 索引バイト表の最大値 + 1） |
| 1(d) | 表の各移動先 | B + RVA が実行セクション内にあり、**同じ `.pdata` の RUNTIME_FUNCTION（連鎖含む）の範囲内**であること（MSVC の switch の移動先は同じ関数の中）。範囲外のエントリが 1 つでもあれば、その site は**解決しない**（範囲外のエントリが何であるかを報告に列挙）。かつ移動先が命令境界にデコードできる（`bad_nodes` 0） |
| 2 | 生成 C | 解決した site は、索引から移動先へ飛ぶ `switch`（または既存の jump table 実装と同じ形）にし、各 case は同じ関数内のラベルへ `goto`。**表の外の索引（N を超える）は `fle_jumptab_miss`（名前付き停止）**。移動先のラベルを関数のコントロールフローグラフに加える（既存の 0036-indirect-jmp / 0079 の実装に従う。到達した命令が増えるのでクロージャが増える） |
| 2' | 扱わない形 | **(c1)(c2) 以外**（基底が別の方法で得られる、表が 64 ビット絶対 VA、`.rdata` の関数ポインタ表形）は**今回扱わない**（従来どおり） |
| 3 | FLEngine への影響 | FLEngine の生成 C の SHA-256 は **`43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1` のまま**が期待。変わったら、変わった関数の一覧と原因（FLEngine の `jmp reg` のうち新しい形に当たったもの）を報告に書く（隠さない。原因が新規認識のみであることを確認する。変わった場合の判断は PM が行う）。`unsupported_total`、`register_jmp_sites` の統計の変化も記録する |
| 4 | 変えないもの | 決定 0074（pdata 開始の reloc 値だけを種にする）、0079（CPU 振り分けスタブ）、0080（基底）、0086（同梱 DLL の方向）。**58 個の種は決定ではない**（測定用のまま）。**この実装は種 58 個に依存しない**: 種なしの Blend2D でも switch の解決は動く |
| 5 | 状態 | **仮の決定ではない**（バイト列で検証できる）。1(a) の簡易条件は PM の決定で、緩めない |

## 背景（事実。出典の節番号）

- Blend2D.dll（リベース後基底 0x4700000。測定用の値）に、調査が列挙した「`.pdata` に無い葉関数 58 個」を種として足すと、ATTACH の停止は `fle: indirect call to 0x48a57f8`（B5）から `fle: indirect call to 0x470a8a0 is not a known function entry`（`fle_dispatch_miss`）へ移った（`blend2d-seed58.md` §6.1。実測）。
- 停止は `f_4709ea0`（`.pdata` の `[0x4709ea0, 0x470abac)`）の `0x470a89e: jmp rcx`。直前は `movzx eax, byte [rax+r13+0xab8c]; mov ecx, [r13+rax*4+0xab74]; add rcx, r13`（r13 = 0x4700000 = イメージ基底）。索引バイト表 0x470ab8c（32 バイト）、RVA 表 0x470ab74（6 エントリ: 0x470a8a0、0x470a8af、0x470aa40、0x470aa6b、0x470aab1、0x470aafc）、索引 = `r9d − 0x80000000`、32 を超えると `ja 0x470aafc`。移動先のうち 5 つは生成 C にラベルが無い（`fle_dispatch(R[1]); return;` のまま）（`blend2d-seed58.md` §6.4。バイト列と表の中身は実測、switch 表と読むことは読み）。
- 調査 B3: `jmp reg` の `rva-switch` 95 か所（distinct な移動先 1,116、うち `.pdata` 開始 1）。閉包の約 8% が生成 C に入らず、うち 982 命令は switch 表の移動先の位置にある（読み）。
- 既存の実装: 決定 0036 は FLEngine の 4 か所を人が注釈（`Spec`）にした。決定 0079 は CPU 振り分けスタブ（`movsxd; lea; jmp [r10+rax*8]`）を、`.reloc` の DIR64 の連続から範囲を取って認識する。**今回の形（基底が r13 に既にある、索引バイト表が間に入る 2 段）を認識する箇所は、実装前には無かった**（`jumptabs 0`）。

## 範囲外（今回扱わない。従来どおり）

- 基底が `lea B,[rip+disp]` 以外で得られるもの（`mov B, imm`、引数から来るもの、複数回書かれるもの、入口ブロックの外で `lea` されるもの）。
- 表が 64 ビット絶対 VA（`jmp [B+I*8+disp]`。決定 0079 の 183 か所のうち 155 の形）。`.rdata` の関数ポインタ表形、仮想関数表の tail call 風（13）。
- `cmp`/`ja` から N を導けないもの。
- 移動先が `.pdata` の RUNTIME_FUNCTION の範囲外のもの（site ごと解決しない）。
- 決定 0074、0079、0080、0086 は変えない。58 個の種は決定にしない。

## 実装した場所

- `lift/crates/fe-decode/src/jumptab.rs`（追記）:
  - `recognize_msvc(insns, va, jmp)`: 閉包が前向きにデコードした命令だけを、jmp から後ろへ 1 命令ずつたどる（`add rT,B`、`mov eT,[B+I*4+T2]`、あれば `movzx eX,byte [B+I0+T1]`、`ja`、`cmp I0,imm`）。命令長の当て推量はしない。`ja` と表の idiom の間に置いてよいのは `cdqe`（添字が rax のときだけ）と `lea B,[rip+d]` だけで、最大 `MSVC_SKIP` = 2 個。N = `cmp` の即値（32 ビットなら下位 32 ビット）で、4,096 以上は導けない扱い（`MSVC_MAX_ENTRIES`）。形が表 idiom でなければ `Ok(None)`（何も報告しない）、表 idiom だが N が導けなければ `Err`（`msvc_rejected` に理由）。
  - `FuncMap`（`.pdata` の RUNTIME_FUNCTION を `fe_pe::Pe::function_table` の連鎖解決つきで引く）、`Sweep`（1 関数の全ピースの線形走査。表のバイトでデコードできない所は飛ばす）。
  - `prove_base`: 決定 1(a) の簡易条件そのまま。関数（連鎖したピース全部）の命令列で B への書き込み（iced-x86 の `used_registers` の write 系。暗黙の書き込みを含む）がちょうど 1 回、それが `lea B,[rip+d]` で rip+d = ImageBase、かつ関数の入口ブロック（入口から最初の非フォールスルー命令まで。call は終わらせない）の中。`push B` は書き込みに数えず、`pop B` は同じ関数に `push B` がある場合だけ数えない。**決定に足した追加の条件が 1 つ**: B が揮発レジスタ（rax rcx rdx r8–r11）で関数内に call があれば証明しない（呼び先が B を変えうる。非揮発レジスタは呼び先が保存すると仮定。`dataflow.rs` と同じ仮定）。
  - `resolve_msvc`: 表を読む（(c2) は索引バイト表 N+1 バイトの最大値 + 1 が RVA 表のエントリ数）。全エントリが B+RVA で、**同じ関数のいずれかのピースの中**かつ実行セクション内、命令境界（線形走査の命令開始）であること。1 つでも外なら site ごと拒否し、外のエントリを「別の関数の開始か、どの .pdata にも無いか、実行セクション外か」つきで列挙する。順序は 関数 → 基底証明 → 表と移動先。
  - `verify_msvc`（閉包の完成後）: 閉包が `cmp; ja` を認識した形でデコードしていること、`ja` の次から jmp までの間でブロックが始まらないこと（ガードを飛ばして着地する分岐がない）、jmp が関数の入口の関数に属していること、閉包がデコードした関数内の命令がすべて線形走査の命令開始に当たること、他の閉包関数がこの関数のピース内で始まらないこと。`foreign_entries`: 関数の外の命令（直接分岐、解決済み間接 jmp、他の表の case）がこの関数の入口以外へ飛ぶなら、その関数の表は捨てる（入口ブロックの `lea` を通らずに jmp へ来うるため）。`landings_in_idioms`: ガードの `cmp` と jmp の間に着地する分岐があれば捨てる。
  - `Origin::Msvc { n, t1, t2, from, guard_ja, guard_cmp, function }`、統計キー `jmp:msvc_table`。`BaseProof::{EntryBlock（既定。決定）, Dominating（**決定ではない**。測定専用。`fe-lift --msvc-base-dominating` でだけ有効）}`、`Options`。
- `lift/crates/fe-decode/src/lib.rs`: `analyze_with_options`（`analyze_with_specs` は既定の `Options` で呼ぶ）。未解決の `jmp reg` で、`Spec` にもスタブ形にも当たらないものに `recognize_msvc` を試す。解決した表の移動先は**ブロックとして**閉包に積む（同じ関数の中のラベル。決定 0036 の注釈つき表と同じ。0079 のスタブは関数として積む）。`Closure::msvc_tables`、`Closure::msvc_rejected`（jmp VA と最初に失敗した条件）を追加。`jumptabs` には含める。
- `lift/crates/fe-lift/src/jumptab.rs`（4 行）: ハンドラ名 `jmp_ind:msvc_table`。`switch` の生成は既存のもの（`Select::Target`: 選択子は jmp のレジスタの実際の値、case は移動先の VA、各 case は `goto L_<VA>`、既定は `fle_jumptab_miss`）。**生成 C の形は決定 0036 の FLEngine の MSVC 形 3 か所と同じ**。
- `lift/crates/fe-lift/src/lib.rs`、`main.rs`: `generate_with_options`、`Stats::msvc_tables`、`Stats::msvc_rejected`、`fe-lift` の出力に `msvc_tables`、`msvc_rejected`（件数と、1 件ずつ jmp VA と理由）、`--msvc-base-dominating`（測定用）。`runtime` は編集していない（`fle_jumptab_miss` は既存）。

## テスト

- `lift/crates/fe-decode/tests/msvc_common/mod.rs`（新規。合成 PE と小さなアセンブラ）、`msvc_switch.rs`（21 件）、`lift/crates/fe-lift/tests/msvc_switch.rs`（8 件）。詳細と結果は `docs/investigations/blend2d-switch-tables.md`。
- 解決するもの: (c1)、(c2)（索引バイト 6 個、RVA 表は max+1 = 3 エントリ。4 つ目はファイルにあるが読まない）、`cmp rax`、`cdqe`、`call` を含む関数での非揮発 B、揮発 B（call なし）、連鎖した RUNTIME_FUNCTION のピースにある移動先。
- **解決してはいけないもの**（解決されないこと、jmp が `Unresolved` のまま、`fle_dispatch(R[1]); return;` のままであることを確認）: `lea` が入口ブロックの外、B が 2 回書かれる（`lea` の後の `xor`、`push B` の無い `pop B`）、B が引数から来る、`lea` の値が ImageBase でない（+0x10、−0x1000）、揮発 B で call あり、ガードなし、ガードが別のレジスタ、ガードを飛ばして idiom に着地する分岐、別の関数が途中へ jmp する、移動先が別の関数／実行セクション外／命令境界でない、例外ディレクトリなし。
- 合成 PE を clang で native にも実行（`harness/image.c`）: 索引 0..N+3 のすべてで native と生成 C の結果が同じで、表のバイトが指す先と一致。ガードを飛ばして B が ImageBase でないまま idiom に入ると `fle_jumptab_miss`。
- 実 DLL（Blend2D `f_4709ea0` の 0x470a89e）。**決定の基底証明（既定）では解決されない**（`lea r13,[ImageBase]` は 0x4709f18 で入口ブロックの外）。**この結果を PM の決定どおりとして報告する**。`--msvc-base-dominating`（決定ではない）では解決し、索引 0..32 の 33 通りを含む 40 通りで、native（本物の命令）と生成 C の到達先が、ファイルの索引バイト 32 個と RVA 6 個から読んだ値と一致。

## 検証

実測値と手順は `docs/investigations/blend2d-switch-tables.md`（以下「調査」。節番号はその文書のもの）。`lift/out-jtb2d/` に作業物。

| 項目 | 結果 |
|---|---|
| `cargo test --workspace`（`CARGO_TARGET_DIR=lift/out-jtb2d/target`） | **97 binaries / 1,161 passed / 0 failed / 0 ignored**（基準 95 / 1,130。+2 binaries、+31 件 = `fe-decode/tests/msvc_switch.rs` 22 + `fe-lift/tests/msvc_switch.rs` 9）。既存の期待値を変えた行: `fe-decode/tests/jt_cpu.rs` の `dsp_ippv2_has_434_stubs`（dsp_ippv2 に解決される MSVC switch が 1 か所あったため。`jumptabs` 434 → 435、`jmp:unresolved` 549 → 548、`jmp:msvc_table` 1 を追加。183 は変えていない） |
| **FLEngine の生成 C の SHA-256** | **`43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1` のまま**（HEAD のビルドの出力と `cmp` でバイト一致）。変わった関数 0。functions 59,297、bad_nodes 0、unsupported_total 525、register_jmp_sites 20、jumptabs 4、`msvc_tables` 0、`msvc_rejected` 0（FLEngine の MSVC 形 3 か所は決定 0036 の `Spec` が先に解決する）。単独構成の a4 は生成 C が変わらないので省略 |
| 95 か所（`jmpreg.txt` の `rva-switch`）の見積もり | 決定どおりで**解決できるもの 2 か所**（Blend2D 種なし `--pdata`。`0x4701384`、`0x4701cbb`）。解決されない 93: N が導けない 40、基底が証明できない 34（B が複数回書かれる 33、`lea` が入口ブロックの外 1）、形が隣接でない 15、閉包に未到達 4、**移動先が関数の外 0**。静的な見積もり（Python + capstone + 自前の `.pdata` パーサ）と実装は 95 か所すべてで一致（調査 §3） |
| DLL ごとの統計の前後 | Blend2D 種なし: closure 398,879 → 398,980、jumptabs 0 → 2、bad_nodes 0、unsupported_total 662 のまま、register_jmp_sites 99 のまま。Blend2D 種 58 個あり: 399,508 → 399,609、jumptabs 0 → 2、bad_nodes 0、unsupported_total 665 のまま。dsp_ippv2（0x2000000 `--pdata`）: 6,464,799 → 6,465,008、jumptabs 434 → 435、bad_nodes 0、unsupported_total 974,527 のまま。QFC: 変化なし。**bad_nodes はどこも増えなかった**（調査 §5.3） |
| 解決した site の全エントリの個別照合（objdump / capstone） | 決定どおりで解決した 3 か所（Blend2D 2、dsp_ippv2 1）の distinct な移動先 17 件すべてが、関数のピース内の命令の先頭（objdump の最初の行がちょうどその VA）。範囲外のエントリは 0 件（調査 §5.4） |
| **Blend2D `f_4709ea0` の site `0x470a89e`（実 DLL）** | **決定どおりの基底証明（簡易条件）では解決されない**: `lea r13,[ImageBase]` は 0x4709f18 で、関数の入口ブロック（0x4709ea0〜0x4709efa）の外。N = 31、索引バイト表 32 バイト、RVA 表 6 エントリ、移動先 6 つは関数内で、基底以外の条件は満たす。**したがって決定どおりでは停止点は動かない**。表の 6 エントリ（`0x470a8a0`、`0x470a8af`、`0x470aa40`、`0x470aa6b`、`0x470aab1`、`0x470aafc`。元の VA で 0x18000a8a0 …）と索引バイト 32 個の対応は、参考の変種（下）で全 33 通りの索引（0..32）と範囲外の値 7 通りを native（本物の命令）と生成 C の両方で実行して、ファイルから読んだ値と一致することを確認した（調査 §5.6） |
| 3 モジュール（種 58 個あり、基底 0x4700000、決定どおり。測定のみ） | **a4 の行は前回（`blend2d-seed58.md` §6.1）と同一**: init_count 150/942、last unit 149、functions_distinct 1,058、function_entries 27,316、停止 `fle: indirect call to 0x470a8a0 is not a known function entry`（`f_4709ea0`）。2 回同一。DllMain は戻らず、LoadLibraryW は 0x4700000 を返さない。ATTACH の import 172 回。分類 (v)（B3）のまま（調査 §6.2） |
| **参考（決定ではない。判断の材料）: 基底証明を「`lea` が jmp へ到達する全経路の前に実行される」に緩めた測定** | `fe-lift --msvc-base-dominating`（測定専用。既定ではなく、決定の実装には入っていない）で `0x470a89e` が解決され、停止は `fle: unsupported instruction at 0x4899fed`（`f_4899d90` の `call [IAT]` = `KERNEL32!GetModuleHandleA`。行なし 15 名の 1 つ。B8）へ移る。ATTACH の import は 174 回（`GetSystemInfo` が 2 回増える）。他の site への影響は Blend2D の `0x470a89e` の 1 か所だけ（調査 §6.3）。**緩めるかは未決** |

## 解釈が分かれた点

詳細は調査 §8。決定の内容は変えていない。即興で代えず、実装した解釈を挙げる。

1. 「`sub idx,K; cmp idx,N; ja`」: N は `cmp` の即値で、`sub` の有無で変わらない。`cmp` が比べるレジスタが表の添字と一致することを要求する。
2. `ja` と idiom の間: `cdqe`（添字が rax のとき）と `lea B,[rip+d]` だけ、最大 2 個を許した。**許さないと、解決する 2 か所が 0 か所になる**。
3. (c1)(c2) の「`mov; add; jmp`」は**隣接**を要求した（間に別の命令が挟まる 15 か所は解決しない）。
4. 「入口ブロック」= 入口から最初の非フォールスルー命令（条件分岐、jmp、ret）まで。call では終わらせない。実 DLL で差は出ない。
5. **決定に足した 4 つの条件**（いずれも解決を減らす方向。実 DLL では解決を落とさなかった）: 揮発 B と call、`verify_msvc`（ガードのデコード、ガードを飛ばす着地、入口の関数への帰属）、`foreign_entries`（関数の外から入口以外へ飛び込む分岐）、`landings_in_idioms`。PM が不要と判断すれば外せる。
6. `.pdata` に無い葉関数の中の `jmp reg`（Blend2D に 3 か所。調査 §5.3）は、決定の (d) を満たせないので解決しない。

## 限界

- 決定の簡易条件が、実 DLL の 95 か所のうち 2 か所しか解かない（調査 §3）。MSVC が switch の直前で `lea B,[ImageBase]` を出し直す形（調査 §3.4: 38 か所が同じブロック内に `lea` を持ち、N が導けて移動先が関数内のものは 20 か所）は、簡易条件では解決しない。簡易条件を変える判断は PM が行う。
- 解決した 3 か所が実行時に正しい表であることは、バイト列（命令境界、関数内）の確認に限る。3 か所のうち 2 か所（Blend2D `f_4701310`、`f_4701bb0`）と dsp_ippv2 の 1 か所は、native との突き合わせをしていない（実行されたかも未確認）。`0x470a89e` だけ（参考の変種）40 通りを native と生成 C で突き合わせた。
- 母集団の誤認識の検査は、FLEngine、dsp_ippv2、Blend2D、QFC の 4 つに限る（同梱の他の DLL は未実施）。
- `.pdata` を持たない image では何も解決しない。
- 実 Windows での挙動は未確認。wasm は使っていない。
