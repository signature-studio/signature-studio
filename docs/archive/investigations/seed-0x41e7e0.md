# 0x41e7e0 を seed に加えた場合の閉包・生成 C・実行の結果

対象: `binaries/win64/FLEngine_x64.dll`（読み取りのみ）。
計測は `lift/out-seed41e/`（新規）で行った。本番の seed 既定値（entry と export）、`lift/crates/`、`lift/runtime/`、既存の生成物は変えていない。

「実測」は計測ツールと実行ログの出力。「推測」は未検証の解釈。

## 0. 方法

- 計測用の複製 `lift/out-seed41e/crates/`（現行の `lift/crates` を `cp -r` で複製）に、extra seed の口を足した。
  - `fe-decode`: `analyze_with_seeds(pe, extra)` を追加。`analyze` は `&[]` で呼ぶ形に変更。
  - `fe-lift`: `generate_with_seeds(data, extra, w)` を追加。`generate` は `&[]` で呼ぶ形に変更。
- 計測ツール `lift/out-seed41e/tool/gen41e`（新規）。`gen41e FILE OUT.c [SEED_HEX ...]`。
- 基準は現行の `lift/crates` で取り直した（extra なし）。`lift/out-string/after-report.txt` は古い生成器の数字で、今回の基準とは一致しない（後述）。
- ビルド:
  - `lift/out-seed41e/build.sh`（入力 `out/plus41e.c`。GetCurrentThreadId だけ書き換え）。
  - `lift/out-seed41e/build-both.sh`（入力 `out/plus41d-41e.c`。GetCurrentThreadId、TlsAlloc、LocalAlloc、TlsSetValue、TlsGetValue、GetLastError の 6 件を書き換え。各書き換えは grep の件数で確認）。
- 実行: `./flengine_* ../../binaries/win64/FLEngine_x64.dll`。

## 1. 閉包と生成 C（実測）

生成レポート: `lift/out-seed41e/out/base-check-report.txt`、`plus41e-report.txt`、`plus41d-41e-report.txt`。

| 項目 | 基準（extra なし） | 0x41e7e0 のみ | 0x41d050 + 0x41e7e0 |
|---|---|---|---|
| 関数 | 6,969 | 6,970（+1） | 6,972（+3） |
| 閉包の命令（closure_instructions） | 460,479 | 460,482（+3） | 460,495（+16） |
| 生成した命令（emitted） | 460,559 | 460,562（+3） | 460,575（+16） |
| 未対応（unsupported_total） | 1,570 | 1,570（+0） | 1,571（+1） |
| 未対応の種類数 | 64 | 64 | 64 |
| goto | 42,815 | 42,815（+0） | 42,816（+1） |
| C のバイト数 | 39,175,934 | 39,176,343（+409） | 39,177,589（+1,655） |

- 0x41d050 + 0x41e7e0 の増分（実測）: 関数 +3、命令 +16、未対応 +1。
- 増えた関数は 3 件（実測。生成 C の `void f_...(void)` 行の差）: `f_41cf40`、`f_41d050`、`f_41e7e0`。
- 未対応の増分は 1 件、`Jmp: indirect through IAT slot (not lifted)` の 409 → 410（実測）。増えた未対応は `L_41cf40`（IAT jmp）。
- 0x41e7e0 のみの場合、増分の未対応は 0（実測）。
- ニーモニック別の未対応の増分: `Jmp`（IAT）+1。他は変化なし（実測。両レポートの全行比較）。
- 未対応を含む関数: 基準 837 → 838（+1。`f_41cf40` が増えた。実測）。

### 基準の差（実測）

- `lift/out-string/after-report.txt` の基準は未対応 1,576、c_bytes 39,175,730。現行の `lift/crates` で取り直すと未対応 1,570、c_bytes 39,175,934。
- 差の原因は推測: `lift/crates/fe-lift/src/mxcsr.rs`（stmxcsr、ldmxcsr の lift）が後から入ったため。実測として、現行の生成 C では `fle_mxcsr_store` が 2 件あり、`lift/out-string/after.c` では 0 件。`mxcsr.rs` を外した比較は未実施。

## 2. ビルドとリンク（実測）

- `sh lift/out-seed41e/build.sh` は成功（`flengine_seed41e`）。
- `sh lift/out-seed41e/build-both.sh` は成功（`flengine_seed41d41e`）。
- stmxcsr は現行の生成器で `fle_mxcsr_store` に lift されているため、前回の計測にあった stmxcsr の書き換え（`out-mxcsr/build.sh`）は今回不要だった（実測。生成 C の `L_40a3b2` 等で確認）。
- thunk 書き換えの対象行は、閉包に存在する行だけ（grep の件数が 1 件であることを確認）。

## 3. 実行の結果（実測）

### 3.1 0x41e7e0 のみ（`lift/out-seed41e/run1.txt`）

```
fle: indirect call to 0x41d050 is not a known function entry
abort kind: fle_dispatch_miss
dispatch target: 0x41d050
functions executed (distinct): 7
call depth at stop: 3
```

- 停止点は 0x41d050（dispatch miss）。0x41e7e0 の本体には入っていない。
- 0x41e7e0 だけでは不足で、0x41d050 も必要。前回（0x41d050 だけの計測）の結果と合わせると、0x41d050 → 0x41cf40 → … の経路で 0x41e7e0 に届く、と読める（推測。経路は未確認）。

### 3.2 0x41d050 + 0x41e7e0（`lift/out-seed41e/run-41d-41e.txt`）

```
fle: indirect call to 0x41ce10 is not a known function entry
== stop: SIGABRT
abort kind: fle_dispatch_miss
dispatch target: 0x41ce10
functions executed (distinct): 18
function entries (calls): 18
call depth at stop: 4
  #0 0x4120c0 f_4120c0 guest-return=0x41229a
  #1 0x412160 f_412160 guest-return=0x41d989
  #2 0x41d8f0 f_41d8f0 guest-return=0x1376875
  #3 0x1376820 f_1376820 guest-return=0xdead0000
```

- 停止点: VA 0x41ce10。
- abort の種類: `fle_dispatch_miss`（表に無い間接 call の先）。
- 関数実行数（distinct）: 18。entry 数: 18。
- 呼び出し深さ: 4（0x41ce10 は含まない。0x41ce10 は未到達）。

## 4. 0x41e7e0 の本体に入ったか（実測）

- 入った。trace の entry log の 17 番目: `entry 17 callee=0x41e7e0 f_41e7e0 guest-return=0x412129`。
- 戻り先 0x412129 は、0x412126 の間接 call（`call [rbp+0x38]`、f_4120c0 の中）の次の命令。
- その後、同じ呼び出し元（f_4120c0）から 0x41ce10 への間接 call で dispatch miss が起きた。
- 0x41e7e0 の本体（`sub DWORD PTR [rip+0x11cdae1],1` の後の `cmp`、`ret`）の実行は、trace に entry log としてのみ出ている。内部の命令単位のログは取っていない（推測ではなく、そのログ形式を使っていないという事実）。

## 5. 次の停止点（実測と推測）

objdump（`binaries/win64/FLEngine_x64.dll`）:

```
41ce10: 55                push   rbp
41ce11: 48 83 ec 20       sub    rsp,0x20
41ce15: 48 8b ec          mov    rbp,rsp
41ce18: 90                nop
41ce19: 83 2d 18 9c 1c 01 01  sub DWORD PTR [rip+0x11c9c18],0x1   # 0x15e6a38
41ce20: 83 3d 11 9c 1c 01 ff  cmp DWORD PTR [rip+0x11c9c11],0xffffffff
41ce27: 0f 85 c8 00 00 00    jne    0x41cef5
41ce2d: e8 7e 26 ff ff       call   0x40f4b0
41ce32: e8 b9 03 ff ff       call   0x40d1f0
41ce37: e8 b4 cd fe ff       call   0x409bf0
```

- 性質: 内部の関数の先頭（IAT ではない。`lift/out-imports/imports.txt` に 0x41ce10 は無い。実測）。
- 0x41e7e0 と同じ形（`sub DWORD PTR [rip+...],1` で始まるカウンタ操作）。カウンタのアドレスは 0x15e6a38（0x41e7e0 は 0x15ec2c8）。
- 閉包に入っていない（生成 C に `f_41ce10` が無い。`fle_dispatch_table` に 0x41ce10 が無い。実測）。
- 呼び出し元: f_4120c0 の中の唯一の間接 dispatch（`L_412126`、`call [rbp+0x38]`）。この 1 点が呼び出し元と読める（推測。f_4120c0 内の dispatch 行を grep で確認した結果が 1 件だったという実測から導いた）。
- 同じ call 点を 2 回目に通ったときに 0x41ce10 が来た、という経路の読みは推測。停止時の呼び出し深さだけでは、0x41e7e0 の戻りの後だと確定できない。

## 6. 増分の未対応を含む関数（実測）

- 増分の関数は 3 件（0x41cf40、0x41d050、0x41e7e0）。
- 未対応を含むのは `f_41cf40` の 1 件のみ（`Jmp: indirect through IAT slot`、1 件）。
- ニーモニック別の増分: `Jmp`（IAT）1 件。

## 7. 失敗・拒否された操作

- 権限拒否: なし。
- git 操作: なし（status、log、diff を含む）。
- /tmp: 使用なし（TMPDIR は `lift/out-seed41e/tmp`）。
- sleep、timeout: 使用なし。
- rm: 使用なし。既存ファイルの上書き: なし。
- 既存の `lift/crates`、`lift/runtime`、`lift/out-*`（out-seed41e 以外）は変更なし（`find -newer` で確認。該当なし）。
- 作業中の誤り（修正済み）:
  - 作業ディレクトリ（workdir）の指定を 3 回誤り、`cd` と相対パスが失敗した。いずれも副作用なし（失敗したコマンドのみ）。
  - `build-both.sh` のテストの期待値を一度 `-eq 2` と書いたが、新規ファイルのうちに `-eq 1` に直した（ビルドは直後に実行。副作用なし）。
  - `build.sh` の sed 行に不要な `printf` が残ったが、結果には影響なし。
- 作業用の補助ファイル（新規）: `lift/out-seed41e/out/*.fns`、`*.unsup-fns`、`f_4120c0.extract.txt`。

## 8. 残りの作業量（実測の範囲）

- フル lift（依頼の最終目的）は未達。
- 次の停止点は 0x41ce10（内部、閉包外）。前回と同じ型。繰り返さない方針のため、ここで止めた。
- 0x41ce10 を閉包に加えるか（seed 追加の判断）、間接 call の解決を直すかは未決の事項。
- 0x41e7e0 の本体の残りの経路（戻り後の処理）は未確認。
- 本番の seed 既定値、生成器の既定値は変えていない。本番に採用するかは PM に委ねる。
