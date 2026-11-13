実エンジン: 実行なし(`cargo test`。一時コピー上。`lift/` は未変更)
原因: (1) リンク対象に fle_win.c が無い — ただし fle_win.c を足すだけでは通らない(別の未定義が 56 件出る。下の 3)。/ (2) 別の原因 — 確定せず。未確認は 4 に書く。
`lift/` は未変更(`git status --short lift` → 空)

## 1 sweep1 がリンクする対象(`lift/crates/fe-lift/tests/sweep1.rs`)

- `OBJS`(32 行): 13 個 = `fle_sweep1_kbd, fle_api2_call, fle_kbd, fle_input, fle_locale, fle_time, fle_glob, fle_imports, fle_env, fle_vmap, fle_proc, fle_sys, fle_vfs`。**`fle_win` は無い。**
- 各 `.c` は `clang -c -I runtime runtime/<n>.c` でコンパイルし、`build_run` と `undefined_inputs_stop_by_name` は `OBJS` の `.o` 13 個と `sweep1_cases.c`(または生成した `.c`)を `clang` でリンクする(sweep1.rs 66〜77 行・186〜191 行)。
- `sweep1_cases.c` の 21〜32 行に、`fle_win_input_*` と `fle_win_pointer_get/set` の**空のスタブ**がある。コメント(20 行): `fle_input.c (the US layout table of ToUnicode) references the window layer, which these cases never reach: link stubs, not behaviour`。
- `sweep1.rs` の 159〜161 行にも、同じ名前の空スタブがある(`undefined_inputs_stop_by_name` の C ソース内)。
- 取り込み前の `lift/runtime/fle_input.c`(`lift/` は未変更)が参照する `fle_win_*` は 52 行(`grep -n 'fle_win_[a-z_]+\(' lift/runtime/fle_input.c`)。このうち `fle_win_input_call_hooks`・`fle_win_input_hit_list` は取り込み前の `lift/` に定義が無い(`grep -rn` で `patches/ci1/` の中にだけある)。取り込み後の定義は `fle_win.c`(一時コピーで確認: `fle_win.c:2529 int fle_win_input_call_hooks(...)`、`fle_win.c:6715 unsigned fle_win_input_hit_list(...)`)。

## 2 再現(一時コピー・取り込み 4 本の上)

コピー: `rsync -a --exclude='/target' --exclude='/out*' --exclude='.git' lift/ $TMPDIR/r1-link-tree/lift/`、`diff -rq -x target -x 'out*' lift $TMPDIR/r1-link-tree/lift` → 出力なし(rc 0)。
パッチ 4 本: 0323 contract-fixes・0323b disabled-input・0330 0323c・0331 0323d の順に `patch -p1 -d $TMPDIR/r1-link-tree`。各 rc 0、`.rej` 0。
コマンド: `cd $TMPDIR/r1-link-tree/lift && FLENGINE_PE=$HOME/work/signature-studio/binaries/win64/FLEngine_x64.dll CARGO_TARGET_DIR=$TMPDIR/r1-link-target cargo test -p fe-lift --test sweep1 --offline`
ログ: `$TMPDIR/r1-link-sweep1.log`(47 行。rc 101)

```
test header_and_rows_agree ... ok
test rows_are_new_declared_and_defined ... ok
test undefined_inputs_stop_by_name ... FAILED
test cases_equal_the_wine_output_except_the_listed_divergences ... FAILED
thread 'undefined_inputs_stop_by_name' panicked at crates/fe-lift/tests/sweep1.rs:192:5:
fle_input.c:(.text+0x1c23): undefined reference to `fle_win_input_call_hooks'
fle_input.c:(.text+0x1cde): undefined reference to `fle_win_input_call_hooks'
fle_input.c:(.text+0x3e8c): undefined reference to `fle_win_input_hit_list'
thread 'cases_equal_the_wine_output_except_the_listed_divergences' panicked at crates/fe-lift/tests/sweep1.rs:74:5:
(同じ 3 件)
test result: FAILED. 2 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s
```

`grep -c 'undefined reference'` → 6(2 テスト × 3 件)。`evidence/R1-3b-after.log` の 2307〜2322 行と同じ 2 シンボル・同じ 3 箇所(2307・2308・2310 行。2319・2320・2322 行が同じ内容の 2 回目)。**再現した。**

## 3 使い捨ての版(`fle_win.c` を足した。一時コピー `$TMPDIR/r1-link-tree-throw` だけ)

変更(一時コピーの上だけ。`lift/` には書いていない):
- `sweep1.rs`: `OBJS` に `"fle_win"` を足し(13 → 14)、159〜161 行の空スタブ 9 個を削除。差分: `$TMPDIR/r1-link-throw-sweep1.diff`(23 行)。
- `sweep1_cases.c`: 21〜32 行の空スタブ(`fle_win_input_*`・`fle_win_pointer_*`)を削除。差分: `$TMPDIR/r1-link-throw-cases.diff`(21 行)。
- スタブを外したのは、同名の関数を `fle_win.c` と空スタブの 2 か所に置くと重複定義になるおそれがあったため。外さない版は作って確かめていない(未確認)。

コマンド: 同じ(`CARGO_TARGET_DIR=$TMPDIR/r1-link-target-throw`)。ログ: `$TMPDIR/r1-link-throw-sweep1.log`(127 行。rc 101)。

```
test undefined_inputs_stop_by_name ... FAILED
test cases_equal_the_wine_output_except_the_listed_divergences ... FAILED
test result: FAILED. 2 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
```

- `fle_input.c` 由来の未定義: **0 件**(`grep -c 'fle_input.c:.*undefined'` → 0)。`fle_win_input_call_hooks`・`fle_win_input_hit_list` は解決した。
- 残った未定義(`grep -o` で重複を除いた件数。計 56 件):

| 未定義シンボル | 件数 |
|---|---|
| `fle_call_guest` | 16 |
| `fle_dispatch_table` | 10 |
| `fle_class_name_of` | 10 |
| `fle_dispatch_count` | 4 |
| `fle_class_style_of` | 4 |
| `fle_class_set_hicon` / `fle_class_lookup` / `fle_class_hinst_known` / `fle_class_hcursor_of` / `fle_class_hbrbackground_of` | 各 2 |

- 定義元(一時コピーの `runtime/` の grep): `fle_class_*` は `fle_class.c`。`fle_call_guest` は `fle_callback.c:29`(ほか `fle_dsound.c`・`fle_gdi.c` に呼び出し)。`fle_dispatch_table`・`fle_dispatch_count` は `runtime/fle_dispatch.h:16-17` の `extern` 宣言だけで、コメント(2〜3 行)によると **fe-lift が生成する翻訳単位が出す**。`runtime/` の `.c` には定義が無い。
- 結果: **通らない。別の失敗が出る**。`fle_win.c` を足すと、その依存(`fle_class.c`・`fle_callback.c`・生成物)もリンクが要り、`sweep1` の 13 個の枠では収まらない。
- `sweep1.rs` の意図との関係: 冒頭コメントは「No Wine is needed」「sweep1 は user32 の keyboard layout の 12 件を Wine 10.0 の出力と比べる」。`sweep1_cases.c` の 20 行は「fle_input.c が window layer を参照するが、これらのケースは到達しない。link stubs, not behaviour」と明記している。つまり sweep1 は **window 層をスタブで置き換えて作られた小さなハーネス**。`fle_win.c` を足すと、このスタブの前提(20 行のコメント)に反する。

## 4 判定と未確認

- 判定: 2 件のリンク失敗の**直接の原因**は (1)「リンク対象に `fle_win.c` が無い」。根拠: `fle_win.c` を足すと、`fle_input.c` 由来の未定義 6 件(×2 テスト)が 0 件になる(3 節)。
- ただし (1) の直し方は「足す」だけでは済まない。別の未定義 56 件が残る(3 節)。よって「テストのリンク対象を変えれば通る」とは言えない。
- 「別の原因」の有無: 2 テストの `fle_input.c` 由来の失敗は (1) で説明できる。`fle_win.c` 以降の未定義は別の層(fle_class・fle_callback・生成物)の話で、これは (2) に当たる。
- 未確認:
  - 生成物(`fle_dispatch_table`・`fle_dispatch_count`)を、sweep1 で何によって供給するべきか。`fe-lift` の生成経路は読んでいない。
  - 取り込み 4 本のパッチ(0323・0323b・0323c・0323d)の意味上の正しさ(Wine との差、実エンジンの挙動)。実エンジンは実行していない。
  - `sweep1_cases.c` の `#ifdef SWEEP1_WINE` 側(Wine 側)への影響。Wine 側は実行していない。
  - 別のテスト(`sweep1.rs` 以外)が `fle_win.c` の有無で変わるか。確かめていない。

## 5 変えたファイル・コマンドの出力

- `lift/`: 変更なし(`git status --short lift` → 空)。
- `patches/`・`app/`: 変更なし。
- 一時コピー(`$TMPDIR/r1-link-tree`・`$TMPDIR/r1-link-tree-throw`): パッチ 4 本の上。元の `lift/` には書いていない。
- `$TMPDIR/r1-tree`(別の作業の物): 触っていない(`ls -ld` の更新時刻 Nov 14 01:27 は変わっていない)。
- 使った target: `$TMPDIR/r1-link-target`・`$TMPDIR/r1-link-target-throw`(元の `lift/target` は使っていない)。

## 6 提案(直さない。この依頼の範囲外)

- 案 A: sweep1 の 2 テストを `fle_win.c` 抜きで通す。`fle_input.c` の `fle_win_input_call_hooks`・`fle_win_input_hit_list` の 2 つをスタブ側に足す(`sweep1_cases.c` と `sweep1.rs` の stub 一覧に 2 行)。ただし、これは FL の挙動を補う行為ではなく、テスト側のスタブの拡充。取り込みの分類(b 契約の実装 / a 除去 / 通信・変換の削減)とは別の分類になるため、決定文書が要る。
- 案 B: sweep1 のリンク対象に `fle_win.c` と `fle_class.c`・`fle_callback.c`・生成物を足す。sweep1 の意図(20 行のコメント)に反するため、採るなら sweep1 の前提の変更として決定する。
- main の判断を要する点(決定はしていない): 案 A と案 B のどちらを採るか。

## 7 検査

- `scripts/scan-forbidden.sh evidence/R1-link.md` → 結果は次の行に記録する(push 前)。
