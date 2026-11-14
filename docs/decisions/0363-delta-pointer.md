# 0363: 仮想ポインタを delta 形に置き換える(0319-o20-delta-pointer を取り込む。分類 (b)+(a)。「表示中は絶対」は選択)

分類: (b)+(a)。(b): 表示カウントが負の間(カーソルが隠れている間)に限り、SetCursorPos の後のホストの移動を、設定点からの移動量として届ける。この「隠れている間だけ」は Win32 の契約ではなく選択である(決定 2)。(a): 旧 `virt_off`(offset)と、`set_pointer`・`fle_win_pointer_reclamp` でのホストの位置の上書きを除く。新しい環境変数・旗・`#ifdef`・URL パラメータ・ビルド変数は増やさない。
撤回条件: 取り込み後に、P1 の起動・`cargo test` の新しい失敗・P3 の音の書き込みのいずれかが悪化したら、取り込み(`527cb67`)を戻す。または、実エンジンで Tempo の 3 px ドラッグが取り込み後も 522.000 に張り付くことが測れたら、取り込みを戻すか、別の決定で選択を見直す。『Tempo が 522 にならない』は期待ではなく、測定の予測。今回は、どちらも測っていない(未実施)。

日付: 2026-11-14   ROADMAP: R2-3c   一覧: -   コミット: 取り込み `527cb67`(`lift/runtime/fle_win.c`・`lift/crates/fe-lift/tests/wall_fr11.rs` のみ)。2 段目の commit は `d4be398`(この文書・0353 と 0361 の追記・`evidence/R2-3c.md`・`evidence/R2-3c-cargo.log` の 5 ファイル。`git show --stat d4be398` で確認)。この直しの commit は `git log --oneline -- docs/decisions/0363-delta-pointer.md` で見る。

## 決定
決定: `patches/alpha22/impl/0319-o20-delta-pointer.diff` と `patches/alpha22/impl/0319-o20-delta-pointer-tests.diff` を取り込む(HEAD の `6926320` の上に当てる)。`patches/alpha22/impl/0319-o20-delta-rte-to-rtf*.diff` は当てない。

## main の決定(転記)
1. delta-pointer を取り込む。根拠: (i) 取り込み前の `set_pointer` は `virt_off = 0` とし、`fle_win_host_pointer_set` は絶対位置で上書きする(行番号は `evidence/R2-3b.md` §1 の HEAD 上の位置。この作業では、適用の差分の削除行でこの内容を確認した)。0361 の契約(SetCursorPos の後の入力は設定点からの相対で届く)を実装していない。(ii) 実測(main の転記。`evidence/R2-4c2.md`。この作業では未読): 取り込み前(base と work-r24。どちらも `527cb67` より前)は 3 px で 140.000 → 522.000(`evidence/R2-4c2.md`、目視)。取り込み後(work-r25)の測定は `evidence/R2-4f.md`(3 px 後 140.000。目視。n=3。reviewer 検査は未了)。(iii) dry-run: HEAD の版への適用は両方 rc 0(`patch -p1 --dry-run`、適用時も rc 0)。取り込み済みテストの**値**は、適用後も同じ(下の 4 節)。(iv) ROADMAP R2-3 の「README が置換すると書く」の README は存在しない(`find patches -iname 'README*'` の出力は「出力なし」。2026-11-14 に再実行)。組合せは `cargo test` で検証した(4 節)。
2. 分類: (b)(移動量を設定点に足す部分。契約の実装)と (a) の除去(`virt_off` と上書き)。ただし、それを「隠れている間だけ」に限る区別は、Win32 の契約ではなく選択である。旧記録 `docs/archive/decisions/0319-owner-observations-contract-reading.md` の §27.4 が「SetCursorPos の page がこの区別を書いているわけではない」と書く。適用後の注釈も同じ(`lift/runtime/fle_win.c:5840` 付近の「S-pointer-shown-absolute, choice」)。理由の読み(仮説。未検証): ブラウザは表示中のポインタを host の位置に描くので、ゲストの位置が host の位置とずれると、見える位置と届く位置が食い違う。隠れている間(FL の drag 中)は、ずれても見えない。この読みが正しいかは測っていない(未確認)。
3. 撤回条件: 上の文頭のとおり。
4. 0361 との関係: 0361 は変えず、末尾に 1 行の追記を足した(6 節)。食い違いと置換は、この文書の 5 節に書いた。
5. delta の hunk 1 は `fle_win.c` の先頭コメントを delta の説明に差し替える。コメントに `FLE_` の環境変数名・`getenv` は入っていない(2 節の検査)。

## 1 適用(手順 3)
- `patch -p1 --dry-run < patches/alpha22/impl/0319-o20-delta-pointer.diff`: rc 0(hunk 1〜4 が offset −183 で当たる)。`-tests.diff`: rc 0。
- 適用: 両方 rc 0。`.rej` は無し。`.orig` が 1 つ(`lift/runtime/fle_win.c.orig`)できたが、HEAD と `cmp` で同一の写しであり、適用の前に無かったことを確かめて削除した。
- `git diff --stat -- lift`: 2 files changed(`fle_win.c` +16・−17、`wall_fr11.rs` +134・−32。合計 +150・−49)。他のファイルは変えていない。

## 2 `+` 行の検査(手順 2)
- `grep -n -E '^\+.*(getenv|process\.env|URLSearchParams|#if|FLE_|__attribute__|weak)'` を両方の diff に当てた: 0 件(rc 1)。
- 適用後の `+` 行の分類(`git diff -- lift` の `+` 行は計 150 行: `fle_win.c` 16 行・`wall_fr11.rs` 134 行。`git show --numstat 527cb67`):

| 種類 | 行数 | 内容 | 分類 |
|---|---|---|---|
| コメント | 5 | `fle_win.c` 5840 付近の delta の注釈(決定の説明。5843-5847) | 注釈 |
| 空行 | 1 | `fle_win.c:5919` | 体裁 |
| 宣言 | 1 | `int32_t nx = clamp_x(x), ny = clamp_y(y);`(`fle_win.c:5918`) | (b) の実装 |
| 分岐の条件 | 1 | `if (fle_gdi_cursor_display_count && fle_gdi_cursor_display_count() < 0)`(`fle_win.c:5920`)。前半の `fle_gdi_cursor_display_count &&` は弱シンボルの NULL 検査で、`fle_win.c:51` の既存の宣言と、`GetCursorInfo` の `:5902`・`:5903` が同じ検査を既に使う | 選択(隠れている間だけ相対・表示中は絶対。契約ではない) |
| 分岐の括弧 | 2 | `} else {`(`fle_win.c:5923`)・`}`(`:5926`) | 選択(上の分岐の一部) |
| 代入(移動量) | 2 | `ptr_x = clamp_x(ptr_x + (nx - host_ptr_x));`・`ptr_y = …`(`:5921-5922`) | (b) 契約の実装(隠れている間の移動量を設定点に足す) |
| 代入(絶対) | 2 | `ptr_x = nx;`・`ptr_y = ny;`(`:5924-5925`) | 選択(表示中は絶対) |
| 代入(基準) | 2 | `host_ptr_x = nx;`・`host_ptr_y = ny;`(`:5927-5928`) | (b)(次の移動量の基準にする) |
| テスト | 134 | `wall_fr11.rs` の新規テスト 4 本と注釈・期待値(numstat の 134。内訳は未確認) | テスト(コードではない) |

- `fle_win.c` の `+` 行 16 = 上の表のコメント 5・空行 1・宣言 1・分岐の条件 1・分岐の括弧 2・代入 6。`wall_fr11.rs` の `+` 行 134 は numstat の値。
- `-` 行(削除)は両ファイル合計 49 行(`git show --numstat 527cb67`: `fle_win.c` 17・`wall_fr11.rs` 32)。内訳: (a) の除去 11 行(`fle_win.c`: static の `virt_off_x/y` 1、`set_pointer` の `host_ptr`・`virt_off` の代入 4、`fle_win_host_pointer_set` の旧式 4、`fle_win_pointer_reclamp` の offset 再計算 2)。注釈の差し替え 6 行(旧コメント。分類なし)。`wall_fr11.rs` の旧テスト 1 本の差し替え 32 行(分類なし)。11 + 6 + 32 = 49。
- 決定: 旧コメント 6 行は (a)(処理の除去)に数えず、コメントの差し替えとする。根拠: `+` 側のコメント 5 行が対になり、動作を変えない。
- `fle_input.c` は変更なし(`fle_input.c:145-146` が `fle_win_host_pointer_set` の後に `fle_win_pointer_get`、`:745` が `fle_win_pointer_reclamp`。呼び出しの形は HEAD と同じ)。

## 3 実装の読み(適用後の `lift/runtime/fle_win.c`。手順 4)
- `set_pointer`(`:5876-5884`): `ptr_x = clamp_x(x)`・`ptr_y = clamp_y(y)` だけ。`host_ptr` は触らない。保存するのは設定点だけ。
- `fle_win_host_pointer_set`(`:5916-5929`): 表示カウントが負(`fle_gdi_cursor_display_count() < 0`)なら `ptr = clamp(ptr + (n − host_ptr))`、そうでなければ `ptr = n`。最後に `host_ptr = n`(`n` は clamp 済みの新しいホストの点)。
- `fle_win_pointer_reclamp`(`:5930-5936`): `host_ptr` と `ptr` を新しい画面へ clamp するだけ。offset の再計算は無い。

表示カウントの読み(`lift/runtime/fle_gdi.c`): `show_count` は `:140`(宣言。コメントに初期値 0)。`fle_imp_ShowCursor` は `:3279-3287`。`R[1]` が 0 なら `show_count--`(`:3285`)、0 以外なら `show_count++`(`:3284`)。`fle_gdi_cursor_display_count` は `:3356`。負になるのは、FALSE(非表示)の呼び出しが TRUE より多いとき。FL の drag がこれを起こすかは未確認(旧記録 §27.2 の読み。repo には無い)。

追跡表(Y 軸。H = 1080。初期 0。FL の SetPhysicalCursorPos(1037) は R2-4d の仮定で未確認。scale は未確認なので dy は単位なし。値は `(ptr_y, host_ptr_y)`):

| 手順 | HEAD(取り込み前)の ptr_y | delta・隠れている間 | delta・表示中 |
|---|---|---|---|
| 0 初期 | 0 | (0, 0) | (0, 0) |
| 1 ホスト y = 20(表示カウントが負のとき) | 20 | (20, 20)。20 + (20 − 0) ではなく、ptr 0 + (20 − 0) = 20 | 20(絶対。表示カウントが 0 以上のとき) |
| 2 SetPhysicalCursorPos(…, 1037) | 1037(host_ptr も 1037) | (1037, 20)。host_ptr は触らない | (1037, 20) |
| 3 ホスト y = 21 | 21 | (1038, 21)。1037 + (21 − 20) | 21(絶対) |

- FL の次の dy(前の ptr_y − 今の ptr_y): HEAD は隠れている間も表示中も 1037 − 21 = 1016。delta の隠れている間は 1037 − 1038 = −1。delta の表示中は HEAD と同じ 1016。
- 読み: 表示中の絶対は、set の後でも host の絶対位置に戻る。したがって、FL の drag の最後(開始点への SetPhysicalCursorPos と ShowCursor(TRUE))の後の最初のホストの移動は、ブラウザの実位置へ跳ぶ(開始点 + 移動量ではない)。実機の挙動は未確認。
- 限界(旧記録 0319:864 の読み): 画面の端で止まったブラウザのポインタは移動量を出せない。

## 4 `cargo test`(手順 5)
実行: `cd lift && FLENGINE_PE=… cargo test -j 8 --workspace --offline --no-fail-fast -- --test-threads=8 > ../evidence/R2-3c-cargo.log`(バックグラウンド。`.done` の rc 101。`.done` は起動前に無いことを確かめた)。

| 項目 | 基準 `evidence/R2-6-cargo.log` | 今回 `evidence/R2-3c-cargo.log` |
|---|---|---|
| `^test result` の行数 | 219 | 219 |
| 通過 | 2419 | 2422 |
| 失敗 | 1 | 1 |
| 無視 | 1 | 1 |
| 失敗名の集合 | `getmodulehandleexw_returns_in_every_build` | 同じ 1 件 |
| `undefined reference` | 0 | 0 |

- 通過の増分は +3(旧テスト 1 本の削除、新テスト 4 本の追加)。通過は 2422(基準 2419 から +3)。
- 新テスト 4 本は ok: `the_virtual_pointer_moves_by_the_host_delta_while_the_cursor_is_hidden`(`evidence/R2-3c-cargo.log:2828`)、`the_virtual_pointer_is_the_host_point_again_when_the_cursor_is_shown_after_a_set_cursor_pos`(`:2826`)、`the_virtual_pointer_is_brought_inside_a_smaller_screen_and_keeps_moving_by_the_delta`(`:2823`)、`the_virtual_pointer_equals_the_clamped_host_point_when_the_guest_never_sets_the_cursor`(`:2824`)。
- 旧テスト `the_virtual_pointer_keeps_set_cursor_pos_as_a_relative_offset` は今回のログに無い(`grep` rc 1)。基準のログには ok(`:2823`)。
- 注意: これらのテストは C の runtime を Rust のテストから呼ぶ native の検査で、実行して ok になった(パッチの期待値と実行結果が一致した)。FL の窓での値ではない。0361 の契約の読み(表示中の SetCursorPos 後の host 移動が 110 200 になる、という読み。出典: `evidence/R2-3b.md` の 48 行目、§3 の表の行)とは、テストの期待値(表示中は絶対の 20 10)が一致しない。これは決定 2 の選択の結果で、テストが誤りという意味ではない。
- 字義: ROADMAP の「`cargo test` 失敗 0」は、既知の 1 件のため字義どおりには未達。0353 の読み(既知の 1 件を除く新しい失敗 0)で通過。
- 既存の弱シンボルで足りた: `undefined reference` が 0 件なので、`sweep1.rs`・`sweep1_cases.c` へのスタブ追加は不要で、足していない。

## 5 0361 との関係(手順 6)
- 食い違いの事実: 0361 の本文 14 行目は「ホストの最新の絶対位置をその点に再アンカーして offset を 0 にする」「ゲストの位置 = clamp(ホストの位置 + offset)」と書く。取り込み前の実装(`6926320`)はこの式どおりだが、ホストの移動を相対で届けない(offset が 0 のままで、ゲストは絶対位置に動く)。
- 置換: delta は offset を使わない。隠れている間の移動量を、設定点に足す(`ptr = clamp(ptr + (n − host_ptr))`)。0361 の式は、この追記の時点では実装されていない。
- 0361 の変更: 末尾に 1 行の追記のみ。本文・分類・撤回条件は変えていない。
- 0361 の注釈(`fle_win.c` 5843-5848)は、この取り込みで差し替えた(delta の注釈)。

## 6 未実施・未確認
- 実エンジン(FL の窓・ブラウザ)での確認: 未実施。wasm のビルドも未実施。push 後に GitHub の engine-bundle のビルドが走る(結果は未確認)。
- P1(Tempo の 3 px ドラッグで 522.000 にならないこと): 未測定。次の依頼で bundle を作って測る。
- P2・P3: 未測定。`wrap set` 列の値(native の検査)は、FL の窓での値ではない。
- 「隠れている間だけ相対」の読み(ブラウザは表示中のポインタを host の位置に描く、という理由): 仮説。検証していない。
- FL の drag が表示カウントを負にすること: repo に無い。旧記録 §27.2 の読みのみ。
- FL の SetPhysicalCursorPos(1037)(wrap の値): R2-4d の仮定。未確認。
- 画面の端で止まったポインタの限界、drag の最後の跳びの実機の挙動: 未確認。
- P2 の 120 fps は未確認。P3 の音切れは残る(既存の記録のとおり。この取り込みで変わるかは未測定)。O9 は未特定。依頼者機の値は 0 件。
- O20 の Tempo: 取り込み前(base と work-r24)は、既存の記録(`evidence/R2-4c2.md`、目視)で 522.000。取り込み後(work-r25)は `evidence/R2-4f.md` で 3 px 後 140.000(目視。n=3)。この決定は、その原因を断定しない。

## 追記 2026-11-14(reviewer 指摘の直し)

- 直す前の 0363:6(コミット欄): 「2 段目の commit(sha は `git log --oneline -- docs/decisions/0363-delta-pointer.md` で確認)」(`d4be398` の sha が無かった)
- 直した後: 「2 段目の commit は `d4be398`(この文書・0353 と 0361 の追記・`evidence/R2-3c.md`・`evidence/R2-3c-cargo.log` の 5 ファイル。`git show --stat d4be398` で確認)。この直しの commit は `git log --oneline -- docs/decisions/0363-delta-pointer.md` で見る」
- 直す前の 0363:13(決定 2 の分類): 「分類: (b) 契約の実装(隠れている間の相対)と (a) の除去(`virt_off` と上書き)。「表示中は絶対・隠れている間だけ相対」の区別は、Win32 の契約ではなく選択である。」
- 直した後: 「分類: (b)(移動量を設定点に足す部分。契約の実装)と (a) の除去(`virt_off` と上書き)。ただし、それを「隠れている間だけ」に限る区別は、Win32 の契約ではなく選択である。」
- 直す前の 0363:4(撤回条件): 「…実エンジンで Tempo の 3 px ドラッグが取り込み後も 522.000 に張り付くことが測れ、delta が原因の経路に効いていないと分かったら、取り込みを戻すか、別の決定で選択を見直す。」
- 直した後: 「…実エンジンで Tempo の 3 px ドラッグが取り込み後も 522.000 に張り付くことが測れたら、取り込みを戻すか、別の決定で選択を見直す。『Tempo が 522 にならない』は期待ではなく、測定の予測。」(main の決定の文に戻した。張り付きの原因が delta の経路かは、測定の前に決められない)
- 直す前の 0363:25(§2 の `+` 行の計): 「`git diff -- lift` の `+` 行は計 146 行: `fle_win.c` 15 行・`wall_fr11.rs` 131 行」
- 直した後: 「計 150 行: `fle_win.c` 16 行・`wall_fr11.rs` 134 行(`git show --numstat 527cb67`)」
- 直す前の 0363:§2 の分類: 「`if` の 1 行を (b)、代入 9 行を (b) の実装、`-` 行 49 行をすべて (a)」
- 直した後: 「`if` の 1 行と表示中の `ptr = n` の 2 行を「選択」に移した。(b) は移動量・基準の代入。`-` 行 49 行の内訳は (a) 11・旧コメント 6(分類なし)・旧テスト 32(分類なし)」
- 直す前の 0363:12(決定 1 の (iv)): 「`find patches -iname 'README*'` が 0 件」
- 直した後: 「`find patches -iname 'README*'` の出力は「出力なし」(2026-11-14 に再実行)」
- 直す前の 0363:69(§4): 「main の予測の 2422 と一致した」
- 直した後: 「通過は 2422(基準 2419 から +3)」
- 直す前の 0363:72(§4 の 110 200 の出典): 「(`evidence/R2-3b.md` §3)」
- 直した後: 「(`evidence/R2-3b.md` の 48 行目(§3 の表の行))」

### R2-4f の事実(2026-11-14 の追記)

R2-4f(`57fe30c`。`evidence/R2-4f.md`。reviewer 検査は未了)で、work-r25(527cb67 入り)の Tempo の値は 3 px ドラッグ後が 3 run とも 140.000、80 px 後が 120.000、20 px 上後が 125.000(目視。n=3)。同じ時の base は 522.000(3 px 後)。撤回条件の二つ目(3 px 後も 522.000 に張り付く)に当たる値は、この測定の範囲では出ていない。一つ目(P1 の起動・cargo test の新しい失敗・P3 の音の書き込み)は、P1 は n=1 の起動確認(`evidence/R2-4e.md`)だけ、P3 の音の書き込みは未測定。80 px 後・20 px 上後の値の理由は未確認。O20 は未解消。

## 追記(reviewer 再指摘の直し。2026-11-14)

- 括弧(ワークツリーの 79 行目。HEAD では 78 行目): 直す前: 「0361 の契約の読み(表示中の SetCursorPos 後の host 移動が 110 200 になる、という読み。`evidence/R2-3b.md` の 48 行目(§3 の表の行)とは、」(`(表示中の` が閉じていない。この行は `(` 4 個・`)` 3 個)。直した後: 「…という読み。出典: `evidence/R2-3b.md` の 48 行目、§3 の表の行)とは、」(この行は `(` 3 個・`)` 3 個)。直した後の 79 行目は `node -e` の数えで `(` 3 個・`)` 3 個。なお、この追記の中の「直す前」「直した後」の引用行は、引用として括弧を途中で切っているため、括弧の数えの対象外(数えの結果、この追記の引用行に不一致 2 件が出た。対象は本文の行)。
- 「取り込み前後とも 522.000」の訂正(3 箇所)。誤り: 取り込み後(work-r25)の run は `evidence/R2-4c2.md` に無い(`grep -c 'work-r25' evidence/R2-4c2.md` の出力: 0)。取り込み前(base と work-r24)の 522.000 は、その記録の目視の値。
  - 決定 1 の (ii): 直す前: 「取り込み前も後も、Tempo の 3 px ドラッグで 140.000 → 522.000」。直した後: 「取り込み前(base と work-r24。どちらも `527cb67` より前)は 3 px で 140.000 → 522.000」と、取り込み後(work-r25)は `evidence/R2-4f.md` の 3 px 後 140.000。根拠: `git log --oneline 527cb67..f1635bf` と `git log --oneline 527cb67..818b5b0` の出力は空。`git merge-base --is-ancestor` で、両方とも `527cb67` の祖先(終了 0)。work-r24 = `f1635bf`・base = `818b5b0` は `evidence/R2-4c2.md` の 14 行目。
  - 「O20 の Tempo」(§6): 直す前: 「取り込み前も後も、既存の記録(`evidence/R2-4c2.md`、目視)では 522.000。」直した後: 「取り込み前(base と work-r24)は、既存の記録(`evidence/R2-4c2.md`、目視)で 522.000。取り込み後(work-r25)は `evidence/R2-4f.md` で 3 px 後 140.000(目視。n=3)。」
  - `evidence/R2-3c.md` の 4 行目も、同じ形に直した(その追記を参照)。
- 旧コメント 6 行の分類: 直す前: 「注釈の差し替え 6 行(旧コメント。分類なし)」のみ。直した後: §2 に 1 行を足した。「決定: 旧コメント 6 行は (a)(処理の除去)に数えず、コメントの差し替えとする。根拠: `+` 側のコメント 5 行が対になり、動作を変えない。」
- 数値(cargo test の件数・行数・Tempo の値・sha256)は変えていない。

## 追記(reviewer 再々指摘の直し。2026-11-14)

- 出典の行番号(127 行目付近): 直す前: 「work-r24 = `f1635bf`・base = `818b5b0` は `evidence/R2-4c2.md` の 13 行目。」 直した後: 「work-r24 = `f1635bf`・base = `818b5b0` は `evidence/R2-4c2.md` の 14 行目。」 確かめ: `grep -n '818b5b0' evidence/R2-4c2.md` の出力の 14 行目に「base の HEAD は manifest の description では `818b5b0`、work-r24 は `f1635bf`」がある。
- 数値・表・他の節は変えていない。
