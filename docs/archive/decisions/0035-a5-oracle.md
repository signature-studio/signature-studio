# 0035: A5 native オラクル（関数単位 bit-exact 差分実行）の設計と判定基準

状態: 作業者が決定。実行手順と結果は `docs/investigations/a5-oracle-run.md`、不一致の一覧は `docs/investigations/a5-oracle-mismatches.md`。

実装は `lift/harness/oracle/`（新規ディレクトリのみ。既存ファイルの変更なし）。出力は `lift/out-oracle/`。lifter、runtime、`imports_table.rs` は変更していない。

## 参照した旧実装の設計情報（コードは写していない）

旧リポジトリ（signature-studio-dev-old）の `tools/flelift/fle_native.c`、`fle_ms_call.S`、`fle_natdrv.c`、`fle_diff.py` を `lift/out-oracle/old-ref/repo/`（shallow clone、読み取り用）で読んだ。`docs/investigations/2026-10-19-old-repo-survey.md` はこのリポジトリになく、読めていない。使った設計情報は次の 5 点だけ。

1. 実機械語を ImageBase に mmap し、MS x64 ABI（RCX/RDX/R8/R9、shadow space 32 バイト）で直接呼ぶ。
2. python は 0x400000 を占有するので、ドライバは別プロセスの PIE 実行ファイルにし、`setarch -R` で起動する。
3. 比較は rax、xmm0、メモリ領域の 8 バイトブロック単位。
4. 実行前の内容との差分を領域ごとに取り、native と lift で突き合わせる。
5. fault は case 単位で回収し、判定を「比較不能」に分ける。

## 設計

| 要素 | 内容 |
|---|---|
| native 側 `native_drv` | `harness/image.c`（既存）が PE をセクション表どおりに ImageBase 0x400000 へ mmap。`ms_call.S`（新規、約 100 行）が全 GPR と XMM0-15 を入力どおりに設定し、オラクル用スタックへ切り替えて `call`、戻りで全 GPR/XMM を保存する。 |
| lift 側 `lift_drv` | 同じ `image.c`、同じ `oracle_common.c`。`fle_dispatch_table` で VA から `f_XXXXXX` を引き、`R[]`/`X[]` を入力どおり設定して呼ぶ。生成 C は `lift/out-pdata/parts/after-pdata-b-part-*.o`（all-pdata の split ビルド、参照のみ）。runtime オブジェクトは `out-pdata/fle_*.o` を `out-oracle/rt/` にコピー（未存在のときだけ）。 |
| 比較スクリプト | `run_diff.py`。case ファイルを 16 分割して両ドライバを並列に走らせ、case ID で結合する。 |
| 選別 | `select_pure.py`（capstone）。下の条件。 |
| case ごとのプロセス | case ごとに `fork`。fault、abort、2 秒のタイムアウトは子の signal handler が結果ページ（共有メモリ）に書いて終了する。子が死んでも親は次の case を続ける。 |

### 入力（同じ値を両側へ）

- 全 GPR（rsp を除く）と XMM0-15 の 128 ビット。rsp は両側とも `OR_STACK_TOP`（0x300C0000）。
- スタック引数 4 個（`[rsp+32]` から）。スタック領域 1 MiB は case の seed で埋める。
- scratch（0x20000000、1 MiB）の初期内容は 5 種類（乱数、ポインタ混じり、全 0、全 -1、境界値表）。
- 12 case/関数: 全 0、全 -1、境界値表、すべて 0 の引数、ポインタ引数、浮動小数の境界値（FP64/FP32 の NaN、Inf、非正規数）、32/64 ビット境界の整数、乱数。

### 比較対象

- 16 GPR（rax、rdx を含む。rsp は戻り後の値）。
- XMM0-15 の 128 ビット全部（XMM0/XMM1 に限らず）。
- 3 領域の 8 バイトブロック差分（呼び出し前との差。個数と FNV ハッシュと先頭 64 ブロックを持ち、ハッシュで一致判定する）。
  - 書き込み可能セクションの全体（`IMAGE_SCN_MEM_WRITE` の付いた `.data`、`.bss`、`.idata`、`.didata` の VA 範囲 0x13a5000〜0x1977000。実 DLL で書き込み可能なのはこの 4 つだけ）
  - scratch
  - スタック領域
- `.text` は両側とも読み取り専用に保護（native 側は実行可）。書き込みは両側で fault する。

### 比較しないもの

- EFLAGS: native 側の `ms_call.S` は戻り時の flags を保存していない。lift の `FL` は 6 つの状態フラグで、未定義値（0xff）を持つ。関数戻り時の flags は呼び出し規約上の戻り値ではないので比較対象から外した。命令単位の flag の正しさは関数単位の結果（rax、メモリ、分岐の結果）を通してだけ間接的に検査される。
- x87、MXCSR、MMX 状態（選別で x87 を除外。MMX の関数が選別に残るかは未確認）。
- TEB/GS（選別で GS/FS 使用関数を除外。native 側は GS ベースを設定していない）。

### 判定基準（bit-exact のみ）

case 単位:

| 判定 | 条件 |
|---|---|
| `bit_exact` | native が戻り、lift も戻り、上の比較対象がすべて一致 |
| `mismatch` | 両方戻って 1 つでも違う。または native が戻り lift が fault/タイムアウト |
| `lift_unsupported` | native が戻り、lift が `fle_unsupported` で停止 |
| `lift_stop_other` | native が戻り、lift が他の runtime 停止（未定義フラグ読み出し、整列違反など）、または表に VA がない |
| `native_fault` | native が戻らない（SIGSEGV/SIGBUS/SIGILL/SIGFPE/タイムアウト）。比較不能。lift 側の挙動は内訳として記録（同じ種別か、同じ fault アドレスか） |

関数単位: 1 つでも `mismatch` があれば `has_mismatch`。`mismatch` がなく `bit_exact` があれば `bit_exact_all_comparable`。それ以外で lift が実行不能なら `lift_not_executable`。全 case が native_fault なら `native_fault_all`。

「不一致なし」は、比較できた case の範囲での結論。`native_fault` の case は何も主張しない。

### import（IAT）の扱い

lift 側 `image.c` は IAT スロットに自身のスロット VA を書き、呼ぶと `fle_dispatch_miss` に届く。native 側も同じ `image.c` を使うので同じ値になる。スロット VA は実行不能なので、IAT 経由の call/jmp は native で SIGSEGV になる。このため **import を呼ぶ関数は選別で除外**した。lift の `fle_imp_*` 応答表と同じ応答を native 側に用意する作業は今回していない（対象を「import を呼ばない純計算関数」に限った）。

### 純計算関数の選別（`select_pure.py`）

対象は all-pdata の `.pdata`（チェーンの子を除く）で、`fle_dispatch_table` に入口がある関数。次を 1 つでも含む関数は除外し、その関数を（直接 call/jmp で）呼ぶ関数も、推移的に除外する。

- `.idata`/`.didata` を参照するオペランド（IAT 経由の call/jmp、IAT スロットの読み出し）
- 間接 call/jmp（レジスタ、メモリ、ジャンプ表）
- x87 命令（capstone の FPU グループ）
- FS/GS のメモリオペランド（LEA を除く）
- `cpuid`、`rdtsc(p)`、`xgetbv`、`syscall`、`int*`、`hlt`、`ud2`、`in/out`、`rdrand`、`rdseed` など
- 直接の call/jmp 先が、lift 済みの関数入口でないもの

### ネガティブコントロール

native ドライバだけ、バイトを 1 つ XOR 0x01 した PE のコピーで走らせ、lift ドライバは元の PE の生成 C のまま。`mismatch` が 1 case でも出れば「検出」。

- 定数返却関数（`mov eax, imm32; ret`）の imm の 1 バイトを反転した 3 件。rax が変わるので、必ず検出できなければならない。
- 全 case が bit_exact だった関数の本体の乱数位置のバイトを反転した最大 57 件。反転が実行されない経路や出力に出ない命令に当たると検出されない（検出の下限を測るもので、全件検出を期待しない）。

## 限界（事実）

- 純計算関数は小さい（選別された 9,901 件のうち命令数の中央値 5、最大 1,784）。深い関数の検査にはなっていない。
- native が戻らない case が多い（ポインタ引数に乱数を渡すため）。比較できる case の割合は 1457/2400。
- 比較できても、case の入力が通す経路は一部。
- 乱数を入力にするので、`bit_exact` は「その case で一致した」であって、関数全体の等価性ではない。
