# 解決不能な間接 jmp の実装（W1 tail dispatch、W2 jump table）の実行記録

日付: 2026-10-20。決定は `docs/decisions/0036-indirect-jmp.md`、仕様は `docs/investigations/unsupported-inventory-pdata.md` §1.1、§6 W1/W2。成果物は `lift/out-ijmp/`（`tmp/` は TMPDIR）。

「実測」はツールの出力、「推測」は未検証の解釈。

**混在の注意**: 作業ツリーには他の担当（x87、sysinfo、a5 oracle v2）の未コミット変更が混在している（`crates/fe-lift/src/x87.rs`、`runtime/fle_x87.h`、`tests/x87.rs`、`runtime/fle_sysinfo.c`、`imports_table.rs`、`fle_mmx.h` ほか）。以下の生成・ビルド・実行・テストは、すべてその混在状態の作業ツリー（HEAD fe9a1f6 + 他担当の未コミット + 私の変更）で行った。

## 1. 範囲の導出（objdump、実測）

`objdump -d -M intel` の逆アセンブルと、PE のバイトから読んだ表（`out-ijmp/tables.py`、`ranges.py`。表の内容は生成器の出力ではなく、独立の Python が DLL から読んだ）。

- **f_40d210 0x40d253 `jmp [r10+rdx*8+0x40]`**: 0x40d221 `test rdx,rdx; jle 0x40d2b9`（ret）、0x40d238 `cmp rdx,0x20; jg 0x40d2d1`、0x40d242 `sub rdx,8; jg 0x40d2a0`。rdx は 1〜8 でここに来て、`sub` 後は −7〜0。r10 = 0x40d258（0x40d24c `lea r10,[rip+0x5]`、jmp の直前）。スロットは 0x40d260〜0x40d298（8 つ）。idx −8 のスロット 0x40d258 は 0x40d2b9（ret）を指すが不達。8 つの移動先は 0x40d2b6〜0x40d2ce の ret 付き断片（前任の `jmp_table_40d210.txt` と一致）。**前任は「表 0x40d258〜0x40d298 の 9 エントリ」と書いたが、注釈に入れたのは不達の idx −8 を除く 8 つ。**
- **0x5e5eb6**（fn 0x5e5860–0x5e6dd0）: 0x5e5e9c `add ecx,0xffffffbb; cmp ecx,0x22; ja 0x5e63ae`、0x5e5ea8 `lea rax,[rip+0xe95]` = 0x5e6d44、`movsxd rcx,[rax+rcx*4]; add rcx,rax; jmp rcx`。N = 35。表 0x5e6d44〜0x5e6dd0。移動先は 5 種（0x5e5eb8、0x5e63ae、0x5e6030、0x5e615c、0x5e627f）。
- **0x5e9ae8**（fn 0x5e8da0–0x5eb3d4）: 0x5e9ac8 `add ecx,0xffffc0cc; cmp ecx,0x1e; ja 0x5eaf38`、`movsxd rcx,[r15+rcx*4]; add rcx,r15; jmp rcx`。N = 31。表 0x5eb358〜0x5eb3d4。移動先 31 種（0x5e8f87〜0x5eb037）。基底 r15 は直前に設定されない: 0x5e9ac8 への飛び込みが 20 か所あり、各所の手前で r15 を設定している箇所とそうでない箇所がある。**関数入口からの全経路で r15 = 0x5eb358**（17 か所の `lea r15,[rip+...]` はすべて 0x5eb358）であることを `out-ijmp/r15proof2.py`（must-constant 解析、join は積集合、call は揮発レジスタを破壊し非揮発を保存すると仮定）で確認した（関数の命令 2,633 のうち 2,632 到達、`PROVEN`）。1 版目（`r15proof.py`、`mov r15,rsi` と `mov r15,r12` の退避・復元の対を扱えない）は `NOT PROVEN` だった。2 版目の根拠は自作の解析で、Win64 の非揮発レジスタの保存を仮定している。この DLL に r15 を変更する経路が（解析が見落とした形で）あれば、`fle_jumptab_miss` か誤った移動先になる。実行時は選択子が表の 31 の VA 以外なら abort する。
- **0x5f1499**（fn 0x5f05f0–0x5f167c）: 0x5f1480 `cmp edx,5; ja 0x5f161c`、`mov eax,edx; lea rcx,[rip+0x1d2]` = 0x5f1664、`movsxd rax,[rcx+rax*4]; add rax,rcx; jmp rax`。N = 6。表 0x5f1664〜0x5f167c。移動先 6 種。
- 4 つとも、jmp、表、全移動先が同じ pdata 関数の範囲内（`ranges.py`: 0x40d210–0x40d31b、0x5e5860–0x5e6dd0、0x5e8da0–0x5eb3d4、0x5f05f0–0x5f167c）。
- 表の直後のバイトを idx = N として読むと、表の外の値になる（0x5e5860: `-0x76593c67`、0x5e8da0: `0x2ec519be`、0x5f05f0: `0x9f3573`）。つまり N は表の実長と一致する（表がデータとして続いてはいない）。

## 2. 実装の要約

- `fe-decode/src/jumptab.rs`（新規）: `Spec`、`FLENGINE`（4 件）、`resolve`、`verify`。
- `fe-decode/src/lib.rs`: `analyze_with_specs`、`Closure::{jumptabs, register_jumps, jumptab_rejected}`、`Analysis::jumptabs`。`indirect.rs`: `Stats::move_kind`。
- `fe-lift/src/tailjmp.rs`（新規、W1）、`fe-lift/src/jumptab.rs`（新規、`switch`）、`lib.rs`（`mod` 2 行、`Jmp` の分岐、`traverse` の表の移動先、`generate_with_specs`、`Stats::{register_jmps, register_jmp_sites, jumptabs, jumptab_rejected}`）、`main.rs`（統計の出力）。`runtime/fle_dispatch.h`: `fle_jumptab_miss`。
- 既存のテストの期待値の更新（意図した閉包の増加）: `fe-decode/tests/extra_seed.rs`（460,479 → 460,489 など +10）、`pdata_seeds.rs`（2,523,194 → 2,526,826、+3,632）。

## 3. 生成の件数（実測、all-pdata、`fe-lift lift FILE OUT.c --pdata`、`out-ijmp/gen-pdata.txt`）

| 項目 | 前（生成 C、`out-pdata/gen-pdata-b.txt`。他担当の変更前のツリー） | この作業の前の状態を再現（`generate_with_specs` で specs なし + `jmp reg` は tail dispatch あり、`out-ijmp/baseline.txt`） | 後（specs あり、`gen-pdata.txt`） |
|---|---:|---:|---:|
| `Jmp: indirect target unresolved` | **62** | 2 | **1**（`f_4126f0` の `jmp [rcx]`） |
| `Jmp: indirect through IAT slot (not lifted)` | 618 | 588 | 588 |
| 閉包の命令 | 2,523,194 | 2,523,194 | **2,526,826**（+3,632） |
| 生成の emitted_instructions | 2,523,994 | 2,523,994 | 2,527,626 |
| `unsupported_total` | **2,991**（54 種類） | 690（18 種類） | **698**（19 種類） |
| register 形 jmp の箇所（`register_jmp_sites`） | （統計になし） | 20 | 20 |
| register 形 jmp の occurrence（`register_jmps`） | （統計になし） | `jmp_ind:reg_dispatch` 60 | `jmp_ind:reg_dispatch` **57**、`jmp_ind:table` **3**、unsupported 0 |
| 表の jmp（`jmp_ind:table` の `handled`） | 0 | 0 | **4**（register 形 3 + `f_40d210` のメモリ形 1） |

- 62 → 1: 実装した jmp は **61**（`jmp reg` の tail dispatch 57 + switch の表 3 + `f_40d210` の表 1）。残る 1 は `f_4126f0`（W3、名前付き abort のまま。生成 C の `L_412752: fle_unsupported`）。
- 「register 形 jmp」は 20 箇所（遅延ロードヘルパの `jmp rax` 17 + switch 3）で、occurrence は 60（17 か所が 57 thunk に展開 + switch 3）。`jmp [mem]` は含まない。A2 の盲点（閉包に `jmp reg` が何件あるか）は **20 箇所**。
- 「前」の `unsupported_total` 2,991 は、他担当の変更（x87、VEX など）が入る前の生成 C（`gen-pdata-b.txt`）。2,991 → 698 の差は私の変更だけではない: 他担当の変更（x87 の 2,193 行、SSE の packed など。x87 は `src/x87.rs` が作業ツリーにある）も含み、私の分だけは分離できない。私の変更だけの効果は、同じツリーで specs の有無だけを切り替えた測定（`baseline.txt`）で見る: 
  - specs なし（`jmp reg` は tail dispatch される）→ あり: `Jmp: unresolved` 2 → 1、表の 3 件の `jmp reg` は `reg_dispatch` から `table` へ（60 → 57 + 3）、閉包 +3,632 命令。
  - 「specs なし」でも switch の 3 件の `jmp reg` は W1 で tail dispatch されるので、**W1 単独の効果**（Jmp unresolved 62 → 2。うち 60 が `jmp reg`）は、この測定の「specs なし」の列から読める。2 の内訳は `f_4126f0` の `jmp [rcx]` と `f_40d210` の `jmp [r10+rdx*8+0x40]`（`jmp reg` でなく、メモリ形）。ただし switch の `jmp reg` 3 件は specs なしでは tail dispatch で `fle_dispatch_miss` になる（移動先が関数入口でない）ので、specs なしの生成 C は実行すると switch で止まる。
- specs あり: `unsupported_total` が 690 → 698（**+8**）。内訳（実測、`baseline.txt` と `gen-pdata.txt` の一覧の差）: 表の移動先が新しく閉包に入れた命令のうち未対応のもの +9（`Test: high-byte register` 0 → 7、`Bswap` 5 → 7。すべて `f_5e8da0` 内）と、`Jmp: indirect target unresolved` 2 → 1（−1、`f_40d210` の表 jmp が解決された分）。+9 − 1 = +8。

## 4. テスト

### 4.1 新規（`lift/out-ijmp/target` の cargo、オフライン）

- `fe-lift/tests/tailjmp.rs`: **4 件**。合成 PE（5 関数）を C にして clang -O0 でビルドして実行: `jmp rax`（T1）と `jmp r11`（T2）が、レジスタの値の関数へ飛ぶ（周囲のレジスタに別の関数の VA を入れて取り違えを検出）、RSP とスタック 8 ワードが不変、関数入口でない値（0x1234、入口 + 1）は `not a known function entry` で abort、`jmp [rcx]` は `unsupported instruction at 0x140001040` のまま、`handled`/`register_jmps` の件数。
- `fe-lift/tests/jumptab.rs`: **5 件**。(1) 合成 PE の 2 形（`jmp [r10+rdx*8+0x40]` と `lea; movsxd; add; jmp rcx`、後者は移動先が重複する 4 エントリ）。`harness/image.c` で合成イメージを ImageBase に mmap し、同じ命令を native（ms_abi の関数呼び出し）と lifted で実行し、全添字（F1: −7〜0、F2: 0〜3）で一致、かつ表のバイトから決めた期待値と一致。表外の添字（−8、1、100、4、100）は lifted が `outside the table` で abort し、何も出力しない。(2) 実 DLL: 4 つの関数を生成 C から切り出し、各表の移動先とガードの範囲外の先に命中プローブを埋め、ガードを含む block へ入る `goto` を挿入して、native 側は同じアドレスに 0xCC を置き SIGTRAP で到達アドレスを取る。全添字を native と lifted の両方で実行し、**両方が表のバイトから読んだ移動先と一致**（A: 0〜36 = 37 件、B: 0〜32 = 33 件、C: 0〜7 = 8 件、D: n = 1〜8 の 8 件。計 86 件。範囲外の添字（A の 35、36 など）はガードの範囲外の先（0x5e63ae、0x5eaf38、0x5f161c）に着く）。f_40d210 は `memset` として n = 0〜0x60、オフセット 6 通り、値 4 通りの 2,328 ケースを native とホストの memset と lifted で比較し、不一致 0。
- `fe-decode/tests/jumptab.rs`: **5 件**。合成 PE（表の移動先が閉包に入る、範囲外の移動先で表を捨てる、移動先が命令の途中で表を捨てる、命令バイトの不一致で表を捨てる）と、実 DLL（4 件が検査を通過、全体で `jumptab_rejected` が空、移動先がすべて 4 つの関数の範囲内、decode 失敗 0）。
- 実 DLL の 2 本は `FLENGINE_PE`（なければ `binaries/win64/FLEngine_x64.dll`。明示指定でファイルが無ければ失敗、既定のパスだけが無ければスキップ）。このマシンでは DLL があるのでスキップなしで実行された。

### 4.2 変異テスト（検出力。結果は `out-ijmp/test-mut*.txt`）

| 変異 | 結果 |
|---|---|
| `Spec` の N（0x5e5eb6）を 35 → 34 | fe-lift/tests/jumptab.rs の実 DLL 2 件、fe-decode/tests/jumptab.rs の実 DLL 1 件が失敗（`label for 0x5e627f in f_5e5860`、`mut1`） |
| `switch` の case 値に +8 | fe-lift/tests/jumptab.rs 5 件すべて失敗（`mut2`） |
| tail dispatch の `return;` を外す | tailjmp.rs 2 件失敗（生成テキストと、実行テストの「飛んだ先の後に 0x99 を返すブロックへ落ちる」）（`mut3b`）。1 回目の変異（`mut3`）はテキストを見る 1 件しか失敗せず、実行テストは通過した。実行テストの T1 に、jmp の後に実行されるブロックを足して検出できるようにした |
| tail dispatch で `R[11]` を `R[10]` に取り違える | tailjmp.rs 2 件失敗（`mut4`） |

変異は元に戻して再実行し、それぞれ全件通過を確認した。

### 4.3 ワークスペース全体（`out-ijmp/test-ws.txt`、`cargo test --offline --workspace --no-fail-fast`、EXIT=101）

- 60 バイナリ、**531 件 passed、8 件 failed**、0 ignored。
- 失敗 8 件はすべて他担当の x87 のテスト: `fe-lift/tests/x87.rs` の 7 件（`clang: error: no such file or directory: runtime/fle_x87.c`。作業ツリーに `runtime/fle_x87.c` がまだ無い）、`fe-lift/tests/x87ctl.rs` の `x87_arithmetic_stays_unsupported` 1 件（`lift_body(&[0xd9, 0x07]).is_err()` が成り立たない。x87 の命令が lift されるようになったため）。私の変更に起因するものはない（根拠: 失敗の原因が上記で、私が触っていないファイルと機能。**推測を含む**: 私の変更を外した状態では未測定）。
- その後、tailjmp.rs の T1 の変更とその変異の確認のあとで、`tailjmp` と `jumptab`（fe-lift）が 4 件・5 件、`fe-decode` 全体が 22 件で通過することを再確認した。ワークスペース全体は再実行していない。
- 私の追加分: tailjmp 4 + fe-lift/jumptab 5 + fe-decode/jumptab 5 = 14 件。

## 5. 実行（harness）

### 5.1 生成・ビルド

- `fe-lift lift ... --pdata`（`gen-pdata.txt`）→ `split_c.py`（16 分割、3,700 関数/ファイル。`out-pdata` のスクリプトの複製）→ `build-parts.sh`（`out-ijmp/build-parts.sh`。`out-pdata` の複製に、x87 のスタブのリンクを追加）。`fle_imp_*` 173 個はすべて `runtime/*.c` のどれかに定義されている（`imp-defs.txt`、MISSING 0）。
- **x87 のランタイム `runtime/fle_x87.c` が作業ツリーに無い**ため、生成 C が呼ぶ `fle_x87_*` の未定義を埋めるスタブ（`out-ijmp/fle_x87_stub.c`、呼ばれたら abort して名前を出す）を測定用にリンクした。x87 の命令を実行する経路に着くとスタブで止まる。製品のコードではない。
- ビルド 32.8 秒、最大 RSS 1,201 MB、EXIT=0（`build.txt`、`build.err`）。生成 C は 217,968,523 バイト、2,880,335 行。

### 5.2 停止点（実測、`run.txt`）

| 項目 | 前（`out-pdata/run-pdata-b.txt`） | 後（`out-ijmp/run.txt`） |
|---|---|---|
| 停止点 | 0x41cfa0（`RaiseException`、遅延ロードの GetProcAddress が NULL） | **`VerifyVersionInfoW`**（`fle: VerifyVersionInfoW: dwOSVersionInfoSize is not sizeof(OSVERSIONINFOEXW) (not defined by the MSDN page; not implemented)`、`fle_osver.c`） |
| 関数実行数（distinct） | 140 | **147** |
| 関数入口 | 708 | 736 |
| 呼び出し深さ | 9 | 7 |
| スタック（上が最も内側） | f_41cfa0 ← f_41d9a0 ← f_41cfc0 ← f_4236f0 ← f_454090 ← f_4120c0 ← … | f_423cc0（0x423cc0 の `jmp [IAT 0x1971620]` = `VerifyVersionInfoW`）← f_454030 ← f_454090 ← f_4120c0 ← f_412160 ← f_41d8f0 ← 入口 f_1376820 |

- gdb（`gdb-tail.cmds`、`gdb-tail.txt`）で、遅延ロードのヘルパが tail dispatch で `fle_imp_GetLogicalProcessorInformation`（2 回、f_407320 内の 0x4072ff の `jmp rax` 経由、`fle_dispatch` 経由）と `fle_imp_GetNativeSystemInfo`（f_4236f0 内の 0x42316f の `jmp rax` 経由）に実際に着いたことを確認した。`fle_dispatch_miss` には着いていない。**停止点が 0x41cfa0 から先へ進んだのは、W1 と、別担当の GetNativeSystemInfo 等（sysinfo、`fle_sysinfo.c`）の両方の効果**（W1 だけで進んだかは、sysinfo を外した状態では測っていない）。
- 次の停止点は `VerifyVersionInfoW` の呼び出しで、`dwOSVersionInfoSize` がゲストの渡した値で OSVERSIONINFOEXW のサイズでないため、`fle_osver.c` が名前付きで止めている（私の担当外。ゲスト側の引数の確認はしていない）。f_40d210 の表 jmp と switch の 3 つは、この実行では到達しなかった（停止点の前に踏んだかは、これらの命中を数えていない）。

## 6. 制約の遵守と残置物

- `rm`、`>` による既存ファイルの上書き、git 操作、`/tmp`、sleep/timeout は使っていない。TMPDIR は `lift/out-ijmp/tmp`。cargo の target は `lift/out-ijmp/target`（`CARGO_TARGET_DIR`。ワークスペース外の拒否は無かったが、他担当の cargo と衝突しないため）、測定用は `target-scratch`。権限拒否はなし。2 GB 級の中間ファイルなし（最大は `out/ijmp-pdata.c` 218 MB と分割 16 個の合計 218 MB、`parts/*.o`）。
- 編集した既存ファイル: `fe-decode/src/lib.rs`、`fe-decode/src/indirect.rs`（追記）、`fe-decode/tests/extra_seed.rs`、`pdata_seeds.rs`（期待値）、`fe-lift/src/lib.rs`（`mod` 2 行、`Jmp` 分岐の 1 箇所、`traverse` の引数と分岐、`generate_with_specs`、`Stats` の項目、命令ループ）、`fe-lift/src/main.rs`、`runtime/fle_dispatch.h`（追記）。`lib.rs` は他担当が編集中だったため、編集のたびに読み直した。`imports_table.rs` は触っていない。
- 後始末できていないもの（削除禁止）: `out-ijmp/build-parts.sh.bak`（`sed -i.bak` が作った。内容は同じ）、`out-ijmp/delta.rs.txt`（空）、`out-ijmp/stub_marker.txt`（スタブの複製。不要）、`out-ijmp/r15proof.py`（1 版目。失敗する）。`out-ijmp/test-mut1-count.txt` などの変異のログ、`dis-tmp-*.txt`。
- `out-ijmp/gen-pdata.err` は空。

## 7. 触ったファイル

新規: `lift/crates/fe-decode/src/jumptab.rs`、`lift/crates/fe-decode/tests/jumptab.rs`、`lift/crates/fe-lift/src/tailjmp.rs`、`lift/crates/fe-lift/src/jumptab.rs`、`lift/crates/fe-lift/tests/tailjmp.rs`、`lift/crates/fe-lift/tests/jumptab.rs`、`docs/decisions/0036-indirect-jmp.md`、`docs/investigations/indirect-jmp-run.md`、`lift/out-ijmp/`（作業物一式）。
変更: `lift/crates/fe-decode/src/lib.rs`、`lift/crates/fe-decode/src/indirect.rs`、`lift/crates/fe-decode/tests/extra_seed.rs`、`lift/crates/fe-decode/tests/pdata_seeds.rs`、`lift/crates/fe-lift/src/lib.rs`、`lift/crates/fe-lift/src/main.rs`、`lift/runtime/fle_dispatch.h`。
