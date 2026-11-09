# 0051: pdata 外の this 補正 thunk（adjustor thunk）を閉包の seed にする

状態: 作業者（サブエージェント）が決定。依頼者（PM）の判断は含まない。調査は `docs/investigations/adjustor-thunks.md`、実行は `docs/investigations/adjustor-thunks-run.md`。

## 背景

ネイティブ harness は `fle_dispatch_miss`（`indirect call to 0x403582 is not a known function entry`）で止まっていた（`event-sync-run.md` §5）。0x403582 は `add rcx,-0x10; jmp 0x41c1a0`（9 B）で、`.pdata` の範囲に入らず、直接 call もされず、`.text` 内の表（0x403598、DIR64 の再配置付き）からだけ参照される。`pdata_seeds`（決定 0031）にも、既定の seed（エントリ、export）にも入らないので、関数入口の表 `fle_dispatch_table` に無い。

依頼文が前提にしていた「ADJUSTOR の seed 方針」「`fe-data/src/roots.rs`」は、このリポジトリに無かった（`adjustor-thunks.md` §0）。そこで「既存の方針の拡張」ではなく、決定 0031 の隣に新しい seed の種類として足した。

## 決定

1. **seed に、`.pdata` 外でデータ（DIR64 再配置のスロット）から参照される短い this 補正 thunk を足す。** 次の 4 条件を、すべてデコードしたバイトで確かめたものだけを採る。
   1. DIR64 の再配置が付いた 8 B スロットの値である（`Pe::dir64_relocation_vas`。全セクションのスロットを対象にする。thunk を指す 602 スロットは全部 `.text` 内にある）。
   2. `.text` 内で、`.pdata` の開始でも、いずれの RUNTIME_FUNCTION の範囲の内側でもない（開始は既に seed、範囲の内側は関数の途中）。
   3. 補正命令 1〜3 個のあとに直接 near `jmp`、で終わり、全体が 16 B 以下。補正命令は `add`/`sub` の 32/64 bit レジスタと即値、`lea r64,[base+disp]`、`mov r64,[base+disp]`（index なし、rip 相対でない、セグメントなし）。補正命令が無い `jmp` だけのものは採らない（import や転送の thunk）。
   4. `jmp` 先が `.text` 内の `.pdata` の関数開始である。
2. **これは `--pdata` の既定に含める。** `fe-lift lift FILE OUT.c --pdata` は `.pdata` の開始 + thunk の seed。従来の `.pdata` のみの動作は `--no-adjustors` で選べる。`--pdata` なしの既定の閉包（エントリと export の 3 seed）は変えない。`fe-decode closure FILE --pdata [--adjustors]` は `--adjustors` を付けたときだけ thunk を足す（従来の `--pdata` の数値を変えないため。fe-lift とは既定が逆。理由は、fe-decode の既存テストの数値と記録を保つため）。
3. API: `fe_decode::adjustor::{adjustors, adjustor_seeds, classify, decode_thunk, Adjustor, Reject}`、`fe_decode::{pdata_and_adjustor_seeds, analyze_with_pdata_adjustors}`、`fe_pe::Pe::dir64_relocation_vas`（と `dir::BASERELOC`）。既存の関数は変更なし（`pdata_seeds`、`analyze_with_pdata` の挙動は同じ）。

## 根拠

- 実測（`adjustor-thunks.md` §2）: この条件に合うのは **602 件**。すべて `add rcx, imm; jmp rel32`（9 B が 467、12 B が 135）。`sub`、`lea`、`mov` の形で `.pdata` 外・参照ありのものは 0 件。jmp 先は 275 通りで、すべて `.pdata` の開始。
- garbage root を避けるための条件: 参照を要求する（条件 1）。参照なしで同じ形を全バイト位置から探すと 1,206 件で、参照されるのが 602 件、残り 604 件は REX 接頭辞をまたいだ命令境界でない読み（0x40357a、0x403583 など。条件 1 で落ちる）。値が `.text` に入るだけの 55,586 値のうち、条件 2〜4 を満たすのは 602 件で、データ（UTF-16、ASCII）の偶然ではこの形と「jmp 先が関数開始」を同時に満たしにくい（読み。確率は測っていない）。
- 補正命令を要求する（条件 3）: 補正なしの `jmp` を許すと、`.didata` の `push rdx; lea; jmp` などとは別に、IAT の thunk が入る。これは従来から閉包に入る（決定 0031）か、別の扱い（`imports_table`）。
- 逆に、ここで seed にしないもの: 範囲の内側の 2 件（0x40c6fc、0x882586。`.data` から参照されるが関数の途中）、`cwde; ret` 系 606 件（データの偶然と読む）、`push rdx; lea; jmp` 57 件（`.didata` 内の遅延 import の stub と読む。推測）。

## 検討した代替

- 表（3 要素の連）単位で seed にする: 採らない。連は表と一致するとは限らず、要素が thunk でない関数のこともあるので、アドレスごとに判定する。
- 決定 0031 の seed に `jmp` 先だけ足す: 採らない。jmp 先はすでに `.pdata` の開始で seed。足りないのは thunk 自身で、これは実行時に「間接 call の先」としてだけ現れる。
- 範囲の内側の thunk 形（2 件）も採る: 採らない。関数の途中の命令で、入口として dispatch に入れると、別の関数の途中に飛び込む意味になる。

## 影響（実測。詳細は run 記録）

- `.pdata` seed 57,816 に 602 を足す（seed 58,418）。閉包の関数 58,695 → 59,297（+602 = thunk のみ。jmp 先はどれもすでに関数）、命令 2,526,826 → 2,528,030（+1,204 = 602 × 2）。`decode_failures` 0、`out_of_text_*` 0、生成の `bad_nodes` 0、`unsupported_total` 554（変わらない）。
- harness の停止点: `fle_dispatch_miss`（0x403582）→ `VirtualAlloc flAllocationType 0x101000 is not supported`。

## 確認できていないこと

- 602 件が実行時に呼ばれるか（0x403582 がこの実行で呼ばれたかは、entry log が先頭 256 件なので、見えていない）。
- この thunk 群が Delphi の interface の vtable かどうか（読み）。
- 再配置が付かない参照元（実測では、thunk 形で `.pdata` 外のものは 0 件）。再配置の無い DLL（ベース固定）では条件 1 が働かない。この DLL は再配置あり。
- 実 Windows との比較。
