# A5 native オラクルの実行手順と結果

日付: 2026-10-20。設計と判定基準は `docs/decisions/0035-a5-oracle.md`、不一致の一覧は `docs/investigations/a5-oracle-mismatches.md`。「実測」はツールの出力、「推測」は未検証の解釈。

## 1. 実行手順（再現コマンド 1 本）

```
lift/harness/oracle/run.sh
```

環境変数: `FLENGINE_PE`（既定 `binaries/win64/FLEngine_x64.dll`）、`PARTS_NAME`（既定 `after-pdata-b`）、`COUNT`（200）、`N`（12）、`SEED`（1）、`NEGCTL`（24）、`JOBS`（16）、`OUTDIR`（既定 `run3`、`lift/out-oracle/results/` の下）。run3 は `NEGCTL=60` で実行した。

`run.sh` の内容: `build.sh`（`native_drv` と `lift_drv`）、`select_pure.py`（純計算関数の選別）、`run_diff.py`（差分実行、ネガコン、集計）。ドライバは `setarch x86_64 -R` で自分自身を再起動する（`ORACLE_NOASLR`）。TMPDIR は `lift/out-oracle/tmp`。所要時間は約 20 秒（16 並列、`time` の real）。

前提: `lift/out-pdata/parts/after-pdata-b-part-*.o` と `out-pdata/fle_*.o` が既にあること（読み取りのみ。再生成していない）。python に capstone（5.0.7）が必要。

## 2. 作ったもの（すべて新規）

- `lift/harness/oracle/`: `oracle_common.h`、`oracle_common.c`（case ファイルの読み込み、case ごとの fork、入力の敷設、3 領域の差分、JSON 出力）、`ms_call.S`（MS x64 の thunk）、`native_drv.c`、`lift_drv.c`、`build.sh`、`select_pure.py`、`run_diff.py`、`run.sh`。
- `lift/out-oracle/`: `bin/`、`obj/`、`rt/`（out-pdata の runtime オブジェクトの複写）、`results/`、`tmp/`、`old-ref/repo/`（旧リポジトリの shallow clone、103 MB、読み取り用）。

## 3. 選別（`select_pure.py`、実測）

| 項目 | 件数 |
|---|---|
| `.pdata` 開始（チェーンの子を除く） | 57,816 |
| lift 済み入口（`fle_dispatch_table`） | 58,695 |
| 候補（pdata 開始かつ lift 済み） | 57,816 |
| **純計算（除外条件に当たらない）** | **9,901** |
| 除外: 間接 call | 16,461 |
| 除外: 呼び先が除外（推移） | 29,331 |
| 除外: 呼び先が lift 済み入口でない | 2,036 |
| 除外: x87 | 78 |
| 除外: 間接 jmp | 4 |
| 除外: cpuid | 4 |
| 除外: GS/FS オペランド | 1 |

（IAT の参照を直接の理由として除外した関数は 0 件（実測）。IAT 経由の呼び出しは `jmp [rip+IAT]` のサンク（pdata 開始でない）への直接 call になっているため、「呼び先が lift 済み入口でない」2,036 件がそれに当たる。候補の関数から、その形のサンクへの直接 call/jmp は 3,192 か所で、サンク以外の「入口でない呼び先」は 0 か所（実測）。したがって import を呼ぶ関数は、この条件と、その呼び元への推移的除外で漏れなく除かれている。）

純計算 9,901 件の命令数は中央値 5、最大 1,784。

## 4. 本番の結果（run3: 無作為 200 関数、各 12 case、seed 1）

実測（`lift/out-oracle/results/run3/a5-summary.json`）:

| 項目 | 件数 |
|---|---|
| 対象関数 | **200** |
| case | **2,400** |
| bit-exact | **1,457** |
| mismatch | **0** |
| lift が unsupported で実行不能 | **0** |
| lift のその他の停止 | **0** |
| native が戻らず比較不能 | **943**（SIGSEGV 933、タイムアウト 10） |

native が戻らない 943 件のうち、lift 側も同じ結果だった: 933 件は両側とも SIGSEGV で fault アドレスが一致、10 件は両側ともタイムアウト（関数 0x4104e0）。

関数単位: bit_exact_all_comparable 173、native_fault_all 27、has_mismatch 0、lift_not_executable 0。

bit_exact の 1,457 case のうち、image または scratch に書き込みがあったもの 535 case（61 関数）。残りは rax/xmm/スタックだけが変わる case。スタックは、戻りアドレス領域より下の書き込みを含めて比較している（`wrote_stack_blocks > 0` は全 1,457 case。呼び出し時の戻りアドレススロットの書き込みを含むため）。

結果の一貫性: run2 と run3 は同じ `categories`（bit_exact 1457、native_fault 943）。run1 は case 生成を直す前（bit_exact 1440、native_fault 960）で、`run2`/`run3` より前の版。`smoke` は 10 関数の試走。どちらも比較用に残した。

## 5. ネガティブコントロール（実測）

native だけ、PE の 1 バイトを XOR 0x01 したコピーで実行。lift は元の PE の生成 C。`mismatch` が 1 case でもあれば検出。

| 種類 | 件数 | 検出 |
|---|---|---|
| 定数返却関数（`mov eax, imm32; ret`）の imm | 3 | **3** |
| 全 case が bit_exact だった関数の本体の乱数位置 | 57 | 26 |
| 合計 | 60 | **29** |

定数返却 3 件（0x410030、0x4536d0、0x453900）は全 12 case が mismatch。乱数位置の未検出 31 件の内訳: 全 case が bit_exact のまま 17、native が fault する case が残って、比較できた case は一致 14。未検出の原因（実行されない経路か、出力に出ない変化か）は調べていない（推測）。run2 の同種の対照では 12/24。

結論（実測の範囲）: オラクルは、出力に現れるバイトの変化を検出する。検出されない変化がある以上、「このオラクルで不一致が 0」は「lift が bit-exact」ではない。

## 6. lift が unsupported の関数を狙った追加実行（実測）

無作為 200 関数には本体に `fle_unsupported` を含む関数が入らなかったため、選別 9,901 件のうち本体に `fle_unsupported` を持つ 21 件を全部走らせた（`results/run-unsupported/`、252 case）。

bit_exact 7、mismatch 0、lift_unsupported 42、native_fault 203。`lift_unsupported` は Bswap、Prefetcht0、Cvtdq2ps、Addpd、Psubw（11 か所）。詳細は `a5-oracle-mismatches.md`。

## 7. 限界（事実）

- 比較は 2,400 case 中 1,457 case。入力のポインタが無効だと native が落ちる。有効なポインタを渡す case の設計は、関数の引数規約を知らないので、レジスタ 4 つに scratch 内のポインタを入れる程度。
- 純計算関数は小さい。1 関数 12 case では、分岐の全経路は通らない。
- import を呼ぶ関数、間接 call を持つ関数、x87、GS を使う関数は対象外（選別で除外）。lift の `fle_imp_*` 応答表と native 側を合わせる作業は未着手。
- EFLAGS、x87、MXCSR、MMX 状態、GS/TEB は比較していない。
- 乱数 seed は固定（seed 1）。別の seed で走らせると、native_fault の割合や検出数は変わる。
- lifter、runtime、`imports_table.rs` は変更していない。

## 8. 触ったファイル

新規: `lift/harness/oracle/*`（上記 10 ファイル）、`docs/decisions/0035-a5-oracle.md`、`docs/investigations/a5-oracle-run.md`、`docs/investigations/a5-oracle-mismatches.md`、`lift/out-oracle/` 配下の生成物。既存ファイルの変更なし。
