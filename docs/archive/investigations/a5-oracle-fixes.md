# A5 native オラクルの修正: 呼び出し枠の揃え（`0x412680`）と native_fault の修復入力（a5f）

日付: 2026-10-28。ベース: HEAD `144a703`。変更は **オラクルのハーネス（`lift/harness/oracle/`）だけ**。lift・ランタイム・生成 C は変えていない。何も決めない（判断が要る点は §10）。

凡例: **実測** = この作業でツールを走らせて得た出力。**読み** = 実測からの解釈で、直接は確かめていないもの。**未確認** = 確かめていない。

前提（前回）: `docs/investigations/a5-full-pure/a5-oracle-full-pure.md`（純計算 9,901 関数すべてを測定。mismatch_core 57 = `lift_timeout` 41 + `0x412680` の 16 ケース。比較できたケースが 0 の関数 395）。

## 1. 結論

1. **`0x412680`: 修正後 bit_exact 16/16**（実測。前回 mismatch_core 16/16。jmp_buf の先頭 8 バイト = 自分の戻りアドレス。native も `0xdead0000` を積んで入るようにした。lift 側は変えていない）。
2. **比較できたケースが 0 の関数: 395 → 10**（実測。段階: 395 →〔枠の修正〕394 →〔修復入力: 下表〕10）。10 関数は §6.4。
   | 段階 | 内容 | 比較できたケースが 0 の関数（純計算 9,901 のうち） |
   |---|---|---|
   | 前回 | – | 395（全ケース native_fault 394 + `0x412680`） |
   | S1 | 枠の修正（native が `0xdead0000` を戻りアドレスとして積む） | 394 |
   | S3 | 修復入力、入力レジスタ・スタック引数・scratch の 8 バイト語だけ | 222 |
   | S4 | + 書き込み可能なイメージ（グローバル変数）の 8 バイト語 | 46 |
   | S5 | + 倍率・添字つき・読み書き・狭い読みを辿る | 13 |
   | S6（最終） | + `add/sub reg,[mem]` | **10** |
   （S3〜S6 は 394 関数 × 16 ケース = 6,304 ケースから。`0x412760` は S3 で解決: §5。）
3. **他の関数の回帰: なし**（実測）。seed 1（16,000 ケース）と 7 バッチ（111,888 ケース）を新しい native ドライバと再リンクした lift ドライバで再実行し、判定行（関数・ケース・判定・原因）を前回の記録と比べた: 127,888 行中、違うのは **`0x412680` の 16 行（mismatch_core → bit_exact）と、タイミングで原因語だけが変わる 1 行（`6b11f0.9`、native_fault のまま）**。さらに、戻りアドレスの値を変えて native を 2 回走らせる走査（158,287 ケース対）でも、戻りアドレスに依存する関数は `0x412680` だけ。静的走査でも 9,901 関数のうち `[entry_rsp+0]` に触れるのは `0x412680` だけ。
4. **ネガコン（前回: 出力の差で 62.5%、native fault 込みで 91.5%）→ 62.7%（890/1,420）／92.1%（1,308/1,420）**。下回っていない（実測）。バッチ 04 だけ制御の選ばれ方が変わった（`0x412680` が候補に入るため。他の 6 バッチは制御の集合も結果も同一）。修復入力に対するネガコン（新規）は 36.5%／84.0%（200 件）で、通常入力より弱い（§8）。
5. **mismatch は 0**（実測）。修復入力で native が戻ったケースはすべて bit_exact: 394 関数の 4,854 ケース、7 バッチの native_fault 16,650 ケースのうち 12,380 ケース（74.4%）。重なりを除くと 13,780 ケース・1,599 関数。lift_unsupported・lift_stop_other・mismatch_aux も 0。見つかった lift の欠陥は無い。
6. 範囲: 純計算 9,901 関数のみ。修復入力は「入力に有効な値を足す」ものであり、実プログラムの状態を再現するものではない。**Intel 実機、純計算でない 47,915 関数、HEAD の runtime でのバイナリ作り直し、`lift_timeout` 41 件の延長再実行のやり直しは未確認**（§9）。

## 2. 環境・ゲート・変更の範囲

| 項目 | 内容 | 区分 |
|---|---|---|
| HEAD | `144a70381315545c4d8dc7fbfd709f7eaf1775f1`（作業ツリーは未変更。`git status` は着手時と同じ未追跡 5 件） | 実測 |
| 着手後の HEAD | 作業の終わりに HEAD は `8f63bda`（wasm 6 モジュールの commit）に進んでいた（他の作業者）。`a5f.diff` はその HEAD の `git archive` にも `patch -p1 --dry-run` で当たる（実測）。**`8f63bda` で生成 C の SHA を再確認していない（未確認）**。ゲートと測定は `144a703` の木 | 実測／未確認 |
| PE | `binaries/win64/FLEngine_x64.dll` SHA-256 `b2c64da4…9496d`（確認済み） | 実測 |
| ゲート | HEAD の生成 C（`fe-lift lift DLL OUT.c --pdata`）SHA-256 **`f6d3a9715ddb92d734577fc1e87a381142c722aa17562a22a72c565599cbb08c`**（215,773,171 B。指示の値と一致）。functions 59,380、unsupported_total 390。前回 C（0d58b0f）との関数本体の比較: 共通 59,297、本体が変わったもの 136、**純計算 9,901 のうち 0**、HEAD で増えたもの 83。`GATE OK: no pure function body changed` | 実測（`results/examples/`） |
| ホスト | AMD Ryzen 9 9950X（16 CPU）。他の作業者が同居 | 実測 |
| lift_drv2 | **前回の生成 C 由来のオブジェクト（`lift/out-a5-head/.../parts/*.o`、`rt/*.o`）を読み取りで使い、`lift_drv2.o`・`oracle2_common.o`・`image.o` だけ HEAD のソースから作り直して再リンク**（258 MB、RAM 上）。`lift_drv2.c` を HEAD の runtime ヘッダと元の `oracle2_common.h` でコンパイルした逆アセンブルは前回のオブジェクトと同一（`objdump -d` の差は先頭のファイル名の行だけ）。今回のオブジェクトは `OR_MAX_POKE` 64 の構造体配置になる（構造体のオフセットだけの違い。読み）。ドライバ（`oracle2_common.c`）の変更は、イメージの書き込み可能な節への poke を受け付けることと `OR_MAX_POKE` 24 → 64 だけ（§5.3） | 実測 |
| native_drv2 | HEAD のソースから作り直し（§3、§5） | 実測 |
| 生成 C から作り直したか | **していない**。前回のバイナリの部品（0d58b0f の runtime と生成 C）をそのまま使った。ゲートは純計算の関数本体が HEAD と同一であることまで | 実測／読み |

変更したファイル（`results/../a5f.diff`、`patch -p1` で `git archive HEAD` に当たる）:

| ファイル | 変更 |
|---|---|
| `lift/harness/oracle/ms_call2.S` | `call *host_fn` を「`pushq oracle_ret_target` + `jmp *host_fn`」に。戻り先ラベル `oracle_ms_ret` を公開 |
| `lift/harness/oracle/native_drv2.c` | 戻り用スタブ（`0xdead0000` に 1 ページ。`jmp *0(%rip)` + 8 バイトの絶対アドレス）。リング記録（`ORACLE_RING_DIR`） |
| `lift/harness/oracle/oracle2_common.c/.h` | poke の範囲にイメージの書き込み可能な節を追加（`poke_ok`）、`OR_MAX_POKE` 64 |
| `lift/harness/oracle/a5f_repair.py`（新規） | native の fault から入力を直す修復（後ろ向きスライス） |
| `lift/harness/oracle/a5f_run.py`（新規） | 修復 → 通常と同じ判定（`run_diff2.classify`）。ネガコンも |
| `lift/harness/oracle/a5f_retaddr_scan.py`（新規） | 戻りアドレスに依存する関数の静的・動的走査 |
| `docs/investigations/a5-oracle-fixes.md`（新規） | 本書 |

`run_diff2.py`、`argptr.py`、`select_pure.py`、`lift_drv2.c`、`build*.sh`、lift・runtime は **変更なし**（実測: diff に出ない）。

## 3. `0x412680` の枠の揃え

### 3.1 原因の再確認（実測）

- `0x412680`（`setjmp` の本体、25 命令）の先頭は `mov rax,[rsp]; lea rdx,[rsp+8]; mov [rcx],rax; mov [rcx+8],rdx; …`。自分の戻りアドレスを jmp_buf の先頭に保存する。
- 前回の枠: native は `oracle_ms_call` の `call *host_fn` が積む本物の戻りアドレス（ドライバ内。PIE の基底つきで `0x555555556cff`）、lift ドライバは番兵 `OR_SENTINEL = 0xdead0000` を `R[4]=OR_STACK_TOP-8` に書く。両側が「自分の枠が与えた戻りアドレス」を忠実に保存しただけで、lift の値の誤りではない（読み。16 ケースとも違いはこの 8 バイトだけ）。

### 3.2 修正（上の方針 (a)）

- native: `call` をやめ、戻りアドレスとして `oracle_ret_target`（= `OR_SENTINEL`）を積んで `jmp`。`0xdead0000` に 1 ページ（PROT_READ|EXEC）を置き、そこに `jmp *0(%rip)` と戻り先ラベル `oracle_ms_ret` の絶対アドレスを書く。callee の `ret` は `0xdead0000` に飛び、スタブが `oracle_ms_ret` に戻る。レジスタ・フラグはスタブで壊れない。lift 側は今のまま。
- **途中で見つけた誤り（直した）**: 最初の版は戻りアドレスの push を `pushq 384(%rax); popfq`（入力 EFLAGS を設定）より前に置いたため、フラグ用の push が `OR_STACK_TOP-16` に残り、native だけスタックの差分が 1 つ増えて 16 ケースが `mem:stack` で不一致になった。push を `popfq` の後に移して解消（その後のすべての実行は修正後の版）。
- 比較の弱め方はしていない（マスクなし）。ネイティブ側のスタックは、callee の入口で `rsp = 0x300bfff8`（§4）。

### 3.3 結果（実測）

- `repro-412680-cases.txt` の 16 ケース: **bit_exact 16/16**（native の jmp_buf の先頭は `0xdead0000`、`[rcx+8]` は `0x300c0000` = `rsp+8`。`results/examples/412680-*`）。
- バッチ 04（`0x412680` を含む）の再実行: 判定行の違いはちょうどこの 16 行（mismatch_core → bit_exact）。

### 3.4 同じ型の関数が他にあるか（実測）

| 方法 | 結果 |
|---|---|
| 静的（`a5f_retaddr_scan.py`）: 純計算 9,901 関数の線形走査で rsp（と `mov rbp,rsp` による rbp）を入口の rsp からの差で追い、`[entry_rsp+0]`（戻りアドレスの置き場）を指すメモリ参照を探す | **1 関数（`0x412680`）**。`0x412760`（`lea rcx,[rcx+0x38]; jmp 0x412680`）は自身の 3 命令には無いが、飛び先が `0x412680` |
| 動的: 全 9,901 関数 × 16 ケース（seed 1、158,416 ケース）を native で 2 回走らせる。戻りアドレスのスタブを `0xdead0000` と `0xbeef0000`（`ORACLE_RETADDR=beef0000`）にして、出力 JSON 全体を比べる（片方でもタイムアウトの 129 ケースは除く） | 比較できた 158,287 ケースのうち、違いが出るのは **`0x412680` の 16 ケース（`mem`）だけ** |
| 限界（読み） | 動的は実行されたケースだけ（native が fault する約 15% は比べていない）。静的は直接の `[rsp]`／`[rbp+…]` だけ（間接の経路、`and rsp,-16` 後の相対は追わない）。`rsp` の絶対値に依存する関数は両側で `R[4]` が同じなので問題にならない |

## 4. `0x412760`（入口）の `movapd` fault

**原因は呼び出し時のスタックの境界ではなく、入力の `rcx` の境界**（実測）。

- スタック: `OR_STACK_TOP = 0x300C0000`（16 の倍数）。native も lift も callee の入口で `rsp = OR_STACK_TOP-8 = 0x300bfff8`、つまり rsp ≡ 8 (mod 16)。MS x64 の規約（呼び出し直前に rsp が 16 の倍数、入口で ≡ 8）に合っている。`0x412680` の jmp_buf に保存された `rsp+8` が `0x300c0000` であることでも確認（`examples/412680-native-out.jsonl`）。
- `0x412760` は `lea rcx,[rcx+0x38]; jmp 0x412680`。`0x412680` の `movapd [rax-0x20],xmm6`（`rax = rcx+0x70`）は `rcx+0x38+0x50 = rcx+0x88` を 16 バイト境界に要求する。つまり**この関数は `rcx ≡ 8 (mod 16)` の入力を前提にしている**（読み: FPC の `TJmpBuf` は 0x38 バイトの前置きの後に Windows の `_JUMP_BUFFER` が来る）。入力生成は ポインタ引数を常に 16 の倍数にするので、16 ケースとも native が `#GP`（`gp_misaligned_sse_operand`）で fault していた。
- lift も同じ入力で `fle: misaligned 16-byte access` で abort する（lift の生成 C が `movapd` の境界を検査して同じ入力を拒否している。実測）。
- 確認: 同じ 16 ケースの `rcx` に 8 を足して両側を走らせると **16/16 bit_exact**（実測）。
- 修正: 通常のケース生成は変えない（既存の結果を変えない）。修復入力（§5）が「misaligned の fault → ポインタの出所の入力を、境界に合う最小の量だけずらす」を行い、`rcx` を `+8` した新しいケース（id `412760.<k>r`）を足す。16/16 bit_exact（`examples/412760-repaired-cases.txt`、`412760-repair-steps.json`）。

## 5. native_fault の分類と修復入力

### 5.1 分類（実測）

全ケースが native_fault の 394 関数（`native-fault-all.txt`）の 16 ケース = 6,304 ケースを、新しい native ドライバで 1 回走らせた最初の fault の原因（`run_diff2.fault_cause`）:

| 原因 | ケース数 |
|---|---|
| `null_deref`（アドレス < 0x10000） | 3,803 |
| `wild_pointer` | 1,295 |
| `gp_noncanonical_address` | 1,125 |
| `gp_other`（`movsq` ほか） | 20 |
| `timeout_2s(loop)` | 18 |
| `gp_misaligned_sse_operand` | 16（`0x412760` の 16 ケース） |
| `sigfpe(divide)` | 13 |
| `scratch_range_exceeded` | 12 |
| `stack_range_exceeded` | 2 |

7 バッチ全体（native_fault 16,650 ケース）: `null_deref` 8,365、`wild_pointer` 4,242、`gp_noncanonical_address` 2,126、`scratch_range_exceeded` 1,456、`timeout_2s` 257、`sigfpe` 113、`gp_other` 50、`stack_range_exceeded` 24、`gp_misaligned_sse_operand` 17。

壊れたアドレスのレジスタの出所（前回の `origin_of`、394 関数の fault 7,046 件）: 呼び先の戻り値または引数でない入力レジスタ（`no_def_in_function`）約 1,900、関数内の別のメモリ参照からの読み（`loaded_from_memory`）約 1,570、「引数と同じ値」約 2,000（整数引数かスタック引数をポインタとして使う）、計算した値約 600、イメージのグローバルからの読み 157、与えた構造体の中身 334（読み: 分類は値の一致による近似）。

### 5.2 方式（`a5f_repair.py`、`a5f_run.py`）

1. 関数ごとに、元と同じ 16 ケースを `run_diff2.gen_cases(fn, 16, seed)`（seed はその関数が全ケース fault だった実行の seed）で作り直す。**既存のケース集合・seed・判定は変えない**。足すのは新しいケース（id に `r` を付ける）だけ。
2. native を **1 命令ずつ**（`ORACLE_TRACE`）走らせ、`native_drv2` が各命令の直前の状態（rip と 16 個の GPR）を直近 16,384 件までリングに持ち、fault・abort・タイムアウトで `ORACLE_RING_DIR/<case id>.ring` に書く（`ptrace` は使わない。SIGTRAP と、既存の SIGSEGV ハンドラへの連鎖）。
3. fault した命令のアドレス計算（ベース／インデックス）から壊れたレジスタを決め、その値が**どの入力から来たか**をリングの上で後ろ向きにたどる（`a5f_repair.Trace.follow`）: `mov`／`lea`／`add`・`sub`（定数）／`shl`／`imul`／`neg`／`inc`・`dec`／`movzx`／`movsxd`／`add|sub reg,[mem]`／`pop`、関数自身の書き込み（`mov [m],reg`、`add|sub|inc|dec [m],imm`）、メモリ読み。値は「入力 × S + K」の形で持つ。たどれたのは (a) 入力レジスタ、(b) スタック引数の 8 バイト語、(c) scratch または**書き込み可能なイメージの節**の 8 バイト語。
4. その入力だけを、fault を起こさない有効な値に置き換えた新しいケースを作り（有効な値 = scratch の中ほど `[0x20020000, 0x20060000)` の 16 の倍数。misaligned のときは境界に合う最小のずらし。除数・添字・カウントは 0〜16 の小さい値）、手順 2 から繰り返す（最大 40 回）。同じ fault と同じ修復が出たら打ち切る。
5. native が戻った新しいケースを、通常と同じ段階にかける: native（通常）、lift（**同じケースファイル**）、native 単一ステップ（網羅率と、通常の native との一致の確認）、タイムアウトの 10 秒再実行、`run_diff2.classify`。**結果は無視しない**（一致扱いにするものは無い）。修復の段階では lift を走らせない。決定的（`random.Random` を case id と回から作る）。

「ふりをしない」ことの根拠（実測）: 修復入力のケースは lift と native が同じ入力で両方戻り、全レジスタ・XMM・メモリ・フラグ・MXCSR・x87 を比べる（`run_diff2.compare`）。native の単一ステップの結果と通常の結果は、394 関数の 4,854 ケースと 7 バッチの 12,380 ケースのどちらの実行でもすべて一致（`trace_consistency: equal`）。

### 5.3 ハーネスの変更（ドライバ）

- イメージの書き込み可能な節に poke できるようにした（`oracle2_common.c: poke_ok`）。ケース行の形式は変えていない（poke の宛先が scratch の範囲外のとき前回までは黙って無視していた。今は書き込み可能な節の中なら書く）。**これを使う既存ケースは無い**（`gen_cases` の poke は scratch の中だけ。実測: 再実行で全判定行が一致）。
- poke の最大数 24 → 64。

### 5.4 段階ごとの結果（394 関数 × 16 ケース = 6,304 ケース）

`A5F_FEATURES` で修復の部品を切って、同じ最終コードで段階を再現した（実測。各段階を最初から再実行）。

| 段階 | 部品 | native が戻った新ケース | 比較できたケースが 1 つ以上ある関数 | 0 の関数 | 判定（新ケース） | 所要 |
|---|---|---|---|---|---|---|
| S3 | 入力レジスタ、スタック引数、scratch の 8 バイト語、`mov`／`lea`／`add`・`sub` 定数／`inc`・`dec`／`pop` | 1,805 | 172 | 222 | bit_exact 1,805、他 0 | 7 分 08 秒 |
| S4 | + 書き込み可能なイメージの 8 バイト語（`image`） | 4,015 | 348 | 46 | bit_exact 4,015、他 0 | 7 分 52 秒 |
| S5 | + 倍率・添字つき `lea`、`add reg,reg`、`shl`、`imul`、`neg`、`movzx`／`movsxd`、`[mem]` の読み書き、複数オペランド、`div [mem]`（`arith`） | 4,807 | 381 | 13 | bit_exact 4,807、他 0 | 10 分 18 秒 |
| S6（最終） | + `add|sub reg,[mem]`（`addmem`） | **4,854** | **384** | **10** | **bit_exact 4,854、mismatch 0、lift_unsupported 0、lift_stop_other 0** | 12 分 56 秒 |

- S1（枠の修正）の効果は `0x412680` の 16 ケースが mismatch_core → bit_exact（比較できたケースが 0 の関数 395 → 394）。`0x412760` の 16 ケースは S3 から bit_exact。
- 新ケースの native 網羅率（実行された自分の命令の割合）: 1 ケースあたり平均 0.841（中央値 1.0）、関数ごとの和集合の平均 0.889（384 関数のうち 249 が全命令）。通常のケースは平均 0.916。新ケースのうち image／scratch に書くものは 45.1%（通常は約 38%）。
- 修復の回数: 1 回 2,715、2 回 1,161、3 回 305、…、40 回 8（最大）。成功した 4,854 ケースが使った修復の種類: scratch の poke を含むもの 3,328、イメージの poke を含むもの 2,361、入力レジスタを直したもの 238、スタック引数を直したもの 66（1 ケースが複数の種類を使うので和は 4,854 を超える）。

### 5.5 7 バッチ（通常の実行）の native_fault 16,650 ケースへの適用

`a5f_run.py --faulting-from bNN/a5-cases.json --seed <バッチの seed>`。各バッチの native_fault のケースだけを修復し、通常と同じ判定にかけた（実測）。

| バッチ | native_fault ケース | 修復して native が戻ったケース | 打ち切り（40 回） | 判定 | 関数（native_fault を持つ） | 全ケース native_fault だった関数のうち修復ケースで比較できた |
|---|---|---|---|---|---|---|
| 01 | 2,607 | 1,913 | 235 | bit_exact 1,913 | 284 | 42 / 44 |
| 02 | 2,656 | 1,966 | 172 | bit_exact 1,966 | 284 | 37 / 37 |
| 03 | 2,282 | 1,649 | 177 | bit_exact 1,649 | 252 | 38 / 41 |
| 04 | 2,318 | 1,696 | 195 | bit_exact 1,696 | 278 | 34 / 35 |
| 05 | 2,365 | 1,801 | 152 | bit_exact 1,801 | 258 | 48 / 49 |
| 06 | 2,184 | 1,677 | 127 | bit_exact 1,677 | 256 | 34 / 34 |
| 07 | 2,238 | 1,678 | 160 | bit_exact 1,678 | 262 | 35 / 36 |
| **計** | **16,650** | **12,380（74.4%）** | 1,218 | **bit_exact 12,380、他 0** | | 268 / 276 |

native_fault の原因別の改善（7 バッチ、最初の fault の原因 → 修復して native が戻った割合）:

| 原因 | ケース | 修復して戻った | 割合 |
|---|---|---|---|
| `null_deref` | 8,365 | 7,204 | 86.1% |
| `wild_pointer` | 4,242 | 3,263 | 76.9% |
| `gp_noncanonical_address` | 2,126 | 1,755 | 82.5% |
| `scratch_range_exceeded` | 1,456 | 29 | **2.0%** |
| `timeout_2s(loop)` | 257 | 0 | 0% |
| `sigfpe(divide)` | 113 | 62 | 54.9% |
| `gp_other` | 50 | 45 | 90.0% |
| `stack_range_exceeded` | 24 | 6 | 25.0% |
| `gp_misaligned_sse_operand` | 17 | 16 | 94.1% |

394 関数の 6,304 ケースでは `null_deref` 75.6%、`wild_pointer` 73.4%、`gp_noncanonical_address` 87.1%、`gp_misaligned_sse_operand` 100%、`scratch_range_exceeded` 66.7%（12 ケース）、`sigfpe` 23.1%（13 ケース）。

7 バッチの native_fault 16,650 ケース（base 111,888 の 14.88%）のうち、修復ケースが比較できなかったのは 4,270 ケース（3.8%）。内訳（修復できなかった理由。実測）: 40 回では終わらない 1,218（連結リスト・配列・木をたどるループで、毎回新しい要素が壊れている）、リングに入りきらない長いループ（`ring_wrapped`）1,072、計算の途中（`add reg,reg` ほか）450、タイムアウト 438、`lea [rip+…]` 154、倍率が割り切れない 133、`imul` 118、`sub` 113、`shr` 101 ほか（`results/agg.txt`）。**`scratch_range_exceeded`（loop が scratch の終端を越える）は事実上直せていない**（読み: 繰り返し回数の引数を絞る修復が要る）。

### 5.6 mismatch の詳細

**無い**（修復ケース 13,780（重なりを除く）すべて bit_exact）。lift の欠陥の疑いも、オラクルの限界によるものも見つかっていない。

## 6. 比較できたケースが 0 の関数（最終: 10）

### 6.1 数え方

純計算 9,901 のうち、比較できて一致したケースを 1 つ以上持つ関数: 前回 9,506 → **9,891**（9,506 + 修復入力で比較できた 384 + `0x412680`）。0 の関数: 395 → **10**（実測）。ただし 384 関数の比較できたケースは**新しい入力（修復入力）**によるもので、元の 16 ケースは今も全部 native_fault。関数ごとの比較できたケース数（394 関数、最終）: 0 個 10、1 個 5、2 個 5、3 個 9、4〜15 個 140、16 個 225。

### 6.2 7 バッチ以外の 118 関数

前回の seed 1・2・3 で全ケース native_fault だった 118 関数も、394 関数の修復に含めた（seed は各関数の実行の seed。`results/seedmap.json`）。

### 6.3 修復入力が通った関数の性質（読み）

通った関数の 7 割以上はポインタを辿るだけの構造体アクセスで、修復は「構造体のフィールドに有効なポインタ」を置く操作に帰着する。通っても、その入力が実プログラムで起こる状態かどうかは検査していない（§8）。

### 6.4 残る 10 関数（実測の理由と読み）

| 関数 | 命令数 | 修復できなかった理由（実測は最終 fault の位置と理由、読みは解釈） |
|---|---|---|
| `0x412510` | 22 | 40 回で終わらない（リスト `[r10]`／`[rdx+8]` をたどるループ。毎回別の要素が null）。読み: 終端が null のリストが前提 |
| `0x54f810` | 1,784 | `lea [rip+…]` の結果をベースに使う（3 件はタイムアウト）。読み: 関数内に静的テーブル |
| `0x6b94b0` | 145 | `imul` の結果とストア経由の値。13 ケースは `imul` |
| `0xa21170` | 102 | リスト（`[rsi+0x18]`）をたどるループで 40 回で終わらない 14 ケース、他 2 ケース |
| `0x10273c0` | 7 | `0x10275f0` を呼ぶ。修復後にタイムアウト（16 ケース） |
| `0x10275f0` | 55 | 二重ループで、ループの上限がグローバルから来る。修復後にタイムアウト（16 ケース） |
| `0x10424c0` | 73 | 共有の末尾が `xor eax,eax; mov rax,[rax+0x7b4]`（定数 0 を基底にした読み＝意図した fault と読む）。入力では直せない |
| `0x10feed0` | 64 | イメージのグローバルの配列（`[rsi+rax*8]`）をたどるループ。要素ごとに別のグローバルが null |
| `0x10ff2b0` | 57 | 同上（13 ケースは `add` の計算） |
| `0x128f720` | 26 | 同上（イメージの 8 バイト語を次々に置く必要） |

## 7. 回帰: 他の関数が変わっていないことの証拠（実測）

新しい `native_drv2`（枠の修正、リング記録、poke の拡張）と再リンクした `lift_drv2` で、`run_diff2.py`（無変更）を前回と同じ引数（ハーネス・seed・`--negctl 200`・`--jobs`）で再実行し、`a5-cases.csv`（関数・ケース・入力の種類・判定・理由・fault の原因）を行ごとに前回の記録と比べた。

| 実行 | ケース | 違う行 | 内訳 |
|---|---|---|---|
| seed 1（旧 `pure.json`、1,000 関数） | 16,000 | **0** | `a5-cases.json`、`.csv`、`a5-functions.json`、`a5-mismatches.json`、`a5-negctl.json`、`run.txt` は前回とバイト一致。`a5-summary.json` と `a5-native-faults.json` だけが違う: 前者は 2 秒のアラームで再実行されたケース数が 17（前回 18）、後者は 13 件の `rip`（タイムアウトの位置。前回も毎回違うと記録されたもの） |
| バッチ 01 | 16,000 | 0 | |
| バッチ 02 | 16,000 | 0 | |
| バッチ 03 | 16,000 | 1 | `6b11f0.9`: native_fault のまま、原因が `timeout_2s(loop)` → `scratch_range_exceeded`（2 秒のアラームに間に合うかのタイミング） |
| バッチ 04 | 16,000 | **16** | `0x412680` の 16 行（mismatch_core → bit_exact）。他は 0 |
| バッチ 05 | 16,000 | 0 | |
| バッチ 06 | 16,000 | 0 | |
| バッチ 07 | 15,888 | 0 | |
| 計 | 127,888 | 17（`0x412680` の 16 + タイミング 1） | |

- `lift_timeout`（mismatch_core の `fields == ['lift_timeout']`）の件数はバッチごとに前回と同じ（10、2、4、8、9、6、2 = 41）。**延長した再実行（約 78 分）はやり直していない**（前回 41/41 bit_exact。lift 側のコードは同じ）。
- bit_exact の件数: seed 1 は 13,535（前回と同じ）。バッチは 13,383／13,342／13,714／13,674（前回 13,658 + `0x412680` の 16）／13,626／13,810／13,648（前回と同じ。バッチ 04 のみ +16）。
- 戻りアドレス依存の走査（§3.4）と合わせ、**他の関数の結果は変わっていない**。差は `0x412680` の枠と、タイミングの揺れの 1 行だけ。
- 修復は `run_diff2.py` とは別のスクリプトなので、通常の実行の出力（上の比較）は修復の有無に関係しない。

## 8. ネガコン

### 8.1 通常の実行（前回と同じ手順・同じ選び方）

7 バッチ合計（`--negctl 200` + 定数返却 3 件、1,420 件）。

| | 前回 | 今回 |
|---|---|---|
| 出力の差で検出（core + aux） | 888（62.5%） | **890（62.7%）** |
| native fault を含めて検出 | 1,299（91.5%） | **1,308（92.1%）** |
| detected_core／aux_only／native_fault_only | 817／71／411 | 818／72／418 |
| 未検出（masked／equivalent） | 121 | 112（47／65） |
| 定数返却の変異 | 20/20 | 20/20（`const_returner_imm` は全部 detected_core） |

バッチ 04 だけ制御の集合が違う（`0x412680` に bit_exact のケースができ、制御の候補の関数に入るため、シャッフルの結果が変わる）: 131／193／203（前回 129／184／203）。他の 6 バッチは制御の集合・結果とも前回と同一（バッチ 01〜03、05〜07: 131/188、115/185、123/180（202 件）、130/183、138/191、122/188）。seed 1 は 123／187／203（前回と同じ）。

### 8.2 修復入力に対するネガコン（新規、同じ方式）

394 関数の修復ケース（4,854）に対して 200 件: 出力の差で検出 73（36.5%）、native fault 込み 168（84.0%）、`undetected_state_changed_but_masked` 18、`undetected_equivalent_no_state_change` 14、`detected_by_native_fault_only` 95。**通常の入力より出力の差での検出が弱い**（62.7% → 36.5%）。読み: 修復入力は「fault しない最小の入力」で、書き換えた命令が native を fault させる件（95 件）が検出を占める。bit_exact の件数は、この弱さを考えて読む必要がある（確認はしていない）。

## 9. 限界・未確認

- 修復入力は「有効なポインタを置いた入力」であって、実プログラムの状態ではない。イメージのグローバルへの poke は、その関数が見るグローバル（FPC のランタイム変数など）を両側で同じ値にするだけで、そのグローバルが実行時に取る値を表さない。
- 修復入力の出力の差でのネガコン検出は 36.5%（§8.2）。通常の入力の約半分の検出力。
- 修復できていない native_fault が 7 バッチで 4,270 ケース（3.8%）。`scratch_range_exceeded`（1,456）はほぼ全部残る。ループの回数引数を絞る修復は未実装。
- 比較できたケースが 0 の関数が 10 残る（§6.4）。
- 純計算 9,901 関数に限る。純計算でない 47,915 関数、IAT・間接 call・x87・GS を持つ関数は対象外（前回と同じ）。
- 前回の `lift_timeout` 41 件の延長再実行は、今回やり直していない（lift 側のオブジェクトは同一）。
- lift_drv2 は 0d58b0f の生成 C と runtime のオブジェクトを使った再リンク。**HEAD の runtime と生成 C で全部作り直した測定ではない**（前回の §3 と同じ読み。ゲートは純計算の本体が同一であることまで）。
- AMD Ryzen 9 9950X の 1 台のみ。Intel 実機は未確認。
- 戻りアドレスの走査（§3.4）は seed 1 の 16 ケースと静的走査まで。native が fault する関数（約 15%）の動的な依存は比べていない。
- 修復の回数上限 40、リングは直近 16,384 命令。リングを 65,536 にしても 7 バッチの 1 バッチ（バッチ 01）で戻るケース数は変わらなかった（1,913、実測）。
- 新ケースの id は `<元の id>r`。通常の実行の集計（`a5-cases.json` など）には入れていない。7 バッチの集計や `docs/investigations/a5-full-pure/` の表は更新していない。

## 10. PM の判断が要る点（決めない）

1. **修復入力のケースを「比較できたケース」に数えるか**（R2）。数えれば 0 の関数は 10、数えなければ 394（`0x412680` は枠の修正で解決）。
2. **残り 10 関数の扱い**（§6.4）。ループの上限・グローバルの配列・意図した fault があり、入力生成の追加（回数引数を絞る、配列を終端つきで置く）が要る。やるか、説明を付けて除くか。
3. **ドライバの採用**: 新しい `native_drv2`（枠の修正）と `lift_drv2` の再リンクを標準にするか。前回の表（`0046` の A5、`docs/progress.md`、`a5-full-pure`）の `0x412680`／395 の記述を更新するか。
4. **修復入力の位置づけ**: 通常の `run_diff2.py` の出力に統合するか、別の補助測定として残すか。修復ケースの出力の差での検出力が弱い（§8.2）ことをどう読むか。
5. **`scratch_range_exceeded`**（native_fault の 8.7%）を直すか。
6. **HEAD の runtime でバイナリを全部作り直して再測定するか**（前回から未解決）、`lift_timeout` 41 件の延長再実行をやり直すか。

## 11. 規則との食い違い（自己申告）

1. `sleep`、`timeout` コマンド、`nohup`、末尾の `&` は使っていない。語 `timeout` は環境変数名・status 値・シェルツールの `timeout` パラメータ・出力の文字列としてだけ。ループ用に `echo "######## …"` の区切りを数回使った（ハーネスの注意「区切り用の echo を連ねない」に反する。結果の読みやすさを損ねた）。
2. 存在しない `bash` ツールを 1 回呼んだ。`write` ツールが 1 回、引数が空で失敗した（ファイルは作られていない）。
3. `rm` は `/tmp/opencode/a5f/` の下だけ: `oracle/gen/head-pdata.c`（ゲートで作った 215 MB。SHA を記録して削除）、各実行の `work/`、リング・回ごとの中間ファイル（`a5f_repair.py` が回ごとに消す）、試験用の `r5`、`r6`、`r7`、`rsmoke`、`dbg`、`t_rep_work`、`r394`（旧版）、`bin/lift_drv2`（最初の PREV へのシンボリックリンクと、再リンク前の版）。リポジトリ、`lift/out-*` の既存物、他の作業者のディレクトリは読み取りのみ。
4. `git` は `rev-parse`、`archive`、`status` だけ。commit なし。作業ツリーは未変更（`git status` は着手時と同じ未追跡 5 件）。
5. python は `PYTHONDONTWRITEBYTECODE=1` を付けた（`env.sh` で export）。着手直後の `python3 -c "import capstone…"` の 1 回は `env.sh` を読む前（`.pyc` の確認は最後に実施: 新しい `.pyc` は 0 件）。
6. 書き込み: `/` へは `lift/out-git/a5f-results/` だけ（約 2〜3 MB）。tmpfs: `du` で確認した最大は `/tmp/opencode/a5f/` が約 0.55 GB（`lift_drv2` 258 MB と実行の `work/` を含む）、`/tmp/opencode` 全体が約 1.97 GB（他の作業者の `wasm6` が約 1.1 GB を使っていた時）で、2.3 GB を超えていない（各段階の前後で `du -sm /tmp/opencode` を確認）。
7. 同時に重い作業を走らせないようにし、負荷は他の作業者の分が同居（バッチの再実行の所要時間は前回と同程度）。

## 12. 再現

```
export R=/home/ubuntu/work/signature-studio-develop W=/tmp/opencode/a5f
. $R/lift/out-git/a5f-results/results/tools/env.sh        # W, SRC, O, PE, PREV, PLAN, PAST
# 木: git archive HEAD の lift/harness lift/runtime lift/crates … を $SRC に展開し、a5f.diff を patch -p1 で当てる
# native_drv2: 4 つのオブジェクト（oracle2_common、native_drv2、ms_call2、image）を clang でビルドしてリンク
# lift_drv2  : 同じ oracle2_common.o、lift_drv2.o、image.o + $PREV/parts/*.o $PREV/rt/*.o -Wl,--wrap=fprintf -Wl,--wrap=fputs -lm
# 通常の再実行（例: バッチ 04）:  results/tools/runb.sh 04       # run_diff2.py（無変更）+ 前回との比較
# 修復入力（バッチの native_fault）: results/tools/runrep.sh 04
# 394 関数: results/tools/runstage.sh final image,arith,addmem 40   （A5F_FEATURES で段階）
# 戻りアドレス走査: python3 $H/a5f_retaddr_scan.py $PE pure.json argptr.json OUT --seed 1
```

## 13. 成果物

`lift/out-git/a5f-results/`（`/` の約 2〜3 MB。`.gitignore` の `lift/out*/` で追跡されない）:

| 場所 | 内容 |
|---|---|
| `a5f.diff` | `git archive HEAD` に `patch -p1` で当たる（`lift/harness/oracle/` と `docs/investigations/a5-oracle-fixes.md`） |
| `a5-oracle-fixes.md` | 本書 |
| `results/regression/` | seed 1 と 7 バッチの再実行: `a5-summary.json`、`a5-cases.csv.gz`、`a5-negctl.json.gz`、`a5-mismatches.json`、`run.txt`、`time.txt`、`regression-summary.json`（前回との行の比較） |
| `results/retaddr/` | 静的・動的走査の結果 |
| `results/repair-394/{s3,s4,s5,final}/` | 394 関数の段階ごとの `a5f-summary.json`、`a5f-functions.json.gz`。final は `a5f-cases.json.gz`、`a5f-repair-report.json.gz`（ケースごとの修復の記録）、`a5f-negctl.json.gz` |
| `results/repair-batches/bNN/` | 7 バッチの native_fault への修復の `a5f-summary.json`、`a5f-functions.json.gz`、`a5f-cases.json.gz` |
| `results/examples/` | `0x412680` の 16 ケースと出力、`0x412760` の修復ケースと修復の記録、ゲートの記録（HEAD の C の SHA、`fe-lift` の標準出力、本体の比較） |
| `results/seedmap.json`、`agg.txt`、`regress.txt` | 394 関数ごとの seed、原因別の集計、回帰の集計 |
| `results/tools/` | 実行に使ったシェルと環境 |

大きい中間物（各実行の `work/`、リングの記録）は tmpfs `/tmp/opencode/a5f/` にあり、再起動で消える。
