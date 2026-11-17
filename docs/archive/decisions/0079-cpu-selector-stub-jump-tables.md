# 0079: CPU 振り分けのスタブ（`movsxd; lea; jmp [r10+rax*8]`）を、再配置から読んだ表の `switch` にする（W3(d)）

状態: 作業者（サブエージェント）が決定。PM の決定は含まない。方向は決定 0073、作業は `docs/investigations/dsp-ippv2-lift-feasibility.md` §5、§9 の W3(d)。仕組みは決定 0036（注釈つき jump table）の `Select::Slot` の `switch` を流用した。**FLEngine_x64.dll の生成 C は変わらない**（SHA-256 一致、下の実測）。

## 1. スタブの形（実バイト列の確認）

`lift/out-ext/dsp_ippv2_x64.dll`（SHA-256 497dd4f7…c352、ImageBase 0x180000000）を、`.text` の実行セクションから `41 ff 24 c2`（`jmp [r10+rax*8]`）を探し、直前 14 バイトが次の形のものを数えた（自作の python。capstone と pefile）。434 個。

```
180001840  e8 cb 7f 83 01        call 0x181839810        ; 選択値の初期化（stub-16）
180001845  66 66 66 0f 1f 84 00 00 00 00 00   nop word ptr [rax+rax]   ; 11 バイト
180001850  48 63 05 a9 f7 53 02  movsxd rax, dword ptr [rip+0x253f7a9]   ; = [0x182541000]（選択値）
180001857  4c 8d 15 ca f7 53 02  lea r10, [rip+0x253f7ca]                ; = 0x182541028（表）
18000185e  41 ff 24 c2           jmp qword ptr [r10+rax*8]
```

- 434 個すべてで、スタブの直前 16 バイトが「`call rel32`（5 バイト）+ 11 バイトの `nop`」（例外 0）。スタブ自身（movsxd の位置）は `.pdata` の開始ではない（434 個とも）。432 個はエクスポート。
- 選択値の変数（`movsxd` の読み先、すべて `.data`、ファイル上の初期値は 4 つとも `0xffffffff`）: `0x182541000`（377）、`0x18255d050`（33）、`0x1825410a0`（20）、`0x18254f270`（4）。調査文書 §5.2 の数と一致。
- 表（`lea` の指す先）: 434 個とも別々（共有なし）。`.data` に 377、`.text` に 57。
- **添字 −1（表の 8 バイト手前）は、434 個すべてで再配置（DIR64）されたスロットで、値は「そのスタブの 16 バイト手前」（初期化の `call` の位置）**。つまり、選択値が −1 のとき、ハードウェアは初期化の `call` に入り、`nop` を経てスタブに落ちる。初期化の `call` の先は 4 通り（選択値の変数と 1 対 1）: `0x181839810`（377）、`0x181cb9f80`（33）、`0x1817e08c0`（20）、`0x181b30cb0`（4）。
- 表のエントリ数（添字 0 から、「DIR64 で再配置されたスロットで、値が実行セクション内」である間）: **6（376 表、うち 373 は表が背中合わせ）、7（57 表）、9（1 表）**。調査文書 §5.2 の 6/7/9 と一致。6 エントリの表の次のスロットは再配置なしの 0、7 エントリの `.text` の表の次は命令のバイト（再配置なし）、9 エントリの表の次はデータ（再配置なし）。
- 表のエントリのうち 1,724 件が `.pdata` の開始、940 件は `.pdata` の範囲に入らない（開始でも内部でもない）。`.pdata` の範囲の途中を指すエントリは 0。distinct な移動先は 2,555（`.pdata` 開始 1,650、開始でない 905）。

### 決定 0036 の仕組みで解けるか

| 形 | 件数 | 同じ仕組みで解けるか |
|---|---|---|
| スタブ形 `movsxd; lea; jmp [r10+rax*8]`（表は `.data` 377、`.text` 57） | 434 | **解ける**（0036 の `Select::Slot` の `switch`）。ただし **範囲の根拠が違う**: 0036 は `cmp`/`ja` のガードを人が読んで注釈にした。スタブにはガードが無い。範囲は **再配置の連続（§2）**から取る。 |
| 実行時に変数を読む形である点 | 434 | 静的に 1 つに決まらない。生成 C は 選択値を実行時に読み、`switch (表のスロットの実効アドレス)`。範囲外は `fle_jumptab_miss`。 |

## 2. 実装

- `fe-decode/src/jumptab.rs`:
  - `recognize_stub(text, va, jmp)`: 次の形だけに一致する（注釈の `Spec` は要らない）。`jmp [B+I*8]`（変位なし、RIP 相対でない、`fs`/`gs` なし、B と I は 64 ビットで別物）、直前 7 バイトが `lea B,[rip+表]`、その前 7 バイトが `movsxd I,dword ptr [rip+選択値]`。
  - `resolve_stub(pe, text, relocs, form)`: 表の添字 0 から、`.reloc` の DIR64 スロットで、値が実行範囲内である間の連続をエントリとする。上限 `STUB_MAX_ENTRIES` = 64（これに達したら拒否）。空なら拒否（`no relocated code pointer at the table address`）。`JumpTable { select: Slot, cases: (スロット VA, 移動先), origin: Stub { sel_var, table } }`。
  - `verify_stub`: 閉包の完成後、`movsxd` と `lea` が順にデコードされていて、`lea` と `jmp` の位置が**ブロックの先頭になっていない**ことを確かめる（途中に分岐が入る場合、レジスタの前提が崩れる）。失敗したら表を捨て、jmp は未解決のまま（`jumptab_rejected` に理由）。既存の `verify`（移動先が命令の開始）も通す。
  - `JumpTable` に `origin: Origin`（`Annotated`／`Stub`）を追加。`kind_key()`（統計のキー: `jmp:table`／`jmp:sel_table`）。
- `fe-decode/src/lib.rs`: 未解決の jmp が `Spec` に載っていなければ `recognize_stub` を試す。`Closure::stub_tables`（スタブ形の件数）を追加。`.reloc` の DIR64 は最初のスタブで 1 回だけ読む。
- `fe-lift/src/jumptab.rs`: ハンドラ名を `jmp_ind:sel_table`（スタブ形）／`jmp_ind:table`（注釈つき）に分けた（5 行）。`switch` の生成は共有。`fe-lift/src/lib.rs` は変更なし。
- **表のエントリは、ブロックではなく関数（`fn_queue`）として閉包に積む。** スタブの jmp は tail jump で、移動先は実装関数の入口。`ret` は呼び出し元へ戻る（0036 の `{ f_X(); return; }` の形）。
- 生成 C の例（実物、`f_180001850`）: `{ uint64_t t = (R[10] + (R[0]*8)); switch (t) { case UINT64_C(0x182541028): { f_1800018a0(); return; } … default: fle_jumptab_miss(UINT64_C(0x18000185e), t); } }`

## 3. 範囲外の扱い

- 選択値の範囲はスタブから静的には分からない（ガードが無い）。**エントリ数 N（上の再配置の連続）の外は名前付き停止**: `fle: jump table at 0x… : selector 0x… is outside the table`（`fle_jumptab_miss`、既存。`fle_dispatch.h` は変更なし）。
- **添字 −1（選択値の初期値 `0xffffffff`）も停止する**。実機のコードは −1 のとき初期化の `call` に入ってからスタブに落ちる（上の実バイト列）が、これは表の添字 0〜N−1 の外なので、依頼どおり停止にした。したがって、**選択値が初期化される前にスタブが呼ばれると、生成 C は止まる**（実機は初期化して進む）。DllMain の `DLL_PROCESS_ATTACH` が `0x1040` を先に呼ぶこと（調査文書 §5.2 の 4）に依存している。−1 の経路（初期化の `call` + `nop`）をたどる形にするかは決めていない（表の添字 −1 が再配置されたスロットであることは、434 個で確認した）。
- 表のエントリ数は再配置の連続であって、選択値の実際の範囲ではない。選択値が 0〜5（調査文書 §5.2。読み）のとき、6 エントリの表は添字 0〜5 で足りる。7 エントリ、9 エントリの表で、添字 6〜8 に選択値が届くか（届かなければ、その分は死んだ枝）は確認していない。

## 4. 閉包への影響（調査文書「増えないはず」の確認）

**増えた**。`--pdata` は `.pdata` の開始を種にするが、スタブの表の移動先の distinct 2,555 のうち 905 は `.pdata` の開始ではない（上）。それらが関数入口として閉包に入った。

| 項目（`fe-lift lift lift/out-ext/dsp_ippv2_x64.dll /dev/null --pdata`） | 変更前 | 変更後 |
|---|---|---|
| functions | 32,162 | 32,648（+486） |
| closure_instructions | 6,458,203 | 6,464,799（+6,596） |
| emitted_instructions | 6,504,638 | 6,511,234 |
| bad_nodes | 0 | 0 |
| decode_failures / out_of_text_flow / out_of_text_calls（`fe-decode closure --pdata --adjustors`） | 0 / 0 / 0 | 0 / 0 / 0 |
| tail_calls | 19,495 | 22,594 |
| gotos | 449,963 | 450,886 |
| c_lines | 6,698,311 | 6,707,823 |
| c_bytes | 565,516,290 | 566,289,705 |
| unsupported_total | 973,919 | 974,582（+663） |
| `Jmp: indirect target unresolved` | 617 | **183** |
| `jumptabs`（`fe-lift` の出力） | 0 | 434 |
| `jmp_ind:sel_table`（handled） | 0 | 434 |
| register_jmp_sites（閉包で到達した `jmp reg`） | 451 | 836 |
| `jmp_ind:reg_const` / `reg_dispatch`（register 形、結果別） | 57 / 365 | 441 / 366 |

- 増えた 486 関数の中の命令は、増えた V* などの未対応（+663）を含む（内訳は `lift/out-w3d/base-dll.txt` と `after-dll.txt` の差。個別の命令名は多数で、上位は V* の `Vmovsd`、`Vmovss`、`Vmovupd`、`EVEX Vmovups` など）。新しい種類の未対応の追加は、`Kandw` の順位の入れ替わり以外で確かめていない。
- 新しく閉包に入った 486 関数のうち、何個が実際に関数の入口かは確認していない。**表が指す値を関数入口として扱う（`.pdata` の開始 1,650 は確認済み、残り 905 は未確認）**。その 905 の最初の命令の列は、`push; push` 209、`mov; jmp` 432（`jmp` で別関数に飛ぶ小さな入口）、`test; je` 47 ほか。いずれも関数入口に見えるが、読みであり確認ではない。
- 閉包の「`--seeds` で系統を絞る」使い方（調査文書 §5.5）では、スタブの表の全エントリ（添字 0〜N−1。添字 6〜8 を含む）が閉包に入る。添字 0 のみに絞る設計は、スタブを通る限り崩れる（`fle_dispatch` ではなく `switch` が全エントリを関数として持つため）。実測: 種を `--pdata` なしの既定（エントリとエクスポートだけ）にしても、閉包は 7,992 関数・`jumptabs` 434（変更後。スタブ 432 個がエクスポートそのものなので、エクスポートを種にする限りスタブは全部入り、その表の全エントリが入る）。絞り込みの設計（W3(c)）は、この点を見直す必要がある。**今回は変更していない**。なお、変更前の既定の閉包の関数数（7,992 か）は測り直していない（変更後の値のみ）。

## 5. FLEngine_x64.dll

`fe-lift lift binaries/win64/FLEngine_x64.dll OUT.c --pdata` の SHA-256 は **67827424301459e6f2da1ccb46a5fd081b7de9a38c64e1bca44ba9bf5d15584d のまま**（215,700,923 B）。`jumptabs` は 4、`stub_tables` は 0（FLEngine にスタブ形の `jmp [r10+rax*8]` は無い。実行セクションの `41 ff 24 c2` を数えて 0。`recognize_stub` は形で判定するので、FLEngine でも走るが一致しない。`tests/jt_cpu.rs::flengine_has_no_stub`）。

## 6. 残りの 183 か所の形（分類のみ。実装していない）

未解決の jmp は 617 → 183。生成 C の `Jmp` の `fle_unsupported` は 190 行で、うち 7 行は別の理由（`jmp [IAT]`: `Jmp: indirect through IAT slot (not lifted)`、`call_iat` の領域）。残り 183 は次のとおり（バイト列から。自作の python）。

| 形 | 件数 | 読み |
|---|---|---|
| `jmp [R + I*8 + disp32]`、`R` は直前の `lea R,[rip - X]` で **ImageBase** になる（表は `.rdata`、`disp32` が表の ImageBase からの相対位置） | 155 | 表は 155 個（共有なし）、再配置された連続は 5〜128 エントリ（5: 7、7: 40、13〜16: 55、21: 25、28: 17、32: 7、35/42/64/128: 各 1）。**スタブと同じく再配置の連続で範囲が取れる形だが、命令の形（基底が ImageBase、`disp32` つき、`cmp`/`ja` のガードがあるものと無いものがある）が違う**ので、`recognize_stub` には含めていない。ガードの `cmp` の値が直前 6 命令に取れるもの 41 件のうち、33 件は「ガード + 1 = 再配置の連続の長さ」、8 件は「ガード + 1 < 連続の長さ」（例: ガード 15 で連続 32）。連続の長さがガードより大きくなる実例があるので、**再配置の連続は範囲の上限であって、実際の範囲とは限らない**（スタブ形の 6/7/9 でも同じ性質）。 |
| `jmp [R + I*8 + disp32]`、`R` が直前 6 命令で設定されない（`r15` を持続的な基底にしているもの、`sub edx,esi; dec edx; movsxd rdx,edx` の後で `rcx`/`rsi`/`r11` を基底にするもの） | 15 | 基底の値の証明（決定 0036 の r15 の解析と同種）が要る。実装していない。 |
| `jmp [rax + 0x50]`、`jmp [rax + 0x338]` など（`mov rax,[rsi]; … pop rdi; jmp [rax+disp]`） | 13 | 仮想関数表の呼び出しに見える tail call（読み）。移動先は実行時に決まる。表の範囲は無い。静的には解けない（`fle_dispatch` の形で動的に引くのは別の設計: `jmp [mem]` を `fle_dispatch(ld(ea))` にするか、決定 0036 の W1 の対象外とした理由「メモリから読む値は IAT の外の関数ポインタとは限らない」を再検討する必要がある）。 |

## 7. テスト

- `lift/crates/fe-decode/tests/jt_cpu.rs`（6 件）: 合成 PE（`.text`、`.data`、`.reloc`）。スタブ 2 つ（表 A は `.data` に 4 エントリ、表 B は `.text` に 3 エントリ `T2,T0,T2`）。エントリ数が再配置の連続であること（添字 −1 の再配置スロットと、次の「再配置されているがデータを指す」スロット、「`.text` にあるが再配置されていない」スロットを含まない）、移動先が閉包の関数になること、先頭が再配置されていない表の拒否（`jmp:unresolved` に戻る）、変位つきの jmp はスタブでない（拒否ではなく未一致）、スタブの途中への分岐で拒否、FLEngine にスタブなし、実 DLL で 434 件（選択値の変数別 377/20/4/33、エントリ数別 6:376、7:57、9:1、未解決 183 は「jmp:unresolved」549 のうち register 形 366 を除いたメモリ形）。
- `lift/crates/fe-lift/tests/jt_cpu.rs`（5 件）: 合成 PE を clang でビルドして native でも実行（`harness/image.c` で ImageBase に写す）。選択値 k ごとに native と生成 C の結果が同じで、表のバイトが指す値になること。選択値 −1、N、N+1、100 で `fle_jumptab_miss`（標準出力に何も出さない）。実 DLL（`DSP_IPPV2_PE` または `lift/out-ext/dsp_ippv2_x64.dll`。無ければ何もせず通る）のスタブ 3 個（`0x180001850`: `.data` に 6、`0x180001930`: `.text` に 7、`0x181056520`: `.data` に 9）で、選択変数をイメージに書き、移動先を確認用の関数に置き換えた生成 C と、本物の命令（移動先に 0xCC を置き SIGTRAP で到達先を取る）が、選択値 k のとき表[k]（ファイルの表のバイトから読む。生成器からではない）に着くこと（計 44 回: 22 点 × native と生成 C）。選択値 −1、N、N+1、100、`i32::MIN` で名前付きに停止すること。**選択値 0 のとき表[0]（`0x1800018a0`、`0x180001a30`）に着くこと**は、この選択変数を直接書いた実行で確認した。
- 既存の表明は変更していない。

### 全体テスト

`cd lift && CARGO_TARGET_DIR=…/lift/out-w3d/target FLENGINE_PE=…/binaries/win64/FLEngine_x64.dll cargo test --workspace --no-fail-fast`: 終了コード 0、**失敗 0、通過 1,061 件、結果行 92 個**（基準 1,050 件・90 バイナリ + 本件の新規 11 件・2 バイナリ）。ログ `lift/out-w3d/test-workspace.log`。

a4 survey（`OUT=lift/out-w3d/surv` に、この変更で生成した FLEngine の C から native を作り直して実行）: 停止点 `LoadLibraryW: module "dsp_ippv2_x64.dll" …`、`init_count=145 / 942`、関数の実行 821（distinct）、`started=132 completed=131 in_progress=1 of 895 runnable`。変更前と同じ。

## 8. 確認できていないこと

- **実行時に選択値が 0 になること**（調査文書 §5.4 の仮説）。DllMain を通して実行していない。上のテストは、選択変数に値を直接書いた実行。
- 選択値の初期化前（−1）の経路は停止にした（§3）。実機は初期化の `call` を経て進む。
- 添字 6〜8 に選択値が届くか。表の添字と命令集合の対応（調査文書 §5.3）は読みのまま。
- 閉包に新しく入った 905 の非 `.pdata` の移動先が、すべて本当の関数入口か（§4）。
- `.reloc` の無い PE では、スタブ形に一致しても表が空になり、拒否される（`no relocated code pointer at the table address`）。実在の DLL 2 つ（FLEngine、dsp_ippv2）でしか確かめていない。
- 残りの 183 か所の個別の読み（§6）は、直前の命令列とリロケーションの連続の統計のみ。個々の jmp の移動先の正しさは確認していない。
- 生成 C 全体（DLL）を clang で通していない（統計のみ。`/dev/null`）。3 つのスタブを切り出した C のみビルド・実行した。
- 実 Windows での挙動。

## 作ったファイル

- `lift/crates/fe-decode/src/jumptab.rs`、`lift/crates/fe-decode/src/lib.rs`（変更）
- `lift/crates/fe-lift/src/jumptab.rs`（変更、5 行）
- `lift/crates/fe-decode/tests/jt_cpu.rs`、`lift/crates/fe-lift/tests/jt_cpu.rs`（新規）
- 調査用スクリプトとログ: `lift/out-w3d/`（`stubscan*.py`、`tabscan.py`、`classify183.py`、`ctx183*.py`、`newfuncs.py`、`pick.py`、`fle_scan.py`、`base-dll.txt`、`after-dll.txt`、`after-fle.txt`、`test-workspace.log`、`surv/` ほか）

## 訂正（2026-10-24）: 「`.pdata` 開始でない 905」の数え直し

本節は追記である。上の本文は書き換えていない（以下の行番号は追記前のもの）。出典は後の調査 `docs/investigations/dll-nonpdata-reloc-seeds.md`（以下「seeds 文書」）§4.5 と、そのログ `lift/out-dll-seeds/logs/`。

### 訂正する数字

- 22 行目の distinct な移動先 2,555 のうち「`.pdata` 開始 1,650、開始でない 905」は、**2,007 が開始、548 が開始でない**。2,007 + 548 = 2,555 で、distinct の総数は変わらない。原本（`lift/out-ext/dsp_ippv2_x64.dll`）とリベース済みで同じ。出典: seeds 文書 §4.5。`logs/q2_check0079.txt` の 2〜4 行目（リベース済み）と 9〜11 行目（原本）: stubs 434、entries 2664、distinct 2555、distinct in function_starts 2007、not 548。
- 原因: 0079 の測定スクリプト `lift/out-w3d/tabscan.py` は、pefile の `DIRECTORY_ENTRY_EXCEPTION` から `.pdata` の開始の集合を作る。pefile（2024.8.26）はこれを **29,274 エントリで返し終えた**。生の `.pdata` は **50,529 エントリ**（余りなし）で、読み残しは **21,255**。0079 は、読み残した後半のエントリの開始を「開始でない」と数えた。出典: seeds 文書 §4.5（50,529 は §3 の `.pdata` の記述にも）、`logs/q4_callers.txt` 9 行目（`pefile exception entries: 29274 raw entries: 50529`）。seeds 文書 §4.5 は、905 のうち読み残した分の内訳を未確認としている。この追記の作業で数えた（python。ログは残していない）: pefile の開始の集合は、生の `.pdata` の先頭 29,274 エントリの開始と一致し（`.pdata` は開始アドレス順）、distinct 2,555 のうち、生の `.pdata` 全体で開始なのが 2,007、pefile の集合で開始なのが 1,650。差の **357 件**が、pefile の読み残し（生のエントリ番号 29,274 以降。357 件の中で番号が最小のものは 34,256）にある開始で、905 = 357 + 548（どの開始でもない 548 は生の `.pdata` 全体でも開始でない）。エントリ単位では 2,095 − 1,724 = 371 の差（内訳の数え直しはしていない）。
- **pefile が 29,274 で止まった理由は未確認**。seeds 文書 §4.5 は、`parse_exceptions_directory` のソースに `break` が複数あることを読んだだけで、どれが働いたかは調べていない、と書く。
- この追記の作業で `python3 -B lift/out-w3d/tabscan.py` を再実行した。1 行目は `pdata starts 29274`、末尾の `(k, 'pdata')` の合計は 1,724、`(k, 'nonpdata')` の合計は 940 だった（標準出力のみ。ログは残していない）。同じ判定（pefile の開始の集合）で distinct を数える別の python（保存していない）でも、entries 2,664 / distinct 2,555、開始 1,650・開始でない 905 になった。つまり 0079 の数字は、pefile の 29,274 件の集合なら再現する。

### 影響しないもの

- §4 の閉包の増加（32,162 → 32,648 関数、+486）は、fe-pe の `function_starts`（全 50,529 エントリを読む）を使う `--pdata` の結果なので、**その数値は変わらない**。出典: seeds 文書 §4.5、§5.1 の表。seeds 文書の `--pdata` の閉包 (A) は 32,648 関数で、0079 §4 の「変更後」と一致した。「変更前」の 32,162 は seeds 文書に記述が無い（grep で 0 件）ので、突き合わせていない。
- 0079 の判断（スタブの表を `switch` にする）には影響しない。
- 548 件はすべて `--pdata` の閉包 (A) に既に入っている（seeds 文書 §4.4。`stub-table-entry` 548 全部）。+486 との関係（変更前の閉包に既に入っていた分があるか）は未確認。
- 17 行目の「スタブ自身（movsxd の位置）は `.pdata` の開始ではない（434 個とも）」。この追記の作業で、生の `.pdata` の全 50,529 エントリの開始と照合したところ、434 個中 0 個が開始だった（結論は変わらない。ログは残していない）。

### 読み替え

- 72 行目の「残り 905 は未確認」の 905 は **548** に読み替える。同じ行の「`.pdata` の開始 1,650 は確認済み」の 1,650 は 2,007。51 行目の「distinct 2,555 のうち 905」、106 行目の「905 の非 `.pdata` の移動先」も、同じく 548。
- 72 行目にある 905 の最初の命令の列の分類（`push; push` 209、`mov; jmp` 432、`test; je` 47 ほか）は、905 を前提にした数え方である。**548 についての同じ分類は未確認**（数え直していない）。
- 参考（分類の軸が違うので、上の数字と同じものとは確認していない）: seeds 文書 §4.4 は `stub-table-entry` 548 について、最初の `ret`/`jmp` で止まった形で 432 が `jmp reg`、12 が `rsp` を変える命令（`push -6; pop rax`）を含む、と書く。

### 表のエントリ件数の突き合わせ（22 行目の数字と、seeds 文書・ログ）

一致するもの:

- スタブ 434（7 行目）: `q2_check0079.txt` の stubs 434。
- エントリの合計 2,664（= 1,724 + 940）: `q2_check0079.txt` の entries 2664（合計のみ。内訳は下の「一致しないもの」）。
- distinct 2,555: `q2_check0079.txt` の distinct 2555。
- 「`.pdata` の範囲の途中を指すエントリは 0」: `q2_check0079.txt` の「distinct covered by some pdata range: 2007」が開始 2007 と同じ数（開始でない 548 は全部が範囲の外）。
- 表の置き場所（19 行目の `.data` 377、`.text` 57）: `logs/q2_why0079.txt` 11 行目。

一致しないもの:

- distinct の開始 1,650 → 2,007、開始でない 905 → 548（上のとおり）。
- エントリ単位の開始 1,724 → 2,095、開始でない 940 → 569。出典: `q2_check0079.txt` 3 行目（`entries (non-distinct) in function_starts: 2095  not: 569`）。seeds 文書の本文には 1,724 も 940 も出てこない（grep で 0 件）。
- 「940 件は `.pdata` の範囲に入らない」は、開始でない 569 件が範囲に入らない、になる。導出: `q2_check0079.txt` の「distinct 開始でない 548」と「non-start & not covered 548」が同数なので、開始でない移動先は全部が範囲の外。エントリ単位の範囲外の件数がログに直接あるわけではない。

未確認:

- seeds 文書 §4.5 は、1,650 / 905 の再現を `q2_why0079.txt` の「最後の再計算」としている。しかし、そのログ（11 行）には 1,650 / 905 を出す行は無い（H1〜H3 の仮説ごとの数と、表の置き場所だけ）。`q2_why0079.py` の docstring は 1,650 / 905 と 1,724 / 940 を探す目的と書く。上の「この追記の作業で再実行した」結果は、このログとは別のもの。
- 1,724 / 940 と 2,095 / 569 の差の件数ごとの内訳（読み残した分のどれがどの表に入るか）は未確認。

### 同じ調査で見つかった別の差

- 基準 (A)（`fe-lift lift … --pdata`、リベース済み DLL）の `unsupported_total` は、0079 §4 の 974,582 に対し **974,527**（55 差）。`c_lines` は 0079 の 6,707,823 に対し **6,707,922**（99 差）。functions 32,648、closure_instructions 6,464,799、emitted_instructions 6,511,234、tail_calls 22,594、gotos 450,886、bad_nodes 0 は一致。**差の出どころは未確認**。出典: seeds 文書 §5.1、`logs/A-pdata.txt`（`unsupported_total` 332 行目、`c_lines` 7 行目）、原本でも同じ数（`logs/G-orig-pdata.txt`）。

### 905 を含む他の記述

- 0079 の本文: 22、51、72、106 行目（上のとおり）。1,650 は 22、72 行目。
- `docs/README.md` の索引行 104（0079 の行）にも「表の移動先 905 が `.pdata` 開始でない」とある。README の索引行 104 も同じ数字を含む（訂正は別途）。
