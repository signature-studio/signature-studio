# 0355: L08 の診断の文字列(窓のクラス名・タイトル)を、合成のたびでなく fle_desk_describe が呼ばれたときに作る(0322)

分類: (a)。診断の文字列(窓のクラス名・タイトル)を、合成のたびでなく `fle_desk_describe` が呼ばれたときに作る。
撤回条件: R2-7 の P1(first idle)が手元の基準 +100 ms を超えたら外す。
日付: 2026-11-14   ROADMAP: R2-1   一覧: -   コミット: 取り込みは `46e2d1a`(`git log --oneline -1 46e2d1a`)。

## 決定
決定: `patches/ci1/0322/diffs/d2-desk-describe-lazy.diff`(`lift/runtime/fle_desk.c` だけ)を取り込む。`dwin` の `cls[48]`・`title[80]` を `uint64_t hwnd` に変え、`names_of` を `build_desc`(= `fle_desk_describe` の中)から呼ぶ。`build_desc()` は 2 か所の compose から外す。根拠: 窓のクラス名・タイトルは `fle_desk_describe`(診断)だけが読む。合成のたびに作るのは、読まれない文字列の変換と `fle_win_get` の呼び出しの余計な処理(分類 a)。

## 元のパッチとの違い
なし。`patch -p1 --dry-run` が rc 0、本適用も rc 0。`.rej`・`.orig` は無し。`+` 行の `getenv(`・`#ifdef FLE_` は 0 件(`grep -c -e '^[+].*getenv[(]' -e '^[+]#ifdef FLE_' patches/ci1/0322/diffs/d2-desk-describe-lazy.diff` → 0)。

## 確かめた事実
- `build_desc` の呼び出しは `fle_desk_describe` の 1 か所(`grep -n build_desc lift/runtime/fle_desk.c`)。
- describe を読むテスト: `lift/crates/fe-lift/tests/desk1_compose.rs`(2b)と、ハーネス `lift/harness/alpha1/glass_seq.c`・`lift/harness-docs-tools/alpha1/alpha1_rec.c`(`cargo test` の対象外)。
- `cargo test` の失敗集合は HEAD と同じ(`getmodulehandleexw_returns_in_every_build` の 1 件)。

## 未確認
- 実機(Windows の FL Studio)での describe の行の内容。
- ハーネス `glass_seq.c` の実行(`cargo test` の外)。
