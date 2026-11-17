# 0036: 解決不能な間接 jmp（`jmp r64` の tail dispatch と、注釈つき jump table）

状態: 作業者（サブエージェント）が決定。PM の決定は含まない。計測と実行は `docs/investigations/indirect-jmp-run.md`。仕様は `docs/investigations/unsupported-inventory-pdata.md` §1.1、§6 W1/W2。

注意: `docs/decisions/0036-a5-oracle-v2.md`（別の担当、未追跡）と番号が重なっている。依頼で指定されたファイル名を使った。

## 背景

all-pdata の生成 C に `Jmp: indirect target unresolved` が 62 行あった（在庫の §1.1）。内訳は、遅延ロードヘルパ末尾の `jmp rax` 57 行（17 か所が 57 thunk に展開）、`f_40d210` の `jmp [r10+rdx*8+0x40]` 1 行、MSVC 形 switch の `movsxd; add; jmp reg` 3 行、`f_4126f0` の `jmp [rcx]` 1 行（longjmp 風、W3）。

## 決定

### W1: `jmp r64` を tail dispatch にする

- 解決できなかった `jmp r64`（`Resolved::Unresolved`）を `{ fle_dispatch(R[reg]); return; }` に落とす（`fe-lift/src/tailjmp.rs`）。ゲストの戻りアドレスはスタックに残る。移動先の関数の `ret` は C の `return;` なので、呼び出し元へ戻る。間接 call（`dispatch.rs`）と違い、戻りアドレスは積まない（`jmp` は積まない）。
- 対応表に無い値は、既存の `fle_dispatch_miss` で abort する（`fle_dispatch.h` は変更していない）。
- 対象は 64 ビットのレジスタ操作数だけ。`jmp [mem]` は従来どおり `Jmp: indirect target unresolved`（`f_4126f0` の `jmp [rcx]` は W3 のまま名前付きで止まる）。
- 呼び先の選択: 遅延ロードヘルパの `jmp rax` は、`GetProcAddress` が返した擬似 VA（`fle_resp_table` の行）か本物の関数入口を指す。どちらも `fle_dispatch_table` にあるので、それで引く。
- 統計: ハンドラ名 `jmp_ind:reg_dispatch`（`handled`）。**register 形 jmp の別計上**を追加した。`Stats::register_jmps`（出力された occurrence ごと、結果別: ハンドラ名または `unsupported`）、`Stats::register_jmp_sites`（閉包で到達した `jmp reg` の箇所数）、`fe-lift lift` の出力に `register_jmp_sites`、`jumptabs`、`register_jmps` を追加。`fe-decode` の `Closure::register_jumps` も同じ数。

### W2: 表を引く jmp は注釈（Spec）で与え、`switch` に落とす

- 汎用のパターン認識（`cmp`/`ja`/`jg` の読み取り）は**入れない**。範囲の根拠は人が `objdump` で確認し、`fe-decode/src/jumptab.rs` の `FLENGINE` に 4 件の `Spec`（jmp の VA、命令バイト、関数の範囲、表の VA、エントリ数、形）として書いた。母集団（PE 全体の同形 jmp）での誤認識の実測は、認識を入れないので対象外。
- 4 件の根拠（すべて `objdump -d -M intel` と、`out-ijmp/tables.py` が PE のバイトから読んだ表。数値は `indirect-jmp-run.md`）:

| jmp | 関数 | 形 | 範囲の導出 | 表 |
|---|---|---|---|---|
| 0x40d253 | 0x40d210–0x40d31b | `jmp [r10+rdx*8+0x40]`、8B 絶対 VA | `test rdx,rdx; jle ret`（rdx ≥ 1）、`cmp rdx,0x20; jg`（≤ 0x20）、`sub rdx,8; jg`（rdx−8 ≤ 0）。添字は −7〜0 の 8 つ。基底 r10 は直前の `lea r10,[rip+5]` = 0x40d258、スロットは 0x40d260〜0x40d298。idx −8 のスロット（0x40d258）は不達で、注釈に含めない | 8 エントリ |
| 0x5e5eb6 | 0x5e5860–0x5e6dd0 | `jmp rcx`、4B 相対 | `add ecx,-0x45; cmp ecx,0x22; ja 0x5e63ae`。0〜34、N = 35。基底 rax は直前の `lea rax,[rip+0xe95]` = 0x5e6d44 | 35 エントリ（移動先は異なる 5 つ） |
| 0x5e9ae8 | 0x5e8da0–0x5eb3d4 | `jmp rcx`、4B 相対 | `add ecx,-0x3f34; cmp ecx,0x1e; ja 0x5eaf38`。0〜30、N = 31。基底 r15 | 31 エントリ（すべて異なる） |
| 0x5f1499 | 0x5f05f0–0x5f167c | `jmp rax`、4B 相対 | `cmp edx,5; ja 0x5f161c`。0〜5、N = 6。基底 rcx は直前の `lea rcx,[rip+0x1d2]` = 0x5f1664 | 6 エントリ |

- 0x5e9ae8 の基底 r15 は、jmp の直前には設定されていない（関数内 17 か所の `lea r15,[rip+0x...]`、すべて 0x5eb358）。関数入口から jmp までの全経路で r15 = 0x5eb358 であることを、`objdump` の命令列に対する自作の must-constant 解析（`out-ijmp/r15proof2.py`、join は積集合、call は Win64 の非揮発レジスタを保存すると仮定、揮発レジスタを破壊とみなす）で確認した（2,633 命令中 2,632 到達、証明成功）。**前提**: 被呼び出し側が r15 を保存すること（Win64 の規約）。自作の解析なので、解析の誤りは否定できない（`r15proof.py` の最初の版は、ループ内の `mov r15,rsi ... mov r15,r12` の対（r15 の退避と復元）を扱えず失敗し、2 版目で寄せた）。
- 範囲外は abort（`fle_jumptab_miss(jmp_va, selector)`、`fle_dispatch.h` に追記）。`switch` の選択子はハードウェアが実際に使う値: `jmp [ea]` では実効アドレス（case はスロットの VA）、`jmp reg` ではレジスタ（case は移動先の VA）。
- 検査（`fe_decode::jumptab::resolve`）: jmp の位置の命令バイトが注釈と一致、jmp と表と全移動先が注釈の関数の範囲内かつ `.text` 内。**`verify`（閉包完成後）**: 全移動先がデコード済み命令の開始で、その命令の途中に入らず、他の命令がその中から始まらない。失敗したら表を捨てて jmp は `Unresolved`のまま（`Closure::jumptab_rejected` に理由）。「検査に失敗したら実装せず報告」の実装。この DLL の 4 件はすべて通った。
- 閉包への伝え方: `fe_decode::analyze_with_specs(pe, extra, specs)` を追加。`analyze_with_seeds` は `FLENGINE` を渡す。`Resolver` が `Unresolved` を返した間接 jmp が Spec に載っていれば、表を読んで全移動先を block として積む（旧実装の `jt.rs` と同じ考え方。この新リポジトリの構造に合わせて書いた）。`Analysis::jumptabs`（jmp VA → `JumpTable`）に残し、`fe-lift` の `traverse` が移動先を辿り、`generate` が `switch` を出す。`resolved` 上は `Unresolved` のまま（`Resolved` の形は変えていない）。
- `fe-lift::generate_with_specs(data, extra, specs, w)` を追加（`generate_with_seeds` はこれに `FLENGINE` を渡す）。合成 PE のテストが任意の Spec を渡すために必要。

## 副作用（意図した変更）

- 既定の閉包（`analyze`）が +10 命令（`f_40d210` の表の移動先 0x40d2ba〜0x40d2d0）、all-pdata が +3,632 命令（4 つの関数内の表の移動先。関数数は不変）。既存のテスト `fe-decode/tests/extra_seed.rs`、`pdata_seeds.rs` の期待値を更新した（コメントに元の値）。
- 新しく閉包に入った命令に未対応の命令が 9 ある（`f_5e8da0`: Test 7（high-byte）、Bswap 2）。`unsupported_total` はそれだけ増える（実測は run 記録）。

## 検討した代替

- 汎用パターン認識（`cmp`/`ja` から範囲を導く）: 入れない。誤認識 0 を母集団で示す作業が要り、依頼は「まず 4 か所を確実に」だった。
- 表の全エントリを `goto` する `switch` ではなく、添字範囲の記述（min/max）を持つ: 選択子を「ハードウェアが使う値」にしたほうが、基底レジスタの値が想定と違えば abort で検出できるので、case を値で持つ形にした。
- `jmp [mem]` を W1 に含める: メモリから読む値は IAT の外の関数ポインタとは限らない（`f_4126f0` の `jmp [rcx]` は longjmp）。対象外。

## 確認できていないこと

- 4 件以外の jump table 風の jmp が閉包にあるか。all-pdata の `register_jmp_sites` は 20 箇所（遅延ロードヘルパの `jmp rax` 17 + switch 3）で、`jmp reg` はそれで全部。メモリ形の解決不能な jmp は `Jmp: indirect target unresolved` の 1 件（`f_4126f0`）だけ。ただし、`Resolved::Code`（既存の解決器が解いた表）の jmp の移動先が、表の範囲として正しいかは今回確認していない。
- `f_40d210` の呼び出し側が、表の範囲（rdx = 1〜8 のとき）以外の値で表の jmp に来ることは、命令列上ありえない（ガードの導出）。ただしガードの導出は人の読みで、ホスト突合（`tests/jumptab.rs`）が n = 0〜0x60 の全値を native と lifted で比較した範囲でのみ裏付けている。
- 実 Windows での挙動。
- r15 の解析（上記）は自作。Win64 の非揮発レジスタの保存は仮定。
