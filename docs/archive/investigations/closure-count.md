# 閉包の計測（fe-decode closure）

対象: `binaries/win64/FLEngine_x64.dll`（22,227,016 B、SHA-256 `b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d`）

計測コマンド: `lift/target/debug/fe-decode closure binaries/win64/FLEngine_x64.dll --mnemonics`（実行 0.8 秒）

本ドキュメントは実測値のみを記す。推測は「推測」と明記する。

## 定義（計測の前提）

- .text: VA `0x401000`、サイズ `0xfa34b0`。
- seed: エントリポイント（VA `0x1376820`）と export のうち .text 内の VA を持つもの。
- export `dbkFCallWrapperAddr`（VA `0x15ec2b0`）は .bss のデータ変数なので seed から除外（skipped として記録）。
- 再帰降下: 直接 call/jmp/jcc の宛先と fall-through を辿る。間接 call/jmp は辿らない。
- 1 命令は最初に到達した関数に帰属させる（重複なし）。

## 結果（実測）

| 項目 | 値 |
|---|---|
| seed（コード） | 3（entry, `__dbk_fcall_wrapper`, `CreateFruityInstance`） |
| seed（スキップ） | 1（`dbkFCallWrapperAddr`、データ） |
| 関数（到達） | 6,956 |
| 基本ブロック（到達） | 67,310 |
| 命令（到達、一意 VA） | 459,902 |
| デコード失敗 | 0 |
| .text 外への fall-through/分岐 | 0 |
| .text 外への direct call | 0 |
| direct call | 38,913 |
| direct jmp | 10,481 |
| 条件分岐 jcc | 32,262 |
| ret | 6,549 |
| IAT 経由 jmp（`jmp [rip]` が IAT を指す） | 402 |
| IAT 経由 call（`call [rip]` が IAT を指す） | 0 |
| 他の間接 call（`call reg/[mem]`） | 6,482 |
| 他の間接 jmp | 16 |
| far transfer | 0 |
| interrupt/exception | 0 |
| ニーモニック種類 | 165 |

上位 10 ニーモニック: Mov 189,239 / Call 45,395 / Lea 30,312 / Cmp 18,524 / Sub 15,003 / Test 13,913 / Je 13,320 / Push 12,907 / Pop 12,904 / Add 12,083。

全一覧は `closure-run1.txt`（`lift/` 直下、4,386 B）にある。

## 照合（線形走査との比較）

`objdump -d` で .text 全体（線形）を逆アセンブルし、`tools/count_rip_indirect.py` で `call/jmp [rip+disp]` の宛先を数えた（独立実装）。

| 種類 | 宛先 | 件数 |
|---|---|---|
| jmp `[rip]` | IAT（.idata, VA `0x196f000`〜`0x1975710`） | 825 |
| jmp `[rip]` | その他 | 139 |
| call `[rip]` | その他（.data 等の関数ポインタ） | 754 |
| call `[rip]` | IAT | 0 |

- 閉包の IAT 経由 jmp 402 は、線形走査の 825 以下である（件数の大小関係のみ確認。個々の包含関係は未検証）。
- 閉包の IAT 経由 call 0 と、線形走査の call→IAT 0 は一致する。

注意: objdump の出力は .text 全体を線形に走査したもので、データ領域も含むため `(bad)` が 130,082 行ある。これは到達可能なコードの失敗（0 件）とは別物。

## 未確定事項（推測を含む）

- **閉包は下界（推測を含む）**。間接 call 6,482 件、間接 jmp 16 件は辿っていない。これらの宛先が閉包に新たな関数を加える可能性がある。件数は計測済みだが、宛先の解決はしていない。
- 「CreateFruityInstance から到達できる関数」という定義は、このツールの seed 定義に依存する。他の export の追加や、ランタイムで登録される関数ポインタ（`dbkFCallWrapperAddr` 等）は含まれない。
- 命令数 459,902 は「一意 VA の数」であり、動的な実行頻度ではない。
- 関数数はこの traversal の帰属による。関数境界の正確さ（tail call の扱い等）は未検証。

## 生成物（計測の副産物）

- `lift/objdump-text.txt`: 243 MB。線形逆アセンブル結果。計測の照合に使った中間ファイルで、削除は禁止のため残置。Git 管理に入れないこと。
- `lift/closure-run1.txt`: 4,386 B。閉包の出力。
- `tools/count_rip_indirect.py`: 照合用の小スクリプト。

## 再計測: 間接 call/jmp の宛先解決（単位 U-E）

計測コマンド: `lift/target/debug/fe-decode closure binaries/win64/FLEngine_x64.dll --mnemonics`（結果は `lift/closure-run2-indirect.txt`、新規）。

解決の順序（`lift/crates/fe-decode/src/indirect.rs`）:

1. `[rip+disp]` が IAT スロットを指す: 名前で解決（閉包には入れない）。
2. `[rip+disp]` の指す 8 バイトが `.text` 内の VA: その VA を宛先（`rip_table`）。
3. 直前 1 命令の `lea reg,[rip+X]` / `mov reg,imm` / `mov reg,[rip+X]` で得た定数を `reg` として使う（`reg_const`、`reg_table`）。ただし分岐命令が基本ブロックの先頭、または別経路の分岐先になっている場合は使わない。
4. それ以外は未解決（`unresolved`）として件数だけ数える。

### 閉包の前後（実測）

| 項目 | 前（closure-run1.txt） | 後（closure-run2-indirect.txt） | 差 |
|---|---|---|---|
| 関数 | 6,956 | 6,969 | +13 |
| 基本ブロック | 67,310 | 67,399 | +89 |
| 命令 | 459,902 | 460,479 | +577 |
| 間接 call（IAT 以外） | 6,482 | 6,490 | +8 |
| 間接 jmp（IAT 以外） | 16 | 26 | +10 |
| IAT 経由 jmp | 402 | 409 | +7 |

閉包の増分が、解決した宛先から来ていることは未検証（推測）。宛先ごとの照合はしていない。

### 解決の内訳（実測、実機 DLL）

| 種類 | 解決方法 | 件数 |
|---|---|---|
| call | rip_table | 12 |
| call | reg_const / reg_table | 0 |
| call | IAT（名前で解決） | 0 |
| call | unresolved | 6,478 |
| jmp | rip_table | 14 |
| jmp | IAT（名前で解決） | 409 |
| jmp | unresolved | 12 |

- call の IAT 経由は 0 件。閉包の計測と一致する。
- レジスタ形式は 0 件。閉包内の間接 call の宛先レジスタは、直前の `lea`/`mov` で定数化されていない（推測: 宛先は計算や他の命令で得ている。未検証）。
- 解決 12 件と 14 件の増分は、閉包を広げた分である。増分 13 関数の内訳は、個々の宛先まで照合していない（未検証）。

### 生成 C（実測）

`lift/out/fle_lift_indirect.c`（新規、既存の `fle_lift.c` は変更していない）。`lift/out/indirect-report.txt`。

| 項目 | 値 |
|---|---|
| 間接 call の直接化（`call_ind:rip_table`） | 12 |
| 間接 jmp の直接化（`jmp_ind:rip_table`） | 14 |
| 未対応 `Call: indirect target unresolved` | 6,478 |
| 未対応 `Jmp: indirect through IAT slot (not lifted)` | 409 |
| 未対応 `Jmp: indirect target unresolved` | 16 |

### 未確定事項

- 未解決の 6,478 件は、本単位の方法では宛先を得られない。メモリ、スタック、他の命令で得た値が必要で、データフロー解析が要る（推測）。
- 閉包は依然として下界である。未解決の間接 call を辿れば閉包は広がる。

## 再計測: データフローによる宛先解決（単位 U-E2、実測）

計測の対象は `lift/` の `fe-decode`（`dataflow.rs`、`indirect.rs` の 1 箇所の呼び出し）。`fe-lift` は変更していない。結果は `lift/closure-run5-dataflow.txt`（新規）、生成側は `lift/out-uE2/`（新規）。

### 閉包の前後（実測）

| 項目 | 前（U-E、`closure-run2-indirect.txt`） | 後（U-E2、`closure-run5-dataflow.txt`） | 差 |
|---|---|---|---|
| 関数 | 6,969 | 6,969 | 0 |
| 基本ブロック | 67,399 | 67,399 | 0 |
| 命令 | 460,479 | 460,479 | 0 |

閉包は変わらなかった。解決した宛先が 0 件のため、閉包を広げる宛先が無かった（実測）。

### 解決の結果（実測）

データフロー解析（前方評価、同一ブロックの直線区間、`.text` の線形走査で得た分岐先による leader 判定）が、既存規則で未解決になった間接 call/jmp に対して返した判定:

| 判定理由 | call | jmp |
|---|---|---|
| `branch_is_leader` | 31 | 0 |
| `mem_form_index` | 4 | 1 |
| `mem_form_unknown_base` | 6,032 | 1 |
| `slot_after_store_or_call` | 81 | 0 |
| `table_not_code` | 14 | 0 |
| `table_unreadable` | 226 | 0 |
| `unknown` | 90 | 10 |
| 解決（code） | 0 | 0 |

判定は 6,490 件。解決は 0 件。

### 分類（担当の (a)〜(d)、実測、独立の走査）

未解決 6,490 件の分岐について、ベースレジスタの最後の定義を同一ブロック内で辿った（`mov reg,reg` と、`mov reg,[reg2+d]` の先の根まで）。

| 分類 | call | jmp | 内容 |
|---|---|---|---|
| (a) 定数計算 | 2 | 1 | `lea rip` / `mov imm` が根 |
| (b) 引数由来 | 260 | 1 | 根が rcx/rdx/r8/r9 の引数 |
| (c) メモリ読み | 573 + 159 | 0 | `call [rip+X]` 159（IAT 以外の rip スロット）、`mov reg,[rip+X]` が根 573 |
| (d) その他 | 5,484 | 10 | 根が `lea` の reg、`pop`、戻り値、2 段以上のポインタ読み等 |

分類は根の種類の集計で、(d) の内訳は `tests/dataflow.rs` の出力（`detail by root`）に出る。(d) の多くは、ポインタを 2 段以上読んだ先の値（`PtrLoad(PtrLoad(...))`）で、値の根が同一ブロックの外にある。

### 解決できなかった理由（推測を含む）

- (a) の 2 件は `call [reg+reg*s+d]` の形（`mem_form_index`）と、根の追跡不能。正しく未解決。
- (c) の `[rip+X]` スロットの内容は、430 件のうち 419 件がコードの VA ではなかった（実測）。スロットは関数ポインタではないと推測される（未検証）。
- (d) の大半は、ポインタの根が同一ブロック外（呼び出し元や別ブロック）にあると推測される（未検証）。
- 解決 0 件の直接原因は、`mov reg,[reg2+d]` の読み出しが `clean` 判定と `Unk` 扱いで落ちること、と推測される（未検証。`dataflow.rs` の `load` と `clean` の判定を参照）。

### 生成側（実測）

`lift/out-uE2/before-report.txt` は、この単位の作業開始時点（データフロー接続の前）の `fe-lift` で生成した。`lift/out-uE2/after-report.txt` は接続後。両者の `diff` は空（全行一致）。

| 項目 | 前（`out-uE2/before`） | 後（`out-uE2/after`） |
|---|---|---|
| `call_ind:rip_table` | 12 | 12 |
| `jmp_ind:rip_table` | 14 | 14 |
| `Call: indirect target unresolved` | 6,478 | 6,478 |
| `Jmp: indirect through IAT slot (not lifted)` | 409 | 409 |
| `Jmp: indirect target unresolved` | 16 | 16 |
| `unsupported_total` | 9,181 | 9,181 |

注: `lift/out-uC/before-report.txt` の `unsupported_total` は 29,393 で、本単位の `before` の 9,181 と一致しない。間接 call/jmp の未対応件数は一致しているので、差は間接以外の未対応（他単位の変更による）と推測される（未検証）。本単位の比較は `out-uE2` 内の前後で行った。

### テスト（実測）

`cargo test --workspace` の結果は、fe-decode の lib 4 件、`tests/dataflow.rs` 4 件（合成 3 件、実機 1 件）、fe-lift の 7 + 5 + 8 + 9 件、fe-pe の実機 9 件と合成 19 件、すべて成功。`tests/dump_tmp.rs` は作業中に誤って作った空のテスト（1 件、成功）で、削除できないため残置している。

## 追記: extra seed 0x41d050 の追加（2026-10-20）

理由: 表 0x13a9410 の要素 [1] として 0x41d050 を呼ぶ読み手 0x41223e が、実行で確かめられているため（`docs/investigations/mode-0x41223e-runtime.md`）。決定の詳細は `docs/decisions/0007-extra-seed-0x41d050.md`。

既定の seed（entry と export）は変えていない。追加の seed は `fe_decode::analyze_with_seeds(pe, extra)` で渡す。`analyze(pe)` は `analyze_with_seeds(pe, &[])` と同じ結果になる（テストで確認）。

計測コマンド: `lift/out-seed-final/target/debug/seed-final closure binaries/win64/FLEngine_x64.dll 41d050`（実測）。

| 項目 | 既定の seed（前） | extra 0x41d050（後） | 差 |
|---|---|---|---|
| seed（コード） | 3 | 4 | +1 |
| 関数 | 6,969 | 6,971 | +2 |
| 基本ブロック | 67,399 | 67,403 | +4 |
| 命令 | 460,479 | 460,492 | +13 |
| IAT 経由 jmp | 409 | 410 | +1 |

既定の seed の数字（6,969 / 460,479）は、この追記の計測でも再現した（実測）。

以後、本番の閉包の数字は、extra seed を含む場合は「6,971 関数、460,492 命令」を正とする。既定の seed だけの場合は、従来どおり「6,969 関数、460,479 命令」。

## 追記: extra seed 0x41e7e0 の追加（2026-10-20）

理由: 0x412126 の `call [rbp+0x38]` が、harness の既定の入力（rdx=1）の 1 回の実行で 0x41e7e0 を呼ぶことが確かめられたため（`docs/investigations/slot-0x412126-runtime.md`）。決定は `docs/decisions/0008-extra-seed-0x41e7e0.md`。0x41d050 は残し、両方を extra seed にした。既定の seed（entry と export）は変えていない。

計測コマンド: `lift/out-seed-both/target/debug/seed-both closure binaries/win64/FLEngine_x64.dll 41d050 41e7e0`（実測）。

| 項目 | 既定の seed | extra 0x41d050 | extra 0x41d050 + 0x41e7e0 | 差（前回の extra 0x41d050 との差） |
|---|---|---|---|---|
| seed（コード） | 3 | 4 | 5 | +1 |
| 関数 | 6,969 | 6,971 | 6,972 | +1 |
| 基本ブロック | 67,399 | 67,403 | 67,404 | +1 |
| 命令 | 460,479 | 460,492 | 460,495 | +3 |
| IAT 経由 jmp | 409 | 410 | 410 | 0 |

閉包の数字の正は、extra seed を両方含む場合は「6,972 関数、460,495 命令」。0x41d050 のみの場合は「6,971 関数、460,492 命令」。既定の seed だけの場合は「6,969 関数、460,479 命令」。

数字が変わることは PM が受け入れた（0008 の決定）。ここで閉包の数字の前提（0006 の「閉包の関数数と命令数は変わらないこと」）は、0007 に続いて置き換えられた。

生成と実行の結果は `docs/investigations/extra-seed-both-run.md` を参照。
