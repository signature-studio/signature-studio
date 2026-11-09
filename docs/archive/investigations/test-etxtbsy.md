# tests/ の ETXTBSY ("Text file busy") 競合

対象: `lift/crates/fe-lift/tests/`。HEAD ab58ed4。環境: clang 21.1.8、cargo 1.99.0、16 コア、TMPDIR は `lift/out-etxt/tmp`。

## 症状

`cargo test --workspace` や 2 本同時の実行で、`null0.rs` の `undefined_cases_stop_the_run` や `runtime_answers` が
`Os { code: 26, kind: ExecutableFileBusy, message: "Text file busy" }` で落ちる。単独や `--test-threads=1` では通る。

## 原因（測定済み）

各テストは C ファイルを書き、clang で実行ファイルを作り、exec する。出力先の名前が呼び出しごとに一意ではなかった。

1. 同一プロセス内のスレッド間。
   - `null0.rs` の `build_exe` は `out-null0/test/null0-run`（と `null0-unit.c`、`null0-main.c`）を固定名で作る。
   - `runtime_answers` と `undefined_cases_stop_the_run` は並列スレッドで同じ `build_exe` を呼ぶ。
   - 一方の clang が `null0-run` を書いている間、またはリンク直後に、もう一方が同じパスを exec する（または上書きする）。
   - 書き込み用 fd が開いたままのファイルを exec すると ETXTBSY になる。
2. 別プロセス間。
   - 出力先は `out-<名前>/test` で、プロセス ID を含まない。
   - 同じツリーで 2 本の `cargo test` を動かすと、同名ファイルを別プロセスが書く。
   - `cargo test --workspace` でも同じ出力先を使うテストバイナリが並列に動く。例:
     - `glob.rs`、`vol.rs`、`vmap.rs`（`apis_common`）は `out-apis/test`
     - `proc.rs` と `time.rs` は `out-proctime/test`
     - `int_more.rs` と `sse_more.rs` は `out-more/test`
     - `jumptab.rs` と `tailjmp.rs` は `out-ijmp/test`
   - 共有オブジェクトの `obj-*` ディレクトリ（例: `obj-vv`、`obj-glob`、`obj-proc`）も、バイナリごとに同じパスへ書き直していた。

出力先の決め方（調査結果）:
- 出力先は `crate/../../out-XXX/test` の固定ディレクトリ。TMPDIR は `<dir>/tmp` を clang に渡している（/tmp は使っていない）。
- ファイル名は `name`、`group`、固定文字列のどれか。カウンタも PID も含まない。
- `apis_common/mod.rs`（vol.rs と vmap.rs が使う）と `fsx_common/mod.rs`（vfs.rs、reg.rs、env.rs が使う）も `dir.join(name)` で、`glob.rs` などに同じ形のヘルパがコピーされている。

## 再現手順（修正前）

TMPDIR は `lift/out-etxt/tmp`。いずれもテストバイナリを直接実行した。

- 同一プロセス内（単独の `cargo test -p fe-lift --test null0` を 6 回連続）: 1 回失敗。
  `undefined_cases_stop_the_run` が `run failed: Os { code: 26, kind: ExecutableFileBusy ... }`、`5 passed; 1 failed; finished in 1.08s`。残り 5 回は成功。
- 2 プロセス同時（null0 のテストバイナリを `&` で 2 本、3 セット）: 1 セットで失敗（`4 passed; 2 failed`、ETXTBSY）。
- 別の再現手順は 2 つの `cargo test --workspace`（CARGO_TARGET_DIR を別にする）の並走。修正前にこの条件で実行はしていない。

いずれも発生は確率的で、毎回は落ちない。

## 修正

- 共通ヘルパを新規追加: `tests/etxt_common/mod.rs`
  - `uniq(name)`: `name-<pid>-<カウンタ>` を返す（呼び出しごとに一意）。
  - `per_process(name)`: `name-<pid>` を返す（1 プロセス内で 1 回だけ作って共有するファイル用）。
- 次のファイルについて、`.c`、実行ファイル、`.o` のパスを `uniq` または `per_process` で決めるよう変更した。
  - 各テストファイルはファイル先頭に `mod etxt_common;` を追加した。
  - `apis_common/mod.rs` と `fsx_common/mod.rs` は `#[path = "../etxt_common/mod.rs"] mod etxt_common;` で取り込んだ。
- 共有オブジェクトのディレクトリ（`obj-vv`、`obj-glob`、`obj-sysinfo`、`obj-proc`、`obj-time`、`obj`、`obj-a9`）は `per_process` にした。
- 出力先ディレクトリ（`out-*/test`）と TMPDIR の流儀は変えていない。テスト本体の期待値も変えていない。
- ETXTBSY の再試行は入れていない。パスを一意にしたことで衝突の組がなくなり、検証で 0 件だったため。fork 中の別プロセスが fd を継承する古典的な競合は、この測定では観測していない（推測: 起きても頻度は低い）。

## 検証結果（修正後）

`cargo test --workspace --no-fail-fast`、TMPDIR は `lift/out-etxt/tmp`、フォアグラウンドで実行。ログは `lift/out-etxt/ws-{1,2,3}.txt` と `par-{a,b}.txt`。

| 実行 | テストバイナリ | passed | failed | ETXTBSY |
|---|---|---|---|---|
| 全体 1 回目 | 63 | 599 | 0 | 0 |
| 全体 2 回目 | 63 | 599 | 0 | 0 |
| 全体 3 回目 | 63 | 599 | 0 | 0 |
| 並走 A（CARGO_TARGET_DIR=out-etxt/target-a） | 63 | 599 | 0 | 0 |
| 並走 B（CARGO_TARGET_DIR=out-etxt/target-b、A と同時） | 63 | 599 | 0 | 0 |

null0 のテストバイナリを 3 本同時に 5 セット（計 15 実行）動かした場合も、全て `6 passed`、ETXTBSY は 0 件（修正前は 2 本同時 3 セットで 1 セット失敗）。

## 未修正の箇所（他 subagent の担当ファイル）

次のファイルは同じパターン（固定名、または `dir.join(name)`）を持っており、同じ競合が起こり得る。触っていない。
- `tests/osver.rs`: `build_c` が `dir.join(name)` で固定名（別 subagent が編集予定）。
- `tests/x87.rs`、`tests/x87ctl.rs`: x87 作業中のファイルで、`out_dir().join(name)` の形。
- `tests/mxcsr_flags.rs`: 未追跡の新規ファイル。`compile()` が `dir.join(name)`。

これらは今回の検証では失敗しなかった（失敗しないことは安全の証明にはならない）。`etxt_common::uniq` を同じ形で使えば直せる。

## 副作用

- 呼び出しごとに一意なファイルが `out-*/test` に残り、実行のたびに増える。削除は禁止されているので掃除していない。
- 全体 3 回と並走後のサイズ（概算）: `out-uC/test` 771 MB、`out-x87/test` 504 MB、`out-int2/test` 350 MB、`out-sse2/test` 291 MB。
- 並走確認用の `out-etxt/target-a` と `target-b` は各 1.8 GB（ビルドディレクトリ）。ディスクの空きは 783 GB。
