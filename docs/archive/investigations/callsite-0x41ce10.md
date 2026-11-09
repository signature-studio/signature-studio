# 0x41ce10 の呼び出し点（計測、2026-10-20）

対象: `binaries/win64/FLEngine_x64.dll`。停止点 0x41ce10（`extra-seed-both-run.md` §5）の呼び出し元を、実行で確かめた。
作業場所: `lift/out-ce10/`（新規）。本番の既定の生成 C、seed、`out-seed-both/` は変えていない。
入力は既定どおり（entry 0x1376820、rdx=1、harness の既定）。

本文の数字は実測。推測は「推測」と明記する。

## 1. 間接参照と直接 call の列挙（閉包の範囲）

### 1.1 閉包内の間接 call（実測）

- 生成 C（`lift/out-seed-both/out/after-seed-both.c`）の間接 call は、全 6,478 件が `fle_dispatch(t);` の形（`out-seed-both/out/gen-report.txt` の `call_ind:dispatch` と一致）。
- 各行の `/* 0x... Call */` コメントで、呼び出し点の VA を取れる（6,478 件すべてで取れた）。
- 呼び出し点ごとの宛先 `t` は、直前の `fle_ld` で読んだ値。静的には宛先を決められない。
- 閉包内の `call_ind:rip_table`（12 件）は別の形（RIP 相対の表）で、今回の探索では宛先の一覧を取っていない（未検証）。

### 1.2 閉包外を含む、PE 全体の線形走査（`lift/out-ce10/tool/scan_ce10.py`、新規。実測）

| 種類 | 件数 | 閉包内 |
|---|---|---|
| 0x41ce10 への直接 call | 0 | — |
| 即値 0x41ce10 | 0 | — |
| RIP 相対で 0x41ce10 を指す参照 | 0 | — |
| 0x41ce10 を値に持つ RIP 相対スロットの読み出し | 0 | — |
| RIP 相対の間接 call で、スロットの値が 0x41ce10 のもの | 0 | — |
| 8 バイトの値 0x41ce10 の出現（全セクションの生バイト） | 1（0x13768f0、`.text` 内。DIR64 の再配置なし） | — |

注意（実測と推測の区別）:
- 走査の対象は、直接 call、RIP 相対のオペランド、RIP 相対の間接 call だけ。レジスタや `[rbp+X]` を介する間接 call（0x412126 の形。PE 全体で mem call 49,042 件、reg call 1,237 件）は、静的には宛先を解決していない。よって「0 件」は、RIP 系の参照についての 0 件であり、`[rbp+X]` や `[reg+X]` 経由の参照が 0x41ce10 を指すかどうかは、静的には未確認。これを確かめたのが §2 の実行。
- 線形走査（`ce10-scan-summary-v2.txt`）は、デコードできないバイトを 1 バイト飛ばして進めた。デコードできないバイトは 161,366 あり、走査は完全ではない（推測: 一部の命令を取り逃がしている）。
- 0x13768f0 の 1 件は `.text` 内のコード上の一致で、表の要素ではないと読める（推測）。
- 上の表は「0 件」の確認であり、完全な証明ではない。実行の probe（§2）が、閉包内の全間接 call を対象にした確認になる。

### 1.3 0x41ce10 の値の書き込み先（実測と推測）

- `.data` や `.rdata` に 0x41ce10 の値は無い（上の走査。DIR64 の再配置も無い）。よって、0x41ce10 は静的な表に入っていない。
- 0x412126 の宛先 `[rbp+0x38]` の元は、`[[0x15eac90]+8 + 添字×16]` のテーブル要素（`extra-seed-both` と `slot-0x412126-runtime.md` の既存の読み）。
- `[0x15eac90]` を直接書く命令は、生成 C に無い（`fle_st(UINT64_C(0x15eac90)...)` は 0 件。`[0x15eac90]` の読み出しは 2 箇所（0x41202c、0x4120cc））。`objdump-text.txt` でも `# 0x15eac90` の参照は 2 行（読み出しのみ）。
- 0x15eac98（件数）への書き込みは 3 箇所（0x412057、0x41210f、0x412948）。
- テーブル要素そのものが、どの書き込みで 0x41ce10 になったかは、未検証（推測: ヒープ上のテーブルに、閉包外の書き込みで値が入る）。

## 2. 実行での呼び出し点の確認（実測）

### 2.1 計測用の複製

- 作成: `lift/out-ce10/out/after-ce10.c`（`out-seed-both/out/after-seed-both.c` の複製）。
- 変更点: 閉包内の全 6,478 件の `fle_dispatch(t);` の直前に、`if (t == 0x41ce10)` のときだけ stderr に出す probe を入れた。`fle_dispatch` 本体は変えていない。
- 出力: `CE10 site=<呼び出し点 VA> hit=<回数> target=<宛先> rsp=<rsp>`。
- ビルド: `lift/out-ce10/build.sh`（新規。`out-slot/build.sh` と同じ手順）。結果は exit=0（`build-log.txt`）。

### 2.2 実行（実測）

- コマンド: `lift/out-ce10/flengine-after-ce10 binaries/win64/FLEngine_x64.dll`。stderr は `lift/out-ce10/run-ce10.txt`。終了コード 134。
- probe の出力（1 行のみ）:

```
CE10 site=0x412126 hit=1 target=0x41ce10 rsp=0x7c2f9ddffc18
fle: indirect call to 0x41ce10 is not a known function entry
```

| 項目 | 結果 |
|---|---|
| 呼び出し点の VA | 0x412126（閉包内、f_4120c0 の中。`call [rbp+0x38]`） |
| 宛先の値 | 0x41ce10 |
| 0x412126 から 0x41ce10 への到達回数 | 1 回（probe の hit=1） |
| 停止の種類 | fle_dispatch_miss（0x41ce10 が関数の入口に無い） |
| 関数の入口（呼び出し回数） | 18 |
| 呼び出し深さ（停止時） | 4（#0 f_4120c0、#1 f_412160、#2 f_41d8f0、#3 f_1376820） |
| 停止の stderr（既存の `out-seed-both/run1.txt` と比較） | 関数数、深さ、スタックは一致 |

- 同じ実行の entry log では、entry 17 が `f_41e7e0`（guest-return 0x412129）。これは 0x412126 の `call [rbp+0x38]` で、宛先 0x41e7e0 の呼び出し（`slot-0x412126-runtime.md` の既存の観測と一致）。
- 読み（推測）: 0x412126 の同じ call 点を、ループで 2 回目以降に通り、そのとき宛先が 0x41ce10 になった。probe は 0x41ce10 の宛先だけを出すので、0x41e7e0 の呼び出しは出力に出ていない（probe の仕様どおり）。

### 2.3 probe の影響（実測）

- 停止の位置、関数の数（18）、深さ（4）、スタックは、probe を入れない既存の実行（`out-seed-both/run1.txt`）と一致した。probe が挙動を変えていない。

## 3. 呼び出し点の由来

- 呼び出し点 0x412126 の由来は、`slot-0x412126-runtime.md` §1 と同じ（実測。objdump と生成 C の一致）。
  - 0x412104 で `[rbp+0x38] = [[0x15eac90]+8 + 添字×16]` を書く。
  - 0x41211c〜0x412124 で `[rax]` が 0 でないことを確認してから、0x412126 で call する。
- 値 0x41ce10 がテーブルのどの要素に入ったかは、未検証（§1.3）。
- 0x41ce10 の関数（`push rbp; sub rsp,0x20; mov rbp,rsp`）は、閉包の外にある（生成 C に `f_41ce10` が無い。実測）。

## 4. 結論: 0x41ce10 を extra seed に加える根拠は、実行で確かめられたか

**yes（この 1 回の実行の範囲で）。**

根拠（実測）:
- 閉包内の間接 call 0x412126 の宛先は、既定の入力での 1 回の実行で 0x41ce10 だった。
- 0x41ce10 は閉包の外（dispatch 表に無い）で、そこへの dispatch が miss で止まった。

0x41e7e0 と同じ水準（既定の入力での 1 回の実行の観測）である。

未検証（同じ水準で残るもの）:
- 入力を変えた実行での値。
- テーブル要素 `[[0x15eac90]+8]` のどの要素が 0x41ce10 になったか、その書き込み元。
- 0x41ce10 を seed に加えた後の挙動（その先で別の dispatch miss が出るか）。加えて実行するまで分からない。
- 閉包の生成と実行は、今回は行っていない（指示どおり）。
- 線形走査（§1.2）は完全ではない。

## 5. 残りの作業量

- 0x41ce10 を seed に加えるかの判断は PM。加える場合は、閉包の生成、ビルド、実行が必要（今回は未実施）。
- 追加で確かめるなら: (a) テーブル要素の書き込み元の追跡、(b) 別の入力での 0x412126 の宛先の確認、(c) 0x412126 の 0x41ce10 の呼び出しが何回目のループかの確認。いずれも未実施。
- フル lift（停止点ゼロ）は未達成。

## 6. 操作の記録

- 新規作成: `lift/out-ce10/`（`tool/scan_ce10.py`、`out/after-ce10.c`、`out/ce10-refs.tsv`、`out/ce10-refs-v2.tsv`、`out/ce10-scan-summary.txt`、`out/ce10-scan-summary-v2.txt`、`build.sh`、`build-log.txt`、`flengine-after-ce10`、`run-ce10.txt`、`run-ce10-stdout.txt`、`harness/`、`tmp/`、`*.o`）、`docs/investigations/callsite-0x41ce10.md`。
- 失敗した操作（自己の不具合。すぐ修正した）:
  - 線形走査の初版は、デコード不能なバイトで 4 命令で止まった。走査を 1 バイト飛ばしに直し、出力を `-v2` の新しいファイル名に変えて再実行した（`x` モードの既存ファイルを上書きしないため）。
- 使っていないもの: rm、既存ファイルの上書き、git 操作（status、log、diff を含む）、/tmp、sleep、timeout。
- 権限拒否: なし。
- 2 GB 級の中間ファイル: 作っていない（`flengine-after-ce10` は約 43 MB、`after-ce10.c` は約 39 MB）。
