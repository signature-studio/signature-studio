# 0331(ci1、仮の番号): 0323 / 0323b の守り・期待の更新(テスト側のみ)— 0323c の続き `0323d-test-guards.diff`

## 0. 実際に実行した物(先頭)
- 新しい `git archive HEAD`(5ea554e1)の `lift` に、`0323-contract-fixes.diff`・`0323b-disabled-input.diff`・`0323c-test-expectations.diff`・`0323d-test-guards.diff` を `patch -p1` で順に当てた木(`lift/out-ci1/kit/out/w0331/`)で、**本物の `cargo test --offline -p fe-lift --no-fail-fast --test window --test wall_fr9 --test wall_fr14 --test alpha5_erase_dc --test win2_act`**(nice 19、`CARGO_BUILD_JOBS=2`、別の `CARGO_TARGET_DIR`)を 3 回: 1 回目 wall_fr9 が落ちる(GetDC の関数別の数)、2 回目 wall_fr9 と window が別の守りで落ちる、3 回目(最終の diff)で **5 つの test binary すべて通る**。
- 結果(最終): alpha5_erase_dc 1 passed、wall_fr14 26 passed、wall_fr9 30 passed、win2_act 2 passed、window 37 passed = **96 passed、0 failed**。ログ: `cargo-test-0323abcd-5targets.log`。
- 別の木(HEAD d78f3c98 + 0323 + 0323b + 0323c)に `patch -p1 --dry-run` rc 0、mt3 の `tree-rtb` に `patch -p1 --dry-run` rc 0(読み取りのみ)。
**実エンジン・Chrome・Wine の実行 0。runtime は変えていない(テスト側のみ)。これら 5 つ以外のテストは走らせていない。**

## 1. 変更(`0323d-test-guards.diff`、3 ファイル。いずれも 0323 / 0323b の意図した変更に対する守り・期待の更新)
| ファイル | 変更 | 理由 |
|---|---|---|
| `window.rs` | ① `uintptr_t` を禁止語から外し、fle_win.c に 1 回だけ、`def_erase_background` の中にだけ現れることを検査(`assert_eq!(count, 1)` と関数の範囲)。② `fle_call_guest(` の数 8 → 9(+1 は `fle_win_input_call_hooks` = WH_MOUSE / WH_KEYBOARD のフック手続き。0323③) | ① 0323① の `def_erase_background` が自分の 16 byte の RECT の番地を guest の番地として GetClipBox / FillRect に渡す(この runtime では guest の番地 = host の番地。`fle_addr` は恒等か上位 32 bit の検査)。**runtime を `size_t` や guest stack に変える案もあるが、テスト側のみという指示で守りを 1 か所だけ許す形にした**(guest stack に作る方が筋なら次の runtime の変更で) |
| `wall_fr9.rs` | ① 『FillRect を使わない』を、weak の `fle_imp_FillRect`(宣言 1・呼び出し 1・他の形なし)だけ許す形に。② GetDC / ReleaseDC の関数別の数: `paint_now_tree` の 1 / 1 を `erase_now_one` の 1 / 1 と `paint_now_tree` の 0 / 0 に(関数の先頭は前方宣言と区別するため `\n{` までを含める) | 0323①: RedrawWindow が WM_ERASEBKGND に渡す DC(GetDC … ReleaseDC)が `paint_now_tree` から新しい `erase_now_one`(更新領域でクリップ)に移った |
| `wall_fr14.rs` | disabled な最上位窓の期待に `  A msg 20 SETCURSOR wp A hit -2 mouse 200 / 201 / 202` の 3 行 | 0323b |
