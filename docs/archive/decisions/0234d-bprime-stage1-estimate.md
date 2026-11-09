# 0234d(mt3): B′ の第 1 段(ゲストの文脈 R/X/… の `_Thread_local` 化。opt-in)の見積りの精査 — 決定 0234(mt2)§15 の絞り込み

状態: **読みと micro の実測**。この段は**まだ実装していない**(PM の指示: 着手は 04:10 の報告で PM が決める)。区分: **実測**=このセッションで出力を見た物(下の micro)、**読み**=コード・測定からの解釈、**未確認**。

## 1. 生成 C の形を変える範囲(fe-lift の出力の変更行数)

- **fe-lift の出力は 0 行**(読み)。生成 C は `R`・`FL`・`X` を**名前で**使い(`R[4] -= 8; fle_st(...)`)、定義と extern 宣言は 1 行ずつ生成 C の頭にあるだけ。出荷の LOWREGS=2 では、その頭の行を後処理 `lift/harness/two/lowregs.py`(31 行)が `#ifndef FLE_LOWREGS … #endif` で包み、`R`・`FL` を `fle_lowregs.h` のマクロ(固定アドレス)に置き換えている(`X` は `FLE_LOWREGS_X`)。**TLS 版はこの後処理のもう 1 つのモード**(`FLE_TLS_REGS`: 同じ行を `#ifndef FLE_TLS_REGS` で包み、定義を runtime の 1 ファイルに置く)で足りる。= **fe-lift の出力は変わらず、既定の生成 C の sha(cd9107ce…)も変わらない**(後処理は出荷の全ビルドで既に通っている。TLS モードは opt-in)。
- 影響する変数(協調スケジューラの `save_ctx`/`load_ctx` が列挙する物): `R`、`X`、`YH`、`FL`、`MM`、`fle_x87_r`、`fle_x87`、`fle_x87_mmx_active`、`fle_mxcsr`、`fle_landing_top`、`fle_seh_top`、`fle_seh_cur`、コールバックの深さ、トレースの深さ、`fle_teb_cur`、`fle_tls_cur` = **16 変数**。各 2〜4 行の宣言・定義(runtime のヘッダ・ソース)= 約 50〜70 行 + 新規 `fle_regs_tls.c`(約 60 行)+ `lowregs.py` の新しいモード(約 15 行)。
- **直 wasm(fe-wasmgen の混成)**は定数アドレス(`--hybrid-r` = LOWREGS_BASE 0x1000、`--hybrid-x`、`--hybrid-mx 0x3000 0x3100`、`--hybrid-progress 0x3300`、`--teb`)を埋め込む。TLS 版では `global.get $__tls_base` + 定数のオフセットにする(関数ごとに基底を 1 回だけローカルに取る)。fe-wasmgen の変更は約 100〜200 行 + テスト約 100 行(`mt2` の §15 の見積りと同じ)。
- リンク: `-matomics -mbulk-memory`(共有メモリ版の mt2 の道。`MT2_SHARED=1`)と `__wasm_init_tls`(最初のインスタンスの呼び出し 1 回)。

## 2. 1 スレッドのときの CPU 費用(micro。実測。`tables/tlsbench-single-thread.txt`、`tools/tlsbench2.c`)

lifted C の形(レジスタファイルが extern で、全部の関数呼び出しが未知のコードへの呼び出しなので、コンパイラが `R` をレジスタに置き続けられない)の最小の核を、(c)固定アドレス(出荷の LOWREGS=2)、(g)データシンボル、(t)`_Thread_local` の 3 通りで emcc 6.0.12 -O2 + node 24.19(1 CPU に固定、5 回の最小、load 約 12):

| 核 | (c) 固定アドレス | (g) データシンボル | (t) TLS |
|---|---|---|---|
| flat(1 億ステップ) | 418 ms | 371 ms | 404 ms |
| calls(3 関数の鎖を 3 千万回) | 753 ms | 643 ms | 632 ms |

= **TLS は出荷の固定アドレス形より遅くない**(flat −3%、calls −16%)。(g) より flat で +9%、calls で −2%。3 通りの結果は一致。LOWREGS が選ばれた理由はコードの大きさ(LEB128)で、速さではない。TLS のオフセットは小さい(< 4 KiB)ので、命令の長さは固定アドレスと同じ(`global.get $__tls_base; i64.load offset=<小>`)(読み)。**実エンジンでの値は未測定**(TLS 版を作らないと出ない)。最初の版の micro(R が static でコンパイラがレジスタに昇格させた物)は代表的でなく、その値(TLS が固定アドレスより 20% 遅く見えた)は採らない(`tables/tlsbench-single-thread.txt` の末尾に残した)。第 1 報の mp1 の『TLS 126 ms 対 グローバル 105 ms』も同じ理由で過大。

## 3. 同一性

協調モード(1 Worker)では、スレッドの切り替えが文脈を値で保存・復元する今の仕組み(`save_ctx`/`load_ctx`)のままで、TLS はその Worker の中の 1 本の文脈になるだけ。= **84 行(cmp-short)、参照プロジェクトの 55 行、parity の 552 行は変わらないはず**(読み。**未検証**。TLS 版をビルドして確かめる)。実スレッド(2 Worker)の同一性は別(PCM のハッシュ。mt2 §6)。

## 4. 費用(読み)

第 1 段だけ(協調モードのまま TLS 化、opt-in、既定の出力は不変): ランタイム 約 130 行、後処理 15 行、fe-wasmgen 100〜200 行、テスト 100〜200 行、CI の変種 1 つ、ビルドと同一性の確認 = **1.0〜1.5 人週**。第 2 段(アトミック、LOCK)以降は mt2 §15 のとおり。第 1 段の単独の価値: (1) 実スレッドの前提が整う、(2) 切り替えごとの文脈の複写(約 1.6 kB × 約 2,000 回/s = 3 MB/s、CPU には効かない小ささ)が TLS のポインタの付け替えにできる可能性(今の複写は値渡しで、Worker をまたがない第 1 段では複写が残る)。

## 5. 未確認

TLS 版のエンジン全体の CPU 費用と wasm の大きさ(S1 の線)、ハイブリッドの直 wasm への影響、`X`/`YH` の 16 バイト整列(TLS ブロックの整列指定)、Blend2D・freetype など他モジュールの `_Thread_local` との衝突。
