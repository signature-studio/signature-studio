# 0330(ci1、仮の番号): 0323① / 0323b で期待が古くなった native テスト 2 件の修正(テストの期待の更新のみ)

## 0. 実際に実行した物(先頭)

| 実行した | 内容 |
|---|---|
| `git archive HEAD d78f3c98`(`lift/runtime lift/harness lift/crates/fe-lift/tests`)に `0323-contract-fixes.diff` と `0323b-disabled-input.diff` を `patch -p1` | dry-run rc 0、実適用 rc 0、FAILED 0(ワークツリー `lift/out-ci1/kit/out/w0330/`) |
| cargo を使わず、`fe-lift/tests/{win2_act,alpha5_erase_dc}.rs` と同じ clang の手順(`WIN_OBJS` + `fle_x87ctl` を -O0、`OBJS` を -O1 `-DFLE_DESKTOP -DFLE_R_FAST_COPY=1`。nice 19、1 つずつ)で `win_act` と `erase_dc` をビルドして実行し、.rs と同じ比較(pin・Wine 10.0・known-divergence / `checks N failed 0`)を python で再現(`run-tests.py`) | 修正前: **win2_act の pin が 402 行目で不一致**(今 `  P3 WM_SETCURSOR hit=-2 msg=0x200`、pin `= z: P1 P3 P2`。差は 0x200/0x201/0x202 の 3 行の追加だけ)、**alpha5_erase_dc は `checks 13 failed 1`**(『WM_ERASEBKGND に 0 を返すと BeginPaint がもう一度送る(計 2 通)』)。修正後: 両方通る(下) |
| 修正版 diff を別の新しい木(HEAD + 0323 + 0323b)に `patch -p1` | dry-run rc 0、実適用 rc 0、再実行で pin OK・Wine/known-divergence OK・`checks 13 failed 0` |
| 修正版 diff を mt3 の RT-B の木 `~/work/mt3w-s1/tree-rtb` に `patch -p1 --dry-run`(読み取りのみ) | rc 0 |

**実エンジン・Chrome・Wine の実行 0。cargo は 0(.rs と同じ clang の手順の再現であり、`cargo test` そのものではない)。**

## 1. 変更(`0323c-test-expectations.diff`、28 行、sha256 先頭 `c57f0d99696e6783`、2 ファイル。どちらも期待の更新で、契約の変更は 0323 / 0323b の意図どおり)

| ファイル | 変更 | 理由 |
|---|---|---|
| `lift/harness/alpha5/erase_dc.c` | 『WM_ERASEBKGND に 0 を返す → BeginPaint がもう一度送る(計 2 通)』を『もう一度は送らない(delayed erase。計 1 通)』に。`PAINTSTRUCT.fErase` が TRUE の検査はそのまま | 0323①: `WM_ERASEBKGND` が 0 を返したら窓は消去待ちのまま(fErase TRUE)で、2 通目は送らない(Wine 10.0 の UPDATE_DELAYED_ERASE と同じ。Learn: 0 を返すと消去は済んでおらず fErase が TRUE)。意図した変更 |
| `lift/harness/api2/expected/win-act-runtime-0222.txt` | 402 行目の後に `  P3 WM_SETCURSOR hit=-2 msg=0x200` / `0x201` / `0x202` の 3 行を追加(win_act の現在の出力を `render(blocks(...))` した物と一致) | 0323b: 無効な最上位窓への入力は `WM_SETCURSOR` に HTERROR(−2)を送って捨てる(Wine 10.0 の `process_mouse_message`〔`HTERROR` → `WM_SETCURSOR` を送って受理〕と同じ)。`expected/win-act-wine-10.0.txt` の同じ steps とも一致するので、`win-act-known-divergence.txt` に消す行は無い(確認済み: OK) |

**他のテストは走らせていない**(mt3 の test.sh full の他の結果は mt3 の物)。この 2 件以外で 0323 / 0323b の期待が変わるテストが無いかは、mt3 の RT-B の full で見える(**未確認**)。
