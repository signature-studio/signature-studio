# fe-lift の試験の基準(`cargo test -p fe-lift`)

対象: `lift/crates/fe-lift/tests/`。製品(`lift/runtime/`・`lift/harness/`・`app/`)は変えていない。直したのは、試験の link と試験の入力だけ。

## 1. 数(この機械で測定)

| | 変更前(HEAD `34684635`) | 変更後 |
|---|---|---|
| 通った件数 | 1636 | 2086 |
| 落ちた件数 | 509(49 binary) | 59(21 binary) |
| `undefined reference` を出す link の失敗 | 452 件 | 0 件 |

依頼文の「HEAD で 446 件」とは一致しない。同じ機械・同じコマンドで測った件数は 509 件。

## 2. 変更前の分類(509 件)

| 分類 | 件数 | 代表の文言 | 直し方 |
|---|---|---|---|
| link: `fle_clip_host_write`・`fle_clip_host_read` が未定義(#1389) | 218 | `undefined reference to 'fle_clip_host_write'` | `tests/support/clip_host_stub.c` を、`fle_*.c` を全部集めて link する試験に加えた(27 試験) |
| link: `fle_win_api_trace_on` が未定義 | 194 | `undefined reference to 'fle_win_api_trace_on'` | `tests/support/win_trace_off.c`(weak。`fle_win.c` があればそちらが勝つ)を、固定リストで link する 7 試験に加えた |
| link: `fle_vfs_restore_put`・`_mkdir`・`_drop` が未定義 | 40 | `undefined reference to 'fle_vfs_restore_put'`(oleaut_ole) | `fle_reg.c` が呼ぶ `fle_vfs.c` を、oleaut_ole の link に加えた |
| 入力の Python が無い(`auth117/mk_signed_pe.py`) | 8 | `can't open file '.../auth117/mk_signed_pe.py'` | 直していない(§5 の「残り」) |
| 入力のファイル・スクリプトが無い | 12 | `No such file or directory` | 直していない(§5) |
| その他 | 37 | 試験ごとの assertion | 直していない(§5) |

`rcp_table_header_matches_host_dump`・`rcpps_all_2e32_inputs_match_host`・`rcpps_matches_host` は、この runner では**通っている**。host の CPU との比較の skip は入れていない(依頼の (3) は今回は該当しない)。

## 3. 直したもの

- `tests/support/clip_host_stub.c`(新規): `fle_clip_host_write` は文字列を捨て、`fle_clip_host_read` は NULL(文字列なし)を返す。`fle_clip.c` の `os_text_read` は NULL を「文字列なし」として扱う。製品の wasm の link は `harness/web/clip/fle_clip_lib.js`(emscripten の js-library)が定義するので、この C は製品の link には入らない。`fle_clip.c` のコメントの「native の試験の link が定義する」に合わせた。
- `tests/support/win_trace_off.c`(新規): `fle_win_api_trace_on` が 0 を返す(fle_win.c の既定)。weak 定義にしたので、`fle_win.c` と同じ link に入っても重複しない。
- 上の 2 つを、試験の link に加えた。
  - clip: `call_iat`・`callback`・`deferred`・`hle_entries`・`hook_points`・`k32_dsp`・`load`・`modules`・`msctf`・`null0`・`site_override`・`srw`・`sysinfo`・`uilang`・`wall_b`・`wall_c1`・`wall_e1`・`wall_fr11`・`wall_fr3`・`wall_fr4`・`wall_fr5`・`wall_fr6`・`wall_q`・`wall_r`・`wall_r2`・`wall_s`・`wall_tbl1`(`fle_*.c` を集めた直後に push)
  - trace: `gdi`・`gdi_bulk`・`cursor`・`loadimage`・`wall_fr7`(gdi 側)・`wall_fr21`(gdi 側)・`wall_r`(固定リストの driver)
- `tests/oleaut_ole.rs`: link の一覧に `fle_vfs.c` を加えた。
- `tests/realmod_s1/tools/patch_base_only.py`(新規。試験入力の生成のみ): 負の対照。PE32+ の ImageBase だけを書き換える。これが無いと、`realmod` の初期化が最初に落ち、後続の試験が同じ出力先に `create_new` して `File exists` で落ちていた。今回の修正で、この連鎖の根は消えた(ただし、§5 の `split_mod.py` は残っている)。
- `tests/known-failures.txt`(新規): 残り 59 件の名前。形式は `<binary> <試験名>`。

## 4. 検証の手順(試験の判定に使う)

- 全体: `cd lift && cargo test -p fe-lift --no-fail-fast`。この機械で 30 分以上かかる。失敗の名前を `binary 名前` の形にして、`tests/known-failures.txt` と比べる。外に出た失敗は退行。
- 変えた試験だけ: `cargo test -p fe-lift --test <binary>` を走らせ、その binary の行と比べる。
- 試験の link を増やすとき: 新しい `fle_*.c` が製品の未定義シンボル(`fle_clip_host_*`・`fle_win_api_trace_on`・`fle_vfs_restore_*` と同種)を持つと、同じ link の失敗が出る。上の一覧の場所に、試験専用の定義を足す(製品の runtime には入れない)。

## 5. 残り(59 件)と理由

| 理由 | 試験(binary) | 件数 | 直し方 |
|---|---|---|---|
| 入力の Python が repo に無い: `tests/auth117/mk_signed_pe.py`・`mutate_real.py`・`tools/authenticode_check.py` | authenticode_check | 13 | 署名付き PE を作る生成器が必要。git の履歴にも無い。作るなら別の Issue |
| `harness/noreturn-audit/noreturn_audit.py` が無い(`allow.tsv` だけがある) | noreturn_audit | 5 | 同上 |
| `tests/realmod_s1/tools/split_mod.py` が無い(`harness/two/gen_two.sh` の `TOOLS` が指す先)。Rust の `src/split.rs` は別の規則の移植で、同じ出力ではない。残りの 15 件は、この最初の失敗の後の `File exists` の連鎖 | realmod | 16 | 古い規則の再現が要る。Python を書くか、試験を Rust の関数に切り替えるかを決める |
| `harness/two/lowregs.py` が無い | size1_lowregs | 1 | 同上 |
| `harness/two/split_bytes.py` が無い(規則は `src/split.rs` の冒頭に写してある) | split_units | 3 | 同上 |
| `tests/x87_trans/gen_ref.py`・`check_bf.py` が無い | x87_trans | 2 | 同上 |
| `archive/harness-attic/` が無い(`a4-multi.sh`・`vis1/vis1_apis.inc`・`web/opfs1/fle_opfs_mount.c`) | wall_fr3 2・wall_fr4 2・wall_fr21 1・wall_opfs1_map 1 | 6 | 参照先を新しい場所に移すか、試験を外す。rc 127 は `a4-multi.sh` の不在 |
| `harness/idle/gen_idle_wraps.py` が無い(1 件)と、directory line の期待の差(1 件) | wall_fr12 | 2 | 生成器を書くか、試験を外す |
| 製品の答えと試験の期待が違う(原因は未確認) | adjustor(58752 対 58695)・vcall_thunk_dll(58 対 1)・wall_c1(shared cursor handle)・wall_dlg1 2(attrs 20 対 80、vfs seed の出力)・wall_fr11(regtrail-double)・wall_fr8(42 対 41)・wall_s(leading-missing-key)・wall_fr20(row 0 が webview2loader 対 user32) | 9 | 製品側の変更か、試験の期待の更新。link の問題ではないので、本作業では触っていない |
| `mutate.py` の結果が 0 件(CAUGHT なし)。未確認 | wall_fr18 | 1 | 原因を確かめる |
| `fle_appfile.c` に `fprintf(` がある(製品の hygiene 検査)。`fle_appfile.c` の window-api trace の行 | wall_fr19 | 1 | 製品の変更。本作業では直していない |

## 6. 数字の由来

- 変更前: `cargo test -p fe-lift --no-fail-fast`(HEAD `34684635`、直す前)。
- 変更後: 2 回走らせた。1 回目は stub に `NULL` の include 漏れがあり、link が 275 件落ちた(stub を直して、もう一度走らせた)。2 回目の結果が上の 59 件。
- 件数は、各試験の出力の `test ... FAILED` 行と、`error: N targets failed` の行から数えた。
