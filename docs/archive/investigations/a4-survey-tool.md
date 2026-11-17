# A4 の進捗測定ツール（a4-survey）

実行日: 2026-10-21〜22。HEAD c2fc046 に未コミットの変更を載せた状態。定義は `docs/decisions/0060-a4-measure.md`。成果物は `lift/out-surv/`（gitignore 済み）。「実測」はコマンドの出力。

## 1. 使い方

```
bash lift/harness/a4-survey.sh            # native だけ
WASM=1 bash lift/harness/a4-survey.sh     # wasm(node) も実行し、a4 の行が native と同じか確認する
```

- 前提: `$OUT/native/flengine-native`（既定 `OUT=lift/out-surv`）。作り方は `OUT=... bash lift/harness/wasm/build_native.sh`（生成 C は `build.sh gen split` の出力を再利用）。`WASM=1` は `$OUT/bin-base/flengine.js`（`build.sh`）も要る。
- ログは `$OUT/a4/<UTC 時刻>-{units.tsv,native.txt,native.err,node.txt,node.err}`（毎回新規）。終了コード: 0 = 測定できた（停止点は結果であってエラーではない）、2 = 前提が無い、3 = WASM=1 で native と wasm の a4 行が違う。
- 静的な一覧だけ: `fe-pe pe-init-table FILE [OUT.tsv]`（`lift/crates/fe-pe`）。TSV は `index proc_va kind second_va run_rank`（942 行 + ヘッダ）。実測の出力は `lift/out-surv/units.tsv`。
- harness の停止報告（`trace_report`）に `a4:` の行が入る（`lift/harness/a4.c`）。生成 C は変えていない。計数は既存の `-finstrument-functions` の入口フックを使う（新しい計数器なし。生成コードの性能・サイズへの影響は 0）。

## 2. 静的な unit 表（実測。DLL のバイト列。fe-pe と、独立の Python の読みが一致）

| 項目 | 実測 | 0046 の記録 | 差 |
|---|---|---|---|
| UnitCount（ヘッダ 0x13768B0 の +0） | 942 | 942 | 0 |
| UnitsPtr（+8） | 0x13768E0 | 0x13768E0 | 0 |
| runnable | 895（proc の異なる値 895） | 895 | 0 |
| empty | 46（全て proc 0x13a9390） | 46 | 0 |
| noproc | 1（index 941） | 1 | 0 |
| ヘッダの出どころ | DllMain 0x1376858 の `lea rdx,[rip+0x51]` = 0x13768B0 | 同じ | 0 |

食い違いはない。InitContext（0x15EAC90、`+8` = 0x15EAC98）は .bss で実行時に 0x4121bb が書く。ファイルからは読めない（DLL の objdump で、`[0x15eac98]` の書き込みは 0x412057、0x41210f、0x412948 の 3 か所）。

## 3. 現在の停止点の実測（native と wasm が一致）

```
stop   : fle: unsupported instruction at 0x4250e0        (user32!LoadCursorW の thunk)
depth  : init_count=115 / 942   last unit: index=114 run_rank=111/895 proc=0x623840 in_progress
         started=111 completed=110 in_progress=1 of 895 runnable   all_units_done=no
         entry check: started_but_not_entered=0 runnable_entered_beyond_init_count=0
funcs  : executed (distinct)=578  entries=13245
```

- スタック: f_4250e0 ← f_8659f0 ← f_8651a0 ← f_620cd0 ← f_623840（unit 114 の proc）← f_4120c0 ← f_412160 ← f_41d8f0 ← エントリ。
- 今日の基準値: **completed 110 / 895（12.3%）、InitCount 115 / 942**。0046 の時点（index 17 で停止）から、index 114 まで進んだ。
- native と wasm(node): `calling entry` 行（スタックのアドレス）を除いて、出力全文が一致（`diff` が空）。a4 の行は 6 行とも同じ。

## 4. 限界

- InitCount は「index+1」で完了数ではない（定義は 0060）。報告の completed は、最後の unit の proc がスタックにあるかで補正した値。
- 停止（abort）か戻り（`returned from entry`）の時点しか測らない。途中経過の時系列は無い。
- 入口の計数は「その関数が一度でも入った」を表す。unit 内部で何が実行されたかは測らない。
- `entry_check` の `runnable_entered_beyond_init_count` は、まだ started でない unit の proc が別の経路で呼ばれた数（現在 0）。
- 計数の元の入口フックは `-finstrument-functions` のビルド（既存の build.sh の `INSTRUMENT=1`、native も同じ）が前提。フックなしのビルドでは `entry_check` が意味を持たない。
- 旧 `harness/build.sh`（`out-dispatch/after.c` 用）は a4.c をリンクしない。`harness/main.c` は weak 参照にして壊さない（a4 行は出ない）。
- native の trace は、これまで gitignore 下の `lift/out-loc/trace.c` だった。同一内容に a4 の 3 つのフックを足して `lift/harness/wasm/trace_native.c` として追跡下に置き、`build_native.sh` の既定を変えた（`NATIVE_TRACE` で旧ファイルも指せる）。

## 5. テスト

- `crates/fe-pe/tests/inittable.rs` 10 件: 合成 PE（全 kind、.bss、範囲外、上限）と実 DLL（942/895/46/1、既知の unit）。変異 23 件を全て検出（`out-surv/mutpe/`、`out-surv/mkmut_pe.py`）。
- `crates/fe-lift/tests/a4_report.rs` 15 件: `a4.c` を偽のイメージと偽の入口フックで固定入力から実行（InitCount が noproc/empty に乗る場合、in_progress、done 条件、範囲外、イメージ端の 8 バイト）。変異 39 件を全て検出（`out-surv/mut3/`、`out-surv/mkmut.py`）。最初の実行で生き残った変異 6 件（4 バイト読み、境界など）は、テスト入力とコードの冗長な条件を直して検出するようにした。
- ワークスペース全体（`out-surv/test-ws.txt`）: 79 バイナリ、802 件、失敗 0。
