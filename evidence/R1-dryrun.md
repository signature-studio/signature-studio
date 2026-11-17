# R1-1〜R1-4 パッチの適用可否(dry-run と連鎖の適用)

実エンジン: 実行なし(パッチの適用可否。`lift/` は未変更)
当たったパッチ 6 本 / 当たらなかった 0 本(なし)
dry-run(`patch -p1 --dry-run`)は 6 本とも rc 0。一時コピーの上での本適用も 6 本とも rc 0。
cargo・ビルドは実行していない(依頼どおり)。コンパイルの通否と `cargo test` は未確認。

## 1 一時コピー

- 元の `lift/`: `du -sh lift` = 4.9G(うち `lift/target` = 2.5G、`lift/out*` 各所)
- コピー: `rsync -a --exclude='/target' --exclude='/out*' --exclude='.git' lift/ $TMPDIR/r1-tree/lift/`(rc 0)
- `$TMPDIR` = `/home/ubuntu/.cache/signature-studio/tmp`(`/tmp` は使わない)
- 除外後の `du -sh --exclude=target --exclude='out*' --exclude=.git lift` = 19M。コピーの `du -sh` = 19M
- コピー直後の確認: `diff -rq -x target -x 'out*' lift $TMPDIR/r1-tree/lift` の出力なし(rc 0)
- 残す。R1-1 の本番の適用の確認に使う

## 2 `-p` の段数

6 本とも先頭の接頭辞は `a/lift/…`(0319・0323c・0323d)、`new/lift/…`(0323)、`new4/lift/…`(0323b)。
リポジトリ直下の `lift/` として置いたコピーに対し、`-p1` で当たる。`-p0`・`-p2` の試行は不要だった。

## 3 結果の表

| # | パッチ | -p | dry-run rc | FAILED / rejected / fuzz | offset | 本適用 rc | `.rej` | このパッチが触れたファイル数 | 累積の変更ファイル数 | `getenv` / `#ifdef FLE_` の + 行数 |
|---|---|---|---|---|---|---|---|---|---|---|
| R1-1 | `patches/ci1/0323/diffs/0323-contract-fixes.diff`(32 hunk) | 1 | 0 | なし | `fle_input.c` の 4 hunk が -5 行 | 0 | 0 | 16 | 16 | 0 |
| R1-2 | `patches/ci1/0323/diffs/0323b-disabled-input.diff`(12 hunk) | 1 | 0 | なし | `fle_input.c` の 5 hunk が -5 行 | 0 | 0 | 5 | 17 | 0 |
| R1-3a | `patches/ci1/0330/0323c-test-expectations.diff`(2 hunk) | 1 | 0 | なし | なし | 0 | 0 | 2 | 19 | 0 |
| R1-3b | `patches/ci1/0331/0323d-test-guards.diff`(5 hunk) | 1 | 0 | なし | なし | 0 | 0 | 3 | 22 | 0 |
| R1-4a | `patches/alpha22/impl/0319-callback-stack-limit.diff`(6 hunk) | 1 | 0 | なし | なし | 0 | 0 | 3 | 25 | 1 |
| R1-4b | `patches/alpha22/impl/0319-callback-stack-limit-tests.diff`(4 hunk) | 1 | 0 | なし | なし | 0 | 0 | 1 | 26 | 0 |

- 「累積の変更ファイル数」は 26 = 既存ファイルの変更 16 + 新規ファイル 10(`lift/harness/c0323/` の 9 件と `runtime-0323-plus-4.out` の 1 件)。最後の `diff -rq` の出力と一致する。
- R1-2 の hunk 数: 依頼文は「8 hunk が当たる」。差分の `@@` は 12(`fle_input.h` 2・`fle_input.c` 5・`fle_win.c` 3・`README.md` 1・`runtime-0323-plus-4.out` 1)。12 hunk すべてが rc 0 で当たった。数の食い違いは依頼文側の記載。
- `.orig`: GNU patch 2.8 が R1-1 の offset 時に作った `lift/runtime/fle_input.c.orig` がコピーに 1 件残る(`lift/` には無い。コピーの中だけの物)。

## 4 `FLE_TLS_MORE` と `getenv`

- `grep -rn 'FLE_TLS_MORE' lift/runtime`(現木・未適用): 出力なし、rc 1(0 件)。
- `grep -n 'FLE_TLS_MORE' patches/alpha22/impl/0319-callback-stack-limit.diff`: 31 行目に `+#ifdef FLE_TLS_MORE`(1 件)。
- R1-4a を適用後のコピー: `grep -rn 'FLE_TLS_MORE' lift/runtime` → `lift/runtime/fle_callback.c:51:#ifdef FLE_TLS_MORE`(1 件)。
- 分類(a/b/c、除去するか残すか)は main が決める。この依頼では直していない。

## 5 各コマンドの出力

### 5.1 R1-1 dry-run(rc 0)
```
checking file lift/harness/c0323/build.sh
checking file lift/harness/c0323/contract_0323.c
checking file lift/harness/c0323/expected/README.md
checking file lift/harness/c0323/expected/runtime-0323-scenario-N.out
checking file lift/harness/c0323/expected/runtime-0323.out
checking file lift/harness/c0323/expected/runtime-HEAD-3969ad31-scenario-N.out
checking file lift/harness/c0323/expected/runtime-HEAD-3969ad31.out
checking file lift/harness/c0323/expected/wine-10.0.out
checking file lift/harness/c0323/regress.sh
checking file lift/runtime/fle_class.c
checking file lift/runtime/fle_class.h
checking file lift/runtime/fle_gdc.c
checking file lift/runtime/fle_gdc.h
checking file lift/runtime/fle_input.c
Hunk #1 succeeded at 476 (offset -5 lines).
Hunk #2 succeeded at 538 (offset -5 lines).
Hunk #3 succeeded at 758 (offset -5 lines).
Hunk #4 succeeded at 855 (offset -5 lines).
checking file lift/runtime/fle_input.h
checking file lift/runtime/fle_win.c
```

### 5.2 R1-1 本適用(rc 0)
```
patching file lift/harness/c0323/build.sh
patching file lift/harness/c0323/contract_0323.c
patching file lift/harness/c0323/expected/README.md
patching file lift/harness/c0323/expected/runtime-0323-scenario-N.out
patching file lift/harness/c0323/expected/runtime-0323.out
patching file lift/harness/c0323/expected/runtime-HEAD-3969ad31-scenario-N.out
patching file lift/harness/c0323/expected/runtime-HEAD-3969ad31.out
patching file lift/harness/c0323/expected/wine-10.0.out
patching file lift/harness/c0323/regress.sh
patching file lift/runtime/fle_class.c
patching file lift/runtime/fle_class.h
patching file lift/runtime/fle_gdc.c
patching file lift/runtime/fle_gdc.h
patching file lift/runtime/fle_input.c
Hunk #1 succeeded at 476 (offset -5 lines).
Hunk #2 succeeded at 538 (offset -5 lines).
Hunk #3 succeeded at 758 (offset -5 lines).
Hunk #4 succeeded at 855 (offset -5 lines).
patching file lift/runtime/fle_input.h
patching file lift/runtime/fle_win.c
```

### 5.3 R1-2 dry-run(rc 0)→ 本適用(rc 0)
dry-run:
```
checking file lift/harness/c0323/expected/README.md
checking file lift/harness/c0323/expected/runtime-0323-plus-4.out
checking file lift/runtime/fle_input.c
Hunk #1 succeeded at 376 (offset -5 lines).
Hunk #2 succeeded at 396 (offset -5 lines).
Hunk #3 succeeded at 493 (offset -5 lines).
Hunk #4 succeeded at 517 (offset -5 lines).
Hunk #5 succeeded at 556 (offset -5 lines).
checking file lift/runtime/fle_input.h
checking file lift/runtime/fle_win.c
```
本適用:
```
patching file lift/harness/c0323/expected/README.md
patching file lift/harness/c0323/expected/runtime-0323-plus-4.out
patching file lift/runtime/fle_input.c
Hunk #1 succeeded at 376 (offset -5 lines).
Hunk #2 succeeded at 396 (offset -5 lines).
Hunk #3 succeeded at 493 (offset -5 lines).
Hunk #4 succeeded at 517 (offset -5 lines).
Hunk #5 succeeded at 556 (offset -5 lines).
patching file lift/runtime/fle_input.h
patching file lift/runtime/fle_win.c
```

### 5.4 R1-3a(0323c)dry-run(rc 0)→ 本適用(rc 0)
dry-run:
```
checking file lift/harness/alpha5/erase_dc.c
checking file lift/harness/api2/expected/win-act-runtime-0222.txt
```
本適用:
```
patching file lift/harness/alpha5/erase_dc.c
patching file lift/harness/api2/expected/win-act-runtime-0222.txt
```

### 5.5 R1-3b(0323d)dry-run(rc 0)→ 本適用(rc 0)
dry-run:
```
checking file lift/crates/fe-lift/tests/window.rs
checking file lift/crates/fe-lift/tests/wall_fr9.rs
checking file lift/crates/fe-lift/tests/wall_fr14.rs
```
本適用:
```
patching file lift/crates/fe-lift/tests/window.rs
patching file lift/crates/fe-lift/tests/wall_fr9.rs
patching file lift/crates/fe-lift/tests/wall_fr14.rs
```

### 5.6 R1-4a(0319 callback-stack-limit)dry-run(rc 0)→ 本適用(rc 0)
dry-run:
```
checking file lift/runtime/fle_callback.c
checking file lift/runtime/fle_callback.h
checking file lift/runtime/fle_thread_wasm.c
```
本適用:
```
patching file lift/runtime/fle_callback.c
patching file lift/runtime/fle_callback.h
patching file lift/runtime/fle_thread_wasm.c
```

### 5.7 R1-4b(0319 tests)dry-run(rc 0)→ 本適用(rc 0)
dry-run:
```
checking file lift/crates/fe-lift/tests/callback.rs
```
本適用:
```
patching file lift/crates/fe-lift/tests/callback.rs
```

### 5.8 `.rej` の探索
`find $TMPDIR/r1-tree -name '*.rej' | wc -l` → 0

## 6 リポジトリの確認

- `git diff --stat -- lift` → 出力なし(`lift/` は未変更)
- 適用は `$TMPDIR/r1-tree` の上だけで行った。`lift/`・`app/`・`ci/`・`.github/`・`patches/` は変更していない

## 7 未確認

- 適用後のコピーのコンパイル(`cargo`)と `cargo test`: 実行していない(依頼どおり。ビルドは走らせない)
- 本適用の結果は行数の上での当たりだけ。意味の上での正しさ(たとえば R1-4 の `FLE_TLS_MORE` の扱い)は確かめていない
