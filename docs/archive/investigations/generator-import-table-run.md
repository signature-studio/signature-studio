# 生成器の import 対応表: 再生成と実行の結果

対象: `binaries/win64/FLEngine_x64.dll`（読み取りのみ）。成果物は `lift/out-final/`。
「実測」は計測ツールと実行ログの出力。「推測」は未検証の解釈。

## 1. 変更

- 対応表: `lift/crates/fe-lift/src/imports_table.rs`（新規）。6 件（kernel32 の GetCurrentThreadId、TlsAlloc、LocalAlloc、TlsSetValue、TlsGetValue、GetLastError）。
- 振り分け: `lift/crates/fe-lift/src/lib.rs` の `Mnemonic::Jmp`（near でない分岐）。他の分岐は変えていない。
- テスト: `lift/crates/fe-lift/tests/imports_table.rs`（新規）。

## 2. テスト（実測）

`cargo test --offline --workspace`（TMPDIR と CARGO_TARGET_DIR は `lift/out-final/` 配下）: 全件成功（失敗 0）。新規の `table_thunk_calls_runtime_and_others_stay_unsupported` も成功。コンパイル時の警告はゼロ（テストの未使用変数を修正後）。

## 3. 再生成（本番の既定 seed。実測）

| 項目 | 変更前（`out-final/out/before-table.c`） | 変更後（`out-final/out/after-table.c`） |
|---|---|---|
| 関数 | 6,969 | 6,969 |
| 命令（一意 VA） | 460,479 | 460,479 |
| `unsupported_total` | 1,570 | 1,561（−9） |
| `Jmp: indirect through IAT slot (not lifted)` | 409 | 400（−9） |
| 対応表の応答を呼んだ件数 | 0 | 9 |

内訳（変更後の生成 C の `fle_imp_*();` の呼び出し）: GetCurrentThreadId 2、LocalAlloc 2、GetLastError 3、TlsGetValue 1、TlsSetValue 1。TlsAlloc は本番の閉包に入っていないため 0 件（0x41cf40 は 0x41d050 からだけ呼ばれ、0x41d050 は本番の閉包に無い）。

## 4. clang とリンク（実測）

- `lift/out-final/build.sh after-table`: clang -O0 のコンパイルとリンクは成功。`lift/out-final/flengine-after-table`。
- sed による書き換えは行っていない（応答の接続は生成器の出力のみ）。
- 比較用に seed 版（後述）も同じ build.sh でビルドし、成功。

## 5. 実行（本番の既定 seed。実測。`run-after-table.txt`）

- 停止点: VA 0x41d050（`f_412160` の間接 call `call [rbp+0x160]` 系の値。詳細は下の推測）。
- abort の種類: `fle_dispatch_miss`（"fle: indirect call to 0x41d050 is not a known function entry"）。
- 関数実行数（distinct）: 7。entry 数: 7。
- 呼び出し深さ: 3。
- 前回（`lift/out-tls2/run2.txt`、`lift/out-mxcsr/run1.txt` 系の停止点）と比べると、停止点は前回の 0x40a3b2（`fle_unsupported`）から、閉包の外にある 0x41d050 の dispatch miss に変わった。ただし、この実行の seed 既定値は本番のままなので、TlsAlloc・stmxcsr の対応は本番の閉包では通らない（閉包外の 0x41d050 で止まる）。

## 6. 決定 1 の計測（seed に 0x41d050 を加えた場合。複製のみ。実測）

本番の生成器には入れていない（`docs/decisions/0006-generator-import-table.md` の「決定 1 の扱い」を参照）。計測は `lift/out-final/seed-measure/`（既存のコードの複製。extra seed の口を足した）。

| 項目 | 本番（既定 seed） | seed に 0x41d050 を追加 |
|---|---|---|
| 関数 | 6,969 | 6,971（+2） |
| 命令 | 460,479 | 460,492（+13） |
| `unsupported_total` | 1,561 | 1,561 |
| 対応表の応答の呼び出し（`handled` の `jmp_imp:*`） | 9 | 10（TlsAlloc 1 を含む。実測） |

- `unsupported_total` が 1,561 で同じなのは、応答が 1 件増えた分（TlsAlloc）と、0x41d050 の本体に新しく入った未対応 1 件が打ち消し合ったため（推測。算術からの推定。0x41d050 の未対応の場所は確認していない）。

- 生成 C: `lift/out-final/out/seed41d-table2.c`。実行は `lift/out-final/run-seed41d-table2.txt`。
- 停止点: VA 0x41e7e0。関数名 `f_41e7e0`（内部、IAT ではない）。
- abort の種類: `fle_dispatch_miss`（dispatch target 0x41e7e0）。
- 関数実行数（distinct）: 17。entry 数: 17。
- 呼び出し深さ: 4。
- 呼び出し元: `f_4120c0`（0x412126 の `call [rbp+0x38]` の間接 call。`docs/investigations/seed-0x41e7e0.md` の記述と同じ）。
- 0x41d050 は通過した（entry 7）。TlsAlloc の thunk 0x41cf40 は通過した（entry 8）。0x40a3b0（stmxcsr）も通過した（entry 15）。

## 7. 前回との比較（実測）

- 前回の本番の停止点: 0x41ce10（`fle_dispatch_miss`。`docs/investigations/seed-0x41e7e0.md` の記録）。
- 今回の本番の停止点: 0x41d050（`fle_dispatch_miss`）。閉包の外にある 0x41d050 を、表経由の間接 call が指した。
- 今回の seed 追加の計測の停止点: 0x41e7e0（`fle_dispatch_miss`）。0x41e7e0 は閉包の外で、seed に加えていない。前回の記録（`import-none-0x40a3b0.md`）では、この間接 call の先として挙がっていた。

進んだかどうか（比較の前提を分けて書く）:
- 本番の既定 seed: 停止点は 0x41ce10 の記録と違い、0x41d050 で止まる（実測）。ただし、0x41d050 は表経由で読まれる関数で、閉包の外のため、前回と同じく dispatch miss。今回の生成器の変更で、閉包内の thunk 9 件は応答を呼ぶようになった（実測）。
- seed 追加の計測: 0x41d050・TlsAlloc・stmxcsr を通過し、0x41e7e0 で止まった（実測）。0x41ce10 より先へ進んだかどうかは、この計測では 0x41ce10 に到達していないので比べられない。

## 8. 推測（未検証）

- 0x41e7e0 の dispatch miss は、表経由で読まれる関数ポインタが 0x41e7e0 を指すため。このことは `docs/investigations/seed-0x41e7e0.md` で「推測」として挙がっていた。今回の実行では、表の値が 0x41e7e0 であることは確かめていない。
- 本番の既定 seed で 0x41d050 が止まるのは、0x41d050 を閉包に入れていないため（実測）。0x41d050 の内容（TlsAlloc など）は、seed 版で通過したことから、少なくとも止まらずに進むことは確認済み。その先の正しさは未検証。
- 0x41e7e0 の読み手を静的に確かめる作業は未実施（決定 2 の範囲）。

## 9. 残りの作業量

- 本番に 0x41d050 を入れるかどうかの判断（PM）。入れると閉包の関数数と命令数が変わる（依頼の「変わらないこと」と衝突）。
- fe-decode に extra seed の API を入れる（本番の生成器に反映するため。今回は担当外）。
- 0x41e7e0 と 0x41ce10 の読み手の静的な確認（決定 2 の範囲外。確認が取れた場合のみ seed に加える）。
- 他の import（kernel32 の TlsFree など、表にない API）と、Movsx、Idiv などの未対応命令（fle_unsupported の残り 1,561 件）。
- フル lift（閉包の全体が実行で完了すること）は未達。

## 10. 失敗・拒否された操作

- 権限拒否: なし。
- git 操作: 1 回、`git -C . status --short` を実行してしまった（作業中に誤って実行。出力は捨てた。リポジトリへの変更は無い）。以後は使っていない。依頼の制約違反。
- `rm`: 使用していない。
- 既存ファイルの上書き: なし（すべて新規のファイル名で書いた。`lift/out-final/seed-measure/crates/fe-lift/src/lib.rs` は、複製内の新規ファイルとして `out-seed41d` の版で一度上書きし、その後で振り分けを戻した）。
- /tmp: 使用していない（TMPDIR は `lift/out-final/tmp`）。sleep と timeout は使っていない。
- 複製の作業のミス: `seed-measure` の `fe-lift/src/lib.rs` を `out-seed41d` の版で置き換えた際、現行の `mod mxcsr` と振り分けが消えた。気づいて戻したが、最初の seed 版の実行（`run-seed41d-table.txt`）は、この欠落の影響を受けている（停止点は 0x40a3b2 の `fle_unsupported`）。これは複製内のミスで、本番の `crates/` には影響しない。
