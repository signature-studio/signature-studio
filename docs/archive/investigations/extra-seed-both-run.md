# extra seed 0x41d050 と 0x41e7e0 の併用（閉包・生成・実行の計測、2026-10-20）

対象: `binaries/win64/FLEngine_x64.dll`。決定は `docs/decisions/0008-extra-seed-0x41e7e0.md`。
作業場所: `lift/out-seed-both/`（新規）。既存の `out-final/`、`out-seed-final/`、`out-slot/`、`out-seed41e/` は読み取りのみ。
ビルド・実行の入力は既定どおり（entry 0x1376820、rdx=1、harness の既定）。

本文の数字は実測。推測は「推測」と明記する。

## 1. 閉包（実測）

計測: `lift/out-seed-both/target/debug/seed-both closure binaries/win64/FLEngine_x64.dll 41d050 41e7e0`

| 項目 | 既定の seed | extra 0x41d050 | extra 0x41d050 + 0x41e7e0 | 0x41e7e0 の増分 |
|---|---|---|---|---|
| seed（コード） | 3 | 4 | 5 | +1 |
| 関数 | 6,969 | 6,971 | 6,972 | +1 |
| 基本ブロック | 67,399 | 67,403 | 67,404 | +1 |
| 命令 | 460,479 | 460,492 | 460,495 | +3 |
| デコード失敗 | 0 | 0 | 0 | 0 |
| IAT 経由 jmp | 409 | 410 | 410 | 0 |

既存（extra 0x41d050 のみ、6,971 / 460,492）との差は、+1 関数、+3 命令。
既定の seed との差は、+3 関数、+16 命令。

## 2. 生成（実測）

計測: `seed-both lift ... out/after-seed-both.c 41d050 41e7e0`。出力は `lift/out-seed-both/out/after-seed-both.c`（新規）、報告は `out/gen-report.txt`（新規）。

| 項目 | 値 |
|---|---|
| 閉包の命令 | 460,495 |
| 関数 | 6,972 |
| 出力の命令 | 460,575 |
| bad_nodes | 0 |
| c_lines | 502,421 |
| c_bytes | 39,177,903 |
| 応答の呼び出し（`call_ind:dispatch`） | 6,478 |
| 応答の呼び出し（`call_ind:rip_table`） | 12 |
| fle_unsupported（unsupported_total） | 1,561（`out-seed-final` の 1,561 と同じ） |
| 未対応の種類 | 64 |

`out-seed-final/out/gen-report.txt` との差（diff、実測）: `cmp` +1、`ret` +1、`sub` +1 の 3 命令。`c_lines` +9、`c_bytes` +409。未対応件数は変わらない。

注: 生成の `call_ind:dispatch` は、閉包内で解決できなかった間接 call の数（6,478）を表す。既存の `out-seed-final` と同じ数。

## 3. コンパイルとリンク（実測）

- 手順: `lift/out-seed-both/build.sh`（新規。`out-seed-final/build.sh` と同じフラグ）。
- 結果: clang -O0 のコンパイル、リンクともに成功。出力 `lift/out-seed-both/flengine-after-seed-both`。
- 警告: 生成 C は `-w` で、harness と runtime は `-Wall -Wextra`。build の出力に警告は無かった（`tail -30` で確認。実測）。

## 4. 実行（実測）

コマンド: `lift/out-seed-both/flengine-after-seed-both binaries/win64/FLEngine_x64.dll`（ログ `lift/out-seed-both/run1.txt`）。
終了コード: 134（SIGABRT）。

| 項目 | 今回（0x41d050 + 0x41e7e0） | 前回（0x41d050 のみ） | 前回（0x41e7e0 の計測、`out-seed41e` 等） |
|---|---|---|---|
| 停止点 | 0x41ce10 | 0x41e7e0 | 0x41ce10（`lift/out-seed41e/run-41d-41e.txt`、0x41d050 と 0x41e7e0 の既存の実行） |
| 停止の種類 | fle_dispatch_miss | fle_dispatch_miss | fle_dispatch_miss |
| 関数実行数（distinct） | 18 | 17 | 18 |
| 関数の入口（呼び出し回数） | 18 | 17 | 18 |
| 呼び出し深さ（停止時） | 4 | 4 | 4 |
| 停止時のスタック | #0 f_4120c0、#1 f_412160、#2 f_41d8f0、#3 f_1376820 | 同左 | 同左 |

- 2 列目は `lift/out-seed-final/run-after-seed41d050.txt`（0x41d050 のみ）。
- 3 列目は `lift/out-seed41e/run-41d-41e.txt`。今回と同じ seed の組（0x41d050 と 0x41e7e0）の既存の実行で、停止点、件数、深さ、スタックが今回と一致した（実測）。今回の結果は既存の計測の再現になる。
- 進んだかどうか: 2 列目の停止点 0x41e7e0 を越えて、0x41ce10 まで進んだ（実測）。

- 前回の 0x41e7e0 の停止から、**進んだ**（実測）。0x41e7e0 は今回の entry 17 として呼ばれ、戻った。
- 停止時の深さ 4、#0 が f_4120c0 である（実測）。0x41e7e0 の関数が戻った後に、f_4120c0 が直接 0x41ce10 を呼んだ、と読める（推測。戻り先 0x412129 は `entry 17 guest-return=0x412129` の記録と一致する。実測）。

## 5. 次の停止点 0x41ce10（実測）

- VA: 0x41ce10。`fle: indirect call to 0x41ce10 is not a known function entry`。
- 命令（objdump、`lift/objdump-text.txt` の 39691 行付近。0x11fb4af より前なので既存の objdump を使った）:

```
41ce10:  55                   push   rbp
41ce11:  48 83 ec 20          sub    rsp,0x20
41ce15:  48 8b ec             mov    rbp,rsp
41ce18:  90                   nop
41ce19:  83 2d 18 9c 1c 01 01 sub    DWORD PTR [rip+0x11c9c18],0x1   # 0x15e6a38
41ce20:  83 3d 11 9c 1c 01 ff cmp    DWORD PTR [rip+0x11c9c11],0xffffffff
41ce27:  0f 85 c8 00 00 00    jne    0x41cef5
41ce2d:  e8 7e 26 ff ff       call   0x40f4b0
...
```

- 性質（実測）:
  - 内部の関数の先頭。IAT ではない（`lift/out-imports/imports.txt` に無い。`seed-0x41e7e0.md` と同じ）。
  - 閉包の外。生成 C に `f_41ce10` が無い。`fle_dispatch_table` に無い（`grep` の結果）。
  - 0x41ce10 を指す間接 call は f_4120c0 の中の、既存の記録（`seed-0x41e7e0.md` §3.2）と同じ call 点と推測される。今回は呼び出し元の命令の特定はしていない（未検証）。
- 繰り返していない。0x41ce10 を seed に加える判断は、PM が決める。

## 6. テスト（実測）

- 追加: `lift/crates/fe-decode/tests/extra_seed.rs` に `extra_seed_41e7e0_enters_closure` を 1 件（既存のテストは変えていない）。
  - 0x41e7e0 と 0x41d050 の両方が関数の入口であること、seed が 5 件であること、閉包の数字（6,972 / 460,495）を確かめる。
- `cargo test --offline --workspace`（`lift/` で実行）: 全件成功。失敗 0。
  - fe-decode の extra_seed 5 件（新規 1 件を含む）、既存の全テストを含む。

## 7. 残りの作業量

- 次の停止点 0x41ce10 の扱い（seed に加えるか、間接 call の解決を直すか）は、PM の判断待ち。
- 作業者は繰り返していない。以降、0x41ce10 を越える検証は行っていない。
- フル lift（停止点ゼロ）は未達成。

## 8. 操作の記録

- 新規作成: `lift/out-seed-both/`（`Cargo.toml`、`tool/`、`target/`、`build.sh`、`out/`、`tmp/`、`harness/`、`*.o`、`flengine-after-seed-both`、`run1.txt`）。`docs/decisions/0008-extra-seed-0x41e7e0.md`、`docs/investigations/extra-seed-both-run.md`。
- 既存のファイルの変更: `lift/crates/fe-decode/tests/extra_seed.rs`（テストの追記のみ）、`docs/investigations/closure-count.md`（追記のみ）。
- 使っていないもの: rm、git 操作、/tmp、sleep、timeout。
- 権限拒否: なし。
- 2 GB 級の中間ファイル: 作っていない（生成 C は 39 MB、バイナリは約 41 MB）。
