# 0364: 到達しない DPI 分岐(fle_proc.c:408)の除去
分類: (a 除去)
撤回条件: 0347 の案 B を採り、FL が `GetProcessDpiAwareness` の応答で描画の経路を選ぶと確かめられ、かつ起動時の DPI を 96 より大きく届ける決定が出たとき、その応答を DPI から決める 1 行を (b) で戻す。
日付: 2026-11-15   ROADMAP: R7-6   一覧: 0347 の決定 5   コミット: 11b29f1..33b676a

## 決定
決定: `lift/runtime/fle_proc.c:408` の三項演算子を除き、`GetProcessDpiAwareness` の応答を常に `FLE_PROCESS_DPI_UNAWARE`(0)にする。`FLE_PROCESS_PER_MONITOR_DPI_AWARE`(2)は参照が 0 になったので定義ごと除く。根拠: `fle_screen_dpi()` は `lift/runtime/fle_screen.h:59` で常に `FLE_SCREEN_DPI_DEFAULT`(96)を返すため、条件 `96 > 96` は偽で、真の枝には HEAD で届かない(R7-6a §3)。除去の前後とも応答は 0(除去前の応答は読み。`evidence/R7-6a.md` §3。実行していない。除去後の応答は、名前指定のテスト `get_process_dpi_awareness_answers_the_current_process_as_dpi_unaware` の ok で確かめた実測)で、値は変わらない。
分類: (a) 到達しない分岐の除去。新しい処理・分岐は足していない。
`lift/runtime/fle_appfile.c:239` のコメントは、HEAD で偽だった「96 unless FLE_SCREEN_DPI says another」を事実に合う文に書き換えた。コメントだけで、`FLE_SCREEN_PX_DPI` の経路・値は変えていない。`lift/crates/fe-lift/tests/screen1.rs` は変えていない(値が変わらないため)。

## 実測(コマンドと日付つき)
- 除去の前: `expire_grep`(パターン `FLE_PROCESS_PER_MONITOR_DPI_AWARE|FLE_PROCESS_DPI_UNAWARE|FLE_SCREEN_DPI`、`lift/`)は 11 件。`fle_proc.h:96`・`:97`、`fle_proc.c:408`、`fle_screen.h:58`・`:59`、`fle_appfile.c:239`、`fle_gdc.h:66`、`screen1.rs:1`・`:109`・`:140`・`:143`。
- 除去の後: `expire_grep`(パターン `FLE_PROCESS_PER_MONITOR_DPI_AWARE`、`lift/`)は 0 件。
- テスト(名前指定、`FLENGINE_PE` は既存の `binaries/win64/FLEngine_x64.dll`):
  - `get_process_dpi_awareness_answers_the_current_process_as_dpi_unaware`(`wall_s.rs`): ok。1 passed・14 filtered out。rc 0。
  - `the_dpi_is_96_whatever_the_environment_says_and_the_physical_size_follows_it`(`screen1.rs`): ok。1 passed・5 filtered out。rc 0。
- cargo test 全体(`cd lift && FLENGINE_PE=…/binaries/win64/FLEngine_x64.dll cargo test -j 8 --workspace --offline --no-fail-fast -- --test-threads=8`。rc 101): 通過 2422・失敗 1・無視 1。失敗名は `getmodulehandleexw_returns_in_every_build` の 1 件だけ(既知。0353 の読み)。`undefined reference` 0 件。ログは `evidence/R7-6b-cargo.log`。

## 読み(解釈。実行していない)
- 応答 0 は decision 0099 の既存の契約(`fle_proc.h:94-95`)で、現在のプロセスは DPI-unaware とする。除去はこの契約を変えない。
- 三項演算子は decision 0212 の opt-in の残り。`fle_screen.h:57` は「decision 0243: the page has no DPI choice any more」と書き、`FLE_SCREEN_DPI` の読みは HEAD の `lift/` に無い。0243 の一覧(`docs/archive/decisions/0243-no-options.md:53`)に `FLE_SCREEN_DPI` は無い。
- 0347 との関係: 0347 の案 B(`IN.DISPLAY` の `hwev` に DPI を載せる、または `WM_DPICHANGED` を送る)は未決で、採否は測定の後。案 B で DPI を 96 より大きく届けるなら、FL が応答 2 を見て経路を変えるかで、この分岐の復活が要り得る。この決定は案 B を採否しない。
- cargo test の判定は 0353 の『読み方』(既知の 1 件 `getmodulehandleexw_returns_in_every_build` を除く新しい失敗 0。`docs/decisions/0353-l01-l04-and-sweep1-stubs.md` の 51 行付近)を R7-6 に適用する。0353 側の追記には R7-6 は載っていない。
- 復活のコスト: 三項演算子の 1 行の戻しと、「何から DPI を決めるか」の値の経路。`fle_screen_dpi()` は今は定数なので、戻すだけでは値の出所が無い。新しい経路は案 B の決定に入る。

## 未確認
- FL が `GetProcessDpiAwareness` の応答 0 をどう使うか: 未確認。
- O20 の原因、O7(画面のぼやけ)の原因が DPI か: 未確認。この決定は O20 を直すものではなく、DPI の問題を解決したとも書かない。
- CI の engine-bundle(wasm のビルド)の結果: 未確認(この依頼ではビルドしていない)。
- `docs/decisions/README.md` の規則 11 の現行の文面: 未読(本依頼の対象外)。

## 出典
- `evidence/R7-6a.md` §4(除去の案 1〜7)・§5(0347 との関係)。
- `docs/decisions/0347-screen-dpi.md`:5・:6・:7・:22(決定 5)。
- `lift/runtime/fle_proc.c:399-410`(旧 408 行)・`lift/runtime/fle_proc.h:94-97`・`lift/runtime/fle_appfile.c:239`・`lift/runtime/fle_screen.h:57-59`。
- `lift/crates/fe-lift/tests/screen1.rs:139-143`・`lift/crates/fe-lift/tests/wall_s.rs:478`。
- `docs/archive/decisions/0212-screen-fit.md:72`(opt-in の出典)・`docs/archive/decisions/0243-no-options.md:53`。
