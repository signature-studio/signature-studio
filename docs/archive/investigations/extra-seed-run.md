# extra seed 0x41d050 の実行（2026-10-20）

決定: `docs/decisions/0007-extra-seed-0x41d050.md`。本記録は実測のみを記す。推測は「推測」と明記する。

## 1. fe-decode / fe-lift の API（実測）

- `fe_decode::analyze_with_seeds(pe, extra)` を追加。`analyze(pe)` は `analyze_with_seeds(pe, &[])` を呼ぶ。
- `fe_lift::generate_with_seeds(data, extra, w)` を追加。`generate` は空の `extra` で呼ぶ。
- 既存の既定の seed（entry と export）は変えていない。`extra` は既定の seed の後ろに積む。
- 既存のテストは全件通過（後述）。`analyze(pe)` と `analyze_with_seeds(pe, &[])` の一致は、新規テストで確認。

## 2. 閉包（実測）

計測: `lift/out-seed-final/target/debug/seed-final closure binaries/win64/FLEngine_x64.dll [41d050]`

| 項目 | 既定 | extra 0x41d050 |
|---|---|---|
| seed（コード） | 3 | 4 |
| 関数 | 6,969 | 6,971 |
| 基本ブロック | 67,399 | 67,403 |
| 命令 | 460,479 | 460,492 |
| IAT 経由 jmp | 409 | 410 |
| decode 失敗 | 0 | 0 |

期待値（6,971 / 460,492）と実測は一致した。計測用の複製（`lift/out-final/seed-measure/`）の結果とも一致する（`docs/investigations/closure-count.md` の追記も参照）。

## 3. 生成（実測）

生成: `lift/out-seed-final/target/debug/seed-final lift binaries/win64/FLEngine_x64.dll lift/out-seed-final/out/after-seed41d050.c 41d050`
件数の記録: `lift/out-seed-final/out/gen-report.txt`

| 項目 | 値 |
|---|---|
| 関数 | 6,971 |
| 閉包の命令 | 460,492 |
| 生成した命令（重複を含む） | 460,572 |
| bad_nodes | 0 |
| goto | 42,816 |
| C の行数 | 502,412 |
| C のバイト数 | 39,177,494 |
| 未対応（unsupported_total） | 1,561 |
| うち `Jmp: indirect through IAT slot (not lifted)` | 400 |
| 応答の呼び出し（`jmp_imp:*`） | 合計 10（GetCurrentThreadId 2、GetLastError 3、LocalAlloc 2、TlsAlloc 1、TlsGetValue 1、TlsSetValue 1） |
| `call_ind:dispatch` | 6,478 |
| `call_ind:rip_table` | 12 |
| `jmp_ind:rip_table` | 14 |

本番の統計（`lift/out-final/after-table-stats.txt`）との比較（実測）:

- 未対応の節（`unsupported_total` 以降）は全行一致。未対応の合計は 1,561 件で同じ。
- 応答の呼び出しは、`jmp_imp:TlsAlloc` が 1 件増えた（本番は 0 件）。増えた thunk は `L_41cf40`（`f_41cf40`）にある。`f_41cf40` は 0x41d050 の追加で閉包に入った関数（実行ログの entry 8 で呼ばれる）。
- `ret` は 6,555 → 6,556（+1）。その他の命令数の増分は、閉包の増分（+13 命令）と整合する。

## 4. clang とリンク（実測）

- ビルド: `sh lift/out-seed-final/build.sh after-seed41d050`（`out-final/build.sh` と同じフラグ。出力先は `lift/out-seed-final/` のみ）
- 結果: 成功。`lift/out-seed-final/flengine-after-seed41d050` を生成。

## 5. 実行（実測）

実行: `lift/out-seed-final/flengine-after-seed41d050 binaries/win64/FLEngine_x64.dll`
ログ: `lift/out-seed-final/run-after-seed41d050.txt`（終了コード 134、SIGABRT）

| 項目 | 前回（0x41d050 を加えない本番、`lift/out-final/run-after-table.txt`） | 今回（extra 0x41d050） |
|---|---|---|
| 停止点 | 0x41d050 | 0x41e7e0 |
| 停止の種類 | `fle_dispatch_miss`（SIGABRT） | `fle_dispatch_miss`（SIGABRT） |
| 関数実行数（distinct） | 7 | 17 |
| 呼び出し回数 | 7 | 17 |
| 呼び出し深さ（停止時） | 3 | 4 |
| 停止時のスタック | `f_412160`、`f_41d8f0`、`f_1376820` | `f_4120c0`、`f_412160`、`f_41d8f0`、`f_1376820` |

- 0x41d050 は越えた（entry 7 として `f_41d050` が呼ばれ、その後 `f_41cf40` などが続いた。実測）。
- 次の停止点は 0x41e7e0（`fle: indirect call to 0x41e7e0 is not a known function entry`）。0x41ce10 ではない。
- 生成 C（`after-seed41d050.c`）に `f_41e7e0` は**含まれない**（grep 実測）。0x41e7e0 は閉包の外にある関数で、生成されていない。実行時の dispatch miss は、表（`fle_dispatch_table`）に無い VA への間接 call である。
- 関数名: 0x41e7e0 は名前を持たない（export ではない。seed でもない）。

## 6. 次の停止点について（推測を含む）

- 0x41e7e0 は、決定 0007 で seed に加えないと決めた関数。したがって、この停止は決定どおりの結果である。
- 0x41e7e0 が表経由で呼ばれたこと（間接 call の宛先）は、実行で確かめられた。どの間接 call から来たかは、本記録では特定していない（未検証）。
- 0x41e7e0 を seed に加えると、フル lift に近づく。ただし、この判断は PM が決める（本記録では行わない）。

## 7. 残りの作業量

- 0x41e7e0 の扱い（seed に加えるか、表経由の読み手を静的に確かめるか）: 未決定。PM の判断が要る。
- フル lift の達成: 未達。次の停止点が 0x41e7e0 で、その先にさらに停止点がある可能性は未検証。
